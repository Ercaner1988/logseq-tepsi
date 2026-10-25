//! Yerel uç sunucusunun Win32 yapıştırıcısı: sunucu (`uc/src/sunucu.rs`) Win32'den
//! bağımsızdır; pencere ve ayar işleri ileti penceresine `PostMessageW` ile devredilir.

use crate::sunucu::Saglayici;
use crate::tepsi::{WM_UC_AYAR, WM_UC_GIZLE, WM_UC_GOSTER};
use crate::uc::Is;
use crate::{api, ayarlar, pencere};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Sunucuyu başlatır; gizle/göster/ayar işleri `ileti_penceresi`ne postalanır.
pub fn baslat(ileti_penceresi: isize) {
    let devret = move |is: &Is| {
        let ileti = match is {
            Is::Yok => return true,
            Is::Gizle => WM_UC_GIZLE,
            Is::Goster => WM_UC_GOSTER,
            // Değerler kuyruğa girer, ileti döngüsü sırayla işler (postalama başarısızsa 503 döner,
            // kuyrukta kalan değişiklik bir sonraki ayar iletisiyle birlikte işlenir).
            Is::Ayar(degerler) => {
                ayarlar::kuyruga_ekle(degerler.clone());
                WM_UC_AYAR
            }
        };
        unsafe { PostMessageW(ileti_penceresi as HWND, ileti, 0, 0) != 0 }
    };
    let sag = Saglayici {
        surum: env!("CARGO_PKG_VERSION"),
        durum: pencere::durum_ozeti,
        ayar_metni: ayarlar::govde,
        api_acik: api::acik_mi,
    };
    crate::sunucu::baslat(sag, devret);
}
