//! Süreç sorguları (Win32): adı verilen süreç çalışıyor mu, bir sürecin yolu ve başlangıç anı.

use logseq_tepsi_cekirdek::yollar;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE, MAX_PATH};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// Sürecin tam exe yolu; süreç açılamazsa (başka kullanıcı, yükseltilmiş) `None`.
pub fn yol(pid: u32) -> Option<String> {
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

/// Çalışan süreçler (kimlik, exe dosya adı). Liste alınamazsa `None`: süreç listesi boş olamaz
/// (en az bu süreç vardır), `Process32FirstW` başarısızlığı da "süreç yok" demek DEĞİL, belirsizliktir.
fn surecleri_say() -> Option<Vec<(u32, String)>> {
    unsafe {
        let anlik = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if anlik == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut girdi: PROCESSENTRY32W = std::mem::zeroed();
        girdi.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut var = Process32FirstW(anlik, &mut girdi) != 0;
        if !var {
            CloseHandle(anlik);
            return None;
        }
        let mut liste = Vec::new();
        while var {
            let ad_uzunlugu = girdi
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(girdi.szExeFile.len());
            liste.push((
                girdi.th32ProcessID,
                String::from_utf16_lossy(&girdi.szExeFile[..ad_uzunlugu]),
            ));
            var = Process32NextW(anlik, &mut girdi) != 0;
        }
        CloseHandle(anlik);
        Some(liste)
    }
}

/// Aynı adlı süreç `bilinen` yollardan birinde mi? Yol okunamadıysa (ya da bilinen yol yoksa)
/// temkinli: sayılır, çağıran "kapandı" kararıyla dosyaya yazmasın.
fn sayilir(yol: Option<&str>, bilinen: &[String]) -> bool {
    match yol {
        Some(y) if !bilinen.is_empty() => bilinen.iter().any(|b| yollar::ayni_yol_mu(b, y)),
        _ => true,
    }
}

/// Karar (saf): `liste` yoksa belirsizlik = çalışıyor; yoksa `ad`lı süreçlerden biri sayılırsa.
fn var_mi(
    liste: Option<&[(u32, String)]>,
    ad: &str,
    yol_oku: impl Fn(u32) -> Option<String>,
    bilinen: &[String],
) -> bool {
    let Some(liste) = liste else {
        return true;
    };
    liste
        .iter()
        .any(|(pid, a)| a.eq_ignore_ascii_case(ad) && sayilir(yol_oku(*pid).as_deref(), bilinen))
}

/// `ad` (exe dosya adı, büyük/küçük harf duyarsız) adlı bir süreç çalışıyor mu. Süreç listesi
/// alınamazsa belirsizlikte "çalışıyor" sayılır: çağıran "kapandı" kararıyla dosyaya yazmasın.
pub fn calisiyor_mu(ad: &str) -> bool {
    var_mi(surecleri_say().as_deref(), ad, |_| None, &[])
}

/// `calisiyor_mu`, ama aynı adı taşıyan başka bir programın süreci saymaz: tam yolu `bilinen`
/// yollardan (ASCII harf duyarsız) birine uymayan süreç yok sayılır. Yolu okunamayan süreç
/// (başka kullanıcı, yükseltilmiş) temkinli sayılır.
pub fn calisiyor_mu_bilinen(ad: &str, bilinen: &[String]) -> bool {
    var_mi(surecleri_say().as_deref(), ad, yol, bilinen)
}

/// 1601-01-01 ile 1970-01-01 arası (ms).
const FILETIME_UNIX_FARKI_MS: u64 = 11_644_473_600_000;

/// Sürecin başlangıç anı (unix ms); süreç açılamazsa `None`.
pub fn baslangic_ms(pid: u32) -> Option<u64> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let sifir = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut baslangic, mut bitis, mut cekirdek, mut kullanici) = (sifir, sifir, sifir, sifir);
        let tamam = GetProcessTimes(h, &mut baslangic, &mut bitis, &mut cekirdek, &mut kullanici);
        CloseHandle(h);
        if tamam == 0 {
            return None;
        }
        let yuz_ns =
            (u64::from(baslangic.dwHighDateTime) << 32) | u64::from(baslangic.dwLowDateTime);
        (yuz_ns / 10_000).checked_sub(FILETIME_UNIX_FARKI_MS)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kendi_sureci_var_olmayan_yok() {
        let ben = std::env::current_exe().unwrap();
        let ad = ben.file_name().unwrap().to_string_lossy().into_owned();
        assert!(calisiyor_mu(&ad), "{ad}");
        assert!(
            calisiyor_mu(&ad.to_uppercase()),
            "büyük/küçük harf duyarsız"
        );
        assert!(!calisiyor_mu("logseq-tepsi-olmayan-surec-xyz.exe"));
    }

    fn liste(girdiler: &[(u32, &str)]) -> Vec<(u32, String)> {
        girdiler
            .iter()
            .map(|(p, a)| (*p, (*a).to_owned()))
            .collect()
    }

    /// `Process32FirstW` başarısızsa (liste alınamadı) "süreç yok" DEĞİL, belirsizlik: çalışıyor.
    #[test]
    fn liste_alinamazsa_surec_var_sayilir_bos_liste_yok_der() {
        let yolsuz = |_: u32| None;
        assert!(
            var_mi(None, "Logseq.exe", yolsuz, &[]),
            "belirsizlik: var say"
        );
        assert!(!var_mi(Some(&[]), "Logseq.exe", yolsuz, &[]));
    }

    #[test]
    fn yalniz_bilinen_yoldaki_ayni_adli_surec_sayilir() {
        let l = liste(&[(1, "explorer.exe"), (2, "LOGSEQ.EXE"), (3, "Logseq.exe")]);
        let bilinen = vec![r"C:\Program Files\Logseq\Logseq.exe".to_owned()];
        let yollar = |pid: u32| match pid {
            2 => Some(r"D:\baska\Logseq.exe".to_owned()),
            3 => Some(r"c:/program files/logseq/logseq.exe".to_owned()),
            _ => None,
        };
        assert!(
            var_mi(Some(&l), "Logseq.exe", yollar, &bilinen),
            "bilinen yol (harf/ayraç duyarsız)"
        );
        // Yalnız bilinmeyen yoldaki aynı adlı süreç: başka bir program, sayılmaz.
        let yalniz_yabanci = liste(&[(2, "Logseq.exe")]);
        assert!(!var_mi(
            Some(&yalniz_yabanci),
            "Logseq.exe",
            yollar,
            &bilinen
        ));
        // Yolu okunamayan süreç temkinli sayılır; bilinen yol listesi boşsa ada göre.
        let okunamayan = liste(&[(9, "Logseq.exe")]);
        assert!(var_mi(Some(&okunamayan), "Logseq.exe", yollar, &bilinen));
        assert!(var_mi(Some(&yalniz_yabanci), "Logseq.exe", yollar, &[]));
        // Ad uymuyorsa yol ne olursa olsun yok.
        let baska_ad = liste(&[(1, "explorer.exe")]);
        assert!(!var_mi(Some(&baska_ad), "Logseq.exe", yollar, &bilinen));
    }

    #[test]
    fn gercek_surec_yolu_okunur_ve_bilinen_yolla_sinirlanir() {
        let ben = std::env::current_exe().unwrap();
        let ad = ben.file_name().unwrap().to_string_lossy().into_owned();
        let kendi = ben.to_string_lossy().into_owned();
        let okunan = yol(std::process::id()).expect("kendi yolumuz okunabilmeli");
        assert!(yollar::ayni_yol_mu(&okunan, &kendi), "{okunan} != {kendi}");
        assert!(calisiyor_mu_bilinen(&ad, &[kendi]), "kendi yolu bilinen");
        let baska = r"C:\olmayan\dizin\".to_owned() + &ad;
        assert!(
            !calisiyor_mu_bilinen(&ad, &[baska]),
            "bilinmeyen yoldaki süreç sayılmaz"
        );
    }

    #[test]
    fn baslangic_ani_makul() {
        let simdi = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let b = baslangic_ms(std::process::id()).expect("kendi sürecimiz açılabilmeli");
        assert!(b <= simdi, "başlangıç gelecekte: {b} > {simdi}");
        assert!(
            simdi - b < 6 * 3600 * 1000,
            "sınama süreci saatler önce başlamış olamaz"
        );
        assert_eq!(baslangic_ms(0xFFFF_FFF0), None, "var olmayan süreç");
    }
}
