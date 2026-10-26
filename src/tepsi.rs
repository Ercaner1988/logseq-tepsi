//! Gizli ileti penceresi, tepsi simgesi, menü, küçültme olayı kancası ve zamanlayıcılar.
//! Tüm pencere işleri bu (ileti döngüsü) iş parçacığında yapılır; kanca ve yerel uç
//! iş parçacığı yalnız `PostMessageW` ile iş devreder.

use crate::ayar::AyarDegeri;
use crate::mantik::{self, KucultmeKarari, MenuEylemi, TepsiOlayi, MENU};
use crate::menu::{self, Tur};
use crate::uc::Devir;
use crate::{
    api, ayar, ayarlar, davranis, gunluk, kisayol, kurulum, not_gonder, pencere, soluk, sunucu,
    tus, uc_sunucu,
};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, AtomicU8, Ordering};
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
    AppendMenuW, CheckMenuRadioItem, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, KillTimer, LoadIconW,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetTimer, TrackPopupMenu, TranslateMessage, EVENT_SYSTEM_MINIMIZESTART, HICON, IDI_APPLICATION,
    MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, OBJID_WINDOW,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    WM_DESTROY, WM_ENDSESSION, WM_HOTKEY, WM_NULL, WM_QUERYENDSESSION, WM_TIMER, WNDCLASSW,
};

pub const ILETI_SINIFI: &str = "LogseqTepsiIletiPenceresi";
/// Tepsi simgesinden gelen geri çağrı iletisi.
pub const WM_TEPSI: u32 = WM_APP + 1;
const WM_KUCULTULDU: u32 = WM_APP + 2;
/// Yerel uçtan: görünür ana pencereleri gizle / tepsidekileri göster.
pub const WM_UC_GIZLE: u32 = WM_APP + 3;
pub const WM_UC_GOSTER: u32 = WM_APP + 4;
/// Yerel uçtan: kuyruktaki doğrulanmış ayar değişikliklerini uygula ve kaydet.
pub const WM_UC_AYAR: u32 = WM_APP + 5;
/// İkinci kopyanın mevcut kopyaya yolladığı kayıtlı ileti adı.
const YENIDEN_EKLE_ADI: &str = "LogseqTepsiSimgeyiYenidenEkle";
const ZAMANLAYICI_BAKIM: usize = 1;
const ZAMANLAYICI_KUCULT: usize = 2;
/// Logseq açık/kapalı yoklaması (simge ve ipucu durumu).
const ZAMANLAYICI_DURUM: usize = 3;
/// Oturumda gizli başlatma: Logseq ana penceresi görününce gizlemek için yoklama.
const ZAMANLAYICI_OTURUM: usize = 4;
/// En çok bu kadar tık aralığı (ms) çift tık sayılır (sistem çift tık süresinin üst sınırı).
const CIFT_TIK_TAVANI_MS: u64 = 500;

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static YENIDEN_EKLE: AtomicU32 = AtomicU32::new(0);
static ILETI_PENCERESI: AtomicIsize = AtomicIsize::new(0);
static SIMGE: AtomicIsize = AtomicIsize::new(0);
/// Logseq kapalıyken gösterilen soluk simge (0: üretilemedi, normal simge kullanılır).
static SIMGE_SOLUK: AtomicIsize = AtomicIsize::new(0);
/// Son yoklanan Logseq durumu: 0 henüz bilinmiyor, 1 kapalı, 2 açık.
static LOGSEQ_DURUMU: AtomicU8 = AtomicU8::new(0);
static KANCA: AtomicIsize = AtomicIsize::new(0);
static SIMGE_TAMAM: AtomicBool = AtomicBool::new(false);
static BAKIM_DENEME: AtomicU32 = AtomicU32::new(0);
static SON_TIK_MS: AtomicU64 = AtomicU64::new(u64::MAX);
/// Oturumda gizli başlatma yoklamasının başlangıcı (`GetTickCount64`, ms).
static OTURUM_BASLANGIC_MS: AtomicU64 = AtomicU64::new(0);
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
    let acik = logseq_acik_kayitli();
    let soluk_simge = SIMGE_SOLUK.load(Ordering::Relaxed);
    nid.hIcon = if acik || soluk_simge == 0 {
        SIMGE.load(Ordering::Relaxed)
    } else {
        soluk_simge
    } as HICON;
    let ipucu = davranis::ipucu_ayarli(
        acik,
        KANCA.load(Ordering::Relaxed) != 0,
        &ayar::guncel(),
        tus::hatali_mi(tus::KIMLIK_GOSTER_GIZLE) || tus::hatali_mi(tus::KIMLIK_NOT),
    );
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

