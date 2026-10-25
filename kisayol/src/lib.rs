//! Genel kısayol dizgesi ayrıştırıcısı (saf): `"Ctrl+Alt+L"` → `RegisterHotKey` değiştirici
//! bayrakları ve sanal tuş kodu. Yalnız ASCII; Türkçe harf karşılaştırması yapılmaz (çıplak
//! `to_lowercase` kullanılmaz, `eq_ignore_ascii_case` yeter). Win32 çağrısı ikili crate'te.

pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

/// Bir genel kısayol: değiştirici bayrakları (`MOD_*`) ve sanal tuş kodu (`VK_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kisayol {
    pub degistiriciler: u32,
    pub tus: u32,
}

const DEGISTIRICILER: [(&str, u32); 4] = [
    ("Ctrl", MOD_CONTROL),
    ("Alt", MOD_ALT),
    ("Shift", MOD_SHIFT),
    ("Win", MOD_WIN),
];

/// Adlı tuşlar (kurallı yazım bu tablodaki gibidir).
const ADLI_TUSLAR: [(&str, u32); 14] = [
    ("Space", 0x20),
    ("Enter", 0x0D),
    ("Tab", 0x09),
    ("Esc", 0x1B),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Up", 0x26),
    ("Down", 0x28),
    ("Left", 0x25),
    ("Right", 0x27),
];

const VK_F1: u32 = 0x70;

fn degistirici_coz(ad: &str) -> Option<u32> {
    match ad {
        a if a.eq_ignore_ascii_case("control") => Some(MOD_CONTROL),
        a if a.eq_ignore_ascii_case("windows") => Some(MOD_WIN),
        a => DEGISTIRICILER
            .iter()
            .find(|(d, _)| d.eq_ignore_ascii_case(a))
            .map(|(_, b)| *b),
    }
}

/// F1-F24 ise 1-24 arası sayı.
fn f_tusu(ad: &str) -> Option<u32> {
    let sayi = ad.strip_prefix(['F', 'f'])?;
    if sayi.is_empty() || !sayi.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    sayi.parse().ok().filter(|n| (1..=24).contains(n))
}

fn tus_coz(ad: &str) -> Option<u32> {
    if let [b] = ad.as_bytes() {
        if b.is_ascii_alphabetic() {
            return Some(u32::from(b.to_ascii_uppercase()));
        }
        if b.is_ascii_digit() {
            return Some(u32::from(*b));
        }
    }
    if let Some(n) = f_tusu(ad) {
        return Some(VK_F1 + n - 1);
    }
    ADLI_TUSLAR
        .iter()
        .find(|(a, _)| a.eq_ignore_ascii_case(ad))
        .map(|(_, vk)| *vk)
}

/// `"Ctrl+Alt+L"`, `"Ctrl+Shift+F12"` (büyük/küçük harf duyarsız, ASCII). En az bir değiştirici
/// gerekir; Shift tek başına yalnız F tuşlarıyla kabul edilir (yoksa yazı yazmayı engellerdi).
pub fn ayristir(metin: &str) -> Result<Kisayol, String> {
    if !metin.is_ascii() {
        return Err(format!("kısayol yalnız ASCII olabilir: {metin}"));
    }
    let parcalar: Vec<&str> = metin.split('+').map(str::trim).collect();
    let (tus_adi, onlar) = parcalar.split_last().ok_or("kısayol boş")?;
    if tus_adi.is_empty() || onlar.iter().any(|p| p.is_empty()) {
        return Err(format!("kısayol biçimi bozuk: {metin}"));
    }
    let mut degistiriciler = 0;
    for ad in onlar {
        let b = degistirici_coz(ad).ok_or_else(|| format!("bilinmeyen değiştirici: {ad}"))?;
        if degistiriciler & b != 0 {
            return Err(format!("değiştirici yinelendi: {ad}"));
        }
        degistiriciler |= b;
    }
    let tus = tus_coz(tus_adi).ok_or_else(|| format!("bilinmeyen tuş: {tus_adi}"))?;
    let kalici = degistiriciler & !MOD_SHIFT != 0;
    if !kalici && (degistiriciler == 0 || f_tusu(tus_adi).is_none()) {
        return Err(format!(
            "kısayol Ctrl, Alt ya da Win içermeli (Shift tek başına yalnız F tuşuyla): {metin}"
        ));
    }
    Ok(Kisayol {
        degistiriciler,
        tus,
    })
}

