//! Yerel uç sunucusunun Win32 yapıştırıcısı: sunucu (`uc/src/sunucu.rs`) Win32'den
//! bağımsızdır; pencere ve ayar işleri ileti penceresine devredilir. Gizle/göster `PostMessageW`
//! ile (sonuç beklenmez); ayar değişikliği EŞZAMANLI (`SendMessageTimeoutW`): pencere sahibi
//! uygulayıp sonucu yazana kadar beklenir ve sonuç yanıt gövdesinde döner.

use crate::ayar::AyarDegeri;
use crate::sunucu::Saglayici;
use crate::tepsi::{WM_UC_AYAR, WM_UC_GIZLE, WM_UC_GOSTER};
use crate::uc::{Devir, Is};
use crate::{api, ayarlar, pencere};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
};

/// Ayar işinin pencere sahibinde işlenmesi için en çok beklenecek süre (eklentinin 3 sn'lik
/// zaman aşımından kısa: eklenti 503'ü görsün, kendi zaman aşımına düşmesin).
const AYAR_ZAMAN_ASIMI_MS: u32 = 1_500;

/// Ayar değerlerini ileti penceresine devreder ve uygulanmış sonucu bekler. Zaman aşımında ya da
/// pencere yanıt vermezse istek iptal edilir (sonradan uygulanmaz) ve `Basarisiz` (503) döner.
fn ayar_devret(hedef: HWND, degerler: &[AyarDegeri]) -> Devir {
    let istek = ayarlar::kuyruk().ekle(degerler.to_vec());
    let mut sonuc = 0usize;
    // Dönüş değeri önemsiz: işlenip işlenmediğini `istek.sonuc()` söyler (iptal de oradadır).
    unsafe {
        SendMessageTimeoutW(
            hedef,
            WM_UC_AYAR,
            0,
            0,
            SMTO_ABORTIFHUNG,
            AYAR_ZAMAN_ASIMI_MS,
            &mut sonuc,
        )
    };
    match istek.sonuc() {
        Some(govde) if govde.is_empty() => Devir::Tamam,
        Some(govde) => Devir::Govde(govde),
        None => Devir::Basarisiz,
    }
}

/// Sunucuyu başlatır; gizle/göster `ileti_penceresi`ne postalanır, ayar işi eşzamanlı işlenir.
pub fn baslat(ileti_penceresi: isize) {
    let devret = move |is: &Is| {
        let hedef = ileti_penceresi as HWND;
        let postala = |ileti| {
            if unsafe { PostMessageW(hedef, ileti, 0, 0) } != 0 {
                Devir::Tamam
            } else {
                Devir::Basarisiz
            }
        };
        match is {
            Is::Yok => Devir::Tamam,
            Is::Gizle => postala(WM_UC_GIZLE),
            Is::Goster => postala(WM_UC_GOSTER),
            Is::Ayar(degerler) => ayar_devret(hedef, degerler),
        }
    };
    let sag = Saglayici {
        surum: env!("CARGO_PKG_VERSION"),
        durum: pencere::durum_ozeti,
        ayar_metni: ayarlar::govde,
        api_acik: api::acik_mi,
    };
    crate::sunucu::baslat(sag, devret);
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::ayar::Tik;
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostQuitMessage, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WM_CLOSE, WM_DESTROY,
        WNDCLASSW,
    };

    /// Sınama penceresinin kuyruktan işlediği istek sayısı.
    static ISLENEN: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn sinama_proc(hwnd: HWND, ileti: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        match ileti {
            WM_UC_AYAR => {
                ayarlar::kuyruk().isle(|d| {
                    ISLENEN.fetch_add(1, Ordering::SeqCst);
                    // Tek değer: API sonucu gövdesi; çok değer: gövdesiz.
                    if d.len() == 1 {
                        "api_otomatik_baslat=yazildi\n".to_owned()
                    } else {
                        String::new()
                    }
                });
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, ileti, w, l),
        }
    }

    fn genis(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// İleti penceresini ayrı iş parçacığında kurar; `bekle` kadar uyuduktan sonra ileti döngüsüne
    /// girer (öncesinde pencere sahibi yanıt vermiyor sayılır).
    fn pencere_kur(ad: &str, bekle: Duration) -> (isize, std::thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();
        let sinif_adi = format!("LogseqTepsiSinama{ad}");
        let is = std::thread::spawn(move || unsafe {
            let sinif = genis(&sinif_adi);
            let ornek = GetModuleHandleW(null());
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.lpfnWndProc = Some(sinama_proc);
            wc.hInstance = ornek;
            wc.lpszClassName = sinif.as_ptr();
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                0,
                sinif.as_ptr(),
                sinif.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                ornek,
                null(),
            );
            tx.send(h as isize).unwrap();
            std::thread::sleep(bekle);
            let mut m: MSG = std::mem::zeroed();
            while GetMessageW(&mut m, null_mut(), 0, 0) > 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        });
        (rx.recv().unwrap(), is)
    }

    fn kapat(h: isize, is: std::thread::JoinHandle<()>) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(h as HWND, WM_CLOSE, 0, 0)
        };
        is.join().unwrap();
    }

    /// Gerçek `SendMessageTimeoutW` yolu (süreç genelindeki kuyrukla çalışan tek sınama).
    #[test]
    fn ayar_devri_eszamanli_sonuc_doner_zaman_asiminda_iptal_edilir() {
        // 1) Pencere sahibi ileti döngüsünde: sonuç (204 değil) gövdede döner, iş işlenmiştir.
        let (h, is) = pencere_kur("Canli", Duration::ZERO);
        let devir = ayar_devret(h as HWND, &[AyarDegeri::Tik(Tik::Cift)]);
        assert_eq!(devir, Devir::Govde("api_otomatik_baslat=yazildi\n".into()));
        assert_eq!(ISLENEN.load(Ordering::SeqCst), 1, "yanıt dönünce iş bitmiş");
        let iki = [AyarDegeri::Tik(Tik::Cift), AyarDegeri::Tik(Tik::Tek)];
        assert_eq!(
            ayar_devret(h as HWND, &iki),
            Devir::Tamam,
            "gövdesiz sonuç 204"
        );
        assert_eq!(ISLENEN.load(Ordering::SeqCst), 2);
        kapat(h, is);

        // 2) Pencere sahibi yanıt vermiyor (ileti döngüsünde değil): zaman aşımı, 503, iptal.
        let once = ISLENEN.load(Ordering::SeqCst);
        let (h2, is2) = pencere_kur("Donuk", Duration::from_millis(2500));
        let basla = Instant::now();
        assert_eq!(
            ayar_devret(h2 as HWND, &[AyarDegeri::Tik(Tik::Tek)]),
            Devir::Basarisiz
        );
        let gecen = basla.elapsed();
        assert!(gecen >= Duration::from_millis(1400), "{gecen:?}");
        assert!(gecen < Duration::from_millis(2400), "{gecen:?}");
        // Pencere sahibi sonradan uyanınca iptal edilmiş istek UYGULANMAZ; yenisi uygulanır.
        std::thread::sleep(Duration::from_millis(1300));
        let devir = ayar_devret(h2 as HWND, &[AyarDegeri::Tik(Tik::Cift)]);
        assert_eq!(devir, Devir::Govde("api_otomatik_baslat=yazildi\n".into()));
        assert_eq!(
            ISLENEN.load(Ordering::SeqCst),
            once + 1,
            "iptal edilen istek sonradan işlenmemeli"
        );
        kapat(h2, is2);
    }
}
