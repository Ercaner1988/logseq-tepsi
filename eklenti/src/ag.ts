// Saf ağ katmanı: istek kurma, yanıt ayrıştırma, hata sınıflama.
// Logseq'e bağımlı değildir; fetch dışarıdan verilebilir (sınama için).

export const VARSAYILAN_PORT = 47123;
export const ISTEK_BASLIGI = "X-Logseq-Tepsi";
export const ZAMAN_ASIMI_MS = 3000;
export const BASLAT_ADRESI = "logseq-tepsi://baslat";

export type HataTuru = "kapali" | "yasak" | "zaman-asimi" | "beklenmedik";

export interface Hata {
  tur: HataTuru;
  /** HTTP durum kodu (yalnız yanıt alındıysa). */
  durum?: number;
  ayrinti?: string;
}

export type Sonuc<T> = { ok: true; deger: T } | { ok: false; hata: Hata };

export interface Durum {
  surum: string;
  gizli: number;
  logseq: "acik" | "kapali";
  /** Logseq API sunucusu (yalnız araç `api=` satırı gönderiyorsa). */
  api?: "acik" | "kapali";
}

export type FetchIslevi = (girdi: string, ayar: RequestInit) => Promise<Response>;

export interface Secenekler {
  port?: number;
  fetch?: FetchIslevi;
  zamanAsimiMs?: number;
}

/** Ayardan gelen değeri 1-65535 arası tamsayı porta çevirir; olmazsa varsayılan. */
export function portCoz(deger: unknown): number {
  const sayi = typeof deger === "string" && deger.trim() !== "" ? Number(deger) : deger;
  if (typeof sayi === "number" && Number.isInteger(sayi) && sayi >= 1 && sayi <= 65535) {
    return sayi;
  }
  return VARSAYILAN_PORT;
}

export function adresKur(port: number, yol: string): string {
  return `http://127.0.0.1:${port}${yol}`;
}

/** `/durum` gövdesini ayrıştırır; biçim bozuksa null. */
export function durumAyristir(metin: string): Durum | null {
  const satirlar = metin.split(/\r?\n/).filter((s) => s.length > 0);
  const ilk = /^logseq-tepsi (\S+)$/.exec(satirlar[0] ?? "");
  if (!ilk) return null;
  const alanlar = new Map<string, string>();
  for (const s of satirlar.slice(1)) {
    const i = s.indexOf("=");
    if (i > 0) alanlar.set(s.slice(0, i), s.slice(i + 1));
  }
  const gizliMetin = alanlar.get("gizli");
  const logseq = alanlar.get("logseq");
  if (gizliMetin === undefined || !/^\d+$/.test(gizliMetin)) return null;
  if (logseq !== "acik" && logseq !== "kapali") return null;
  const api = alanlar.get("api");
  return {
    surum: ilk[1],
    gizli: Number(gizliMetin),
    logseq,
    ...(api === "acik" || api === "kapali" ? { api } : {}),
  };
}

/** fetch'in fırlattığı hatayı sınıflar. */
export function hataSinifla(hata: unknown): Hata {
  const ad =
    typeof hata === "object" && hata !== null
      ? String((hata as { name?: unknown }).name ?? "")
      : "";
  if (ad === "AbortError" || ad === "TimeoutError") return { tur: "zaman-asimi" };
  // Bağlantı reddedilince tarayıcı fetch'i TypeError ("Failed to fetch") fırlatır.
  // CORS ön denetiminin reddi de aynı görünür; ayırt edilemez.
  if (ad === "TypeError") return { tur: "kapali" };
  const ayrinti = hata instanceof Error ? hata.message : String(hata);
  return { tur: "beklenmedik", ayrinti };
}

/** HTTP durum kodunu sınıflar; başarıysa null. */
export function durumKoduSinifla(durum: number): Hata | null {
  if (durum >= 200 && durum < 300) return null;
  if (durum === 403) return { tur: "yasak", durum };
  return { tur: "beklenmedik", durum };
}

