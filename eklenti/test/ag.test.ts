import { describe, expect, test } from "bun:test";
import {
  adresKur,
  ayarAl,
  ayarAyristir,
  ayarGonder,
  durumAl,
  durumAyristir,
  durumKoduSinifla,
  gizle,
  goster,
  hataSinifla,
  ISTEK_BASLIGI,
  portCoz,
  VARSAYILAN_PORT,
  type FetchIslevi,
} from "../src/ag";

function yanit(durum: number, govde = ""): FetchIslevi {
  return async () => new Response(durum === 204 ? null : govde, { status: durum });
}

function firlat(hata: Error): FetchIslevi {
  return async () => {
    throw hata;
  };
}

const DURUM_GOVDESI = "logseq-tepsi 0.1.0\ngizli=2\nlogseq=acik\n";

describe("portCoz", () => {
  test("geçerli değerleri alır", () => {
    expect(portCoz(47123)).toBe(47123);
    expect(portCoz("5000")).toBe(5000);
  });
  test("geçersizde varsayılana döner", () => {
    for (const k of [0, 70000, -1, 1.5, "abc", "", null, undefined, NaN]) {
      expect(portCoz(k)).toBe(VARSAYILAN_PORT);
    }
  });
});

describe("istek kurma", () => {
  test("adres 127.0.0.1 üzerindedir", () => {
    expect(adresKur(47123, "/durum")).toBe("http://127.0.0.1:47123/durum");
  });

  test("başlık, yöntem ve adres doğru gider", async () => {
    let girdi = "";
    let ayar: RequestInit = {};
    const f: FetchIslevi = async (g, a) => {
      girdi = g;
      ayar = a;
      return new Response(null, { status: 204 });
    };
    await gizle({ port: 5555, fetch: f });
    expect(girdi).toBe("http://127.0.0.1:5555/gizle");
    expect(ayar.method).toBe("POST");
    expect((ayar.headers as Record<string, string>)[ISTEK_BASLIGI]).toBe("1");
    await goster({ port: 5555, fetch: f });
    expect(girdi).toBe("http://127.0.0.1:5555/goster");
  });
});

describe("durumAyristir", () => {
  test("geçerli gövdeyi çözer (CRLF dahil)", () => {
    expect(durumAyristir(DURUM_GOVDESI)).toEqual({ surum: "0.1.0", gizli: 2, logseq: "acik" });
    expect(durumAyristir("logseq-tepsi 1.2.3\r\ngizli=0\r\nlogseq=kapali\r\n")).toEqual({
      surum: "1.2.3",
      gizli: 0,
      logseq: "kapali",
    });
  });
  test("bozuk gövdede null döner", () => {
    for (const g of [
      "",
      "baska-arac 1\ngizli=0\nlogseq=acik\n",
      "logseq-tepsi 0.1.0\nlogseq=acik\n",
      "logseq-tepsi 0.1.0\ngizli=x\nlogseq=acik\n",
      "logseq-tepsi 0.1.0\ngizli=1\nlogseq=belki\n",
    ]) {
      expect(durumAyristir(g)).toBeNull();
    }
  });
});

describe("hata sınıflama", () => {
  test("TypeError bağlantı reddi sayılır", () => {
    expect(hataSinifla(new TypeError("Failed to fetch"))).toEqual({ tur: "kapali" });
  });
  test("AbortError ve TimeoutError zaman aşımıdır", () => {
    const a = new Error("x");
    a.name = "AbortError";
    expect(hataSinifla(a).tur).toBe("zaman-asimi");
    const t = new Error("x");
    t.name = "TimeoutError";
    expect(hataSinifla(t).tur).toBe("zaman-asimi");
  });
  test("tanınmayan hata beklenmedik olur ve ayrıntı taşır", () => {
    expect(hataSinifla(new Error("garip"))).toEqual({ tur: "beklenmedik", ayrinti: "garip" });
    expect(hataSinifla("düz metin")).toEqual({ tur: "beklenmedik", ayrinti: "düz metin" });
  });
  test("HTTP durum kodları", () => {
    expect(durumKoduSinifla(200)).toBeNull();
    expect(durumKoduSinifla(204)).toBeNull();
    expect(durumKoduSinifla(403)).toEqual({ tur: "yasak", durum: 403 });
    expect(durumKoduSinifla(500)).toEqual({ tur: "beklenmedik", durum: 500 });
    expect(durumKoduSinifla(404)).toEqual({ tur: "beklenmedik", durum: 404 });
  });
});

