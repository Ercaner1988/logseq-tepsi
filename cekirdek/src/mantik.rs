//! Win32'den bağımsız saf mantık: pencere seçimi, göster/gizle karar tabloları, menü ve
//! tepsi iletisi çözümü.

use crate::gizlenen::Gizlenenler;

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

/// Gizlenenler kümesinde kalma kuralı: pencere hâlâ var ve görünür değil. Başkası göstermişse
/// (Logseq yeniden açıldı, başka işlem) küme bayat kalmasın.
pub fn kumede_kalmali_mi(durum: Option<PencereDurumu>) -> bool {
    durum.is_some_and(|d| !d.gorunur)
}

/// Sınıf adı Logseq (Electron) üst düzey penceresine uyuyor mu? Süreç yolunu okumadan
/// önce ucuz eleme için (EnumWindows yüzlerce pencere gezer).
pub fn sinif_uygun_mu(sinif: &str) -> bool {
    sinif.eq_ignore_ascii_case(SINIF_ADI)
}

/// Küme kararı (tepsi tıkı, menü, yerel uç): hangi pencereler üzerinde ne yapılır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Toplu {
    Baslat,
    /// Alttaki pencere önce, en üstteki en son gizlenir (o "en son gizlenen" olur).
    Gizle(Vec<isize>),
    Goster(isize),
}

/// Göster hedefi: en son gizlenen, yoksa görünür olan, yoksa ilk ana pencere.
fn goster_hedefi(p: &[(isize, PencereDurumu)], g: &Gizlenenler) -> Option<isize> {
    let var = |h: &isize| p.iter().any(|(k, _)| k == h);
    g.hepsi()
        .iter()
        .rev()
        .copied()
        .find(var)
        .or_else(|| p.iter().find(|(_, d)| d.gorunur).map(|(h, _)| *h))
        .or_else(|| p.first().map(|(h, _)| *h))
}

/// Tepsi simgesine sol tık: görünür (küçük olmayan) pencere varsa hepsini gizle; yoksa
/// en son gizleneni göster; ana pencere hiç yoksa Logseq'i başlat.
pub fn tikla_toplu(p: &[(isize, PencereDurumu)], g: &Gizlenenler) -> Toplu {
    let gizlenecek: Vec<isize> = p
        .iter()
        .rev()
        .filter(|(_, d)| tikla_karari(Some(*d)) == Eylem::Gizle)
        .map(|(h, _)| *h)
        .collect();
    if !gizlenecek.is_empty() {
        return Toplu::Gizle(gizlenecek);
    }
    goster_toplu(p, g)
}

/// "Göster" menü öğesi: pencere yoksa başlat, varsa tek pencere göster.
pub fn goster_toplu(p: &[(isize, PencereDurumu)], g: &Gizlenenler) -> Toplu {
    match (
        goster_karari(p.first().map(|(_, d)| *d)),
        goster_hedefi(p, g),
    ) {
        (Eylem::Goster, Some(h)) => Toplu::Goster(h),
        _ => Toplu::Baslat,
    }
}

/// "Tepsiye gizle" (menü, yerel uç): görünür tüm ana pencereler; alttaki önce.
pub fn gizlenecekler(p: &[(isize, PencereDurumu)]) -> Vec<isize> {
    p.iter()
        .rev()
        .filter(|(_, d)| gizle_karari(Some(*d)).is_some())
        .map(|(h, _)| *h)
        .collect()
}

/// "Hepsini göster", çıkış, oturum kapanışı, yerel uç: görünür olmayan tüm ana pencereler.
/// Önce eski gizlenenler, kümede olmayanlar en başta; en son gizlenen en son (öne gelir).
pub fn gosterilecekler(p: &[(isize, PencereDurumu)], g: &Gizlenenler) -> Vec<isize> {
    let gizli = |h: isize| {
        p.iter()
            .any(|(k, d)| *k == h && cikista_gostermeli_mi(Some(*d)))
    };
    let mut sonuc: Vec<isize> = p
        .iter()
        .filter(|(k, d)| cikista_gostermeli_mi(Some(*d)) && !g.hepsi().contains(k))
        .map(|(k, _)| *k)
        .collect();
    sonuc.extend(g.hepsi().iter().copied().filter(|h| gizli(*h)));
    sonuc
}

