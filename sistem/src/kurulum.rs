//! `logseq-tepsi kur` / `kaldir`: başlangıç kısayolu (IShellLinkW) ve `logseq-tepsi:` URL
//! şeması (HKCU). Üst kısım saf (yol ve komut dizesi üretimi), alt kısım Win32/COM.
//! Görev Zamanlayıcı kullanılmaz; yalnız kendi eklediği şeyleri siler.

use crate::kayit;
use logseq_tepsi_cekirdek::ayar::{self, Ayarlar};
use logseq_tepsi_cekirdek::gunluk;
use logseq_tepsi_cekirdek::metin::{bicim, metin, Dil, DilTercihi, Mesaj, Metin};
use logseq_tepsi_cekirdek::yollar::{
    ayni_yol_mu, baslangic_kisayolu, kisayol_karari_argumanli, sema_anahtari, sema_degerleri,
    url_komutu, KisayolKarari, BASLANGIC_ARGUMANI, SEMA,
};
use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use windows_sys::core::GUID;
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};

// --- COM (IShellLinkW + IPersistFile; windows-sys arayüz metotları vermez) ---

type Hr = i32;
type Yontem = unsafe extern "system" fn();

#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> Hr,
    add_ref: Yontem,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct ShellLinkVtbl {
    temel: UnknownVtbl,
    get_path: unsafe extern "system" fn(*mut c_void, *mut u16, i32, *mut c_void, u32) -> Hr,
    get_id_list: Yontem,
    set_id_list: Yontem,
    get_description: Yontem,
    set_description: unsafe extern "system" fn(*mut c_void, *const u16) -> Hr,
    get_working_directory: Yontem,
    set_working_directory: unsafe extern "system" fn(*mut c_void, *const u16) -> Hr,
    get_arguments: unsafe extern "system" fn(*mut c_void, *mut u16, i32) -> Hr,
    set_arguments: unsafe extern "system" fn(*mut c_void, *const u16) -> Hr,
    get_hotkey: Yontem,
    set_hotkey: Yontem,
    get_show_cmd: Yontem,
    set_show_cmd: Yontem,
    get_icon_location: Yontem,
    set_icon_location: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> Hr,
    set_relative_path: Yontem,
    resolve: Yontem,
    set_path: unsafe extern "system" fn(*mut c_void, *const u16) -> Hr,
}

#[repr(C)]
struct PersistFileVtbl {
    temel: UnknownVtbl,
    get_class_id: Yontem,
    is_dirty: Yontem,
    load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> Hr,
    save: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> Hr,
    save_completed: Yontem,
    get_cur_file: Yontem,
}

#[repr(C)]
struct Nesne<T> {
    vtbl: *const T,
}

const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
const IID_ISHELL_LINK_W: GUID = GUID::from_u128(0x000214f9_0000_0000_c000_000000000046);
const IID_IPERSIST_FILE: GUID = GUID::from_u128(0x0000010b_0000_0000_c000_000000000046);

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Başlangıç kısayolunu `--oturum` argümanıyla yazar (aşağıdaki yazıcı); `aciklama`: kısayolun
/// açıklama metni (kullanıcının dilinde).
fn kisayol_yaz(lnk: &str, exe: &str, aciklama: &str) -> Result<(), String> {
    kisayol_yaz_argumanli(lnk, exe, BASLANGIC_ARGUMANI, aciklama)
}

