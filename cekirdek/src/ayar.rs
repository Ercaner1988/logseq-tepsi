//! Kullanıcı ayarları (saf yapı + dosya): tek kaynak `%LOCALAPPDATA%\logseq-tepsi\ayar.toml`.
//! Aynı anahtar/değer biçimi yerel uçta (`GET/POST /ayar`) ve dosyada kullanılır. Win32 yok;
//! sınamalar geçici dizin kullanır. `baslangicta_calis` dosyada tutulmaz (kısayolun varlığı
//! gerçeğin kendisidir), yalnız uçtan okunur/yazılır.

use crate::kisayol;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const DOSYA_ADI: &str = "ayar.toml";
pub const VARSAYILAN_KISAYOL: &str = "Ctrl+Alt+L";
pub const VARSAYILAN_KISAYOL_NOT: &str = "Ctrl+Alt+N";
/// Dosyada tutulan anahtarlar (`baslangicta_calis` ve `api_otomatik_baslat` yok; `api_bekleyen`
/// uca açık değil, yalnız dosyada).
const DOSYA_ANAHTARLARI: [&str; 9] = [
    "kucultme",
    "tik",
    "kapaliyken_tik_acar",
    "kisayol_goster_gizle",
    "oturumda_gizli_baslat",
    "kisayol_not",
    "not_hedefi",
    "kalinan_sayfa",
    "api_bekleyen",
];
/// `satirlar`ın (sözleşmedeki eski GET gövdesi) kullandığı ilk dosya anahtarı sayısı.
const ESKI_CIFT_SAYISI: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kucultme {
    Tepsi,
    GorevCubugu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tik {
    Tek,
    Cift,
}

/// Anında not nereye yazılır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotHedefi {
    Gunluk,
    YeniSayfa,
}

impl NotHedefi {
    /// Dosyadaki, uçtaki ve eklentiye giden payload adı.
    pub fn ad(self) -> &'static str {
        match self {
            Self::Gunluk => "gunluk",
            Self::YeniSayfa => "yeni_sayfa",
        }
    }
}

/// Dosyada tutulan ayarlar. Anahtar adları dosyada ve uçta aynıdır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ayarlar {
    pub kucultme: Kucultme,
    pub tik: Tik,
    pub kapaliyken_tik_acar: bool,
    /// Kurallı yazım (`kisayol::bicimle`); boş = kapalı.
    pub kisayol_goster_gizle: String,
    pub oturumda_gizli_baslat: bool,
    /// Anında not genel kısayolu (kurallı yazım); boş = kapalı.
    pub kisayol_not: String,
    pub not_hedefi: NotHedefi,
    /// Eklenti, Logseq açılışında son bakılan sayfaya dönsün mü (eklentiyle eşitlenir).
    pub kalinan_sayfa: bool,
    /// `api_otomatik_baslat` isteği Logseq açıkken verildi: kapanınca `configs.edn`e yazılır.
    /// Uçtan değiştirilemez (yalnız `ayar.toml`da).
    pub api_bekleyen: Option<bool>,
}

impl Default for Ayarlar {
    fn default() -> Self {
        Self {
            kucultme: Kucultme::Tepsi,
            tik: Tik::Tek,
            kapaliyken_tik_acar: true,
            kisayol_goster_gizle: VARSAYILAN_KISAYOL.to_owned(),
            oturumda_gizli_baslat: false,
            kisayol_not: VARSAYILAN_KISAYOL_NOT.to_owned(),
            not_hedefi: NotHedefi::Gunluk,
            kalinan_sayfa: true,
            api_bekleyen: None,
        }
    }
}

