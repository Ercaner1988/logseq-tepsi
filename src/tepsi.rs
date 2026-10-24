//! Gizli ileti penceresi, tepsi simgesi, menü, küçültme olayı kancası ve zamanlayıcılar.
//! Tüm pencere işleri bu (ileti döngüsü) iş parçacığında yapılır; kanca ve yerel uç
//! iş parçacığı yalnız `PostMessageW` ile iş devreder.

use crate::mantik::{self, KucultmeKarari, MenuEylemi, TepsiOlayi, MENU};
use crate::{gunluk, pencere, uc_sunucu};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows_sys::Win32::UI::Shell::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu,
    DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, KillTimer, LoadIconW, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    TrackPopupMenu, TranslateMessage, EVENT_SYSTEM_MINIMIZESTART, HICON, IDI_APPLICATION,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, OBJID_WINDOW, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_DESTROY, WM_ENDSESSION, WM_NULL,
    WM_QUERYENDSESSION, WM_TIMER, WNDCLASSW,
};

pub const ILETI_SINIFI: &str = "LogseqTepsiIletiPenceresi";
/// Tepsi simgesinden gelen geri çağrı iletisi.
pub const WM_TEPSI: u32 = WM_APP + 1;
const WM_KUCULTULDU: u32 = WM_APP + 2;
/// Yerel uçtan: görünür ana pencereleri gizle / tepsidekileri göster.
pub const WM_UC_GIZLE: u32 = WM_APP + 3;
pub const WM_UC_GOSTER: u32 = WM_APP + 4;
/// İkinci kopyanın mevcut kopyaya yolladığı kayıtlı ileti adı.
const YENIDEN_EKLE_ADI: &str = "LogseqTepsiSimgeyiYenidenEkle";
const ZAMANLAYICI_BAKIM: usize = 1;
const ZAMANLAYICI_KUCULT: usize = 2;
/// En çok bu kadar tık aralığı (ms) çift tık sayılır (sistem çift tık süresinin üst sınırı).
const CIFT_TIK_TAVANI_MS: u64 = 500;

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static YENIDEN_EKLE: AtomicU32 = AtomicU32::new(0);
static ILETI_PENCERESI: AtomicIsize = AtomicIsize::new(0);
static SIMGE: AtomicIsize = AtomicIsize::new(0);
static KANCA: AtomicIsize = AtomicIsize::new(0);
static SIMGE_TAMAM: AtomicBool = AtomicBool::new(false);
static BAKIM_DENEME: AtomicU32 = AtomicU32::new(0);
static SON_TIK_MS: AtomicU64 = AtomicU64::new(u64::MAX);
/// Küçültme yarışı için bekleyen (pencere, deneme) çiftleri.
static BEKLEYEN_KUCULTME: Mutex<Vec<(isize, u32)>> = Mutex::new(Vec::new());

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn simge_yukle() -> HICON {
    unsafe {
        let mut kucuk: HICON = null_mut();
        if let Some(yol) = pencere::logseq_yolu() {
            let yol_g = genis(&yol);
            if ExtractIconExW(yol_g.as_ptr(), 0, null_mut(), &mut kucuk, 1) >= 1 && !kucuk.is_null()
            {
                return kucuk;
            }
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
    let ipucu = mantik::ipucu(KANCA.load(Ordering::Relaxed) != 0);
    for (i, c) in ipucu.encode_utf16().take(nid.szTip.len() - 1).enumerate() {
        nid.szTip[i] = c;
    }
    nid
}

/// Simgeyi tepsiye koyar ya da (varsa) tazeler. Başarısızlık `SIMGE_TAMAM`'a yansır;
/// bakım zamanlayıcısı yeniden dener.
fn simge_koy(hwnd: HWND) {
    let nid = bildirim(hwnd);
    let tamam = unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &nid) != 0 || Shell_NotifyIconW(NIM_ADD, &nid) != 0
    };
    SIMGE_TAMAM.store(tamam, Ordering::Relaxed);
    if !tamam {
        gunluk::yaz("tepsi simgesi eklenemedi (NIM_ADD); yeniden denenecek");
    }
}

fn simge_sil(hwnd: HWND) {
    let nid = bildirim(hwnd);
    unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
}