/// Açılış taraması kararı (araç Logseq açıkken başladığında).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcilisEylemi {
    Yok,
    /// Küçük ve görünür: gizle ve hatırla.
    Gizle,
    /// Zaten gizli: yalnız hatırla (tepsiden geri getirilebilsin).
    Hatirla,
}

pub fn acilis_karari(d: PencereDurumu) -> AcilisEylemi {
    if !d.gorunur {
        AcilisEylemi::Hatirla
    } else if d.simge_durumunda {
        AcilisEylemi::Gizle
    } else {
        AcilisEylemi::Yok
    }
}

/// Küçültme olayı kararı: olay `IsIconic` tazelenmeden önce gelebilir, bir kez daha bak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KucultmeKarari {
    Gizle,
    Tekrar,
    Birak,
}

pub const KUCULTME_YENIDEN_DENEME: u32 = 1;
pub const KUCULTME_GECIKME_MS: u32 = 120;

pub fn kucultme_karari(simge_durumunda: bool, deneme: u32) -> KucultmeKarari {
    if simge_durumunda {
        KucultmeKarari::Gizle
    } else if deneme < KUCULTME_YENIDEN_DENEME {
        KucultmeKarari::Tekrar
    } else {
        KucultmeKarari::Birak
    }
}

/// Kanca/simge kurulumu başarısızsa yeniden deneme gecikmesi (ms); en çok 30 sn.
pub fn yeniden_gecikmesi_ms(deneme: u32) -> u32 {
    const ADIMLAR: [u32; 5] = [1_000, 2_000, 5_000, 10_000, 30_000];
    ADIMLAR[(deneme as usize).min(ADIMLAR.len() - 1)]
}

/// Tepsi ipucu: küçültme izlenemiyorsa kullanıcı görsün.
pub fn ipucu(kanca_tamam: bool) -> &'static str {
    if kanca_tamam {
        "Logseq"
    } else {
        "Logseq (küçültme izlenemiyor)"
    }
}

pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONUP: u32 = 0x0205;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TepsiOlayi {
    SolTik,
    SagTik,
    Yok,
}

/// Tepsi geri çağrı iletisinin `lParam` çözümü. Simge sürümü 0 (`NIM_SETVERSION` yok):
/// `lParam` doğrudan fare iletisidir. Sürüm 4'e geçilirse (`LOWORD` olay, `HIWORD` kimlik;
/// `NIN_SELECT`/`WM_CONTEXTMENU`) yalnız bu işlev ve sınamaları birlikte değişmeli.
pub fn tepsi_olayi(lparam: isize) -> TepsiOlayi {
    match (lparam & 0xFFFF) as u32 {
        WM_LBUTTONUP => TepsiOlayi::SolTik,
        WM_RBUTTONUP => TepsiOlayi::SagTik,
        _ => TepsiOlayi::Yok,
    }
}

/// Çift tık iki `WM_LBUTTONUP` üretir (gizle sonra göster olmasın): eşik içindeki
/// ikinci tık yok sayılır.
pub fn cift_tik_mi(simdi_ms: u64, onceki_ms: Option<u64>, esik_ms: u64) -> bool {
    onceki_ms.is_some_and(|o| simdi_ms.saturating_sub(o) < esik_ms)
}

pub const KIMLIK_GOSTER: usize = 1;
pub const KIMLIK_GIZLE: usize = 2;
pub const KIMLIK_CIKIS: usize = 3;
pub const KIMLIK_ARACI_KAPAT: usize = 4;
pub const KIMLIK_HEPSINI_GOSTER: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEylemi {
    Goster,
    Gizle,
    HepsiniGoster,
    Cikis,
    AraciKapat,
}