/// Kayıtlı Logseq durumu; henüz yoklanmadıysa açık sayılır (normal simge, eski davranış).
fn logseq_acik_kayitli() -> bool {
    LOGSEQ_DURUMU.load(Ordering::Relaxed) != 1
}

/// Son yoklamada Logseq açıktı (kapalı ya da henüz bilinmiyor: `false`); API yoklayıcısı yalnız
/// bu durumda bağlanır.
fn logseq_acik_biliniyor() -> bool {
    LOGSEQ_DURUMU.load(Ordering::Relaxed) == 2
}

/// Logseq durumunu yoklar ve kaydeder; simge/ipucu çizimi bu kayıttan okur.
fn durum_oku() -> bool {
    let acik = pencere::logseq_acik_mi();
    LOGSEQ_DURUMU.store(if acik { 2 } else { 1 }, Ordering::Relaxed);
    acik
}

/// Durum zamanlayıcısının her adımında (yalnız o; tıklamalar saymaz) Logseq kapalı kaldıkça sayar. Kapalıyken eklenti-hazır işareti sıfırlanır (yeniden açılınca
/// eklenti kendini yeniden gösterir) ve bekleyen `api_otomatik_baslat` yazılır (kapanış
/// yazımı bitsin diye `KAPALI_YOKLAMA_SAYISI` ardışık yoklama sonra).
fn kapali_yoklama(acik: bool) {
    ayarlar::kapali_say(acik);
    if acik {
        return;
    }
    sunucu::eklenti_goruldu_sifirla();
    ayarlar::bekleyen_api_uygula();
}

