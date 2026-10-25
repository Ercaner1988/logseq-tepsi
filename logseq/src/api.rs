//! Logseq HTTP API sunucusu: "çalışıyor mu" yoklaması (kısa TCP bağlantısı) ve
//! `api_otomatik_baslat` ayarının nasıl uygulanacağı kararı (yazma YALNIZ Logseq kapalıyken).

use crate::yapilandirma::{self, Yazim};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Logseq'in varsayılan API sunucusu adresi (`:server/host`, `:server/port` varsayılanları).
pub const ADRES: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12315);
const YOKLAMA_ZAMANI: Duration = Duration::from_millis(300);

/// `adres`e kısa TCP bağlantısı kurulabiliyor mu (sınama için adres verilebilir).
pub fn acik_mi_adres(adres: SocketAddr, zaman: Duration) -> bool {
    TcpStream::connect_timeout(&adres, zaman).is_ok()
}

/// Varsayılan API sunucusu çalışıyor mu.
pub fn acik_mi() -> bool {
    acik_mi_adres(SocketAddr::V4(ADRES), YOKLAMA_ZAMANI)
}

/// Son yoklamanın sonucu (arka plan yoklayıcısı yazar; menü ve benzeri çizim yolları bunu okur,
/// 300 ms'lik TCP yoklamasıyla pencere iş parçacığını bekletmez).
static SON_ACIK: AtomicBool = AtomicBool::new(false);

/// Arka plan yoklayıcısının son bildirdiği API sunucusu durumu (en çok `YOKLAMA_ARALIGI` eski).
pub fn son_durum() -> bool {
    SON_ACIK.load(Ordering::Relaxed)
}

/// Arka plan yoklama aralığı.
pub const YOKLAMA_ARALIGI: Duration = Duration::from_secs(2);

/// `etkin()` iken her `aralik`te `adres`i yoklar ve sonucu `hedef`e yazar (etkin değilken yoklamaz,
/// sonuç `false`); `durdur` kurulunca biter. Yoklama arasında uyur (boşta CPU yok denecek kadar az).
fn yokla_dongusu(
    adres: SocketAddr,
    aralik: Duration,
    etkin: impl Fn() -> bool,
    hedef: &AtomicBool,
    durdur: &AtomicBool,
) {
    while !durdur.load(Ordering::Relaxed) {
        let acik = etkin() && acik_mi_adres(adres, YOKLAMA_ZAMANI);
        hedef.store(acik, Ordering::Relaxed);
        std::thread::sleep(aralik);
    }
}

/// Arka plan yoklayıcısını başlatır (`etkin`: Logseq açık mı; kapalıyken bağlanılmaz). İş
/// parçacığı süreç boyunca yaşar.
pub fn yoklayici_baslat(etkin: impl Fn() -> bool + Send + 'static) {
    static DURDURMA: AtomicBool = AtomicBool::new(false);
    let adres = SocketAddr::V4(ADRES);
    let _ = std::thread::Builder::new()
        .name("api-yoklama".into())
        .spawn(move || yokla_dongusu(adres, YOKLAMA_ARALIGI, etkin, &SON_ACIK, &DURDURMA));
}

/// `api_otomatik_baslat` isteğinin sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Karar {
    Yok,
    /// Logseq kapalı: `configs.edn`e şimdi yaz.
    Yaz(bool),
    /// Logseq açık: bekleyen olarak sakla, Logseq kapanınca uygula.
    Beklet(bool),
    /// İstenen zaten gerçek değer: bekleyen varsa sil.
    BekleyeniSil,
}

/// Kullanıcı `istenen`i seçti. `gercek`: `configs.edn`deki değer; `bekleyen`: ayar.toml'daki.
pub fn karar(istenen: bool, gercek: bool, bekleyen: Option<bool>, logseq_acik: bool) -> Karar {
    if istenen == gercek {
        if bekleyen.is_some() {
            Karar::BekleyeniSil
        } else {
            Karar::Yok
        }
    } else if logseq_acik {
        Karar::Beklet(istenen)
    } else {
        Karar::Yaz(istenen)
    }
}