/// Küçültme kancasını kurar; başarısızsa günlüğe yazar (ipucu uyarır, bakım yeniden dener).
fn kanca_kur() -> bool {
    let k = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZESTART,
            null_mut(),
            Some(kanca),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    KANCA.store(k as isize, Ordering::Relaxed);
    if k.is_null() {
        gunluk::yaz(&format!(
            "SetWinEventHook basarisiz (hata {}); yeniden denenecek",
            unsafe { GetLastError() }
        ));
    }
    !k.is_null()
}

/// Kanca ya da simge eksikse bakım zamanlayıcısını (artan gecikmeyle) kurar.
fn bakim_planla(hwnd: HWND) {
    if SIMGE_TAMAM.load(Ordering::Relaxed) && KANCA.load(Ordering::Relaxed) != 0 {
        BAKIM_DENEME.store(0, Ordering::Relaxed);
        return;
    }
    let deneme = BAKIM_DENEME.fetch_add(1, Ordering::Relaxed);
    unsafe {
        SetTimer(
            hwnd,
            ZAMANLAYICI_BAKIM,
            mantik::yeniden_gecikmesi_ms(deneme),
            None,
        )
    };
}

fn bakim(hwnd: HWND) {
    unsafe { KillTimer(hwnd, ZAMANLAYICI_BAKIM) };
    let kanca_vardi = KANCA.load(Ordering::Relaxed) != 0;
    if !SIMGE_TAMAM.load(Ordering::Relaxed) {
        simge_koy(hwnd);
    }
    if !kanca_vardi && kanca_kur() {
        gunluk::yaz("SetWinEventHook yeniden denemede basarili");
        // İpucundaki uyarıyı kaldır.
        simge_koy(hwnd);
        pencere::acilis_taramasi();
    }
    bakim_planla(hwnd);
}

fn menu_ac(hwnd: HWND) {
    unsafe {
        let gizli = pencere::gizli_sayisi();
        let menu = CreatePopupMenu();
        for oge in MENU {
            match oge {
                Some((kimlik, etiket)) => {
                    let bayrak = if mantik::menu_etkin_mi(kimlik, gizli) {
                        MF_STRING
                    } else {
                        MF_STRING | MF_GRAYED
                    };
                    AppendMenuW(menu, bayrak, kimlik, genis(etiket).as_ptr());
                }
                None => {
                    AppendMenuW(menu, MF_SEPARATOR, 0, null());
                }
            }
        }
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
        let Some(eylem) = mantik::menu_eylemi(secim as usize) else {
            return;
        };
        pencere::menu_uygula(eylem);
        // Çıkış: Logseq kapanır, araç tepside kalır (simgeye tıklayınca Logseq yeniden açılır).
        if eylem == MenuEylemi::AraciKapat {
            simge_sil(hwnd);
            PostQuitMessage(0);
        }
    }
}

/// Sol tık: çift tığın ikinci bırakması yok sayılır.
fn sol_tik() {
    let simdi = unsafe { GetTickCount64() };
    let onceki = SON_TIK_MS.swap(simdi, Ordering::Relaxed);
    let esik = u64::from(unsafe { GetDoubleClickTime() }).min(CIFT_TIK_TAVANI_MS);
    let onceki = (onceki != u64::MAX).then_some(onceki);
    if mantik::cift_tik_mi(simdi, onceki, esik) {
        return;
    }
    pencere::tikla();
}

fn kucultme_isle(hwnd: HWND, h: isize, deneme: u32) {
    if pencere::kucultuldu(h as HWND, deneme) == KucultmeKarari::Tekrar {
        BEKLEYEN_KUCULTME
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((h, deneme + 1));
        unsafe { SetTimer(hwnd, ZAMANLAYICI_KUCULT, mantik::KUCULTME_GECIKME_MS, None) };
    }
}

