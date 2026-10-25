//! Tepsi menüsündeki "Ayarlar" alt menüsünün modeli (saf): hangi madde işaretli, bir maddeye
//! tıklanınca hangi ayar değişir. Win32 yalnız bu modeli çizer (`AppendMenuW`,
//! `CheckMenuRadioItem`); karar burada, sınamalı.

use crate::ayar::{
    AyarDegeri, Ayarlar, Kucultme, NotHedefi, Tik, VARSAYILAN_KISAYOL, VARSAYILAN_KISAYOL_NOT,
};

pub const KIMLIK_KUCULTME_TEPSI: usize = 100;
pub const KIMLIK_KUCULTME_GOREV: usize = 101;
pub const KIMLIK_TIK_TEK: usize = 102;
pub const KIMLIK_TIK_CIFT: usize = 103;
pub const KIMLIK_KAPALIYKEN_ACAR: usize = 104;
pub const KIMLIK_KISAYOL: usize = 105;
pub const KIMLIK_OTURUMDA_GIZLI: usize = 106;
pub const KIMLIK_BASLANGICTA: usize = 107;
pub const KIMLIK_NOT_KISAYOL: usize = 108;
pub const KIMLIK_NOT_GUNLUK: usize = 109;
pub const KIMLIK_NOT_YENI: usize = 110;
pub const KIMLIK_KALINAN_SAYFA: usize = 111;
pub const KIMLIK_API_OTOMATIK: usize = 112;
/// Ana menüdeki (Ayarlar alt menüsü dışı) ek maddeler.
pub const KIMLIK_ANINDA_NOT: usize = 120;
pub const KIMLIK_API_ACKAPA: usize = 121;

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
    /// Tıklanınca iş yapan madde (`etkin = false` ise soluk, seçilemez).
    Eylem {
        etkin: bool,
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
    let kisayol = kisayol_etiketi(
        "Genel kısayol",
        &a.kisayol_goster_gizle,
        VARSAYILAN_KISAYOL,
        &a.kisayol_not,
    );
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

/// Menü kurulurken tepsi aracının dışarıdan öğrendiği gerçek durumlar (ayar dosyasında değil).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisDurum {
    /// Başlangıç kısayolu şu an var mı.
    pub baslangicta: bool,
    /// `configs.edn`deki `:server/autostart` (gerçek değer).
    pub api_otomatik: bool,
    /// Logseq API sunucusu şu an çalışıyor mu (TCP yoklaması).
    pub api_acik: bool,
    pub logseq_acik: bool,
}

/// Tıklanınca yapılan ana menü işi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnaEylem {
    AnindaNot,
    /// `true`: sunucuyu başlat, `false`: durdur.
    ApiSunucusu(bool),
}

/// Kısayol maddesinin etiketi: açıksa kısayolu, kapalıysa varsayılanı gösterir (varsayılan diğer
/// kısayolla aynıysa açınca çakışacağı söylenir; açma isteği reddedilir).
fn kisayol_etiketi(onek: &str, kisayol: &str, varsayilan: &str, diger: &str) -> String {
    if kisayol.is_empty() && varsayilan == diger {
        format!("{onek} ({varsayilan} diğer kısayolla çakışır)")
    } else if kisayol.is_empty() {
        format!("{onek} ({varsayilan})")
    } else {
        format!("{onek}: {kisayol}")
    }
}

/// İstenen `api_otomatik_baslat`: bekleyen varsa o, yoksa `configs.edn`deki gerçek değer.
fn api_istenen(a: &Ayarlar, d: &DisDurum) -> bool {
    a.api_bekleyen.unwrap_or(d.api_otomatik)
}

