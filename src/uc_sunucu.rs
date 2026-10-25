//! Yerel uç sunucusunun Win32 yapıştırıcısı: sunucu (`cekirdek/src/sunucu.rs`) Win32'den
//! bağımsızdır; pencere işleri ileti penceresine `PostMessageW` ile devredilir.

use crate::pencere;
use crate::tepsi::{WM_UC_GIZLE, WM_UC_GOSTER};
use crate::uc::Is;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Sunucuyu başlatır; gizle/göster işleri `ileti_penceresi`ne postalanır.
pub fn baslat(ileti_penceresi: isize) {
    let devret = move |is: Is| {
        let ileti = match is {
            Is::Yok => return true,
            Is::Gizle => WM_UC_GIZLE,
            Is::Goster => WM_UC_GOSTER,
        };
        unsafe { PostMessageW(ileti_penceresi as HWND, ileti, 0, 0) != 0 }
    };
    crate::sunucu::baslat(env!("CARGO_PKG_VERSION"), pencere::durum_ozeti, devret);
}
