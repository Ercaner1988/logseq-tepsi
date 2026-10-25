import { describe, expect, test } from "bun:test";
import {
  anindaNotEylemi,
  benzersizAd,
  EN_COK_SONEK,
  GIDIS_BEKLEME_MS,
  gunMetni,
  gunSayisi,
  hedefCoz,
  yeniSayfaAdi,
  type DefterApi,
  type NotOrtami,
  type SayfaKimligi,
} from "../src/not";

const SIMDI = new Date(2026, 9, 25, 14, 3, 40); // 25 Ekim 2026 14:03:40 (yerel)
const GUNLUK: SayfaKimligi = { uuid: "uuid-gunluk", ad: "Oct 25th, 2026" };

interface Ayar {
  gunluk?: SayfaKimligi | null;
  olusturulan?: SayfaKimligi | null;
  varOlanlar?: string[];
  yeniSayfa?: SayfaKimligi | null;
  blok?: string | null;
  firlat?: string;
  notHedefi?: unknown;
}

function kur(a: Ayar = {}) {
  const olay: string[] = [];
  const mesajlar: { metin: string; tur: string }[] = [];
  const defter: DefterApi = {
    gunlukSayfa: async (gun) => {
      olay.push(`gunlukSayfa:${gun}`);
      if (a.firlat) throw new Error(a.firlat);
      return a.gunluk === undefined ? GUNLUK : a.gunluk;
    },
    gunlukOlustur: async (gun) => {
      olay.push(`gunlukOlustur:${gun}`);
      return a.olusturulan === undefined ? GUNLUK : a.olusturulan;
    },
    sayfaVarMi: async (ad) => {
      olay.push(`sayfaVarMi:${ad}`);
      return (a.varOlanlar ?? []).includes(ad);
    },
    sayfaOlustur: async (ad) => {
      olay.push(`sayfaOlustur:${ad}`);
      return a.yeniSayfa === undefined ? { uuid: "uuid-yeni", ad } : a.yeniSayfa;
    },
    bosBlokEkle: async (s) => {
      olay.push(`bosBlokEkle:${s.ad}`);
      return a.blok === undefined ? "uuid-blok" : a.blok;
    },
    sayfayaGit: (s) => olay.push(`git:${s.ad}`),
    blokDuzenle: async (uuid) => {
      olay.push(`duzenle:${uuid}`);
    },
  };
  const o: NotOrtami = {
    mesaj: (metin, tur) => mesajlar.push({ metin, tur }),
    uyku: async (ms) => {
      olay.push(`uyku:${ms}`);
    },
    ayarlar: () => ({ notHedefi: a.notHedefi }),
    defter,
  };
  return { o, olay, mesajlar };
}

describe("hedefCoz", () => {
  test("geçerli payload önce, sonra eklenti ayarı, sonra günlük", () => {
    expect(hedefCoz("yeni_sayfa", "gunluk")).toBe("yeni_sayfa");
    expect(hedefCoz("gunluk", "yeni_sayfa")).toBe("gunluk");
    expect(hedefCoz(undefined, "yeni_sayfa")).toBe("yeni_sayfa");
    // Logseq arayüz tıklaması nesne payload gönderir: yok sayılır.
    expect(hedefCoz({ type: "click" }, "yeni_sayfa")).toBe("yeni_sayfa");
    expect(hedefCoz("bilinmeyen", 3)).toBe("gunluk");
    expect(hedefCoz(null, undefined)).toBe("gunluk");
  });
});

describe("ad ve tarih üretimi", () => {
  test("gün metni, sayısı ve yeni sayfa adı yerel saatle, sıfır dolgulu", () => {
    expect(gunMetni(SIMDI)).toBe("2026-10-25");
    expect(gunSayisi(SIMDI)).toBe(20261025);
    expect(yeniSayfaAdi(SIMDI)).toBe("Not 2026-10-25 14:03");
    const erken = new Date(2026, 0, 5, 7, 9);
    expect(gunMetni(erken)).toBe("2026-01-05");
    expect(gunSayisi(erken)).toBe(20260105);
    expect(yeniSayfaAdi(erken)).toBe("Not 2026-01-05 07:09");
  });

  test("benzersizAd: çakışırsa sonuna sayı, sınırı aşarsa null", async () => {
    const dolu = new Set<string>();
    const var_ = async (ad: string) => dolu.has(ad);
    expect(await benzersizAd("Not A", var_)).toBe("Not A");
    dolu.add("Not A");
    expect(await benzersizAd("Not A", var_)).toBe("Not A 2");
    dolu.add("Not A 2");
    dolu.add("Not A 3");
    expect(await benzersizAd("Not A", var_)).toBe("Not A 4");
    expect(await benzersizAd("Not A", async () => true)).toBeNull();
    expect(EN_COK_SONEK).toBe(50);
  });
});

