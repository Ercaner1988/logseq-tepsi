//! Yerel uç TCP sunucusu: yalnız `127.0.0.1`, ayrı iş parçacığı, bloklayıcı `accept`
//! (boşta CPU sıfır). Pencere işleri burada YAPILMAZ: çağıranın verdiği `devret` işlevi
//! (ikili crate'te ileti penceresine `PostMessageW`) işi devralır. Bu iş parçacığında panik
//! olmamalı (`panic = "abort"`). Win32'ye bağımlı değildir.

use crate::onbellek::ZamanliOnbellek;
use crate::uc::{self, Devir, Is, Yanit};
use logseq_tepsi_cekirdek::gunluk;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// `/durum` sonucunun ömrü: sürekli yoklamada `EnumWindows` yükünü sınırlar.
const DURUM_ONBELLEK_MS: u64 = 500;

static DURUM_ONBELLEGI: Mutex<ZamanliOnbellek<(usize, bool)>> =
    Mutex::new(ZamanliOnbellek::yeni(DURUM_ONBELLEK_MS));

/// `api=` yoklamasının ömrü (TCP bağlantısı; sürekli yoklamada her istekte yapılmasın).
const API_ONBELLEK_MS: u64 = 1_000;

static API_ONBELLEGI: Mutex<ZamanliOnbellek<bool>> =
    Mutex::new(ZamanliOnbellek::yeni(API_ONBELLEK_MS));

/// Eklentinin (izinli Origin ile yetkili istek) araca son ulaştığı an (unix ms, 0 = hiç). Logseq
/// kapanınca sıfırlanır; ayrıca zaman damgası, bayat işaretin YENİ Logseq sürecinde hazır
/// sayılmasını önler (`not::eklenti_hazir_mi`).
static EKLENTI_SON_MS: AtomicU64 = AtomicU64::new(0);

pub fn eklenti_goruldu_mu() -> bool {
    EKLENTI_SON_MS.load(Ordering::Relaxed) != 0
}

/// Eklentinin son teması (unix ms); hiç ulaşmadıysa 0.
pub fn eklenti_son_ms() -> u64 {
    EKLENTI_SON_MS.load(Ordering::Relaxed)
}

pub fn eklenti_goruldu_sifirla() {
    EKLENTI_SON_MS.store(0, Ordering::Relaxed);
}

fn simdi_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_millis() as u64)
        .max(1)
}

