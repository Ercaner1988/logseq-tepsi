//! Tepsi menüsündeki "Ayarlar" alt menüsünün modeli (saf): hangi madde işaretli, bir maddeye
//! tıklanınca hangi ayar değişir. Win32 yalnız bu modeli çizer (`AppendMenuW`,
//! `CheckMenuRadioItem`); karar burada, sınamalı.

use crate::ayar::{AyarDegeri, Ayarlar, Kucultme, Tik, VARSAYILAN_KISAYOL};

pub const KIMLIK_KUCULTME_TEPSI: usize = 100;
pub const KIMLIK_KUCULTME_GOREV: usize = 101;
pub const KIMLIK_TIK_TEK: usize = 102;
pub const KIMLIK_TIK_CIFT: usize = 103;
pub const KIMLIK_KAPALIYKEN_ACAR: usize = 104;
pub const KIMLIK_KISAYOL: usize = 105;
pub const KIMLIK_OTURUMDA_GIZLI: usize = 106;
pub const KIMLIK_BASLANGICTA: usize = 107;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tur {
    Ayirici,
    /// `MF_CHECKED` ile işaretlenen açık/kapalı madde.
    Onay(bool),
    /// Radyo seçimi: `ilk..=son` aynı gruptaki kimlikler (`CheckMenuRadioItem` aralığı).
    Radyo {
        isaretli: bool,
        ilk: usize,
        son: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Madde {
    pub kimlik: usize,
    pub etiket: String,
    pub tur: Tur,
}

fn madde(kimlik: usize, etiket: &str, tur: Tur) -> Madde {
    Madde {
        kimlik,
        etiket: etiket.to_owned(),
        tur,
    }
}

/// Alt menünün maddeleri sırasıyla. `baslangicta`: başlangıç kısayolu şu an var mı.
pub fn ayar_maddeleri(a: &Ayarlar, baslangicta: bool) -> Vec<Madde> {
    let kuc = |kimlik, etiket, isaretli| {
        madde(
            kimlik,
            etiket,
            Tur::Radyo {
                isaretli,
                ilk: KIMLIK_KUCULTME_TEPSI,
                son: KIMLIK_KUCULTME_GOREV,
            },
        )
    };
    let tik = |kimlik, etiket, isaretli| {
        madde(
            kimlik,
            etiket,
            Tur::Radyo {
                isaretli,
                ilk: KIMLIK_TIK_TEK,
                son: KIMLIK_TIK_CIFT,
            },
        )
    };
    let kisayol = if a.kisayol_goster_gizle.is_empty() {
        format!("Genel kısayol ({VARSAYILAN_KISAYOL})")
    } else {
        format!("Genel kısayol: {}", a.kisayol_goster_gizle)
    };
    vec![
        kuc(
            KIMLIK_KUCULTME_TEPSI,
            "Küçültünce tepsiye gizle",
            a.kucultme == Kucultme::Tepsi,
        ),
        kuc(
            KIMLIK_KUCULTME_GOREV,
            "Küçültünce görev çubuğunda kalsın",
            a.kucultme == Kucultme::GorevCubugu,
        ),
        madde(0, "", Tur::Ayirici),
        tik(KIMLIK_TIK_TEK, "Tek tıkla göster/gizle", a.tik == Tik::Tek),
        tik(
            KIMLIK_TIK_CIFT,
            "Çift tıkla göster/gizle",
            a.tik == Tik::Cift,
        ),
        madde(
            KIMLIK_KAPALIYKEN_ACAR,
            "Logseq kapalıyken tıklayınca açılsın",
            Tur::Onay(a.kapaliyken_tik_acar),
        ),
        madde(0, "", Tur::Ayirici),
        madde(
            KIMLIK_KISAYOL,
            &kisayol,
            Tur::Onay(!a.kisayol_goster_gizle.is_empty()),
        ),
        madde(
            KIMLIK_OTURUMDA_GIZLI,
            "Oturum açılınca Logseq'i gizli başlat",
            Tur::Onay(a.oturumda_gizli_baslat),
        ),
        madde(
            KIMLIK_BASLANGICTA,
            "Bilgisayar açılınca başlat",
            Tur::Onay(baslangicta),
        ),
    ]
}

/// Seçilen menü kimliğinin yaptığı ayar değişikliği (bilinmeyen kimlik: `None`). Onay maddeleri
/// mevcut durumu tersine çevirir; kısayol maddesi kapatınca boşaltır, açınca varsayılanı yazar.
pub fn secim_degeri(kimlik: usize, a: &Ayarlar, baslangicta: bool) -> Option<AyarDegeri> {
    Some(match kimlik {
        KIMLIK_KUCULTME_TEPSI => AyarDegeri::Kucultme(Kucultme::Tepsi),
        KIMLIK_KUCULTME_GOREV => AyarDegeri::Kucultme(Kucultme::GorevCubugu),
        KIMLIK_TIK_TEK => AyarDegeri::Tik(Tik::Tek),
        KIMLIK_TIK_CIFT => AyarDegeri::Tik(Tik::Cift),
        KIMLIK_KAPALIYKEN_ACAR => AyarDegeri::KapaliykenTikAcar(!a.kapaliyken_tik_acar),
        KIMLIK_KISAYOL => AyarDegeri::KisayolGosterGizle(if a.kisayol_goster_gizle.is_empty() {
            VARSAYILAN_KISAYOL.to_owned()
        } else {
            String::new()
        }),
        KIMLIK_OTURUMDA_GIZLI => AyarDegeri::OturumdaGizliBaslat(!a.oturumda_gizli_baslat),
        KIMLIK_BASLANGICTA => AyarDegeri::BaslangictaCalis(!baslangicta),
        _ => return None,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    fn isaretliler(a: &Ayarlar, baslangicta: bool) -> Vec<usize> {
        ayar_maddeleri(a, baslangicta)
            .iter()
            .filter(|m| matches!(m.tur, Tur::Onay(true) | Tur::Radyo { isaretli: true, .. }))
            .map(|m| m.kimlik)
            .collect()
    }

    #[test]
    fn varsayilan_ayarda_isaretliler() {
        assert_eq!(
            isaretliler(&Ayarlar::default(), false),
            vec![
                KIMLIK_KUCULTME_TEPSI,
                KIMLIK_TIK_TEK,
                KIMLIK_KAPALIYKEN_ACAR,
                KIMLIK_KISAYOL
            ]
        );
    }

    #[test]
    fn isaretler_ayarlari_yansitir() {
        let a = Ayarlar {
            kucultme: Kucultme::GorevCubugu,
            tik: Tik::Cift,
            kapaliyken_tik_acar: false,
            kisayol_goster_gizle: String::new(),
            oturumda_gizli_baslat: true,
        };
        assert_eq!(
            isaretliler(&a, true),
            vec![
                KIMLIK_KUCULTME_GOREV,
                KIMLIK_TIK_CIFT,
                KIMLIK_OTURUMDA_GIZLI,
                KIMLIK_BASLANGICTA
            ]
        );
    }

    #[test]
    fn baslangicta_isareti_kisayolun_varligini_gosterir_dosyayi_degil() {
        let a = Ayarlar::default();
        assert!(!isaretliler(&a, false).contains(&KIMLIK_BASLANGICTA));
        assert!(isaretliler(&a, true).contains(&KIMLIK_BASLANGICTA));
    }

    #[test]
    fn radyo_gruplari_tek_isaretli_ve_aralik_dogru() {
        for a in [
            Ayarlar::default(),
            Ayarlar {
                kucultme: Kucultme::GorevCubugu,
                tik: Tik::Cift,
                ..Ayarlar::default()
            },
        ] {
            let maddeler = ayar_maddeleri(&a, false);
            for (ilk, son) in [
                (KIMLIK_KUCULTME_TEPSI, KIMLIK_KUCULTME_GOREV),
                (KIMLIK_TIK_TEK, KIMLIK_TIK_CIFT),
            ] {
                let grup: Vec<&Madde> = maddeler
                    .iter()
                    .filter(|m| matches!(m.tur, Tur::Radyo { ilk: i, son: s, .. } if (i, s) == (ilk, son)))
                    .collect();
                assert_eq!(grup.len(), son - ilk + 1);
                assert_eq!(
                    grup.iter()
                        .filter(|m| matches!(m.tur, Tur::Radyo { isaretli: true, .. }))
                        .count(),
                    1,
                    "grup {ilk}..{son} tam bir işaretli içermeli"
                );
            }
        }
    }

    #[test]
    fn kimlikler_benzersiz_ve_ana_menu_kimlikleriyle_cakismaz() {
        let maddeler = ayar_maddeleri(&Ayarlar::default(), false);
        let mut kimlikler: Vec<usize> = maddeler
            .iter()
            .filter(|m| m.tur != Tur::Ayirici)
            .map(|m| m.kimlik)
            .collect();
        let n = kimlikler.len();
        kimlikler.sort_unstable();
        kimlikler.dedup();
        assert_eq!(kimlikler.len(), n);
        assert!(kimlikler.iter().all(|k| *k >= 100), "ana menü 1..5");
    }

    #[test]
    fn kisayol_etiketi_gecerli_kisayolu_ya_da_varsayilani_gosterir() {
        let etiket = |a: &Ayarlar| {
            ayar_maddeleri(a, false)
                .into_iter()
                .find(|m| m.kimlik == KIMLIK_KISAYOL)
                .unwrap()
                .etiket
        };
        let mut a = Ayarlar::default();
        assert_eq!(etiket(&a), "Genel kısayol: Ctrl+Alt+L");
        a.kisayol_goster_gizle = "Ctrl+Shift+F12".into();
        assert_eq!(etiket(&a), "Genel kısayol: Ctrl+Shift+F12");
        a.kisayol_goster_gizle.clear();
        assert_eq!(etiket(&a), "Genel kısayol (Ctrl+Alt+L)");
    }

    #[test]
    fn secim_radyoda_sabit_onayda_tersine_cevirir() {
        let a = Ayarlar::default();
        assert_eq!(
            secim_degeri(KIMLIK_KUCULTME_GOREV, &a, false),
            Some(AyarDegeri::Kucultme(Kucultme::GorevCubugu))
        );
        assert_eq!(
            secim_degeri(KIMLIK_KUCULTME_TEPSI, &a, false),
            Some(AyarDegeri::Kucultme(Kucultme::Tepsi))
        );
        assert_eq!(
            secim_degeri(KIMLIK_TIK_CIFT, &a, false),
            Some(AyarDegeri::Tik(Tik::Cift))
        );
        assert_eq!(
            secim_degeri(KIMLIK_TIK_TEK, &a, false),
            Some(AyarDegeri::Tik(Tik::Tek))
        );
        assert_eq!(
            secim_degeri(KIMLIK_KAPALIYKEN_ACAR, &a, false),
            Some(AyarDegeri::KapaliykenTikAcar(false))
        );
        assert_eq!(
            secim_degeri(KIMLIK_OTURUMDA_GIZLI, &a, false),
            Some(AyarDegeri::OturumdaGizliBaslat(true))
        );
    }

    #[test]
    fn secim_kisayol_ac_kapa_ve_baslangicta_tersine() {
        let mut a = Ayarlar::default();
        assert_eq!(
            secim_degeri(KIMLIK_KISAYOL, &a, false),
            Some(AyarDegeri::KisayolGosterGizle(String::new())),
            "açıkken kapatır"
        );
        a.kisayol_goster_gizle.clear();
        assert_eq!(
            secim_degeri(KIMLIK_KISAYOL, &a, false),
            Some(AyarDegeri::KisayolGosterGizle("Ctrl+Alt+L".into())),
            "kapalıyken varsayılanı açar"
        );
        assert_eq!(
            secim_degeri(KIMLIK_BASLANGICTA, &a, false),
            Some(AyarDegeri::BaslangictaCalis(true))
        );
        assert_eq!(
            secim_degeri(KIMLIK_BASLANGICTA, &a, true),
            Some(AyarDegeri::BaslangictaCalis(false)),
            "gerçek kısayol durumuna göre (dosyaya değil)"
        );
        for k in [0, 1, 5, 99, 108, 1000] {
            assert_eq!(secim_degeri(k, &a, false), None, "{k}");
        }
    }
}
