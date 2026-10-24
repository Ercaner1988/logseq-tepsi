import { describe, expect, test } from "bun:test";
import type { Durum, Hata, Sonuc } from "../src/ag";
import { acilisKontrolu, baslatEylemi, durumEylemi, gizleEylemi, type Ortam } from "../src/eylemler";

const DURUM: Durum = { surum: "0.1.0", gizli: 1, logseq: "acik" };

function ok<T>(deger: T): Sonuc<T> {
  return { ok: true, deger };
}
function hata(h: Hata): Sonuc<never> {
  return { ok: false, hata: h };
}

function kur(durumlar: Sonuc<Durum>[], gizleSonucu: Sonuc<null> = ok(null)) {
  const kayit = { mesajlar: [] as { metin: string; tur: string }[], acilan: [] as string[], uyku: 0, durumCagri: 0 };
  const o: Ortam = {
    mesaj: (metin, tur) => kayit.mesajlar.push({ metin, tur }),
    adresAc: (u) => kayit.acilan.push(u),
    uyku: async () => {
      kayit.uyku++;
    },
    secenekler: () => ({}),
    ag: {
      durumAl: async () => {
        const s = durumlar[Math.min(kayit.durumCagri, durumlar.length - 1)];
        kayit.durumCagri++;
        return s;
      },
      gizle: async () => gizleSonucu,
    },
  };
  return { o, kayit };
}

describe("gizleEylemi", () => {
  test("başarıda mesaj göstermez", async () => {
    const { o, kayit } = kur([ok(DURUM)]);
    await gizleEylemi(o);
    expect(kayit.mesajlar).toEqual([]);
  });
  test("araç kapalıysa Türkçe uyarı verir", async () => {
    const { o, kayit } = kur([], hata({ tur: "kapali" }));
    await gizleEylemi(o);
    expect(kayit.mesajlar).toHaveLength(1);
    expect(kayit.mesajlar[0].metin).toContain("Tepsi aracı çalışmıyor");
    expect(kayit.mesajlar[0].metin).toContain("Tepsi: aracı başlat");
    expect(kayit.mesajlar[0].tur).toBe("warning");
  });
  test("403 ayrı metinle bildirilir", async () => {
    const { o, kayit } = kur([], hata({ tur: "yasak", durum: 403 }));
    await gizleEylemi(o);
    expect(kayit.mesajlar[0].metin).toContain("403");
  });
  test("zaman aşımı ayrı metinle bildirilir", async () => {
    const { o, kayit } = kur([], hata({ tur: "zaman-asimi" }));
    await gizleEylemi(o);
    expect(kayit.mesajlar[0].metin).toContain("zamanında yanıt vermedi");
  });
});

describe("durumEylemi", () => {
  test("sürümü ve gizli sayısını bildirir", async () => {
    const { o, kayit } = kur([ok(DURUM)]);
    await durumEylemi(o);
    expect(kayit.mesajlar[0].metin).toBe(
      "Tepsi aracı çalışıyor (sürüm 0.1.0). Gizli pencere: 1. Logseq: açık.",
    );
  });
  test("araç yoksa uyarır", async () => {
    const { o, kayit } = kur([hata({ tur: "kapali" })]);
    await durumEylemi(o);
    expect(kayit.mesajlar[0].tur).toBe("warning");
  });
});

describe("acilisKontrolu", () => {
  test("araç varsa sessizdir", async () => {
    const { o, kayit } = kur([ok(DURUM)]);
    await acilisKontrolu(o, true);
    expect(kayit.mesajlar).toEqual([]);
  });
  test("araç yoksa tek uyarı verir", async () => {
    const { o, kayit } = kur([hata({ tur: "kapali" })]);
    await acilisKontrolu(o, true);
    expect(kayit.mesajlar).toHaveLength(1);
  });
  test("ayarda kapalıysa istek bile atmaz", async () => {
    const { o, kayit } = kur([hata({ tur: "kapali" })]);
    await acilisKontrolu(o, false);
    expect(kayit.mesajlar).toEqual([]);
    expect(kayit.durumCagri).toBe(0);
  });
});

describe("baslatEylemi", () => {
  test("araç zaten çalışıyorsa adres açmaz", async () => {
    const { o, kayit } = kur([ok(DURUM)]);
    expect(await baslatEylemi(o)).toBe("zaten-calisiyor");
    expect(kayit.acilan).toEqual([]);
  });
  test("403 varsa yeniden başlatmaya kalkmaz", async () => {
    const { o, kayit } = kur([hata({ tur: "yasak", durum: 403 })]);
    expect(await baslatEylemi(o)).toBe("hata");
    expect(kayit.acilan).toEqual([]);
  });
  test("şemayı açar, sonra durumu bekler ve başarıyı bildirir", async () => {
    const { o, kayit } = kur([hata({ tur: "kapali" }), hata({ tur: "kapali" }), ok(DURUM)]);
    expect(await baslatEylemi(o, { aralikMs: 1 })).toBe("baslatildi");
    expect(kayit.acilan).toEqual(["logseq-tepsi://baslat"]);
    expect(kayit.uyku).toBe(2);
    expect(kayit.mesajlar.at(-1)?.metin).toContain("başlatıldı");
  });
  test("deneme biterse yönerge verir", async () => {
    const { o, kayit } = kur([hata({ tur: "kapali" })]);
    expect(await baslatEylemi(o, { deneme: 3, aralikMs: 1 })).toBe("baslatilamadi");
    expect(kayit.uyku).toBe(3);
    expect(kayit.mesajlar.at(-1)?.metin).toContain("logseq-tepsi kur");
    expect(kayit.mesajlar.at(-1)?.tur).toBe("warning");
  });
});