/// Tek bir ayar değişikliği (menüden ya da `POST /ayar`tan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AyarDegeri {
    Kucultme(Kucultme),
    Tik(Tik),
    KapaliykenTikAcar(bool),
    KisayolGosterGizle(String),
    OturumdaGizliBaslat(bool),
    /// Dosyada tutulmaz: başlangıç kısayolunun varlığı.
    BaslangictaCalis(bool),
    KisayolNot(String),
    NotHedefi(NotHedefi),
    KalinanSayfa(bool),
    /// Dosyada tutulmaz: gerçek kaynak Logseq'in `configs.edn` `:server/autostart` değeri.
    ApiOtomatikBaslat(bool),
}

fn ev(b: bool) -> &'static str {
    if b {
        "evet"
    } else {
        "hayir"
    }
}

/// `deger`, `a` ya da `b` (ASCII büyük/küçük harf duyarsız): `a` ise `true`.
fn iki_secenek(anahtar: &str, deger: &str, a: &str, b: &str) -> Result<bool, String> {
    if deger.eq_ignore_ascii_case(a) {
        Ok(true)
    } else if deger.eq_ignore_ascii_case(b) {
        Ok(false)
    } else {
        Err(format!("{anahtar}: {a} ya da {b} olmalı: {deger}"))
    }
}

/// Boş (kapalı) ya da geçerli kısayol; kurallı yazıma çevrilir.
fn kisayol_degeri(anahtar: &str, deger: &str) -> Result<String, String> {
    if deger.is_empty() {
        return Ok(String::new());
    }
    let k = kisayol::ayristir(deger).map_err(|e| format!("{anahtar}: {e}"))?;
    Ok(kisayol::bicimle(&k))
}

impl AyarDegeri {
    /// `anahtar=değer` çiftini doğrular.
    pub fn coz(anahtar: &str, deger: &str) -> Result<Self, String> {
        let evet_hayir = || iki_secenek(anahtar, deger, "evet", "hayir");
        Ok(match anahtar {
            "kucultme" => {
                Self::Kucultme(if iki_secenek(anahtar, deger, "tepsi", "gorev_cubugu")? {
                    Kucultme::Tepsi
                } else {
                    Kucultme::GorevCubugu
                })
            }
            "tik" => Self::Tik(if iki_secenek(anahtar, deger, "tek", "cift")? {
                Tik::Tek
            } else {
                Tik::Cift
            }),
            "kapaliyken_tik_acar" => Self::KapaliykenTikAcar(evet_hayir()?),
            "kisayol_goster_gizle" => Self::KisayolGosterGizle(kisayol_degeri(anahtar, deger)?),
            "oturumda_gizli_baslat" => Self::OturumdaGizliBaslat(evet_hayir()?),
            "baslangicta_calis" => Self::BaslangictaCalis(evet_hayir()?),
            "kisayol_not" => Self::KisayolNot(kisayol_degeri(anahtar, deger)?),
            "not_hedefi" => {
                Self::NotHedefi(if iki_secenek(anahtar, deger, "gunluk", "yeni_sayfa")? {
                    NotHedefi::Gunluk
                } else {
                    NotHedefi::YeniSayfa
                })
            }
            "kalinan_sayfa" => Self::KalinanSayfa(evet_hayir()?),
            "api_otomatik_baslat" => Self::ApiOtomatikBaslat(evet_hayir()?),
            _ => return Err(format!("bilinmeyen anahtar: {anahtar}")),
        })
    }
}

/// `POST /ayar` gövdesi: bir ya da birkaç `anahtar=değer` satırı. Biri geçersizse hepsi
/// reddedilir (kısmi uygulama yok).
pub fn satirlari_coz(govde: &str) -> Result<Vec<AyarDegeri>, String> {
    let mut degerler = Vec::new();
    for satir in govde.lines().map(str::trim).filter(|s| !s.is_empty()) {
        let (anahtar, deger) = satir
            .split_once('=')
            .ok_or_else(|| format!("satırda '=' yok: {satir}"))?;
        degerler.push(AyarDegeri::coz(anahtar.trim(), deger.trim())?);
    }
    if degerler.is_empty() {
        return Err("ayar satırı yok".into());
    }
    Ok(degerler)
}

