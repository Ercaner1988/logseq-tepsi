//! Tepsiye gizlenen pencerelerin sıralı kümesi (Win32'den bağımsız).
//! Çok pencereli Logseq'te her pencere ayrı hatırlanır; sıra gizleme sırasıdır
//! (sondaki en son gizlenen).

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Gizlenenler {
    sira: Vec<isize>,
}

impl Gizlenenler {
    pub const fn yeni() -> Self {
        Self { sira: Vec::new() }
    }

    /// Pencereyi kümeye ekler; zaten varsa "en son gizlenen" olarak sona taşır.
    pub fn ekle(&mut self, kimlik: isize) {
        self.sira.retain(|&k| k != kimlik);
        self.sira.push(kimlik);
    }

    pub fn cikar(&mut self, kimlik: isize) {
        self.sira.retain(|&k| k != kimlik);
    }

    /// Gizleme sırasına göre (eskiden yeniye) tüm kimlikler.
    pub fn hepsi(&self) -> &[isize] {
        &self.sira
    }

    pub fn len(&self) -> usize {
        self.sira.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sira.is_empty()
    }

    /// Kümeyi DEĞİŞTİRMEDEN, `gecerli` diyen kimliklerin sayısı (bayatlar sayılmaz).
    pub fn gecerli_sayisi(&self, gecerli: impl Fn(isize) -> bool) -> usize {
        self.sira.iter().filter(|&&k| gecerli(k)).count()
    }

    /// Bayat kimlikleri (kapanmış pencere, yeniden kullanılmış HWND) kümeden atar.
    pub fn gecerlileri_tut(&mut self, gecerli: impl Fn(isize) -> bool) {
        self.sira.retain(|&k| gecerli(k));
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn iki_pencere_birbirini_unutturmaz() {
        let mut g = Gizlenenler::yeni();
        g.ekle(10);
        g.ekle(20);
        assert_eq!(g.hepsi(), &[10, 20]);
        assert_eq!(g.len(), 2);
        assert!(!g.is_empty());
        assert!(Gizlenenler::yeni().is_empty());
    }

    #[test]
    fn tekrar_eklenen_sona_tasinir_kopyalanmaz() {
        let mut g = Gizlenenler::yeni();
        g.ekle(10);
        g.ekle(20);
        g.ekle(10);
        assert_eq!(g.hepsi(), &[20, 10]);
    }

    #[test]
    fn cikarma_yalniz_hedefi_siler() {
        let mut g = Gizlenenler::yeni();
        g.ekle(1);
        g.ekle(2);
        g.ekle(3);
        g.cikar(2);
        assert_eq!(g.hepsi(), &[1, 3]);
        g.cikar(99);
        assert_eq!(g.hepsi(), &[1, 3]);
    }

    #[test]
    fn gecerli_sayisi_bayatlari_saymaz_kumeyi_degistirmez() {
        let mut g = Gizlenenler::yeni();
        for k in [5, 6, 7, 8] {
            g.ekle(k);
        }
        assert_eq!(g.gecerli_sayisi(|k| k % 2 == 0), 2);
        assert_eq!(g.gecerli_sayisi(|_| false), 0);
        assert_eq!(g.hepsi(), &[5, 6, 7, 8]);
        assert_eq!(g.len(), 4);
    }

    #[test]
    fn bayat_kimlikler_atilir_sira_korunur() {
        let mut g = Gizlenenler::yeni();
        for k in [5, 6, 7, 8] {
            g.ekle(k);
        }
        g.gecerlileri_tut(|k| k % 2 == 0);
        assert_eq!(g.hepsi(), &[6, 8]);
    }
}
