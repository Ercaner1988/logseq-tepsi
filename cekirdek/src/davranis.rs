//! Ayarların davranışa çevrilmesi (saf): küçültme politikası, tık türü, kapalıyken tık, tepsi
//! ipucu, oturumda gizli başlatma. Temel kararlar `mantik`te kalır; burada yalnız ayara bağlı
//! sarmalayıcılar var (varsayılan ayarla eski davranışla birebir aynıdır, sınamalı).

use crate::ayar::{Ayarlar, Kucultme, Tik};
use crate::mantik::{
    acilis_karari, kucultme_karari, tepsi_olayi, AcilisEylemi, KucultmeKarari, PencereDurumu,
    TepsiOlayi, Toplu,
};

/// Çift tık (`NIM_SETVERSION` yok, sürüm 0 geri çağrısı: doğrudan fare iletisi).
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;

/// `kucultme = gorev_cubugu` iken küçültme kancası gizlemez; pencere görev çubuğunda kalır.
pub fn kucultme_karari_ayarli(
    kucultme: Kucultme,
    simge_durumunda: bool,
    deneme: u32,
) -> KucultmeKarari {
    match kucultme {
        Kucultme::GorevCubugu => KucultmeKarari::Birak,
        Kucultme::Tepsi => kucultme_karari(simge_durumunda, deneme),
    }
}

/// Açılış taraması: görev çubuğu kipinde küçük pencere gizlenmez (zaten gizli olan yine hatırlanır).
pub fn acilis_karari_ayarli(kucultme: Kucultme, d: PencereDurumu) -> AcilisEylemi {
    match (kucultme, acilis_karari(d)) {
        (Kucultme::GorevCubugu, AcilisEylemi::Gizle) => AcilisEylemi::Yok,
        (_, eylem) => eylem,
    }
}

/// Tepsi iletisinin tık türüne göre çözümü: `tek` kipinde yalnız sol tık bırakması (çift tığın
/// ikinci bırakmasını çağıran süzer), `cift` kipinde yalnız çift tık `SolTik` sayılır; seçilmeyen
/// tık türü göster/gizle yapmaz. Sağ tık (menü) her kipte aynı.
pub fn tepsi_eylemi(lparam: isize, tik: Tik) -> TepsiOlayi {
    let olay = tepsi_olayi(lparam);
    let cift = (lparam & 0xFFFF) as u32 == WM_LBUTTONDBLCLK;
    match (tik, olay) {
        (Tik::Cift, TepsiOlayi::SolTik) => TepsiOlayi::Yok,
        (Tik::Cift, TepsiOlayi::Yok) if cift => TepsiOlayi::SolTik,
        (_, olay) => olay,
    }
}

/// Tık sonucu `Toplu::Baslat` ise (Logseq kapalı) ve `kapaliyken_tik_acar = hayir` ise hiçbir şey
/// yapılmaz. Menüdeki "Göster" ve genel kısayol bu süzgeçten geçmez (Logseq'i yine başlatır).
pub fn tik_toplu_ayarli(toplu: Toplu, kapaliyken_tik_acar: bool) -> Option<Toplu> {
    match toplu {
        Toplu::Baslat if !kapaliyken_tik_acar => None,
        t => Some(t),
    }
}

/// Tepsi ipucu: Logseq durumu, tık ayarı, izlenemeyen küçültme ve kaydedilemeyen kısayol.
/// (Windows ipucu sınırı 127 UTF-16 birimi; sınamalı.)
pub fn ipucu_ayarli(
    logseq_acik: bool,
    kanca_tamam: bool,
    ayar: &Ayarlar,
    kisayol_hatali: bool,
) -> String {
    let mut s = if logseq_acik {
        "Logseq".to_owned()
    } else if !ayar.kapaliyken_tik_acar {
        "Logseq kapalı".to_owned()
    } else {
        let tik = match ayar.tik {
            Tik::Tek => "tıklayınca",
            Tik::Cift => "çift tıklayınca",
        };
        format!("Logseq kapalı, {tik} açılır")
    };
    let mut uyarilar = Vec::new();
    if !kanca_tamam {
        uyarilar.push("küçültme izlenemiyor");
    }
    if kisayol_hatali {
        uyarilar.push("kısayol kaydedilemedi");
    }
    if !uyarilar.is_empty() {
        s.push_str(&format!(" ({})", uyarilar.join(", ")));
    }
    s
}