/// Menü kimliği → eylem (`TrackPopupMenu` 0 ya da bilinmeyen döndürürse eylem yok).
pub fn menu_eylemi(kimlik: usize) -> Option<MenuEylemi> {
    match kimlik {
        KIMLIK_GOSTER => Some(MenuEylemi::Goster),
        KIMLIK_GIZLE => Some(MenuEylemi::Gizle),
        KIMLIK_HEPSINI_GOSTER => Some(MenuEylemi::HepsiniGoster),
        KIMLIK_CIKIS => Some(MenuEylemi::Cikis),
        KIMLIK_ARACI_KAPAT => Some(MenuEylemi::AraciKapat),
        _ => None,
    }
}

/// Menü öğeleri sırasıyla: (kimlik, etiket). Ayırıcı `None` ile işaretlenir.
pub const MENU: [Option<(usize, &str)>; 7] = [
    Some((KIMLIK_GOSTER, "Göster")),
    Some((KIMLIK_HEPSINI_GOSTER, "Hepsini göster")),
    Some((KIMLIK_GIZLE, "Tepsiye gizle")),
    None,
    Some((KIMLIK_CIKIS, "Çıkış")),
    Some((KIMLIK_ARACI_KAPAT, "Tepsi aracını kapat")),
    None,
];

/// "Hepsini göster" yalnız tepside gizli pencere varken etkin.
pub fn menu_etkin_mi(kimlik: usize, gizli_sayisi: usize) -> bool {
    kimlik != KIMLIK_HEPSINI_GOSTER || gizli_sayisi > 0
}

