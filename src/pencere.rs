//! Logseq penceresi üzerindeki Win32 işlemleri (yalnız pencere; ağ/dosya yok).

use crate::mantik::{self, Aday, Eylem, PencereDurumu};
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, MAX_PATH};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, EnumWindows, GetClassNameW, GetWindow,
    GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, ASFW_ANY, GWL_EXSTYLE,
    GW_OWNER, SW_HIDE, WM_CLOSE,
};

/// Tepsi aracının gizlediği pencere (yoklama yok; yalnız hatırlama).
static GIZLI: AtomicIsize = AtomicIsize::new(0);

fn surec_adi(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut tampon = [0u16; MAX_PATH as usize * 2];
        let mut uzunluk = tampon.len() as u32;
        let tamam = QueryFullProcessImageNameW(h, 0, tampon.as_mut_ptr(), &mut uzunluk);
        CloseHandle(h);
        if tamam == 0 {
            return None;
        }
        let yol = String::from_utf16_lossy(&tampon[..uzunluk as usize]);
        yol.rsplit('\\').next().map(str::to_owned)
    }
}

/// Pencerenin seçim özelliklerini okur.
pub fn aday_oku(hwnd: HWND) -> Option<Aday> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let surec_adi = surec_adi(pid)?;
        let mut sinif = [0u16; 256];
        let n = GetClassNameW(hwnd, sinif.as_mut_ptr(), sinif.len() as i32);
        Some(Aday {
            surec_adi,
            sinif: String::from_utf16_lossy(&sinif[..n.max(0) as usize]),
            sahipli: !GetWindow(hwnd, GW_OWNER).is_null(),
            baslik_var: GetWindowTextLengthW(hwnd) > 0,
            ex_stil: GetWindowLongW(hwnd, GWL_EXSTYLE) as u32,
        })
    }
}

pub fn ana_pencere_mi(hwnd: HWND) -> bool {
    aday_oku(hwnd).is_some_and(|a| mantik::ana_pencere_mi(&a))
}

unsafe extern "system" fn say(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let liste = &mut *(lparam as *mut Vec<HWND>);
    liste.push(hwnd);
    1
}

/// Logseq ana penceresini bulur (görünür ya da gizlenmiş).
pub fn bul() -> Option<HWND> {
    let gizli = GIZLI.load(Ordering::Relaxed) as HWND;
    if !gizli.is_null() && unsafe { IsWindow(gizli) } != 0 {
        return Some(gizli);
    }
    let mut hepsi: Vec<HWND> = Vec::new();
    unsafe { EnumWindows(Some(say), &mut hepsi as *mut _ as LPARAM) };
    let (pencereler, adaylar): (Vec<HWND>, Vec<(Aday, bool)>) = hepsi
        .into_iter()
        .filter_map(|h| Some((h, (aday_oku(h)?, unsafe { IsWindowVisible(h) } != 0))))
        .unzip();
    mantik::sec(&adaylar).map(|i| pencereler[i])
}

pub fn durum(hwnd: HWND) -> PencereDurumu {
    unsafe {
        PencereDurumu {
            gorunur: IsWindowVisible(hwnd) != 0,
            simge_durumunda: IsIconic(hwnd) != 0,
        }
    }
}

pub fn gizle(hwnd: HWND) {
    unsafe { ShowWindow(hwnd, SW_HIDE) };
    GIZLI.store(hwnd as isize, Ordering::Relaxed);
}

pub fn goster(hwnd: HWND) {
    unsafe {
        let komut = mantik::goster_komutu(IsIconic(hwnd) != 0);
        AllowSetForegroundWindow(ASFW_ANY);
        // ALT hilesi: ön plan kilidini açar.
        keybd_event(VK_MENU as u8, 0, 0, 0);
        keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
        ShowWindow(hwnd, komut);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
    }
    GIZLI.store(0, Ordering::Relaxed);
}

pub fn baslat() {
    // Logseq tek örnekli: zaten açıksa kendi penceresini öne getirir.
    let _ = std::process::Command::new(mantik::LOGSEQ_YOLU).spawn();
}

fn uygula(eylem: Eylem, hwnd: Option<HWND>) {
    match (eylem, hwnd) {
        (Eylem::Baslat, _) => baslat(),
        (Eylem::Goster, Some(h)) => goster(h),
        (Eylem::Gizle, Some(h)) => gizle(h),
        _ => {}
    }
}

/// Tepsi simgesine sol tık.
pub fn tikla() {
    let h = bul();
    uygula(mantik::tikla_karari(h.map(durum)), h);
}

/// Menü: "Göster".
pub fn menu_goster() {
    let h = bul();
    uygula(mantik::goster_karari(h.map(durum)), h);
}

/// Menü: "Tepsiye gizle".
pub fn menu_gizle() {
    let h = bul();
    if let Some(e) = mantik::gizle_karari(h.map(durum)) {
        uygula(e, h);
    }
}

/// Çıkış öncesi: gizli pencere kayıp kalmasın.
pub fn cikista_goster() {
    let h = bul();
    if mantik::cikista_gostermeli_mi(h.map(durum)) {
        if let Some(h) = h {
            goster(h);
        }
    }
}

/// Menüdeki Çıkış: Logseq'i de kapatır. Pencere gizliyse önce gösterilir (kapanmazsa kayıp
/// kalmasın), sonra kapatma düğmesine basılmış gibi `WM_CLOSE` gönderilir.
pub fn logseq_kapat() {
    cikista_goster();
    if let Some(h) = bul() {
        unsafe { PostMessageW(h, WM_CLOSE, 0, 0) };
    }
}

/// Küçültme olayı: Logseq ana penceresi simge durumundaysa gizle.
pub fn kucultuldu(hwnd: HWND) {
    if unsafe { IsIconic(hwnd) } != 0 && ana_pencere_mi(hwnd) {
        gizle(hwnd);
    }
}
