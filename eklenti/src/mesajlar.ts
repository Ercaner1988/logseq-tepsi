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
  return `Tepsi aracı çalışıyor (sürüm ${d.surum}). Gizli pencere: ${d.gizli}. Logseq: ${logseq}.`;
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
