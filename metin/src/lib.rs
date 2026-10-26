//! logseq-tepsi'nin kullanıcıya görünen TÜM metinleri (Türkçe ve English), dil kararı ve biçimli
//! iletiler. Saf ve bağımlılıksız: Win32 çağrısı (Windows görüntü dili) `sistem` crate'inde,
//! burada yalnız `LANGID` → `Dil` kararı var. `gunluk.txt` satırları BURADA DEĞİLDİR: günlük
//! iç kayıttır, Türkçe kalır.

mod bicim;
mod metinler;

use std::fmt;

pub use bicim::{ipucu, kisayol_etiketi, IpucuDurumu};
pub use metinler::{metin, Metin, TUMU};

/// Arayüzün etkin dili.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dil {
    Tr,
    En,
}

/// `LANG_TURKISH` birincil dil kimliği (`winnt.h`).
const LANG_TURKISH: u16 = 0x1F;

/// Windows `LANGID` (örn. `GetUserDefaultUILanguage`) → dil: birincil dil kimliği (alt 10 bit)
/// Türkçe ise `Tr` (`0x1F`, `0x041F`), değilse `En`.
pub fn dil_kimliginden(langid: u16) -> Dil {
    if langid & 0x3FF == LANG_TURKISH {
        Dil::Tr
    } else {
        Dil::En
    }
}

/// `dil` ayarı: `otomatik` (Windows görüntü dilini izler), `tr` ya da `en`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DilTercihi {
    #[default]
    Otomatik,
    Tr,
    En,
}

impl DilTercihi {
    /// Ayar değerleri (dosyada, uçta ve menüde aynı), `ad` ile aynı sırada.
    pub const ADLAR: [&'static str; 3] = ["otomatik", "tr", "en"];

    /// Dosyadaki ve uçtaki adı.
    pub fn ad(self) -> &'static str {
        match self {
            Self::Otomatik => "otomatik",
            Self::Tr => "tr",
            Self::En => "en",
        }
    }

    /// Ad (ASCII büyük/küçük harf duyarsız) → tercih; tanınmayan: `None`.
    pub fn ayristir(deger: &str) -> Option<Self> {
        [Self::Otomatik, Self::Tr, Self::En]
            .into_iter()
            .find(|t| t.ad().eq_ignore_ascii_case(deger))
    }

    /// Etkin dil: açık seçim olduğu gibi, `otomatik` ise `sistem_langid`e göre.
    pub fn coz(self, sistem_langid: u16) -> Dil {
        match self {
            Self::Otomatik => dil_kimliginden(sistem_langid),
            Self::Tr => Dil::Tr,
            Self::En => Dil::En,
        }
    }
}

/// Şablondaki `{}` yer tutucularını sırayla `args` ile doldurur (eksik argüman boş kalır; argüman
/// içindeki `{}` yeniden işlenmez).
pub fn bicim(dil: Dil, m: Metin, args: &[&str]) -> String {
    let mut sirada = args.iter();
    let mut s = String::new();
    for (i, parca) in metin(dil, m).split("{}").enumerate() {
        if i > 0 {
            s.push_str(sirada.next().copied().unwrap_or_default());
        }
        s.push_str(parca);
    }
    s
}

/// `Mesaj` argümanı: olduğu gibi yazılan dizge ya da dile göre çevrilen metin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    Dizge(String),
    Metin(Metin),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Self::Dizge(s.to_owned())
    }
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Self::Dizge(s)
    }
}

impl From<Metin> for Arg {
    fn from(m: Metin) -> Self {
        Self::Metin(m)
    }
}

/// Dili henüz bilinmeyen (üretildiği yerde çevrilmeyen) kullanıcı iletisi: hata değerleri bunu
/// taşır, gösterileceği yerde `yaz(dil)` ile dile çevrilir. `Display` Türkçedir (günlük).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mesaj {
    pub metin: Metin,
    args: Vec<Arg>,
    onek: Option<String>,
}

