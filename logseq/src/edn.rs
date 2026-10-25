//! `configs.edn` üst düzey haritasında `:server/autostart` anahtarını okuyan/değiştiren dar,
//! saf metin düzenleyici. Tam EDN ayrıştırıcı değildir: yalnız üst düzey harita elemanlarının
//! sınırlarını bulur (dizge, yorum, karakter sabiti ve iç içe kaplara kanmadan). Beklenmeyen
//! yapıda `Red` döner; hata iletisi içerikten hiçbir şey taşımaz.

use std::fmt;

pub const ANAHTAR: &str = ":server/autostart";

/// Düzenlemenin reddedilme nedeni (sabit metin; dosya içeriği içermez).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Red {
    Bos,
    UstDuzeyHaritaDegil,
    Kapanmamis,
    EslesmeyenKapanis,
    ArtikIcerik,
    TekEleman,
    YasakBicim,
    CiftAnahtar,
    DegerBeklenmedik,
    /// Yazılacak çıktı yeniden ayrıştırılınca anahtar tam bir kez ve beklenen değerde değil.
    Dogrulanamadi,
}

impl fmt::Display for Red {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Red::Bos => "dosya boş",
            Red::UstDuzeyHaritaDegil => "üst düzey öğe bir harita değil",
            Red::Kapanmamis => "kapanmamış dizge ya da harita",
            Red::EslesmeyenKapanis => "eşleşmeyen kapanış",
            Red::ArtikIcerik => "haritadan sonra artık içerik var",
            Red::TekEleman => "haritada anahtar/değer sayısı tek",
            Red::YasakBicim => "beklenmeyen biçim (#_ ya da ^)",
            Red::CiftAnahtar => "anahtar birden çok kez var",
            Red::DegerBeklenmedik => "anahtarın değeri true/false/nil değil",
            Red::Dogrulanamadi => "yazılacak çıktı doğrulanamadı, yazılmadı",
        })
    }
}

/// Bir elemanın `[bas, bit)` bayt aralığı.
type Aralik = (usize, usize);

fn bosluk(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b',')
}

fn sinirlayici(b: u8) -> bool {
    bosluk(b) || matches!(b, b';' | b'"' | b'{' | b'}' | b'[' | b']' | b'(' | b')')
}

/// Boşluk ve yorumları atlar.
fn atla(b: &[u8], mut i: usize) -> usize {
    while i < b.len() {
        if bosluk(b[i]) {
            i += 1;
        } else if b[i] == b';' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else {
            break;
        }
    }
    i
}

/// `"` ile açılan dizgenin bitişi (kapanış tırnağından sonraki indeks).
fn dizge_sonu(b: &[u8], mut i: usize) -> Result<usize, Red> {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    Err(Red::Kapanmamis)
}

/// `\x` karakter sabitinin sonraki indeksi (`x` bir UTF-8 karakter olabilir).
fn karakter_sonu(metin: &str, i: usize) -> usize {
    i + 1 + metin[i + 1..].chars().next().map_or(0, char::len_utf8)
}

