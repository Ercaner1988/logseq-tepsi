//! `windows` alt sistemli exe'den komut satırı çıktısı: standart çıktı yönlendirilmemişse
//! üst sürecin konsoluna bağlanır (yönlendirilmişse dokunmaz).

use std::ptr::null;
use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
    STD_OUTPUT_HANDLE,
};

pub fn baglan() {
    unsafe {
        let cikis = GetStdHandle(STD_OUTPUT_HANDLE);
        if !cikis.is_null() && cikis != INVALID_HANDLE_VALUE {
            return;
        }
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        let ad: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
        let h = CreateFileW(
            ad.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if h != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_OUTPUT_HANDLE, h);
            SetStdHandle(STD_ERROR_HANDLE, h);
        }
    }
}
