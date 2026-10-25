//! `%APPDATA%\Logseq\configs.edn` dosya işleri: `:server/autostart` okuma ve yazma. Yazma:
//! ilk yazımdan önce bir kez `configs.edn.tepsi-yedek`, atomik (geçici dosya + yeniden
//! adlandırma). Hata iletileri dosya içeriği, yol ya da değer taşımaz (yalnız `ErrorKind`).

use crate::edn::{self, Red};
use std::fmt;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub const DOSYA: &str = "configs.edn";
pub const YEDEK_EKI: &str = ".tepsi-yedek";
pub const GECICI_EKI: &str = ".tepsi-tmp";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hata {
    Okunamadi(ErrorKind),
    /// Beklenmeyen yapı: yazma reddedildi, dosyaya dokunulmadı.
    Edn(Red),
    YedekAlinamadi(ErrorKind),
    Yazilamadi(ErrorKind),
}

impl fmt::Display for Hata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hata::Okunamadi(k) => write!(f, "configs.edn okunamadı ({k})"),
            Hata::Edn(r) => write!(f, "configs.edn beklenmeyen yapıda, yazılmadı: {r}"),
            Hata::YedekAlinamadi(k) => write!(f, "yedek alınamadı, yazılmadı ({k})"),
            Hata::Yazilamadi(k) => write!(f, "configs.edn yazılamadı ({k})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yazim {
    /// İstenen değer zaten vardı; dosyaya dokunulmadı.
    Degismedi,
    Yazildi {
        /// Bu yazımda ilk kez yedek alındı.
        yedek_alindi: bool,
    },
}

/// `%APPDATA%` değerinden `configs.edn` yolu.
pub fn yol(appdata: &str) -> PathBuf {
    Path::new(appdata).join("Logseq").join(DOSYA)
}

fn ek_yol(dosya: &Path, ek: &str) -> PathBuf {
    let mut ad = dosya.file_name().unwrap_or_default().to_os_string();
    ad.push(ek);
    dosya.with_file_name(ad)
}

fn metin_oku(dosya: &Path) -> Result<String, Hata> {
    std::fs::read_to_string(dosya).map_err(|e| Hata::Okunamadi(e.kind()))
}

/// `:server/autostart` (yok = `false`).
pub fn autostart_oku(dosya: &Path) -> Result<bool, Hata> {
    edn::oku(&metin_oku(dosya)?).map_err(Hata::Edn)
}

fn atomik_yaz(dosya: &Path, metin: &str) -> std::io::Result<()> {
    let gecici = ek_yol(dosya, GECICI_EKI);
    let yaz = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&gecici)?;
        f.write_all(metin.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&gecici, dosya)
    };
    let sonuc = yaz();
    if sonuc.is_err() {
        let _ = std::fs::remove_file(&gecici);
    }
    sonuc
}

/// `:server/autostart` değerini yazar. Dosya yoksa ya da yapı beklenmedikse hiçbir dosya
/// oluşturulmaz/değişmez. Yalnız Logseq kapalıyken çağrılmalıdır (karar çağıranda).
pub fn autostart_yaz(dosya: &Path, deger: bool) -> Result<Yazim, Hata> {
    let eski = metin_oku(dosya)?;
    let yeni = edn::ayarla(&eski, deger).map_err(Hata::Edn)?;
    if yeni == eski {
        return Ok(Yazim::Degismedi);
    }
    let yedek = ek_yol(dosya, YEDEK_EKI);
    let yedek_alindi = !yedek.exists();
    if yedek_alindi {
        std::fs::copy(dosya, &yedek).map_err(|e| Hata::YedekAlinamadi(e.kind()))?;
    }
    atomik_yaz(dosya, &yeni).map_err(|e| Hata::Yazilamadi(e.kind()))?;
    Ok(Yazim::Yazildi { yedek_alindi })
}

#[cfg(test)]
mod testler {
    use super::*;