/// Ayar dizgesinden kaydedilecek kısayol: boş (kapalı) ya da geçersizse `None`. Ayarlar
/// doğrulanmış gelir; geçersiz dizge yalnız elle bozulmuş dosyada olur (o zaman dosya zaten
/// bozuk sayılıp kenara alınır).
pub fn ayardan(metin: &str) -> Option<Kisayol> {
    if metin.is_empty() {
        None
    } else {
        ayristir(metin).ok()
    }
}

fn tus_adi(vk: u32) -> String {
    if (0x41..=0x5A).contains(&vk) || (0x30..=0x39).contains(&vk) {
        return char::from(vk as u8).to_string();
    }
    if (VK_F1..VK_F1 + 24).contains(&vk) {
        return format!("F{}", vk - VK_F1 + 1);
    }
    ADLI_TUSLAR
        .iter()
        .find(|(_, v)| *v == vk)
        .map_or_else(|| format!("0x{vk:X}"), |(a, _)| (*a).to_owned())
}

/// Kurallı yazım: `Ctrl+Alt+Shift+Win+Tuş` sırası (ayar dosyasında ve `GET /ayar`da bu görünür).
pub fn bicimle(k: &Kisayol) -> String {
    let mut parcalar: Vec<String> = DEGISTIRICILER
        .iter()
        .filter(|(_, b)| k.degistiriciler & b != 0)
        .map(|(a, _)| (*a).to_owned())
        .collect();
    parcalar.push(tus_adi(k.tus));
    parcalar.join("+")
}

/// Kayıt planı: önce `birak` kimlikleri bırakılır, sonra `kaydet` kaydedilir (iki kısayol yer
/// değiştirirken ikincisi birincinin tuşunu tutmadan kaydedilebilsin).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    /// Bırakılacak kimlikler.
    pub birak: Vec<i32>,
    /// Yeni kaydedilecek (kimlik, kısayol) çiftleri.
    pub kaydet: Vec<(i32, Kisayol)>,
    /// Kaydı değişen ya da yeniden denenen kimlikler (hata işaretleri bunlar için silinir).
    pub degisen: Vec<i32>,
}

/// `mevcut` (kayıtlı kimlik, kısayol) ile `istenen` arasındaki fark. İstenen zaten kayıtlıysa ve
/// kaydı hatalı değilse dokunulmaz; `hatali` kimlik yeniden denenir (tuşu tutan program bırakmış
/// olabilir).
pub fn plan(
    mevcut: &[(i32, Kisayol)],
    istenen: &[(i32, Option<Kisayol>)],
    hatali: impl Fn(i32) -> bool,
) -> Plan {
    let mut p = Plan::default();
    for &(kimlik, yeni) in istenen {
        let eski = mevcut.iter().find(|(k, _)| *k == kimlik).map(|(_, s)| *s);
        if eski == yeni && !hatali(kimlik) {
            continue;
        }
        p.degisen.push(kimlik);
        if eski.is_some() {
            p.birak.push(kimlik);
        }
        if let Some(s) = yeni {
            p.kaydet.push((kimlik, s));
        }
    }
    p
}

#[cfg(test)]
mod testler {
    use super::*;

    fn k(d: u32, t: u32) -> Kisayol {
        Kisayol {
            degistiriciler: d,
            tus: t,
        }
    }

    #[test]
    fn belirtilen_ornekler() {
        assert_eq!(ayristir("Ctrl+Alt+L"), Ok(k(MOD_CONTROL | MOD_ALT, 0x4C)));
        assert_eq!(
            ayristir("Ctrl+Shift+F12"),
            Ok(k(MOD_CONTROL | MOD_SHIFT, 0x7B))
        );
    }

    #[test]
    fn buyuk_kucuk_harf_ve_bosluk_duyarsiz() {
        let beklenen = Ok(k(MOD_CONTROL | MOD_ALT, 0x4C));
        assert_eq!(ayristir("ctrl+alt+l"), beklenen);
        assert_eq!(ayristir("CTRL + ALT + L"), beklenen);
        assert_eq!(ayristir("Control+Alt+l"), beklenen);
        assert_eq!(ayristir("alt+win+f1"), Ok(k(MOD_ALT | MOD_WIN, 0x70)));
        assert_eq!(ayristir("ctrl+SPACE"), Ok(k(MOD_CONTROL, 0x20)));
        assert_eq!(ayristir("ctrl+5"), Ok(k(MOD_CONTROL, 0x35)));
    }

