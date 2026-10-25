//! Yerel uç TCP sunucusu: yalnız `127.0.0.1`, ayrı iş parçacığı, bloklayıcı `accept`
//! (boşta CPU sıfır). Pencere işleri burada YAPILMAZ: çağıranın verdiği `devret` işlevi
//! (ikili crate'te ileti penceresine `PostMessageW`) işi devralır. Bu iş parçacığında panik
//! olmamalı (`panic = "abort"`). Win32'ye bağımlı değildir.

use crate::onbellek::ZamanliOnbellek;
use crate::uc::{self, Is, Yanit};
use logseq_tepsi_cekirdek::gunluk;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// `/durum` sonucunun ömrü: sürekli yoklamada `EnumWindows` yükünü sınırlar.
const DURUM_ONBELLEK_MS: u64 = 500;

static DURUM_ONBELLEGI: Mutex<ZamanliOnbellek<(usize, bool)>> =
    Mutex::new(ZamanliOnbellek::yeni(DURUM_ONBELLEK_MS));

/// Sunucunun çağıran tarafından beslendiği veriler.
#[derive(Clone, Copy)]
pub struct Saglayici {
    pub surum: &'static str,
    /// `/durum` içeriği (önbelleklenir): (tepside gizli pencere sayısı, Logseq açık mı).
    pub durum: fn() -> (usize, bool),
    /// `GET /ayar` gövdesi (`anahtar=değer` satırları).
    pub ayar_metni: fn() -> String,
}

