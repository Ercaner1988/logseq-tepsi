// Anında not: bugünün günlük sayfasının sonuna ya da yeni bir sayfaya boş blok açar, sayfaya gider,
// bloğu düzenlemeye açar. Logseq'e bağımlılık `DefterApi` ile dışarıdan verilir (sınanabilir).
// Araç, `logseq://x-callback-url/invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=<hedef>`
// adresiyle bu işlevi çağırır; Logseq içinden de komut paletinden çağrılır.
import { notAcilamadi, notBlokYok, notSayfaAdiYok, notSayfaOlusmadi, notSayfaYok } from "./mesajlar";
import type { MesajTuru } from "./eylemler";

export type NotHedefi = "gunluk" | "yeni_sayfa";

/** Sayfa kimliği: `uuid` varsa Logseq onunla bulur ve yönlendirir, yoksa ad. */
export interface SayfaKimligi {
  uuid?: string;
  ad: string;
}

export interface DefterApi {
  /** `YYYYMMDD` günlük sayfası (yoksa null). */
  gunlukSayfa: (gun: number) => Promise<SayfaKimligi | null>;
  /** `YYYY-MM-DD` günlük sayfasını oluşturur. */
  gunlukOlustur: (gun: string) => Promise<SayfaKimligi | null>;
  sayfaVarMi: (ad: string) => Promise<boolean>;
  sayfaOlustur: (ad: string) => Promise<SayfaKimligi | null>;
  /** Sayfanın sonuna boş blok ekler; blok uuid'i (olmazsa null). */
  bosBlokEkle: (sayfa: SayfaKimligi) => Promise<string | null>;
  sayfayaGit: (sayfa: SayfaKimligi) => void;
  blokDuzenle: (uuid: string) => Promise<void>;
}

export interface NotOrtami {
  mesaj: (metin: string, tur: MesajTuru, sureMs?: number) => void;
  uyku: (ms: number) => Promise<void>;
  /** Logseq eklenti ayarları (varsayılan hedef için). */
  ayarlar?: () => Record<string, unknown> | undefined;
  defter: DefterApi;
  /**
   * Not işlemi başlar başlamaz (ilk eşzamansız işten önce) çağrılır: açılıştaki "kalınan sayfaya
   * dön" bu notun sayfasına gezinmeyi ezmesin diye kullanıcı gezindi sayılır.
   */
  notBasladi?: () => void;
}

/** Sayfaya gittikten sonra düzenlemeyi açmadan önce yönlendirmenin oturmasını bekler. */
export const GIDIS_BEKLEME_MS = 300;
/** Aynı dakikada ikinci not için ad çakışmasında denenecek en çok sonek. */
export const EN_COK_SONEK = 50;

/** Araçtan gelen payload (`gunluk`, `yeni_sayfa`) ya da eklenti ayarı; ikisi de yoksa günlük. */
export function hedefCoz(payload: unknown, ayar: unknown): NotHedefi {
  for (const aday of [payload, ayar]) {
    if (aday === "gunluk" || aday === "yeni_sayfa") return aday;
  }
  return "gunluk";
}

function iki(n: number): string {
  return String(n).padStart(2, "0");
}

/** Yerel tarih: `YYYY-MM-DD`. */
export function gunMetni(d: Date): string {
  return `${d.getFullYear()}-${iki(d.getMonth() + 1)}-${iki(d.getDate())}`;
}

/** `YYYYMMDD` (Logseq `journal-day`). */
export function gunSayisi(d: Date): number {
  return Number(gunMetni(d).replaceAll("-", ""));
}

/** `Not YYYY-MM-DD HH:mm` (yerel saat). */
export function yeniSayfaAdi(d: Date): string {
  return `Not ${gunMetni(d)} ${iki(d.getHours())}:${iki(d.getMinutes())}`;
}

/** Ad doluysa sonuna ` 2`, ` 3` ... ekler; `EN_COK_SONEK` aşılırsa null. */
export async function benzersizAd(
  taban: string,
  varMi: (ad: string) => Promise<boolean>,
): Promise<string | null> {
  if (!(await varMi(taban))) return taban;
  for (let n = 2; n <= EN_COK_SONEK; n++) {
    const ad = `${taban} ${n}`;
    if (!(await varMi(ad))) return ad;
  }
  return null;
}

export type NotSonucu = "tamam" | "hata";

/**
 * Notu açar. `payload`: araçtan gelen hedef (geçersizse eklenti ayarı, o da yoksa günlük).
 * Hata olursa (etkin dilde) ileti gösterir ve "hata" döner; fırlatmaz.
 */
export async function anindaNotEylemi(
  o: NotOrtami,
  payload: unknown,
  simdi: Date = new Date(),
): Promise<NotSonucu> {
  o.notBasladi?.();
  const hedef = hedefCoz(payload, o.ayarlar?.()?.notHedefi);
  const d = o.defter;
  const hata = (neden: string): NotSonucu => {
    o.mesaj(notAcilamadi(neden), "error", 8000);
    return "hata";
  };
  try {
    let sayfa: SayfaKimligi | null;
    if (hedef === "gunluk") {
      sayfa = (await d.gunlukSayfa(gunSayisi(simdi))) ?? (await d.gunlukOlustur(gunMetni(simdi)));
      if (!sayfa) return hata(notSayfaYok());
    } else {
      const ad = await benzersizAd(yeniSayfaAdi(simdi), d.sayfaVarMi);
      if (!ad) return hata(notSayfaAdiYok());
      sayfa = await d.sayfaOlustur(ad);
      if (!sayfa) return hata(notSayfaOlusmadi());
    }
    const uuid = await d.bosBlokEkle(sayfa);
    if (!uuid) return hata(notBlokYok());
    d.sayfayaGit(sayfa);
    await o.uyku(GIDIS_BEKLEME_MS);
    await d.blokDuzenle(uuid);
    return "tamam";
  } catch (e) {
    return hata(e instanceof Error ? e.message : String(e));
  }
}