/// Logseq ana penceresi bu kadar ardışık durum yoklamasında (2 sn aralık) görünmediyse ve
/// `Logseq.exe` süreci de kalmadıysa kapandı sayılır: pencere kaybolduktan en erken ~2, en geç ~4 sn
/// sonra (kapanırken dosyaya hâlâ yazıyor olabilir).
pub const KAPALI_YOKLAMA_SAYISI: u32 = 2;

/// Logseq gerçekten kapandı mı: ana pencere yok, `Logseq.exe` süreci yok ve pencere
/// `kapali_yoklama` ardışık yoklamadır yok. `configs.edn` yazımı ve bekleyenin uygulanması yalnız
/// bu durumda yapılır (menüden gelen istek de, durum yoklaması da aynı kuralı kullanır).
pub fn kapali_mi(ana_pencere_var: bool, surec_var: bool, kapali_yoklama: u32) -> bool {
    !ana_pencere_var && !surec_var && kapali_yoklama >= KAPALI_YOKLAMA_SAYISI
}

/// Durum yoklamasında: bekleyen varsa ve Logseq kapandıysa (`logseq_kapali`, bkz. `kapali_mi`)
/// uygulanır.
pub fn bekleyen_karari(bekleyen: Option<bool>, gercek: bool, logseq_kapali: bool) -> Karar {
    match bekleyen {
        Some(b) if logseq_kapali => karar(b, gercek, bekleyen, false),
        _ => Karar::Yok,
    }
}

/// Kararın kullanıcıya/eklentiye bildirilen etkisi (`POST /ayar` yanıt gövdesi).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Etki {
    /// İstenen değer `configs.edn`de geçerli (yazıldı ya da zaten öyleydi).
    Yazildi,
    /// Logseq açık: Logseq kapanınca uygulanacak.
    Beklemede,
    /// Uygulanamadı (sabit neden metni, dosya içeriği taşımaz).
    Hata(String),
}

impl Etki {
    /// Yanıt gövdesi satırları: `api_otomatik_baslat=yazildi|beklemede|hata` (+ hatada
    /// `api_otomatik_neden=...`, tek satır).
    pub fn govde(&self) -> String {
        match self {
            Etki::Yazildi => "api_otomatik_baslat=yazildi\n".to_owned(),
            Etki::Beklemede => "api_otomatik_baslat=beklemede\n".to_owned(),
            Etki::Hata(n) => format!(
                "api_otomatik_baslat=hata\napi_otomatik_neden={}\n",
                n.replace(['\r', '\n'], " ")
            ),
        }
    }
}

/// Kararın uygulanmış hali: ayar.toml'daki bekleyen değerin yeni durumu, günlük satırı ve etki.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sonuc {
    pub bekleyen: Option<bool>,
    pub gunluk: Option<String>,
    pub etki: Etki,
}

/// `k`yi uygular. Yazma başarısızsa (beklenmeyen yapı vb.) bekleyen silinir (her yoklamada
/// yeniden denenmesin) ve neden günlüğe gider; neden içerik taşımaz.
pub fn uygula(dosya: &Path, k: Karar, onceki_bekleyen: Option<bool>) -> Sonuc {
    let (bekleyen, gunluk, etki) = match k {
        Karar::Yok => (onceki_bekleyen, None, Etki::Yazildi),
        Karar::BekleyeniSil => (None, None, Etki::Yazildi),
        Karar::Beklet(b) => (
            Some(b),
            Some(format!(
                "api_otomatik_baslat={b}: Logseq acik; Logseq kapaninca uygulanacak"
            )),
            Etki::Beklemede,
        ),
        Karar::Yaz(b) => {
            let (g, etki) = match yapilandirma::autostart_yaz(dosya, b) {
                Ok(Yazim::Degismedi) => (
                    format!("api_otomatik_baslat={b}: zaten boyleydi"),
                    Etki::Yazildi,
                ),
                Ok(Yazim::Yazildi { yedek_alindi }) => (
                    format!(
                        "api_otomatik_baslat={b}: configs.edn yazildi{}",
                        if yedek_alindi { " (yedek alindi)" } else { "" }
                    ),
                    Etki::Yazildi,
                ),
                Err(h) => (
                    format!("api_otomatik_baslat={b}: uygulanamadi: {h}"),
                    Etki::Hata(h.to_string()),
                ),
            };
            (None, Some(g), etki)
        }
    };
    Sonuc {
        bekleyen,
        gunluk,
        etki,
    }
}