impl Ayarlar {
    /// Değişikliği uygular; `BaslangictaCalis` dosyada tutulmadığı için yok sayılır.
    pub fn uygula(&mut self, d: &AyarDegeri) {
        match d {
            AyarDegeri::Kucultme(k) => self.kucultme = *k,
            AyarDegeri::Tik(t) => self.tik = *t,
            AyarDegeri::KapaliykenTikAcar(b) => self.kapaliyken_tik_acar = *b,
            AyarDegeri::KisayolGosterGizle(k) => self.kisayol_goster_gizle.clone_from(k),
            AyarDegeri::OturumdaGizliBaslat(b) => self.oturumda_gizli_baslat = *b,
            AyarDegeri::KisayolNot(k) => self.kisayol_not.clone_from(k),
            AyarDegeri::NotHedefi(h) => self.not_hedefi = *h,
            AyarDegeri::KalinanSayfa(b) => self.kalinan_sayfa = *b,
            AyarDegeri::BaslangictaCalis(_) | AyarDegeri::ApiOtomatikBaslat(_) => {}
        }
    }

    /// Dosya biçimindeki (anahtar, değer) çiftleri, sabit sırayla (`api_bekleyen` hariç).
    fn cifler(&self) -> [(&'static str, String); 8] {
        let kucultme = match self.kucultme {
            Kucultme::Tepsi => "tepsi",
            Kucultme::GorevCubugu => "gorev_cubugu",
        };
        let tik = match self.tik {
            Tik::Tek => "tek",
            Tik::Cift => "cift",
        };
        [
            ("kucultme", kucultme.into()),
            ("tik", tik.into()),
            ("kapaliyken_tik_acar", ev(self.kapaliyken_tik_acar).into()),
            ("kisayol_goster_gizle", self.kisayol_goster_gizle.clone()),
            (
                "oturumda_gizli_baslat",
                ev(self.oturumda_gizli_baslat).into(),
            ),
            ("kisayol_not", self.kisayol_not.clone()),
            ("not_hedefi", self.not_hedefi.ad().into()),
            ("kalinan_sayfa", ev(self.kalinan_sayfa).into()),
        ]
    }

    /// `GET /ayar` gövdesi: düz metin `anahtar=değer` satırları; `baslangicta_calis` kısayolun
    /// o anki varlığıdır (dosyada değil).
    pub fn satirlar(&self, baslangicta: bool) -> String {
        let mut s: String = self
            .cifler()
            .iter()
            .take(ESKI_CIFT_SAYISI)
            .map(|(a, d)| format!("{a}={d}\n"))
            .collect();
        s.push_str(&format!("baslangicta_calis={}\n", ev(baslangicta)));
        s
    }

    /// Tam `GET /ayar` gövdesi: `satirlar` (sözleşmedeki ilk altı satır, sırası sabit) + anında
    /// not ve kalınan sayfa + API sunucusu. `api_gercek`: `configs.edn`deki `:server/autostart`.
    /// `api_otomatik_baslat` istenen değeri gösterir (bekleyen varsa o); `api_otomatik_bekleyen`
    /// bekleyen varsa `evet`/`hayir`, yoksa boştur (Logseq kapanınca uygulanır).
    pub fn satirlar_tam(&self, baslangicta: bool, api_gercek: bool) -> String {
        let mut s = self.satirlar(baslangicta);
        for (a, d) in self.cifler().iter().skip(ESKI_CIFT_SAYISI) {
            s.push_str(&format!("{a}={d}\n"));
        }
        s.push_str(&format!(
            "api_otomatik_baslat={}\napi_otomatik_bekleyen={}\n",
            ev(self.api_bekleyen.unwrap_or(api_gercek)),
            self.api_bekleyen.map_or("", ev)
        ));
        s
    }

    /// `ayar.toml` içeriği (değerler TOML dizgesi).
    pub fn toml_metni(&self) -> String {
        let mut s =
            String::from("# logseq-tepsi ayarları (tepsi menüsünden ya da eklentiden değişir)\n");
        for (a, d) in self.cifler() {
            s.push_str(&format!("{a} = {}\n", toml::Value::String(d)));
        }
        if let Some(b) = self.api_bekleyen {
            let b = toml::Value::String(ev(b).into());
            s.push_str(&format!("api_bekleyen = {b}\n"));
        }
        s
    }

    /// `ayar.toml` ayrıştırması: (ayarlar, uyarılar). Eksik anahtar varsayılan kalır; bilinmeyen
    /// anahtar uyarıyla atlanır; sözdizimi ya da değer hatası `Err` (dosya bozuk sayılır).
    pub fn toml_coz(metin: &str) -> Result<(Self, Vec<String>), String> {
        let tablo: toml::Table = metin.parse().map_err(|e| format!("TOML hatası: {e}"))?;
        let mut ayarlar = Self::default();
        let mut uyarilar = Vec::new();
        for (anahtar, deger) in &tablo {
            if !DOSYA_ANAHTARLARI.contains(&anahtar.as_str()) {
                uyarilar.push(format!(
                    "ayar dosyasında bilinmeyen anahtar atlandı: {anahtar}"
                ));
                continue;
            }
            let metin = match deger {
                toml::Value::String(s) => s.clone(),
                toml::Value::Boolean(b) => ev(*b).to_owned(),
                _ => return Err(format!("{anahtar}: dizge olmalı")),
            };
            if anahtar == "api_bekleyen" {
                ayarlar.api_bekleyen = Some(iki_secenek(anahtar, &metin, "evet", "hayir")?);
                continue;
            }
            ayarlar.uygula(&AyarDegeri::coz(anahtar, &metin)?);
        }
        Ok((ayarlar, uyarilar))
    }
}

/// Dosyadan yükleme sonucu.
#[derive(Debug)]
pub struct Yukleme {
    pub ayarlar: Ayarlar,
    /// Günlüğe yazılacak satırlar.
    pub kayitlar: Vec<String>,
    /// `false`: bozuk dosya kenara alınamadı; üstüne yazılmamalı.
    pub yazilabilir: bool,
}

fn yan_yol(dizin: &Path, ek: &str) -> PathBuf {
    dizin.join(format!("{DOSYA_ADI}{ek}"))
}

/// `dizin\ayar.toml` okur. Yok: varsayılanlar. Bozuk: `ayar.toml.bozuk` adıyla kenara alınır,
/// varsayılanlarla çalışılır (kenara alınamazsa `yazilabilir = false`).
pub fn yukle(dizin: &Path) -> Yukleme {
    let dosya = dizin.join(DOSYA_ADI);
    let mut y = Yukleme {
        ayarlar: Ayarlar::default(),
        kayitlar: Vec::new(),
        yazilabilir: true,
    };
    let sonuc = match std::fs::read_to_string(&dosya) {
        Ok(metin) => Ayarlar::toml_coz(&metin),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            y.kayitlar
                .push("ayar dosyası yok; varsayılanlarla çalışılıyor".into());
            return y;
        }
        Err(e) => Err(format!("okunamadı: {e}")),
    };
    match sonuc {
        Ok((ayarlar, uyarilar)) => {
            y.ayarlar = ayarlar;
            y.kayitlar.extend(uyarilar);
        }
        Err(neden) => {
            let bozuk = yan_yol(dizin, ".bozuk");
            match std::fs::rename(&dosya, &bozuk) {
                Ok(()) => y.kayitlar.push(format!(
                    "ayar dosyası bozuk ({neden}); {} adıyla kenara alındı, varsayılanlarla çalışılıyor",
                    bozuk.display()
                )),
                Err(e) => {
                    y.yazilabilir = false;
                    y.kayitlar.push(format!(
                        "ayar dosyası bozuk ({neden}) ve kenara alınamadı ({e}); üstüne yazılmayacak, varsayılanlarla çalışılıyor"
                    ));
                }
            }
        }
    }
    y
}

