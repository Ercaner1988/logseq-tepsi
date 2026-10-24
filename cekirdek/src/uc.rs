//! Yerel uç (eklenti ↔ araç): en küçük HTTP/1.1 ayrıştırıcı ve yetki kararı.
//! Saf kod: ağ, Win32 ve saat yok. Sözleşme (eklenti buna güvenir, DEĞİŞTİRME):
//! `GET /durum`, `POST /gizle`, `POST /goster`; `X-Logseq-Tepsi: 1` zorunlu; `Origin`
//! varsa izin listesinde olmalı; `OPTIONS` ön uçuşu.

pub const PORT: u16 = 47123;
/// Başlık bloğu üst sınırı (bayt).
pub const ISTEK_TAVANI: usize = 8 * 1024;
/// Bir isteğin tamamını okumak için toplam süre.
pub const OKUMA_ZAMAN_ASIMI_MS: u64 = 2_000;
/// Yetki başlığı (ad büyük/küçük harfe duyarsız; değer tam `1`).
pub const YETKI_BASLIGI: &str = "X-Logseq-Tepsi";
/// İzinli `Origin` değerleri (tek yerde tutulur).
pub const IZINLI_ORIGINLER: &[&str] = &["lsp://logseq.io", "lsp://logseq.com", "file://", "null"];

/// Ayrıştırılmış istek (gövde yok sayılır). Başlık adları küçük harfe çevrilir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Istek {
    pub metot: String,
    pub yol: String,
    pub basliklar: Vec<(String, String)>,
}

