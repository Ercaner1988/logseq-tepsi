//! Parçalardan kurulan metinler: tepsi ipucu ve kısayol etiketi.

use crate::{bicim, metin, Dil, Metin};

/// Tepsi ipucunun dayandığı durumlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpucuDurumu {
    pub logseq_acik: bool,
    /// `kapaliyken_tik_acar` ayarı.
    pub kapaliyken_tik_acar: bool,
    /// `tik = cift` ayarı.
    pub cift_tik: bool,
    /// Küçültme kancası kurulu mu.
    pub kanca_tamam: bool,
    /// Bir genel kısayolun kaydı başarısız mı.
    pub kisayol_hatali: bool,
    /// İki kısayol aynı tuşa ayarlı mı (kayıt hatasının tanısı).
    pub kisayollar_cakisiyor: bool,
}

/// Tepsi ipucu: Logseq durumu, tık ayarı, izlenemeyen küçültme ve kaydedilemeyen kısayol.
/// (Windows ipucu sınırı 127 UTF-16 birimi; her birleşim iki dilde sınanır.)
pub fn ipucu(dil: Dil, d: &IpucuDurumu) -> String {
    let m = |m| metin(dil, m);
    let mut s = if d.logseq_acik {
        m(Metin::IpucuAcik)
    } else if !d.kapaliyken_tik_acar {
        m(Metin::IpucuKapali)
    } else if d.cift_tik {
        m(Metin::IpucuKapaliCift)
    } else {
        m(Metin::IpucuKapaliTek)
    }
    .to_owned();
    let mut uyarilar = Vec::new();
    if !d.kanca_tamam {
        uyarilar.push(m(Metin::IpucuKancaYok));
    }
    if d.kisayol_hatali {
        // İki kısayol aynı tuşa ayarlıysa ikincisi kaydedilemez: tanı "başka program" değil, çakışma.
        uyarilar.push(m(if d.kisayollar_cakisiyor {
            Metin::IpucuKisayolCakisir
        } else {
            Metin::IpucuKisayolHatali
        }));
    }
    if !uyarilar.is_empty() {
        s.push_str(&format!(" ({})", uyarilar.join(", ")));
    }
    s
}

/// Kısayol maddesinin etiketi: açıksa kısayolu, kapalıysa varsayılanı gösterir (varsayılan diğer
/// kısayolla aynıysa açınca çakışacağı söylenir; açma isteği reddedilir). `onek`: `AyKisayolGenel`
/// ya da `AyKisayolNot`.
pub fn kisayol_etiketi(
    dil: Dil,
    onek: Metin,
    kisayol: &str,
    varsayilan: &str,
    diger: &str,
) -> String {
    let onek = metin(dil, onek);
    if kisayol.is_empty() && varsayilan == diger {
        bicim(dil, Metin::AyKisayolCakisir, &[onek, varsayilan])
    } else if kisayol.is_empty() {
        format!("{onek} ({varsayilan})")
    } else {
        format!("{onek}: {kisayol}")
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const DIL: [Dil; 2] = [Dil::Tr, Dil::En];

    fn durum(acik: bool, kanca: bool) -> IpucuDurumu {
        IpucuDurumu {
            logseq_acik: acik,
            kapaliyken_tik_acar: true,
            cift_tik: false,
            kanca_tamam: kanca,
            kisayol_hatali: false,
            kisayollar_cakisiyor: false,
        }
    }

    #[test]
    fn ipucu_turkce_eski_metinlerle_ayni() {
        let i = |d: IpucuDurumu| ipucu(Dil::Tr, &d);
        assert_eq!(i(durum(true, true)), "Logseq");
        assert_eq!(i(durum(true, false)), "Logseq (küçültme izlenemiyor)");
        assert_eq!(i(durum(false, true)), "Logseq kapalı, tıklayınca açılır");
        assert_eq!(
            i(durum(false, false)),
            "Logseq kapalı, tıklayınca açılır (küçültme izlenemiyor)"
        );
        let cift = IpucuDurumu {
            cift_tik: true,
            ..durum(false, true)
        };
        assert_eq!(i(cift), "Logseq kapalı, çift tıklayınca açılır");
        let acmaz = IpucuDurumu {
            kapaliyken_tik_acar: false,
            ..durum(false, true)
        };
        assert_eq!(i(acmaz), "Logseq kapalı");
    }

    #[test]
    fn ipucu_ingilizce() {
        let i = |d: IpucuDurumu| ipucu(Dil::En, &d);
        assert_eq!(i(durum(true, true)), "Logseq");
        assert_eq!(i(durum(false, true)), "Logseq closed, click to open");
        let cift = IpucuDurumu {
            cift_tik: true,
            kisayol_hatali: true,
            kisayollar_cakisiyor: true,
            ..durum(false, false)
        };
        assert_eq!(
            i(cift),
            "Logseq closed, double-click to open (minimize tracking unavailable, shortcuts conflict)"
        );
        let hata = IpucuDurumu {
            kisayol_hatali: true,
            ..durum(true, true)
        };
        assert_eq!(i(hata), "Logseq (shortcut registration failed)");
    }

    #[test]
    fn ipucu_her_birlesim_iki_dilde_127_birimin_altinda() {
        for dil in DIL {
            for bits in 0u8..64 {
                let b = |n: u8| bits & (1 << n) != 0;
                let d = IpucuDurumu {
                    logseq_acik: b(0),
                    kapaliyken_tik_acar: b(1),
                    cift_tik: b(2),
                    kanca_tamam: b(3),
                    kisayol_hatali: b(4),
                    kisayollar_cakisiyor: b(5),
                };
                let s = ipucu(dil, &d);
                assert!(s.encode_utf16().count() < 127, "{dil:?} {s}");
            }
        }
    }

    #[test]
    fn kisayol_etiketi_uc_durum_iki_dilde() {
        let e = |dil, k: &str, diger: &str| {
            kisayol_etiketi(dil, Metin::AyKisayolGenel, k, "Ctrl+Alt+L", diger)
        };
        assert_eq!(e(Dil::Tr, "Alt+X", "Ctrl+Alt+N"), "Genel kısayol: Alt+X");
        assert_eq!(e(Dil::En, "Alt+X", "Ctrl+Alt+N"), "Global shortcut: Alt+X");
        assert_eq!(e(Dil::Tr, "", "Ctrl+Alt+N"), "Genel kısayol (Ctrl+Alt+L)");
        assert_eq!(e(Dil::En, "", "Ctrl+Alt+N"), "Global shortcut (Ctrl+Alt+L)");
        assert_eq!(
            e(Dil::Tr, "", "Ctrl+Alt+L"),
            "Genel kısayol (Ctrl+Alt+L diğer kısayolla çakışır)"
        );
        assert_eq!(
            e(Dil::En, "", "Ctrl+Alt+L"),
            "Global shortcut (Ctrl+Alt+L conflicts with the other shortcut)"
        );
    }
}