    const SIR: &str = "elma-kitap-nehir-tas";

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("logseq-tepsi-cfg-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sahte() -> String {
        format!("{{:theme \"dark\"\n :server/tokens [{{:name \"kedi\" :value \"{SIR}\"}}]}}\n")
    }

    #[test]
    fn yol_appdata_logseq_configs() {
        assert_eq!(
            yol("C:\\Veri\\Roaming"),
            Path::new("C:\\Veri\\Roaming")
                .join("Logseq")
                .join("configs.edn")
        );
    }

    #[test]
    fn yazma_yedek_alir_atomik_yazar_gecici_birakmaz() {
        let d = gecici("yaz");
        let f = d.join(DOSYA);
        std::fs::write(&f, sahte()).unwrap();
        assert_eq!(autostart_oku(&f), Ok(false));
        assert_eq!(
            autostart_yaz(&f, true),
            Ok(Yazim::Yazildi { yedek_alindi: true })
        );
        assert_eq!(autostart_oku(&f), Ok(true));
        assert_eq!(
            std::fs::read_to_string(d.join("configs.edn.tepsi-yedek")).unwrap(),
            sahte(),
            "yedek özgün içerik"
        );
        assert!(!d.join("configs.edn.tepsi-tmp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn yedek_yalniz_bir_kez_ilk_icerikle_alinir() {
        let d = gecici("yedek");
        let f = d.join(DOSYA);
        std::fs::write(&f, sahte()).unwrap();
        autostart_yaz(&f, true).unwrap();
        // Logseq araya girip dosyayı değiştirmiş olsun.
        std::fs::write(&f, "{:baska 1}").unwrap();
        assert_eq!(
            autostart_yaz(&f, true),
            Ok(Yazim::Yazildi {
                yedek_alindi: false
            })
        );
        assert_eq!(
            std::fs::read_to_string(d.join("configs.edn.tepsi-yedek")).unwrap(),
            sahte(),
            "yedek ilk yazımdaki içerikte kalır"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ayni_deger_dosyaya_dokunmaz_yedek_almaz() {
        let d = gecici("ayni");
        let f = d.join(DOSYA);
        std::fs::write(&f, "{:server/autostart true}").unwrap();
        assert_eq!(autostart_yaz(&f, true), Ok(Yazim::Degismedi));
        assert!(!d.join("configs.edn.tepsi-yedek").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn beklenmeyen_yapi_reddedilir_dosya_ve_yedek_olusmaz() {
        let d = gecici("red");
        let f = d.join(DOSYA);
        let bozuk = format!("[\"{SIR}\"]");
        std::fs::write(&f, &bozuk).unwrap();
        let hata = autostart_yaz(&f, true).unwrap_err();
        assert_eq!(hata, Hata::Edn(Red::UstDuzeyHaritaDegil));
        assert!(!hata.to_string().contains(SIR));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), bozuk, "dokunulmadı");
        assert!(!d.join("configs.edn.tepsi-yedek").exists());
        assert!(!d.join("configs.edn.tepsi-tmp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn eksik_dosya_olusturulmaz() {
        let d = gecici("eksik");
        let f = d.join(DOSYA);
        assert_eq!(
            autostart_yaz(&f, true),
            Err(Hata::Okunamadi(ErrorKind::NotFound))
        );
        assert_eq!(autostart_oku(&f), Err(Hata::Okunamadi(ErrorKind::NotFound)));
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn hata_iletileri_yol_ve_icerik_tasimaz() {
        let d = gecici("ileti");
        let f = d.join(DOSYA);
        std::fs::write(&f, format!("{{:x \"{SIR}\" :server/autostart \"{SIR}\"}}")).unwrap();
        let ileti = autostart_oku(&f).unwrap_err().to_string();
        assert!(!ileti.contains(SIR));
        assert!(!ileti.contains(&*d.to_string_lossy()));
        let _ = std::fs::remove_dir_all(&d);
    }
}
