//! Logseq penceresi üzerindeki Win32 işlemleri (yalnız pencere; Logseq'e API/ağ yok).
//! Pencere değiştiren işlevler ve `ana_pencereler` (Gizlenenler kümesini budar, ayar
//! dosyasına yazar) yalnız ileti döngüsü iş parçacığından çağrılır. Sunucu iş parçacığı
//! (`/durum`) yalnız `durum_ozeti`ni çağırır: o kümeyi ve ayarı DEĞİŞTİRMEZ, salt okur.

use crate::gizlenen::Gizlenenler;
use crate::mantik::{self, AcilisEylemi, Aday, KucultmeKarari, MenuEylemi, PencereDurumu, Toplu};
use crate::{ayar, davranis, gunluk, kayit, yollar};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Mutex, MutexGuard};
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, MAX_PATH};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MENU,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow,
    GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, ASFW_ANY, GWL_EXSTYLE,
    GW_OWNER, SW_HIDE, SW_SHOW, WM_CLOSE,
};

/// Tepsi aracının gizlediği pencereler (yoklama yok; yalnız hatırlama).
static GIZLI: Mutex<Gizlenenler> = Mutex::new(Gizlenenler::yeni());
/// Ayar dosyasına en son yazılan Logseq yolu (gereksiz yazmayı önler).
static HATIRLANAN_YOL: Mutex<String> = Mutex::new(String::new());

fn kume() -> MutexGuard<'static, Gizlenenler> {
    GIZLI.lock().unwrap_or_else(|e| e.into_inner())
}

fn surec_yolu(pid: u32) -> Option<String> {
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
        Some(String::from_utf16_lossy(&tampon[..uzunluk as usize]))
    }
}

fn pencere_surec_yolu(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    surec_yolu(pid)
}