describe("anindaNotEylemi: günlük", () => {
  test("var olan günlük sayfasına boş blok, sayfaya git, düzenle (bu sırayla)", async () => {
    const { o, olay, mesajlar } = kur();
    expect(await anindaNotEylemi(o, "gunluk", SIMDI)).toBe("tamam");
    expect(olay).toEqual([
      "gunlukSayfa:20261025",
      "bosBlokEkle:Oct 25th, 2026",
      "git:Oct 25th, 2026",
      `uyku:${GIDIS_BEKLEME_MS}`,
      "duzenle:uuid-blok",
    ]);
    expect(mesajlar).toEqual([]);
  });

  test("günlük yoksa YYYY-MM-DD ile oluşturulur", async () => {
    const { o, olay } = kur({ gunluk: null });
    expect(await anindaNotEylemi(o, "gunluk", SIMDI)).toBe("tamam");
    expect(olay.slice(0, 3)).toEqual([
      "gunlukSayfa:20261025",
      "gunlukOlustur:2026-10-25",
      "bosBlokEkle:Oct 25th, 2026",
    ]);
  });

  test("günlük bulunamaz/oluşturulamazsa Türkçe hata, blok eklenmez", async () => {
    const { o, olay, mesajlar } = kur({ gunluk: null, olusturulan: null });
    expect(await anindaNotEylemi(o, "gunluk", SIMDI)).toBe("hata");
    expect(olay.some((x) => x.startsWith("bosBlokEkle"))).toBe(false);
    expect(mesajlar).toHaveLength(1);
    expect(mesajlar[0].tur).toBe("error");
    expect(mesajlar[0].metin).toContain("Anında not açılamadı");
    expect(mesajlar[0].metin).toContain("günlük sayfası");
  });

  test("payload yoksa eklenti ayarındaki hedef kullanılır", async () => {
    const { o, olay } = kur({ notHedefi: "yeni_sayfa" });
    await anindaNotEylemi(o, undefined, SIMDI);
    expect(olay[0]).toBe("sayfaVarMi:Not 2026-10-25 14:03");
  });
});

describe("anindaNotEylemi: yeni sayfa", () => {
  test("Not <tarih saat> sayfası, boş blok, git, düzenle", async () => {
    const { o, olay } = kur();
    expect(await anindaNotEylemi(o, "yeni_sayfa", SIMDI)).toBe("tamam");
    expect(olay).toEqual([
      "sayfaVarMi:Not 2026-10-25 14:03",
      "sayfaOlustur:Not 2026-10-25 14:03",
      "bosBlokEkle:Not 2026-10-25 14:03",
      "git:Not 2026-10-25 14:03",
      `uyku:${GIDIS_BEKLEME_MS}`,
      "duzenle:uuid-blok",
    ]);
  });

  test("aynı dakikada çakışırsa sonuna sayı eklenir", async () => {
    const { o, olay } = kur({ varOlanlar: ["Not 2026-10-25 14:03", "Not 2026-10-25 14:03 2"] });
    await anindaNotEylemi(o, "yeni_sayfa", SIMDI);
    expect(olay).toContain("sayfaOlustur:Not 2026-10-25 14:03 3");
  });

  test("sayfa oluşturulamazsa hata", async () => {
    const { o, mesajlar, olay } = kur({ yeniSayfa: null });
    expect(await anindaNotEylemi(o, "yeni_sayfa", SIMDI)).toBe("hata");
    expect(mesajlar[0].metin).toContain("sayfa oluşturulamadı");
    expect(olay.some((x) => x.startsWith("git:"))).toBe(false);
  });
});

describe("hata dayanıklılığı", () => {
  test("boş blok eklenemezse hata, sayfaya gidilmez", async () => {
    const { o, mesajlar, olay } = kur({ blok: null });
    expect(await anindaNotEylemi(o, "gunluk", SIMDI)).toBe("hata");
    expect(mesajlar[0].metin).toContain("boş blok");
    expect(olay.some((x) => x.startsWith("git:") || x.startsWith("duzenle:"))).toBe(false);
  });

  test("Logseq API'si fırlatırsa çökmez, ileti verir", async () => {
    const { o, mesajlar } = kur({ firlat: "createJournalPage is not a function" });
    expect(await anindaNotEylemi(o, "gunluk", SIMDI)).toBe("hata");
    expect(mesajlar[0].metin).toContain("createJournalPage is not a function");
    expect(mesajlar[0].tur).toBe("error");
  });
});
