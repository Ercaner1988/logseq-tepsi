//! logseq-tepsi'nin Win32'den bağımsız saf çekirdeği (sandık sınırı için ayrı crate).
//! Pencere/Win32 işleri ikili crate'te (`../src`), karar ve ayrıştırma burada, sınamalı.

pub mod gizlenen;
pub mod gunluk;
pub mod komut;
pub mod mantik;
pub mod yollar;