/// Atomik yazma: geçici dosyaya yaz, diske işle, `ayar.toml` üzerine yeniden adlandır.
pub fn kaydet(dizin: &Path, ayarlar: &Ayarlar) -> std::io::Result<()> {
    std::fs::create_dir_all(dizin)?;
    let gecici = yan_yol(dizin, ".tmp");
    let yaz = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&gecici)?;
        f.write_all(ayarlar.toml_metni().as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&gecici, dizin.join(DOSYA_ADI))
    };
    let sonuc = yaz();
    if sonuc.is_err() {
        let _ = std::fs::remove_file(&gecici);
    }
    sonuc
}

static GUNCEL: Mutex<Option<Ayarlar>> = Mutex::new(None);

/// Geçerli ayarlar (süreç genelinde; yazan ileti döngüsü, okuyan sunucu iş parçacığı da olur).
pub fn guncel() -> Ayarlar {
    GUNCEL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

pub fn ayarla(a: Ayarlar) {
    *GUNCEL.lock().unwrap_or_else(|e| e.into_inner()) = Some(a);
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("logseq-tepsi-ayar-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn varsayilanlar_belirtilen_gibi() {
        let a = Ayarlar::default();
        assert_eq!(a.kucultme, Kucultme::Tepsi);
        assert_eq!(a.tik, Tik::Tek);
        assert!(a.kapaliyken_tik_acar);
        assert_eq!(a.kisayol_goster_gizle, "Ctrl+Alt+L");
        assert!(!a.oturumda_gizli_baslat);
    }

    #[test]
    fn get_ayar_govdesi_sabit_sirali_duz_metin() {
        assert_eq!(
            Ayarlar::default().satirlar(false),
            "kucultme=tepsi\ntik=tek\nkapaliyken_tik_acar=evet\nkisayol_goster_gizle=Ctrl+Alt+L\n\
             oturumda_gizli_baslat=hayir\nbaslangicta_calis=hayir\n"
        );
        let a = Ayarlar {
            kucultme: Kucultme::GorevCubugu,
            tik: Tik::Cift,
            kapaliyken_tik_acar: false,
            kisayol_goster_gizle: String::new(),
            oturumda_gizli_baslat: true,
            ..Ayarlar::default()
        };
        let s = a.satirlar(true);
        assert!(s.contains("kucultme=gorev_cubugu\ntik=cift\nkapaliyken_tik_acar=hayir\n"));
        assert!(s.contains("kisayol_goster_gizle=\n"), "boş = kapalı: {s}");
        assert!(s.ends_with("oturumda_gizli_baslat=evet\nbaslangicta_calis=evet\n"));
    }

    #[test]
    fn toml_gidis_donus_ve_kisayol_kurali() {
        let a = Ayarlar {
            kucultme: Kucultme::GorevCubugu,
            tik: Tik::Cift,
            kapaliyken_tik_acar: false,
            kisayol_goster_gizle: "Ctrl+Shift+F12".into(),
            oturumda_gizli_baslat: true,
            ..Ayarlar::default()
        };
        let (b, uyari) = Ayarlar::toml_coz(&a.toml_metni()).unwrap();
        assert_eq!(a, b);
        assert!(uyari.is_empty());
        let (bos, _) = Ayarlar::toml_coz("kisayol_goster_gizle = \"\"\n").unwrap();
        assert_eq!(bos.kisayol_goster_gizle, "");
    }

    #[test]
    fn toml_eksik_anahtar_varsayilan_bilinmeyen_uyari_bool_kabul() {
        let (a, uyari) = Ayarlar::toml_coz("tik = \"cift\"\noturumda_gizli_baslat = true\nyeni = 1\nbaslangicta_calis = \"evet\"\n").unwrap();
        assert_eq!(a.tik, Tik::Cift);
        assert!(a.oturumda_gizli_baslat);
        assert_eq!(a.kucultme, Kucultme::Tepsi, "eksik: varsayılan");
        assert_eq!(
            uyari.len(),
            2,
            "yeni ve dosyada tutulmayan baslangicta_calis: {uyari:?}"
        );
        assert!(Ayarlar::toml_coz("").unwrap().0 == Ayarlar::default());
    }

    #[test]
    fn toml_bozuklari_reddeder() {
        for metin in [
            "tik = ",
            "tik = \"uc\"",
            "kucultme = 3",
            "kapaliyken_tik_acar = \"belki\"",
            "kisayol_goster_gizle = \"Ctrl+Foo\"",
            "kisayol_goster_gizle = \"Ctrl+Alt+İ\"",
        ] {
            assert!(Ayarlar::toml_coz(metin).is_err(), "{metin:?}");
        }
    }

    #[test]
    fn post_govdesi_coklu_satir_ve_hatalar() {
        let v = satirlari_coz(
            "tik=cift\r\nkucultme = gorev_cubugu\n\nkisayol_goster_gizle=ctrl+alt+x\n",
        )
        .unwrap();
        assert_eq!(
            v,
            vec![
                AyarDegeri::Tik(Tik::Cift),
                AyarDegeri::Kucultme(Kucultme::GorevCubugu),
                AyarDegeri::KisayolGosterGizle("Ctrl+Alt+X".into()),
            ]
        );
        assert_eq!(
            satirlari_coz("baslangicta_calis=evet").unwrap(),
            vec![AyarDegeri::BaslangictaCalis(true)]
        );
        for kotu in [
            "",
            "\n\r\n",
            "tik",
            "tik=uc",
            "bilinmeyen=evet",
            "tik=cift\nkucultme=yok",
            "kisayol_goster_gizle=Ctrl+Alt+İ",
            "oturumda_gizli_baslat=true",
        ] {
            assert!(satirlari_coz(kotu).is_err(), "{kotu:?}");
        }
    }

    #[test]
    fn uygula_her_degeri_ayarlara_isler() {
        let mut a = Ayarlar::default();
        for d in satirlari_coz(
            "kucultme=gorev_cubugu\ntik=cift\nkapaliyken_tik_acar=hayir\n\
             kisayol_goster_gizle=\noturumda_gizli_baslat=evet\nbaslangicta_calis=evet",
        )
        .unwrap()
        {
            a.uygula(&d);
        }
        assert_eq!(a.kucultme, Kucultme::GorevCubugu);
        assert_eq!(a.tik, Tik::Cift);
        assert!(!a.kapaliyken_tik_acar);
        assert_eq!(a.kisayol_goster_gizle, "");
        assert!(a.oturumda_gizli_baslat);
    }

    #[test]
    fn eksik_dosya_varsayilan_ve_kayit() {
        let d = gecici("eksik");
        let y = yukle(&d);
        assert_eq!(y.ayarlar, Ayarlar::default());
        assert!(y.yazilabilir);
        assert_eq!(y.kayitlar.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kaydet_yukle_gidis_donus_ve_gecici_dosya_kalmaz() {
        let d = gecici("gidis");
        let a = Ayarlar {
            tik: Tik::Cift,
            kisayol_goster_gizle: "Alt+Win+Space".into(),
            ..Ayarlar::default()
        };
        kaydet(&d, &a).unwrap();
        kaydet(&d, &a).unwrap(); // üzerine yazma
        assert_eq!(yukle(&d).ayarlar, a);
        assert!(!d.join("ayar.toml.tmp").exists(), "geçici dosya kalmamalı");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bozuk_dosya_kenara_alinir_ustune_yazilmaz() {
        let d = gecici("bozuk");
        std::fs::write(d.join("ayar.toml"), "tik = [bozuk").unwrap();
        let y = yukle(&d);
        assert_eq!(y.ayarlar, Ayarlar::default());
        assert!(y.yazilabilir);
        assert!(y.kayitlar[0].contains("bozuk") && y.kayitlar[0].contains(".bozuk"));
        assert_eq!(
            std::fs::read_to_string(d.join("ayar.toml.bozuk")).unwrap(),
            "tik = [bozuk",
            "bozuk içerik kaybolmamalı"
        );
        assert!(!d.join("ayar.toml").exists(), "asıl dosya kenara alındı");
        // Sonraki kayıt temiz dosya yazar, kenardakine dokunmaz.
        kaydet(&d, &y.ayarlar).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.join("ayar.toml.bozuk")).unwrap(),
            "tik = [bozuk"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn gecersiz_deger_de_bozuk_sayilir() {
        let d = gecici("deger");
        std::fs::write(d.join("ayar.toml"), "tik = \"uc\"\n").unwrap();
        let y = yukle(&d);
        assert_eq!(y.ayarlar, Ayarlar::default());
        assert!(d.join("ayar.toml.bozuk").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kenara_alinamazsa_yazilabilir_false() {
        // `.bozuk` adında bir DİZİN varsa üzerine rename başarısız olur (Windows ve Unix).
        let d = gecici("engel");
        std::fs::write(d.join("ayar.toml"), "tik = [").unwrap();
        std::fs::create_dir(d.join("ayar.toml.bozuk")).unwrap();
        std::fs::write(d.join("ayar.toml.bozuk").join("x"), "x").unwrap();
        let y = yukle(&d);
        assert!(!y.yazilabilir, "{:?}", y.kayitlar);
        assert!(d.join("ayar.toml").exists(), "asıl dosya yerinde kalmalı");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn genel_durum_varsayilanla_baslar_ayarlaninca_okunur() {
        // Süreç genelinde tek durum: bu sınama başka sınama tarafından okunmaz.
        let a = Ayarlar {
            tik: Tik::Cift,
            ..Ayarlar::default()
        };
        ayarla(a.clone());
        assert_eq!(guncel(), a);
    }

    #[test]
    fn not_ve_kalinan_sayfa_varsayilanlari() {
        let a = Ayarlar::default();
        assert_eq!(a.kisayol_not, "Ctrl+Alt+N");
        assert_eq!(a.not_hedefi, NotHedefi::Gunluk);
        assert!(a.kalinan_sayfa);
        assert_eq!(a.api_bekleyen, None);
    }

    #[test]
    fn tam_get_govdesi_eski_alti_satir_sozlesmesini_korur_yenileri_ekler() {
        let a = Ayarlar::default();
        let tam = a.satirlar_tam(false, false);
        assert!(
            tam.starts_with(&a.satirlar(false)),
            "eski gövde önek kalmalı: {tam}"
        );
        assert_eq!(
            &tam[a.satirlar(false).len()..],
            "kisayol_not=Ctrl+Alt+N\nnot_hedefi=gunluk\nkalinan_sayfa=evet\n\
             api_otomatik_baslat=hayir\napi_otomatik_bekleyen=\n"
        );
        // Gerçek değer evet, bekleyen yok.
        assert!(a
            .satirlar_tam(false, true)
            .ends_with("api_otomatik_baslat=evet\napi_otomatik_bekleyen=\n"));
        // Bekleyen varsa istenen değer ve bekleyen satırı görünür.
        let b = Ayarlar {
            api_bekleyen: Some(true),
            not_hedefi: NotHedefi::YeniSayfa,
            kalinan_sayfa: false,
            kisayol_not: String::new(),
            ..Ayarlar::default()
        };
        let s = b.satirlar_tam(true, false);
        assert!(s.ends_with(
            "kisayol_not=\nnot_hedefi=yeni_sayfa\nkalinan_sayfa=hayir\n\
             api_otomatik_baslat=evet\napi_otomatik_bekleyen=evet\n"
        ));
    }

    #[test]
    fn yeni_anahtarlar_dogrulanir() {
        assert_eq!(
            AyarDegeri::coz("kisayol_not", "ctrl+alt+x"),
            Ok(AyarDegeri::KisayolNot("Ctrl+Alt+X".into()))
        );
        assert_eq!(
            AyarDegeri::coz("kisayol_not", ""),
            Ok(AyarDegeri::KisayolNot(String::new()))
        );
        assert!(AyarDegeri::coz("kisayol_not", "Ctrl+Alt+İ").is_err());
        assert!(AyarDegeri::coz("kisayol_not", "Ctrl+Foo").is_err());
        assert_eq!(
            AyarDegeri::coz("not_hedefi", "yeni_sayfa"),
            Ok(AyarDegeri::NotHedefi(NotHedefi::YeniSayfa))
        );
        assert_eq!(
            AyarDegeri::coz("not_hedefi", "GUNLUK"),
            Ok(AyarDegeri::NotHedefi(NotHedefi::Gunluk))
        );
        assert!(AyarDegeri::coz("not_hedefi", "sayfa").is_err());
        assert_eq!(
            AyarDegeri::coz("kalinan_sayfa", "hayir"),
            Ok(AyarDegeri::KalinanSayfa(false))
        );
        assert!(AyarDegeri::coz("kalinan_sayfa", "true").is_err());
        assert_eq!(
            AyarDegeri::coz("api_otomatik_baslat", "evet"),
            Ok(AyarDegeri::ApiOtomatikBaslat(true))
        );
        assert!(AyarDegeri::coz("api_otomatik_baslat", "belki").is_err());
        // Bekleyen değer uçtan değiştirilemez (yalnız dosyada).
        assert!(AyarDegeri::coz("api_bekleyen", "evet").is_err());
        assert!(satirlari_coz("api_otomatik_bekleyen=evet").is_err());
    }

    #[test]
    fn yeni_degerler_uygulanir_api_degeri_dosyaya_girmez() {
        let mut a = Ayarlar::default();
        for d in satirlari_coz(
            "kisayol_not=Ctrl+Shift+F9\nnot_hedefi=yeni_sayfa\nkalinan_sayfa=hayir\n\
             api_otomatik_baslat=evet",
        )
        .unwrap()
        {
            a.uygula(&d);
        }
        assert_eq!(a.kisayol_not, "Ctrl+Shift+F9");
        assert_eq!(a.not_hedefi, NotHedefi::YeniSayfa);
        assert!(!a.kalinan_sayfa);
        assert_eq!(a.api_bekleyen, None, "uygula bekleyen değeri değiştirmez");
        assert!(!a.toml_metni().contains("api_otomatik"));
    }

    #[test]
    fn toml_yeni_alanlar_gidis_donus_ve_bekleyen_yalniz_varsa_yazilir() {
        let a = Ayarlar {
            kisayol_not: "Alt+Win+N".into(),
            not_hedefi: NotHedefi::YeniSayfa,
            kalinan_sayfa: false,
            api_bekleyen: Some(false),
            ..Ayarlar::default()
        };
        let (b, uyari) = Ayarlar::toml_coz(&a.toml_metni()).unwrap();
        assert_eq!(a, b);
        assert!(uyari.is_empty());
        assert!(!Ayarlar::default().toml_metni().contains("api_bekleyen"));
        let (c, _) = Ayarlar::toml_coz("api_bekleyen = \"evet\"\n").unwrap();
        assert_eq!(c.api_bekleyen, Some(true));
        for kotu in [
            "api_bekleyen = \"belki\"",
            "api_bekleyen = 3",
            "not_hedefi = \"sayfa\"",
            "kalinan_sayfa = \"x\"",
            "kisayol_not = \"Ctrl+Foo\"",
        ] {
            assert!(Ayarlar::toml_coz(kotu).is_err(), "{kotu:?}");
        }
    }
}