/// Durum değiştiyse (ya da hiç yazılmadıysa) simgeyi ve ipucunu tazeler; yoksa dokunmaz.
/// Logseq şu an açık mı döner.
fn durum_tazele(hwnd: HWND) -> bool {
    let onceki = match LOGSEQ_DURUMU.load(Ordering::Relaxed) {
        0 => None,
        v => Some(v == 2),
    };
    let simdi = pencere::logseq_acik_mi();
    if mantik::durum_degisti_mi(onceki, simdi) {
        durum_oku();
        simge_koy(hwnd);
    }
    simdi
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

/// Kanca, simge ya da genel kısayol kaydı eksikse bakım zamanlayıcısını (artan gecikmeyle) kurar.
fn bakim_planla(hwnd: HWND) {
    if !mantik::bakim_gerekli(
        SIMGE_TAMAM.load(Ordering::Relaxed),
        KANCA.load(Ordering::Relaxed) != 0,
        tus::hatali_var(),
    ) {
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
    if tus::hatali_var() {
        // Tuşu tutan program bırakmış olabilir: kaydı başarısız kısayollar artan gecikmeyle yeniden denenir.
        kisayol_tazele(hwnd);
        if !tus::hatali_var() {
            gunluk::yaz("genel kisayol yeniden denemede kaydedildi");
            simge_koy(hwnd);
        }
    }
    bakim_planla(hwnd);
}

/// Menü modelinin (`menu::Madde`) maddelerini `hedef` menüye ekler; radyo işaretleri sonda.
unsafe fn maddeleri_ekle(
    hedef: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
    maddeler: &[menu::Madde],
) {
    for m in maddeler {
        let bayrak = match m.tur {
            Tur::Ayirici => {
                AppendMenuW(hedef, MF_SEPARATOR, 0, null());
                continue;
            }
            Tur::Onay(true) => MF_STRING | MF_CHECKED,
            Tur::Eylem { etkin: false } => MF_STRING | MF_GRAYED,
            Tur::Onay(false) | Tur::Radyo { .. } | Tur::Eylem { etkin: true } => MF_STRING,
        };
        AppendMenuW(hedef, bayrak, m.kimlik, genis(&m.etiket).as_ptr());
    }
    for m in maddeler {
        if let Tur::Radyo {
            isaretli: true,
            ilk,
            son,
        } = m.tur
        {
            CheckMenuRadioItem(hedef, ilk as u32, son as u32, m.kimlik as u32, MF_BYCOMMAND);
        }
    }
}

/// Menü açılırken öğrenilen gerçek durumlar (başlangıç kısayolu, `configs.edn`, son API durumu).
fn dis_durum() -> menu::DisDurum {
    let logseq_acik = pencere::logseq_acik_mi();
    menu::DisDurum {
        baslangicta: kurulum::baslangic_var_mi(),
        api_otomatik: ayarlar::api_gercek(),
        // Son bilinen (arka planda 2 sn'de bir yoklanan) durum: menü açılışı 300 ms'lik TCP
        // yoklamasıyla ileti döngüsünü bekletmez.
        api_acik: logseq_acik && api::son_durum(),
        logseq_acik,
    }
}

/// "Ayarlar" alt menüsünü model (`menu::ayar_maddeleri_tam`) üzerinden kurar ve `menu`ye ekler.
unsafe fn ayar_alt_menusu(
    menu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
    dis: &menu::DisDurum,
) {
    let alt = CreatePopupMenu();
    maddeleri_ekle(alt, &menu::ayar_maddeleri_tam(&ayar::guncel(), dis));
    AppendMenuW(
        menu,
        MF_POPUP | MF_STRING,
        alt as usize,
        genis("Ayarlar").as_ptr(),
    );
}

/// Ayar uygulandıktan sonra sırayla yapılan tazeleme adımları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AyarSonrasi {
    KisayolTazele,
    SimgeKoy,
    BakimPlanla,
}

/// Dosyada tutulan bir ayar değiştiyse genel kısayol kaydı, ipucu ve bakım planı tazelenir. Bakım
/// SONDA planlanır: kısayol kaydı çalışırken başarısız olduysa (tuşu başka program tutuyor)
/// bakım zamanlayıcısı artan gecikmeyle yeniden dener; yalnız açılışta planlansaydı kayıt
/// bir sonraki Explorer yeniden başlatmasına kadar kalıcı eksik kalırdı.
fn ayar_sonrasi(degisti: bool, mut adim: impl FnMut(AyarSonrasi)) {
    if degisti {
        adim(AyarSonrasi::KisayolTazele);
        adim(AyarSonrasi::SimgeKoy);
        adim(AyarSonrasi::BakimPlanla);
    }
}

/// Ayar değişikliklerini uygular (`ayar_sonrasi` adımlarıyla). Çakışma gibi bir nedenle
/// reddedilirse `Devir::Red`; yalnız ileti döngüsü iş parçacığında çağrılır.
fn ayar_uygula(hwnd: HWND, degerler: &[AyarDegeri]) -> Devir {
    let (degisti, govde) = match ayarlar::uygula(degerler) {
        Ok(s) => s,
        Err(neden) => return Devir::Red(neden),
    };
    ayar_sonrasi(degisti, |a| match a {
        AyarSonrasi::KisayolTazele => kisayol_tazele(hwnd),
        AyarSonrasi::SimgeKoy => simge_koy(hwnd),
        AyarSonrasi::BakimPlanla => bakim_planla(hwnd),
    });
    if govde.is_empty() {
        Devir::Tamam
    } else {
        Devir::Govde(govde)
    }
}

/// Genel kısayolları geçerli ayara göre kaydeder: aynıysa dokunmaz; değişenlerin (ve kaydı
/// başarısız olanların) hepsi önce bırakılır, sonra kaydedilir (iki kısayol yer değiştirebilir).
fn kisayol_tazele(hwnd: HWND) {
    let ayar = ayar::guncel();
    tus::ayarla_hepsi(
        hwnd as isize,
        &[
            (
                tus::KIMLIK_GOSTER_GIZLE,
                kisayol::ayardan(&ayar.kisayol_goster_gizle),
            ),
            (tus::KIMLIK_NOT, kisayol::ayardan(&ayar.kisayol_not)),
        ],
    );
}

/// Oturumda gizli başlatma: Logseq kapalıysa başlatır, ana penceresi görünene kadar (en çok
/// `OTURUM_TAVAN_MS`) yoklar.
fn oturum_baslat(hwnd: HWND) {
    gunluk::yaz("oturum acilisi: Logseq gizli baslatilacak");
    OTURUM_BASLANGIC_MS.store(unsafe { GetTickCount64() }, Ordering::Relaxed);
    if !pencere::logseq_acik_mi() {
        pencere::baslat();
    }
    unsafe { SetTimer(hwnd, ZAMANLAYICI_OTURUM, davranis::OTURUM_YOKLAMA_MS, None) };
}

fn oturum_yokla(hwnd: HWND) {
    let gecen =
        unsafe { GetTickCount64() }.saturating_sub(OTURUM_BASLANGIC_MS.load(Ordering::Relaxed));
    match davranis::oturum_adimi(pencere::gorunur_ana_sayisi(), gecen) {
        davranis::OturumAdimi::Bekle => {}
        davranis::OturumAdimi::Gizle => {
            unsafe { KillTimer(hwnd, ZAMANLAYICI_OTURUM) };
            pencere::gizle_hepsi();
            gunluk::yaz(&format!(
                "oturum acilisi: Logseq tepsiye gizlendi ({gecen} ms)"
            ));
        }
        davranis::OturumAdimi::Vazgec => {
            unsafe { KillTimer(hwnd, ZAMANLAYICI_OTURUM) };
            gunluk::yaz("oturum acilisi: Logseq penceresi beklenen surede gorunmedi, vazgecildi");
        }
    }
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
        let dis = dis_durum();
        maddeleri_ekle(menu, &menu::ana_ek_maddeleri(&dis));
        ayar_alt_menusu(menu, &dis);
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
            if let Some(e) = menu::ana_secim(secim as usize, &dis) {
                match e {
                    menu::AnaEylem::AnindaNot => not_gonder::aninda_not(hwnd),
                    menu::AnaEylem::ApiSunucusu(baslat) => not_gonder::api_gonder(baslat),
                }
            } else if let Some(d) = menu::secim_degeri_tam(secim as usize, &ayar::guncel(), &dis) {
                ayar_uygula(hwnd, &[d]);
            }
            return;
        };
        pencere::menu_uygula(eylem);
        durum_tazele(hwnd);
        // Çıkış: Logseq kapanır, araç tepside kalır (simgeye tıklayınca Logseq yeniden açılır).
        if eylem == MenuEylemi::AraciKapat {
            simge_sil(hwnd);
            PostQuitMessage(0);
        }
    }
}

