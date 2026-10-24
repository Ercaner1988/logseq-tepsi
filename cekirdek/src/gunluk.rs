//! Küçük dönen günlük: `%LOCALAPPDATA%\logseq-tepsi\gunluk.txt` (UTC zaman damgalı).
//! Yazma hatası sessizce yutulur: günlük aracı asla düşürmez.

use std::io::Write;
use std::path::PathBuf;

/// Günlük bu boyutu aşınca `gunluk.eski.txt` olur, yeni dosya açılır.
const BOYUT_SINIRI: u64 = 64 * 1024;

/// Aracın veri dizini (`%LOCALAPPDATA%\logseq-tepsi`).
pub fn dizin() -> Option<PathBuf> {
    let d = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(d).join("logseq-tepsi"))
}

pub fn donmeli_mi(boyut: u64) -> bool {
    boyut >= BOYUT_SINIRI
}

/// Unix saniyesi → `YYYY-MM-DD HH:MM:SS` (UTC, Gregoryen takvimi).
pub fn zaman_bicimle(sn: u64) -> String {
    let gun = (sn / 86_400) as i64;
    let gunici = sn % 86_400;
    // Howard Hinnant'ın "civil from days" algoritması.
    let z = gun + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let g = doy - (153 * mp + 2) / 5 + 1;
    let ay = if mp < 10 { mp + 3 } else { mp - 9 };
    let yil = yoe + era * 400 + i64::from(ay <= 2);
    format!(
        "{yil:04}-{ay:02}-{g:02} {:02}:{:02}:{:02}",
        gunici / 3_600,
        gunici % 3_600 / 60,
        gunici % 60
    )
}

/// Tek günlük satırı; ileti içindeki satır sonları boşluğa çevrilir.
pub fn satir(sn: u64, ileti: &str) -> String {
    let tek: String = ileti
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();
    format!("{} {tek}\n", zaman_bicimle(sn))
}

pub fn yaz(ileti: &str) {
    let Some(d) = dizin() else { return };
    let _ = std::fs::create_dir_all(&d);
    let dosya = d.join("gunluk.txt");
    if let Ok(m) = std::fs::metadata(&dosya) {
        if donmeli_mi(m.len()) {
            let _ = std::fs::rename(&dosya, d.join("gunluk.eski.txt"));
        }
    }
    let sn = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |s| s.as_secs());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dosya)
    {
        let _ = f.write_all(satir(sn, ileti).as_bytes());
    }
}

/// Panik kancasının gövdesi: iletiyi `yaz` ile günlüğe yazar, ardından `geri_cagri`yı çağırır
/// (kök crate pencereleri geri getirir; çekirdek Win32'ye bağlanmaz).
pub fn panik_kancasi(
    yaz: fn(&str),
    geri_cagri: fn(),
) -> Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static> {
    Box::new(move |bilgi| {
        yaz(&format!("PANIK: {bilgi}"));
        geri_cagri();
    })
}

/// Panik iletisini günlüğe yazar ve `geri_cagri`yı çağırır (`panic = "abort"` olduğundan süreç
/// hemen ardından ölür; tepsideki pencereler kayıp kalmasın diye geri çağrı onları gösterir).
pub fn panik_kancasi_kur(geri_cagri: fn()) {
    std::panic::set_hook(panik_kancasi(yaz, geri_cagri));
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn zaman_bicimi_bilinen_degerler() {
        assert_eq!(zaman_bicimle(0), "1970-01-01 00:00:00");
        assert_eq!(zaman_bicimle(951_782_400), "2000-02-29 00:00:00"); // artık gün
        assert_eq!(zaman_bicimle(1_000_000_000), "2001-09-09 01:46:40");
        assert_eq!(zaman_bicimle(1_761_350_523), "2025-10-25 00:02:03");
        assert_eq!(zaman_bicimle(4_102_444_799), "2099-12-31 23:59:59");
    }

    #[test]
    fn satir_tek_satir_ve_yeni_satirla_biter() {
        let s = satir(0, "a\r\nb\nc");
        assert_eq!(s, "1970-01-01 00:00:00 a  b c\n");
        assert_eq!(s.matches('\n').count(), 1);
    }

    #[test]
    fn donme_siniri() {
        assert!(!donmeli_mi(0));
        assert!(!donmeli_mi(BOYUT_SINIRI - 1));
        assert!(donmeli_mi(BOYUT_SINIRI));
        assert!(donmeli_mi(BOYUT_SINIRI + 1));
    }

    static YAZILAN: AtomicUsize = AtomicUsize::new(0);
    static GERI_CAGRI: AtomicUsize = AtomicUsize::new(0);
    static SON_SIRA: AtomicUsize = AtomicUsize::new(0);
    static KANCA_KILIDI: Mutex<()> = Mutex::new(());

    fn sahte_yaz(ileti: &str) {
        assert!(ileti.starts_with("PANIK: "), "{ileti}");
        assert!(ileti.contains("deneme paniği"), "{ileti}");
        YAZILAN.store(
            SON_SIRA.fetch_add(1, Ordering::SeqCst) + 1,
            Ordering::SeqCst,
        );
    }

    fn sahte_geri_cagri() {
        GERI_CAGRI.store(
            SON_SIRA.fetch_add(1, Ordering::SeqCst) + 1,
            Ordering::SeqCst,
        );
    }

    /// Küresel panik kancasını geçici değiştirir (gerçek günlüğe yazmaz); kanca sınaması tek koşar.
    #[test]
    fn panikte_gunluk_yazilir_sonra_geri_cagri_calisir() {
        let _k = KANCA_KILIDI.lock().unwrap_or_else(|e| e.into_inner());
        let eski = std::panic::take_hook();
        std::panic::set_hook(panik_kancasi(sahte_yaz, sahte_geri_cagri));
        let sonuc = std::panic::catch_unwind(|| panic!("deneme paniği"));
        std::panic::set_hook(eski);
        assert!(sonuc.is_err());
        assert_eq!(YAZILAN.load(Ordering::SeqCst), 1, "günlük önce");
        assert_eq!(GERI_CAGRI.load(Ordering::SeqCst), 2, "geri çağrı sonra");
    }
}
