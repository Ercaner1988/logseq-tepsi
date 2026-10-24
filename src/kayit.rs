//! Kayıt defteri okuma/yazma yardımcıları (Win32). Ayrıştırma mantığı `yollar`/`kurulum`
//! modüllerinde saf kodda; burası yalnız ince bir kabuk.

use crate::yollar::{KayitAdayi, KayitDegerleri};
use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_WOW: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// REG_SZ/REG_EXPAND_SZ (genişletilmiş) değeri okur; yoksa `None`.
pub fn deger_oku(kok: HKEY, alt: &str, ad: &str) -> Option<String> {
    let (alt_g, ad_g) = (genis(alt), genis(ad));
    let alt_p = if alt.is_empty() {
        null()
    } else {
        alt_g.as_ptr()
    };
    let ad_p = if ad.is_empty() { null() } else { ad_g.as_ptr() };
    let bayrak = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    let mut bayt = 0u32;
    // SAFETY: tamponlar çağrı süresince yaşar; boyut önce sorulur.
    unsafe {
        if RegGetValueW(kok, alt_p, ad_p, bayrak, null_mut(), null_mut(), &mut bayt) != 0 {
            return None;
        }
        let mut tampon = vec![0u16; (bayt as usize).div_ceil(2) + 1];
        let mut boyut = (tampon.len() * 2) as u32;
        if RegGetValueW(
            kok,
            alt_p,
            ad_p,
            bayrak,
            null_mut(),
            tampon.as_mut_ptr().cast(),
            &mut boyut,
        ) != 0
        {
            return None;
        }
        let uzunluk = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
        Some(String::from_utf16_lossy(&tampon[..uzunluk]))
    }
}

fn alt_anahtarlar(kok: HKEY, yol: &str) -> Vec<String> {
    let mut adlar = Vec::new();
    let yol_g = genis(yol);
    let mut anahtar: HKEY = null_mut();
    // SAFETY: standart Reg* kullanımı; tutaç her yolda kapatılır.
    unsafe {
        if RegOpenKeyExW(kok, yol_g.as_ptr(), 0, KEY_READ, &mut anahtar) != 0 {
            return adlar;
        }
        for i in 0.. {
            let mut tampon = [0u16; 256];
            let mut uzunluk = tampon.len() as u32;
            let hata = RegEnumKeyExW(
                anahtar,
                i,
                tampon.as_mut_ptr(),
                &mut uzunluk,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
            );
            if hata != 0 {
                break;
            }
            adlar.push(String::from_utf16_lossy(&tampon[..uzunluk as usize]));
        }
        RegCloseKey(anahtar);
    }
    adlar
}

/// HKCU, HKLM ve HKLM(WOW6432Node) Uninstall anahtarlarındaki Logseq girdilerinin
/// `Logseq.exe` yol adayları, girdinin `DisplayVersion` değeriyle (tarama sırasıyla; sürüme göre
/// sıralama ve ayrıştırma saf kodda: `yollar::adaylar`, `yollar::surumlu_kayit_adaylari`).
pub fn uninstall_adaylari() -> Vec<KayitAdayi> {
    let mut sonuc = Vec::new();
    for (kok, yol) in [
        (HKEY_CURRENT_USER, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL_WOW),
    ] {
        for anahtar in alt_anahtarlar(kok, yol) {
            let alt = format!(r"{yol}\{anahtar}");
            let ad = deger_oku(kok, &alt, "DisplayName");
            if !crate::yollar::logseq_girdisi_mi(&anahtar, ad.as_deref()) {
                continue;
            }
            let d = KayitDegerleri {
                gorunen_ad: ad,
                kurulum_konumu: deger_oku(kok, &alt, "InstallLocation"),
                gorunen_simge: deger_oku(kok, &alt, "DisplayIcon"),
                kaldirma_komutu: deger_oku(kok, &alt, "UninstallString"),
                gorunen_surum: deger_oku(kok, &alt, "DisplayVersion"),
                anahtar_adi: anahtar,
            };
            sonuc.extend(crate::yollar::surumlu_kayit_adaylari(&d));
        }
    }
    sonuc
}

/// HKCU altında REG_SZ değer yazar (anahtar yoksa oluşturur). Hata: Win32 hata kodu.
pub fn hkcu_deger_yaz(alt: &str, ad: &str, veri: &str) -> Result<(), u32> {
    let (alt_g, ad_g, veri_g) = (genis(alt), genis(ad), genis(veri));
    let mut anahtar: HKEY = null_mut();
    // SAFETY: tamponlar çağrı süresince yaşar; tutaç kapatılır.
    unsafe {
        let h = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            alt_g.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            null(),
            &mut anahtar,
            null_mut(),
        );
        if h != 0 {
            return Err(h);
        }
        let ad_p = if ad.is_empty() { null() } else { ad_g.as_ptr() };
        let h = RegSetValueExW(
            anahtar,
            ad_p,
            0,
            REG_SZ,
            veri_g.as_ptr().cast(),
            (veri_g.len() * 2) as u32,
        );
        RegCloseKey(anahtar);
        if h == 0 {
            Ok(())
        } else {
            Err(h)
        }
    }
}

/// HKCU altındaki anahtarı alt ağacıyla siler. Anahtar yoksa `Ok(false)`.
pub fn hkcu_agac_sil(alt: &str) -> Result<bool, u32> {
    const BULUNAMADI: u32 = 2;
    let alt_g = genis(alt);
    // SAFETY: tek çağrı; tampon yaşıyor.
    let h = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, alt_g.as_ptr()) };
    match h {
        0 => Ok(true),
        BULUNAMADI => Ok(false),
        e => Err(e),
    }
}

/// HKCU değeri (kurulum idempotentliği için).
pub fn hkcu_deger_oku(alt: &str, ad: &str) -> Option<String> {
    deger_oku(HKEY_CURRENT_USER, alt, ad)
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Gerçek kayıt defterini dener ama yalnız kendi geçici HKCU anahtarında.
    #[test]
    fn hkcu_yaz_oku_sil_gecici_anahtar() {
        let kok = format!(r"Software\logseq-tepsi-sina-{}", std::process::id());
        let komut = format!(r"{kok}\shell\open\command");
        hkcu_deger_yaz(&kok, "", "URL:deneme").unwrap();
        hkcu_deger_yaz(&kok, "URL Protocol", "").unwrap();
        hkcu_deger_yaz(&komut, "", r#""C:\a b\x.exe" --url "%1""#).unwrap();
        assert_eq!(hkcu_deger_oku(&kok, "").as_deref(), Some("URL:deneme"));
        assert_eq!(hkcu_deger_oku(&kok, "URL Protocol").as_deref(), Some(""));
        assert_eq!(
            hkcu_deger_oku(&komut, "").as_deref(),
            Some(r#""C:\a b\x.exe" --url "%1""#)
        );
        assert_eq!(hkcu_deger_oku(&kok, "yok"), None);
        assert_eq!(hkcu_agac_sil(&kok), Ok(true));
        assert_eq!(hkcu_deger_oku(&komut, ""), None);
        assert_eq!(hkcu_agac_sil(&kok), Ok(false));
    }

    #[test]
    fn uninstall_taramasi_cokmez() {
        // Makineye bağlı sonuç; yalnız çalışır ve yalnız Logseq girdilerinden aday üretir.
        for k in uninstall_adaylari() {
            let y = k.yol;
            assert!(y.to_ascii_lowercase().ends_with("logseq.exe"), "{y}");
        }
    }
}
