//! Genel kısayollar (`RegisterHotKey`): kimlik başına tek kayıt, ileti penceresi iş parçacığında.
//! Yeni kısayol (ör. anında not) eklemek için yeni bir kimlik sabiti ve ayar yeter; ayrıştırma ve
//! kayıt planı saf kodda (`kisayol`).

use logseq_tepsi_cekirdek::kisayol::{self, Kisayol};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{GetLastError, HWND};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT,
};

/// Logseq'i göster/gizle kısayolu.
pub const KIMLIK_GOSTER_GIZLE: i32 = 1;
/// Anında not kısayolu.
pub const KIMLIK_NOT: i32 = 2;

/// Şu an kayıtlı kısayollar (kimlik, kısayol).
static KAYITLI: Mutex<Vec<(i32, Kisayol)>> = Mutex::new(Vec::new());
/// Kaydı başarısız olan kimlikler (bit = kimlik; kimlik < 32).
static HATALI: AtomicU32 = AtomicU32::new(0);

/// Kimliklerin kaydını `istenen`e getirir (`None` = yalnız sil; `hwnd`: ileti penceresi
/// tutamacı): aynısı kayıtlıysa dokunmaz; değişenlerin ve kaydı
/// başarısız olanların (yeniden denenir) HEPSİ önce bırakılır, sonra yenileri kaydedilir; böylece
/// iki kısayol yer değiştirirken ikincisi birincinin tuşuna takılmaz. Başka program tuşu
/// tutuyorsa kayıt başarısız olur: günlüğe yazılır (yalnız ilk başarısızlıkta), `hatali_mi` doğru
/// döner (ipucunda uyarı).
pub fn ayarla_hepsi(hwnd: isize, istenen: &[(i32, Option<Kisayol>)]) {
    let hwnd = hwnd as HWND;
    let mut kayitli = KAYITLI.lock().unwrap_or_else(|e| e.into_inner());
    let plan = kisayol::plan(&kayitli, istenen, hatali_mi);
    for kimlik in &plan.birak {
        unsafe { UnregisterHotKey(hwnd, *kimlik) };
        kayitli.retain(|(k, _)| k != kimlik);
    }
    let onceki = HATALI.load(Ordering::Relaxed);
    for kimlik in &plan.degisen {
        HATALI.fetch_and(!(1 << kimlik), Ordering::Relaxed);
    }
    for (kimlik, yeni) in plan.kaydet {
        // SAFETY: hwnd ileti penceresi; çağıran ileti döngüsü iş parçacığıdır.
        let tamam =
            unsafe { RegisterHotKey(hwnd, kimlik, yeni.degistiriciler | MOD_NOREPEAT, yeni.tus) }
                != 0;
        if tamam {
            kayitli.push((kimlik, yeni));
            continue;
        }
        HATALI.fetch_or(1 << kimlik, Ordering::Relaxed);
        if onceki & (1 << kimlik) == 0 {
            logseq_tepsi_cekirdek::gunluk::yaz(&format!(
                "genel kisayol kaydedilemedi (kimlik {kimlik}, {}): hata {}; baska program ya da diger kisayol kullaniyor olabilir",
                kisayol::bicimle(&yeni),
                unsafe { GetLastError() }
            ));
        }
    }
}

/// Kimliğin son kayıt denemesi başarısız mı?
pub fn hatali_mi(kimlik: i32) -> bool {
    HATALI.load(Ordering::Relaxed) & (1 << kimlik) != 0
}

/// Herhangi bir kimliğin son kayıt denemesi başarısız mı (bakım zamanlayıcısı yeniden dener)?
pub fn hatali_var() -> bool {
    HATALI.load(Ordering::Relaxed) != 0
}

/// Çıkışta tüm kısayolları bırakır.
pub fn hepsini_birak(hwnd: isize) {
    let hwnd = hwnd as HWND;
    let mut kayitli = KAYITLI.lock().unwrap_or_else(|e| e.into_inner());
    for (kimlik, _) in kayitli.drain(..) {
        unsafe { UnregisterHotKey(hwnd, kimlik) };
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    // Gerçek sistem kısayollarıyla çakışmasın diye uç tuşlar: Ctrl+Alt+Shift+Win+F23 / F24
    // (iş parçacığı kısayolu: hwnd boş, yalnız bu sınama iş parçacığına bağlanır).
    fn bir(vk: u32) -> Kisayol {
        Kisayol {
            degistiriciler: kisayol::MOD_CONTROL
                | kisayol::MOD_ALT
                | kisayol::MOD_SHIFT
                | kisayol::MOD_WIN,
            tus: vk,
        }
    }

    fn kayitli_kisayol(kimlik: i32) -> Option<Kisayol> {
        let k = KAYITLI.lock().unwrap_or_else(|e| e.into_inner());
        k.iter().find(|(i, _)| *i == kimlik).map(|(_, s)| *s)
    }

    /// Süreç genelindeki durumla (tek `KAYITLI`) çalışan tek sınama: tüm aşamalar birlikte.
    #[test]
    fn takas_yeniden_deneme_ve_birakma_gercek_kayitla() {
        let (f23, f24) = (bir(0x86), bir(0x87));
        let h = 0;
        ayarla_hepsi(h, &[(1, Some(f23)), (2, Some(f24))]);
        assert!(!hatali_var());
        assert_eq!(
            (kayitli_kisayol(1), kayitli_kisayol(2)),
            (Some(f23), Some(f24))
        );
        // Takas: biri ötekinin tuşunu tutarken kaydedilemezdi.
        ayarla_hepsi(h, &[(1, Some(f24)), (2, Some(f23))]);
        assert!(!hatali_var(), "takas kayıt hatası vermemeli");
        assert_eq!(
            (kayitli_kisayol(1), kayitli_kisayol(2)),
            (Some(f24), Some(f23))
        );
        // Aynı tuşu ikinci kimliğe vermek başarısız olur; tuş bırakılınca yeniden deneme çalışır.
        ayarla_hepsi(h, &[(2, None)]);
        ayarla_hepsi(h, &[(2, Some(f24))]);
        assert!(hatali_mi(2) && hatali_var(), "tuş 1. kimlikte");
        ayarla_hepsi(h, &[(1, None)]);
        assert!(hatali_mi(2), "bırakma tek başına ikinciyi kaydetmez");
        ayarla_hepsi(h, &[(2, Some(f24))]);
        assert!(!hatali_var(), "hatalı kimlik yeniden denenince kaydolur");
        assert_eq!(kayitli_kisayol(2), Some(f24));
        ayarla_hepsi(h, &[(2, None)]);
        assert_eq!(kayitli_kisayol(2), None);
        hepsini_birak(h);
    }
}