/// Pencerenin seçim özelliklerini okur. Ucuz okumalar (sınıf, sahip, başlık, stil) önce,
/// pahalı süreç açma en sonda; sınıf uymuyorsa süreç hiç açılmaz.
pub fn aday_oku(hwnd: HWND) -> Option<Aday> {
    unsafe {
        let mut sinif = [0u16; 256];
        let n = GetClassNameW(hwnd, sinif.as_mut_ptr(), sinif.len() as i32);
        let sinif = String::from_utf16_lossy(&sinif[..n.max(0) as usize]);
        if !mantik::sinif_uygun_mu(&sinif) {
            return None;
        }
        let sahipli = !GetWindow(hwnd, GW_OWNER).is_null();
        let baslik_var = GetWindowTextLengthW(hwnd) > 0;
        let ex_stil = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let yol = pencere_surec_yolu(hwnd)?;
        let surec_adi = yol.rsplit('\\').next().map(str::to_owned)?;
        Some(Aday {
            surec_adi,
            sinif,
            sahipli,
            baslik_var,
            ex_stil,
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

/// Sınıfı Logseq'e uyan tüm üst düzey pencereler (Z sırasıyla, üstteki önce) ve görünürlükleri.
fn tum_adaylar() -> Vec<(HWND, Aday, bool)> {
    let mut hepsi: Vec<HWND> = Vec::new();
    unsafe { EnumWindows(Some(say), &mut hepsi as *mut _ as LPARAM) };
    hepsi
        .into_iter()
        .filter_map(|h| Some((h, aday_oku(h)?, unsafe { IsWindowVisible(h) } != 0)))
        .collect()
}

/// Logseq ana penceresini bulur (görünür ya da gizlenmiş); canlı sınama komutları için.
pub fn bul() -> Option<HWND> {
    let (pencereler, adaylar): (Vec<HWND>, Vec<(Aday, bool)>) = tum_adaylar()
        .into_iter()
        .map(|(h, a, g)| (h, (a, g)))
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

/// Logseq ana pencereleri ve görünürlükleri; hiçbir şeyi değiştirmez (her iş parçacığında güvenli).
fn ana_listesi() -> Vec<(HWND, bool)> {
    tum_adaylar()
        .into_iter()
        .filter(|(_, a, _)| mantik::ana_pencere_mi(a))
        .map(|(h, _, g)| (h, g))
        .collect()
}

/// Küme kimliği hâlâ geçerli mi: pencere var ve gizli (kapanmış ya da başka süreçte yeniden
/// kullanılmış HWND bayat sayılır).
fn kumede_gecerli_mi(ana: &[(HWND, bool)], kimlik: isize) -> bool {
    let durum = ana
        .iter()
        .find(|(h, _)| *h as isize == kimlik)
        .map(|(_, g)| PencereDurumu {
            gorunur: *g,
            simge_durumunda: false,
        });
    mantik::kumede_kalmali_mi(durum)
}

/// Tüm Logseq ana pencereleri ve durumları. Bayat kimlikleri kümeden atar; Logseq yolunu
/// ayara kaydeder. Küme ve ayar yazar: yalnız ileti döngüsü iş parçacığında çağrılır.
pub fn ana_pencereler() -> Vec<(isize, PencereDurumu)> {
    let ana = ana_listesi();
    kume().gecerlileri_tut(|k| kumede_gecerli_mi(&ana, k));
    if let Some((h, _)) = ana.first() {
        yolu_hatirla(*h);
    }
    ana.into_iter()
        .map(|(h, _)| (h as isize, durum(h)))
        .collect()
}

fn ayar_dosyasi() -> Option<std::path::PathBuf> {
    Some(gunluk::dizin()?.join("logseq-yolu.txt"))
}

/// Dizinde `Update.exe` var mı (Squirrel kök dizini göstergesi).
fn update_exe_var(kok: &str) -> bool {
    Path::new(kok).join("Update.exe").is_file()
}

/// Çalışan Logseq'in yolu değiştiyse ayar dosyasına yazar (sonraki başlatma için). Sürümü
/// bilinen en yüksek kurulumdan düşük olan yol (örn. HKLM'de 2.0.2 varken eski Squirrel
/// 0.10.15) yazılmaz; değerlendirilen son yol her durumda hatırlanır (kayıt defteri her
/// yoklamada taranmasın).
fn yolu_hatirla(hwnd: HWND) {
    let Some(yol) = pencere_surec_yolu(hwnd) else {
        return;
    };
    let mut son = HATIRLANAN_YOL.lock().unwrap_or_else(|e| e.into_inner());
    if son.eq_ignore_ascii_case(&yol) {
        return;
    }
    if !yollar::hatirlanmali_mi(&yol, &kayit::uninstall_adaylari(), update_exe_var) {
        gunluk::yaz(&format!(
            "Logseq yolu kaydedilmedi (daha yüksek sürümlü kurulum var): {yol}"
        ));
        *son = yol;
        return;
    }
    if let Some(dosya) = ayar_dosyasi() {
        if let Some(d) = dosya.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        match std::fs::write(&dosya, format!("{yol}\r\n")) {
            Ok(()) => gunluk::yaz(&format!("Logseq yolu kaydedildi: {yol}")),
            Err(e) => gunluk::yaz(&format!("Logseq yolu kaydedilemedi: {e}")),
        }
    }
    *son = yol;
}

/// Başlatılacak/simgesi alınacak `Logseq.exe` yolu: çalışan süreç, ayar dosyası, kayıt
/// defteri (sürümü en yüksek önce), sabit yollar sırasıyla; ilk var olan. Ayar dosyasındaki
/// yol, daha yüksek sürümlü kurulum varken (eski sürümde kalmış olabilir) atlanır.
pub fn logseq_yolu() -> Option<String> {
    let calisan = bul().and_then(pencere_surec_yolu);
    let kayit = kayit::uninstall_adaylari();
    let ayar = ayar_dosyasi()
        .and_then(|d| std::fs::read_to_string(d).ok())
        .and_then(|i| yollar::ayar_yolu(&i))
        .filter(|y| yollar::hatirlanmali_mi(y, &kayit, update_exe_var));
    let sabit = yollar::sabit_yollar(std::env::var("LOCALAPPDATA").ok().as_deref());
    yollar::adaylar(calisan, ayar, kayit, sabit)
        .into_iter()
        .find(|y| Path::new(y).is_file())
}

pub fn gizle(hwnd: HWND) {
    unsafe { ShowWindow(hwnd, SW_HIDE) };
    kume().ekle(hwnd as isize);
}

/// ALT tuşuna basıp bırakır: ön plan kilidini açma yedeği (yalnız düz deneme olmazsa).
fn alt_dokun() {
    let tus = |bayrak| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_MENU,
                wScan: 0,
                dwFlags: bayrak,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let girdiler = [tus(0), tus(KEYEVENTF_KEYUP)];
    unsafe {
        SendInput(
            girdiler.len() as u32,
            girdiler.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
}

pub fn goster(hwnd: HWND) {
    unsafe {
        let komut = mantik::goster_komutu(IsIconic(hwnd) != 0);
        AllowSetForegroundWindow(ASFW_ANY);
        ShowWindow(hwnd, komut);
        BringWindowToTop(hwnd);
        if SetForegroundWindow(hwnd) == 0 || GetForegroundWindow() != hwnd {
            alt_dokun();
            SetForegroundWindow(hwnd);
        }
    }
    kume().cikar(hwnd as isize);
}

fn surec_olustur(
    yol: &str,
    argumanlar: &[&str],
    breakaway: bool,
) -> std::io::Result<std::process::Child> {
    std::process::Command::new(yol)
        .args(argumanlar)
        .env_remove("ELECTRON_RUN_AS_NODE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(mantik::baslat_bayraklari(breakaway))
        .spawn()
}

/// Logseq'i ayrık süreç olarak başlatır (Logseq tek örnekli: açıksa kendi penceresini öne getirir).
pub fn baslat() {
    let Some(yol) = logseq_yolu() else {
        gunluk::yaz("Logseq baslatilamadi: Logseq.exe yolu bulunamadi");
        return;
    };
    surec_calistir(&yol, &[], "baslatildi", "baslatilamadi");
}

/// `Logseq.exe <adres>`: çalışan örneğe `logseq://` adresi iletir (örnek `second-instance` ile
/// alır, ikinci süreç hemen çıkar). Logseq kapalıyken çağrılmamalı: eklenti yüklenmeden gelen
/// adres sessizce kaybolur.
pub fn adres_gonder(adres: &str) {
    match logseq_yolu() {
        Some(yol) => surec_calistir(&yol, &[adres], "adres gonderildi", "adres gonderilemedi"),
        None => gunluk::yaz("Logseq adresi gonderilemedi: Logseq.exe yolu bulunamadi"),
    }
}

fn surec_calistir(yol: &str, argumanlar: &[&str], tamam: &str, hata: &str) {
    let sonuc = surec_olustur(yol, argumanlar, true).or_else(|e| {
        gunluk::yaz(&format!(
            "Logseq iş nesnesinden kopuk baslatilamadi ({e}); bayraksiz deneniyor"
        ));
        surec_olustur(yol, argumanlar, false)
    });
    match sonuc {
        Ok(_) => gunluk::yaz(&format!("Logseq {tamam}: {yol} {}", argumanlar.join(" "))),
        Err(e) => gunluk::yaz(&format!("Logseq {hata} ({yol}): {e}")),
    }
}

fn toplu_uygula(t: Toplu) {
    match t {
        Toplu::Baslat => baslat(),
        Toplu::Gizle(ler) => ler.into_iter().for_each(|h| gizle(h as HWND)),
        Toplu::Goster(h) => goster(h as HWND),
    }
}

fn hepsini_goster_uygula() {
    let p = ana_pencereler();
    let sira = mantik::gosterilecekler(&p, &kume());
    sira.into_iter().for_each(|h| goster(h as HWND));
}

/// Menüdeki "Göster" gibi: pencere varsa gösterip öne getirir (gizliyse tepsiden çıkarır),
/// Logseq kapalıysa başlatır. Logseq zaten açıktıysa `true`.
pub fn goster_veya_baslat() -> bool {
    let p = ana_pencereler();
    let t = mantik::goster_toplu(&p, &kume());
    let acikti = !matches!(t, Toplu::Baslat);
    toplu_uygula(t);
    acikti
}

/// Tepsi simgesine sol tık: görünür varsa gizle, yoksa en son gizleneni göster; Logseq kapalıysa
/// başlat (genel kısayol ve sınama komutları böyle çağırır).
pub fn tikla() {
    tikla_ayarli(true);
}

/// Simge tıkı: `kapaliyken_ac = false` iken Logseq kapalıysa hiçbir şey yapmaz
/// (`kapaliyken_tik_acar = hayir`).
pub fn tikla_ayarli(kapaliyken_ac: bool) {
    let p = ana_pencereler();
    if let Some(t) = davranis::tik_toplu_ayarli(mantik::tikla_toplu(&p, &kume()), kapaliyken_ac) {
        toplu_uygula(t);
    }
}

/// Menü eylemi (Çıkış ve Aracı kapat ileti döngüsü sahibinde ele alınır).
pub fn menu_uygula(e: MenuEylemi) {
    match e {
        MenuEylemi::Goster => {
            let p = ana_pencereler();
            let t = mantik::goster_toplu(&p, &kume());
            toplu_uygula(t);
        }
        MenuEylemi::Gizle => gizle_hepsi(),
        MenuEylemi::HepsiniGoster => hepsini_goster_uygula(),
        MenuEylemi::Cikis => logseq_kapat(),
        MenuEylemi::AraciKapat => cikista_goster(),
    }
}

/// "Tepsiye gizle" ve `POST /gizle`: görünür tüm ana pencereler.
pub fn gizle_hepsi() {
    let p = ana_pencereler();
    mantik::gizlenecekler(&p)
        .into_iter()
        .for_each(|h| gizle(h as HWND));
}

/// `POST /goster`: tepsideki tüm ana pencereleri göster, en son gizlenen öne gelir.
pub fn goster_hepsi() {
    hepsini_goster_uygula();
}

/// Çıkış/oturum kapanışı öncesi: gizli hiçbir pencere kayıp kalmasın.
pub fn cikista_goster() {
    hepsini_goster_uygula();
}

/// Menüdeki Çıkış: tüm Logseq ana pencerelerini kapatır. Gizli olanlar önce gösterilir
/// (kapanmazsa kayıp kalmasın), sonra her birine `WM_CLOSE` (kapatma düğmesi gibi).
pub fn logseq_kapat() {
    cikista_goster();
    for (h, _) in ana_pencereler() {
        unsafe { PostMessageW(h as HWND, WM_CLOSE, 0, 0) };
    }
}

/// Küçültme olayı: Logseq ana penceresi simge durumundaysa gizle. `IsIconic` olay anında
/// henüz tazelenmemiş olabilir: kararı `Tekrar` ise çağıran kısa gecikmeyle bir kez daha dener.
pub fn kucultuldu(hwnd: HWND, deneme: u32) -> KucultmeKarari {
    if !ana_pencere_mi(hwnd) {
        return KucultmeKarari::Birak;
    }
    let kucultme = ayar::guncel().kucultme;
    let karar = davranis::kucultme_karari_ayarli(kucultme, unsafe { IsIconic(hwnd) } != 0, deneme);
    if karar == KucultmeKarari::Gizle {
        gizle(hwnd);
    }
    karar
}

/// Açılış taraması: araç Logseq açıkken başladıysa küçük pencereyi gizle, gizli olanı hatırla.
pub fn acilis_taramasi() {
    let kucultme = ayar::guncel().kucultme;
    for (h, d) in ana_pencereler() {
        match davranis::acilis_karari_ayarli(kucultme, d) {
            AcilisEylemi::Gizle => gizle(h as HWND),
            AcilisEylemi::Hatirla => kume().ekle(h),
            AcilisEylemi::Yok => {}
        }
    }
}

/// `GET /durum` için (sunucu iş parçacığı): (tepside gizli pencere sayısı, Logseq açık mı).
/// Salt okur: kümeyi budamaz, ayara yazmaz; bayat kimlikleri yalnız saymaz.
pub fn durum_ozeti() -> (usize, bool) {
    let ana = ana_listesi();
    let gizli = kume().gecerli_sayisi(|k| kumede_gecerli_mi(&ana, k));
    (gizli, !ana.is_empty())
}

/// Panik kancası (`gunluk::panik_kancasi_kur`): araç ölmeden önce tepsideki pencereler geri
/// gelsin. Panik küme kilidini tutarken olabilir: kilit alınamazsa görünmez ana pencereler
/// gösterilir. Kilit/küme dışında yalnız `ShowWindow` çağırır.
pub fn panikte_goster() {
    let kimlikler: Vec<isize> = match GIZLI.try_lock() {
        Ok(k) => k.hepsi().to_vec(),
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner().hepsi().to_vec(),
        Err(std::sync::TryLockError::WouldBlock) => ana_listesi()
            .into_iter()
            .filter(|(_, gorunur)| !gorunur)
            .map(|(h, _)| h as isize)
            .collect(),
    };
    for h in kimlikler {
        unsafe { ShowWindow(h as HWND, SW_SHOW) };
    }
}

/// Logseq ana penceresi (görünür ya da tepsiye gizlenmiş) var mı; salt okur, her iş parçacığında güvenli.
pub fn logseq_acik_mi() -> bool {
    !ana_listesi().is_empty()
}

/// Görünür Logseq ana penceresi sayısı (oturumda gizli başlatma yoklaması); salt okur.
pub fn gorunur_ana_sayisi() -> usize {
    ana_listesi().iter().filter(|(_, g)| *g).count()
}

/// Tepsi simgesinin menüsünde "Hepsini göster" öğesinin etkinliği için gizli sayısı.
pub fn gizli_sayisi() -> usize {
    kume().len()
}
