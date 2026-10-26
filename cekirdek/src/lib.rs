//! logseq-tepsi'nin Win32'den bağımsız saf çekirdeği (sandık sınırı için ayrı crate).
//! Pencere/Win32 işleri ikili crate'te (`../src`), karar ve ayrıştırma burada, sınamalı.

pub mod ayar;
pub mod davranis;
pub mod gizlenen;
pub mod gunluk;
pub use logseq_tepsi_kisayol as kisayol;
pub use logseq_tepsi_metin as metin;
pub mod komut;
pub mod mantik;
pub mod menu;
pub mod not;
pub mod yollar;