/// Başlangıçtan bu yana ms (tekdüze saat; önbellek bunu `simdi_ms` olarak alır).
fn simdi_ms() -> u64 {
    static BASLANGIC: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    BASLANGIC.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// `uret` sonucu, `DURUM_ONBELLEK_MS` içinde yeniden hesaplanmaz.
fn durum_onbellekli_ile(uret: impl FnOnce() -> (usize, bool)) -> (usize, bool) {
    let mut o = DURUM_ONBELLEGI.lock().unwrap_or_else(|e| e.into_inner());
    o.al(simdi_ms(), uret)
}

/// Sunucuyu başlatır. `devret`: yetkili gizle/göster/ayar işini pencere sahibine iletir
/// (başarısızsa `false` → 503). Bağlanamazsa günlüğe yazar; araç yine çalışır.
pub fn baslat(sag: Saglayici, devret: impl Fn(&Is) -> bool + Send + 'static) {
    let adres = SocketAddrV4::new(Ipv4Addr::LOCALHOST, uc::PORT);
    let dinleyici = match TcpListener::bind(adres) {
        Ok(d) => d,
        Err(e) => {
            gunluk::yaz(&format!("yerel uc: {adres} dinlenemedi: {e}"));
            return;
        }
    };
    let sonuc = std::thread::Builder::new()
        .name("yerel-uc".into())
        .spawn(move || dongu(&dinleyici, sag, &devret));
    match sonuc {
        Ok(_) => gunluk::yaz(&format!("yerel uc: {adres} dinleniyor")),
        Err(e) => gunluk::yaz(&format!("yerel uc: is parcacigi baslamadi: {e}")),
    }
}

fn dongu(dinleyici: &TcpListener, sag: Saglayici, devret: &impl Fn(&Is) -> bool) {
    for akim in dinleyici.incoming() {
        match akim {
            Ok(a) => baglanti(a, sag, devret),
            Err(e) => {
                gunluk::yaz(&format!("yerel uc: accept hatasi: {e}"));
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

/// `tampon`a, `tamam` olana, akış kapanana ya da `son` anına kadar okur.
fn oku_kadar(
    akim: &mut TcpStream,
    tampon: &mut Vec<u8>,
    son: Instant,
    tamam: impl Fn(&[u8]) -> bool,
) -> Result<(), std::io::ErrorKind> {
    let mut parca = [0u8; 1024];
    loop {
        if tamam(tampon) {
            return Ok(());
        }
        let kalan = son.saturating_duration_since(Instant::now());
        if kalan.is_zero() {
            return Err(std::io::ErrorKind::TimedOut);
        }
        akim.set_read_timeout(Some(kalan)).map_err(|e| e.kind())?;
        match akim.read(&mut parca) {
            Ok(0) => return Ok(()),
            Ok(n) => tampon.extend_from_slice(&parca[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => {
                return Err(match e.kind() {
                    std::io::ErrorKind::WouldBlock => std::io::ErrorKind::TimedOut,
                    k => k,
                })
            }
        }
    }
}

/// Başlık bloğu bitene, tavana ya da toplam zaman aşımına kadar okur; okuma bitiş anını da
/// verir (gövde okuması aynı toplam süreyi paylaşır).
fn oku(akim: &mut TcpStream) -> Result<(Vec<u8>, Instant), std::io::ErrorKind> {
    let son = Instant::now() + Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS);
    let mut tampon: Vec<u8> = Vec::with_capacity(1024);
    oku_kadar(akim, &mut tampon, son, |t| {
        uc::baslik_sonu(t).is_some() || t.len() >= uc::ISTEK_TAVANI
    })?;
    Ok((tampon, son))
}

/// `POST /ayar` gövdesi: başlıkla birlikte okunmuş baytlar + kalanı, `Content-Length` kadar.
fn govde_oku(
    akim: &mut TcpStream,
    istek: &uc::Istek,
    ham: &[u8],
    son: Instant,
) -> Result<Vec<u8>, Yanit> {
    let uzunluk = uc::govde_uzunlugu(istek)?;
    let basla = uc::baslik_sonu(ham).unwrap_or(ham.len());
    let mut govde = ham[basla..].to_vec();
    oku_kadar(akim, &mut govde, son, |t| t.len() >= uzunluk).map_err(|k| match k {
        std::io::ErrorKind::TimedOut => uc::zaman_asimi_yaniti(),
        _ => uc::ayris_hatasi_yaniti(uc::AyrisHatasi::Bozuk),
    })?;
    if govde.len() < uzunluk {
        return Err(uc::ayris_hatasi_yaniti(uc::AyrisHatasi::Bozuk));
    }
    govde.truncate(uzunluk);
    Ok(govde)
}

fn yanit_ver(mut akim: TcpStream, y: &Yanit) {
    let _ = akim.set_write_timeout(Some(Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS)));
    let _ = akim.write_all(&uc::yanit_baytlari(y));
    let _ = akim.flush();
    // Okunmamış gövde varken kapatmak RST üretir; yanıt istemciye ulaşsın diye kısaca boşalt.
    let _ = akim.shutdown(Shutdown::Write);
    let _ = akim.set_read_timeout(Some(Duration::from_millis(50)));
    let mut cop = [0u8; 512];
    let _ = akim.read(&mut cop);
}

/// İsteği okur ve kararı verir; `None`: yanıtsız kapat (okuma hatası).
fn yanit_hazirla(akim: &mut TcpStream, sag: Saglayici) -> Option<Yanit> {
    let (ham, son) = match oku(akim) {
        Err(std::io::ErrorKind::TimedOut) => return Some(uc::zaman_asimi_yaniti()),
        Err(_) => return None,
        Ok(o) => o,
    };
    let istek = match uc::ayristir(&ham) {
        Err(h) => return Some(uc::ayris_hatasi_yaniti(h)),
        Ok(i) => i,
    };
    // Gövde yalnız yetkili `POST /ayar` için okunur; yetkisizde karar_ver 403 verir.
    let govde = if uc::govde_gerekli_mi(&istek) && uc::yetkili_mi(&istek) {
        match govde_oku(akim, &istek, &ham, son) {
            Ok(g) => g,
            Err(y) => return Some(y),
        }
    } else {
        Vec::new()
    };
    Some(uc::karar_ver(
        &istek,
        &govde,
        sag.surum,
        || durum_onbellekli_ile(sag.durum),
        sag.ayar_metni,
    ))
}

fn baglanti(mut akim: TcpStream, sag: Saglayici, devret: &impl Fn(&Is) -> bool) {
    let Some(yanit) = yanit_hazirla(&mut akim, sag) else {
        return;
    };
    let yanit = pencere_isi_devret(yanit, devret);
    yanit_ver(akim, &yanit);
}

/// Yetkili `POST /gizle|/goster|/ayar` ise işi `devret` ile pencere sahibine verir; verilemezse 503.
fn pencere_isi_devret(y: Yanit, devret: &impl Fn(&Is) -> bool) -> Yanit {
    if y.is == Is::Yok || devret(&y.is) {
        y
    } else {
        gunluk::yaz("yerel uc: is pencere sahibine devredilemedi");
        Yanit {
            kod: 503,
            govde: "503 Service Unavailable\n".into(),
            basliklar: Vec::new(),
            is: Is::Yok,
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use logseq_tepsi_cekirdek::ayar::{AyarDegeri, Kucultme, Tik};

    const SINAMA: Saglayici = Saglayici {
        surum: "sinama",
        durum: || (0, false),
        ayar_metni: || "tik=tek\n".into(),
    };

    /// Yerel soketle gerçek bağlantı döngüsü: ham istek parçalarını (aralarında kısa bekleme)
    /// gönder, tüm yanıtı oku; sunucuya devredilen işleri de döndürür.
    fn yanit_al_parcali(
        parcalar: &[&[u8]],
        kapat_yazma: bool,
        devret_sonucu: bool,
    ) -> (String, Vec<Is>) {
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let devredilen = std::sync::Arc::new(Mutex::new(Vec::new()));
        let kayit = devredilen.clone();
        let sunucu = std::thread::spawn(move || {
            let (akim, _) = dinleyici.accept().unwrap();
            baglanti(akim, SINAMA, &|is: &Is| {
                kayit.lock().unwrap().push(is.clone());
                devret_sonucu
            });
        });
        let mut istemci = TcpStream::connect(adres).unwrap();
        istemci
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        for (i, p) in parcalar.iter().enumerate() {
            if i > 0 {
                std::thread::sleep(Duration::from_millis(60));
            }
            istemci.write_all(p).unwrap();
        }
        if kapat_yazma {
            istemci.shutdown(Shutdown::Write).unwrap();
        }
        let mut yanit = Vec::new();
        let _ = istemci.read_to_end(&mut yanit);
        sunucu.join().unwrap();
        let is = devredilen.lock().unwrap().clone();
        (String::from_utf8_lossy(&yanit).into_owned(), is)
    }

    fn yanit_al(gonder: &[u8], kapat_yazma: bool) -> String {
        yanit_al_parcali(&[gonder], kapat_yazma, true).0
    }

    #[test]
    fn tavani_asan_baslik_431() {
        let mut ham = b"GET /durum HTTP/1.1\r\nX-Dolgu: ".to_vec();
        ham.resize(uc::ISTEK_TAVANI + 512, b'a');
        let y = yanit_al(&ham, false);
        assert!(
            y.starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n"),
            "{y}"
        );
    }

    #[test]
    fn bozuk_istek_400() {
        let y = yanit_al(b"bu http degil\r\n\r\n", false);
        assert!(y.starts_with("HTTP/1.1 400 Bad Request\r\n"), "{y}");
    }

    #[test]
    fn bitmeyen_istek_zaman_asiminda_408() {
        // Başlık bloğu hiç bitmez, istemci de kapatmaz: toplam okuma süresi dolunca 408.
        let basla = Instant::now();
        let y = yanit_al(b"GET /durum HTTP/1.1\r\nHost: 127.0.0.1", false);
        assert!(y.starts_with("HTTP/1.1 408 Request Timeout\r\n"), "{y}");
        assert!(basla.elapsed() >= Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS - 100));
    }

    #[test]
    fn durum_onbellegi_ardisik_isteklerde_taramayi_sinirlar() {
        // 20 ardışık istek ≪ 500 ms: üretici (EnumWindows) en çok bir kez (saat sınırı
        // arada geçilirse iki kez) çalışır; önbelleksiz 20 olurdu.
        let sayac = std::cell::Cell::new(0);
        for _ in 0..20 {
            let d = durum_onbellekli_ile(|| {
                sayac.set(sayac.get() + 1);
                (3, true)
            });
            assert_eq!(d, (3, true));
        }
        assert!(sayac.get() <= 2, "üretici {} kez çağrıldı", sayac.get());
    }

    const BASLIK: &str = "POST /ayar HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\n";

    fn ayar_istegi(govde: &str) -> Vec<u8> {
        format!("{BASLIK}Content-Length: {}\r\n\r\n{govde}", govde.len()).into_bytes()
    }

    #[test]
    fn post_ayar_govde_ayni_pakette_devredilir_204() {
        let (y, is) = yanit_al_parcali(
            &[&ayar_istegi("tik=cift\nkucultme=gorev_cubugu\n")],
            false,
            true,
        );
        assert!(y.starts_with("HTTP/1.1 204 No Content\r\n"), "{y}");
        assert_eq!(
            is,
            vec![Is::Ayar(vec![
                AyarDegeri::Tik(Tik::Cift),
                AyarDegeri::Kucultme(Kucultme::GorevCubugu),
            ])]
        );
    }

    #[test]
    fn post_ayar_govde_ayri_pakette_gelince_beklenir() {
        let tam = ayar_istegi("kapaliyken_tik_acar=hayir\n");
        let (baslik, govde) = tam.split_at(tam.len() - 10);
        let (y, is) = yanit_al_parcali(&[baslik, govde], false, true);
        assert!(y.starts_with("HTTP/1.1 204"), "{y}");
        assert_eq!(
            is,
            vec![Is::Ayar(vec![AyarDegeri::KapaliykenTikAcar(false)])]
        );
    }

    #[test]
    fn post_ayar_gecersiz_deger_400_nedenli_devredilmez() {
        let (y, is) = yanit_al_parcali(&[&ayar_istegi("tik=uc\n")], false, true);
        assert!(y.starts_with("HTTP/1.1 400 Bad Request\r\n"), "{y}");
        assert!(y.contains("tik: tek ya da cift olmalı"), "{y}");
        assert!(is.is_empty(), "geçersiz ayar devredilmemeli");
        let (y, is) = yanit_al_parcali(&[&ayar_istegi("tik=cift\nbilinmeyen=1\n")], false, true);
        assert!(y.starts_with("HTTP/1.1 400"), "{y}");
        assert!(is.is_empty(), "kısmi uygulama yok");
    }

    #[test]
    fn post_ayar_content_length_kurallari() {
        let yok = format!("{BASLIK}\r\ntik=cift");
        assert!(yanit_al(yok.as_bytes(), false).starts_with("HTTP/1.1 411 Length Required\r\n"));
        let kotu = format!("{BASLIK}Content-Length: abc\r\n\r\n");
        assert!(yanit_al(kotu.as_bytes(), false).starts_with("HTTP/1.1 400"));
        let buyuk = format!("{BASLIK}Content-Length: {}\r\n\r\n", uc::GOVDE_TAVANI + 1);
        let (y, is) = yanit_al_parcali(&[buyuk.as_bytes()], false, true);
        assert!(y.starts_with("HTTP/1.1 413 Content Too Large\r\n"), "{y}");
        assert!(is.is_empty());
        // Tavan sınırı: tam GOVDE_TAVANI kabul (geçerli satır + boşluk dolgusu).
        let govde = format!("tik=cift\n{}", " ".repeat(uc::GOVDE_TAVANI - 9));
        let (y, _) = yanit_al_parcali(&[&ayar_istegi(&govde)], false, true);
        assert!(y.starts_with("HTTP/1.1 204"), "{y}");
    }

    #[test]
    fn post_ayar_eksik_govde_zaman_asiminda_408() {
        let istek = format!("{BASLIK}Content-Length: 50\r\n\r\ntik=c");
        let basla = Instant::now();
        let y = yanit_al(istek.as_bytes(), false);
        assert!(y.starts_with("HTTP/1.1 408 Request Timeout\r\n"), "{y}");
        assert!(basla.elapsed() >= Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS - 100));
        // İstemci gövdeyi yarıda keserse (kapatırsa) 400.
        let y = yanit_al(istek.as_bytes(), true);
        assert!(y.starts_with("HTTP/1.1 400"), "{y}");
    }

    #[test]
    fn post_ayar_yetkisiz_govde_okunmadan_403() {
        // Başlık yok: gövde gelmese de (408 beklemeden) hemen 403.
        let basla = Instant::now();
        let istek = "POST /ayar HTTP/1.1\r\nContent-Length: 50\r\n\r\n";
        let y = yanit_al(istek.as_bytes(), false);
        assert!(y.starts_with("HTTP/1.1 403 Forbidden\r\n"), "{y}");
        assert!(basla.elapsed() < Duration::from_millis(1500));
        // Yabancı Origin de aynı.
        let istek =
            format!("{BASLIK}Origin: https://evil.example\r\nContent-Length: 8\r\n\r\ntik=cift");
        let (y, is) = yanit_al_parcali(&[istek.as_bytes()], false, true);
        assert!(y.starts_with("HTTP/1.1 403"), "{y}");
        assert!(is.is_empty());
    }

    #[test]
    fn post_ayar_devredilemezse_503() {
        let (y, is) = yanit_al_parcali(&[&ayar_istegi("tik=cift\n")], false, false);
        assert!(y.starts_with("HTTP/1.1 503"), "{y}");
        assert_eq!(is.len(), 1);
    }

    #[test]
    fn get_ayar_saglayicidan_duz_metin() {
        let istek = "GET /ayar HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\n\r\n";
        let y = yanit_al(istek.as_bytes(), false);
        assert!(y.starts_with("HTTP/1.1 200 OK\r\n"), "{y}");
        assert!(y.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(y.ends_with("\r\n\r\ntik=tek\n"), "{y}");
    }
}
