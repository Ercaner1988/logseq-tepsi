// Kullanıcıya gösterilen iletiler (Türkçe + English). Metinlerin kendisi `metin.ts` tablosunda;
// burada etkin dile göre seçen ince işlevler var. Sabit dizge yok: dil çalışırken değişebilir.
import type { Durum, Hata } from "./ag";
import { m } from "./metin";

export const yonergeBaslatma = (): string => m().yonergeBaslatma;

export function hataMetni(hata: Hata): string {
  return m().hata(hata);
}

export function durumMetni(d: Durum): string {
  return m().durum(d);
}

export const baslatiliyor = (): string => m().baslatiliyor;

export function zatenCalisiyor(d: Durum): string {
  return m().zatenCalisiyor(d);
}

export function baslatildi(d: Durum): string {
  return m().baslatildi(d);
}

export const baslatilamadi = (): string => m().baslatilamadi;

/** Araç ayarı reddedince gövde `400 Bad Request\n<neden>` gelir; yalnız nedeni döndürür (araçtan geldiği dilde). */
export function redNedeni(ayrinti: string | undefined): string {
  const satirlar = (ayrinti ?? "").split(/\r?\n/).filter((s) => s.length > 0 && !/^400 /.test(s));
  return satirlar.join("; ");
}

export function ayarReddedildi(ayrinti: string | undefined): string {
  return m().ayarReddedildi(redNedeni(ayrinti));
}

export function ayarIletilemedi(hata: Hata): string {
  return m().ayarIletilemedi(hataMetni(hata));
}

export function notAcilamadi(neden: string): string {
  return m().notAcilamadi(neden);
}

export const notSayfaYok = (): string => m().notSayfaYok;
export const notBlokYok = (): string => m().notBlokYok;
export const notSayfaAdiYok = (): string => m().notSayfaAdiYok;
export const notSayfaOlusmadi = (): string => m().notSayfaOlusmadi;

export const apiIzinYok = (): string => m().apiIzinYok;

export function apiIstekGonderildi(baslat: boolean): string {
  return m().apiIstekGonderildi(baslat);
}

export function apiDurumu(acik: boolean, istenenAcik: boolean): string {
  return m().apiDurumu(acik, istenenAcik);
}

export const apiAyariBeklemede = (): string => m().apiAyariBeklemede;

/** Araç nedeni bildirdiyse ekler (neden dosya içeriği taşımaz). */
export function apiAyariYazilamadi(neden: string | undefined): string {
  const n = (neden ?? "").trim();
  const taban = m().apiAyariYazilamadi;
  return n ? m().apiAyariNedeni(taban, n) : taban;
}

/** Komut paleti etiketleri: `window.top.apis.doAction` erişimi canlı doğrulanana kadar "deneysel". */
export const apiBaslatEtiketi = (): string => m().apiBaslatEtiketi;
export const apiDurdurEtiketi = (): string => m().apiDurdurEtiketi;
