//! Logseq HTTP API sunucusu: "çalışıyor mu" yoklaması (kısa TCP bağlantısı) ve
//! `api_otomatik_baslat` ayarının nasıl uygulanacağı kararı (yazma YALNIZ Logseq kapalıyken).

use crate::yapilandirma::{self, Yazim};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::Path;
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

/// Logseq ana penceresi bu kadar ardışık yoklamada görünmediyse kapandı sayılır (kapanırken
/// dosyaya hâlâ yazıyor olabilir; 2 sn aralıkla ~4 sn bekler).
pub const KAPALI_YOKLAMA_SAYISI: u32 = 2;

/// Durum yoklamasında: bekleyen varsa ve Logseq `kapali_yoklama` ardışık yoklamadır kapalıysa
/// uygulanır (0 = şu an açık).
pub fn bekleyen_karari(bekleyen: Option<bool>, gercek: bool, kapali_yoklama: u32) -> Karar {
    match bekleyen {
        Some(b) if kapali_yoklama >= KAPALI_YOKLAMA_SAYISI => karar(b, gercek, bekleyen, false),
        _ => Karar::Yok,
    }
}

/// Kararın uygulanmış hali: ayar.toml'daki bekleyen değerin yeni durumu ve günlük satırı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sonuc {
    pub bekleyen: Option<bool>,
    pub gunluk: Option<String>,
}

/// `k`yi uygular. Yazma başarısızsa (beklenmeyen yapı vb.) bekleyen silinir (her yoklamada
/// yeniden denenmesin) ve neden günlüğe gider; neden içerik taşımaz.
pub fn uygula(dosya: &Path, k: Karar, onceki_bekleyen: Option<bool>) -> Sonuc {
    let (bekleyen, gunluk) = match k {
        Karar::Yok => (onceki_bekleyen, None),
        Karar::BekleyeniSil => (None, None),
        Karar::Beklet(b) => (
            Some(b),
            Some(format!(
                "api_otomatik_baslat={b}: Logseq acik; Logseq kapaninca uygulanacak"
            )),
        ),
        Karar::Yaz(b) => (
            None,
            Some(match yapilandirma::autostart_yaz(dosya, b) {
                Ok(Yazim::Degismedi) => format!("api_otomatik_baslat={b}: zaten boyleydi"),
                Ok(Yazim::Yazildi { yedek_alindi }) => format!(
                    "api_otomatik_baslat={b}: configs.edn yazildi{}",
                    if yedek_alindi { " (yedek alindi)" } else { "" }
                ),
                Err(h) => format!("api_otomatik_baslat={b}: uygulanamadi: {h}"),
            }),
        ),
    };
    Sonuc { bekleyen, gunluk }
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

    #[test]
    fn bekleyen_yalniz_logseq_kapaliyken_uygulanir() {
        assert_eq!(bekleyen_karari(None, false, 9), Karar::Yok);
        assert_eq!(
            bekleyen_karari(Some(true), false, 0),
            Karar::Yok,
            "Logseq açıkken yazılmaz"
        );
        assert_eq!(
            bekleyen_karari(Some(true), false, KAPALI_YOKLAMA_SAYISI - 1),
            Karar::Yok,
            "yeni kapandı: kapanış yazımı bitsin diye beklenir"
        );
        assert_eq!(
            bekleyen_karari(Some(true), false, KAPALI_YOKLAMA_SAYISI),
            Karar::Yaz(true)
        );
        assert_eq!(
            bekleyen_karari(Some(true), true, KAPALI_YOKLAMA_SAYISI),
            Karar::BekleyeniSil,
            "gerçek değer zaten istenen"
        );
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
