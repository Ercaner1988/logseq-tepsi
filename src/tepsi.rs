//! Gizli ileti penceresi, tepsi simgesi, menü ve küçültme olayı kancası.

use crate::{mantik::LOGSEQ_YOLU, pencere};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::Shell::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
    EVENT_SYSTEM_MINIMIZESTART, HICON, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, OBJID_WINDOW,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};

const WM_TEPSI: u32 = WM_APP + 1;
const WM_KUCULTULDU: u32 = WM_APP + 2;
const KIMLIK_GOSTER: usize = 1;
const KIMLIK_GIZLE: usize = 2;
const KIMLIK_CIKIS: usize = 3;

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static ILETI_PENCERESI: AtomicIsize = AtomicIsize::new(0);
static SIMGE: AtomicIsize = AtomicIsize::new(0);

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn simge_yukle() -> HICON {
    unsafe {
        let mut kucuk: HICON = null_mut();
        let yol = genis(LOGSEQ_YOLU);
        if ExtractIconExW(yol.as_ptr(), 0, null_mut(), &mut kucuk, 1) >= 1 && !kucuk.is_null() {
            return kucuk;
        }
        LoadIconW(null_mut(), IDI_APPLICATION)
    }
}

fn bildirim(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TEPSI;
    nid.hIcon = SIMGE.load(Ordering::Relaxed) as HICON;
    for (i, c) in "Logseq".encode_utf16().enumerate() {
        nid.szTip[i] = c;
    }
    nid
}

fn simge_ekle(hwnd: HWND) {
    let nid = bildirim(hwnd);
    unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
}

fn simge_sil(hwnd: HWND) {
    let nid = bildirim(hwnd);
    unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
}

fn menu_ac(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, KIMLIK_GOSTER, genis("Göster").as_ptr());
        AppendMenuW(
            menu,
            MF_STRING,
            KIMLIK_GIZLE,
            genis("Tepsiye gizle").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, KIMLIK_CIKIS, genis("Çıkış").as_ptr());
        let mut nokta = POINT { x: 0, y: 0 };
        GetCursorPos(&mut nokta);
        SetForegroundWindow(hwnd);
        let secim = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            nokta.x,
            nokta.y,
            0,
            hwnd,
            null(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        match secim as usize {
            KIMLIK_GOSTER => pencere::menu_goster(),
            KIMLIK_GIZLE => pencere::menu_gizle(),
            KIMLIK_CIKIS => {
                pencere::cikista_goster();
                simge_sil(hwnd);
                PostQuitMessage(0);
            }
            _ => {}
        }
    }
}

unsafe extern "system" fn pencere_proc(hwnd: HWND, ileti: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let tb = TASKBAR_CREATED.load(Ordering::Relaxed);
    if tb != 0 && ileti == tb {
        simge_ekle(hwnd);
        return 0;
    }
    match ileti {
        WM_TEPSI => {
            match (l & 0xFFFF) as u32 {
                WM_LBUTTONUP => pencere::tikla(),
                WM_RBUTTONUP => menu_ac(hwnd),
                _ => {}
            }
            0
        }
        WM_KUCULTULDU => {
            pencere::kucultuldu(w as HWND);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, ileti, w, l),
    }
}

/// Kanca: yalnızca pencere kimliğini ileti penceresine postalar (iş orada yapılır).
unsafe extern "system" fn kanca(
    _kanca: HWINEVENTHOOK,
    _olay: u32,
    hwnd: HWND,
    nesne: i32,
    _cocuk: i32,
    _is_parcacigi: u32,
    _zaman: u32,
) {
    if nesne == OBJID_WINDOW && !hwnd.is_null() {
        let hedef = ILETI_PENCERESI.load(Ordering::Relaxed) as HWND;
        PostMessageW(hedef, WM_KUCULTULDU, hwnd as WPARAM, 0);
    }
}

/// Aracın normal çalışması: tek örnek, tepsi simgesi, ileti döngüsü.
pub fn calistir() {
    unsafe {
        let ad = genis("Local\\logseq-tepsi-tek-ornek");
        let mutex = CreateMutexW(null(), 0, ad.as_ptr());
        if mutex.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        TASKBAR_CREATED.store(
            RegisterWindowMessageW(genis("TaskbarCreated").as_ptr()),
            Ordering::Relaxed,
        );
        let sinif_adi = genis("LogseqTepsiIletiPenceresi");
        let ornek = GetModuleHandleW(null());
        let sinif = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(pencere_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: ornek,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: sinif_adi.as_ptr(),
        };
        RegisterClassW(&sinif);
        // Görünmez üst düzey pencere (HWND_MESSAGE değil: TaskbarCreated yayınını alsın).
        let hwnd = CreateWindowExW(
            0,
            sinif_adi.as_ptr(),
            sinif_adi.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            ornek,
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        ILETI_PENCERESI.store(hwnd as isize, Ordering::Relaxed);
        let simge = simge_yukle();
        SIMGE.store(simge as isize, Ordering::Relaxed);
        simge_ekle(hwnd);
        let kanca_tutacagi = SetWinEventHook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZESTART,
            null_mut(),
            Some(kanca),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let mut ileti: MSG = std::mem::zeroed();
        while GetMessageW(&mut ileti, null_mut(), 0, 0) > 0 {
            TranslateMessage(&ileti);
            DispatchMessageW(&ileti);
        }
        if !kanca_tutacagi.is_null() {
            UnhookWinEvent(kanca_tutacagi);
        }
        simge_sil(hwnd);
        DestroyIcon(simge);
    }
}