/// `{ [ (` ile açılan kabın bitişi; içteki dizge, yorum ve karakter sabitlerine kanmaz.
fn kap_sonu(metin: &str, bas: usize) -> Result<usize, Red> {
    let b = metin.as_bytes();
    let mut yigin: Vec<u8> = Vec::new();
    let mut i = bas;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i = dizge_sonu(b, i)?;
                continue;
            }
            b';' => {
                i = atla(b, i);
                continue;
            }
            b'\\' => {
                i = karakter_sonu(metin, i);
                continue;
            }
            b'{' => yigin.push(b'}'),
            b'[' => yigin.push(b']'),
            b'(' => yigin.push(b')'),
            k @ (b'}' | b']' | b')') => {
                if yigin.pop() != Some(k) {
                    return Err(Red::EslesmeyenKapanis);
                }
                if yigin.is_empty() {
                    return Ok(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(Red::Kapanmamis)
}

/// `i`'de başlayan tek bir EDN öğesinin bitişi (etiketli öğe, etiketle birlikte).
fn oge_sonu(metin: &str, i: usize) -> Result<usize, Red> {
    let b = metin.as_bytes();
    match b[i] {
        b'"' => dizge_sonu(b, i),
        b'{' | b'[' | b'(' => kap_sonu(metin, i),
        b'^' => Err(Red::YasakBicim),
        b'#' => match b.get(i + 1) {
            Some(b'_') => Err(Red::YasakBicim),
            Some(b'{') => kap_sonu(metin, i + 1),
            Some(b'"') => dizge_sonu(b, i + 1),
            Some(b'#') => {
                // Sembolik değer (##Inf, ##-Inf, ##NaN): etiket değil, tek simge.
                let mut j = i + 2;
                while j < b.len() && !sinirlayici(b[j]) {
                    j += 1;
                }
                Ok(j)
            }
            _ => {
                // Etiket (#inst, #uuid ...): etiket sözcüğü + onu izleyen öğe.
                let mut j = i + 1;
                while j < b.len() && !sinirlayici(b[j]) {
                    j += 1;
                }
                let sonraki = atla(b, j);
                if sonraki >= b.len() || matches!(b[sonraki], b'}' | b']' | b')') {
                    return Err(Red::TekEleman);
                }
                oge_sonu(metin, sonraki)
            }
        },
        _ => {
            let mut j = i;
            if b[j] == b'\\' {
                j = karakter_sonu(metin, j);
            }
            while j < b.len() && !sinirlayici(b[j]) {
                j += 1;
            }
            Ok(j.max(i + 1))
        }
    }
}

/// Üst düzey haritanın elemanları (anahtar, değer, anahtar ...), açılış `{` ve kapanış `}` indeksi.
fn ayristir(metin: &str) -> Result<(Vec<Aralik>, usize, usize), Red> {
    let b = metin.as_bytes();
    let bas = if metin.starts_with('\u{feff}') { 3 } else { 0 };
    let acilis = atla(b, bas);
    match b.get(acilis) {
        None => return Err(Red::Bos),
        Some(b'{') => {}
        Some(_) => return Err(Red::UstDuzeyHaritaDegil),
    }
    let mut elemanlar = Vec::new();
    let mut i = acilis + 1;
    loop {
        i = atla(b, i);
        match b.get(i) {
            None => return Err(Red::Kapanmamis),
            Some(b'}') => break,
            Some(b']' | b')') => return Err(Red::EslesmeyenKapanis),
            Some(_) => {
                let son = oge_sonu(metin, i)?;
                elemanlar.push((i, son));
                i = son;
            }
        }
    }
    if atla(b, i + 1) != b.len() {
        return Err(Red::ArtikIcerik);
    }
    if elemanlar.len() % 2 != 0 {
        return Err(Red::TekEleman);
    }
    Ok((elemanlar, acilis, i))
}

/// `:server/autostart` değerinin aralığı (yoksa `None`; birden çoksa `CiftAnahtar`).
fn deger_araligi(metin: &str, elemanlar: &[Aralik]) -> Result<Option<Aralik>, Red> {
    let mut bulunan = None;
    for cift in elemanlar.chunks(2) {
        if &metin[cift[0].0..cift[0].1] == ANAHTAR {
            if bulunan.is_some() {
                return Err(Red::CiftAnahtar);
            }
            bulunan = Some(cift[1]);
        }
    }
    Ok(bulunan)
}

/// `:server/autostart` değeri: yok ya da `nil`/`false` = `false`.
pub fn oku(metin: &str) -> Result<bool, Red> {
    let (elemanlar, _, _) = ayristir(metin)?;
    Ok(match deger_araligi(metin, &elemanlar)? {
        None => false,
        Some((bas, bit)) => match &metin[bas..bit] {
            "true" => true,
            "false" | "nil" => false,
            _ => return Err(Red::DegerBeklenmedik),
        },
    })
}

fn sozcuk(deger: bool) -> &'static str {
    if deger {
        "true"
    } else {
        "false"
    }
}

/// Son anahtarın satır girintisi (anahtar satırın ilk öğesiyse); yoksa tek boşluk.
fn girinti(metin: &str, anahtar_bas: usize) -> &str {
    let satir_bas = metin[..anahtar_bas].rfind('\n').map_or(0, |n| n + 1);
    let onek = &metin[satir_bas..anahtar_bas];
    if !onek.is_empty() && onek.bytes().all(|c| c == b' ' || c == b'\t') {
        onek
    } else {
        " "
    }
}

/// `:server/autostart` değerini `deger` yapar: anahtar varsa yalnız değer aralığı değişir,
/// yoksa son elemandan sonra (kapanış `}`'den önce) satır sonu/girinti biçimine uyarak eklenir.
/// Öbür baytlar olduğu gibi kalır. Beklenmeyen yapıda `Red`; mevcut değer `true`/`false`/`nil`
/// değilse üstüne yazılmaz (`oku` ile aynı kural). Çıktı yazılmadan önce yeniden ayrıştırılır:
/// anahtar tam bir kez ve beklenen değerde değilse `Red::Dogrulanamadi` (sigorta).
pub fn ayarla(metin: &str, deger: bool) -> Result<String, Red> {
    let yeni = ayarla_ham(metin, deger)?;
    dogrula(&yeni, deger)?;
    Ok(yeni)
}

/// `yeni` metninde `:server/autostart` tam bir kez ve `deger`de mi?
fn dogrula(yeni: &str, deger: bool) -> Result<(), Red> {
    let (elemanlar, _, _) = ayristir(yeni).map_err(|_| Red::Dogrulanamadi)?;
    match deger_araligi(yeni, &elemanlar) {
        Ok(Some((bas, bit))) if &yeni[bas..bit] == sozcuk(deger) => Ok(()),
        _ => Err(Red::Dogrulanamadi),
    }
}

fn ayarla_ham(metin: &str, deger: bool) -> Result<String, Red> {
    let (elemanlar, acilis, kapanis) = ayristir(metin)?;
    let yeni_deger = sozcuk(deger);
    if let Some((bas, bit)) = deger_araligi(metin, &elemanlar)? {
        if !matches!(&metin[bas..bit], "true" | "false" | "nil") {
            return Err(Red::DegerBeklenmedik);
        }
        return Ok(format!("{}{yeni_deger}{}", &metin[..bas], &metin[bit..]));
    }
    let cift = format!("{ANAHTAR} {yeni_deger}");
    let Some(&(_, son_bit)) = elemanlar.last() else {
        return Ok(format!(
            "{}{cift}{}",
            &metin[..=acilis],
            &metin[acilis + 1..]
        ));
    };
    let b = metin.as_bytes();
    let mut yer = kapanis;
    while yer > son_bit && bosluk(b[yer - 1]) {
        yer -= 1;
    }
    let harita = &metin[acilis..kapanis];
    let ek = if harita.contains('\n') {
        let nl = if harita.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let son_anahtar = elemanlar[elemanlar.len() - 2].0;
        format!("{nl}{}{cift}", girinti(metin, son_anahtar))
    } else {
        format!(" {cift}")
    };
    Ok(format!("{}{ek}{}", &metin[..yer], &metin[yer..]))
}

#[cfg(test)]
mod testler {
    use super::*;

    // Sahte içerik: sözcük listesinden üretilmiş değerler (gerçek token biçimi değil).
    const SIR: &str = "elma-kitap-nehir-tas";
    const SIR2: &str = "bulut-masa-kedi-deniz";

    fn sahte() -> String {
        format!(
            "{{:meta/version \"{SIR2}\"\n :theme \"dark\"\n :server/tokens [{{:name \"kedi\" :value \"{SIR}\"}}]\n :not \"}} :server/autostart true\"}}\n"
        )
    }

    #[test]
    fn oku_yok_nil_false_true() {
        assert_eq!(oku("{:a 1}"), Ok(false));
        assert_eq!(oku("{:a 1 :server/autostart nil}"), Ok(false));
        assert_eq!(oku("{:server/autostart false}"), Ok(false));
        assert_eq!(oku("{:server/autostart true :a 2}"), Ok(true));
        assert_eq!(oku("{}"), Ok(false));
    }

    #[test]
    fn oku_dizge_ve_yorum_icindeki_benzerlerine_kanmaz() {
        // Dizge içindeki `}` ve anahtar benzeri: gerçek anahtar yok => false.
        assert_eq!(oku(&sahte()), Ok(false));
        assert_eq!(
            oku("{:a 1 ; :server/autostart true }\n :b 2}"),
            Ok(false),
            "yorumdaki anahtar sayılmaz, yorumdaki }} kapatmaz"
        );
        // Anahtar adına benzeyen başka anahtar ve karakter sabitleri.
        assert_eq!(
            oku("{:server/autostart-x true :c \\} :d \\\" :e 1}"),
            Ok(false)
        );
    }

    #[test]
    fn oku_beklenmedik_deger_ve_cift_anahtar_reddedilir() {
        assert_eq!(
            oku("{:server/autostart \"true\"}"),
            Err(Red::DegerBeklenmedik)
        );
        assert_eq!(oku("{:server/autostart 1}"), Err(Red::DegerBeklenmedik));
        assert_eq!(
            oku("{:server/autostart true :server/autostart false}"),
            Err(Red::CiftAnahtar)
        );
    }

    #[test]
    fn beklenmeyen_yapilar_reddedilir() {
        let beklenen = [
            ("", Red::Bos),
            ("  ; yalniz yorum\n", Red::Bos),
            ("[1 2]", Red::UstDuzeyHaritaDegil),
            (":a", Red::UstDuzeyHaritaDegil),
            ("{:a 1", Red::Kapanmamis),
            ("{:a \"acik}", Red::Kapanmamis),
            ("{:a [1 2}", Red::EslesmeyenKapanis),
            ("{:a (1 2]}", Red::EslesmeyenKapanis),
            ("{:a 1] }", Red::EslesmeyenKapanis),
            ("{:a 1} {:b 2}", Red::ArtikIcerik),
            ("{:a 1} x", Red::ArtikIcerik),
            ("{:a}", Red::TekEleman),
            ("{:a 1 #_ :b :c 2}", Red::YasakBicim),
            ("{:a ^:meta {} :b 1}", Red::YasakBicim),
            ("{:a #inst}", Red::TekEleman),
            ("{:server/autostart 1}", Red::DegerBeklenmedik),
            ("{:server/autostart \"evet\"}", Red::DegerBeklenmedik),
            ("{:server/autostart {:a 1}}", Red::DegerBeklenmedik),
        ];
        for (metin, red) in beklenen {
            assert_eq!(oku(metin), Err(red), "{metin:?}");
            assert_eq!(ayarla(metin, true), Err(red), "{metin:?}");
        }
    }

    #[test]
    fn var_olan_degeri_yalniz_degeri_degistirir() {
        let metin = format!("{{:a \"x\"\n :server/autostart false ; yorum\n :tok \"{SIR}\"}}\n");
        let yeni = ayarla(&metin, true).unwrap();
        assert_eq!(yeni, metin.replace("autostart false", "autostart true"));
        assert_eq!(oku(&yeni), Ok(true));
        // nil değeri de değiştirilir.
        assert_eq!(
            ayarla("{:server/autostart nil}", true).unwrap(),
            "{:server/autostart true}"
        );
        // Aynı değer: metin aynen kalır.
        assert_eq!(ayarla(&yeni, true).unwrap(), yeni);
    }

    #[test]
    fn eksik_anahtar_kapanistan_once_girintiyle_eklenir() {
        let metin = sahte();
        let yeni = ayarla(&metin, true).unwrap();
        // Önceki içeriğin tamamı yerinde (yalnız kapanıştan önce ek satır).
        let beklenen = metin.replacen("\"}\n", "\"\n :server/autostart true}\n", 1);
        assert_eq!(yeni, beklenen);
        assert_eq!(oku(&yeni), Ok(true));
        assert_eq!(
            yeni.matches(ANAHTAR).count(),
            2,
            "biri dizgenin içindeki metin"
        );
        // İkinci uygulama yalnız değeri değiştirir; yinelenmez.
        let kapali = ayarla(&yeni, false).unwrap();
        assert_eq!(oku(&kapali), Ok(false));
        assert_eq!(kapali.matches(ANAHTAR).count(), 2);
    }

    #[test]
    fn tek_satirli_ve_bos_harita() {
        assert_eq!(
            ayarla("{:a 1}", true).unwrap(),
            "{:a 1 :server/autostart true}"
        );
        assert_eq!(ayarla("{}", false).unwrap(), "{:server/autostart false}");
        assert_eq!(
            ayarla("{:a 1 }", true).unwrap(),
            "{:a 1 :server/autostart true }",
            "kapanıştan önceki boşluk korunur"
        );
    }

    #[test]
    fn crlf_ve_bom_korunur_son_yorum_yerinde_kalir() {
        let crlf = "{:a 1\r\n :b 2\r\n}\r\n";
        assert_eq!(
            ayarla(crlf, true).unwrap(),
            "{:a 1\r\n :b 2\r\n :server/autostart true\r\n}\r\n"
        );
        let bom = "\u{feff}{:a 1}";
        assert_eq!(
            ayarla(bom, true).unwrap(),
            "\u{feff}{:a 1 :server/autostart true}"
        );
        // Son elemandan sonraki yorum, eklenen satırdan ÖNCE kalır.
        let yorumlu = "{:a 1 ; son yorum\n}";
        assert_eq!(
            ayarla(yorumlu, true).unwrap(),
            "{:a 1 ; son yorum\n :server/autostart true\n}"
        );
        assert_eq!(oku(&ayarla(yorumlu, true).unwrap()), Ok(true));
    }

    #[test]
    fn girinti_son_anahtar_satirindan_alinir() {
        let metin = "{:a 1\n    :b 2\n}";
        assert_eq!(
            ayarla(metin, false).unwrap(),
            "{:a 1\n    :b 2\n    :server/autostart false\n}"
        );
    }

    #[test]
    fn etiketli_ogeler_ve_kume_degerleri_tek_eleman_sayilir() {
        let metin = "{:t #inst \"2026-10-25\" :s #{1 2} :r #\"a}b\" :server/autostart true}";
        assert_eq!(oku(metin), Ok(true));
        assert_eq!(
            ayarla(metin, false).unwrap(),
            metin.replace("autostart true", "autostart false")
        );
    }

    #[test]
    fn ic_ice_kaplarda_dizge_yorum_ve_karakter_kapatmaz() {
        // Kap içindeki dizgede `}` `]` ve anahtar benzeri, yorumda `}`, karakter sabiti `\}`.
        let metin = format!(
            "{{:a [{{:note \"}}] :server/autostart true\"}}\n ; }}\n \\}} \"{SIR}\"] :b (1 ; )\n 2) :server/autostart false}}"
        );
        assert_eq!(oku(&metin), Ok(false));
        let yeni = ayarla(&metin, true).unwrap();
        assert_eq!(oku(&yeni), Ok(true));
        assert_eq!(yeni, metin.replace("autostart false", "autostart true"));
        // Eksik anahtar: kapaniş yine gerçek son `}`'den önce eklenir.
        let eksik = format!("{{:a [{{:n \"}}\"}}] ; }}\n :b \"{SIR}\"}}");
        let eklenen = ayarla(&eksik, true).unwrap();
        assert!(eklenen.ends_with(":server/autostart true}"), "{eklenen}");
        assert_eq!(oku(&eklenen), Ok(true));
    }

    #[test]
    fn utf8_icerik_bozulmaz() {
        let metin = "{:başlık \"Çalışma günlüğü ğüşiöç\" :c \\ş :server/autostart false}";
        let yeni = ayarla(metin, true).unwrap();
        assert_eq!(yeni, metin.replace("autostart false", "autostart true"));
    }

    #[test]
    fn sembolik_degerler_etiket_degil_tek_oge_sayilir() {
        // `##Inf`/`##NaN` etiket sanılıp sonraki öğeyi yutarsa eleman sayısı bozulur ve anahtar
        // bulunamayıp ikinci kez eklenirdi.
        let metin = "{:a ##Inf :b ##NaN :server/autostart true :c ##-Inf}";
        assert_eq!(oku(metin), Ok(true));
        let yeni = ayarla(metin, false).unwrap();
        assert_eq!(yeni, metin.replace("autostart true", "autostart false"));
        assert_eq!(yeni.matches(ANAHTAR).count(), 1, "yinelenen anahtar yok");
        // Anahtar yokken de tek kez eklenir.
        let eksik = "{:a ##Inf :b ##NaN}";
        let eklenen = ayarla(eksik, true).unwrap();
        assert_eq!(eklenen, "{:a ##Inf :b ##NaN :server/autostart true}");
        assert_eq!(oku(&eklenen), Ok(true));
        // Sondaki sembolik değer harita sonunu yutmaz.
        assert_eq!(oku("{:a ##Inf}"), Ok(false));
    }

    #[test]
    fn beklenmedik_degerin_ustune_yazilmaz() {
        for metin in [
            "{:server/autostart 1}",
            "{:server/autostart \"evet\"}",
            "{:server/autostart {:a 1}}",
            "{:server/autostart :acik}",
        ] {
            for deger in [true, false] {
                assert_eq!(
                    ayarla(metin, deger),
                    Err(Red::DegerBeklenmedik),
                    "{metin:?} {deger}"
                );
            }
        }
    }

    #[test]
    fn sigorta_cift_eksik_ya_da_yanlis_degeri_reddeder() {
        // Doğru çıktı.
        assert_eq!(dogrula("{:a 1 :server/autostart true}", true), Ok(()));
        assert_eq!(dogrula("{:server/autostart false}", false), Ok(()));
        // Yinelenen anahtar, eksik anahtar, beklenenden başka değer ve çözülemeyen çıktı.
        for (metin, deger) in [
            ("{:server/autostart true :server/autostart true}", true),
            ("{:a 1}", false),
            ("{:server/autostart false}", true),
            ("{:server/autostart true", true),
        ] {
            assert_eq!(
                dogrula(metin, deger),
                Err(Red::Dogrulanamadi),
                "{metin:?} {deger}"
            );
        }
    }

    #[test]
    fn red_iletileri_icerik_tasimaz() {
        let kotu = [
            format!("{{:value \"{SIR}\" :x}}"),
            format!("{{:value \"{SIR}\" :server/autostart \"{SIR2}\"}}"),
            format!("{{:value \"{SIR}\""),
            format!("[\"{SIR}\"]"),
            format!("{{:a \"{SIR}\"}} {SIR2}"),
        ];
        for metin in &kotu {
            let hata = oku(metin).unwrap_err().to_string();
            assert!(!hata.contains(SIR) && !hata.contains(SIR2), "{hata}");
            assert!(!format!("{:?}", oku(metin)).contains(SIR));
        }
    }
}
