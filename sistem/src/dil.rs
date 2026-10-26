//! Windows görüntü dili (`GetUserDefaultUILanguage`) ve etkin arayüz dili. Karar (`LANGID` →
//! `Dil`, `otomatik` çözümü) saf ve sınamalıdır: `logseq-tepsi-metin` içinde.

use logseq_tepsi_cekirdek::ayar;
use logseq_tepsi_cekirdek::metin::{Dil, DilTercihi};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

/// Kullanıcının Windows görüntü dili kimliği (`LANGID`).
pub fn sistem_langid() -> u16 {
    // SAFETY: argümansız, yalnız okuyan Win32 çağrısı.
    unsafe { GetUserDefaultUILanguage() }
}

/// `tercih`in etkin dili (`otomatik`: Windows görüntü dili).
pub fn dil_coz(tercih: DilTercihi) -> Dil {
    tercih.coz(sistem_langid())
}

/// Geçerli ayardaki etkin dil: menü, ipucu ve uçtan dönen nedenler bununla yazılır.
pub fn etkin_dil() -> Dil {
    dil_coz(ayar::guncel().dil)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sistem_dil_kimligi_okunur_ve_karar_birincil_kimlige_uyar() {
        let id = sistem_langid();
        assert_ne!(id, 0, "Windows görüntü dili okunamadı");
        assert_eq!(dil_coz(DilTercihi::Otomatik) == Dil::Tr, id & 0x3FF == 0x1F);
    }

    #[test]
    fn acik_secim_sistem_dilini_ezer() {
        assert_eq!(dil_coz(DilTercihi::Tr), Dil::Tr);
        assert_eq!(dil_coz(DilTercihi::En), Dil::En);
    }
}
