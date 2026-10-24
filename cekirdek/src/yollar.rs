//! Logseq yürütülebilir dosyasının yol adayları (Win32'den bağımsız saf kısım).
//! Sıra: çalışan ana pencerenin süreç yolu, ayar dosyasındaki son bilinen yol,
//! kayıt defteri (Uninstall) girdileri (`DisplayVersion`'ı en yüksek olan önce), son çare
//! sabit yollar.

use std::cmp::Ordering;

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
    pub gorunen_surum: Option<String>,
}

/// Kayıt defterinden gelen bir `Logseq.exe` adayı ve girdisinin `DisplayVersion` değeri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KayitAdayi {
    pub yol: String,
    pub surum: Option<String>,
}

/// Sürüm dizelerini parça parça sayısal karşılaştırır (`0.10.15` < `2.0.2`, `2.0` = `2.0.0`).
/// Parçanın baştaki rakamları alınır (`2-beta` → 2); rakamsız ya da eksik parça 0 sayılır.
pub fn surum_karsilastir(a: &str, b: &str) -> Ordering {
    fn parca(p: &str) -> u64 {
        let r: String = p.chars().take_while(char::is_ascii_digit).collect();
        r.parse().unwrap_or(0)
    }
    let (mut x, mut y) = (a.trim().split('.'), b.trim().split('.'));
    loop {
        let (p, q) = (x.next(), y.next());
        if p.is_none() && q.is_none() {
            return Ordering::Equal;
        }
        let o = parca(p.unwrap_or("0")).cmp(&parca(q.unwrap_or("0")));
        if o != Ordering::Equal {
            return o;
        }
    }
}