/// Kısayol dosyasını oluşturur: hedef = exe, çalışma dizini = exe dizini, simge = exe.
fn kisayol_yaz_argumanli(
    lnk: &str,
    exe: &str,
    arguman: &str,
    aciklama: &str,
) -> Result<(), String> {
    let dizin = PathBuf::from(exe)
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    // SAFETY: COM çağrıları tek iş parçacığında; her arayüz işaretçisi Release edilir.
    unsafe {
        let hr = CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
        if hr < 0 {
            return Err(format!("CoInitializeEx HRESULT {hr:#x}"));
        }
        let sonuc = (|| {
            let mut ham: *mut c_void = null_mut();
            let hr = CoCreateInstance(
                &CLSID_SHELL_LINK,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_ISHELL_LINK_W,
                &mut ham,
            );
            if hr < 0 || ham.is_null() {
                return Err(format!("CoCreateInstance(ShellLink) HRESULT {hr:#x}"));
            }
            let kisayol = ham as *mut Nesne<ShellLinkVtbl>;
            let v = &*(*kisayol).vtbl;
            let (exe_g, dizin_g, ac_g, arguman_g) =
                (genis(exe), genis(&dizin), genis(aciklama), genis(arguman));
            let mut hr = (v.set_path)(ham, exe_g.as_ptr());
            if hr >= 0 {
                hr = (v.set_arguments)(ham, arguman_g.as_ptr());
            }
            if hr >= 0 {
                hr = (v.set_working_directory)(ham, dizin_g.as_ptr());
            }
            if hr >= 0 {
                hr = (v.set_description)(ham, ac_g.as_ptr());
            }
            if hr >= 0 {
                hr = (v.set_icon_location)(ham, exe_g.as_ptr(), 0);
            }
            let mut dosya_ham: *mut c_void = null_mut();
            if hr >= 0 {
                hr = (v.temel.query_interface)(ham, &IID_IPERSIST_FILE, &mut dosya_ham);
            }
            if hr >= 0 && !dosya_ham.is_null() {
                let dosya = dosya_ham as *mut Nesne<PersistFileVtbl>;
                let lnk_g = genis(lnk);
                hr = ((*(*dosya).vtbl).save)(dosya_ham, lnk_g.as_ptr(), 1);
                ((*(*dosya).vtbl).temel.release)(dosya_ham);
            }
            (v.temel.release)(ham);
            if hr < 0 {
                Err(format!("IShellLink/IPersistFile::Save HRESULT {hr:#x}"))
            } else {
                Ok(())
            }
        })();
        CoUninitialize();
        sonuc
    }
}

/// `IShellLinkW::GetPath` bayrağı: ortam değişkenleri açılmamış ham yol.
const SLGP_RAWPATH: u32 = 4;

/// Var olan kısayolun gösterdiği hedef yol (okunamazsa hata); yalnız sınamalar kullanır.
#[cfg(test)]
fn kisayol_hedefi(lnk: &str) -> Result<String, String> {
    kisayol_oku(lnk).map(|(hedef, _)| hedef)
}

/// Var olan kısayolun (hedef yol, komut satırı argümanı) çifti (okunamazsa hata).
fn kisayol_oku(lnk: &str) -> Result<(String, String), String> {
    // SAFETY: COM çağrıları tek iş parçacığında; her arayüz işaretçisi Release edilir.
    unsafe {
        let hr = CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
        if hr < 0 {
            return Err(format!("CoInitializeEx HRESULT {hr:#x}"));
        }
        let sonuc = (|| {
            let mut ham: *mut c_void = null_mut();
            let hr = CoCreateInstance(
                &CLSID_SHELL_LINK,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_ISHELL_LINK_W,
                &mut ham,
            );
            if hr < 0 || ham.is_null() {
                return Err(format!("CoCreateInstance(ShellLink) HRESULT {hr:#x}"));
            }
            let v = &*(*(ham as *mut Nesne<ShellLinkVtbl>)).vtbl;
            let mut dosya_ham: *mut c_void = null_mut();
            let mut hr = (v.temel.query_interface)(ham, &IID_IPERSIST_FILE, &mut dosya_ham);
            let mut yol = (String::new(), String::new());
            if hr >= 0 && !dosya_ham.is_null() {
                let dosya = dosya_ham as *mut Nesne<PersistFileVtbl>;
                hr = ((*(*dosya).vtbl).load)(dosya_ham, genis(lnk).as_ptr(), 0);
                ((*(*dosya).vtbl).temel.release)(dosya_ham);
                if hr >= 0 {
                    let mut t = [0u16; 1024];
                    hr = (v.get_path)(
                        ham,
                        t.as_mut_ptr(),
                        t.len() as i32,
                        null_mut(),
                        SLGP_RAWPATH,
                    );
                    let n = t.iter().position(|&c| c == 0).unwrap_or(t.len());
                    yol.0 = String::from_utf16_lossy(&t[..n]);
                    if hr >= 0 {
                        let mut a = [0u16; 1024];
                        hr = (v.get_arguments)(ham, a.as_mut_ptr(), a.len() as i32);
                        let n = a.iter().position(|&c| c == 0).unwrap_or(a.len());
                        yol.1 = String::from_utf16_lossy(&a[..n]);
                    }
                }
            }
            (v.temel.release)(ham);
            if hr < 0 {
                Err(format!("IShellLink/IPersistFile::Load HRESULT {hr:#x}"))
            } else {
                Ok(yol)
            }
        })();
        CoUninitialize();
        sonuc
    }
}