describe("uçtan uca (sahte fetch)", () => {
  test("durum başarılı", async () => {
    const s = await durumAl({ fetch: yanit(200, DURUM_GOVDESI) });
    expect(s).toEqual({ ok: true, deger: { surum: "0.1.0", gizli: 2, logseq: "acik" } });
  });
  test("gizle 204 başarılı", async () => {
    expect(await gizle({ fetch: yanit(204) })).toEqual({ ok: true, deger: null });
  });
  test("bağlantı reddi: kapali", async () => {
    const s = await gizle({ fetch: firlat(new TypeError("Failed to fetch")) });
    expect(s).toEqual({ ok: false, hata: { tur: "kapali" } });
  });
  test("403: yasak", async () => {
    const s = await durumAl({ fetch: yanit(403, "yasak") });
    expect(s).toEqual({ ok: false, hata: { tur: "yasak", durum: 403 } });
  });
  test("500: beklenmedik", async () => {
    const s = await gizle({ fetch: yanit(500) });
    expect(s).toEqual({ ok: false, hata: { tur: "beklenmedik", durum: 500 } });
  });
  test("çözülemeyen /durum gövdesi: beklenmedik", async () => {
    const s = await durumAl({ fetch: yanit(200, "<html>") });
    expect(s.ok).toBe(false);
    if (!s.ok) expect(s.hata.tur).toBe("beklenmedik");
  });
  test("zaman aşımı: yanıt gelmezse istek iptal edilir", async () => {
    const asili: FetchIslevi = (_g, a) =>
      new Promise((_coz, red) => {
        a.signal?.addEventListener("abort", () => {
          const e = new Error("iptal");
          e.name = "AbortError";
          red(e);
        });
      });
    const s = await durumAl({ fetch: asili, zamanAsimiMs: 20 });
    expect(s).toEqual({ ok: false, hata: { tur: "zaman-asimi" } });
  });
});

describe("ayar ucu", () => {
  const AYAR_GOVDESI =
    "kucultme=tepsi\ntik=tek\nkapaliyken_tik_acar=evet\nkisayol_goster_gizle=Ctrl+Alt+L\n" +
    "oturumda_gizli_baslat=hayir\nbaslangicta_calis=hayir\n";

  test("ayarAyristir anahtar=değer çiftlerini ayırır (CRLF, boş değer)", () => {
    expect(ayarAyristir("a=1\r\nb=\r\nc=x=y\n")).toEqual({ a: "1", b: "", c: "x=y" });
    expect(ayarAyristir("")).toBeNull();
    expect(ayarAyristir("düz metin\n")).toBeNull();
  });

  test("ayarAl GET /ayar ister ve çözer", async () => {
    let girdi = "";
    let ayar: RequestInit = {};
    const f: FetchIslevi = async (g, a) => {
      girdi = g;
      ayar = a;
      return new Response(AYAR_GOVDESI, { status: 200 });
    };
    const s = await ayarAl({ port: 5555, fetch: f });
    expect(girdi).toBe("http://127.0.0.1:5555/ayar");
    expect(ayar.method).toBe("GET");
    expect((ayar.headers as Record<string, string>)[ISTEK_BASLIGI]).toBe("1");
    expect(s.ok && s.deger.kucultme).toBe("tepsi");
    expect(s.ok && s.deger.kisayol_goster_gizle).toBe("Ctrl+Alt+L");
    expect(s.ok && Object.keys(s.deger)).toHaveLength(6);
  });

  test("ayarAl çözülemeyen gövdede beklenmedik, araç kapalıysa kapali", async () => {
    const s = await ayarAl({ fetch: yanit(200, "<html>") });
    expect(s.ok).toBe(false);
    if (!s.ok) expect(s.hata.tur).toBe("beklenmedik");
    const k = await ayarAl({ fetch: firlat(new TypeError("Failed to fetch")) });
    expect(k).toEqual({ ok: false, hata: { tur: "kapali" } });
  });

  test("ayarGonder POST /ayar, gövde satırları + başlık", async () => {
    let girdi = "";
    let ayar: RequestInit = {};
    const f: FetchIslevi = async (g, a) => {
      girdi = g;
      ayar = a;
      return new Response(null, { status: 204 });
    };
    const s = await ayarGonder(["tik=cift", "kucultme=gorev_cubugu"], { port: 5555, fetch: f });
    expect(s).toEqual({ ok: true, deger: null });
    expect(girdi).toBe("http://127.0.0.1:5555/ayar");
    expect(ayar.method).toBe("POST");
    expect(ayar.body).toBe("tik=cift\nkucultme=gorev_cubugu\n");
    expect((ayar.headers as Record<string, string>)[ISTEK_BASLIGI]).toBe("1");
  });

  test("400 nedeni ayrıntı olarak taşınır; 403 ve 500 değişmez", async () => {
    const s = await ayarGonder(["tik=uc"], {
      fetch: yanit(400, "400 Bad Request\ntik: tek ya da cift olmalı: uc\n"),
    });
    expect(s).toEqual({
      ok: false,
      hata: { tur: "beklenmedik", durum: 400, ayrinti: "400 Bad Request\ntik: tek ya da cift olmalı: uc" },
    });
    const bos = await ayarGonder(["x=1"], { fetch: yanit(400, "") });
    expect(bos).toEqual({ ok: false, hata: { tur: "beklenmedik", durum: 400 } });
    expect(await ayarGonder(["x=1"], { fetch: yanit(403, "yasak") })).toEqual({
      ok: false,
      hata: { tur: "yasak", durum: 403 },
    });
  });
});
