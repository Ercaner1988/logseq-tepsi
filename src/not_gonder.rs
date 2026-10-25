//! Anında not ve API sunucusu başlat/durdur: Logseq'e eklenti adresi (`logseq://...invokeCommand`)
//! göndermek. Logseq kapalıysa önce başlatılır; ana pencere görününce ve eklenti araca kendini
//! gösterince (en çok `NOT_TAVAN_MS`) adres gönderilir (eklenti yüklenmeden gelen adres Logseq'te
//! sessizce kaybolur). Yalnız ileti döngüsü iş parçacığında çağrılır.

use crate::{ayar, gunluk, not, pencere, sunucu};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

/// Logseq başlatıldıktan sonra hazır olmasını bekleyen yoklama zamanlayıcısı.
pub const ZAMANLAYICI_NOT: usize = 5;

/// Logseq hazır olunca gönderilecek adres (yoklama sürerken dolu).
static BEKLEYEN_ADRES: Mutex<Option<String>> = Mutex::new(None);
static BASLANGIC_MS: AtomicU64 = AtomicU64::new(0);

fn bekleyen() -> std::sync::MutexGuard<'static, Option<String>> {
    BEKLEYEN_ADRES.lock().unwrap_or_else(|e| e.into_inner())
}

/// Kısayol ve menüden: Logseq'i gösterir/başlatır, sonra not adresini gönderir.
pub fn aninda_not(hwnd: HWND) {
    if bekleyen().is_some() {
        gunluk::yaz("aninda not: Logseq hazirlaniyor, tekrar istek yok sayildi");
        return;
    }
    let adres = not::not_url(ayar::guncel().not_hedefi);
    if pencere::goster_veya_baslat() {
        pencere::adres_gonder(&adres);
        return;
    }
    *bekleyen() = Some(adres);
    BASLANGIC_MS.store(unsafe { GetTickCount64() }, Ordering::Relaxed);
    unsafe { SetTimer(hwnd, ZAMANLAYICI_NOT, not::NOT_YOKLAMA_MS, None) };
    gunluk::yaz("aninda not: Logseq kapaliydi, baslatildi; hazir olunca adres gonderilecek");
}

/// Menüden: çalışan Logseq'e API sunucusunu başlat/durdur adresini gönderir.
pub fn api_gonder(baslat: bool) {
    pencere::adres_gonder(&not::api_url(baslat));
}

/// Zamanlayıcı adımı: karar `not::not_adimi`de.
pub fn yokla(hwnd: HWND) {
    let gecen = unsafe { GetTickCount64() }.saturating_sub(BASLANGIC_MS.load(Ordering::Relaxed));
    let adim = not::not_adimi(
        pencere::gorunur_ana_sayisi(),
        sunucu::eklenti_goruldu_mu(),
        gecen,
    );
    if adim == not::NotAdimi::Bekle {
        return;
    }
    unsafe { KillTimer(hwnd, ZAMANLAYICI_NOT) };
    let adres = bekleyen().take();
    match (adim, adres) {
        (not::NotAdimi::Gonder, Some(a)) => {
            pencere::adres_gonder(&a);
            gunluk::yaz(&format!("aninda not: Logseq hazir ({gecen} ms)"));
        }
        (not::NotAdimi::Vazgec, _) => {
            gunluk::yaz("aninda not: Logseq penceresi beklenen surede gorunmedi, vazgecildi");
        }
        _ => {}
    }
}
