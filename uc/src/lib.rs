//! logseq-tepsi'nin yerel ucu (eklenti ↔ araç): en küçük HTTP/1.1 ayrıştırıcı, yetki kararı ve
//! yalnız-yerel TCP sunucusu. Pencere işleri burada yapılmaz (ikili crate'e devredilir);
//! karar ve ayar ayrıştırması `logseq-tepsi-cekirdek`te.

pub mod onbellek;
pub mod sunucu;
pub mod uc;
