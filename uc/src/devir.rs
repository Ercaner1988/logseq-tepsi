//! Sunucu iş parçacığından ileti döngüsü (pencere sahibi) iş parçacığına EŞZAMANLI ayar devri.
//! Sunucu isteği kuyruğa koyar, pencere sahibini uyandırır ve (zaman aşımına kadar) beklediği
//! SONUCU yanıta yazar; zaman aşımında istek iptal edilir (kuyrukta ya da yarım kalmaz): "204
//! döndü ama uygulanmadı/sonradan uygulandı" durumu olmaz. Win32'ye bağımlı değildir; uyandırma
//! ve bekleme çağıranda (ikili crate'te `SendMessageTimeoutW`).

use logseq_tepsi_cekirdek::ayar::AyarDegeri;
use std::sync::{Arc, Mutex, MutexGuard};

enum Durum {
    Bekliyor,
    /// Sunucu vazgeçti: pencere sahibi işlemez.
    Iptal,
    /// Pencere sahibi işledi; yanıt gövdesi (boş olabilir).
    Bitti(String),
}

/// Devredilen bir ayar isteği.
pub struct Istek {
    degerler: Vec<AyarDegeri>,
    durum: Mutex<Durum>,
}

fn kilit(m: &Mutex<Durum>) -> MutexGuard<'_, Durum> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Istek {
    /// Sunucu: bekleme bittikten sonra. İşlendiyse yanıt gövdesi; işlenmediyse istek iptal edilir
    /// (sonradan uygulanmaz) ve `None` döner. İşlem sürüyorsa bitmesini bekler (kilit).
    pub fn sonuc(&self) -> Option<String> {
        let mut d = kilit(&self.durum);
        match &*d {
            Durum::Bitti(g) => Some(g.clone()),
            _ => {
                *d = Durum::Iptal;
                None
            }
        }
    }
}

/// Pencere sahibini bekleyen istekler.
pub struct Kuyruk(Mutex<Vec<Arc<Istek>>>);

impl Kuyruk {
    pub const fn yeni() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    /// Sunucu iş parçacığı: isteği kuyruğa koyar (iptal edilmişler ayıklanır).
    pub fn ekle(&self, degerler: Vec<AyarDegeri>) -> Arc<Istek> {
        let istek = Arc::new(Istek {
            degerler,
            durum: Mutex::new(Durum::Bekliyor),
        });
        let mut k = self.0.lock().unwrap_or_else(|e| e.into_inner());
        k.retain(|i| !matches!(*kilit(&i.durum), Durum::Iptal));
        k.push(istek.clone());
        istek
    }

    /// Pencere sahibi: kuyruktaki iptal edilmemiş her isteği `isle` ile işler (gövde metni döner).
    pub fn isle(&self, mut isle: impl FnMut(&[AyarDegeri]) -> String) {
        let bekleyenler = std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()));
        for istek in bekleyenler {
            let mut d = kilit(&istek.durum);
            if matches!(*d, Durum::Bekliyor) {
                *d = Durum::Bitti(isle(&istek.degerler));
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use logseq_tepsi_cekirdek::ayar::Tik;

    fn tik(t: Tik) -> Vec<AyarDegeri> {
        vec![AyarDegeri::Tik(t)]
    }

    #[test]
    fn islenen_istegin_sonucu_gelir() {
        let k = Kuyruk::yeni();
        let a = k.ekle(tik(Tik::Cift));
        let b = k.ekle(tik(Tik::Tek));
        let mut gorulen = Vec::new();
        k.isle(|d| {
            gorulen.push(d.to_vec());
            format!("sonuc{}", gorulen.len())
        });
        assert_eq!(
            gorulen,
            vec![tik(Tik::Cift), tik(Tik::Tek)],
            "sırayla işlenir"
        );
        assert_eq!(a.sonuc().as_deref(), Some("sonuc1"));
        assert_eq!(b.sonuc().as_deref(), Some("sonuc2"));
        // Kuyruk boşaldı: ikinci işleme boş çalışır.
        k.isle(|_| panic!("kuyruk boş olmalıydı"));
    }

    #[test]
    fn islenmeden_sonuc_istenirse_iptal_edilir_ve_sonradan_uygulanmaz() {
        // Pencere sahibi zaman aşımına kadar yanıt vermedi (PostMessage/SendMessage başarısız).
        let k = Kuyruk::yeni();
        let istek = k.ekle(tik(Tik::Cift));
        assert_eq!(istek.sonuc(), None, "işlenmedi: sunucu 503 verir");
        // Pencere sahibi sonradan uyanınca iptal edilmiş değişiklik UYGULANMAZ.
        k.isle(|_| panic!("iptal edilen istek uygulanmamalı"));
        assert_eq!(istek.sonuc(), None);
    }

    #[test]
    fn iptal_edilenler_kuyruktan_ayiklanir() {
        let k = Kuyruk::yeni();
        for _ in 0..50 {
            let i = k.ekle(tik(Tik::Cift));
            assert_eq!(i.sonuc(), None);
        }
        let canli = k.ekle(tik(Tik::Tek));
        assert_eq!(
            k.0.lock().unwrap().len(),
            1,
            "iptal edilenler ayıklandı, yalnız canlı kaldı"
        );
        k.isle(|_| "tamam".into());
        assert_eq!(canli.sonuc().as_deref(), Some("tamam"));
    }

    #[test]
    fn baska_is_parcaciginda_islenip_beklenen_sonuc_gelir() {
        let k = Arc::new(Kuyruk::yeni());
        let istek = k.ekle(tik(Tik::Cift));
        let k2 = k.clone();
        let is = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            k2.isle(|_| "baska-parcacik".into());
        });
        is.join().unwrap();
        assert_eq!(istek.sonuc().as_deref(), Some("baska-parcacik"));
    }
}
