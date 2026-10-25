import { describe, expect, test } from "bun:test";
import {
  ACILIS_BEKLEME_MS,
  depodanOku,
  depoyaYaz,
  GRAF_ARALIK_MS,
  GRAF_DENEME,
  kalinanaDon,
  kalinanKarar,
  rotaDegisti,
  rotadanSayfa,
  saklananCoz,
  saklananMetni,
  yeniDurum,
  type DepoKatmani,
  type KalinanOrtami,
} from "../src/kalinan";

describe("rotadanSayfa", () => {
  test("yalnız /page/:name rotası; ad yüzde çözülür, sorgu ve çapa atılır", () => {
    expect(rotadanSayfa({ path: "/page/Not%20Defteri", template: "/page/:name" })).toBe("Not Defteri");
    expect(rotadanSayfa({ path: "/page/abc-123?anchor=x", template: "/page/:name" })).toBe("abc-123");
    expect(rotadanSayfa({ path: "/page/%C3%87al%C4%B1%C5%9Fma", template: "/page/:name" })).toBe("Çalışma");
    expect(rotadanSayfa({ path: "/", template: "/" })).toBeNull();
    expect(rotadanSayfa({ path: "/all-journals", template: "/all-journals" })).toBeNull();
    expect(rotadanSayfa({ path: "/whiteboard/x", template: "/whiteboard/:name" })).toBeNull();
    expect(rotadanSayfa({ path: "/page/", template: "/page/:name" })).toBeNull();
    expect(rotadanSayfa({ path: "/page/%E0%A4%A", template: "/page/:name" })).toBeNull();
  });
});

describe("saklanan kayıt", () => {
  test("gidiş dönüş; bozuk kayıt null (çökme yok)", () => {
    const s = { graf: "ana", sayfa: "Not Defteri" };
    expect(saklananCoz(saklananMetni(s))).toEqual(s);
    expect(saklananCoz(null)).toBeNull();
    expect(saklananCoz("")).toBeNull();
    expect(saklananCoz("{bozuk")).toBeNull();
    expect(saklananCoz("[1]")).toBeNull();
    expect(saklananCoz('{"graf":"a"}')).toBeNull();
    expect(saklananCoz('{"sayfa":""}')).toBeNull();
    expect(saklananCoz('{"sayfa":"S","graf":3}')).toEqual({ graf: null, sayfa: "S" });
  });
});

describe("kalinanKarar", () => {
  const kayit = { graf: "ana", sayfa: "S" };
  test("karar tablosu", () => {
    const k = (g: Partial<Parameters<typeof kalinanKarar>[0]>) =>
      kalinanKarar({ ayar: true, gezildi: false, saklanan: kayit, graf: "ana", ...g });
    expect(k({})).toBe("git");
    expect(k({ ayar: false })).toBe("ayar-kapali");
    expect(k({ gezildi: true })).toBe("gezildi");
    expect(k({ saklanan: null })).toBe("kayit-yok");
    expect(k({ graf: "baska" })).toBe("baska-graf");
    // Graf bilinmiyorsa (ya da kayıt grafsızsa) engel olmaz.
    expect(k({ graf: null })).toBe("git");
    expect(k({ saklanan: { graf: null, sayfa: "S" }, graf: "baska" })).toBe("git");
  });
});

describe("depo katmanları", () => {
  const bozuk: DepoKatmani = {
    oku: async () => {
      throw new Error("erişim yok");
    },
    yaz: async () => {
      throw new Error("erişim yok");
    },
  };
  function bellek(baslangic?: string): DepoKatmani & { deger: () => string | undefined } {
    let v = baslangic;
    return { oku: async () => v, yaz: async (x) => void (v = x), deger: () => v };
  }

  test("okuma: hata fırlatan ve boş katman atlanır", async () => {
    expect(await depodanOku([bozuk, bellek(undefined), bellek("x")])).toBe("x");
    expect(await depodanOku([bozuk])).toBeNull();
    expect(await depodanOku([])).toBeNull();
  });

  test("yazma: ilk çalışan katmana yazar, hiçbiri çalışmazsa false", async () => {
    const b = bellek();
    expect(await depoyaYaz([bozuk, b], "v")).toBe(true);
    expect(b.deger()).toBe("v");
    expect(await depoyaYaz([bozuk], "v")).toBe(false);
  });
});

interface Kur {
  depoIcerik?: string;
  sayfalar?: string[];
  graf?: (string | null)[]; // her çağrıda sıradaki; son değer tekrarlanır
  grafFirlat?: boolean;
  varMiFirlat?: boolean;
}

function kur(a: Kur = {}) {
  const olay: string[] = [];
  let icerik: string | undefined = a.depoIcerik;
  let grafCagri = 0;
  const o: KalinanOrtami = {
    depo: [
      {
        oku: async () => icerik,
        yaz: async (v) => {
          olay.push("yaz");
          icerik = v;
        },
      },
    ],
    rota: {
      sayfaVarMi: async (ad) => {
        if (a.varMiFirlat) throw new Error("sorgu hatası");
        return (a.sayfalar ?? []).includes(ad);
      },
      git: (ad) => olay.push(`git:${ad}`),
      grafAdi: async () => {
        if (a.grafFirlat) throw new Error("graf yok");
        const dizi = a.graf ?? ["ana"];
        return dizi[Math.min(grafCagri++, dizi.length - 1)];
      },
    },
    uyku: async (ms) => {
      olay.push(`uyku:${ms}`);
    },
  };
  return { o, olay, icerik: () => icerik };
}

