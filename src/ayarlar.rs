//! Ayarların Win32 tarafı: açılışta yükleme, değişiklikleri uygulama + kaydetme (yalnız ileti
//! döngüsü iş parçacığında) ve yerel uç iş parçacığından gelen değişikliklerin kuyruğu.
//! Saf yapı ve dosya biçimi çekirdekte (`ayar`).

use crate::ayar::{self, AyarDegeri};
use crate::gunluk;
use logseq_tepsi_sistem::kurulum;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Yerel uçtan gelip ileti döngüsünün işlemesini bekleyen doğrulanmış değişiklikler.
static KUYRUK: Mutex<Vec<Vec<AyarDegeri>>> = Mutex::new(Vec::new());
/// `false`: bozuk ayar dosyası kenara alınamadı; üstüne yazılmaz.
static YAZILABILIR: AtomicBool = AtomicBool::new(true);

/// Açılışta bir kez: dosyayı yükler (bozuksa kenara alır), sonucu günlüğe yazar, geçerli ayar yapar.
pub fn yukle() {
    let Some(dizin) = gunluk::dizin() else {
        gunluk::yaz("ayar dizini bulunamadi (LOCALAPPDATA); varsayilanlarla calisiliyor");
        return;
    };
    let y = ayar::yukle(&dizin);
    y.kayitlar.iter().for_each(|k| gunluk::yaz(k));
    YAZILABILIR.store(y.yazilabilir, Ordering::Relaxed);
    ayar::ayarla(y.ayarlar);
}

/// Sunucu iş parçacığı: değişiklikleri ileti döngüsüne bırakır.
pub fn kuyruga_ekle(degerler: Vec<AyarDegeri>) {
    KUYRUK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(degerler);
}

pub fn kuyruktan_al() -> Vec<Vec<AyarDegeri>> {
    std::mem::take(&mut *KUYRUK.lock().unwrap_or_else(|e| e.into_inner()))
}

/// `GET /ayar` gövdesi (sunucu iş parçacığı; salt okur): geçerli ayarlar + başlangıç kısayolunun
/// o anki varlığı.
pub fn govde() -> String {
    ayar::guncel().satirlar(kurulum::baslangic_var_mi())
}

/// Değişiklikleri uygular ve kaydeder (ileti döngüsü iş parçacığı). `BaslangictaCalis` dosyaya
/// yazılmaz, kısayolu kurar/siler. Dosyada tutulan bir ayar değiştiyse `true` (çağıran kısayol
/// kaydını ve ipucunu tazeler).
pub fn uygula(degerler: &[AyarDegeri]) -> bool {
    let eski = ayar::guncel();
    let mut yeni = eski.clone();
    for d in degerler {
        match d {
            AyarDegeri::BaslangictaCalis(ac) => match kurulum::baslangic_ayarla(*ac) {
                Ok(s) => gunluk::yaz(&format!("baslangic kisayolu ({ac}): {s}")),
                Err(e) => gunluk::yaz(&format!("baslangic kisayolu degistirilemedi: {e}")),
            },
            d => yeni.uygula(d),
        }
    }
    if yeni == eski {
        return false;
    }
    ayar::ayarla(yeni.clone());
    gunluk::yaz(&format!("ayar degisti: {degerler:?}"));
    match gunluk::dizin() {
        Some(_) if !YAZILABILIR.load(Ordering::Relaxed) => {
            gunluk::yaz("ayar dosyasi yazilmadi: bozuk dosya kenara alinamamisti");
        }
        Some(dizin) => {
            if let Err(e) = ayar::kaydet(&dizin, &yeni) {
                gunluk::yaz(&format!("ayar dosyasi yazilamadi: {e}"));
            }
        }
        None => gunluk::yaz("ayar dosyasi yazilamadi: LOCALAPPDATA yok"),
    }
    true
}
