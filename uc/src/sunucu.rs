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

/// Sunucuyu başlatır. `durum`: `/durum` içeriği (önbelleklenir), `devret`: yetkili gizle/göster
/// işini pencere sahibine iletir (başarısızsa `false` → 503). Bağlanamazsa günlüğe yazar;
/// araç yine çalışır.
pub fn baslat(
    surum: &'static str,
    durum: fn() -> (usize, bool),
    devret: impl Fn(Is) -> bool + Send + 'static,
) {
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
        .spawn(move || dongu(&dinleyici, surum, durum, &devret));
    match sonuc {
        Ok(_) => gunluk::yaz(&format!("yerel uc: {adres} dinleniyor")),
        Err(e) => gunluk::yaz(&format!("yerel uc: is parcacigi baslamadi: {e}")),
    }
}

fn dongu(
    dinleyici: &TcpListener,
    surum: &str,
    durum: fn() -> (usize, bool),
    devret: &impl Fn(Is) -> bool,
) {
    for akim in dinleyici.incoming() {
        match akim {
            Ok(a) => baglanti(a, surum, durum, devret),
            Err(e) => {
                gunluk::yaz(&format!("yerel uc: accept hatasi: {e}"));
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

/// Başlık bloğu bitene, tavana ya da toplam zaman aşımına kadar okur.
fn oku(akim: &mut TcpStream) -> Result<Vec<u8>, std::io::ErrorKind> {
    let son = Instant::now() + Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS);
    let mut tampon: Vec<u8> = Vec::with_capacity(1024);
    let mut parca = [0u8; 1024];
    loop {
        if uc::baslik_sonu(&tampon).is_some() || tampon.len() >= uc::ISTEK_TAVANI {
            return Ok(tampon);
        }
        let kalan = son.saturating_duration_since(Instant::now());
        if kalan.is_zero() {
            return Err(std::io::ErrorKind::TimedOut);
        }
        akim.set_read_timeout(Some(kalan)).map_err(|e| e.kind())?;
        match akim.read(&mut parca) {
            Ok(0) => return Ok(tampon),
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

fn baglanti(
    mut akim: TcpStream,
    surum: &str,
    durum: fn() -> (usize, bool),
    devret: &impl Fn(Is) -> bool,
) {
    let yanit = match oku(&mut akim) {
        Err(std::io::ErrorKind::TimedOut) => uc::zaman_asimi_yaniti(),
        Err(_) => return,
        Ok(ham) => match uc::ayristir(&ham) {
            Err(h) => uc::ayris_hatasi_yaniti(h),
            Ok(istek) => uc::karar_ver(&istek, surum, || durum_onbellekli_ile(durum)),
        },
    };
    let yanit = pencere_isi_devret(yanit, devret);
    yanit_ver(akim, &yanit);
}

/// Yetkili `POST /gizle|/goster` ise işi `devret` ile pencere sahibine verir; verilemezse 503.
fn pencere_isi_devret(y: Yanit, devret: &impl Fn(Is) -> bool) -> Yanit {
    if y.is == Is::Yok || devret(y.is) {
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

    /// Yerel soketle gerçek bağlantı döngüsü: ham istek gönder, tüm yanıtı oku.
    /// `baglanti` yalnız ayrıştırma/zaman aşımı hatalarında (pencere işine varmadan) sınanır.
    fn yanit_al(gonder: &[u8], kapat_yazma: bool) -> String {
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let sunucu = std::thread::spawn(move || {
            let (akim, _) = dinleyici.accept().unwrap();
            baglanti(akim, "sinama", || (0, false), &|_| true);
        });
        let mut istemci = TcpStream::connect(adres).unwrap();
        istemci
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        istemci.write_all(gonder).unwrap();
        if kapat_yazma {
            istemci.shutdown(Shutdown::Write).unwrap();
        }
        let mut yanit = Vec::new();
        let _ = istemci.read_to_end(&mut yanit);
        sunucu.join().unwrap();
        String::from_utf8_lossy(&yanit).into_owned()
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
}
