//! Logseq yürütülebilir dosyasının yol adayları (Win32'den bağımsız saf kısım).
//! Sıra: çalışan ana pencerenin süreç yolu, ayar dosyasındaki son bilinen yol,
//! kayıt defteri (Uninstall) girdileri, son çare sabit yollar.

const EXE_ADI: &str = "Logseq.exe";

/// Son çare yollar; `%LOCALAPPDATA%` değeri verilirse Squirrel/kullanıcı kurulumları da eklenir.
pub fn sabit_yollar(localappdata: Option<&str>) -> Vec<String> {
    let mut v = vec![r"C:\Program Files\Logseq\Logseq.exe".to_owned()];
    if let Some(d) = localappdata.map(|d| d.trim_end_matches('\\')) {
        if !d.is_empty() {
            v.push(format!(r"{d}\Programs\Logseq\{EXE_ADI}"));
            v.push(format!(r"{d}\Logseq\{EXE_ADI}"));
        }
    }
    v
}

/// Kayıt defterindeki bir Uninstall anahtarından okunan değerler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KayitDegerleri {
    pub anahtar_adi: String,
    pub gorunen_ad: Option<String>,
    pub kurulum_konumu: Option<String>,
    pub gorunen_simge: Option<String>,
    pub kaldirma_komutu: Option<String>,
}

/// ASCII büyük/küçük harfe duyarsız "içerir" (Türkçe `to_lowercase` tuzağı yok).
fn icerir_ci(metin: &str, aranan: &str) -> bool {
    metin
        .as_bytes()
        .windows(aranan.len())
        .any(|w| w.eq_ignore_ascii_case(aranan.as_bytes()))
}

/// Bu Uninstall girdisi Logseq'e mi ait? (Anahtar adı ya da görünen ad "logseq" içerir.)
pub fn logseq_girdisi_mi(anahtar_adi: &str, gorunen_ad: Option<&str>) -> bool {
    icerir_ci(anahtar_adi, "logseq") || gorunen_ad.is_some_and(|a| icerir_ci(a, "logseq"))
}

/// `DisplayIcon`/`UninstallString` değerinden dosya yolunu çıkarır: tırnaklıysa tırnak
/// içi (argümanlar atılır), değilse tümü; sondaki simge dizini (`,0`) atılır.
pub fn komuttan_yol(deger: &str) -> Option<String> {
    let d = deger.trim();
    let yol = if let Some(kalan) = d.strip_prefix('"') {
        kalan.split('"').next().unwrap_or("")
    } else {
        match d.rsplit_once(',') {
            Some((onceki, sonra))
                if !sonra.is_empty()
                    && sonra
                        .trim_start_matches('-')
                        .bytes()
                        .all(|b| b.is_ascii_digit()) =>
            {
                onceki
            }
            _ => d,
        }
    };
    let yol = yol.trim();
    (!yol.is_empty()).then(|| yol.to_owned())
}

fn dosya_adi(yol: &str) -> &str {
    yol.rsplit(['\\', '/']).next().unwrap_or(yol)
}

fn dizin(yol: &str) -> Option<&str> {
    yol.rsplit_once(['\\', '/']).map(|(d, _)| d)
}

fn exe_mi(yol: &str) -> bool {
    dosya_adi(yol).eq_ignore_ascii_case(EXE_ADI)
}

/// Bir Uninstall girdisinden `Logseq.exe` adayları (sırayla: kurulum konumu, simge,
/// kaldırma komutunun dizini). Logseq'e ait olmayan girdi boş döner.
pub fn kayit_adaylari(d: &KayitDegerleri) -> Vec<String> {
    if !logseq_girdisi_mi(&d.anahtar_adi, d.gorunen_ad.as_deref()) {
        return Vec::new();
    }
    let mut v = Vec::new();
    if let Some(k) = d.kurulum_konumu.as_deref().and_then(komuttan_yol) {
        v.push(format!(r"{}\{EXE_ADI}", k.trim_end_matches('\\')));
    }
    if let Some(s) = d.gorunen_simge.as_deref().and_then(komuttan_yol) {
        if exe_mi(&s) {
            v.push(s);
        } else if let Some(dz) = dizin(&s) {
            v.push(format!(r"{dz}\{EXE_ADI}"));
        }
    }
    if let Some(k) = d.kaldirma_komutu.as_deref().and_then(komuttan_yol) {
        if let Some(dz) = dizin(&k) {
            v.push(format!(r"{dz}\{EXE_ADI}"));
        }
    }
    v
}