interface Yanit {
  durum: number;
  govde: string;
}

async function istek(
  yontem: "GET" | "POST",
  yol: string,
  sec: Secenekler = {},
  govde?: string,
): Promise<Sonuc<Yanit>> {
  const f: FetchIslevi = sec.fetch ?? ((g, a) => globalThis.fetch(g, a));
  const port = sec.port ?? VARSAYILAN_PORT;
  const denetci = new AbortController();
  const sayac = setTimeout(() => denetci.abort(), sec.zamanAsimiMs ?? ZAMAN_ASIMI_MS);
  try {
    const yanit = await f(adresKur(port, yol), {
      method: yontem,
      headers: { [ISTEK_BASLIGI]: "1" },
      // Gövde düz metin: tarayıcı Content-Type'ı text/plain yapar (CORS ön denetimi başlık
      // izniyle yetinir, araç başka başlık istemez).
      ...(govde !== undefined ? { body: govde } : {}),
      signal: denetci.signal,
      cache: "no-store",
    });
    const hata = durumKoduSinifla(yanit.status);
    if (hata) {
      // 400: araç nedenini gövdede söyler (örn. geçersiz kısayol); kullanıcıya gösterilir.
      const neden = yanit.status === 400 ? (await yanit.text()).trim() : "";
      return { ok: false, hata: neden ? { ...hata, ayrinti: neden } : hata };
    }
    const yanitGovdesi = yontem === "GET" ? await yanit.text() : "";
    return { ok: true, deger: { durum: yanit.status, govde: yanitGovdesi } };
  } catch (e) {
    return { ok: false, hata: hataSinifla(e) };
  } finally {
    clearTimeout(sayac);
  }
}

export async function durumAl(sec?: Secenekler): Promise<Sonuc<Durum>> {
  const s = await istek("GET", "/durum", sec);
  if (!s.ok) return s;
  const durum = durumAyristir(s.deger.govde);
  if (!durum) {
    return { ok: false, hata: { tur: "beklenmedik", ayrinti: "/durum yanıtı çözülemedi" } };
  }
  return { ok: true, deger: durum };
}

export async function gizle(sec?: Secenekler): Promise<Sonuc<null>> {
  const s = await istek("POST", "/gizle", sec);
  return s.ok ? { ok: true, deger: null } : s;
}

export async function goster(sec?: Secenekler): Promise<Sonuc<null>> {
  const s = await istek("POST", "/goster", sec);
  return s.ok ? { ok: true, deger: null } : s;
}

/** `GET /ayar` gövdesini `anahtar=değer` çiftlerine ayırır; hiç çift yoksa null. */
export function ayarAyristir(metin: string): Record<string, string> | null {
  const sonuc: Record<string, string> = {};
  for (const satir of metin.split(/\r?\n/)) {
    const i = satir.indexOf("=");
    if (i > 0) sonuc[satir.slice(0, i).trim()] = satir.slice(i + 1).trim();
  }
  return Object.keys(sonuc).length > 0 ? sonuc : null;
}

export async function ayarAl(sec?: Secenekler): Promise<Sonuc<Record<string, string>>> {
  const s = await istek("GET", "/ayar", sec);
  if (!s.ok) return s;
  const ayar = ayarAyristir(s.deger.govde);
  if (!ayar) {
    return { ok: false, hata: { tur: "beklenmedik", ayrinti: "/ayar yanıtı çözülemedi" } };
  }
  return { ok: true, deger: ayar };
}

/** `POST /ayar`: satırlar `anahtar=değer` biçiminde; geçersizse araç 400 + neden döner. */
export async function ayarGonder(satirlar: string[], sec?: Secenekler): Promise<Sonuc<null>> {
  const s = await istek("POST", "/ayar", sec, satirlar.join("\n") + "\n");
  return s.ok ? { ok: true, deger: null } : s;
}
