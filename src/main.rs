#![windows_subsystem = "windows"]
//! Logseq'i küçültünce sistem tepsisine gizler. Ayrıntı: README.md.

mod ayarlar;
mod konsol;
mod not_gonder;
mod pencere;
mod sina;
mod soluk;
mod tepsi;
mod tus;
mod uc_sunucu;

use logseq_tepsi_cekirdek::komut::Komut;
// Çekirdek modülleri `crate::...` yoluyla da erişilebilir kalsın.
use logseq_tepsi_cekirdek::{
    ayar, davranis, gizlenen, gunluk, kisayol, komut, mantik, menu, not, yollar,
};
use logseq_tepsi_logseq::{api, yapilandirma};
use logseq_tepsi_sistem::{kayit, kurulum};
use logseq_tepsi_uc::{sunucu, uc};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let komut = komut::cozumle(&args);
    if komut::konsol_gerekir_mi(komut) {
        konsol::baglan();
    }
    match komut {
        Komut::Calistir => {
            gunluk::panik_kancasi_kur(pencere::panikte_goster);
            tepsi::calistir(komut::oturum_mu(&args));
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
