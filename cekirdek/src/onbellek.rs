//! Kısa ömürlü tek değerlik önbellek (saat dışarıdan verilir: saf, sınanabilir).
//! `/durum` sürekli yoklanırsa her istekte `EnumWindows` taraması yapılmasın diye.

/// `omur_ms` içinde istenen değeri yeniden üretmez. Saat geri giderse (ya da ilk çağrıda)
/// yeniden üretir.
#[derive(Debug, Clone, Copy)]
pub struct ZamanliOnbellek<T: Copy> {
    omur_ms: u64,
    deger: Option<(u64, T)>,
}

impl<T: Copy> ZamanliOnbellek<T> {
    pub const fn yeni(omur_ms: u64) -> Self {
        Self {
            omur_ms,
            deger: None,
        }
    }

    /// Önbellekteki değer `simdi_ms - alinma < omur_ms` ise onu, değilse `uret()` sonucunu verir.
    pub fn al(&mut self, simdi_ms: u64, uret: impl FnOnce() -> T) -> T {
        if let Some((t, d)) = self.deger {
            if simdi_ms >= t && simdi_ms - t < self.omur_ms {
                return d;
            }
        }
        let d = uret();
        self.deger = Some((simdi_ms, d));
        d
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn omur_icinde_yeniden_uretmez_sonra_uretir() {
        let sayac = Cell::new(0);
        let uret = || {
            sayac.set(sayac.get() + 1);
            sayac.get()
        };
        let mut o = ZamanliOnbellek::yeni(500);
        assert_eq!(o.al(1_000, uret), 1);
        assert_eq!(o.al(1_499, uret), 1, "499 ms: önbellekten");
        assert_eq!(sayac.get(), 1);
        assert_eq!(o.al(1_500, uret), 2, "tam 500 ms: yenilenir");
        assert_eq!(o.al(1_700, uret), 2);
        assert_eq!(sayac.get(), 2);
    }

    #[test]
    fn saat_geri_giderse_yeniden_uretir() {
        let mut o = ZamanliOnbellek::yeni(500);
        assert_eq!(o.al(10_000, || 1), 1);
        assert_eq!(o.al(9_000, || 2), 2);
    }

    #[test]
    fn sifir_omur_hic_onbellek_tutmaz() {
        let mut o = ZamanliOnbellek::yeni(0);
        assert_eq!(o.al(5, || 1), 1);
        assert_eq!(o.al(5, || 2), 2);
    }
}
