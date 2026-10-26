// Logseq API sunucusunu çalışırken başlat/durdur. Dış yol yok; eklenti yalnız Logseq'in kendi
// arayüzünün kullandığı `window.apis.doAction` kanalını, erişebilirse çağırır (belgesiz, sürüme
// bağlı, iframe yalıtımı yüzünden büyük olasılıkla erişilemez). Özellik algılama: yoksa ya da
// hata verirse kullanıcıya Logseq'in kendi sunucu menüsünü söyler; çökme yok.
// Araç, `logseq://...invokeCommand?action=plugin.logseq-tepsi.models.apiSunucusu&payload=baslat|durdur`
// adresiyle bu işlevi çağırır.
import { durumAl, type Durum, type Secenekler, type Sonuc } from "./ag";
import type { MesajTuru } from "./eylemler";
import { apiDurumu, apiIstekGonderildi, apiIzinYok } from "./mesajlar";

export type ApiEylemi = "baslat" | "durdur";

export function apiEylemiCoz(payload: unknown): ApiEylemi | null {
  return payload === "baslat" || payload === "durdur" ? payload : null;
}

/**
 * Logseq'in kendi `ipc :server/do` çağrısının JS dizisi biçimi (asar: main `decode-main-ipc-message`
 * dizi gelince `bean->clj`, `handle` ilk öğeyi `keyword` yapar; son öğe `js-obj` sonucu JS
 * nesnesi olarak döndürür). Başlat = `restart` (Logseq arayüzündeki başlat düğmesi de öyle).
 */
export function doActionDizisi(e: ApiEylemi): string[] {
  return ["server/do", e === "baslat" ? "restart" : "stop", "js-obj"];
}

export interface ApiOrtami {
  mesaj: (metin: string, tur: MesajTuru, sureMs?: number) => void;
  uyku: (ms: number) => Promise<void>;
  secenekler: () => Secenekler;
  /** `window.top.apis.doAction` yoksa tanımsızdır; erişim hatası fırlatabilir. */
  apiEylemi?: (dizi: string[]) => Promise<unknown>;
  ag?: { durumAl: (sec?: Secenekler) => Promise<Sonuc<Durum>> };
}

export type ApiSonucu = "gonderildi" | "izin-yok" | "gecersiz";

/** İstek gittikten sonra araca sunucunun durumunu sormadan önceki bekleme. */
export const DURUM_BEKLEME_MS = 1500;

export async function apiSunucusuEylemi(o: ApiOrtami, payload: unknown): Promise<ApiSonucu> {
  const eylem = apiEylemiCoz(payload);
  if (!eylem) return "gecersiz";
  if (!o.apiEylemi) {
    o.mesaj(apiIzinYok(), "warning", 10000);
    return "izin-yok";
  }
  try {
    await o.apiEylemi(doActionDizisi(eylem));
  } catch {
    o.mesaj(apiIzinYok(), "warning", 10000);
    return "izin-yok";
  }
  const baslat = eylem === "baslat";
  o.mesaj(apiIstekGonderildi(baslat), "success", 4000);
  await o.uyku(DURUM_BEKLEME_MS);
  const s = await (o.ag?.durumAl ?? durumAl)(o.secenekler());
  if (s.ok && s.deger.api !== undefined) {
    const acik = s.deger.api === "acik";
    o.mesaj(apiDurumu(acik, baslat), acik === baslat ? "success" : "warning", 6000);
  }
  return "gonderildi";
}
