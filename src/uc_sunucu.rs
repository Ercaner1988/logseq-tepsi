//! Yerel uç TCP sunucusu: yalnız `127.0.0.1`, ayrı iş parçacığı, bloklayıcı `accept`
//! (boşta CPU sıfır). Pencere işleri burada YAPILMAZ: ileti penceresine `PostMessageW`
//! ile devredilir. Bu iş parçacığında panik olmamalı (`panic = "abort"`).

use crate::tepsi::{WM_UC_GIZLE, WM_UC_GOSTER};
use crate::uc::{self, Is, Yanit};
use crate::{gunluk, pencere};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Sunucuyu başlatır. Bağlanamazsa günlüğe yazar; araç yine çalışır.
pub fn baslat(ileti_penceresi: isize) {
    let adres = SocketAddrV4::new(Ipv4Addr::LOCALHOST, uc::PORT);
    let dinleyici = match TcpListener::bind(adres) {
        Ok(d) => d,
        Err(e) => {
            gunluk::yaz(&format!("yerel uc: {adres} dinlenemedi: {e}"));
            return;
        }
    };
    let sonuc = std::thread::Builder::new()
        .name("yerel-uc".into())
        .spawn(move || dongu(&dinleyici, ileti_penceresi));
    match sonuc {
        Ok(_) => gunluk::yaz(&format!("yerel uc: {adres} dinleniyor")),
        Err(e) => gunluk::yaz(&format!("yerel uc: is parcacigi baslamadi: {e}")),
    }
}

fn dongu(dinleyici: &TcpListener, ileti_penceresi: isize) {
    for akim in dinleyici.incoming() {
        match akim {
            Ok(a) => baglanti(a, ileti_penceresi),
            Err(e) => {
                gunluk::yaz(&format!("yerel uc: accept hatasi: {e}"));
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

/// Başlık bloğu bitene, tavana ya da toplam zaman aşımına kadar okur.
fn oku(akim: &mut TcpStream) -> Result<Vec<u8>, std::io::ErrorKind> {
    let son = Instant::now() + Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS);
    let mut tampon: Vec<u8> = Vec::with_capacity(1024);
    let mut parca = [0u8; 1024];
    loop {
        if uc::baslik_sonu(&tampon).is_some() || tampon.len() >= uc::ISTEK_TAVANI {
            return Ok(tampon);
        }
        let kalan = son.saturating_duration_since(Instant::now());
        if kalan.is_zero() {
            return Err(std::io::ErrorKind::TimedOut);
        }
        akim.set_read_timeout(Some(kalan)).map_err(|e| e.kind())?;
        match akim.read(&mut parca) {
            Ok(0) => return Ok(tampon),
            Ok(n) => tampon.extend_from_slice(&parca[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => {
                return Err(match e.kind() {
                    std::io::ErrorKind::WouldBlock => std::io::ErrorKind::TimedOut,
                    k => k,
                })
            }
        }
    }
}

fn yanit_ver(mut akim: TcpStream, y: &Yanit) {
    let _ = akim.set_write_timeout(Some(Duration::from_millis(uc::OKUMA_ZAMAN_ASIMI_MS)));
    let _ = akim.write_all(&uc::yanit_baytlari(y));
    let _ = akim.flush();
    // Okunmamış gövde varken kapatmak RST üretir; yanıt istemciye ulaşsın diye kısaca boşalt.
    let _ = akim.shutdown(Shutdown::Write);
    let _ = akim.set_read_timeout(Some(Duration::from_millis(50)));
    let mut cop = [0u8; 512];
    let _ = akim.read(&mut cop);
}

fn baglanti(mut akim: TcpStream, ileti_penceresi: isize) {
    let yanit = match oku(&mut akim) {
        Err(std::io::ErrorKind::TimedOut) => uc::zaman_asimi_yaniti(),
        Err(_) => return,
        Ok(ham) => match uc::ayristir(&ham) {
            Err(h) => uc::ayris_hatasi_yaniti(h),
            Ok(istek) => uc::karar_ver(&istek, env!("CARGO_PKG_VERSION"), pencere::durum_ozeti),
        },
    };
    let yanit = pencere_isi_devret(yanit, ileti_penceresi);
    yanit_ver(akim, &yanit);
}

/// Yetkili `POST /gizle|/goster` ise işi ileti penceresine postalar; postalanamazsa 503.
fn pencere_isi_devret(y: Yanit, ileti_penceresi: isize) -> Yanit {
    let ileti = match y.is {
        Is::Yok => return y,
        Is::Gizle => WM_UC_GIZLE,
        Is::Goster => WM_UC_GOSTER,
    };
    let tamam = unsafe { PostMessageW(ileti_penceresi as HWND, ileti, 0, 0) } != 0;
    if tamam {
        y
    } else {
        gunluk::yaz("yerel uc: ileti penceresine postalanamadi");
        Yanit {
            kod: 503,
            govde: "503 Service Unavailable\n".into(),
            basliklar: Vec::new(),
            is: Is::Yok,
        }
    }
}
