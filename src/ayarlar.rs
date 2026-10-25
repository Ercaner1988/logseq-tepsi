//! Ayarların Win32 tarafı: açılışta yükleme, değişiklikleri uygulama + kaydetme (yalnız ileti
//! döngüsü iş parçacığında) ve yerel uç iş parçacığından gelen değişikliklerin eşzamanlı kuyruğu.
//! Saf yapı ve dosya biçimi çekirdekte (`ayar`); `api_otomatik_baslat` gerçek kaynağı Logseq'in
//! `configs.edn` dosyasıdır (`logseq` crate'i; dosyaya YALNIZ Logseq kapalıyken yazılır).

use crate::ayar::{self, AyarDegeri, Ayarlar};
use crate::{api, gunluk, pencere, yapilandirma};
use logseq_tepsi_sistem::kurulum;
use logseq_tepsi_uc::devir::Kuyruk;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Yerel uçtan gelip ileti döngüsünün işlemesini bekleyen doğrulanmış değişiklikler: sunucu
/// sonucu bekler (`uc_sunucu`), ileti döngüsü `kuyruk().isle(..)` ile işler.
static KUYRUK: Kuyruk = Kuyruk::yeni();
/// Logseq ana penceresi kaç ardışık durum yoklamasıdır yok (0: açık).
static KAPALI_YOKLAMA: AtomicU32 = AtomicU32::new(0);
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

/// Sunucu iş parçacığı isteği buraya koyar; ileti döngüsü `isle` ile işler.
pub fn kuyruk() -> &'static Kuyruk {
    &KUYRUK
}

/// Durum zamanlayıcısının her adımında (yalnız o; tıklamalar saymaz): Logseq ana penceresi
/// kapalı kaldıkça sayar, açılınca sıfırlar. Güncel sayıyı döndürür.
pub fn kapali_say(acik: bool) -> u32 {
    let n = if acik {
        0
    } else {
        (KAPALI_YOKLAMA.load(Ordering::Relaxed) + 1).min(api::KAPALI_YOKLAMA_SAYISI)
    };
    KAPALI_YOKLAMA.store(n, Ordering::Relaxed);
    n
}

/// Logseq'in şu anki gözlemi: menü isteği ve bekleyen yolu AYNI kapanış kuralını kullanır (ana
/// pencere yok, `Logseq.exe` süreci yok, ardışık kapalı yoklama yeterli).
fn gozlem() -> api::Gozlem {
    api::Gozlem {
        ana_pencere_var: pencere::logseq_acik_mi(),
        surec_var: pencere::logseq_surec_var(),
        kapali_yoklama: KAPALI_YOKLAMA.load(Ordering::Relaxed),
    }
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

/// `api_otomatik_baslat` isteği: Logseq kapandıysa (bkz. `gozlem`) `configs.edn`e şimdi yazar,
/// değilse bekleyen olarak `yeni`ye işler (Logseq kapanınca `bekleyen_api_uygula` yazar). Sonuç
/// (yazıldı/beklemede/hata) `POST /ayar` yanıtına gider.
fn api_istegi(yeni: &mut Ayarlar, istenen: bool) -> api::Etki {
    let Some(dosya) = configs_yolu() else {
        gunluk::yaz("api_otomatik_baslat uygulanamadi: APPDATA yok");
        return api::Etki::Hata("APPDATA bulunamadı".into());
    };
    let sonuc = api::istegi_uygula(&dosya, istenen, yeni.api_bekleyen, gozlem());
    yeni.api_bekleyen = sonuc.bekleyen;
    sonuc.gunluk.iter().for_each(|g| gunluk::yaz(g));
    sonuc.etki
}

/// Durum yoklamasından (2 sn): bekleyen `api_otomatik_baslat` varsa ve Logseq kapandıysa
/// `configs.edn`e yazar, bekleyeni siler.
pub fn bekleyen_api_uygula() {
    let eski = ayar::guncel();
    let (Some(_), Some(dosya)) = (eski.api_bekleyen, configs_yolu()) else {
        return;
    };
    let Some(sonuc) = api::bekleyeni_uygula(&dosya, eski.api_bekleyen, gozlem()) else {
        return;
    };
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
/// Dosyada tutulan bir ayar değiştiyse ilk değer `true` (çağıran kısayol kaydını ve ipucunu
/// tazeler); ikinci değer `POST /ayar` yanıt gövdesidir (`api_otomatik_baslat` sonucu; yoksa boş).
/// İki genel kısayolu aynı tuşa getiren değişiklik hiç uygulanmaz (menüden gelen de dahil) ve
/// NEDENİYLE `Err` döner: sunucu doğrulaması ile uygulama arasında başka bir istek durumu
/// değiştirmiş olabilir (iki eşzamanlı POST), bu yüzden red 204 değil 400 olur.
pub fn uygula(degerler: &[AyarDegeri]) -> Result<(bool, String), String> {
    let eski = ayar::guncel();
    if let Err(neden) = ayar::cakisma_denetle(&eski, degerler) {
        gunluk::yaz(&format!("ayar reddedildi: {neden}"));
        return Err(neden);
    }
    let mut yeni = eski.clone();
    let mut govde = String::new();
    for d in degerler {
        match d {
            AyarDegeri::BaslangictaCalis(ac) => match kurulum::baslangic_ayarla(*ac) {
                Ok(s) => gunluk::yaz(&format!("baslangic kisayolu ({ac}): {s}")),
                Err(e) => gunluk::yaz(&format!("baslangic kisayolu degistirilemedi: {e}")),
            },
            AyarDegeri::ApiOtomatikBaslat(b) => govde = api_istegi(&mut yeni, *b).govde(),
            d => yeni.uygula(d),
        }
    }
    if yeni == eski {
        return Ok((false, govde));
    }
    gunluk::yaz(&format!("ayar degisti: {degerler:?}"));
    sakla(&yeni);
    Ok((true, govde))
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::ayar::Ayarlar;

    /// İki eşzamanlı POST: ikincisi, birincisi uygulanmadan doğrulanmıştı. Birincisi uygulanınca
    /// ikincisi çakışır: `uygula` sessizce `(false, "")` (204) DEĞİL nedenli `Err` (400) dönmeli.
    /// Çakışan değer hiçbir yan etki (dosya, kısayol, kayıt) üretmeden reddedilir.
    #[test]
    fn cakisan_degisiklik_nedenle_reddedilir_yan_etkisiz() {
        let birinci_uygulandi = Ayarlar {
            kisayol_goster_gizle: "Ctrl+Alt+X".into(),
            kisayol_not: "Ctrl+Alt+N".into(),
            ..Ayarlar::default()
        };
        ayar::ayarla(birinci_uygulandi.clone());
        let ikinci = [AyarDegeri::KisayolNot("Ctrl+Alt+X".into())];
        let Err(neden) = uygula(&ikinci) else {
            panic!("çakışan değişiklik reddedilmeliydi");
        };
        assert!(neden.contains("aynı kısayol olamaz"), "{neden}");
        assert_eq!(ayar::guncel(), birinci_uygulandi, "ayar değişmemeli");
    }
}
