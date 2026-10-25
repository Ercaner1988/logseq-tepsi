//! Ayarların Win32 tarafı: açılışta yükleme, değişiklikleri uygulama + kaydetme (yalnız ileti
//! döngüsü iş parçacığında) ve yerel uç iş parçacığından gelen değişikliklerin kuyruğu.
//! Saf yapı ve dosya biçimi çekirdekte (`ayar`); `api_otomatik_baslat` gerçek kaynağı Logseq'in
//! `configs.edn` dosyasıdır (`logseq` crate'i; dosyaya YALNIZ Logseq kapalıyken yazılır).

use crate::ayar::{self, AyarDegeri, Ayarlar};
use crate::{api, gunluk, pencere, yapilandirma};
use logseq_tepsi_sistem::kurulum;
use std::path::PathBuf;
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

/// `%APPDATA%\Logseq\configs.edn`.
fn configs_yolu() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| yapilandirma::yol(&a))
}

/// `configs.edn`deki gerçek `:server/autostart` (dosya okunamazsa ya da yapı beklenmedikse
/// `false`; neden yalnız yazarken günlüğe gider, okuma sessizdir).
pub fn api_gercek() -> bool {
    configs_yolu()
        .and_then(|d| yapilandirma::autostart_oku(&d).ok())
        .unwrap_or(false)
}

/// `GET /ayar` gövdesi (sunucu iş parçacığı; salt okur): geçerli ayarlar + başlangıç kısayolunun
/// ve `configs.edn`nin o anki durumu.
pub fn govde() -> String {
    ayar::guncel().satirlar_tam(kurulum::baslangic_var_mi(), api_gercek())
}

/// `api_otomatik_baslat` isteği: Logseq kapalıysa `configs.edn`e şimdi yazar, açıksa bekleyen
/// olarak `yeni`ye işler (Logseq kapanınca `bekleyen_api_uygula` yazar).
fn api_istegi(yeni: &mut Ayarlar, istenen: bool) {
    let Some(dosya) = configs_yolu() else {
        gunluk::yaz("api_otomatik_baslat uygulanamadi: APPDATA yok");
        return;
    };
    let gercek = yapilandirma::autostart_oku(&dosya).unwrap_or(false);
    let karar = api::karar(
        istenen,
        gercek,
        yeni.api_bekleyen,
        pencere::logseq_acik_mi(),
    );
    let sonuc = api::uygula(&dosya, karar, yeni.api_bekleyen);
    yeni.api_bekleyen = sonuc.bekleyen;
    sonuc.gunluk.iter().for_each(|g| gunluk::yaz(g));
}

/// Durum yoklamasından (2 sn): bekleyen `api_otomatik_baslat` varsa ve Logseq `kapali_yoklama`
/// ardışık yoklamadır kapalıysa `configs.edn`e yazar, bekleyeni siler.
pub fn bekleyen_api_uygula(kapali_yoklama: u32) {
    let eski = ayar::guncel();
    let (Some(b), Some(dosya)) = (eski.api_bekleyen, configs_yolu()) else {
        return;
    };
    let gercek = yapilandirma::autostart_oku(&dosya).unwrap_or(false);
    let karar = api::bekleyen_karari(Some(b), gercek, kapali_yoklama);
    if karar == api::Karar::Yok {
        return;
    }
    let sonuc = api::uygula(&dosya, karar, eski.api_bekleyen);
    sonuc.gunluk.iter().for_each(|g| gunluk::yaz(g));
    let mut yeni = eski.clone();
    yeni.api_bekleyen = sonuc.bekleyen;
    if yeni != eski {
        sakla(&yeni);
    }
}

/// Geçerli ayarı yapar ve `ayar.toml`a atomik yazar (yazılamazsa günlüğe).
fn sakla(yeni: &Ayarlar) {
    ayar::ayarla(yeni.clone());
    match gunluk::dizin() {
        Some(_) if !YAZILABILIR.load(Ordering::Relaxed) => {
            gunluk::yaz("ayar dosyasi yazilmadi: bozuk dosya kenara alinamamisti");
        }
        Some(dizin) => {
            if let Err(e) = ayar::kaydet(&dizin, yeni) {
                gunluk::yaz(&format!("ayar dosyasi yazilamadi: {e}"));
            }
        }
        None => gunluk::yaz("ayar dosyasi yazilamadi: LOCALAPPDATA yok"),
    }
}

/// Değişiklikleri uygular ve kaydeder (ileti döngüsü iş parçacığı). `BaslangictaCalis` dosyaya
/// yazılmaz, kısayolu kurar/siler; `ApiOtomatikBaslat` `configs.edn`e (ya da bekleyene) gider.
/// Dosyada tutulan bir ayar değiştiyse `true` (çağıran kısayol kaydını ve ipucunu tazeler).
pub fn uygula(degerler: &[AyarDegeri]) -> bool {
    let eski = ayar::guncel();
    let mut yeni = eski.clone();
    for d in degerler {
        match d {
            AyarDegeri::BaslangictaCalis(ac) => match kurulum::baslangic_ayarla(*ac) {
                Ok(s) => gunluk::yaz(&format!("baslangic kisayolu ({ac}): {s}")),
                Err(e) => gunluk::yaz(&format!("baslangic kisayolu degistirilemedi: {e}")),
            },
            AyarDegeri::ApiOtomatikBaslat(b) => api_istegi(&mut yeni, *b),
            d => yeni.uygula(d),
        }
    }
    if yeni == eski {
        return false;
    }
    gunluk::yaz(&format!("ayar degisti: {degerler:?}"));
    sakla(&yeni);
    true
}