/// Logseq'in o anki gözlemi (menü isteği ve durum yoklaması AYNI kuralla karar verir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gozlem {
    pub ana_pencere_var: bool,
    pub surec_var: bool,
    pub kapali_yoklama: u32,
}

impl Gozlem {
    pub fn kapali(self) -> bool {
        kapali_mi(self.ana_pencere_var, self.surec_var, self.kapali_yoklama)
    }
}

/// Kullanıcının/eklentinin `api_otomatik_baslat` isteği: `configs.edn`i okuyup kararı verir ve
/// uygular. Logseq kapandı sayılmıyorsa (`g.kapali()` değilse) dosyaya yazılmaz, bekletilir.
pub fn istegi_uygula(dosya: &Path, istenen: bool, bekleyen: Option<bool>, g: Gozlem) -> Sonuc {
    let gercek = yapilandirma::autostart_oku(dosya).unwrap_or(false);
    uygula(
        dosya,
        karar(istenen, gercek, bekleyen, !g.kapali()),
        bekleyen,
    )
}

/// Durum yoklamasında bekleyen istek: Logseq kapandıysa uygulanır (`None`: yapılacak iş yok).
pub fn bekleyeni_uygula(dosya: &Path, bekleyen: Option<bool>, g: Gozlem) -> Option<Sonuc> {
    let gercek = yapilandirma::autostart_oku(dosya).unwrap_or(false);
    match bekleyen_karari(bekleyen, gercek, g.kapali()) {
        Karar::Yok => None,
        k => Some(uygula(dosya, k, bekleyen)),
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn karar_tablosu() {
        // (istenen, gercek, bekleyen, acik) -> karar
        let t = [
            (true, true, None, true, Karar::Yok),
            (true, true, None, false, Karar::Yok),
            (true, true, Some(false), true, Karar::BekleyeniSil),
            (false, false, Some(true), false, Karar::BekleyeniSil),
            (true, false, None, true, Karar::Beklet(true)),
            (false, true, Some(true), true, Karar::Beklet(false)),
            (true, false, None, false, Karar::Yaz(true)),
            (false, true, None, false, Karar::Yaz(false)),
        ];
        for (i, g, b, a, beklenen) in t {
            assert_eq!(karar(i, g, b, a), beklenen, "{i} {g} {b:?} {a}");
        }
    }

    /// Pencere yok, süreç yok, `n` ardışık yoklama: durum yoklamasının ve menünün ortak kuralı.
    fn kapali(n: u32) -> bool {
        kapali_mi(false, false, n)
    }

    #[test]
    fn bekleyen_yalniz_logseq_kapaliyken_uygulanir() {
        assert_eq!(bekleyen_karari(None, false, kapali(9)), Karar::Yok);
        assert_eq!(
            bekleyen_karari(Some(true), false, kapali(0)),
            Karar::Yok,
            "Logseq açıkken yazılmaz"
        );
        assert_eq!(
            bekleyen_karari(Some(true), false, kapali(KAPALI_YOKLAMA_SAYISI - 1)),
            Karar::Yok,
            "yeni kapandı: kapanış yazımı bitsin diye beklenir"
        );
        assert_eq!(
            bekleyen_karari(Some(true), false, kapali(KAPALI_YOKLAMA_SAYISI)),
            Karar::Yaz(true)
        );
        assert_eq!(
            bekleyen_karari(Some(true), true, kapali(KAPALI_YOKLAMA_SAYISI)),
            Karar::BekleyeniSil,
            "gerçek değer zaten istenen"
        );
    }

    fn gozlem(ana: bool, surec: bool, n: u32) -> Gozlem {
        Gozlem {
            ana_pencere_var: ana,
            surec_var: surec,
            kapali_yoklama: n,
        }
    }

    /// Menüden (`istegi_uygula`) ve durum yoklamasından (`bekleyeni_uygula`) gelen yol AYNI kapanış
    /// kuralını izler: ana pencere yok + Logseq.exe süreci yok + ardışık yoklama yeterli.
    #[test]
    fn menu_ve_bekleyen_yolu_ayni_kapanis_kuralini_izler() {
        let ayni = kapali_mi(false, false, KAPALI_YOKLAMA_SAYISI);
        assert!(ayni);
        let n = KAPALI_YOKLAMA_SAYISI;
        // (gözlem, dosyaya yazılması beklenir mi)
        let durumlar = [
            (gozlem(true, true, 0), false, "pencere var"),
            (gozlem(false, true, n), false, "pencere yok ama süreç var"),
            (gozlem(false, false, 0), false, "pencere yeni kayboldu"),
            (gozlem(false, false, n - 1), false, "yoklama sayısı yetmedi"),
            (gozlem(false, false, n), true, "gerçekten kapalı"),
        ];
        for (g, yazilir, ad) in durumlar {
            // Menü isteği: açık (true) istenir, dosyada false.
            let d = gecici("kural-menu");
            let f = d.join("configs.edn");
            std::fs::write(&f, "{:a 1}").unwrap();
            let s = istegi_uygula(&f, true, None, g);
            assert_eq!(yapilandirma::autostart_oku(&f), Ok(yazilir), "menü: {ad}");
            assert_eq!(
                s.etki,
                if yazilir {
                    Etki::Yazildi
                } else {
                    Etki::Beklemede
                },
                "menü: {ad}"
            );
            assert_eq!(
                s.bekleyen,
                if yazilir { None } else { Some(true) },
                "menü: {ad}"
            );
            // Durum yoklaması: bekleyen istek aynı kuralla uygulanır.
            std::fs::write(&f, "{:a 1}").unwrap();
            let b = bekleyeni_uygula(&f, Some(true), g);
            assert_eq!(
                yapilandirma::autostart_oku(&f),
                Ok(yazilir),
                "bekleyen: {ad}"
            );
            assert_eq!(b.is_some(), yazilir, "bekleyen: {ad}");
            let _ = std::fs::remove_dir_all(&d);
        }
        // Bekleyen yoksa yapılacak iş yok.
        let d = gecici("kural-yok");
        let f = d.join("configs.edn");
        std::fs::write(&f, "{:a 1}").unwrap();
        assert_eq!(bekleyeni_uygula(&f, None, gozlem(false, false, 9)), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kapali_mi_pencere_surec_ve_yoklama_birlikte_ister() {
        let n = KAPALI_YOKLAMA_SAYISI;
        assert!(kapali_mi(false, false, n));
        assert!(kapali_mi(false, false, n + 5));
        assert!(!kapali_mi(true, false, n), "ana pencere var");
        assert!(
            !kapali_mi(false, true, n),
            "pencere yok ama Logseq.exe süreci hâlâ var (kapanıyor ya da pencere gizli)"
        );
        assert!(
            !kapali_mi(false, false, n - 1),
            "yoklama sayısı yetmedi: menüden gelen istek de beklemeli"
        );
        assert!(!kapali_mi(false, false, 0));
    }

    #[test]
    fn etki_govdesi_sabit_biçimde_ve_tek_satir_neden() {
        assert_eq!(Etki::Yazildi.govde(), "api_otomatik_baslat=yazildi\n");
        assert_eq!(Etki::Beklemede.govde(), "api_otomatik_baslat=beklemede\n");
        assert_eq!(
            Etki::Hata("bir\nneden".into()).govde(),
            "api_otomatik_baslat=hata\napi_otomatik_neden=bir neden\n"
        );
    }

    #[test]
    fn uygula_etkisi_yazildi_beklemede_hata() {
        let d = gecici("etki");
        let f = d.join("configs.edn");
        std::fs::write(&f, "{:a 1}").unwrap();
        assert_eq!(uygula(&f, Karar::Beklet(true), None).etki, Etki::Beklemede);
        assert_eq!(uygula(&f, Karar::Yaz(true), None).etki, Etki::Yazildi);
        assert_eq!(
            uygula(&f, Karar::Yaz(true), None).etki,
            Etki::Yazildi,
            "zaten öyle"
        );
        assert_eq!(uygula(&f, Karar::Yok, None).etki, Etki::Yazildi);
        assert_eq!(
            uygula(&f, Karar::BekleyeniSil, Some(true)).etki,
            Etki::Yazildi
        );
        std::fs::write(&f, "[\"bulut-masa-kedi-deniz\"]").unwrap();
        let Etki::Hata(neden) = uygula(&f, Karar::Yaz(false), None).etki else {
            panic!("hata etkisi bekleniyordu");
        };
        assert!(neden.contains("beklenmeyen yapıda") && !neden.contains("bulut-masa"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `yokla_dongusu`nu iş parçacığında koşturur; `hedef` beklenen değere gelene kadar bekler.
    fn yoklayici_sonucu(hedef: &AtomicBool, beklenen: bool) -> bool {
        for _ in 0..200 {
            if hedef.load(Ordering::Relaxed) == beklenen {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn yoklayici_arka_planda_durumu_gunceller_etkin_degilken_baglanmaz() {
        // Gerçek 12315'e dokunulmaz: sahte dinleyici.
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let hedef = std::sync::Arc::new(AtomicBool::new(false));
        let durdur = std::sync::Arc::new(AtomicBool::new(false));
        let etkin = std::sync::Arc::new(AtomicBool::new(true));
        let (h, d, e) = (hedef.clone(), durdur.clone(), etkin.clone());
        let is = std::thread::spawn(move || {
            yokla_dongusu(
                adres,
                Duration::from_millis(20),
                move || e.load(Ordering::Relaxed),
                &h,
                &d,
            )
        });
        assert!(yoklayici_sonucu(&hedef, true), "dinleyici açıkken true");
        // Logseq kapalı (etkin değil): dinleyici açık olsa da yoklanmaz, sonuç false.
        etkin.store(false, Ordering::Relaxed);
        assert!(yoklayici_sonucu(&hedef, false), "etkin değilken false");
        etkin.store(true, Ordering::Relaxed);
        assert!(yoklayici_sonucu(&hedef, true));
        drop(dinleyici);
        assert!(yoklayici_sonucu(&hedef, false), "port kapanınca false");
        durdur.store(true, Ordering::Relaxed);
        is.join().unwrap();
    }

    fn gecici(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("logseq-tepsi-api-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn uygula_yaz_dosyayi_degistirir_bekleyeni_siler() {
        let d = gecici("yaz");
        let f = d.join("configs.edn");
        std::fs::write(&f, "{:a \"kedi\"}").unwrap();
        let s = uygula(&f, Karar::Yaz(true), Some(true));
        assert_eq!(s.bekleyen, None);
        assert!(s.gunluk.unwrap().contains("yedek alindi"));
        assert_eq!(yapilandirma::autostart_oku(&f), Ok(true));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn uygula_beklet_dosyaya_dokunmaz() {
        let d = gecici("beklet");
        let f = d.join("configs.edn");
        std::fs::write(&f, "{:a 1}").unwrap();
        let s = uygula(&f, Karar::Beklet(true), None);
        assert_eq!(s.bekleyen, Some(true));
        assert!(s.gunluk.unwrap().contains("Logseq kapaninca"));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "{:a 1}");
        assert_eq!(uygula(&f, Karar::Yok, Some(false)).bekleyen, Some(false));
        assert_eq!(uygula(&f, Karar::BekleyeniSil, Some(true)).bekleyen, None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn uygula_reddedilirse_bekleyen_silinir_neden_icerik_tasimaz() {
        let d = gecici("red");
        let f = d.join("configs.edn");
        std::fs::write(&f, "[\"bulut-masa-kedi-deniz\"]").unwrap();
        let s = uygula(&f, Karar::Yaz(true), Some(true));
        assert_eq!(s.bekleyen, None, "sonsuz yeniden deneme olmasın");
        let g = s.gunluk.unwrap();
        assert!(
            g.contains("uygulanamadi") && !g.contains("bulut-masa"),
            "{g}"
        );
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "[\"bulut-masa-kedi-deniz\"]"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn yoklama_dinleyen_porta_true_kapaliya_false() {
        // Gerçek 12315'e dokunulmaz: sahte dinleyici.
        let dinleyici = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adres = dinleyici.local_addr().unwrap();
        assert!(acik_mi_adres(adres, Duration::from_millis(500)));
        drop(dinleyici);
        assert!(!acik_mi_adres(adres, Duration::from_millis(500)));
    }

    #[test]
    fn varsayilan_adres_logseq_api_portu() {
        assert_eq!(ADRES.to_string(), "127.0.0.1:12315");
    }
}