/// "Ayarlar" alt menüsünün tüm maddeleri: `ayar_maddeleri` + anında not + kalınan sayfa + API.
pub fn ayar_maddeleri_tam(a: &Ayarlar, d: &DisDurum) -> Vec<Madde> {
    let mut m = ayar_maddeleri(a, d.baslangicta);
    let hedef = |kimlik, etiket, isaretli| {
        madde(
            kimlik,
            etiket,
            Tur::Radyo {
                isaretli,
                ilk: KIMLIK_NOT_GUNLUK,
                son: KIMLIK_NOT_YENI,
            },
        )
    };
    let api = if a.api_bekleyen.is_some() {
        "API sunucusu Logseq açılınca başlasın (Logseq kapanınca uygulanacak)"
    } else {
        "API sunucusu Logseq açılınca başlasın"
    };
    m.push(madde(0, "", Tur::Ayirici));
    m.push(madde(
        KIMLIK_NOT_KISAYOL,
        &kisayol_etiketi(
            "Anında not kısayolu",
            &a.kisayol_not,
            VARSAYILAN_KISAYOL_NOT,
            &a.kisayol_goster_gizle,
        ),
        Tur::Onay(!a.kisayol_not.is_empty()),
    ));
    m.push(hedef(
        KIMLIK_NOT_GUNLUK,
        "Anında not: günlük sayfasına ekle",
        a.not_hedefi == NotHedefi::Gunluk,
    ));
    m.push(hedef(
        KIMLIK_NOT_YENI,
        "Anında not: yeni sayfa aç",
        a.not_hedefi == NotHedefi::YeniSayfa,
    ));
    m.push(madde(
        KIMLIK_KALINAN_SAYFA,
        "Logseq açılınca kaldığım sayfaya dön",
        Tur::Onay(a.kalinan_sayfa),
    ));
    m.push(madde(
        KIMLIK_API_OTOMATIK,
        api,
        Tur::Onay(api_istenen(a, d)),
    ));
    m
}

/// `ayar_maddeleri_tam` maddesine tıklanınca yapılan ayar değişikliği (bilinmeyen: `None`).
pub fn secim_degeri_tam(kimlik: usize, a: &Ayarlar, d: &DisDurum) -> Option<AyarDegeri> {
    Some(match kimlik {
        KIMLIK_NOT_KISAYOL => AyarDegeri::KisayolNot(if a.kisayol_not.is_empty() {
            VARSAYILAN_KISAYOL_NOT.to_owned()
        } else {
            String::new()
        }),
        KIMLIK_NOT_GUNLUK => AyarDegeri::NotHedefi(NotHedefi::Gunluk),
        KIMLIK_NOT_YENI => AyarDegeri::NotHedefi(NotHedefi::YeniSayfa),
        KIMLIK_KALINAN_SAYFA => AyarDegeri::KalinanSayfa(!a.kalinan_sayfa),
        KIMLIK_API_OTOMATIK => AyarDegeri::ApiOtomatikBaslat(!api_istenen(a, d)),
        _ => return secim_degeri(kimlik, a, d.baslangicta),
    })
}

/// Ana menüye (Göster, Hepsini göster ... Tepsi aracını kapat) eklenen maddeler: ayırıcı,
/// Anında not, API sunucusu başlat/durdur (Logseq kapalıyken soluk, durumu etikette).
pub fn ana_ek_maddeleri(d: &DisDurum) -> Vec<Madde> {
    let (api, etkin) = match (d.logseq_acik, d.api_acik) {
        (true, true) => ("API sunucusunu durdur (deneysel, çalışıyor)", true),
        (true, false) => ("API sunucusunu başlat (deneysel, kapalı)", true),
        (false, _) => ("API sunucusu (deneysel, Logseq kapalı)", false),
    };
    vec![
        madde(0, "", Tur::Ayirici),
        madde(KIMLIK_ANINDA_NOT, "Anında not", Tur::Eylem { etkin: true }),
        madde(KIMLIK_API_ACKAPA, api, Tur::Eylem { etkin }),
    ]
}