/// Sunucunun çağıran tarafından beslendiği veriler.
#[derive(Clone, Copy)]
pub struct Saglayici {
    pub surum: &'static str,
    /// `/durum` içeriği (önbelleklenir): (tepside gizli pencere sayısı, Logseq açık mı).
    pub durum: fn() -> (usize, bool),
    /// `GET /ayar` gövdesi (`anahtar=değer` satırları).
    pub ayar_metni: fn() -> String,
    /// Logseq API sunucusu çalışıyor mu (`/durum` `api=` satırı; önbelleklenir).
    pub api_acik: fn() -> bool,
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

/// `uret` sonucu (API sunucusu yoklaması), `API_ONBELLEK_MS` içinde yeniden yapılmaz.
fn api_onbellekli_ile(uret: impl FnOnce() -> bool) -> bool {
    let mut o = API_ONBELLEGI.lock().unwrap_or_else(|e| e.into_inner());
    o.al(simdi_ms(), uret)
}

/// Sunucuyu başlatır. `devret`: yetkili gizle/göster/ayar işini pencere sahibine iletir
/// (`Devir::Basarisiz` → 503; `Devir::Govde` → işin sonucu 200 gövdesinde;
/// `Devir::Red` → 400 + neden). Bağlanamazsa günlüğe yazar; araç yine çalışır.
pub fn baslat(sag: Saglayici, devret: impl Fn(&Is) -> Devir + Send + Sync + 'static) {
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
        .spawn(move || {
            dongu(
                &dinleyici,
                sag,
                Arc::new(devret),
                Arc::new(AtomicUsize::new(0)),
                BAGLANTI_TAVANI,
            )
        });
    match sonuc {
        Ok(_) => gunluk::yaz(&format!("yerel uc: {adres} dinleniyor")),
        Err(e) => gunluk::yaz(&format!("yerel uc: is parcacigi baslamadi: {e}")),
    }
}

/// Aynı anda en çok bu kadar bağlantı işlenir: yavaş ya da asılı bir istemci (2 sn okuma süresi)
/// diğerlerini bekletmez, ama iş parçacığı sayısı da sınırlı kalır; fazlasına hemen 503.
const BAGLANTI_TAVANI: usize = 8;

/// Açık bağlantı yeri; bırakılınca (bağlantı iş parçacığı bitince) sayaç düşer.
struct Yer(Arc<AtomicUsize>);

impl Drop for Yer {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn yer_al(sayac: &Arc<AtomicUsize>, tavan: usize) -> Option<Yer> {
    if sayac.fetch_add(1, Ordering::Relaxed) >= tavan {
        sayac.fetch_sub(1, Ordering::Relaxed);
        return None;
    }
    Some(Yer(sayac.clone()))
}

/// Kabul döngüsü: her bağlantı kendi iş parçacığında (en çok `tavan`) işlenir.
fn dongu<D: Fn(&Is) -> Devir + Send + Sync + 'static>(
    dinleyici: &TcpListener,
    sag: Saglayici,
    devret: Arc<D>,
    sayac: Arc<AtomicUsize>,
    tavan: usize,
) {
    for akim in dinleyici.incoming() {
        let a = match akim {
            Ok(a) => a,
            Err(e) => {
                gunluk::yaz(&format!("yerel uc: accept hatasi: {e}"));
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
        };
        let Some(yer) = yer_al(&sayac, tavan) else {
            gunluk::yaz("yerel uc: es zamanli baglanti tavani asildi, 503");
            yanit_ver(a, &hizmet_yok());
            continue;
        };
        let devret = devret.clone();
        let sonuc = std::thread::Builder::new()
            .name("yerel-uc-baglanti".into())
            .spawn(move || {
                let _yer = yer;
                baglanti(a, sag, &*devret);
            });
        if let Err(e) = sonuc {
            gunluk::yaz(&format!("yerel uc: baglanti is parcacigi baslamadi: {e}"));
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

#[allow(
    clippy::let_underscore_must_use,
    reason = "yanıt sonrası en iyi çaba: istemci bağlantıyı kapatmışsa bildirilecek kimse yok, sunucu döngüsü sürmeli"
)]
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
    let mut yanit = uc::karar_ver(
        &istek,
        &govde,
        sag.surum,
        || durum_onbellekli_ile(sag.durum),
        sag.ayar_metni,
    );
    if yanit.kod < 300 && uc::eklenti_istegi_mi(&istek) {
        EKLENTI_SON_MS.store(simdi_unix_ms(), Ordering::Relaxed);
    }
    uc::durum_yanitina_api_ekle(&istek, &mut yanit, || api_onbellekli_ile(sag.api_acik));
    Some(yanit)
}

fn baglanti(mut akim: TcpStream, sag: Saglayici, devret: &impl Fn(&Is) -> Devir) {
    let Some(yanit) = yanit_hazirla(&mut akim, sag) else {
        return;
    };
    let yanit = pencere_isi_devret(yanit, devret);
    yanit_ver(akim, &yanit);
}

fn hizmet_yok() -> Yanit {
    Yanit {
        kod: 503,
        govde: "503 Service Unavailable\n".into(),
        basliklar: Vec::new(),
        is: Is::Yok,
    }
}

/// Yetkili `POST /gizle|/goster|/ayar` ise işi `devret` ile pencere sahibine verir; verilemezse (ya
/// da zamanında işlenmezse) 503. İş sonuç gövdesi döndürdüyse yanıt 200 + gövde olur.
fn pencere_isi_devret(y: Yanit, devret: &impl Fn(&Is) -> Devir) -> Yanit {
    if y.is == Is::Yok {
        return y;
    }
    match devret(&y.is) {
        Devir::Tamam => y,
        Devir::Govde(govde) => Yanit {
            kod: 200,
            govde,
            is: Is::Yok,
            ..y
        },
        Devir::Red(neden) => Yanit {
            kod: 400,
            govde: format!("400 Bad Request\n{neden}\n"),
            is: Is::Yok,
            ..y
        },
        Devir::Basarisiz => {
            gunluk::yaz("yerel uc: is pencere sahibine devredilemedi");
            hizmet_yok()
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
        api_acik: || false,
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
                if devret_sonucu {
                    Devir::Tamam
                } else {
                    Devir::Basarisiz
                }
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
    fn post_ayar_devir_sonucu_govdede_200_doner() {
        // Pencere sahibi işi eşzamanlı uyguladı ve sonucu bildirdi: yanıt 204 değil, 200 + gövde.
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let sunucu = std::thread::spawn(move || {
            let (akim, _) = dinleyici.accept().unwrap();
            baglanti(akim, SINAMA, &|_: &Is| {
                Devir::Govde("api_otomatik_baslat=beklemede\n".into())
            });
        });
        let mut istemci = TcpStream::connect(adres).unwrap();
        istemci
            .write_all(&ayar_istegi("api_otomatik_baslat=evet\n"))
            .unwrap();
        let mut yanit = String::new();
        istemci.read_to_string(&mut yanit).unwrap();
        sunucu.join().unwrap();
        assert!(yanit.starts_with("HTTP/1.1 200 OK\r\n"), "{yanit}");
        assert!(yanit.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(
            yanit.ends_with("\r\n\r\napi_otomatik_baslat=beklemede\n"),
            "{yanit}"
        );
    }

    #[test]
    fn post_ayar_kisayol_cakismasi_400_takas_204() {
        // Varsayılan ayar: göster/gizle Ctrl+Alt+L, not Ctrl+Alt+N (bu sınama ikilisinde genel
        // durum hiç değiştirilmez).
        let (y, is) = yanit_al_parcali(&[&ayar_istegi("kisayol_not=Ctrl+Alt+L\n")], false, true);
        assert!(y.starts_with("HTTP/1.1 400"), "{y}");
        assert!(y.contains("aynı kısayol olamaz"), "{y}");
        assert!(is.is_empty(), "çakışan ayar devredilmemeli");
        let takas = "kisayol_goster_gizle=Ctrl+Alt+N\nkisayol_not=Ctrl+Alt+L\n";
        let (y, is) = yanit_al_parcali(&[&ayar_istegi(takas)], false, true);
        assert!(y.starts_with("HTTP/1.1 204"), "{y}");
        assert_eq!(is.len(), 1);
    }

    /// Doğrulama ile uygulama arasında durum değişti (iki eşzamanlı POST): pencere sahibi reddeder,
    /// 204 değil 400 + neden döner (eklenti 400'de araçtaki gerçek durumla eşitler).
    #[test]
    fn post_ayar_pencere_sahibi_reddederse_400_nedenli() {
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let sunucu = std::thread::spawn(move || {
            let (akim, _) = dinleyici.accept().unwrap();
            baglanti(akim, SINAMA, &|_: &Is| {
                Devir::Red("kisayol_not aynı kısayol olamaz".into())
            });
        });
        let mut istemci = TcpStream::connect(adres).unwrap();
        istemci.write_all(&ayar_istegi("tik=tek\n")).unwrap();
        let mut yanit = String::new();
        istemci.read_to_string(&mut yanit).unwrap();
        sunucu.join().unwrap();
        assert!(yanit.starts_with("HTTP/1.1 400 Bad Request\r\n"), "{yanit}");
        assert!(
            yanit.ends_with("\r\n\r\n400 Bad Request\nkisayol_not aynı kısayol olamaz\n"),
            "{yanit}"
        );
    }

    /// `dongu`yu gerçek bir dinleyicide koşturur; adresi döndürür.
    fn dongu_baslat(tavan: usize) -> std::net::SocketAddr {
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        std::thread::spawn(move || {
            dongu(
                &dinleyici,
                SINAMA,
                Arc::new(|_: &Is| Devir::Tamam),
                Arc::new(AtomicUsize::new(0)),
                tavan,
            )
        });
        adres
    }

    fn durum_iste(adres: std::net::SocketAddr) -> String {
        let mut c = TcpStream::connect(adres).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let istek = "GET /ayar HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\n\r\n";
        c.write_all(istek.as_bytes()).unwrap();
        let mut y = String::new();
        let _ = c.read_to_string(&mut y);
        y
    }

    #[test]
    fn asili_istemci_digerlerini_bekletmez() {
        let adres = dongu_baslat(8);
        // Asılı istemci: bağlanır, hiçbir şey göndermez (sunucu 2 sn okuma süresi tutar).
        let _asili = TcpStream::connect(adres).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let basla = Instant::now();
        let y = durum_iste(adres);
        assert!(y.starts_with("HTTP/1.1 200 OK"), "{y}");
        assert!(
            basla.elapsed() < Duration::from_millis(1000),
            "ikinci istek asılı olanı beklemiş: {:?}",
            basla.elapsed()
        );
    }

    #[test]
    fn es_zamanli_baglanti_tavani_asilirsa_hemen_503() {
        let adres = dongu_baslat(1);
        let _asili = TcpStream::connect(adres).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let basla = Instant::now();
        let y = durum_iste(adres);
        assert!(y.starts_with("HTTP/1.1 503 Service Unavailable"), "{y}");
        assert!(basla.elapsed() < Duration::from_millis(1000));
    }

    #[test]
    fn yer_alinir_birakilinca_serbest_kalir() {
        let sayac = Arc::new(AtomicUsize::new(0));
        let a = yer_al(&sayac, 2).unwrap();
        let b = yer_al(&sayac, 2).unwrap();
        assert!(yer_al(&sayac, 2).is_none(), "tavan 2");
        assert_eq!(sayac.load(Ordering::Relaxed), 2, "reddedilen yer sayılmaz");
        drop(a);
        let c = yer_al(&sayac, 2);
        assert!(c.is_some(), "bırakılan yer yeniden alınabilir");
        drop((b, c));
        assert_eq!(sayac.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn post_ayar_devredilemezse_503() {
        let (y, is) = yanit_al_parcali(&[&ayar_istegi("tik=cift\n")], false, false);
        assert!(y.starts_with("HTTP/1.1 503"), "{y}");
        assert_eq!(is.len(), 1);
    }

    #[test]
    fn api_yoklamasi_onbellekli() {
        let sayac = std::cell::Cell::new(0);
        for _ in 0..20 {
            let d = api_onbellekli_ile(|| {
                sayac.set(sayac.get() + 1);
                false
            });
            assert!(!d);
        }
        assert!(sayac.get() <= 2, "yoklama {} kez yapıldı", sayac.get());
    }

    #[test]
    fn durum_yaniti_api_satirini_tasir_eklenti_istegi_bayragi_yalniz_origin_ile() {
        // Bayrağa dokunan tek sınama (paralel koşuda çakışmasın diye iki aşama birlikte).
        eklenti_goruldu_sifirla();
        let curl = "GET /durum HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\n\r\n";
        let y = yanit_al(curl.as_bytes(), false);
        // Önbellek süreç geneli (öbür sınamalar `(3, true)` yazmış olabilir): yalnız biçim.
        let govde = y.split("\r\n\r\n").nth(1).unwrap();
        let satirlar: Vec<&str> = govde.lines().collect();
        assert_eq!(satirlar.len(), 4, "{y}");
        assert_eq!(satirlar[0], "logseq-tepsi sinama");
        assert!(satirlar[1].starts_with("gizli=") && satirlar[2].starts_with("logseq="));
        assert_eq!(satirlar[3], "api=kapali");
        assert!(
            !eklenti_goruldu_mu(),
            "Origin'siz (curl) istek eklenti sayılmaz"
        );
        let yetkisiz =
            "GET /durum HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nOrigin: lsp://logseq.io\r\n\r\n";
        let y = yanit_al(yetkisiz.as_bytes(), false);
        assert!(y.starts_with("HTTP/1.1 403"), "{y}");
        assert!(!eklenti_goruldu_mu(), "yetkisiz istek bayrağı kurmaz");
        let eklenti = "GET /ayar HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\nOrigin: lsp://logseq.io\r\n\r\n";
        let y = yanit_al(eklenti.as_bytes(), false);
        assert!(y.starts_with("HTTP/1.1 200"), "{y}");
        assert!(
            eklenti_goruldu_mu(),
            "izinli Origin + yetki eklenti sayılır"
        );
        assert!(
            eklenti_son_ms() > 1_700_000_000_000,
            "temas zamanı unix ms olarak tutulur: {}",
            eklenti_son_ms()
        );
        eklenti_goruldu_sifirla();
        assert!(!eklenti_goruldu_mu());
        assert_eq!(eklenti_son_ms(), 0);
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