    #[test]
    fn ascii_disi_reddedilir() {
        // Türkçe harf, tam genişlik ve birleşik işaret: ASCII katlama yapılmaz, açık hata.
        for s in [
            "Ctrl+Alt+İ",
            "Ctrl+Alt+ı",
            "Ctrl+Alt+Ｌ",
            "Ctrl+Alt+L\u{307}",
            "Çtrl+L",
        ] {
            let e = ayristir(s).unwrap_err();
            assert!(e.contains("ASCII"), "{s}: {e}");
        }
    }

    #[test]
    fn gecersizler_reddedilir() {
        for s in [
            "",
            "L",
            "Ctrl",
            "Ctrl+",
            "+L",
            "Ctrl++L",
            "Ctrl+Ctrl+L",
            "Ctrl+Foo",
            "Foo+L",
            "Ctrl+F25",
            "Ctrl+F0",
            "Ctrl+LL",
            "Shift+L",
            "Ctrl+Alt+Shift+Win+",
            "Ctrl L",
        ] {
            assert!(ayristir(s).is_err(), "{s:?} kabul edilmemeliydi");
        }
        assert!(ayristir("Shift+F5").is_ok(), "Shift + F tuşu serbest");
        assert!(ayristir("F5").is_err(), "değiştiricisiz F tuşu reddedilir");
    }

    #[test]
    fn kurali_yazim_gidis_donus() {
        for s in [
            "Ctrl+Alt+L",
            "Ctrl+Shift+F12",
            "Alt+Win+Space",
            "Ctrl+Alt+Shift+Win+0",
            "Ctrl+PageDown",
        ] {
            assert_eq!(bicimle(&ayristir(s).unwrap()), s);
        }
        assert_eq!(bicimle(&ayristir("alt+ctrl+x").unwrap()), "Ctrl+Alt+X");
    }

    #[test]
    fn ayardan_bos_ya_da_gecersiz_none() {
        assert_eq!(ayardan(""), None);
        assert_eq!(ayardan("Ctrl+Foo"), None);
        assert_eq!(ayardan("Ctrl+Alt+L"), Some(k(MOD_CONTROL | MOD_ALT, 0x4C)));
    }

    fn a() -> Kisayol {
        k(MOD_CONTROL | MOD_ALT, 0x4C)
    }

    fn b() -> Kisayol {
        k(MOD_CONTROL | MOD_ALT, 0x4E)
    }

    #[test]
    fn plan_takas_once_hepsini_birakir_sonra_kaydeder() {
        let mevcut = [(1, a()), (2, b())];
        let p = plan(&mevcut, &[(1, Some(b())), (2, Some(a()))], |_| false);
        assert_eq!(p.birak, vec![1, 2], "ikisi de bırakılır (takas)");
        assert_eq!(p.kaydet, vec![(1, b()), (2, a())]);
        assert_eq!(p.degisen, vec![1, 2]);
    }

    #[test]
    fn plan_ayni_olana_dokunmaz_hatali_olani_yeniden_dener() {
        let mevcut = [(1, a())];
        assert_eq!(
            plan(&mevcut, &[(1, Some(a())), (2, None)], |_| false),
            Plan::default()
        );
        // Hatalı kimlik (kayıtlı değil, istenen var): yeniden kaydedilir, bırakılacak yok.
        let p = plan(&[], &[(2, Some(b()))], |k| k == 2);
        assert_eq!(
            (p.birak.as_slice(), p.kaydet.as_slice()),
            (&[][..], &[(2, b())][..])
        );
        assert_eq!(p.degisen, vec![2]);
        // Hatalı ve istenen yok: yalnız hata işareti temizlensin.
        let p = plan(&[], &[(2, None)], |k| k == 2);
        assert_eq!((p.birak, p.kaydet, p.degisen), (vec![], vec![], vec![2]));
    }

    #[test]
    fn plan_kapatma_ve_degistirme() {
        let mevcut = [(1, a()), (2, a())];
        let p = plan(&mevcut, &[(1, None), (2, Some(b()))], |_| false);
        assert_eq!(p.birak, vec![1, 2]);
        assert_eq!(p.kaydet, vec![(2, b())]);
    }
}
