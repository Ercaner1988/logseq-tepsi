// Kalınan sayfa: Logseq son açık sayfayı hatırlamaz (yalnız `:default-home`). Eklenti `onRouteChanged`
// ile son sayfayı kendi deposunda tutar; Logseq açılışında ayar açıksa ve kullanıcı henüz
// gezinmediyse, sayfa hâlâ varsa oraya gider. Mantık saf; Logseq `KalinanOrtami` ile verilir.

export interface Rota {
  path: string;
  template: string;
}

/** Yalnız `/page/:name` rotası sayılır; ad (ya da uuid) yüzde çözülür. Başka rota: null. */
export function rotadanSayfa(r: Rota): string | null {
  if (r.template !== "/page/:name") return null;
  const m = /^\/page\/([^?#]+)/.exec(r.path);
  if (!m) return null;
  try {
    const ad = decodeURIComponent(m[1]);
    return ad.trim() === "" ? null : ad;
  } catch {
    return null;
  }
}

/** Depoda tutulan kayıt: hangi grafta hangi sayfa. */
export interface Saklanan {
  graf: string | null;
  sayfa: string;
}

export function saklananMetni(s: Saklanan): string {
  return JSON.stringify(s);
}

/** Bozuk/eksik kayıt null döner (sessiz boş sonuç, çökme yok). */
export function saklananCoz(ham: string | null): Saklanan | null {
  if (!ham) return null;
  try {
    const v: unknown = JSON.parse(ham);
    if (typeof v !== "object" || v === null) return null;
    const { graf, sayfa } = v as { graf?: unknown; sayfa?: unknown };
    if (typeof sayfa !== "string" || sayfa === "") return null;
    return { graf: typeof graf === "string" ? graf : null, sayfa };
  } catch {
    return null;
  }
}

export type Karar = "git" | "ayar-kapali" | "gezildi" | "kayit-yok" | "baska-graf";

export interface KararGirdisi {
  /** `kalinanSayfa` eklenti ayarı. */
  ayar: boolean;
  /** Kullanıcı bu açılışta zaten bir sayfaya gitti mi. */
  gezildi: boolean;
  saklanan: Saklanan | null;
  /** Şu an açık grafın adı (bilinmiyorsa null). */
  graf: string | null;
}

/** Gidilip gidilmeyeceği; gitmiyorsa nedeni. */
export function kalinanKarar(g: KararGirdisi): Karar {
  if (!g.ayar) return "ayar-kapali";
  if (g.gezildi) return "gezildi";
  if (!g.saklanan) return "kayit-yok";
  if (g.saklanan.graf !== null && g.graf !== null && g.saklanan.graf !== g.graf) return "baska-graf";
  return "git";
}

/** Bir depo katmanı (Logseq eklenti deposu ya da iframe localStorage). */
export interface DepoKatmani {
  oku: () => Promise<string | null | undefined>;
  yaz: (deger: string) => Promise<void>;
}

/** Katmanları sırayla dener; hata fırlatan katman atlanır. Hiçbiri çalışmazsa null. */
export async function depodanOku(katmanlar: DepoKatmani[]): Promise<string | null> {
  for (const k of katmanlar) {
    try {
      const v = await k.oku();
      if (typeof v === "string" && v !== "") return v;
    } catch {
      // Bu katman kullanılamıyor (özel pencere, engelli depolama ...): sıradakine geç.
    }
  }
  return null;
}

/** İlk çalışan katmana yazar; hiçbiri çalışmazsa false (çökme yok). */
export async function depoyaYaz(katmanlar: DepoKatmani[], deger: string): Promise<boolean> {
  for (const k of katmanlar) {
    try {
      await k.yaz(deger);
      return true;
    } catch {
      // Sıradaki katmanı dene.
    }
  }
  return false;
}

export interface RotaApi {
  sayfaVarMi: (ad: string) => Promise<boolean>;
  git: (ad: string) => void;
  /** Açık grafın adı; graf henüz yüklenmediyse null. */
  grafAdi: () => Promise<string | null>;
}

export interface KalinanOrtami {
  depo: DepoKatmani[];
  rota: RotaApi;
  uyku: (ms: number) => Promise<void>;
}

/** İzleyicinin bellekte tuttuğu durum. */
export interface KalinanDurumu {
  gezildi: boolean;
  sonSayfa: string | null;
}

export function yeniDurum(): KalinanDurumu {
  return { gezildi: false, sonSayfa: null };
}

/** Logseq arayüzünün oturması için açılıştan sonra beklenecek süre. */
export const ACILIS_BEKLEME_MS = 1500;
/** Graf yüklenene kadar yoklama: en çok bu kadar deneme, aralıkla. */
export const GRAF_DENEME = 10;
export const GRAF_ARALIK_MS = 1000;

/** `onRouteChanged` işleyicisi: sayfa rotasıysa kullanıcı gezindi sayılır ve sayfa saklanır. */
export async function rotaDegisti(o: KalinanOrtami, d: KalinanDurumu, r: Rota): Promise<void> {
  const sayfa = rotadanSayfa(r);
  if (!sayfa) return;
  d.gezildi = true;
  if (d.sonSayfa === sayfa) return;
  d.sonSayfa = sayfa;
  let graf: string | null = null;
  try {
    graf = await o.rota.grafAdi();
  } catch {
    // Graf adı bilinmiyorsa kayıt grafsız tutulur.
  }
  await depoyaYaz(o.depo, saklananMetni({ graf, sayfa }));
}

async function grafBekle(o: KalinanOrtami): Promise<string | null> {
  for (let i = 0; i < GRAF_DENEME; i++) {
    try {
      const g = await o.rota.grafAdi();
      if (g) return g;
    } catch {
      // Henüz hazır değil.
    }
    await o.uyku(GRAF_ARALIK_MS);
  }
  return null;
}

export type DonusSonucu = Karar | "sayfa-yok" | "gidildi";

/**
 * Açılışta bir kez: ayar açıksa, kullanıcı gezinmediyse ve kayıtlı sayfa hâlâ varsa oraya gider.
 * Sayfa yoksa sessizce ana sayfada kalır (ileti yok). Beklemelerin (graf, depo, sayfa sorgusu)
 * herhangi birinde `gezildi` kurulursa (kullanıcı, anında not ya da başka kod) dönüş iptal edilir.
 */
export async function kalinanaDon(
  o: KalinanOrtami,
  d: KalinanDurumu,
  ayar: boolean,
): Promise<DonusSonucu> {
  if (!ayar) return "ayar-kapali";
  await o.uyku(ACILIS_BEKLEME_MS);
  const graf = await grafBekle(o);
  const saklanan = saklananCoz(await depodanOku(o.depo));
  const karar = kalinanKarar({ ayar, gezildi: d.gezildi, saklanan, graf });
  if (karar !== "git" || !saklanan) return karar;
  let varMi = false;
  try {
    varMi = await o.rota.sayfaVarMi(saklanan.sayfa);
  } catch {
    varMi = false;
  }
  if (!varMi) return "sayfa-yok";
  // Sorgu sürerken kullanıcı ya da başka kod (anında not dahil) gezindiyse dönüş iptal: yeni gezintiyi ezmez.
  if (d.gezildi) return "gezildi";
  o.rota.git(saklanan.sayfa);
  return "gidildi";
}
