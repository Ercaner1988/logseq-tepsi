// Kullanıcı eylemlerinin mantığı. Logseq'e bağımlılıklar `Ortam` ile dışarıdan verilir.
import {
  ayarAl,
  ayarGonder,
  BASLAT_ADRESI,
  durumAl,
  gizle,
  type Durum,
  type Hata,
  type Secenekler,
  type Sonuc,
} from "./ag";
import {
  degisenSatirlar,
  eklentiFarki,
  satirlariIsle,
  type AracAyarlari,
  type EklentiAyarlari,
} from "./ayar";
import {
  apiAyariBeklemede,
  apiAyariYazilamadi,
  ayarIletilemedi,
  ayarReddedildi,
  baslatilamadi,
  baslatiliyor,
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

export interface AyarAgApi {
  ayarAl: (sec?: Secenekler) => Promise<Sonuc<Record<string, string>>>;
  /** Araç `api_otomatik_baslat` sonucunu gövdede verirse çözülmüş çiftler, yoksa `null`. */
  ayarGonder: (
    satirlar: string[],
    sec?: Secenekler,
  ) => Promise<Sonuc<Record<string, string> | null>>;
}

export interface Ortam {
  mesaj: (metin: string, tur: MesajTuru, sureMs?: number) => void;
  adresAc: (url: string) => void;
  uyku: (ms: number) => Promise<void>;
  secenekler: () => Secenekler;
  ag?: AgApi;
  ayarAg?: AyarAgApi;
  /** Logseq eklenti ayarları (logseq.settings). */
  ayarlar?: () => Record<string, unknown> | undefined;
  /** Eklenti ayarlarını yazar (logseq.updateSettings). */
  ayarGuncelle?: (kismi: EklentiAyarlari) => void;
}

/** Aracın son bilinen ayarları (açılış eşitlemesi ya da başarılı POST sonrası); araç kapalıysa null. */
export interface AyarDeposu {
  bilinen: AracAyarlari | null;
}

const GERCEK_AYAR_AGI: AyarAgApi = { ayarAl, ayarGonder };

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
  o.mesaj(baslatiliyor(), "success", 6000);
  o.adresAc(BASLAT_ADRESI);
  for (let i = 0; i < deneme; i++) {
    await o.uyku(aralik);
    const s = await ag(o).durumAl(o.secenekler());
    if (s.ok) {
      o.mesaj(baslatildi(s.deger), "success", 5000);
      return "baslatildi";
    }
  }
  o.mesaj(baslatilamadi(), "warning", 12000);
  return "baslatilamadi";
}

/**
 * Açılışta: araçtaki ayarları okuyup eklenti ayarlarını onlara eşitler (araç tek kaynak). Araç
 * yoksa sessiz kalır (açılış uyarısı ayrıdır). Eşitleme yazdığı değerleri `bilinen`e koyar; böylece
 * ardından gelen "ayar değişti" olayı araca geri POST etmez.
 */
export async function ayarEsitle(o: Ortam, depo: AyarDeposu): Promise<void> {
  const s = await (o.ayarAg ?? GERCEK_AYAR_AGI).ayarAl(o.secenekler());
  if (!s.ok) return;
  depo.bilinen = s.deger;
  const fark = eklentiFarki(s.deger, o.ayarlar?.());
  if (Object.keys(fark).length > 0) o.ayarGuncelle?.(fark);
}

/**
 * Kullanıcı eklenti ayarını değiştirince: değişen ayarları araca POST eder. Araç kapalıysa ya da
 * ulaşılamazsa uyarır (sessiz kayıp yok); araç değeri reddederse nedenini söyler ve eklenti
 * ayarını araçtaki değere geri alır; zaman aşımı ve 503'te (sonuç belirsiz) de araçtaki gerçek
 * durumla eşitler.
 */
export async function ayarDegisti(
  o: Ortam,
  depo: AyarDeposu,
  yeni: Record<string, unknown> | undefined,
  eski: Record<string, unknown> | undefined,
): Promise<void> {
  const satirlar = degisenSatirlar(yeni, eski, depo.bilinen);
  if (satirlar.length === 0) return;
  const ag = o.ayarAg ?? GERCEK_AYAR_AGI;
  const s = await ag.ayarGonder(satirlar, o.secenekler());
  if (s.ok) {
    depo.bilinen = satirlariIsle(depo.bilinen, satirlar);
    if (satirlar.some((x) => x.startsWith("api_otomatik_baslat="))) {
      await apiAyariSonucu(o, depo, s.deger);
    }
    return;
  }
  if (s.hata.tur === "beklenmedik" && s.hata.durum === 400) {
    o.mesaj(ayarReddedildi(s.hata.ayrinti), "error", 10000);
    await ayarEsitle(o, depo);
    return;
  }
  o.mesaj(ayarIletilemedi(s.hata), "warning", 12000);
  // Zaman aşımında ve 503'te araç değişikliği uygulamış olabilir ya da uygulamamıştır (iptal
  // edilmiş olabilir): eklenti ayarını araçtaki gerçek durumla eşitle. Ulaşılamıyorsa (kapalı,
  // 403) eşitlenecek bir şey yok.
  if (durumBelirsiz(s.hata)) await ayarEsitle(o, depo);
}

/** POST sonucu belirsiz mi: zaman aşımı ya da 503 (araç ayarı uygulamış olabilir). */
function durumBelirsiz(h: Hata): boolean {
  return h.tur === "zaman-asimi" || (h.tur === "beklenmedik" && h.durum === 503);
}

/**
 * `api_otomatik_baslat` araca iletildikten sonra sonucu bildirir. Sonuç POST yanıtının gövdesinden
 * okunur (araç ayarı eşzamanlı uyguladı, ayrıca GET ile yarışılmaz): `beklemede` = Logseq açık,
 * Logseq kapanınca uygulanacak; `hata` = araç `configs.edn`e yazamadı (neden gövdede): uyarır ve
 * eklenti ayarını araçtaki gerçek değere geri alır; `yazildi` ya da gövde yok: ileti yok.
 */
async function apiAyariSonucu(
  o: Ortam,
  depo: AyarDeposu,
  sonuc: Record<string, string> | null,
): Promise<void> {
  const durum = sonuc?.api_otomatik_baslat;
  if (durum === "beklemede") {
    o.mesaj(apiAyariBeklemede(), "success", 8000);
    return;
  }
  if (durum !== "hata") return;
  o.mesaj(apiAyariYazilamadi(sonuc?.api_otomatik_neden), "warning", 10000);
  // Araç işi bitirdi (eşzamanlı yanıt): şimdi okunan değer araçtaki gerçek değerdir.
  const g = await (o.ayarAg ?? GERCEK_AYAR_AGI).ayarAl(o.secenekler());
  if (!g.ok) return;
  depo.bilinen = g.deger;
  const fark = eklentiFarki(g.deger, o.ayarlar?.());
  if (Object.keys(fark).length > 0) o.ayarGuncelle?.(fark);
}
