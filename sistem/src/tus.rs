//! Genel kısayollar (`RegisterHotKey`): kimlik başına tek kayıt, ileti penceresi iş parçacığında.
//! Yeni kısayol (ör. anında not) eklemek için yeni bir kimlik sabiti ve ayar yeter; ayrıştırma
//! çekirdekte (`kisayol`).

use logseq_tepsi_cekirdek::kisayol::{self, Kisayol};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{GetLastError, HWND};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT,
};

/// Logseq'i göster/gizle kısayolu.
pub const KIMLIK_GOSTER_GIZLE: i32 = 1;
/// Anında not kısayolu.
pub const KIMLIK_NOT: i32 = 2;

/// Şu an kayıtlı kısayollar (kimlik, kısayol).
static KAYITLI: Mutex<Vec<(i32, Kisayol)>> = Mutex::new(Vec::new());
/// Kaydı başarısız olan kimlikler (bit = kimlik; kimlik < 32).
static HATALI: AtomicU32 = AtomicU32::new(0);

/// Kimliğin kaydını `istenen`e getirir: aynıysa dokunmaz, farklıysa eskisini siler (`None` =
/// yalnız sil). Başka program tuşu tutuyorsa kayıt başarısız olur: günlüğe yazılır, `hatali_mi`
/// doğru döner (ipucunda uyarı).
pub fn ayarla(hwnd: HWND, kimlik: i32, istenen: Option<Kisayol>) {
    let mut kayitli = KAYITLI.lock().unwrap_or_else(|e| e.into_inner());
    let mevcut = kayitli.iter().position(|(k, _)| *k == kimlik);
    let hatali = HATALI.load(Ordering::Relaxed) & (1 << kimlik) != 0;
    if mevcut.map(|i| kayitli[i].1) == istenen && !hatali {
        return;
    }
    if let Some(i) = mevcut {
        unsafe { UnregisterHotKey(hwnd, kimlik) };
        kayitli.remove(i);
    }
    HATALI.fetch_and(!(1 << kimlik), Ordering::Relaxed);
    let Some(yeni) = istenen else { return };
    // SAFETY: hwnd ileti penceresi; çağıran ileti döngüsü iş parçacığıdır.
    let tamam =
        unsafe { RegisterHotKey(hwnd, kimlik, yeni.degistiriciler | MOD_NOREPEAT, yeni.tus) } != 0;
    if tamam {
        kayitli.push((kimlik, yeni));
    } else {
        HATALI.fetch_or(1 << kimlik, Ordering::Relaxed);
        logseq_tepsi_cekirdek::gunluk::yaz(&format!(
            "genel kisayol kaydedilemedi (kimlik {kimlik}, {}): hata {}; baska program kullaniyor olabilir",
            kisayol::bicimle(&yeni),
            unsafe { GetLastError() }
        ));
    }
}

/// Kimliğin son kayıt denemesi başarısız mı?
pub fn hatali_mi(kimlik: i32) -> bool {
    HATALI.load(Ordering::Relaxed) & (1 << kimlik) != 0
}

/// Çıkışta tüm kısayolları bırakır.
pub fn hepsini_birak(hwnd: HWND) {
    let mut kayitli = KAYITLI.lock().unwrap_or_else(|e| e.into_inner());
    for (kimlik, _) in kayitli.drain(..) {
        unsafe { UnregisterHotKey(hwnd, kimlik) };
    }
}
