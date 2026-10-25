//! Logseq'e eklentinin bir modelini çağırtan `logseq://` adresi ve "Logseq hazır olana kadar
//! bekle" adım kararı (saf). `logseq://x-callback-url/invokeCommand?action=plugin.<eklenti>.
//! models.<işlev>&payload=<metin>` adresi Logseq'in çalışan örneğine `second-instance` ile gider,
//! eklentinin `logseq.provideModel({ <işlev>(payload) {} })` işlevini çağırır (Logseq 2.0.2
//! asar'ında doğrulandı; ayrıntı README).

use crate::ayar::NotHedefi;

/// Eklenti kimliği: eklenti klasör adı (`~/.logseq/plugins/logseq-tepsi`) ve `package.json` `logseq.id`.
pub const EKLENTI_KIMLIGI: &str = "logseq-tepsi";
const TABAN: &str = "logseq://x-callback-url/invokeCommand";

/// Sorgu değeri için yüzde kodlaması (RFC 3986 korunmayanlar dışı, UTF-8 baytları).
fn yuzde_kodla(metin: &str) -> String {
    let mut s = String::new();
    for b in metin.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            s.push(char::from(b));
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// Eklentinin `models.<model>` işlevini `payload` ile çağıran adres (`payload` boşsa gönderilmez).
pub fn eklenti_url(model: &str, payload: &str) -> String {
    let action = yuzde_kodla(&format!("plugin.{EKLENTI_KIMLIGI}.models.{model}"));
    if payload.is_empty() {
        format!("{TABAN}?action={action}")
    } else {
        format!("{TABAN}?action={action}&payload={}", yuzde_kodla(payload))
    }
}

/// Anında not adresi (`anindaNot` modeli; payload `gunluk` ya da `yeni_sayfa`).
pub fn not_url(hedef: NotHedefi) -> String {
    eklenti_url("anindaNot", hedef.ad())
}

/// API sunucusu başlat/durdur adresi (`apiSunucusu` modeli; payload `baslat` ya da `durdur`).
pub fn api_url(baslat: bool) -> String {
    eklenti_url("apiSunucusu", if baslat { "baslat" } else { "durdur" })
}

/// Logseq başlatıldıktan sonra adresi göndermek için en çok beklenecek süre.
pub const NOT_TAVAN_MS: u64 = 60_000;
/// Hazırlık yoklama aralığı.
pub const NOT_YOKLAMA_MS: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAdimi {
    /// Adresi gönder.
    Gonder,
    Bekle,
    /// Süre doldu, ana pencere görünmedi: vazgeç.
    Vazgec,
}

/// Bir yoklama adımının kararı. Gönderme için ana pencere görünür VE eklenti araca kendini
/// göstermiş olmalı (eklenti yüklenmeden gelen adres Logseq'te sessizce kaybolur). Süre
/// dolduysa pencere görünüyorsa eklenti haber vermese de bir kez denenir.
pub fn not_adimi(gorunur_ana_sayisi: usize, eklenti_hazir: bool, gecen_ms: u64) -> NotAdimi {
    match (
        gorunur_ana_sayisi > 0,
        eklenti_hazir,
        gecen_ms >= NOT_TAVAN_MS,
    ) {
        (true, true, _) | (true, false, true) => NotAdimi::Gonder,
        (false, _, true) => NotAdimi::Vazgec,
        _ => NotAdimi::Bekle,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn not_adresi_bicimi() {
        assert_eq!(
            not_url(NotHedefi::Gunluk),
            "logseq://x-callback-url/invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=gunluk"
        );
        assert_eq!(
            not_url(NotHedefi::YeniSayfa),
            "logseq://x-callback-url/invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=yeni_sayfa"
        );
    }

    #[test]
    fn api_adresi_bicimi() {
        assert!(api_url(true).ends_with("models.apiSunucusu&payload=baslat"));
        assert!(api_url(false).ends_with("models.apiSunucusu&payload=durdur"));
    }

    #[test]
    fn yuzde_kodlama_ozel_karakter_ve_utf8() {
        assert_eq!(
            yuzde_kodla("a b&c=d/e?f#g%h"),
            "a%20b%26c%3Dd%2Fe%3Ff%23g%25h"
        );
        assert_eq!(yuzde_kodla("Çalış-ma_1.~"), "%C3%87al%C4%B1%C5%9F-ma_1.~");
        assert_eq!(yuzde_kodla(""), "");
        // Kodlanmış değer Logseq'te URLSearchParams ile geri çözülür: gidiş dönüş.
        let ham = "tür=ç&x y+z";
        let cozulen = yuzde_coz(&yuzde_kodla(ham));
        assert_eq!(cozulen, ham);
    }

    /// Sınama için en küçük yüzde çözücü (URLSearchParams `+` işlemini ayrıca yapar; kodlayıcı
    /// `+` üretmez).
    fn yuzde_coz(s: &str) -> String {
        let b = s.as_bytes();
        let mut cikis = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                cikis.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                cikis.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8(cikis).unwrap()
    }

    #[test]
    fn bos_payload_gonderilmez_ve_action_noktalari_korunur() {
        let u = eklenti_url("anindaNot", "");
        assert!(!u.contains("payload"));
        assert!(u.contains("action=plugin.logseq-tepsi.models.anindaNot"));
        // Kodlamada nokta ve tire korunur: Logseq action'ı "." ile bölüp [tür kimlik grup] alır.
        let parcalar: Vec<&str> = "plugin.logseq-tepsi.models.anindaNot".split('.').collect();
        assert_eq!(parcalar, ["plugin", "logseq-tepsi", "models", "anindaNot"]);
    }

    #[test]
    fn not_adimi_karar_tablosu() {
        // Pencere yok: bekle, süre dolunca vazgeç.
        assert_eq!(not_adimi(0, false, 0), NotAdimi::Bekle);
        assert_eq!(not_adimi(0, true, NOT_TAVAN_MS - 1), NotAdimi::Bekle);
        assert_eq!(not_adimi(0, false, NOT_TAVAN_MS), NotAdimi::Vazgec);
        assert_eq!(not_adimi(0, true, NOT_TAVAN_MS), NotAdimi::Vazgec);
        // Pencere var ama eklenti henüz haber vermedi: bekle (adres kaybolurdu).
        assert_eq!(not_adimi(1, false, 5_000), NotAdimi::Bekle);
        assert_eq!(not_adimi(2, false, NOT_TAVAN_MS - 1), NotAdimi::Bekle);
        // Pencere var ve eklenti hazır: gönder.
        assert_eq!(not_adimi(1, true, 0), NotAdimi::Gonder);
        // Süre doldu, pencere var: son bir deneme.
        assert_eq!(not_adimi(1, false, NOT_TAVAN_MS), NotAdimi::Gonder);
        assert_eq!(NOT_TAVAN_MS, 60_000);
    }
}