impl Istek {
    /// Başlığın ilk değeri (ad büyük/küçük harfe duyarsız).
    pub fn baslik(&self, ad: &str) -> Option<&str> {
        self.basliklar
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(ad))
            .map(|(_, d)| d.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AyrisHatasi {
    /// İstek satırı ya da başlık biçimi bozuk, ya da başlık bloğu bitmedi.
    Bozuk,
    /// Başlık bloğu `ISTEK_TAVANI`'nı aştı.
    Buyuk,
}

/// `\r\n\r\n` sonrasının dizini (başlık bloğu bitti mi?).
pub fn baslik_sonu(tampon: &[u8]) -> Option<usize> {
    tampon
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
}

fn token_mu(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

/// İstek satırı + başlıklar. Gövde (başlık sonundan sonrası) yok sayılır.
pub fn ayristir(tampon: &[u8]) -> Result<Istek, AyrisHatasi> {
    let Some(son) = baslik_sonu(tampon) else {
        return Err(if tampon.len() >= ISTEK_TAVANI {
            AyrisHatasi::Buyuk
        } else {
            AyrisHatasi::Bozuk
        });
    };
    if son > ISTEK_TAVANI {
        return Err(AyrisHatasi::Buyuk);
    }
    let metin = std::str::from_utf8(&tampon[..son - 4]).map_err(|_| AyrisHatasi::Bozuk)?;
    let mut satirlar = metin.split("\r\n");
    let ilk = satirlar.next().ok_or(AyrisHatasi::Bozuk)?;
    let parcalar: Vec<&str> = ilk.split(' ').collect();
    let [metot, hedef, surum] = parcalar[..] else {
        return Err(AyrisHatasi::Bozuk);
    };
    if !token_mu(metot) || !metot.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(AyrisHatasi::Bozuk);
    }
    if !surum.starts_with("HTTP/1.") || surum.len() != 8 {
        return Err(AyrisHatasi::Bozuk);
    }
    if !hedef.starts_with('/') || hedef.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return Err(AyrisHatasi::Bozuk);
    }
    let yol = hedef.split(['?', '#']).next().unwrap_or("/");
    let mut basliklar = Vec::new();
    for satir in satirlar {
        // Eski satır katlama ve ':' içermeyen satır bozuk sayılır.
        let Some((ad, deger)) = satir.split_once(':') else {
            return Err(AyrisHatasi::Bozuk);
        };
        if !token_mu(ad) {
            return Err(AyrisHatasi::Bozuk);
        }
        basliklar.push((ad.to_ascii_lowercase(), deger.trim().to_owned()));
    }
    Ok(Istek {
        metot: metot.to_owned(),
        yol: yol.to_owned(),
        basliklar,
    })
}

/// Sunucunun istek üzerine yapacağı pencere işi (ileti penceresine devredilir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Is {
    Yok,
    Gizle,
    Goster,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yanit {
    pub kod: u16,
    pub govde: String,
    pub basliklar: Vec<(String, String)>,
    pub is: Is,
}

fn sebep(kod: u16) -> &'static str {
    match kod {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

fn hata(kod: u16, cors: Vec<(String, String)>) -> Yanit {
    Yanit {
        kod,
        govde: format!("{kod} {}\n", sebep(kod)),
        basliklar: cors,
        is: Is::Yok,
    }
}

/// Ayrıştırma hatasına verilecek yanıt.
pub fn ayris_hatasi_yaniti(h: AyrisHatasi) -> Yanit {
    hata(
        match h {
            AyrisHatasi::Bozuk => 400,
            AyrisHatasi::Buyuk => 431,
        },
        Vec::new(),
    )
}

/// Okuma zaman aşımında verilecek yanıt.
pub fn zaman_asimi_yaniti() -> Yanit {
    hata(408, Vec::new())
}

pub fn origin_izinli_mi(origin: &str) -> bool {
    IZINLI_ORIGINLER
        .iter()
        .any(|o| o.eq_ignore_ascii_case(origin))
}

/// `Host` başlığı varsa yerel adreslerden biri olmalı (DNS yeniden bağlama saldırısına karşı).
fn host_izinli_mi(host: &str) -> bool {
    host.eq_ignore_ascii_case(&format!("127.0.0.1:{PORT}"))
        || host.eq_ignore_ascii_case(&format!("localhost:{PORT}"))
}

fn cors_basliklari(origin: Option<&str>) -> Vec<(String, String)> {
    match origin {
        Some(o) => vec![
            ("Access-Control-Allow-Origin".to_owned(), o.to_owned()),
            ("Vary".to_owned(), "Origin".to_owned()),
        ],
        None => Vec::new(),
    }
}

/// `GET /durum` gövdesi.
pub fn durum_govdesi(surum: &str, gizli: usize, logseq_acik: bool) -> String {
    let logseq = if logseq_acik { "acik" } else { "kapali" };
    format!("logseq-tepsi {surum}\ngizli={gizli}\nlogseq={logseq}\n")
}

/// Yetki + yönlendirme kararı. `durum` yalnız yetkili `GET /durum` için çağrılır
/// (pencere sayımı pahalı olabilir): (tepside gizli pencere sayısı, Logseq açık mı).
pub fn karar_ver(istek: &Istek, surum: &str, durum: impl FnOnce() -> (usize, bool)) -> Yanit {
    if istek.baslik("host").is_some_and(|h| !host_izinli_mi(h)) {
        return hata(403, Vec::new());
    }
    let origin = istek.baslik("origin");
    if origin.is_some_and(|o| !origin_izinli_mi(o)) {
        return hata(403, Vec::new());
    }
    let cors = cors_basliklari(origin);
    if istek.metot == "OPTIONS" {
        // Ön uçuş yalnız izinli Origin ile anlamlı; Origin'siz OPTIONS reddedilir.
        if origin.is_none() {
            return hata(403, Vec::new());
        }
        let mut basliklar = cors;
        basliklar.push((
            "Access-Control-Allow-Methods".to_owned(),
            "GET, POST, OPTIONS".to_owned(),
        ));
        basliklar.push((
            "Access-Control-Allow-Private-Network".to_owned(),
            "true".to_owned(),
        ));
        basliklar.push((
            "Access-Control-Allow-Headers".to_owned(),
            YETKI_BASLIGI.to_owned(),
        ));
        return Yanit {
            kod: 204,
            govde: String::new(),
            basliklar,
            is: Is::Yok,
        };
    }
    if istek.baslik(YETKI_BASLIGI) != Some("1") {
        return hata(403, cors);
    }
    let yetkili = |kod: u16, is: Is| Yanit {
        kod,
        govde: String::new(),
        basliklar: cors_basliklari(origin),
        is,
    };
    match (istek.metot.as_str(), istek.yol.as_str()) {
        ("GET", "/durum") => {
            let (gizli, acik) = durum();
            Yanit {
                kod: 200,
                govde: durum_govdesi(surum, gizli, acik),
                basliklar: cors_basliklari(origin),
                is: Is::Yok,
            }
        }
        ("POST", "/gizle") => yetkili(204, Is::Gizle),
        ("POST", "/goster") => yetkili(204, Is::Goster),
        (_, "/durum") => yanit_405("GET", cors),
        (_, "/gizle" | "/goster") => yanit_405("POST", cors),
        _ => hata(404, cors),
    }
}

fn yanit_405(izin: &str, mut cors: Vec<(String, String)>) -> Yanit {
    cors.push(("Allow".to_owned(), izin.to_owned()));
    hata(405, cors)
}

/// Yanıtın HTTP/1.1 bayt dizisi (her bağlantı kapatılır).
pub fn yanit_baytlari(y: &Yanit) -> Vec<u8> {
    let mut s = format!("HTTP/1.1 {} {}\r\n", y.kod, sebep(y.kod));
    if y.kod != 204 {
        s.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        s.push_str(&format!("Content-Length: {}\r\n", y.govde.len()));
    }
    s.push_str(
        "Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n",
    );
    for (ad, deger) in &y.basliklar {
        s.push_str(&format!("{ad}: {deger}\r\n"));
    }
    s.push_str("\r\n");
    if y.kod != 204 {
        s.push_str(&y.govde);
    }
    s.into_bytes()
}

#[cfg(test)]
mod testler {
    use super::*;

    fn istek(metot: &str, yol: &str, basliklar: &[(&str, &str)]) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            basliklar: basliklar
                .iter()
                .map(|(a, d)| (a.to_ascii_lowercase(), (*d).into()))
                .collect(),
        }
    }

