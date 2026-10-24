// Kullanıcı eylemlerinin mantığı. Logseq'e bağımlılıklar `Ortam` ile dışarıdan verilir.
import {
  BASLAT_ADRESI,
  durumAl,
  gizle,
  type Durum,
  type Secenekler,
  type Sonuc,
} from "./ag";
import {
  BASLATILAMADI,
  BASLATILIYOR,
  baslatildi,
  durumMetni,
  hataMetni,
  zatenCalisiyor,
} from "./mesajlar";

export type MesajTuru = "success" | "warning" | "error";

export interface AgApi {
  durumAl: (sec?: Secenekler) => Promise<Sonuc<Durum>>;
  gizle: (sec?: Secenekler) => Promise<Sonuc<null>>;
}

export interface Ortam {
  mesaj: (metin: string, tur: MesajTuru, sureMs?: number) => void;
  adresAc: (url: string) => void;
  uyku: (ms: number) => Promise<void>;
  secenekler: () => Secenekler;
  ag?: AgApi;
}

const GERCEK_AG: AgApi = { durumAl, gizle };

function ag(o: Ortam): AgApi {
  return o.ag ?? GERCEK_AG;
}

export async function gizleEylemi(o: Ortam): Promise<void> {
  const s = await ag(o).gizle(o.secenekler());
  if (!s.ok) o.mesaj(hataMetni(s.hata), "warning", 8000);
}

export async function durumEylemi(o: Ortam): Promise<void> {
  const s = await ag(o).durumAl(o.secenekler());
  if (s.ok) o.mesaj(durumMetni(s.deger), "success", 5000);
  else o.mesaj(hataMetni(s.hata), "warning", 8000);
}

/** Eklenti yüklenince bir kez çalışır; yalnız araç yoksa tek uyarı verir. */
export async function acilisKontrolu(o: Ortam, uyariAcik: boolean): Promise<void> {
  if (!uyariAcik) return;
  const s = await ag(o).durumAl(o.secenekler());
  if (!s.ok) o.mesaj(hataMetni(s.hata), "warning", 10000);
}

export type BaslatSonucu = "zaten-calisiyor" | "baslatildi" | "baslatilamadi" | "hata";

export interface BaslatAyari {
  deneme?: number;
  aralikMs?: number;
}

export async function baslatEylemi(o: Ortam, ayar: BaslatAyari = {}): Promise<BaslatSonucu> {
  const deneme = ayar.deneme ?? 8;
  const aralik = ayar.aralikMs ?? 1000;
  const once = await ag(o).durumAl(o.secenekler());
  if (once.ok) {
    o.mesaj(zatenCalisiyor(once.deger), "success", 4000);
    return "zaten-calisiyor";
  }
  // Araç çalışıyor ama istek reddedildi ya da çözülemedi: yeniden başlatmak çözüm değil.
  if (once.hata.tur === "yasak" || once.hata.tur === "beklenmedik") {
    o.mesaj(hataMetni(once.hata), "warning", 8000);
    return "hata";
  }
  o.mesaj(BASLATILIYOR, "success", 6000);
  o.adresAc(BASLAT_ADRESI);
  for (let i = 0; i < deneme; i++) {
    await o.uyku(aralik);
    const s = await ag(o).durumAl(o.secenekler());
    if (s.ok) {
      o.mesaj(baslatildi(s.deger), "success", 5000);
      return "baslatildi";
    }
  }
  o.mesaj(BASLATILAMADI, "warning", 12000);
  return "baslatilamadi";
}