/// Sürümü bilinen > bilinmeyen; bilinenler arasında büyük sürüm önce.
fn surum_siralamasi(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => surum_karsilastir(y, x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
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

/// `kayit_adaylari` yolları, girdinin `DisplayVersion` değeriyle işaretli.
pub fn surumlu_kayit_adaylari(d: &KayitDegerleri) -> Vec<KayitAdayi> {
    kayit_adaylari(d)
        .into_iter()
        .map(|yol| KayitAdayi {
            yol,
            surum: d.gorunen_surum.clone().filter(|s| !s.trim().is_empty()),
        })
        .collect()
}

/// Başlangıç kısayolunun `kur` karşısındaki durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KisayolKarari {
    /// Kısayol yok: oluştur.
    Olustur,
    /// Kısayol aynı exe'yi gösteriyor: dokunma.
    Dokunma,
    /// Kısayol başka exe'yi gösteriyor ya da hedefi okunamadı: yeniden yaz.
    Yeniden,
}

/// İki yol aynı dosyayı mı gösteriyor (ASCII harf duyarsız, `/` = `\`, sondaki `\` yok sayılır).
pub fn ayni_yol_mu(a: &str, b: &str) -> bool {
    let duz = |s: &str| {
        s.trim()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_owned()
    };
    duz(a).eq_ignore_ascii_case(&duz(b))
}

/// `var`: kısayol dosyası var mı; `hedef`: gösterdiği yol (okunamadıysa `None`); `exe`: bu araç.
pub fn kisayol_karari(var: bool, hedef: Option<&str>, exe: &str) -> KisayolKarari {
    match (var, hedef) {
        (false, _) => KisayolKarari::Olustur,
        (true, Some(h)) if ayni_yol_mu(h, exe) => KisayolKarari::Dokunma,
        (true, _) => KisayolKarari::Yeniden,
    }
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

/// Adayları birleştirir: çalışan, ayar, kayıt, sabit sırasıyla; kayıt adayları kendi içinde
/// `DisplayVersion`'a göre büyükten küçüğe (eşitlikte kayıt sırası) dizilir: eski bir Squirrel
/// kurulumu yeni kurulumun önüne geçemez. Boşlar atılır, tekrarlar (büyük/küçük harfe
/// duyarsız) ilk görüldüğü yerde kalır.
pub fn adaylar(
    calisan: Option<String>,
    ayar: Option<String>,
    mut kayit: Vec<KayitAdayi>,
    sabit: Vec<String>,
) -> Vec<String> {
    kayit.sort_by(|a, b| surum_siralamasi(&a.surum, &b.surum));
    let kayit = kayit.into_iter().map(|k| k.yol);
    let mut sonuc: Vec<String> = Vec::new();
    for yol in calisan.into_iter().chain(ayar).chain(kayit).chain(sabit) {
        let yol = yol.trim().to_owned();
        if !yol.is_empty() && !sonuc.iter().any(|s| s.eq_ignore_ascii_case(&yol)) {
            sonuc.push(yol);
        }
    }
    sonuc
}

/// Squirrel yolundaki `app-<sürüm>` dizininden sürüm (`...\app-0.10.15\Logseq.exe`).
pub fn squirrel_surumu(yol: &str) -> Option<String> {
    yol.split(['\\', '/']).find_map(|p| {
        let on = p.get(..4)?;
        on.eq_ignore_ascii_case("app-")
            .then(|| p[4..].to_owned())
            .filter(|s| s.starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// Squirrel düzeni mi: kök başlatıcı (`<kök>\Logseq.exe`) ya da `<kök>\app-*\Logseq.exe`, ve
/// `<kök>`'te `Update.exe` var. `update_var` kök dizin için bunu söyler (dosya sistemi dışarıda).
pub fn squirrel_mi(yol: &str, update_var: impl Fn(&str) -> bool) -> bool {
    let Some(ust) = dizin(yol) else { return false };
    let kok = if squirrel_surumu(yol).is_some() {
        dizin(ust).unwrap_or(ust)
    } else {
        ust
    };
    update_var(kok)
}

/// Yolun bilinen sürümü: Squirrel `app-<sürüm>` dizini, yoksa aynı yolu veren kayıt girdisi.
fn yol_surumu(yol: &str, kayit: &[KayitAdayi]) -> Option<String> {
    squirrel_surumu(yol).or_else(|| {
        kayit
            .iter()
            .find(|k| k.yol.eq_ignore_ascii_case(yol))
            .and_then(|k| k.surum.clone())
    })
}

/// Bu yol ayara yazılmalı/ayardan geçerli sayılmalı mı? Sürümü bilinen en yüksek kurulumdan
/// düşükse hayır (eski Squirrel kurulumu 2.x'in yerini almasın). Sürümü bilinmeyen yol,
/// Squirrel düzeniyse (daha yüksek sürüm varken) hayır, değilse evet.
pub fn hatirlanmali_mi(yol: &str, kayit: &[KayitAdayi], update_var: impl Fn(&str) -> bool) -> bool {
    let en_yuksek = kayit
        .iter()
        .filter_map(|k| k.surum.as_deref())
        .max_by(|a, b| surum_karsilastir(a, b));
    let Some(en) = en_yuksek else { return true };
    match yol_surumu(yol, kayit) {
        Some(s) => surum_karsilastir(&s, en) != Ordering::Less,
        None => !squirrel_mi(yol, update_var),
    }
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
            gorunen_surum: None,
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

    fn kayit(yol: &str, surum: Option<&str>) -> KayitAdayi {
        KayitAdayi {
            yol: yol.into(),
            surum: surum.map(Into::into),
        }
    }

    const SQUIRREL_KOK: &str = r"C:\Users\kullanici\AppData\Local\Logseq";
    const SQUIRREL_APP: &str = r"C:\Users\kullanici\AppData\Local\Logseq\app-0.10.15\Logseq.exe";
    const SQUIRREL_BASLATICI: &str = r"C:\Users\kullanici\AppData\Local\Logseq\Logseq.exe";
    const NSIS: &str = r"C:\Program Files\Logseq\Logseq.exe";

    /// Makinedeki gerçek durum: HKCU'da eski Squirrel 0.10.15, HKLM'de 2.0.2.
    fn iki_kurulum() -> Vec<KayitAdayi> {
        let squirrel = degerler(
            "Logseq",
            Some("Logseq"),
            Some(SQUIRREL_KOK),
            Some(r"C:\Users\kullanici\AppData\Local\Logseq\app.ico"),
            None,
        );
        let nsis = degerler(
            "8b963d14-71b6-5579-927a-616b0650a9c8",
            Some("Logseq 2.0.2"),
            None,
            Some(r"C:\Program Files\Logseq\uninstallerIcon.ico"),
            None,
        );
        let (mut s, mut n) = (squirrel, nsis);
        s.gorunen_surum = Some("0.10.15".into());
        n.gorunen_surum = Some("2.0.2".into());
        // Kayıt tarama sırası: HKCU (eski) önce, HKLM (yeni) sonra.
        let mut v = surumlu_kayit_adaylari(&s);
        v.extend(surumlu_kayit_adaylari(&n));
        v
    }

    #[test]
    fn surum_karsilastirma_parca_parca_sayisal() {
        use std::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(surum_karsilastir("0.10.15", "2.0.2"), Less);
        assert_eq!(surum_karsilastir("2.0.2", "0.10.15"), Greater);
        // Metin sırası "0.10.15" < "0.9.2" der; sayısal sıra tersini.
        assert_eq!(surum_karsilastir("0.10.15", "0.9.2"), Greater);
        assert_eq!(surum_karsilastir("10.0.0", "9.9.9"), Greater);
        assert_eq!(surum_karsilastir("2.0", "2.0.0"), Equal);
        assert_eq!(surum_karsilastir("2.0.2", "2.0.2"), Equal);
        assert_eq!(surum_karsilastir("2.0.2-beta", "2.0.2"), Equal);
        assert_eq!(surum_karsilastir(" 2.1 ", "2.0.9"), Greater);
        assert_eq!(surum_karsilastir("", "0.0.1"), Less);
    }

    /// Eski sınama (`aday_sirasi_calisan_ayar_kayit_sabit`) kayıt adaylarını girdi sırasında
    /// bırakmayı sabitliyordu; bu, HKCU'daki eski Squirrel 0.10.15'i HKLM'deki 2.0.2'nin
    /// önüne geçiriyordu. Doğru sıra: çalışan, ayar, kayıt (DisplayVersion büyükten küçüğe,
    /// eşit/bilinmeyende girdi sırası), sabit.
    #[test]
    fn aday_sirasi_calisan_ayar_kayit_surumle_sabit() {
        let a = adaylar(
            Some(r"C:\calisan\Logseq.exe".into()),
            Some(r"C:\ayar\Logseq.exe".into()),
            vec![
                kayit(r"C:\kayit1\Logseq.exe", Some("1.0.0")),
                kayit(r"C:\kayit2\Logseq.exe", Some("2.0.0")),
                kayit(r"C:\kayit3\Logseq.exe", None),
                kayit(r"C:\kayit4\Logseq.exe", None),
            ],
            vec![r"C:\sabit\Logseq.exe".into()],
        );
        assert_eq!(
            a,
            vec![
                r"C:\calisan\Logseq.exe",
                r"C:\ayar\Logseq.exe",
                r"C:\kayit2\Logseq.exe",
                r"C:\kayit1\Logseq.exe",
                r"C:\kayit3\Logseq.exe",
                r"C:\kayit4\Logseq.exe",
                r"C:\sabit\Logseq.exe"
            ]
        );
    }

    #[test]
    fn iki_kurulum_yeni_olan_secilir() {
        let a = adaylar(None, None, iki_kurulum(), sabit_yollar(None));
        assert_eq!(a[0], NSIS, "2.0.2 eski Squirrel'in önünde olmalı: {a:?}");
        let sq = a
            .iter()
            .position(|y| y.eq_ignore_ascii_case(SQUIRREL_BASLATICI))
            .unwrap();
        assert!(sq > 0);
    }

    #[test]
    fn squirrel_yolu_ve_surumu_taninir() {
        assert_eq!(squirrel_surumu(SQUIRREL_APP).as_deref(), Some("0.10.15"));
        assert_eq!(squirrel_surumu(SQUIRREL_BASLATICI), None);
        assert_eq!(squirrel_surumu(NSIS), None);
        assert_eq!(squirrel_surumu(r"C:\x\apps\Logseq.exe"), None);
        assert_eq!(squirrel_surumu(r"C:\x\app-beta\Logseq.exe"), None);
        let update_var = |kok: &str| kok.eq_ignore_ascii_case(SQUIRREL_KOK);
        assert!(squirrel_mi(SQUIRREL_APP, update_var));
        assert!(squirrel_mi(SQUIRREL_BASLATICI, update_var));
        assert!(!squirrel_mi(NSIS, update_var));
        assert!(!squirrel_mi("Logseq.exe", update_var));
    }

    #[test]
    fn eski_squirrel_yeni_kurulum_varken_hatirlanmaz() {
        let k = iki_kurulum();
        let update_var = |kok: &str| kok.eq_ignore_ascii_case(SQUIRREL_KOK);
        assert!(!hatirlanmali_mi(SQUIRREL_APP, &k, update_var));
        assert!(!hatirlanmali_mi(SQUIRREL_BASLATICI, &k, update_var));
        assert!(hatirlanmali_mi(NSIS, &k, update_var));
        // Sürümü hiç bilinmeyen yol (kayıt girdisi yok): Squirrel düzeniyse hayır, değilse evet.
        assert!(!hatirlanmali_mi(
            SQUIRREL_BASLATICI,
            &[kayit(NSIS, Some("99.0"))],
            update_var
        ));
        assert!(hatirlanmali_mi(
            r"D:\tasinabilir\Logseq.exe",
            &k,
            update_var
        ));
    }

    #[test]
    fn tek_squirrel_kurulumu_hatirlanir_ve_kayit_yoksa_her_yol() {
        let update_var = |kok: &str| kok.eq_ignore_ascii_case(SQUIRREL_KOK);
        let tek = [kayit(SQUIRREL_BASLATICI, Some("0.10.15"))];
        assert!(hatirlanmali_mi(SQUIRREL_APP, &tek, update_var));
        assert!(hatirlanmali_mi(SQUIRREL_BASLATICI, &tek, update_var));
        assert!(hatirlanmali_mi(SQUIRREL_APP, &[], update_var));
        assert!(hatirlanmali_mi(NSIS, &[kayit(NSIS, None)], update_var));
    }

    #[test]
    fn aday_tekrarlari_harfe_duyarsiz_birlesir_bos_atilir() {
        let a = adaylar(
            None,
            Some(r"C:\PROGRAM FILES\Logseq\Logseq.exe".into()),
            vec![kayit("", None), kayit("  ", None)],
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
    fn kisayol_karari_yok_ayni_baska_okunamadi() {
        let exe = r"C:\a\logseq-tepsi.exe";
        assert_eq!(kisayol_karari(false, None, exe), KisayolKarari::Olustur);
        assert_eq!(
            kisayol_karari(false, Some(exe), exe),
            KisayolKarari::Olustur
        );
        assert_eq!(
            kisayol_karari(true, Some(r"c:/A/Logseq-Tepsi.exe"), exe),
            KisayolKarari::Dokunma
        );
        assert_eq!(
            kisayol_karari(true, Some(r"C:\eski\logseq-tepsi.exe"), exe),
            KisayolKarari::Yeniden
        );
        assert_eq!(kisayol_karari(true, Some(""), exe), KisayolKarari::Yeniden);
        assert_eq!(kisayol_karari(true, None, exe), KisayolKarari::Yeniden);
        assert!(ayni_yol_mu(r"C:\a\", r"C:\A"));
        assert!(!ayni_yol_mu(r"C:\a", r"C:\ab"));
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