/// Ayar dosyası içeriği: ilk dolu satır (BOM atılır) `Logseq.exe` yolu olmalı.
pub fn ayar_yolu(icerik: &str) -> Option<String> {
    let satir = icerik
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())?;
    exe_mi(satir).then(|| satir.to_owned())
}

/// Adayları birleştirir: çalışan, ayar, kayıt, sabit sırasıyla; boşlar atılır, tekrarlar
/// (büyük/küçük harfe duyarsız) ilk görüldüğü yerde kalır.
pub fn adaylar(
    calisan: Option<String>,
    ayar: Option<String>,
    kayit: Vec<String>,
    sabit: Vec<String>,
) -> Vec<String> {
    let mut sonuc: Vec<String> = Vec::new();
    for yol in calisan.into_iter().chain(ayar).chain(kayit).chain(sabit) {
        let yol = yol.trim().to_owned();
        if !yol.is_empty() && !sonuc.iter().any(|s| s.eq_ignore_ascii_case(&yol)) {
            sonuc.push(yol);
        }
    }
    sonuc
}

#[cfg(test)]
mod testler {
    use super::*;

    fn degerler(
        anahtar: &str,
        ad: Option<&str>,
        konum: Option<&str>,
        simge: Option<&str>,
        kaldir: Option<&str>,
    ) -> KayitDegerleri {
        KayitDegerleri {
            anahtar_adi: anahtar.into(),
            gorunen_ad: ad.map(Into::into),
            kurulum_konumu: konum.map(Into::into),
            gorunen_simge: simge.map(Into::into),
            kaldirma_komutu: kaldir.map(Into::into),
        }
    }

