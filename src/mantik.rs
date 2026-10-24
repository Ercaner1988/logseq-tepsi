//! Win32'den bağımsız saf mantık: pencere seçimi ve göster/gizle karar tablosu.

/// Logseq'in kurulu yürütülebilir dosyası.
pub const LOGSEQ_YOLU: &str = r"C:\Program Files\Logseq\Logseq.exe";
const SUREC_ADI: &str = "Logseq.exe";
const SINIF_ADI: &str = "Chrome_WidgetWin_1";
/// `WS_EX_TOOLWINDOW`.
pub const EX_ARAC_PENCERESI: u32 = 0x0000_0080;

/// Bir üst düzey pencerenin seçim için gereken özellikleri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aday {
    pub surec_adi: String,
    pub sinif: String,
    pub sahipli: bool,
    pub baslik_var: bool,
    pub ex_stil: u32,
}

/// Aday, Logseq'in ana penceresi mi? (Başka Logseq.exe süreçlerinin yardımcı
/// pencereleri, sahipli diyaloglar, araç pencereleri ve başlıksızlar elenir.)
pub fn ana_pencere_mi(a: &Aday) -> bool {
    a.surec_adi.eq_ignore_ascii_case(SUREC_ADI)
        && a.sinif.eq_ignore_ascii_case(SINIF_ADI)
        && !a.sahipli
        && a.baslik_var
        && a.ex_stil & EX_ARAC_PENCERESI == 0
}

/// Uygun adaylar arasında görünür olanı, yoksa ilk uygun olanı seçer.
/// Girdi: (aday, görünür mü). Çıktı: seçilenin dizin numarası.
pub fn sec(adaylar: &[(Aday, bool)]) -> Option<usize> {
    let uygun = |i: &usize| ana_pencere_mi(&adaylar[*i].0);
    (0..adaylar.len())
        .filter(uygun)
        .find(|&i| adaylar[i].1)
        .or_else(|| (0..adaylar.len()).find(uygun))
}

/// Seçilmiş pencerenin anlık durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PencereDurumu {
    pub gorunur: bool,
    pub simge_durumunda: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eylem {
    Baslat,
    Goster,
    Gizle,
}

/// Tepsi simgesine sol tık kararı.
pub fn tikla_karari(pencere: Option<PencereDurumu>) -> Eylem {
    match pencere {
        None => Eylem::Baslat,
        Some(d) if d.gorunur && !d.simge_durumunda => Eylem::Gizle,
        Some(_) => Eylem::Goster,
    }
}

/// "Göster" menü öğesi: pencere yoksa başlat, varsa her durumda göster.
pub fn goster_karari(pencere: Option<PencereDurumu>) -> Eylem {
    match pencere {
        None => Eylem::Baslat,
        Some(_) => Eylem::Goster,
    }
}

/// "Tepsiye gizle" menü öğesi: yalnız görünür pencere gizlenir.
pub fn gizle_karari(pencere: Option<PencereDurumu>) -> Option<Eylem> {
    pencere.filter(|d| d.gorunur).map(|_| Eylem::Gizle)
}

/// Gösterirken `ShowWindow` komutu: simge durumundaysa `SW_RESTORE` (büyütülmüş
/// pencere büyütülmüş döner), değilse `SW_SHOW` (boyutu bozmaz).
pub fn goster_komutu(simge_durumunda: bool) -> i32 {
    if simge_durumunda {
        9 // SW_RESTORE
    } else {
        5 // SW_SHOW
    }
}

