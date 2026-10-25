#![windows_subsystem = "windows"]
//! Logseq'i küçültünce sistem tepsisine gizler. Ayrıntı: README.md.

mod kayit;
mod konsol;
mod kurulum;
mod pencere;
mod sina;
mod soluk;
mod tepsi;
mod uc_sunucu;

use logseq_tepsi_cekirdek::komut::Komut;
// Çekirdek modülleri `crate::...` yoluyla da erişilebilir kalsın.
use logseq_tepsi_cekirdek::{gizlenen, gunluk, komut, mantik, sunucu, uc, yollar};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let komut = komut::cozumle(&args);
    if komut::konsol_gerekir_mi(komut) {
        konsol::baglan();
    }
    match komut {
        Komut::Calistir => {
            gunluk::panik_kancasi_kur(pencere::panikte_goster);
            tepsi::calistir();
        }
        Komut::Kur => std::process::exit(kurulum::kur()),
        Komut::Kaldir => std::process::exit(kurulum::kaldir()),
        Komut::SinaGizleGoster => std::process::exit(sina::gizle_goster()),
        Komut::SinaKucult => std::process::exit(sina::kucult()),
        Komut::SinaSimgeTikla => std::process::exit(sina::simge_tikla()),
        Komut::SinaPencereler => std::process::exit(sina::pencereler()),
        Komut::SinaTik => std::process::exit(sina::tik()),
    }
}
