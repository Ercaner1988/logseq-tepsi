//! Soluk (Logseq kapalıyken gösterilen) tepsi simgesinin üretimi: normal simgenin piksellerini
//! `GetDIBits` ile okur, saf `mantik::solukla_bgra` ile gri/saydam yapar, `CreateIconIndirect` ile
//! yeni simge kurar. Dönen simgeyi çağıran `DestroyIcon` ile bırakır.

use crate::mantik;
use std::ffi::c_void;
use std::ptr::null_mut;
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetIconInfo, HICON, ICONINFO,
};

/// `bitmap`'in (genişlik, yükseklik) ölçüsü.
fn olcu(bitmap: HBITMAP) -> Option<(i32, i32)> {
    let mut b: BITMAP = unsafe { std::mem::zeroed() };
    let n = unsafe {
        GetObjectW(
            bitmap,
            std::mem::size_of::<BITMAP>() as i32,
            &mut b as *mut _ as *mut c_void,
        )
    };
    (n != 0 && b.bmWidth > 0 && b.bmHeight > 0).then_some((b.bmWidth, b.bmHeight))
}

/// Bitmap'i yukarıdan aşağıya 32 bit BGRA olarak okur.
fn pikselleri_oku(bitmap: HBITMAP, g: i32, y: i32) -> Option<Vec<u8>> {
    let mut bilgi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bilgi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: g,
        biHeight: -y,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let mut tampon = vec![0u8; g as usize * y as usize * 4];
    unsafe {
        let dc = GetDC(null_mut());
        let n = GetDIBits(
            dc,
            bitmap,
            0,
            y as u32,
            tampon.as_mut_ptr() as *mut c_void,
            &mut bilgi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(null_mut(), dc);
        (n == y).then_some(tampon)
    }
}

fn bitmaplardan(bilgi: &ICONINFO) -> Result<HICON, &'static str> {
    let (g, y) = olcu(bilgi.hbmColor).ok_or("renk bitmap'i okunamadi")?;
    let mut piksel = pikselleri_oku(bilgi.hbmColor, g, y).ok_or("GetDIBits (renk) basarisiz")?;
    // Maske renk bitmap'iyle aynı boyutta değilse yok sayılır (alfalı simgede zaten gerekmez).
    let maske = (olcu(bilgi.hbmMask) == Some((g, y)))
        .then(|| pikselleri_oku(bilgi.hbmMask, g, y))
        .flatten()
        .unwrap_or_default();
    mantik::solukla_bgra(&mut piksel, &maske);
    unsafe {
        let renk = CreateBitmap(g, y, 1, 32, piksel.as_ptr() as *const c_void);
        if renk.is_null() {
            return Err("CreateBitmap basarisiz");
        }
        let yeni = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: bilgi.hbmMask,
            hbmColor: renk,
        };
        let simge = CreateIconIndirect(&yeni);
        DeleteObject(renk);
        if simge.is_null() {
            return Err("CreateIconIndirect basarisiz");
        }
        Ok(simge)
    }
}

/// `normal` simgeden soluk bir kopya üretir; çağıran `DestroyIcon` ile bırakır.
pub fn uret(normal: HICON) -> Result<HICON, &'static str> {
    let mut bilgi: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(normal, &mut bilgi) } == 0 {
        return Err("GetIconInfo basarisiz");
    }
    let sonuc = if bilgi.hbmColor.is_null() {
        Err("tek renkli simge (renk bitmap'i yok)")
    } else {
        bitmaplardan(&bilgi)
    };
    // GetIconInfo bitmap'leri çağırana verir; CreateIconIndirect kendi kopyasını alır.
    unsafe {
        DeleteObject(bilgi.hbmColor);
        DeleteObject(bilgi.hbmMask);
    }
    sonuc
}

#[cfg(test)]
mod testler {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, LoadIconW, IDI_APPLICATION};

    /// Gerçek GDI yolu: sistem simgesinden soluk kopya üretir; pikselleri gri ve yarı saydam olmalı.
    #[test]
    fn sistem_simgesinden_soluk_kopya_uretilir() {
        let normal = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
        let soluk = uret(normal).expect("soluk simge uretilmeli");
        let mut bilgi: ICONINFO = unsafe { std::mem::zeroed() };
        assert_ne!(unsafe { GetIconInfo(soluk, &mut bilgi) }, 0);
        let (g, y) = olcu(bilgi.hbmColor).expect("renk bitmap'i olculmeli");
        let piksel = pikselleri_oku(bilgi.hbmColor, g, y).expect("pikseller okunmali");
        unsafe {
            DeleteObject(bilgi.hbmColor);
            DeleteObject(bilgi.hbmMask);
            DestroyIcon(soluk);
        }
        let gorunen: Vec<&[u8]> = piksel.chunks(4).filter(|p| p[3] > 0).collect();
        assert!(!gorunen.is_empty(), "tamamen saydam simge");
        assert!(
            gorunen.iter().all(|p| p[0] == p[1] && p[1] == p[2]),
            "gri degil"
        );
        assert!(
            gorunen.iter().all(|p| p[3] <= 130),
            "alfa yeterince kisilmamis"
        );
    }
}