impl Mesaj {
    pub fn yeni(metin: Metin) -> Self {
        Self {
            metin,
            args: Vec::new(),
            onek: None,
        }
    }

    pub fn arg(mut self, a: impl Into<Arg>) -> Self {
        self.args.push(a.into());
        self
    }

    /// `anahtar: ileti` biçimi (ayar anahtarı önekli hatalar).
    pub fn onekle(mut self, anahtar: &str) -> Self {
        self.onek = Some(anahtar.to_owned());
        self
    }

    pub fn yaz(&self, dil: Dil) -> String {
        let cevrilen: Vec<String> = self
            .args
            .iter()
            .map(|a| match a {
                Arg::Dizge(s) => s.clone(),
                Arg::Metin(m) => metin(dil, *m).to_owned(),
            })
            .collect();
        let args: Vec<&str> = cevrilen.iter().map(String::as_str).collect();
        let s = bicim(dil, self.metin, &args);
        match &self.onek {
            Some(o) => format!("{o}: {s}"),
            None => s,
        }
    }
}

impl fmt::Display for Mesaj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.yaz(Dil::Tr))
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Türkçeye özgü harfler (İngilizce metinde bulunmamalı).
    const TURKCE_HARFLER: &str = "çğıöşüİĞŞÖÜÇ";

    #[test]
    fn her_metin_iki_dilde_bos_degil() {
        for m in TUMU {
            for dil in [Dil::Tr, Dil::En] {
                assert!(!metin(dil, m).trim().is_empty(), "{m:?} {dil:?} boş");
            }
        }
    }

    /// `TUMU` bildirim sırasıyla ve eksiksiz: `Metin` bildiriminin gövdesindeki varyant sayısı
    /// tabloyla aynı olmalı, her varyant kendi sırasında durmalı.
    #[test]
    fn tumu_tablosu_eksiksiz_ve_sirali() {
        let kaynak = include_str!("metinler.rs");
        let govde = kaynak
            .split("pub enum Metin {")
            .nth(1)
            .and_then(|k| k.split("\n}\n").next())
            .unwrap();
        let varyant = govde
            .lines()
            .map(str::trim)
            .filter(|s| s.ends_with(',') && !s.starts_with("//"))
            .count();
        assert_eq!(varyant, TUMU.len(), "TUMU tablosuna eksik/fazla varyant");
        for (i, m) in TUMU.iter().enumerate() {
            assert_eq!(*m as usize, i, "{m:?} sırada değil");
        }
    }

    #[test]
    fn ingilizce_metinde_turkce_harf_yok_dil_adi_haric() {
        for m in TUMU {
            let s = metin(Dil::En, m).replace("Türkçe", "");
            assert!(!s.chars().any(|c| TURKCE_HARFLER.contains(c)), "{m:?}: {s}");
        }
    }

    #[test]
    fn iki_dilde_yer_tutucu_sayisi_ayni() {
        for m in TUMU {
            assert_eq!(
                metin(Dil::Tr, m).matches("{}").count(),
                metin(Dil::En, m).matches("{}").count(),
                "{m:?}"
            );
        }
    }

    #[test]
    fn iki_dil_gercekten_farkli_sabit_metinler_haric() {
        // Aynı olabilecekler: sabit/teknik metinler.
        const AYNI: [Metin; 5] = [
            Metin::IpucuAcik,
            Metin::KurExe,
            Metin::AyDilEn,
            Metin::MenuApiLogseqKapali,
            Metin::KurTeknik,
        ];
        let ayni = TUMU
            .iter()
            .filter(|m| metin(Dil::Tr, **m) == metin(Dil::En, **m))
            .filter(|m| !AYNI.contains(m))
            .collect::<Vec<_>>();
        assert!(ayni.is_empty(), "çevrilmemiş görünenler: {ayni:?}");
    }

    #[test]
    fn dil_karari_birincil_dil_kimligine_bakar() {
        assert_eq!(dil_kimliginden(0x1F), Dil::Tr);
        assert_eq!(dil_kimliginden(0x041F), Dil::Tr, "tr-TR tam LANGID");
        assert_eq!(dil_kimliginden(0x09), Dil::En);
        assert_eq!(dil_kimliginden(0x0409), Dil::En, "en-US");
        assert_eq!(dil_kimliginden(0x0407), Dil::En, "de-DE");
        assert_eq!(dil_kimliginden(0x0C0A), Dil::En, "es-ES");
        assert_eq!(dil_kimliginden(0), Dil::En);
        assert_eq!(dil_kimliginden(0xFFFF), Dil::En);
        // Alt dil bitleri önemsiz: 0x1F birincil + herhangi alt dil.
        assert_eq!(dil_kimliginden(0x1F | (0x2 << 10)), Dil::Tr);
    }

    #[test]
    fn tercih_otomatikte_sistem_dilini_aciksa_secimi_izler() {
        assert_eq!(DilTercihi::Otomatik.coz(0x041F), Dil::Tr);
        assert_eq!(DilTercihi::Otomatik.coz(0x0409), Dil::En);
        for langid in [0x041F, 0x0409, 0] {
            assert_eq!(DilTercihi::Tr.coz(langid), Dil::Tr);
            assert_eq!(DilTercihi::En.coz(langid), Dil::En);
        }
        assert_eq!(DilTercihi::default(), DilTercihi::Otomatik);
    }

    #[test]
    fn tercih_adi_gidis_donus_ve_buyuk_kucuk_harf_duyarsiz() {
        for t in [DilTercihi::Otomatik, DilTercihi::Tr, DilTercihi::En] {
            assert_eq!(DilTercihi::ayristir(t.ad()), Some(t));
        }
        assert_eq!(DilTercihi::ayristir("TR"), Some(DilTercihi::Tr));
        assert_eq!(DilTercihi::ayristir("Otomatik"), Some(DilTercihi::Otomatik));
        for kotu in ["", "de", "türkçe", "tr-TR", "auto"] {
            assert_eq!(DilTercihi::ayristir(kotu), None, "{kotu:?}");
        }
        let adlar: Vec<&str> = [DilTercihi::Otomatik, DilTercihi::Tr, DilTercihi::En]
            .iter()
            .map(|t| t.ad())
            .collect();
        assert_eq!(adlar, DilTercihi::ADLAR);
    }

    #[test]
    fn bicim_yer_tutuculari_sirayla_doldurur() {
        let a = ["tik", "tek", "cift", "uc"];
        assert_eq!(
            bicim(Dil::Tr, Metin::HataIkiSecenek, &a),
            "tik: tek ya da cift olmalı: uc"
        );
        assert_eq!(
            bicim(Dil::En, Metin::HataIkiSecenek, &a),
            "tik: must be tek or cift: uc"
        );
        // Argüman içindeki `{}` yeniden işlenmez; eksik argüman boş kalır.
        assert_eq!(
            bicim(Dil::En, Metin::HataBilinmeyenAnahtar, &["{}"]),
            "unknown key: {}"
        );
        assert_eq!(
            bicim(Dil::En, Metin::HataBilinmeyenAnahtar, &[]),
            "unknown key: "
        );
    }

    #[test]
    fn mesaj_dile_gore_yazilir_onek_ve_ic_metin_cevrilir() {
        let m = Mesaj::yeni(Metin::HataConfigsYapi).arg(Metin::RedBos);
        assert_eq!(
            m.yaz(Dil::Tr),
            "configs.edn beklenmeyen yapıda, yazılmadı: dosya boş"
        );
        assert_eq!(
            m.yaz(Dil::En),
            "configs.edn has an unexpected structure; not written: file is empty"
        );
        let o = Mesaj::yeni(Metin::HataKisayolBos).onekle("kisayol_not");
        assert_eq!(o.yaz(Dil::Tr), "kisayol_not: kısayol boş");
        assert_eq!(o.yaz(Dil::En), "kisayol_not: shortcut is empty");
        assert_eq!(o.to_string(), "kisayol_not: kısayol boş", "Display Türkçe");
    }
}