/// Ana menü ek maddesine tıklanınca yapılacak iş (soluk madde ve bilinmeyen kimlik: `None`).
pub fn ana_secim(kimlik: usize, d: &DisDurum) -> Option<AnaEylem> {
    match kimlik {
        KIMLIK_ANINDA_NOT => Some(AnaEylem::AnindaNot),
        KIMLIK_API_ACKAPA if d.logseq_acik => Some(AnaEylem::ApiSunucusu(!d.api_acik)),
        _ => None,
    }
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
            ..Ayarlar::default()
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

    const DIS: DisDurum = DisDurum {
        baslangicta: false,
        api_otomatik: false,
        api_acik: false,
        logseq_acik: true,
    };

    fn isaretliler_tam(a: &Ayarlar, d: &DisDurum) -> Vec<usize> {
        ayar_maddeleri_tam(a, d)
            .iter()
            .filter(|m| matches!(m.tur, Tur::Onay(true) | Tur::Radyo { isaretli: true, .. }))
            .map(|m| m.kimlik)
            .collect()
    }

    #[test]
    fn tam_menu_eski_maddeleri_korur_yenileri_sona_ekler() {
        let a = Ayarlar::default();
        let eski = ayar_maddeleri(&a, false);
        let tam = ayar_maddeleri_tam(&a, &DIS);
        assert_eq!(&tam[..eski.len()], &eski[..]);
        let yeni: Vec<usize> = tam[eski.len()..]
            .iter()
            .filter(|m| m.tur != Tur::Ayirici)
            .map(|m| m.kimlik)
            .collect();
        assert_eq!(
            yeni,
            [
                KIMLIK_NOT_KISAYOL,
                KIMLIK_NOT_GUNLUK,
                KIMLIK_NOT_YENI,
                KIMLIK_KALINAN_SAYFA,
                KIMLIK_API_OTOMATIK
            ]
        );
        assert_eq!(tam[eski.len()].tur, Tur::Ayirici);
    }

    #[test]
    fn tam_menu_isaretleri_varsayilanda_ve_degisince() {
        let mut a = Ayarlar::default();
        assert_eq!(
            isaretliler_tam(&a, &DIS),
            vec![
                KIMLIK_KUCULTME_TEPSI,
                KIMLIK_TIK_TEK,
                KIMLIK_KAPALIYKEN_ACAR,
                KIMLIK_KISAYOL,
                KIMLIK_NOT_KISAYOL,
                KIMLIK_NOT_GUNLUK,
                KIMLIK_KALINAN_SAYFA,
            ]
        );
        a.not_hedefi = NotHedefi::YeniSayfa;
        a.kalinan_sayfa = false;
        a.kisayol_not.clear();
        let d = DisDurum {
            api_otomatik: true,
            ..DIS
        };
        let m = isaretliler_tam(&a, &d);
        assert!(m.contains(&KIMLIK_NOT_YENI) && !m.contains(&KIMLIK_NOT_GUNLUK));
        assert!(!m.contains(&KIMLIK_KALINAN_SAYFA) && !m.contains(&KIMLIK_NOT_KISAYOL));
        assert!(
            m.contains(&KIMLIK_API_OTOMATIK),
            "gerçek configs.edn değeri"
        );
    }

    #[test]
    fn api_otomatik_bekleyen_istenen_degeri_ve_etiketi_gosterir() {
        let a = Ayarlar {
            api_bekleyen: Some(true),
            ..Ayarlar::default()
        };
        // Gerçek değer hayır ama bekleyen evet: işaretli ve etiket uyarır.
        let m = ayar_maddeleri_tam(&a, &DIS);
        let api = m.iter().find(|m| m.kimlik == KIMLIK_API_OTOMATIK).unwrap();
        assert_eq!(api.tur, Tur::Onay(true));
        assert!(api.etiket.contains("Logseq kapanınca uygulanacak"));
        // Tıklayınca istenen değerin tersi (bekleyen evetken hayır istenir).
        assert_eq!(
            secim_degeri_tam(KIMLIK_API_OTOMATIK, &a, &DIS),
            Some(AyarDegeri::ApiOtomatikBaslat(false))
        );
        // Bekleyen yokken etiket sade.
        let sade = ayar_maddeleri_tam(&Ayarlar::default(), &DIS);
        let e = &sade
            .iter()
            .find(|m| m.kimlik == KIMLIK_API_OTOMATIK)
            .unwrap()
            .etiket;
        assert!(!e.contains("uygulanacak"));
    }

    #[test]
    fn tam_secimler_yeni_maddeleri_cozer_eskiyi_devreder() {
        let a = Ayarlar::default();
        assert_eq!(
            secim_degeri_tam(KIMLIK_NOT_KISAYOL, &a, &DIS),
            Some(AyarDegeri::KisayolNot(String::new())),
            "açıkken kapatır"
        );
        let kapali = Ayarlar {
            kisayol_not: String::new(),
            ..Ayarlar::default()
        };
        assert_eq!(
            secim_degeri_tam(KIMLIK_NOT_KISAYOL, &kapali, &DIS),
            Some(AyarDegeri::KisayolNot("Ctrl+Alt+N".into()))
        );
        assert_eq!(
            secim_degeri_tam(KIMLIK_NOT_YENI, &a, &DIS),
            Some(AyarDegeri::NotHedefi(NotHedefi::YeniSayfa))
        );
        assert_eq!(
            secim_degeri_tam(KIMLIK_NOT_GUNLUK, &a, &DIS),
            Some(AyarDegeri::NotHedefi(NotHedefi::Gunluk))
        );
        assert_eq!(
            secim_degeri_tam(KIMLIK_KALINAN_SAYFA, &a, &DIS),
            Some(AyarDegeri::KalinanSayfa(false))
        );
        assert_eq!(
            secim_degeri_tam(KIMLIK_API_OTOMATIK, &a, &DIS),
            Some(AyarDegeri::ApiOtomatikBaslat(true))
        );
        let acik = DisDurum {
            api_otomatik: true,
            ..DIS
        };
        assert_eq!(
            secim_degeri_tam(KIMLIK_API_OTOMATIK, &a, &acik),
            Some(AyarDegeri::ApiOtomatikBaslat(false))
        );
        // Eski maddeler eski işlevden geçer.
        assert_eq!(
            secim_degeri_tam(KIMLIK_TIK_CIFT, &a, &DIS),
            Some(AyarDegeri::Tik(Tik::Cift))
        );
        assert_eq!(
            secim_degeri_tam(
                KIMLIK_BASLANGICTA,
                &a,
                &DisDurum {
                    baslangicta: true,
                    ..DIS
                }
            ),
            Some(AyarDegeri::BaslangictaCalis(false))
        );
        for k in [0, 1, 99, 113, 120, 121, 1000] {
            assert_eq!(secim_degeri_tam(k, &a, &DIS), None, "{k}");
        }
    }

    #[test]
    fn not_kisayol_etiketi_gecerli_ya_da_varsayilan() {
        let etiket = |a: &Ayarlar| {
            ayar_maddeleri_tam(a, &DIS)
                .into_iter()
                .find(|m| m.kimlik == KIMLIK_NOT_KISAYOL)
                .unwrap()
                .etiket
        };
        let mut a = Ayarlar::default();
        assert_eq!(etiket(&a), "Anında not kısayolu: Ctrl+Alt+N");
        a.kisayol_not = "Alt+Win+N".into();
        assert_eq!(etiket(&a), "Anında not kısayolu: Alt+Win+N");
        a.kisayol_not.clear();
        assert_eq!(etiket(&a), "Anında not kısayolu (Ctrl+Alt+N)");
    }

    #[test]
    fn ana_menu_ek_maddeleri_api_durumuna_gore() {
        let metin = |d: &DisDurum| {
            ana_ek_maddeleri(d)
                .into_iter()
                .find(|m| m.kimlik == KIMLIK_API_ACKAPA)
                .map(|m| (m.etiket, m.tur))
                .unwrap()
        };
        assert_eq!(
            metin(&DIS),
            (
                "API sunucusunu başlat (deneysel, kapalı)".to_owned(),
                Tur::Eylem { etkin: true }
            )
        );
        let acik = DisDurum {
            api_acik: true,
            ..DIS
        };
        assert_eq!(
            metin(&acik),
            (
                "API sunucusunu durdur (deneysel, çalışıyor)".to_owned(),
                Tur::Eylem { etkin: true }
            )
        );
        let kapali_logseq = DisDurum {
            logseq_acik: false,
            api_acik: false,
            ..DIS
        };
        assert_eq!(
            metin(&kapali_logseq),
            (
                "API sunucusu (deneysel, Logseq kapalı)".to_owned(),
                Tur::Eylem { etkin: false }
            ),
            "Logseq kapalıyken soluk"
        );
        let m = ana_ek_maddeleri(&DIS);
        assert_eq!(m[0].tur, Tur::Ayirici);
        assert_eq!(m[1].kimlik, KIMLIK_ANINDA_NOT);
        assert_eq!(
            m[1].tur,
            Tur::Eylem { etkin: true },
            "Logseq kapalıyken de başlatır"
        );
    }

    #[test]
    fn ana_secim_eylemleri_ve_soluk_madde() {
        assert_eq!(
            ana_secim(KIMLIK_ANINDA_NOT, &DIS),
            Some(AnaEylem::AnindaNot)
        );
        assert_eq!(
            ana_secim(KIMLIK_API_ACKAPA, &DIS),
            Some(AnaEylem::ApiSunucusu(true)),
            "kapalıyken başlat"
        );
        let acik = DisDurum {
            api_acik: true,
            ..DIS
        };
        assert_eq!(
            ana_secim(KIMLIK_API_ACKAPA, &acik),
            Some(AnaEylem::ApiSunucusu(false)),
            "açıkken durdur"
        );
        let logseq_kapali = DisDurum {
            logseq_acik: false,
            ..DIS
        };
        assert_eq!(ana_secim(KIMLIK_API_ACKAPA, &logseq_kapali), None);
        assert_eq!(
            ana_secim(KIMLIK_ANINDA_NOT, &logseq_kapali),
            Some(AnaEylem::AnindaNot)
        );
        for k in [0, 1, 5, 100, 112, 122] {
            assert_eq!(ana_secim(k, &DIS), None, "{k}");
        }
    }

    #[test]
    fn tum_menu_kimlikleri_benzersiz_ve_mantik_kimlikleriyle_cakismaz() {
        let mut k: Vec<usize> = ayar_maddeleri_tam(&Ayarlar::default(), &DIS)
            .into_iter()
            .chain(ana_ek_maddeleri(&DIS))
            .filter(|m| m.tur != Tur::Ayirici)
            .map(|m| m.kimlik)
            .collect();
        let n = k.len();
        k.sort_unstable();
        k.dedup();
        assert_eq!(k.len(), n, "kimlikler benzersiz");
        for kimlik in &k {
            assert!(
                crate::mantik::menu_eylemi(*kimlik).is_none(),
                "{kimlik} ana menü eylemleriyle çakışıyor"
            );
        }
    }
    #[test]
    fn kapaliyken_varsayilan_diger_kisayolla_cakisiyorsa_etiket_uyarir() {
        let etiket = |a: &Ayarlar, kimlik| {
            ayar_maddeleri_tam(a, &DIS)
                .into_iter()
                .find(|m| m.kimlik == kimlik)
                .unwrap()
                .etiket
        };
        let a = Ayarlar {
            kisayol_goster_gizle: String::new(),
            kisayol_not: VARSAYILAN_KISAYOL.into(),
            ..Ayarlar::default()
        };
        assert_eq!(
            etiket(&a, KIMLIK_KISAYOL),
            "Genel kısayol (Ctrl+Alt+L diğer kısayolla çakışır)"
        );
        let b = Ayarlar {
            kisayol_not: String::new(),
            kisayol_goster_gizle: VARSAYILAN_KISAYOL_NOT.into(),
            ..Ayarlar::default()
        };
        assert_eq!(
            etiket(&b, KIMLIK_NOT_KISAYOL),
            "Anında not kısayolu (Ctrl+Alt+N diğer kısayolla çakışır)"
        );
        // Çakışma yoksa eski etiket.
        let c = Ayarlar {
            kisayol_not: String::new(),
            ..Ayarlar::default()
        };
        assert_eq!(
            etiket(&c, KIMLIK_NOT_KISAYOL),
            "Anında not kısayolu (Ctrl+Alt+N)"
        );
    }
}
