//! Süreç sorguları (Win32): adı verilen süreç çalışıyor mu, bir sürecin başlangıç anı.

use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// `ad` (exe dosya adı, büyük/küçük harf duyarsız) adlı bir süreç çalışıyor mu. Süreç listesi
/// alınamazsa belirsizlikte "çalışıyor" sayılır: çağıran "kapandı" kararıyla dosyaya yazmasın.
pub fn calisiyor_mu(ad: &str) -> bool {
    unsafe {
        let anlik = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if anlik == INVALID_HANDLE_VALUE {
            return true;
        }
        let mut girdi: PROCESSENTRY32W = std::mem::zeroed();
        girdi.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut bulundu = false;
        let mut var = Process32FirstW(anlik, &mut girdi) != 0;
        while var && !bulundu {
            let ad_uzunlugu = girdi
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(girdi.szExeFile.len());
            bulundu =
                String::from_utf16_lossy(&girdi.szExeFile[..ad_uzunlugu]).eq_ignore_ascii_case(ad);
            var = Process32NextW(anlik, &mut girdi) != 0;
        }
        CloseHandle(anlik);
        bulundu
    }
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