/// COM/Win32 katmanının dile bağlı olmayan teknik hatası (HRESULT adı) kullanıcı iletisi olarak.
fn teknik(s: String) -> Mesaj {
    Mesaj::yeni(Metin::KurTeknik).arg(s)
}

/// Başlangıç kısayolunu bu exe'ye göre kurar: yoksa oluşturur; başka exe'yi gösteriyorsa, argümanı
/// `--oturum` değilse ya da okunamıyorsa yeniden yazar; aynıysa dokunmaz. Yapılanı çevrilebilir
/// iletiyle döndürür; `aciklama`: kısayolun açıklama metni (kullanıcının dilinde).
fn kisayolu_duzenle(lnk: &str, exe: &str, aciklama: &str) -> Result<Mesaj, Mesaj> {
    let var = std::path::Path::new(lnk).exists();
    let okunan = if var {
        kisayol_oku(lnk)
    } else {
        Ok((String::new(), String::new()))
    };
    let (hedef, arguman) = match &okunan {
        Ok((h, a)) => (Some(h.as_str()), Some(a.as_str())),
        Err(_) => (None, None),
    };
    let yaz = || kisayol_yaz(lnk, exe, aciklama).map_err(teknik);
    match kisayol_karari_argumanli(var, hedef, arguman, exe) {
        KisayolKarari::Dokunma => Ok(Mesaj::yeni(Metin::KisayolZatenVar)),
        KisayolKarari::Olustur => yaz().map(|()| Mesaj::yeni(Metin::KisayolOlusturuldu)),
        KisayolKarari::Yeniden => {
            yaz()?;
            Ok(match okunan {
                Ok((eski, a)) if ayni_yol_mu(&eski, exe) => {
                    let m = Mesaj::yeni(Metin::KisayolYenidenArguman);
                    m.arg(format!("{a:?}")).arg(BASLANGIC_ARGUMANI)
                }
                Ok((eski, _)) if !eski.is_empty() => {
                    Mesaj::yeni(Metin::KisayolYenidenBaskaExe).arg(eski)
                }
                Ok(_) => Mesaj::yeni(Metin::KisayolYenidenHedefBos),
                Err(e) => Mesaj::yeni(Metin::KisayolYenidenOkunamadi).arg(e),
            })
        }
    }
}

/// Başlangıç kısayolu var mı? Kısayol dosyasının varlığı gerçeğin kendisidir (COM yok, ucuz).
pub fn baslangic_var_mi() -> bool {
    kisayol_yolu().is_ok_and(|lnk| std::path::Path::new(&lnk).exists())
}

