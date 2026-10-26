//! logseq-tepsi'nin Windows katmanı: kayıt defteri okuma/yazma (`kayit`), süreç sorguları
//! (`surec`), başlangıç kısayolu ve URL şeması kurulumu (`kurulum`), genel kısayol kaydı (`tus`),
//! soluk tepsi simgesi (`soluk`), Windows görüntü dili (`dil`). Saf karar ve dizge üretimi çekirdekte (`yollar`, `kisayol`,
//! `mantik`).

pub mod dil;
pub mod kayit;
pub mod kurulum;
pub mod soluk;
pub mod surec;
pub mod tus;