/// `CreateProcess` bayrakları: ayrık süreç (DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP),
/// izin varsa iş nesnesinden de kopuk (CREATE_BREAKAWAY_FROM_JOB).
pub fn baslat_bayraklari(breakaway: bool) -> u32 {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    let temel = DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP;
    if breakaway {
        temel | CREATE_BREAKAWAY_FROM_JOB
    } else {
        temel
    }
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

#[cfg(test)]
mod toplu_testler {
    use super::*;

    const GORUNUR: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: false,
    };
    const GIZLI: PencereDurumu = PencereDurumu {
        gorunur: false,
        simge_durumunda: true,
    };
    const GORUNUR_KUCUK: PencereDurumu = PencereDurumu {
        gorunur: true,
        simge_durumunda: true,
    };

    fn kume(ids: &[isize]) -> Gizlenenler {
        let mut g = Gizlenenler::yeni();
        for &i in ids {
            g.ekle(i);
        }
        g
    }

    #[test]
    fn kumede_yalniz_var_olan_gizli_pencere_kalir() {
        assert!(!kumede_kalmali_mi(None));
        assert!(!kumede_kalmali_mi(Some(GORUNUR)));
        assert!(!kumede_kalmali_mi(Some(GORUNUR_KUCUK)));
        assert!(kumede_kalmali_mi(Some(GIZLI)));
    }

    #[test]
    fn sinif_on_eleme() {
        assert!(sinif_uygun_mu("Chrome_WidgetWin_1"));
        assert!(sinif_uygun_mu("chrome_widgetwin_1"));
        assert!(!sinif_uygun_mu("Chrome_WidgetWin_0"));
        assert!(!sinif_uygun_mu("Chrome_RenderWidgetHostHWND"));
    }

    #[test]
    fn tik_pencere_yoksa_baslatir() {
        assert_eq!(tikla_toplu(&[], &kume(&[])), Toplu::Baslat);
        assert_eq!(goster_toplu(&[], &kume(&[])), Toplu::Baslat);
    }

    #[test]
    fn tik_gorunur_pencerelerin_hepsini_gizler_alttaki_once() {
        // Z sırası: 1 en üstte, 3 en altta.
        let p = [(1, GORUNUR), (2, GIZLI), (3, GORUNUR)];
        assert_eq!(tikla_toplu(&p, &kume(&[2])), Toplu::Gizle(vec![3, 1]));
    }

    #[test]
    fn a_gizli_b_gorunur_b_gizlenince_a_kaybolmaz() {
        // A(1) önceden gizlendi; B(2) görünür. Tık B'yi gizler.
        let p = [(1, GIZLI), (2, GORUNUR)];
        let mut g = kume(&[1]);
        assert_eq!(tikla_toplu(&p, &g), Toplu::Gizle(vec![2]));
        g.ekle(2);
        // Şimdi ikisi de gizli: tık en son gizlenen B'yi (A'yı değil) gösterir.
        let p = [(1, GIZLI), (2, GIZLI)];
        assert_eq!(tikla_toplu(&p, &g), Toplu::Goster(2));
        g.cikar(2);
        // B göründükten sonra tık B'yi tekrar gizler; A hâlâ kümede.
        let p = [(1, GIZLI), (2, GORUNUR)];
        assert_eq!(tikla_toplu(&p, &g), Toplu::Gizle(vec![2]));
        assert_eq!(g.hepsi(), &[1]);
    }

    #[test]
    fn tik_kumede_olmayan_gizliyi_de_gosterir() {
        // Gizli ama kümede değil (başkası gizlemiş): görünür yoksa ilk pencere gösterilir.
        let p = [(7, GIZLI)];
        assert_eq!(tikla_toplu(&p, &kume(&[])), Toplu::Goster(7));
    }

    #[test]
    fn tik_gorunur_ama_kucuk_pencereyi_geri_yukler() {
        let p = [(4, GORUNUR_KUCUK)];
        assert_eq!(tikla_toplu(&p, &kume(&[])), Toplu::Goster(4));
    }

    #[test]
    fn goster_hedefi_bayat_kimligi_secmez() {
        // Kümede 9 var ama artık ana pencere değil (listede yok): 1 seçilir.
        let p = [(1, GIZLI)];
        assert_eq!(goster_toplu(&p, &kume(&[9])), Toplu::Goster(1));
    }

    #[test]
    fn gizlenecekler_yalniz_gorunurler() {
        let p = [(1, GORUNUR), (2, GIZLI), (3, GORUNUR_KUCUK)];
        assert_eq!(gizlenecekler(&p), vec![3, 1]);
        assert!(gizlenecekler(&[(2, GIZLI)]).is_empty());
    }

    #[test]
    fn gosterilecekler_eski_gizlenen_once_son_gizlenen_son() {
        let p = [(1, GIZLI), (2, GIZLI), (3, GORUNUR), (4, GIZLI)];
        // 4 kümede yok (başkası gizlemiş): en başta. Kümede 2 sonra 1 gizlenmiş.
        assert_eq!(gosterilecekler(&p, &kume(&[2, 1])), vec![4, 2, 1]);
        assert!(gosterilecekler(&[(3, GORUNUR)], &kume(&[])).is_empty());
    }

    #[test]
    fn acilis_taramasi_karari() {
        assert_eq!(acilis_karari(GIZLI), AcilisEylemi::Hatirla);
        assert_eq!(
            acilis_karari(PencereDurumu {
                gorunur: false,
                simge_durumunda: false
            }),
            AcilisEylemi::Hatirla
        );
        assert_eq!(acilis_karari(GORUNUR_KUCUK), AcilisEylemi::Gizle);
        assert_eq!(acilis_karari(GORUNUR), AcilisEylemi::Yok);
    }

    #[test]
    fn kucultme_yarisi_bir_kez_yeniden_denenir() {
        assert_eq!(kucultme_karari(true, 0), KucultmeKarari::Gizle);
        assert_eq!(kucultme_karari(false, 0), KucultmeKarari::Tekrar);
        assert_eq!(kucultme_karari(true, 1), KucultmeKarari::Gizle);
        assert_eq!(kucultme_karari(false, 1), KucultmeKarari::Birak);
    }

    #[test]
    fn yeniden_deneme_gecikmesi_artar_ve_tavanda_kalir() {
        let g: Vec<u32> = (0..8).map(yeniden_gecikmesi_ms).collect();
        assert_eq!(
            g,
            vec![1_000, 2_000, 5_000, 10_000, 30_000, 30_000, 30_000, 30_000]
        );
        assert_eq!(yeniden_gecikmesi_ms(u32::MAX), 30_000);
    }

    #[test]
    fn ipucu_kanca_sorununu_bildirir() {
        assert_eq!(ipucu(true), "Logseq");
        assert!(ipucu(false).contains("izlenemiyor"));
        assert!(ipucu(false).encode_utf16().count() < 127);
    }

    #[test]
    fn tepsi_lparam_cozumu() {
        assert_eq!(tepsi_olayi(WM_LBUTTONUP as isize), TepsiOlayi::SolTik);
        assert_eq!(tepsi_olayi(WM_RBUTTONUP as isize), TepsiOlayi::SagTik);
        // Yalnız düşük sözcük olaydır (yüksek sözcük başka bilgi taşıyabilir).
        assert_eq!(tepsi_olayi(0x0001_0000 | 0x0202), TepsiOlayi::SolTik);
        // Basma ve çift tık iletileri yok sayılır (yalnız bırakma).
        assert_eq!(tepsi_olayi(0x0201), TepsiOlayi::Yok);
        assert_eq!(tepsi_olayi(0x0203), TepsiOlayi::Yok);
        assert_eq!(tepsi_olayi(0x0204), TepsiOlayi::Yok);
        assert_eq!(tepsi_olayi(0), TepsiOlayi::Yok);
    }

    #[test]
    fn cift_tik_esik_icinde_yok_sayilir() {
        assert!(!cift_tik_mi(1000, None, 500));
        assert!(cift_tik_mi(1100, Some(1000), 500));
        assert!(!cift_tik_mi(1500, Some(1000), 500));
        assert!(!cift_tik_mi(1501, Some(1000), 500));
        // Saat geriye giderse (aynı değer) sıfır fark: yok sayılır, taşma yok.
        assert!(cift_tik_mi(900, Some(1000), 500));
    }

    #[test]
    fn menu_kimligi_eyleme_cozulur() {
        assert_eq!(menu_eylemi(KIMLIK_GOSTER), Some(MenuEylemi::Goster));
        assert_eq!(menu_eylemi(KIMLIK_GIZLE), Some(MenuEylemi::Gizle));
        assert_eq!(
            menu_eylemi(KIMLIK_HEPSINI_GOSTER),
            Some(MenuEylemi::HepsiniGoster)
        );
        assert_eq!(menu_eylemi(KIMLIK_CIKIS), Some(MenuEylemi::Cikis));
        assert_eq!(
            menu_eylemi(KIMLIK_ARACI_KAPAT),
            Some(MenuEylemi::AraciKapat)
        );
        assert_eq!(menu_eylemi(0), None);
        assert_eq!(menu_eylemi(99), None);
    }

    #[test]
    fn menude_her_eylemin_tek_ogesi_var() {
        let kimlikler: Vec<usize> = MENU.iter().flatten().map(|(k, _)| *k).collect();
        for k in 1..=5 {
            assert_eq!(
                kimlikler.iter().filter(|&&x| x == k).count(),
                1,
                "kimlik {k}"
            );
            assert!(menu_eylemi(k).is_some());
        }
        assert_eq!(kimlikler.len(), 5);
    }

    #[test]
    fn hepsini_goster_yalniz_gizli_varken_etkin() {
        assert!(!menu_etkin_mi(KIMLIK_HEPSINI_GOSTER, 0));
        assert!(menu_etkin_mi(KIMLIK_HEPSINI_GOSTER, 2));
        assert!(menu_etkin_mi(KIMLIK_CIKIS, 0));
        assert!(menu_etkin_mi(KIMLIK_GOSTER, 0));
    }

    #[test]
    fn baslat_bayraklari_ayrik_surec() {
        let b = baslat_bayraklari(true);
        assert_eq!(b & 0x8, 0x8, "DETACHED_PROCESS");
        assert_eq!(b & 0x200, 0x200, "CREATE_NEW_PROCESS_GROUP");
        assert_eq!(b & 0x0100_0000, 0x0100_0000, "CREATE_BREAKAWAY_FROM_JOB");
        let y = baslat_bayraklari(false);
        assert_eq!(y & 0x0100_0000, 0);
        assert_eq!(y & 0x208, 0x208);
    }
}