/// "Bilgisayar açılınca başlat": `ac` ise başlangıç kısayolunu kurar (`kur`un kısayol adımı),
/// değilse siler. URL şemasına dokunmaz. Yapılanı çevrilebilir iletiyle döndürür (günlüğe
/// `Display` ile Türkçe gider); `dil`: kısayolun açıklama metninin dili.
pub fn baslangic_ayarla(ac: bool, dil: Dil) -> Result<Mesaj, Mesaj> {
    let aciklama = metin(dil, Metin::KisayolAciklama);
    baslangic_degistir(&kisayol_yolu()?, &exe_yolu()?, ac, aciklama)
}

fn baslangic_degistir(lnk: &str, exe: &str, ac: bool, aciklama: &str) -> Result<Mesaj, Mesaj> {
    if ac {
        return kisayolu_duzenle(lnk, exe, aciklama);
    }
    match std::fs::remove_file(lnk) {
        Ok(()) => Ok(Mesaj::yeni(Metin::KisayolSilindi)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(Mesaj::yeni(Metin::KisayolZatenYoktu))
        }
        Err(e) => Err(Mesaj::yeni(Metin::KurKisayolSilinemedi)
            .arg(lnk)
            .arg(e.to_string())),
    }
}

fn exe_yolu() -> Result<String, Mesaj> {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .map_err(|e| Mesaj::yeni(Metin::KurExeOkunamadi).arg(e.to_string()))
}

fn kisayol_yolu() -> Result<String, Mesaj> {
    std::env::var("APPDATA")
        .map(|a| baslangic_kisayolu(&a))
        .map_err(|_| Mesaj::yeni(Metin::KurAppdataYok))
}

/// `kur`/`kaldir` çıktısının dili: `ayar.toml`daki `dil` (dosya yok, okunamaz ya da bozuksa
/// `otomatik`) ve Windows görüntü dili. Dosyaya DOKUNMAZ (bozuksa kenara almaz).
fn konsol_dili() -> Dil {
    konsol_dili_dizinden(gunluk::dizin().as_deref(), crate::dil::sistem_langid())
}

fn konsol_dili_dizinden(dizin: Option<&std::path::Path>, sistem_langid: u16) -> Dil {
    let tercih = dizin
        .and_then(|d| std::fs::read_to_string(d.join(ayar::DOSYA_ADI)).ok())
        .and_then(|m| Ayarlar::toml_coz(&m).ok())
        .map_or(DilTercihi::Otomatik, |(a, _)| a.dil);
    tercih.coz(sistem_langid)
}

fn kur_icin(dil: Dil) -> Result<(), Mesaj> {
    let m = |m, a: &[&str]| bicim(dil, m, a);
    let exe = exe_yolu()?;
    println!("{}", m(Metin::KurExe, &[&exe]));
    let lnk = kisayol_yolu()?;
    let durum = kisayolu_duzenle(&lnk, &exe, metin(dil, Metin::KisayolAciklama))?;
    println!("{}", m(Metin::KurBaslangic, &[&lnk, &durum.yaz(dil)]));
    let degerler = sema_degerleri(&exe);
    let guncel = degerler
        .iter()
        .all(|(k, ad, veri)| kayit::hkcu_deger_oku(k, ad).as_deref() == Some(veri.as_str()));
    let anahtar = sema_anahtari();
    if guncel {
        println!("{}", m(Metin::KurSemaGuncel, &[SEMA, &anahtar]));
    } else {
        for (k, ad, veri) in &degerler {
            kayit::hkcu_deger_yaz(k, ad, veri).map_err(|e| {
                Mesaj::yeni(Metin::KurKayitYazilamadi)
                    .arg(k.to_string())
                    .arg(e.to_string())
            })?;
        }
        println!("{}", m(Metin::KurSemaYazildi, &[SEMA, &anahtar]));
        println!("{}", m(Metin::KurKomut, &[&url_komutu(&exe)]));
    }
    Ok(())
}