/// Çıkışta pencere kayıp kalmasın: gizliyse önce göster.
pub fn cikista_gostermeli_mi(pencere: Option<PencereDurumu>) -> bool {
    pencere.is_some_and(|d| !d.gorunur)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn iyi() -> Aday {
        Aday {
            surec_adi: "Logseq.exe".into(),
            sinif: "Chrome_WidgetWin_1".into(),
            sahipli: false,
            baslik_var: true,
            ex_stil: 0x0004_0100,
        }
    }

    #[test]
    fn dogru_aday_secilir() {
        assert!(ana_pencere_mi(&iyi()));
    }

    #[test]
    fn surec_adi_buyuk_kucuk_harfe_duyarsiz() {
        let mut a = iyi();
        a.surec_adi = "LOGSEQ.EXE".into();
        assert!(ana_pencere_mi(&a));
        a.surec_adi = "logseq.exe".into();
        assert!(ana_pencere_mi(&a));
    }

    #[test]
    fn baska_surec_elenir() {
        let mut a = iyi();
        a.surec_adi = "chrome.exe".into();
        assert!(!ana_pencere_mi(&a));
        a.surec_adi = "Logseq.exe.bak".into();
        assert!(!ana_pencere_mi(&a));
    }

    #[test]
    fn yanlis_sinif_elenir() {
        let mut a = iyi();
        a.sinif = "Chrome_RenderWidgetHostHWND".into();
        assert!(!ana_pencere_mi(&a));
    }

    #[test]
    fn sahipli_elenir() {
        let mut a = iyi();
        a.sahipli = true;
        assert!(!ana_pencere_mi(&a));
    }

    #[test]
    fn basliksiz_elenir() {
        let mut a = iyi();
        a.baslik_var = false;
        assert!(!ana_pencere_mi(&a));
    }

    #[test]
    fn arac_penceresi_elenir() {
        let mut a = iyi();
        a.ex_stil |= EX_ARAC_PENCERESI;
        assert!(!ana_pencere_mi(&a));
    }

    #[test]
    fn gorunur_uygun_gizliye_tercih_edilir() {
        let mut yardimci = iyi();
        yardimci.baslik_var = false;
        let adaylar = vec![(yardimci, true), (iyi(), false), (iyi(), true)];
        assert_eq!(sec(&adaylar), Some(2));
    }

    #[test]
    fn gorunur_yoksa_gizli_uygun_secilir() {
        let adaylar = vec![(iyi(), false)];
        assert_eq!(sec(&adaylar), Some(0));
    }

    #[test]
    fn uygun_yoksa_hicbiri() {
        let mut a = iyi();
        a.sahipli = true;
        assert_eq!(sec(&[(a, true)]), None);
        assert_eq!(sec(&[]), None);
    }

    const GORUNUR: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: false,
    };
    const GIZLI: PencereDurumu = PencereDurumu {
        gorunur: false,
        simge_durumunda: true,
    };
    const GIZLI_NORMAL: PencereDurumu = PencereDurumu {
        gorunur: false,
        simge_durumunda: false,
    };
    const GORUNUR_KUCUK: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: true,
    };

    #[test]
    fn tik_karar_tablosu() {
        assert_eq!(tikla_karari(None), Eylem::Baslat);
        assert_eq!(tikla_karari(Some(GORUNUR)), Eylem::Gizle);
        assert_eq!(tikla_karari(Some(GIZLI)), Eylem::Goster);
        assert_eq!(tikla_karari(Some(GIZLI_NORMAL)), Eylem::Goster);
        assert_eq!(tikla_karari(Some(GORUNUR_KUCUK)), Eylem::Goster);
    }

    #[test]
    fn menu_kararlari() {
        assert_eq!(goster_karari(None), Eylem::Baslat);
        assert_eq!(goster_karari(Some(GORUNUR)), Eylem::Goster);
        assert_eq!(gizle_karari(None), None);
        assert_eq!(gizle_karari(Some(GIZLI)), None);
        assert_eq!(gizle_karari(Some(GORUNUR)), Some(Eylem::Gizle));
    }

    #[test]
    fn goster_komutu_boyutu_bozmaz() {
        assert_eq!(goster_komutu(true), 9);
        assert_eq!(goster_komutu(false), 5);
    }

    #[test]
    fn cikista_yalniz_gizli_gosterilir() {
        assert!(!cikista_gostermeli_mi(None));
        assert!(!cikista_gostermeli_mi(Some(GORUNUR)));
        assert!(cikista_gostermeli_mi(Some(GIZLI)));
        assert!(cikista_gostermeli_mi(Some(GIZLI_NORMAL)));
    }
}