/// Oturum açılışında gizli başlatma yalnız hem ayar açıksa hem araç başlangıç kısayolundan
/// (`--oturum`) başlamışsa çalışır.
pub fn oturum_gizli_mi(oturum_argumani: bool, ayar: &Ayarlar) -> bool {
    oturum_argumani && ayar.oturumda_gizli_baslat
}

/// Logseq ana penceresini görünce gizlemek için en çok beklenecek süre.
pub const OTURUM_TAVAN_MS: u64 = 90_000;
/// Ana pencere yoklama aralığı.
pub const OTURUM_YOKLAMA_MS: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OturumAdimi {
    /// Görünür ana pencere var: gizle, yoklamayı bitir.
    Gizle,
    Bekle,
    /// Süre doldu, pencere görünmedi: vazgeç.
    Vazgec,
}

/// Bir yoklama adımının kararı (`gecen_ms`: başlangıçtan beri). Pencere görünüyorsa süre dolmuş
/// olsa bile gizlenir.
pub fn oturum_adimi(gorunur_ana_sayisi: usize, gecen_ms: u64) -> OturumAdimi {
    if gorunur_ana_sayisi > 0 {
        OturumAdimi::Gizle
    } else if gecen_ms >= OTURUM_TAVAN_MS {
        OturumAdimi::Vazgec
    } else {
        OturumAdimi::Bekle
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::mantik::{tepsi_ipucu, WM_LBUTTONUP, WM_RBUTTONUP};

    const GIZLI: PencereDurumu = PencereDurumu {
        gorunur: false,
        simge_durumunda: false,
    };
    const GORUNUR_KUCUK: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: true,
    };
    const GORUNUR: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: false,
    };

    #[test]
    fn kucultme_tepsi_kipinde_eski_karar_gorev_cubugunda_hep_birak() {
        for (ic, deneme) in [(true, 0), (false, 0), (true, 1), (false, 1)] {
            assert_eq!(
                kucultme_karari_ayarli(Kucultme::Tepsi, ic, deneme),
                kucultme_karari(ic, deneme),
                "varsayılan kip eski davranışla aynı"
            );
            assert_eq!(
                kucultme_karari_ayarli(Kucultme::GorevCubugu, ic, deneme),
                KucultmeKarari::Birak,
                "görev çubuğu kipi gizlemez ({ic}, {deneme})"
            );
        }
        assert_eq!(
            kucultme_karari_ayarli(Kucultme::Tepsi, true, 0),
            KucultmeKarari::Gizle
        );
    }

    #[test]
    fn acilis_taramasi_gorev_cubugunda_kucugu_gizlemez_gizliyi_hatirlar() {
        assert_eq!(
            acilis_karari_ayarli(Kucultme::Tepsi, GORUNUR_KUCUK),
            AcilisEylemi::Gizle
        );
        assert_eq!(
            acilis_karari_ayarli(Kucultme::GorevCubugu, GORUNUR_KUCUK),
            AcilisEylemi::Yok
        );
        for k in [Kucultme::Tepsi, Kucultme::GorevCubugu] {
            assert_eq!(acilis_karari_ayarli(k, GIZLI), AcilisEylemi::Hatirla);
            assert_eq!(acilis_karari_ayarli(k, GORUNUR), AcilisEylemi::Yok);
        }
    }

    #[test]
    fn tek_kipte_eski_davranis_cift_tik_yok_sayilir() {
        assert_eq!(
            tepsi_eylemi(WM_LBUTTONUP as isize, Tik::Tek),
            TepsiOlayi::SolTik
        );
        assert_eq!(
            tepsi_eylemi(WM_LBUTTONDBLCLK as isize, Tik::Tek),
            TepsiOlayi::Yok
        );
        assert_eq!(
            tepsi_eylemi(WM_RBUTTONUP as isize, Tik::Tek),
            TepsiOlayi::SagTik
        );
        // lParam üst kelimesi (simge kimliği) önemsiz.
        assert_eq!(
            tepsi_eylemi(0x0001_0000 | 0x0202, Tik::Tek),
            TepsiOlayi::SolTik
        );
    }

    #[test]
    fn cift_kipte_yalniz_cift_tik_gizler_gosterir_tek_tik_yapmaz() {
        assert_eq!(
            tepsi_eylemi(WM_LBUTTONUP as isize, Tik::Cift),
            TepsiOlayi::Yok,
            "tek tık göster/gizle yapmamalı"
        );
        assert_eq!(
            tepsi_eylemi(WM_LBUTTONDBLCLK as isize, Tik::Cift),
            TepsiOlayi::SolTik
        );
        assert_eq!(
            tepsi_eylemi(0x0001_0000 | WM_LBUTTONDBLCLK as isize, Tik::Cift),
            TepsiOlayi::SolTik
        );
        assert_eq!(
            tepsi_eylemi(WM_RBUTTONUP as isize, Tik::Cift),
            TepsiOlayi::SagTik,
            "sağ tık menüsü her kipte açılır"
        );
        for l in [0x0201, 0x0204, 0x0200, 0] {
            assert_eq!(tepsi_eylemi(l, Tik::Cift), TepsiOlayi::Yok, "{l:#x}");
        }
    }

    #[test]
    fn kapaliyken_tik_acmaz_yalniz_baslat_susar() {
        assert_eq!(tik_toplu_ayarli(Toplu::Baslat, true), Some(Toplu::Baslat));
        assert_eq!(tik_toplu_ayarli(Toplu::Baslat, false), None);
        for t in [Toplu::Goster(7), Toplu::Gizle(vec![1, 2])] {
            assert_eq!(tik_toplu_ayarli(t.clone(), false), Some(t.clone()));
            assert_eq!(tik_toplu_ayarli(t.clone(), true), Some(t));
        }
    }

    #[test]
    fn ipucu_varsayilanda_eski_metinle_ayni() {
        let a = Ayarlar::default();
        for (acik, kanca) in [(true, true), (true, false), (false, true), (false, false)] {
            assert_eq!(
                ipucu_ayarli(acik, kanca, &a, false),
                tepsi_ipucu(acik, kanca),
                "acik={acik} kanca={kanca}"
            );
        }
    }

    #[test]
    fn ipucu_ayara_ve_kisayol_hatasina_gore() {
        let mut a = Ayarlar {
            tik: Tik::Cift,
            ..Ayarlar::default()
        };
        assert_eq!(
            ipucu_ayarli(false, true, &a, false),
            "Logseq kapalı, çift tıklayınca açılır"
        );
        a.kapaliyken_tik_acar = false;
        assert_eq!(ipucu_ayarli(false, true, &a, false), "Logseq kapalı");
        assert_eq!(ipucu_ayarli(true, true, &a, false), "Logseq");
        assert_eq!(
            ipucu_ayarli(true, true, &a, true),
            "Logseq (kısayol kaydedilemedi)"
        );
        assert_eq!(
            ipucu_ayarli(false, false, &a, true),
            "Logseq kapalı (küçültme izlenemiyor, kısayol kaydedilemedi)"
        );
        // Her birleşim 127 UTF-16 birimin altında.
        for tik in [Tik::Tek, Tik::Cift] {
            for acar in [true, false] {
                for (acik, kanca, hata) in [
                    (true, true, true),
                    (false, false, true),
                    (false, true, false),
                ] {
                    let ay = Ayarlar {
                        tik,
                        kapaliyken_tik_acar: acar,
                        ..Ayarlar::default()
                    };
                    let s = ipucu_ayarli(acik, kanca, &ay, hata);
                    assert!(s.encode_utf16().count() < 127, "{s}");
                }
            }
        }
    }

    #[test]
    fn oturum_yalniz_arguman_ve_ayar_birlikte() {
        let kapali = Ayarlar::default();
        let acik = Ayarlar {
            oturumda_gizli_baslat: true,
            ..Ayarlar::default()
        };
        assert!(oturum_gizli_mi(true, &acik));
        assert!(
            !oturum_gizli_mi(false, &acik),
            "argümansız (elle açılış) gizlemez"
        );
        assert!(!oturum_gizli_mi(true, &kapali), "ayar kapalıyken gizlemez");
        assert!(!oturum_gizli_mi(false, &kapali));
    }

    #[test]
    fn oturum_adimi_gizle_bekle_vazgec() {
        assert_eq!(oturum_adimi(0, 0), OturumAdimi::Bekle);
        assert_eq!(oturum_adimi(0, OTURUM_TAVAN_MS - 1), OturumAdimi::Bekle);
        assert_eq!(oturum_adimi(0, OTURUM_TAVAN_MS), OturumAdimi::Vazgec);
        assert_eq!(oturum_adimi(1, 5_000), OturumAdimi::Gizle);
        assert_eq!(
            oturum_adimi(2, OTURUM_TAVAN_MS + 1),
            OturumAdimi::Gizle,
            "süre dolsa da görünen pencere gizlenir"
        );
        assert_eq!(OTURUM_TAVAN_MS, 90_000, "belirtilen tavan ~90 sn");
    }
}