/// `logseq-tepsi kur`: çıkış kodu 0 başarılı.
pub fn kur() -> i32 {
    let dil = konsol_dili();
    match kur_icin(dil) {
        Ok(()) => {
            println!("{}", metin(dil, Metin::KurTamam));
            0
        }
        Err(e) => {
            println!("{}", bicim(dil, Metin::KurHata, &[&e.yaz(dil)]));
            1
        }
    }
}

fn kaldir_icin(dil: Dil) -> Result<(), Mesaj> {
    let m = |m, a: &[&str]| bicim(dil, m, a);
    let lnk = kisayol_yolu()?;
    match std::fs::remove_file(&lnk) {
        Ok(()) => println!("{}", m(Metin::KaldirKisayolSilindi, &[&lnk])),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("{}", m(Metin::KaldirKisayolYok, &[&lnk]));
        }
        Err(e) => {
            let hata = Mesaj::yeni(Metin::KurKisayolSilinemedi).arg(lnk);
            return Err(hata.arg(e.to_string()));
        }
    }
    let anahtar = sema_anahtari();
    match kayit::hkcu_agac_sil(&anahtar) {
        Ok(true) => println!("{}", m(Metin::KaldirSemaSilindi, &[&anahtar])),
        Ok(false) => println!("{}", m(Metin::KaldirSemaYok, &[&anahtar])),
        Err(e) => {
            return Err(Mesaj::yeni(Metin::KurSemaSilinemedi).arg(e.to_string()));
        }
    }
    Ok(())
}