    #[test]
    fn komuttan_yol_cesitleri() {
        assert_eq!(
            komuttan_yol(r#""C:\Program Files\Logseq\Uninstall Logseq.exe" /allusers"#).as_deref(),
            Some(r"C:\Program Files\Logseq\Uninstall Logseq.exe")
        );
        assert_eq!(
            komuttan_yol(r"C:\Program Files\Logseq\uninstallerIcon.ico").as_deref(),
            Some(r"C:\Program Files\Logseq\uninstallerIcon.ico")
        );
        assert_eq!(
            komuttan_yol(r"C:\Users\x\AppData\Local\Logseq\app.ico,0").as_deref(),
            Some(r"C:\Users\x\AppData\Local\Logseq\app.ico")
        );
        assert_eq!(
            komuttan_yol(r#""C:\a b\Logseq.exe",-3"#).as_deref(),
            Some(r"C:\a b\Logseq.exe")
        );
        // Virgüllü ama sayısal olmayan sonek yolun parçasıdır.
        assert_eq!(
            komuttan_yol(r"C:\a,b\x.exe").as_deref(),
            Some(r"C:\a,b\x.exe")
        );
        assert_eq!(komuttan_yol("  "), None);
        assert_eq!(komuttan_yol("\"\""), None);
    }

    #[test]
    fn nsis_girdisi_kurulum_konumu_olmadan_kaldirma_komutundan() {
        // Makinedeki gerçek HKLM girdisinin biçimi (InstallLocation yok).
        let d = degerler(
            "8b963d14-71b6-5579-927a-616b0650a9c8",
            Some("Logseq 2.0.2"),
            None,
            Some(r"C:\Program Files\Logseq\uninstallerIcon.ico"),
            Some(r#""C:\Program Files\Logseq\Uninstall Logseq.exe" /allusers"#),
        );
        let a = kayit_adaylari(&d);
        assert_eq!(a[0], r"C:\Program Files\Logseq\Logseq.exe");
        assert!(a.iter().all(|y| y == r"C:\Program Files\Logseq\Logseq.exe"));
    }

    #[test]
    fn squirrel_girdisi_kurulum_konumundan() {
        let d = degerler(
            "Logseq",
            Some("Logseq"),
            Some(r"C:\Users\kullanici\AppData\Local\Logseq"),
            Some(r"C:\Users\kullanici\AppData\Local\Logseq\app.ico"),
            Some(r#""C:\Users\kullanici\AppData\Local\Logseq\Update.exe" --uninstall"#),
        );
        let a = kayit_adaylari(&d);
        assert_eq!(a[0], r"C:\Users\kullanici\AppData\Local\Logseq\Logseq.exe");
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn simge_exe_ise_dogrudan_aday() {
        let d = degerler(
            "x",
            Some("Logseq"),
            Some(r"C:\k\"),
            Some(r#""C:\k\app-1\Logseq.exe",0"#),
            None,
        );
        let a = kayit_adaylari(&d);
        assert_eq!(a, vec![r"C:\k\Logseq.exe", r"C:\k\app-1\Logseq.exe"]);
    }

    #[test]
    fn logseq_olmayan_girdi_bos() {
        let d = degerler("Chrome", Some("Google Chrome"), Some(r"C:\c"), None, None);
        assert!(kayit_adaylari(&d).is_empty());
        assert!(logseq_girdisi_mi("LOGSEQ", None));
        assert!(logseq_girdisi_mi("x", Some("My LogSeq")));
        assert!(!logseq_girdisi_mi("x", None));
        assert!(!logseq_girdisi_mi("ab", Some("")));
    }

    #[test]
    fn ayar_dosyasi_ayristirma() {
        assert_eq!(
            ayar_yolu("\u{feff}C:\\a\\Logseq.exe\r\n").as_deref(),
            Some(r"C:\a\Logseq.exe")
        );
        assert_eq!(
            ayar_yolu("\n  C:\\b\\logseq.EXE  \nsonra").as_deref(),
            Some(r"C:\b\logseq.EXE")
        );
        assert_eq!(ayar_yolu(""), None);
        assert_eq!(ayar_yolu("C:\\a\\baska.exe"), None);
        assert_eq!(ayar_yolu("C:\\a\\Logseq.exe.bak"), None);
    }

    #[test]
    fn aday_sirasi_calisan_ayar_kayit_sabit() {
        let a = adaylar(
            Some(r"C:\calisan\Logseq.exe".into()),
            Some(r"C:\ayar\Logseq.exe".into()),
            vec![
                r"C:\kayit1\Logseq.exe".into(),
                r"C:\kayit2\Logseq.exe".into(),
            ],
            vec![r"C:\sabit\Logseq.exe".into()],
        );
        assert_eq!(
            a,
            vec![
                r"C:\calisan\Logseq.exe",
                r"C:\ayar\Logseq.exe",
                r"C:\kayit1\Logseq.exe",
                r"C:\kayit2\Logseq.exe",
                r"C:\sabit\Logseq.exe"
            ]
        );
    }

    #[test]
    fn aday_tekrarlari_harfe_duyarsiz_birlesir_bos_atilir() {
        let a = adaylar(
            None,
            Some(r"C:\PROGRAM FILES\Logseq\Logseq.exe".into()),
            vec![String::new(), "  ".into()],
            vec![
                r"c:\program files\logseq\logseq.exe".into(),
                r"D:\x\Logseq.exe".into(),
            ],
        );
        assert_eq!(
            a,
            vec![r"C:\PROGRAM FILES\Logseq\Logseq.exe", r"D:\x\Logseq.exe"]
        );
    }

    #[test]
    fn sabit_yollar_localappdata_ile_genisler() {
        assert_eq!(
            sabit_yollar(None),
            vec![r"C:\Program Files\Logseq\Logseq.exe"]
        );
        let s = sabit_yollar(Some(r"C:\Users\u\AppData\Local\"));
        assert_eq!(s[0], r"C:\Program Files\Logseq\Logseq.exe");
        assert_eq!(s[1], r"C:\Users\u\AppData\Local\Programs\Logseq\Logseq.exe");
        assert_eq!(s[2], r"C:\Users\u\AppData\Local\Logseq\Logseq.exe");
        assert_eq!(sabit_yollar(Some("")).len(), 1);
    }
}
