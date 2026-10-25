//! Logseq'in kendi dosyaları ve API sunucusu ile konuşan, Win32'den bağımsız katman:
//! `configs.edn` düzenleyicisi (`edn`, `yapilandirma`) ve API sunucusu kararları/yoklaması (`api`).
//! Hiçbir hata iletisi dosya içeriği ya da değer taşımaz (dosyada gizli değerler var).

pub mod api;
pub mod edn;
pub mod yapilandirma;
