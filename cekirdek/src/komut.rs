//! Komut satırı çözümü (saf): hangi alt komut çalışacak.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Komut {
    /// Tepsi aracını çalıştır (varsayılan; `--url` ile açılış da buraya düşer).
    Calistir,
    Kur,
    Kaldir,
    SinaGizleGoster,
    SinaKucult,
    SinaSimgeTikla,
    SinaPencereler,
    SinaTik,
}

/// `argumanlar`: program adından sonrakiler. `--url <adres>` (URL şeması kaydı) argümanı
/// YOK SAYILIR: yalnız aracı başlatır; bilinmeyen argüman da aynı (eski davranış).
pub fn cozumle(argumanlar: &[String]) -> Komut {
    match argumanlar.first().map(String::as_str) {
        Some("kur") => Komut::Kur,
        Some("kaldir") => Komut::Kaldir,
        Some("--sina-gizle-goster") => Komut::SinaGizleGoster,
        Some("--sina-kucult") => Komut::SinaKucult,
        Some("--sina-simge-tikla") => Komut::SinaSimgeTikla,
        Some("--sina-pencereler") => Komut::SinaPencereler,
        Some("--sina-tik") => Komut::SinaTik,
        _ => Komut::Calistir,
    }
}

/// Araç başlangıç kısayolundan (oturum açılışı) mı başladı? Kısayol `--oturum` argümanını taşır
/// (`yollar::BASLANGIC_ARGUMANI`); yalnız ilk argüman sayılır. Komut seçimini değiştirmez.
pub fn oturum_mu(argumanlar: &[String]) -> bool {
    argumanlar.first().map(String::as_str) == Some(crate::yollar::BASLANGIC_ARGUMANI)
}

/// Alt komut çıktı yazar mı? (Yazıyorsa ana konsola bağlanılır.)
pub fn konsol_gerekir_mi(k: Komut) -> bool {
    k != Komut::Calistir
}

#[cfg(test)]
mod testler {
    use super::*;

    fn c(a: &[&str]) -> Komut {
        cozumle(&a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn alt_komutlar() {
        assert_eq!(c(&[]), Komut::Calistir);
        assert_eq!(c(&["kur"]), Komut::Kur);
        assert_eq!(c(&["kaldir"]), Komut::Kaldir);
        assert_eq!(c(&["--sina-gizle-goster"]), Komut::SinaGizleGoster);
        assert_eq!(c(&["--sina-kucult"]), Komut::SinaKucult);
        assert_eq!(c(&["--sina-simge-tikla"]), Komut::SinaSimgeTikla);
        assert_eq!(c(&["--sina-pencereler"]), Komut::SinaPencereler);
        assert_eq!(c(&["--sina-tik"]), Komut::SinaTik);
    }

    #[test]
    fn url_argumani_yok_sayilir_yalniz_arac_baslar() {
        assert_eq!(c(&["--url", "logseq-tepsi://baslat"]), Komut::Calistir);
        assert_eq!(c(&["--url", "kur"]), Komut::Calistir);
        assert_eq!(c(&["--url", "--sina-kucult"]), Komut::Calistir);
        assert_eq!(c(&["--url"]), Komut::Calistir);
    }

    #[test]
    fn bilinmeyen_arguman_araci_baslatir() {
        assert_eq!(c(&["--bilinmeyen"]), Komut::Calistir);
        assert_eq!(c(&["KUR"]), Komut::Calistir);
    }

    #[test]
    fn yalniz_alt_komutlar_konsol_ister() {
        assert!(!konsol_gerekir_mi(Komut::Calistir));
        assert!(konsol_gerekir_mi(Komut::Kur));
        assert!(konsol_gerekir_mi(Komut::SinaPencereler));
    }

    #[test]
    fn oturum_argumani_yalniz_ilk_arguman_ve_komutu_degistirmez() {
        let a = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert!(oturum_mu(&a(&["--oturum"])));
        assert!(!oturum_mu(&a(&[])));
        assert!(!oturum_mu(&a(&["--url", "--oturum"])));
        assert!(!oturum_mu(&a(&["--OTURUM"])));
        assert!(!oturum_mu(&a(&["kur"])));
        // Aracı normal başlatır (bilinmeyen argüman davranışı), alt komut değil.
        assert_eq!(c(&["--oturum"]), Komut::Calistir);
        assert!(!konsol_gerekir_mi(c(&["--oturum"])));
    }
}