/// Sol tık: çift tığın ikinci bırakması yok sayılır.
fn sol_tik(hwnd: HWND) {
    let simdi = unsafe { GetTickCount64() };
    let onceki = SON_TIK_MS.swap(simdi, Ordering::Relaxed);
    let esik = u64::from(unsafe { GetDoubleClickTime() }).min(CIFT_TIK_TAVANI_MS);
    let onceki = (onceki != u64::MAX).then_some(onceki);
    if mantik::cift_tik_mi(simdi, onceki, esik) {
        return;
    }
    pencere::tikla_ayarli(ayar::guncel().kapaliyken_tik_acar);
    durum_tazele(hwnd);
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
        durum_oku();
        simge_koy(hwnd);
        bakim_planla(hwnd);
        return 0;
    }
    let yeniden = YENIDEN_EKLE.load(Ordering::Relaxed);
    if yeniden != 0 && ileti == yeniden {
        // İkinci kopya (ya da URL şeması) "simgeyi yeniden ekle" dedi.
        gunluk::yaz("simgeyi yeniden ekle iletisi alindi (ikinci kopya)");
        durum_oku();
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
            match davranis::tepsi_eylemi(l, ayar::guncel().tik) {
                TepsiOlayi::SolTik => sol_tik(hwnd),
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
        WM_UC_AYAR => {
            // Sunucu iş parçacığı `SendMessageTimeoutW` ile bekliyor: sonuç kuyruğa yazılır.
            ayarlar::kuyruk().isle(|degerler| ayar_uygula(hwnd, degerler));
            0
        }
        WM_HOTKEY => {
            match w as i32 {
                tus::KIMLIK_GOSTER_GIZLE => {
                    pencere::tikla();
                    durum_tazele(hwnd);
                }
                tus::KIMLIK_NOT => not_gonder::aninda_not(hwnd),
                _ => {}
            }
            0
        }
        WM_TIMER => {
            match w {
                ZAMANLAYICI_OTURUM => oturum_yokla(hwnd),
                not_gonder::ZAMANLAYICI_NOT => not_gonder::yokla(hwnd),
                ZAMANLAYICI_BAKIM => bakim(hwnd),
                ZAMANLAYICI_DURUM => {
                    let acik = durum_tazele(hwnd);
                    kapali_yoklama(acik);
                }
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
pub fn calistir(oturum: bool) {
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
        match soluk::uret(simge as isize) {
            Ok(s) => SIMGE_SOLUK.store(s, Ordering::Relaxed),
            Err(neden) => gunluk::yaz(&format!(
                "soluk simge uretilemedi ({neden}); normal simge kullanilacak"
            )),
        }
        ayarlar::yukle();
        kisayol_tazele(hwnd);
        durum_oku();
        if let Err(e) = api::yoklayici_baslat(logseq_acik_biliniyor) {
            gunluk::yaz(&format!("API yoklayıcısı başlatılamadı: {e}"));
        }
        kanca_kur();
        simge_koy(hwnd);
        SetTimer(hwnd, ZAMANLAYICI_DURUM, mantik::DURUM_YOKLAMA_MS, None);
        bakim_planla(hwnd);
        pencere::acilis_taramasi();
        uc_sunucu::baslat(hwnd as isize);
        if davranis::oturum_gizli_mi(oturum, &ayar::guncel()) {
            oturum_baslat(hwnd);
        }
        let mut ileti: MSG = std::mem::zeroed();
        while GetMessageW(&mut ileti, null_mut(), 0, 0) > 0 {
            TranslateMessage(&ileti);
            DispatchMessageW(&ileti);
        }
        let k = KANCA.load(Ordering::Relaxed);
        if k != 0 {
            UnhookWinEvent(k as HWINEVENTHOOK);
        }
        tus::hepsini_birak(hwnd as isize);
        simge_sil(hwnd);
        DestroyIcon(simge);
        let soluk_simge = SIMGE_SOLUK.load(Ordering::Relaxed);
        if soluk_simge != 0 {
            DestroyIcon(soluk_simge as HICON);
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn adimlar(degisti: bool) -> Vec<AyarSonrasi> {
        let mut yapilan = Vec::new();
        ayar_sonrasi(degisti, |a| yapilan.push(a));
        yapilan
    }

    /// Çalışırken başarısız olan kısayol kaydı için bakım planlanmazsa kayıt kalıcı eksik kalır
    /// (README: "bakım yeniden dener"); bakım, kayıt denemesinden SONRA planlanmalıdır.
    #[test]
    fn ayar_degisince_kisayol_tazelenir_sonra_bakim_planlanir() {
        assert_eq!(
            adimlar(true),
            [
                AyarSonrasi::KisayolTazele,
                AyarSonrasi::SimgeKoy,
                AyarSonrasi::BakimPlanla
            ]
        );
    }

    #[test]
    fn ayar_degismediyse_hicbir_adim_yapilmaz() {
        assert_eq!(adimlar(false), []);
    }
}