unsafe extern "system" fn pencere_proc(hwnd: HWND, ileti: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let tb = TASKBAR_CREATED.load(Ordering::Relaxed);
    if tb != 0 && ileti == tb {
        simge_koy(hwnd);
        bakim_planla(hwnd);
        return 0;
    }
    let yeniden = YENIDEN_EKLE.load(Ordering::Relaxed);
    if yeniden != 0 && ileti == yeniden {
        // İkinci kopya (ya da URL şeması) "simgeyi yeniden ekle" dedi.
        gunluk::yaz("simgeyi yeniden ekle iletisi alindi (ikinci kopya)");
        simge_koy(hwnd);
        if KANCA.load(Ordering::Relaxed) == 0 {
            kanca_kur();
        }
        bakim_planla(hwnd);
        pencere::acilis_taramasi();
        return 0;
    }
    match ileti {
        WM_TEPSI => {
            match mantik::tepsi_olayi(l) {
                TepsiOlayi::SolTik => sol_tik(),
                TepsiOlayi::SagTik => menu_ac(hwnd),
                TepsiOlayi::Yok => {}
            }
            0
        }
        WM_KUCULTULDU => {
            kucultme_isle(hwnd, w as isize, 0);
            0
        }
        WM_UC_GIZLE => {
            pencere::gizle_hepsi();
            0
        }
        WM_UC_GOSTER => {
            pencere::goster_hepsi();
            0
        }
        WM_TIMER => {
            match w {
                ZAMANLAYICI_BAKIM => bakim(hwnd),
                ZAMANLAYICI_KUCULT => {
                    KillTimer(hwnd, ZAMANLAYICI_KUCULT);
                    let bekleyen: Vec<(isize, u32)> = std::mem::take(
                        &mut *BEKLEYEN_KUCULTME.lock().unwrap_or_else(|e| e.into_inner()),
                    );
                    for (h, deneme) in bekleyen {
                        kucultme_isle(hwnd, h, deneme);
                    }
                }
                _ => {}
            }
            0
        }
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => {
            // Oturum kapanıyor: gizli pencere kayıp kalmasın.
            if w != 0 {
                pencere::cikista_goster();
            }
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

/// Mevcut kopyanın ileti penceresini bulup ona "simgeyi yeniden ekle" iletisini yollar.
/// İlk kopya mutex'i alıp pencereyi yaratana kadar kısa süre bekler.
fn ikinci_kopya() {
    unsafe {
        let ileti = RegisterWindowMessageW(genis(YENIDEN_EKLE_ADI).as_ptr());
        let sinif = genis(ILETI_SINIFI);
        for _ in 0..20 {
            let h = FindWindowW(sinif.as_ptr(), null());
            if !h.is_null() {
                PostMessageW(h, ileti, 0, 0);
                gunluk::yaz("ikinci kopya: mevcut kopyaya iletildi, cikiliyor");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        gunluk::yaz("ikinci kopya: mevcut kopyanin penceresi bulunamadi");
    }
}

/// Aracın normal çalışması: tek örnek, tepsi simgesi, ileti döngüsü.
pub fn calistir() {
    unsafe {
        let ad = genis("Local\\logseq-tepsi-tek-ornek");
        let mutex = CreateMutexW(null(), 0, ad.as_ptr());
        if mutex.is_null() {
            gunluk::yaz("tek ornek mutex'i olusturulamadi");
            return;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            ikinci_kopya();
            return;
        }
        // Menü/imleç koordinatları yüksek DPI'da bulanık ölçeklenmesin.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        TASKBAR_CREATED.store(
            RegisterWindowMessageW(genis("TaskbarCreated").as_ptr()),
            Ordering::Relaxed,
        );
        YENIDEN_EKLE.store(
            RegisterWindowMessageW(genis(YENIDEN_EKLE_ADI).as_ptr()),
            Ordering::Relaxed,
        );
        let sinif_adi = genis(ILETI_SINIFI);
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
            gunluk::yaz("ileti penceresi olusturulamadi");
            return;
        }
        ILETI_PENCERESI.store(hwnd as isize, Ordering::Relaxed);
        gunluk::yaz(&format!("baslatildi (surum {})", env!("CARGO_PKG_VERSION")));
        let simge = simge_yukle();
        SIMGE.store(simge as isize, Ordering::Relaxed);
        kanca_kur();
        simge_koy(hwnd);
        bakim_planla(hwnd);
        pencere::acilis_taramasi();
        uc_sunucu::baslat(hwnd as isize);
        let mut ileti: MSG = std::mem::zeroed();
        while GetMessageW(&mut ileti, null_mut(), 0, 0) > 0 {
            TranslateMessage(&ileti);
            DispatchMessageW(&ileti);
        }
        let k = KANCA.load(Ordering::Relaxed);
        if k != 0 {
            UnhookWinEvent(k as HWINEVENTHOOK);
        }
        simge_sil(hwnd);
        DestroyIcon(simge);
    }
}