    const YETKI: (&str, &str) = ("X-Logseq-Tepsi", "1");

    fn sor(i: &Istek) -> Yanit {
        karar_ver(i, "9.9.9", || (2, true))
    }

    fn baslik<'a>(y: &'a Yanit, ad: &str) -> Option<&'a str> {
        y.basliklar
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(ad))
            .map(|(_, d)| d.as_str())
    }

    // --- ayrıştırıcı ---

    #[test]
    fn iyi_istek_ayristirilir() {
        let ham = b"GET /durum?x=1 HTTP/1.1\r\nHost: 127.0.0.1:47123\r\nX-Logseq-Tepsi: 1\r\nOrigin:  lsp://logseq.io \r\n\r\n";
        let i = ayristir(ham).unwrap();
        assert_eq!(i.metot, "GET");
        assert_eq!(i.yol, "/durum");
        assert_eq!(i.baslik("x-logseq-tepsi"), Some("1"));
        assert_eq!(i.baslik("ORIGIN"), Some("lsp://logseq.io"));
        assert_eq!(i.baslik("yok"), None);
    }

    #[test]
    fn govde_yok_sayilir() {
        let ham = b"POST /gizle HTTP/1.1\r\nContent-Length: 5\r\n\r\nmerhaba";
        let i = ayristir(ham).unwrap();
        assert_eq!(i.yol, "/gizle");
        assert_eq!(i.basliklar.len(), 1);
    }

    #[test]
    fn bozuk_istek_satirlari() {
        for ham in [
            &b"GET /durum\r\n\r\n"[..],
            b"GET /durum HTTP/1.1 fazla\r\n\r\n",
            b"get /durum HTTP/1.1\r\n\r\n",
            b"GET durum HTTP/1.1\r\n\r\n",
            b"GET /dur um HTTP/1.1\r\n\r\n",
            b"GET /durum HTTP/2.0\r\n\r\n",
            b"GET /durum HTTP/1.11\r\n\r\n",
            b"GET /durum FTP/1.1\r\n\r\n",
            b"\r\n\r\n",
            b" / HTTP/1.1\r\n\r\n",
            b"GET  /durum HTTP/1.1\r\n\r\n",
        ] {
            assert_eq!(
                ayristir(ham),
                Err(AyrisHatasi::Bozuk),
                "{:?}",
                String::from_utf8_lossy(ham)
            );
        }
    }

    #[test]
    fn bozuk_basliklar() {
        for ham in [
            &b"GET /durum HTTP/1.1\r\nBaslikAmaIkiNoktaYok\r\n\r\n"[..],
            b"GET /durum HTTP/1.1\r\n: bos-ad\r\n\r\n",
            b"GET /durum HTTP/1.1\r\nAd Bosluklu: x\r\n\r\n",
            b"GET /durum HTTP/1.1\r\nA: b\r\n katlanmis: x\r\n\r\n",
            b"GET /durum HTTP/1.1\r\nA: \xff\xfe\r\n\r\n",
        ] {
            assert_eq!(
                ayristir(ham),
                Err(AyrisHatasi::Bozuk),
                "{:?}",
                String::from_utf8_lossy(ham)
            );
        }
    }

    #[test]
    fn bitmemis_baslik_bozuk_buyuk_ise_buyuk() {
        assert_eq!(
            ayristir(b"GET /durum HTTP/1.1\r\nHost: x"),
            Err(AyrisHatasi::Bozuk)
        );
        assert_eq!(ayristir(b""), Err(AyrisHatasi::Bozuk));
        let buyuk = vec![b'a'; ISTEK_TAVANI];
        assert_eq!(ayristir(&buyuk), Err(AyrisHatasi::Buyuk));
    }

    #[test]
    fn tavani_asan_baslik_blogu_buyuk() {
        let mut ham = b"GET /durum HTTP/1.1\r\nX: ".to_vec();
        ham.extend(std::iter::repeat_n(b'a', ISTEK_TAVANI));
        ham.extend_from_slice(b"\r\n\r\n");
        assert_eq!(ayristir(&ham), Err(AyrisHatasi::Buyuk));
        // Tavana tam sığan blok kabul edilir.
        let mut tam = b"GET /durum HTTP/1.1\r\nX: ".to_vec();
        let dolgu = ISTEK_TAVANI - tam.len() - 4;
        tam.extend(std::iter::repeat_n(b'a', dolgu));
        tam.extend_from_slice(b"\r\n\r\n");
        assert_eq!(tam.len(), ISTEK_TAVANI);
        assert!(ayristir(&tam).is_ok());
    }

    #[test]
    fn baslik_sonu_bulunur() {
        assert_eq!(baslik_sonu(b"a\r\n\r\nb"), Some(5));
        assert_eq!(baslik_sonu(b"a\r\n\r"), None);
        assert_eq!(baslik_sonu(b"a\n\n"), None);
    }

    // --- yetki kararı ---

    #[test]
    fn yetki_basligi_yoksa_403() {
        for (m, y) in [("GET", "/durum"), ("POST", "/gizle"), ("POST", "/goster")] {
            let r = sor(&istek(m, y, &[]));
            assert_eq!(r.kod, 403, "{m} {y}");
            assert_eq!(r.is, Is::Yok);
        }
    }

    #[test]
    fn yetki_basligi_degeri_tam_bir_olmali() {
        for deger in ["0", "", "true", "11", " 2"] {
            let r = sor(&istek("POST", "/gizle", &[("X-Logseq-Tepsi", deger)]));
            assert_eq!(r.kod, 403, "değer {deger:?}");
            assert_eq!(r.is, Is::Yok);
        }
    }

    #[test]
    fn yetkisiz_istek_pencere_sayimi_yaptirmaz_yol_sizdirmaz() {
        let mut cagrildi = false;
        let r = karar_ver(&istek("GET", "/durum", &[]), "1", || {
            cagrildi = true;
            (0, false)
        });
        assert_eq!(r.kod, 403);
        assert!(!cagrildi);
        // Yetkisiz isteğe bilinmeyen yol da 404 değil 403 (yol varlığı sızmaz).
        assert_eq!(sor(&istek("GET", "/yok", &[])).kod, 403);
    }

    #[test]
    fn yabanci_origin_403_yetki_olsa_bile() {
        for o in [
            "https://evil.example",
            "http://127.0.0.1:47123",
            "lsp://logseq.io.evil",
            "",
            "file:///x",
        ] {
            let r = sor(&istek("POST", "/gizle", &[YETKI, ("Origin", o)]));
            assert_eq!(r.kod, 403, "origin {o:?}");
            assert_eq!(r.is, Is::Yok);
            assert_eq!(baslik(&r, "Access-Control-Allow-Origin"), None);
        }
    }

    #[test]
    fn izinli_originler_gecer_ve_yankilanir() {
        for o in IZINLI_ORIGINLER {
            let r = sor(&istek("POST", "/gizle", &[YETKI, ("Origin", o)]));
            assert_eq!(r.kod, 204, "origin {o}");
            assert_eq!(r.is, Is::Gizle);
            assert_eq!(baslik(&r, "Access-Control-Allow-Origin"), Some(*o));
        }
        // Büyük/küçük harf farkı yankıda olduğu gibi kalır.
        let r = sor(&istek(
            "GET",
            "/durum",
            &[YETKI, ("Origin", "LSP://Logseq.IO")],
        ));
        assert_eq!(r.kod, 200);
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Origin"),
            Some("LSP://Logseq.IO")
        );
    }

    #[test]
    fn origin_olmayan_istek_cors_basligi_almaz() {
        let r = sor(&istek("GET", "/durum", &[YETKI]));
        assert_eq!(r.kod, 200);
        assert_eq!(baslik(&r, "Access-Control-Allow-Origin"), None);
    }

    #[test]
    fn eksik_yetkide_bile_izinli_origin_yankilanir() {
        let r = sor(&istek("POST", "/gizle", &[("Origin", "lsp://logseq.io")]));
        assert_eq!(r.kod, 403);
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Origin"),
            Some("lsp://logseq.io")
        );
    }

    #[test]
    fn on_ucus_izinli_origin_204() {
        let r = sor(&istek(
            "OPTIONS",
            "/gizle",
            &[
                ("Origin", "lsp://logseq.io"),
                ("Access-Control-Request-Method", "POST"),
            ],
        ));
        assert_eq!(r.kod, 204);
        assert_eq!(r.is, Is::Yok);
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Origin"),
            Some("lsp://logseq.io")
        );
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Methods"),
            Some("GET, POST, OPTIONS")
        );
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Private-Network"),
            Some("true")
        );
        assert_eq!(
            baslik(&r, "Access-Control-Allow-Headers"),
            Some("X-Logseq-Tepsi")
        );
        // Ön uçuş yetki başlığı istemez (tarayıcı onu ön uçuşta göndermez).
    }

    #[test]
    fn on_ucus_yabanci_ya_da_origin_siz_403() {
        let yabanci = sor(&istek(
            "OPTIONS",
            "/gizle",
            &[("Origin", "https://evil.example")],
        ));
        assert_eq!(yabanci.kod, 403);
        assert_eq!(baslik(&yabanci, "Access-Control-Allow-Origin"), None);
        assert_eq!(baslik(&yabanci, "Access-Control-Allow-Methods"), None);
        assert_eq!(
            baslik(&yabanci, "Access-Control-Allow-Private-Network"),
            None
        );
        let yok = sor(&istek("OPTIONS", "/gizle", &[]));
        assert_eq!(yok.kod, 403);
    }

    #[test]
    fn host_basligi_yerel_degilse_403() {
        let r = sor(&istek(
            "GET",
            "/durum",
            &[YETKI, ("Host", "evil.example:47123")],
        ));
        assert_eq!(r.kod, 403);
        let r = sor(&istek(
            "GET",
            "/durum",
            &[YETKI, ("Host", "127.0.0.1:47123")],
        ));
        assert_eq!(r.kod, 200);
        let r = sor(&istek(
            "GET",
            "/durum",
            &[YETKI, ("Host", "localhost:47123")],
        ));
        assert_eq!(r.kod, 200);
        let r = sor(&istek("GET", "/durum", &[YETKI, ("Host", "127.0.0.1:80")]));
        assert_eq!(r.kod, 403);
    }

    #[test]
    fn yonlendirme_ve_yan_etkiler() {
        let d = sor(&istek("GET", "/durum", &[YETKI]));
        assert_eq!(d.kod, 200);
        assert_eq!(d.govde, "logseq-tepsi 9.9.9\ngizli=2\nlogseq=acik\n");
        assert_eq!(d.is, Is::Yok);
        let g = sor(&istek("POST", "/gizle", &[YETKI]));
        assert_eq!((g.kod, g.is), (204, Is::Gizle));
        assert!(g.govde.is_empty());
        let s = sor(&istek("POST", "/goster", &[YETKI]));
        assert_eq!((s.kod, s.is), (204, Is::Goster));
    }

    #[test]
    fn bilinmeyen_yol_404_yanlis_metot_405() {
        let r = sor(&istek("GET", "/yok", &[YETKI]));
        assert_eq!((r.kod, r.is), (404, Is::Yok));
        let r = sor(&istek("GET", "/gizle", &[YETKI]));
        assert_eq!((r.kod, r.is), (405, Is::Yok));
        assert_eq!(baslik(&r, "Allow"), Some("POST"));
        let r = sor(&istek("POST", "/durum", &[YETKI]));
        assert_eq!((r.kod, r.is), (405, Is::Yok));
        assert_eq!(baslik(&r, "Allow"), Some("GET"));
        let r = sor(&istek("DELETE", "/gizle", &[YETKI]));
        assert_eq!((r.kod, r.is), (405, Is::Yok));
        assert_eq!(sor(&istek("GET", "/", &[YETKI])).kod, 404);
        assert_eq!(sor(&istek("GET", "/durum/", &[YETKI])).kod, 404);
    }

    #[test]
    fn durum_govdesi_bicimi() {
        assert_eq!(
            durum_govdesi("0.1.0", 0, false),
            "logseq-tepsi 0.1.0\ngizli=0\nlogseq=kapali\n"
        );
    }

    // --- yanıt baytları ---

    #[test]
    fn yanit_200_baytlari() {
        let y = sor(&istek("GET", "/durum", &[YETKI, ("Origin", "null")]));
        let s = String::from_utf8(yanit_baytlari(&y)).unwrap();
        assert!(s.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(s.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(s.contains(&format!("Content-Length: {}\r\n", y.govde.len())));
        assert!(s.contains("Connection: close\r\n"));
        assert!(s.contains("Access-Control-Allow-Origin: null\r\n"));
        assert!(s.ends_with("\r\n\r\nlogseq-tepsi 9.9.9\ngizli=2\nlogseq=acik\n"));
    }

    #[test]
    fn yanit_204_govde_ve_uzunluk_yok() {
        let y = sor(&istek("POST", "/gizle", &[YETKI]));
        let s = String::from_utf8(yanit_baytlari(&y)).unwrap();
        assert!(s.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(!s.contains("Content-Length"));
        assert!(s.ends_with("\r\n\r\n"));
        assert_eq!(s.split("\r\n\r\n").nth(1), Some(""));
    }

    #[test]
    fn hata_yanitlari_durum_satiri() {
        let s = |y: &Yanit| String::from_utf8(yanit_baytlari(y)).unwrap();
        assert!(
            s(&ayris_hatasi_yaniti(AyrisHatasi::Bozuk)).starts_with("HTTP/1.1 400 Bad Request\r\n")
        );
        assert!(s(&ayris_hatasi_yaniti(AyrisHatasi::Buyuk))
            .starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n"));
        assert!(s(&zaman_asimi_yaniti()).starts_with("HTTP/1.1 408 Request Timeout\r\n"));
        let y = sor(&istek("GET", "/yok", &[YETKI]));
        assert!(s(&y).starts_with("HTTP/1.1 404 Not Found\r\n"));
    }
}
