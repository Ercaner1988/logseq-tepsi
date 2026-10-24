//! Elle canlı sınama alt komutları (çıktı standart çıktıya düz metin).

use crate::pencere;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, IsWindowVisible, PostMessageW, SC_MINIMIZE, WM_SYSCOMMAND,
};

fn gor(h: HWND) -> (bool, bool) {
    unsafe { (IsWindowVisible(h) != 0, IsIconic(h) != 0) }
}

fn yaz(etiket: &str, h: HWND) {
    let (g, i) = gor(h);
    println!("{etiket}: gorunur={g} simge_durumunda={i}");
}

fn bekle(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Tepsi simgesi sol tık işlevini iki kez çağırır: gizler, sonra gösterir.
pub fn gizle_goster() -> i32 {
    let Some(h) = pencere::bul() else {
        println!("Logseq penceresi bulunamadi");
        return 1;
    };
    yaz("baslangic", h);
    // Karar tablosu gizli/simge durumundaki pencereyi önce gösterir; sınama gizlemeyle başlasın.
    if gor(h) != (true, false) {
        pencere::tikla();
        bekle(800);
        yaz("normallestirme (tik: goster)", h);
    }
    pencere::tikla();
    bekle(500);
    yaz("tik1 (gizlemeli)", h);
    pencere::tikla();
    bekle(800);
    yaz("tik2 (gostermeli)", h);
    let on_planda = unsafe { GetForegroundWindow() } == h;
    println!("on_planda={on_planda}");
    let (g, i) = gor(h);
    i32::from(!(g && !i))
}

/// Çalışan tepsi sürecini dışarıdan sınar: Logseq'i küçültür, gizlenmesini
/// bekler, sonunda geri yükler.
pub fn kucult() -> i32 {
    let Some(h) = pencere::bul() else {
        println!("Logseq penceresi bulunamadi");
        return 1;
    };
    yaz("baslangic", h);
    let t = Instant::now();
    unsafe { PostMessageW(h, WM_SYSCOMMAND, SC_MINIMIZE as usize, 0) };
    let mut gizlendi = false;
    while t.elapsed() < Duration::from_secs(3) {
        if !gor(h).0 {
            gizlendi = true;
            break;
        }
        bekle(10);
    }
    println!("gizlendi={gizlendi} sure_ms={}", t.elapsed().as_millis());
    yaz("kucultme sonrasi", h);
    bekle(300);
    pencere::goster(h);
    bekle(800);
    yaz("geri yukleme sonrasi", h);
    let on_planda = unsafe { GetForegroundWindow() } == h;
    println!("on_planda={on_planda}");
    let (g, i) = gor(h);
    i32::from(!(gizlendi && g && !i))
}

/// Çalışan tepsi sürecinin ileti penceresine simge tıklaması iletisini yollar
/// ve simgenin tepside kayıtlı olduğunu `Shell_NotifyIconGetRect` ile doğrular.
pub fn simge_tikla() -> i32 {
    use std::ptr::null;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::Shell::{Shell_NotifyIconGetRect, NOTIFYICONIDENTIFIER};
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, WM_LBUTTONUP};
    let sinif: Vec<u16> = "LogseqTepsiIletiPenceresi\0".encode_utf16().collect();
    let ileti = unsafe { FindWindowW(sinif.as_ptr(), null()) };
    if ileti.is_null() {
        println!("tepsi sureci yok");
        return 1;
    }
    let kimlik = NOTIFYICONIDENTIFIER {
        cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: ileti,
        uID: 1,
        guidItem: unsafe { std::mem::zeroed() },
    };
    let mut kutu: RECT = unsafe { std::mem::zeroed() };
    let hr = unsafe { Shell_NotifyIconGetRect(&kimlik, &mut kutu) };
    println!(
        "simge_kayitli={} hr={hr:#x} kutu=({},{})-({},{})",
        hr == 0,
        kutu.left,
        kutu.top,
        kutu.right,
        kutu.bottom
    );
    let Some(h) = pencere::bul() else {
        println!("Logseq penceresi bulunamadi");
        return 1;
    };
    if gor(h) != (true, false) {
        println!("pencere normal degil; once geri yuklenmeli");
        pencere::goster(h);
        bekle(800);
    }
    yaz("baslangic", h);
    let tik = || unsafe { PostMessageW(ileti, crate::tepsi::WM_TEPSI, 0, WM_LBUTTONUP as isize) };
    tik();
    bekle(600);
    yaz("tik1 (gizlemeli)", h);
    let gizlendi = !gor(h).0;
    tik();
    bekle(900);
    yaz("tik2 (gostermeli)", h);
    println!("on_planda={}", unsafe { GetForegroundWindow() } == h);
    let (g, i) = gor(h);
    i32::from(!(gizlendi && g && !i && hr == 0))
}

/// Logseq ana pencerelerini listeler (canlı ölçüm): kimlik, görünür, simge durumunda.
pub fn pencereler() -> i32 {
    let p = pencere::ana_pencereler();
    println!("ana_pencere_sayisi={}", p.len());
    for (h, d) in &p {
        println!(
            "hwnd={h:#x} gorunur={} simge_durumunda={}",
            d.gorunur, d.simge_durumunda
        );
    }
    0
}

/// Çalışan tepsi sürecinin ileti penceresine tek sol tık iletisi yollar (Logseq açık olmasa da).
pub fn tik() -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, WM_LBUTTONUP};
    let sinif: Vec<u16> = crate::tepsi::ILETI_SINIFI
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let ileti = unsafe { FindWindowW(sinif.as_ptr(), std::ptr::null()) };
    if ileti.is_null() {
        println!("tepsi sureci yok");
        return 1;
    }
    unsafe { PostMessageW(ileti, crate::tepsi::WM_TEPSI, 0, WM_LBUTTONUP as isize) };
    println!("tik yollandi");
    0
}