const KAYIT = saklananMetni({ graf: "ana", sayfa: "Not Defteri" });

describe("rotaDegisti", () => {
  test("sayfa rotası saklanır ve kullanıcı gezindi sayılır; aynı sayfa yeniden yazılmaz", async () => {
    const { o, olay, icerik } = kur();
    const d = yeniDurum();
    await rotaDegisti(o, d, { path: "/page/Not%20Defteri", template: "/page/:name" });
    expect(d.gezildi).toBe(true);
    expect(saklananCoz(icerik() ?? null)).toEqual({ graf: "ana", sayfa: "Not Defteri" });
    await rotaDegisti(o, d, { path: "/page/Not%20Defteri", template: "/page/:name" });
    expect(olay.filter((x) => x === "yaz")).toHaveLength(1);
    await rotaDegisti(o, d, { path: "/page/Baska", template: "/page/:name" });
    expect(saklananCoz(icerik() ?? null)?.sayfa).toBe("Baska");
  });

  test("sayfa olmayan rota (ana sayfa, günlükler) ne saklar ne gezildi sayar", async () => {
    const { o, olay } = kur();
    const d = yeniDurum();
    await rotaDegisti(o, d, { path: "/", template: "/" });
    expect(d.gezildi).toBe(false);
    expect(olay).toEqual([]);
  });

  test("graf adı alınamazsa kayıt grafsız tutulur", async () => {
    const { o, icerik } = kur({ grafFirlat: true });
    await rotaDegisti(o, yeniDurum(), { path: "/page/S", template: "/page/:name" });
    expect(saklananCoz(icerik() ?? null)).toEqual({ graf: null, sayfa: "S" });
  });
});

describe("kalinanaDon", () => {
  test("ayar kapalıysa hiçbir şey yapmaz (beklemez bile)", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"] });
    expect(await kalinanaDon(o, yeniDurum(), false)).toBe("ayar-kapali");
    expect(olay).toEqual([]);
  });

  test("kayıtlı sayfa var ve kullanıcı gezinmedi: sayfaya gider", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"] });
    expect(await kalinanaDon(o, yeniDurum(), true)).toBe("gidildi");
    expect(olay).toEqual([`uyku:${ACILIS_BEKLEME_MS}`, "git:Not Defteri"]);
  });

  test("kullanıcı zaten gezindiyse dokunmaz", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"] });
    const d = yeniDurum();
    d.gezildi = true;
    expect(await kalinanaDon(o, d, true)).toBe("gezildi");
    expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
  });

  test("sayfa sorgusu sürerken gezinme (kullanıcı ya da anında not) olursa dönüş iptal edilir", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"] });
    const d = yeniDurum();
    const sorgu = o.rota.sayfaVarMi;
    o.rota.sayfaVarMi = async (ad) => {
      d.gezildi = true; // sorgu sürerken anında not başladı / kullanıcı başka sayfaya gitti
      return sorgu(ad);
    };
    expect(await kalinanaDon(o, d, true)).toBe("gezildi");
    expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
  });

  test("sayfa silinmişse sessizce ana sayfada kalır", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: [] });
    expect(await kalinanaDon(o, yeniDurum(), true)).toBe("sayfa-yok");
    expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
  });

  test("sayfa sorgusu hata verirse de sessiz (git yok)", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, varMiFirlat: true });
    expect(await kalinanaDon(o, yeniDurum(), true)).toBe("sayfa-yok");
    expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
  });

  test("kayıt yok, bozuk kayıt ve başka graf: gidilmez", async () => {
    for (const [icerik, beklenen] of [
      [undefined, "kayit-yok"],
      ["{bozuk", "kayit-yok"],
      [saklananMetni({ graf: "baska", sayfa: "Not Defteri" }), "baska-graf"],
    ] as const) {
      const { o, olay } = kur({ depoIcerik: icerik, sayfalar: ["Not Defteri"] });
      expect(await kalinanaDon(o, yeniDurum(), true)).toBe(beklenen);
      expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
    }
  });

  test("graf yüklenene kadar yoklar, sonra gider", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"], graf: [null, null, "ana"] });
    expect(await kalinanaDon(o, yeniDurum(), true)).toBe("gidildi");
    expect(olay.filter((x) => x === `uyku:${GRAF_ARALIK_MS}`)).toHaveLength(2);
  });

  test("graf hiç gelmezse tavanlı yoklama biter; graf bilinmiyorsa kayıtlı sayfaya yine gidilir", async () => {
    const { o, olay } = kur({ depoIcerik: KAYIT, sayfalar: ["Not Defteri"], graf: [null] });
    expect(await kalinanaDon(o, yeniDurum(), true)).toBe("gidildi");
    expect(olay.filter((x) => x === `uyku:${GRAF_ARALIK_MS}`)).toHaveLength(GRAF_DENEME);
  });
});
