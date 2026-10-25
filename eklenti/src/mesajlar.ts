// Kullanıcıya gösterilen tüm Türkçe metinler.
import type { Durum, Hata } from "./ag";

export const YONERGE_BASLATMA =
  "Aracı başlatmak için komut paletinden “Tepsi: aracı başlat” komutunu çalıştırın " +
  "ya da terminalde logseq-tepsi yazın.";

export function hataMetni(hata: Hata): string {
  switch (hata.tur) {
    case "kapali":
      return `Tepsi aracı çalışmıyor ya da erişilemiyor. ${YONERGE_BASLATMA} Port ayarını da denetleyin.`;
    case "yasak":
      return "Tepsi aracı isteği reddetti (403). Eklenti ile araç sürümleri uyuşmuyor olabilir; aracı güncelleyin.";
    case "zaman-asimi":
      return "Tepsi aracı zamanında yanıt vermedi. Araç takılmış olabilir; yeniden başlatmayı deneyin.";
    case "beklenmedik": {
      const ek =
        hata.durum !== undefined ? ` (HTTP ${hata.durum})` : hata.ayrinti ? ` (${hata.ayrinti})` : "";
      return `Tepsi aracı beklenmedik bir yanıt verdi${ek}.`;
    }
  }
}

export function durumMetni(d: Durum): string {
  const logseq = d.logseq === "acik" ? "açık" : "kapalı";
  const api = d.api === undefined ? "" : ` API sunucusu: ${d.api === "acik" ? "çalışıyor" : "kapalı"}.`;
  return `Tepsi aracı çalışıyor (sürüm ${d.surum}). Gizli pencere: ${d.gizli}. Logseq: ${logseq}.${api}`;
}

export const BASLATILIYOR =
  "Tepsi aracı başlatılıyor… Logseq bir onay penceresi gösterirse logseq-tepsi bağlantısına izin verin.";

export function zatenCalisiyor(d: Durum): string {
  return `Tepsi aracı zaten çalışıyor (sürüm ${d.surum}).`;
}

export function baslatildi(d: Durum): string {
  return `Tepsi aracı başlatıldı (sürüm ${d.surum}).`;
}

export const BASLATILAMADI =
  "Tepsi aracı başlatılamadı. Onay penceresini kabul ettiniz mi? " +
  "Aracı bir kez “logseq-tepsi kur” ile kaydetmiş olmanız gerekir. " +
  "Elle başlatmak için terminalde logseq-tepsi yazın.";

/** Araç ayarı reddedince gövde `400 Bad Request\n<neden>` gelir; yalnız nedeni döndürür. */
export function redNedeni(ayrinti: string | undefined): string {
  const satirlar = (ayrinti ?? "").split(/\r?\n/).filter((s) => s.length > 0 && !/^400 /.test(s));
  return satirlar.join("; ");
}

export function ayarReddedildi(ayrinti: string | undefined): string {
  const neden = redNedeni(ayrinti);
  return (
    `Tepsi aracı ayar değişikliğini reddetti${neden ? `: ${neden}` : ""}. ` +
    "Eklenti ayarı araçtaki değere döndürüldü."
  );
}

export function ayarIletilemedi(hata: Hata): string {
  return (
    `Ayar değişikliği araca iletilemedi, araçta uygulanmadı. ${hataMetni(hata)} ` +
    "Eklenti ayarı araçtakinden farklı kalır; araç çalışınca eklenti açıldığında araçtaki değer geçerli olur."
  );
}

export function notAcilamadi(neden: string): string {
  return `Anında not açılamadı: ${neden}`;
}

export const NOT_SAYFA_YOK = "bugünün günlük sayfası bulunamadı ya da oluşturulamadı";
export const NOT_BLOK_YOK = "sayfaya boş blok eklenemedi";

export const API_IZIN_YOK =
  "Logseq bu sürümde izin vermiyor; Logseq'in kendi sunucu menüsünü kullanın.";

export function apiIstekGonderildi(baslat: boolean): string {
  return `API sunucusunu ${baslat ? "başlatma" : "durdurma"} isteği Logseq'e gönderildi.`;
}

export function apiDurumu(acik: boolean, istenenAcik: boolean): string {
  const simdi = acik ? "çalışıyor" : "kapalı";
  return acik === istenenAcik
    ? `API sunucusu ${simdi}.`
    : `API sunucusu henüz ${simdi} görünüyor; Logseq'in kendi sunucu menüsünden de bakabilirsiniz.`;
}

export const API_AYARI_BEKLEMEDE =
  "Logseq açık olduğu için API sunucusu ayarı Logseq kapanınca uygulanacak.";
export const API_AYARI_YAZILAMADI =
  "Araç API sunucusu ayarını Logseq'in yapılandırma dosyasına yazamadı (ayrıntı aracın günlüğünde).";

/** Araç nedeni bildirdiyse ekler (neden dosya içeriği taşımaz). */
export function apiAyariYazilamadi(neden: string | undefined): string {
  const n = (neden ?? "").trim();
  return n ? `${API_AYARI_YAZILAMADI} Neden: ${n}.` : API_AYARI_YAZILAMADI;
}

/** Komut paleti etiketleri: `window.top.apis.doAction` erişimi canlı doğrulanana kadar "deneysel". */
export const API_BASLAT_ETIKETI = "Tepsi: API sunucusunu başlat (deneysel)";
export const API_DURDUR_ETIKETI = "Tepsi: API sunucusunu durdur (deneysel)";
