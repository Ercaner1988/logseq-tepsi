import { describe, expect, test } from "bun:test";
import type { Durum, Sonuc } from "../src/ag";
import { durumAyristir } from "../src/ag";
import { apiBaslatEtiketi, apiDurdurEtiketi, durumMetni } from "../src/mesajlar";
import {
  apiEylemiCoz,
  apiSunucusuEylemi,
  doActionDizisi,
  DURUM_BEKLEME_MS,
  type ApiOrtami,
} from "../src/sunucu";

function durum(api?: "acik" | "kapali"): Sonuc<Durum> {
  return { ok: true, deger: { surum: "0.1.0", gizli: 0, logseq: "acik", ...(api ? { api } : {}) } };
}

function kur(a: {
  eylem?: ApiOrtami["apiEylemi"] | "yok";
  durum?: Sonuc<Durum>;
}) {
  const mesajlar: { metin: string; tur: string }[] = [];
  const olay: string[] = [];
  const o: ApiOrtami = {
    mesaj: (metin, tur) => mesajlar.push({ metin, tur }),
    uyku: async (ms) => void olay.push(`uyku:${ms}`),
    secenekler: () => ({}),
    ...(a.eylem === "yok"
      ? {}
      : {
          apiEylemi:
            a.eylem ??
            (async (dizi: string[]) => {
              olay.push(`doAction:${JSON.stringify(dizi)}`);
              return null;
            }),
        }),
    ag: {
      durumAl: async () => {
        olay.push("durumAl");
        return a.durum ?? durum();
      },
    },
  };
  return { o, mesajlar, olay };
}

describe("apiEylemiCoz ve doAction dizisi", () => {
  test("yalnız baslat/durdur kabul edilir", () => {
    expect(apiEylemiCoz("baslat")).toBe("baslat");
    expect(apiEylemiCoz("durdur")).toBe("durdur");
    for (const x of ["", "restart", "BASLAT", undefined, null, 1, { type: "click" }]) {
      expect(apiEylemiCoz(x)).toBeNull();
    }
  });

  test("Logseq'in server/do çağrısının JS dizisi biçimi", () => {
    expect(doActionDizisi("baslat")).toEqual(["server/do", "restart", "js-obj"]);
    expect(doActionDizisi("durdur")).toEqual(["server/do", "stop", "js-obj"]);
  });
});

describe("apiSunucusuEylemi", () => {
  test("doAction yoksa özellik algılama: uyarı, çökme yok", async () => {
    const { o, mesajlar, olay } = kur({ eylem: "yok" });
    expect(await apiSunucusuEylemi(o, "baslat")).toBe("izin-yok");
    expect(mesajlar).toHaveLength(1);
    expect(mesajlar[0].tur).toBe("warning");
    expect(mesajlar[0].metin).toContain("Logseq bu sürümde izin vermiyor");
    expect(mesajlar[0].metin).toContain("kendi sunucu menüsünü");
    expect(olay).toEqual([]);
  });

  test("doAction hata fırlatırsa (iframe yalıtımı, sürüm) aynı uyarı", async () => {
    const { o, mesajlar } = kur({
      eylem: async () => {
        throw new Error("Blocked a frame with origin");
      },
    });
    expect(await apiSunucusuEylemi(o, "durdur")).toBe("izin-yok");
    expect(mesajlar).toHaveLength(1);
    expect(mesajlar[0].metin).toContain("Logseq bu sürümde izin vermiyor");
  });

  test("başlat: doğru dizi gider, sonra araçtan durum sorulur, çalışıyorsa başarı", async () => {
    const { o, mesajlar, olay } = kur({ durum: durum("acik") });
    expect(await apiSunucusuEylemi(o, "baslat")).toBe("gonderildi");
    expect(olay).toEqual([
      'doAction:["server/do","restart","js-obj"]',
      `uyku:${DURUM_BEKLEME_MS}`,
      "durumAl",
    ]);
    expect(mesajlar.map((m) => m.tur)).toEqual(["success", "success"]);
    expect(mesajlar[1].metin).toBe("API sunucusu çalışıyor.");
  });

  test("durdur: sunucu hâlâ açık görünürse uyarır", async () => {
    const { o, mesajlar, olay } = kur({ durum: durum("acik") });
    expect(await apiSunucusuEylemi(o, "durdur")).toBe("gonderildi");
    expect(olay[0]).toBe('doAction:["server/do","stop","js-obj"]');
    expect(mesajlar[1].tur).toBe("warning");
    expect(mesajlar[1].metin).toContain("henüz çalışıyor");
  });

  test("araç api satırı göndermiyorsa ya da kapalıysa yalnız istek iletisi", async () => {
    for (const d of [durum(), { ok: false, hata: { tur: "kapali" } } as Sonuc<Durum>]) {
      const { o, mesajlar } = kur({ durum: d });
      expect(await apiSunucusuEylemi(o, "baslat")).toBe("gonderildi");
      expect(mesajlar).toHaveLength(1);
      expect(mesajlar[0].metin).toContain("başlatma isteği");
    }
  });

  test("geçersiz payload: hiçbir şey çağrılmaz", async () => {
    const { o, mesajlar, olay } = kur({});
    expect(await apiSunucusuEylemi(o, "belki")).toBe("gecersiz");
    expect(await apiSunucusuEylemi(o, undefined)).toBe("gecersiz");
    expect(olay).toEqual([]);
    expect(mesajlar).toEqual([]);
  });
});

describe("/durum api satırı", () => {
  test("durumAyristir api satırını taşır, bozuk değeri yok sayar, eskisiyle uyumlu", () => {
    const g = "logseq-tepsi 0.1.0\ngizli=1\nlogseq=acik\n";
    expect(durumAyristir(g)).toEqual({ surum: "0.1.0", gizli: 1, logseq: "acik" });
    expect(durumAyristir(g + "api=acik\n")).toEqual({ surum: "0.1.0", gizli: 1, logseq: "acik", api: "acik" });
    expect(durumAyristir(g + "api=kapali\n")?.api).toBe("kapali");
    expect(durumAyristir(g + "api=belki\n")).toEqual({ surum: "0.1.0", gizli: 1, logseq: "acik" });
  });

  test("durum iletisi api bilgisini yalnız varsa ekler", () => {
    const d: Durum = { surum: "0.1.0", gizli: 0, logseq: "acik" };
    expect(durumMetni(d)).toBe("Tepsi aracı çalışıyor (sürüm 0.1.0). Gizli pencere: 0. Logseq: açık.");
    expect(durumMetni({ ...d, api: "acik" })).toContain("API sunucusu: çalışıyor.");
    expect(durumMetni({ ...d, api: "kapali" })).toContain("API sunucusu: kapalı.");
  });
});

describe("komut paleti etiketleri", () => {
  test("API başlat/durdur canlı doğrulanana kadar (deneysel) etiketli", () => {
    expect(apiBaslatEtiketi()).toContain("(deneysel)");
    expect(apiDurdurEtiketi()).toContain("(deneysel)");
    expect(apiBaslatEtiketi()).toContain("başlat");
    expect(apiDurdurEtiketi()).toContain("durdur");
  });
});
