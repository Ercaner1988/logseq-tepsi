#![windows_subsystem = "windows"]
//! Logseq'i küçültünce sistem tepsisine gizler. Ayrıntı: README.md.

mod mantik;
mod pencere;
mod sina;
mod tepsi;

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--sina-gizle-goster") => std::process::exit(sina::gizle_goster()),
        Some("--sina-kucult") => std::process::exit(sina::kucult()),
        Some("--sina-simge-tikla") => std::process::exit(sina::simge_tikla()),
        _ => tepsi::calistir(),
    }
}