/// `logseq-tepsi kaldir`: yalnız kendi kısayolunu ve şema anahtarını siler.
pub fn kaldir() -> i32 {
    let dil = konsol_dili();
    match kaldir_icin(dil) {
        Ok(()) => {
            println!("{}", metin(dil, Metin::KaldirTamam));
            0
        }
        Err(e) => {
            println!("{}", bicim(dil, Metin::KurHata, &[&e.yaz(dil)]));
            1
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const ACIKLAMA: &str = "sınama";

    /// Sınama sonucunun Türkçe metni (başarısızsa sınama düşer).
    fn tr(sonuc: Result<Mesaj, Mesaj>) -> String {
        sonuc.unwrap().yaz(Dil::Tr)
    }

    /// COM yolunu gerçekten dener ama yalnız geçici dizine yazar (başlangıç klasörüne değil).
    #[test]
    fn kisayol_gecici_dizine_yazilir() {
        let dir = std::env::temp_dir().join(format!("logseq-tepsi-sina-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lnk = dir.join("sina.lnk");
        let exe = r"C:\Program Files\Araclar\logseq-tepsi.exe";
        kisayol_yaz(lnk.to_str().unwrap(), exe, ACIKLAMA).unwrap();
        let boyut = std::fs::metadata(&lnk).unwrap().len();
        if std::env::var_os("LOGSEQ_TEPSI_LNK_KALSIN").is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        } else {
            println!("lnk: {}", lnk.display());
        }
        assert!(boyut > 100, "kisayol dosyasi cok kucuk: {boyut}");
    }

    /// `kur`un kısayol adımı: yoksa oluşturur, aynıysa dokunmaz, başka exe'yi gösteriyorsa ya da
    /// bozuksa yeniden yazar ve bunu söyler. Yalnız geçici dizine yazar.
    #[test]
    fn kisayolu_duzenle_olustur_dokunma_yeniden() {
        let dir = std::env::temp_dir().join(format!("logseq-tepsi-kd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lnk = dir.join("Logseq-tepsi.lnk");
        let lnk_s = lnk.to_str().unwrap();
        let (eski, yeni) = (
            r"C:\Program Files\Eski\logseq-tepsi.exe",
            r"C:\Program Files\Yeni\logseq-tepsi.exe",
        );

        assert_eq!(tr(kisayolu_duzenle(lnk_s, eski, ACIKLAMA)), "oluşturuldu");
        assert_eq!(kisayol_hedefi(lnk_s).unwrap(), eski);

        let bayt = std::fs::read(&lnk).unwrap();
        let ayni = tr(kisayolu_duzenle(lnk_s, eski, ACIKLAMA));
        assert!(ayni.contains("dokunulmadı"), "{ayni}");
        assert_eq!(
            std::fs::read(&lnk).unwrap(),
            bayt,
            "aynı exe: dosya değişmemeli"
        );

        let degisti = tr(kisayolu_duzenle(lnk_s, yeni, ACIKLAMA));
        assert!(
            degisti.contains("yeniden yazıldı") && degisti.contains(eski),
            "{degisti}"
        );
        assert_eq!(kisayol_hedefi(lnk_s).unwrap(), yeni);

        std::fs::write(&lnk, b"bozuk").unwrap();
        let bozuk = tr(kisayolu_duzenle(lnk_s, yeni, ACIKLAMA));
        assert!(bozuk.contains("yeniden yazıldı"), "{bozuk}");
        assert_eq!(kisayol_hedefi(lnk_s).unwrap(), yeni);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn com_vtbl_yerlesimi_dogru_boyutta() {
        // IUnknown 3 + IShellLinkW 18 yöntem; IUnknown 3 + IPersistFile 6 yöntem.
        let p = std::mem::size_of::<usize>();
        assert_eq!(std::mem::size_of::<ShellLinkVtbl>(), 21 * p);
        assert_eq!(std::mem::size_of::<PersistFileVtbl>(), 9 * p);
    }

    fn gecici_dizin(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("logseq-tepsi-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// `kur` kısayolu `--oturum` argümanıyla yazar; hedef aynı ama argüman farklıysa (eski kurulum
    /// ya da elle değişmiş) yeniden yazar ve nedenini söyler.
    #[test]
    fn kisayol_oturum_arguman_yazilir_farkliysa_yeniden_yazilir() {
        let dir = gecici_dizin("arg");
        let lnk = dir.join("Logseq-tepsi.lnk");
        let lnk_s = lnk.to_str().unwrap();
        let exe = r"C:\Program Files\Araclar\logseq-tepsi.exe";

        tr(kisayolu_duzenle(lnk_s, exe, ACIKLAMA));
        let (hedef, arguman) = kisayol_oku(lnk_s).unwrap();
        assert_eq!((hedef.as_str(), arguman.as_str()), (exe, "--oturum"));

        // Eski kurulumu taklit et: aynı hedef, argümansız kısayol.
        kisayol_yaz_argumanli(lnk_s, exe, "", ACIKLAMA).unwrap();
        assert_eq!(kisayol_oku(lnk_s).unwrap().1, "");
        let mesaj = tr(kisayolu_duzenle(lnk_s, exe, ACIKLAMA));
        assert!(
            mesaj.contains("argümanı") && mesaj.contains("yeniden yazıldı"),
            "{mesaj}"
        );
        assert_eq!(kisayol_oku(lnk_s).unwrap().1, "--oturum");
        let ayni = tr(kisayolu_duzenle(lnk_s, exe, ACIKLAMA));
        assert!(ayni.contains("dokunulmadı"), "{ayni}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Menüdeki "Bilgisayar açılınca başlat": açınca kısayol kurulur, kapatınca silinir; ikisi de
    /// tekrarlanabilir (idempotent). Yalnız geçici dizine yazar.
    #[test]
    fn baslangic_degistir_kurar_ve_siler() {
        let dir = gecici_dizin("bas");
        let lnk = dir.join("Logseq-tepsi.lnk");
        let lnk_s = lnk.to_str().unwrap();
        let exe = r"C:\Program Files\Araclar\logseq-tepsi.exe";

        assert_eq!(
            tr(baslangic_degistir(lnk_s, exe, false, ACIKLAMA)),
            "zaten yoktu"
        );
        assert_eq!(
            tr(baslangic_degistir(lnk_s, exe, true, ACIKLAMA)),
            "oluşturuldu"
        );
        assert!(lnk.exists());
        assert!(tr(baslangic_degistir(lnk_s, exe, true, ACIKLAMA)).contains("dokunulmadı"));
        assert_eq!(
            tr(baslangic_degistir(lnk_s, exe, false, ACIKLAMA)),
            "silindi"
        );
        assert!(!lnk.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Kısayol adımının durum iletisi iki dilde: aynı karar, çevrilmiş metin.
    #[test]
    fn kisayol_durumu_iki_dilde_yazilir() {
        let dir = gecici_dizin("durum-dil");
        let lnk = dir.join("Logseq-tepsi.lnk");
        let lnk_s = lnk.to_str().unwrap();
        let exe = r"C:\Program Files\Araclar\logseq-tepsi.exe";
        let en = |s: Result<Mesaj, Mesaj>| s.unwrap().yaz(Dil::En);

        assert_eq!(
            en(baslangic_degistir(lnk_s, exe, false, ACIKLAMA)),
            "did not exist"
        );
        assert_eq!(
            en(baslangic_degistir(lnk_s, exe, true, ACIKLAMA)),
            "created"
        );
        let ayni = en(baslangic_degistir(lnk_s, exe, true, ACIKLAMA));
        assert!(ayni.contains("left untouched"), "{ayni}");
        let baska = en(kisayolu_duzenle(
            lnk_s,
            r"C:\Baska\logseq-tepsi.exe",
            ACIKLAMA,
        ));
        assert!(
            baska.contains("rewritten for this exe") && baska.contains(exe),
            "{baska}"
        );
        assert_eq!(
            en(baslangic_degistir(lnk_s, exe, false, ACIKLAMA)),
            "deleted"
        );
        // Günlüğe giden `Display` Türkçedir.
        let m = Mesaj::yeni(Metin::KisayolZatenYoktu);
        assert_eq!(m.to_string(), "zaten yoktu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `kur`/`kaldir` dili: dosyadaki açık seçim sistem dilini ezer; dosya yok ya da bozuksa
    /// `otomatik` (sistem dili). Sınama dosyaya yazmaz, yalnız geçici dizini okutur.
    #[test]
    fn konsol_dili_ayar_dosyasindan_yoksa_sistem_dilinden() {
        const TR: u16 = 0x041F;
        const EN: u16 = 0x0409;
        let dir = gecici_dizin("konsol-dil");
        assert_eq!(konsol_dili_dizinden(Some(&dir), EN), Dil::En, "dosya yok");
        assert_eq!(konsol_dili_dizinden(Some(&dir), TR), Dil::Tr, "dosya yok");
        assert_eq!(konsol_dili_dizinden(None, TR), Dil::Tr, "dizin yok");
        let yaz = |s: &str| std::fs::write(dir.join("ayar.toml"), s).unwrap();
        yaz("dil = \"en\"\n");
        assert_eq!(konsol_dili_dizinden(Some(&dir), TR), Dil::En);
        yaz("dil = \"tr\"\n");
        assert_eq!(konsol_dili_dizinden(Some(&dir), EN), Dil::Tr);
        yaz("dil = \"otomatik\"\n");
        assert_eq!(konsol_dili_dizinden(Some(&dir), EN), Dil::En);
        assert_eq!(konsol_dili_dizinden(Some(&dir), TR), Dil::Tr);
        // Bozuk dosya: sistem diline düşer ve dosyaya dokunulmaz (kenara alınmaz).
        yaz("dil = [bozuk");
        assert_eq!(konsol_dili_dizinden(Some(&dir), EN), Dil::En);
        assert_eq!(
            std::fs::read_to_string(dir.join("ayar.toml")).unwrap(),
            "dil = [bozuk"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
