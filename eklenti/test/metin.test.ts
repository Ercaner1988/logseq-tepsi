import { afterEach, describe, expect, test } from "bun:test";
import type { Durum, Hata } from "../src/ag";
import {
  AYAR_TANIMLARI,
  aracDegeri,
  ayarSemasi,
  degisenSatirlar,
  eklentiDegeri,
  eklentiFarki,
  satirlariIsle,
  tamSema,
} from "../src/ayar";
import { durumEylemi, type Ortam } from "../src/eylemler";
import * as mesajlar from "../src/mesajlar";
import {
  AYAR_ANAHTARLARI,
  dilKarar,
  dilKur,
  etkinDil,
  metinler,
  type Dil,
  type Metinler,
} from "../src/metin";

const DILLER: Dil[] = ["tr", "en"];
const TURKCEYE_OZGU = /[çğıöşüİĞŞÖÜÇ]/;
const DURUM: Durum = { surum: "0.1.0", gizli: 2, logseq: "acik", api: "kapali" };
const HATALAR: Hata[] = [
  { tur: "kapali" },
  { tur: "yasak", durum: 403 },
  { tur: "zaman-asimi" },
  { tur: "beklenmedik", durum: 500 },
  { tur: "beklenmedik", ayrinti: "garip" },
  { tur: "beklenmedik" },
];

// Başka sınama dosyaları Türkçe beklentilerle koşar: dili her sınamadan sonra Türkçeye geri al.
afterEach(() => dilKur("tr"));

/** Tablodaki her metni (düz dizge ya da örnek bağımsız değişkenlerle işlev çıktısı) düz listeye açar. */
function tumMetinler(d: Dil): Array<[string, string]> {
  const t = metinler[d];
  const cikti: Array<[string, string]> = [];
  for (const [k, v] of Object.entries(t)) {
    if (k === "ayar") {
      for (const [a, { baslik, aciklama }] of Object.entries(t.ayar)) {
        cikti.push([`ayar.${a}.baslik`, baslik], [`ayar.${a}.aciklama`, aciklama]);
      }
    } else if (typeof v === "string") {
      cikti.push([k, v]);
    }
  }
  for (const h of HATALAR) cikti.push([`hata.${h.tur}`, t.hata(h)]);
  cikti.push(
    ["durum", t.durum(DURUM)],
    ["durum-apisiz", t.durum({ ...DURUM, api: undefined })],
    ["zatenCalisiyor", t.zatenCalisiyor(DURUM)],
    ["baslatildi", t.baslatildi(DURUM)],
    ["ayarReddedildi", t.ayarReddedildi("neden")],
    ["ayarReddedildi-nedensiz", t.ayarReddedildi("")],
    ["ayarIletilemedi", t.ayarIletilemedi(t.hata({ tur: "kapali" }))],
    ["notAcilamadi", t.notAcilamadi("neden")],
    ["apiIstekGonderildi-baslat", t.apiIstekGonderildi(true)],
    ["apiIstekGonderildi-durdur", t.apiIstekGonderildi(false)],
    ["apiDurumu-ayni", t.apiDurumu(true, true)],
    ["apiDurumu-farkli", t.apiDurumu(false, true)],
    ["apiAyariNedeni", t.apiAyariNedeni("taban", "neden")],
  );
  return cikti;
}

describe("metin tablosu", () => {
  test("iki dilin anahtar kümeleri eşit (ayar alt tablosu dahil)", () => {
    const anahtarlar = (t: Metinler) => Object.keys(t).sort();
    expect(anahtarlar(metinler.en)).toEqual(anahtarlar(metinler.tr));
    expect(Object.keys(metinler.en.ayar).sort()).toEqual(Object.keys(metinler.tr.ayar).sort());
    expect(Object.keys(metinler.tr.ayar).sort()).toEqual([...AYAR_ANAHTARLARI].sort());
    for (const a of AYAR_ANAHTARLARI) {
      expect(Object.keys(metinler.en.ayar[a]).sort()).toEqual(Object.keys(metinler.tr.ayar[a]).sort());
    }
    // İşlev/dizge ayrımı iki dilde aynı.
    for (const k of Object.keys(metinler.tr) as Array<keyof Metinler>) {
      expect(typeof metinler.en[k]).toBe(typeof metinler.tr[k]);
    }
  });

  test("hiçbir metin boş değil (işlevler her örnek girdide dolu metin verir)", () => {
    for (const d of DILLER) {
      const liste = tumMetinler(d);
      expect(liste.length).toBeGreaterThan(40);
      for (const [ad, metin] of liste) {
        expect(`${d}:${ad}:${metin.trim().length > 0}`).toBe(`${d}:${ad}:true`);
      }
    }
  });

  test("İngilizce metinlerde Türkçeye özgü harf yok (Türkçe dil adı hariç)", () => {
    for (const [ad, metin] of tumMetinler("en")) {
      const temiz = metin.replaceAll("Türkçe", "");
      expect(`${ad}:${TURKCEYE_OZGU.test(temiz)}`).toBe(`${ad}:false`);
    }
  });

  test("Türkçe tablo gerçekten Türkçe kaldı (eski ileti metinleri)", () => {
    expect(metinler.tr.hata({ tur: "zaman-asimi" })).toContain("zamanında yanıt vermedi");
    expect(metinler.tr.durumEtiketi).toBe("Tepsi: durum");
    expect(metinler.tr.notAcilamadi("x")).toBe("Anında not açılamadı: x");
  });

  test("komut etiketleri: ikisi de Tepsi ile başlar, dile göre farklıdır", () => {
    const etiketler = [
      "gizleEtiketi",
      "durumEtiketi",
      "baslatEtiketi",
      "notEtiketi",
      "apiBaslatEtiketi",
      "apiDurdurEtiketi",
    ] as const;
    for (const e of etiketler) {
      expect(metinler.tr[e].startsWith("Tepsi: ")).toBe(true);
      expect(metinler.en[e].startsWith("Tepsi: ")).toBe(true);
      expect(metinler.en[e]).not.toBe(metinler.tr[e]);
    }
    expect(metinler.en.apiBaslatEtiketi).toContain("(experimental)");
    expect(metinler.en.apiDurdurEtiketi).toContain("(experimental)");
  });

  test("yönerge, kendi dilindeki gerçek komut etiketini anar", () => {
    expect(metinler.en.yonergeBaslatma).toContain(metinler.en.baslatEtiketi);
    expect(metinler.tr.yonergeBaslatma).toContain(metinler.tr.baslatEtiketi);
  });
});

describe("etkin dil ve mesajlar cephesi", () => {
  test("varsayılan Türkçe; dilKur bildirimleri anında değiştirir", () => {
    expect(etkinDil()).toBe("tr");
    expect(mesajlar.durumMetni(DURUM)).toContain("Tepsi aracı çalışıyor (sürüm 0.1.0)");
    dilKur("en");
    expect(etkinDil()).toBe("en");
    expect(mesajlar.durumMetni(DURUM)).toBe(
      "The tray tool is running (version 0.1.0). Hidden windows: 2. Logseq: open. API server: closed.",
    );
    dilKur("tr");
    expect(mesajlar.durumMetni(DURUM)).toContain("Tepsi aracı çalışıyor");
  });

  test("cephe işlevleri etkin dile uyar", () => {
    dilKur("en");
    expect(mesajlar.baslatiliyor()).toBe(metinler.en.baslatiliyor);
    expect(mesajlar.apiBaslatEtiketi()).toBe(metinler.en.apiBaslatEtiketi);
    expect(mesajlar.notSayfaYok()).toBe(metinler.en.notSayfaYok);
    expect(mesajlar.hataMetni({ tur: "kapali" })).toContain("not running");
    const red = mesajlar.ayarReddedildi("400 Bad Request\nkisayol: x");
    expect(red).toContain("rejected the setting change: kisayol: x");
    expect(red).not.toContain("400 Bad Request");
    expect(mesajlar.ayarIletilemedi({ tur: "kapali" })).toContain("not running");
    expect(mesajlar.apiAyariYazilamadi(" nedeni var ")).toContain("Reason: nedeni var.");
    expect(mesajlar.apiAyariYazilamadi(undefined)).toBe(metinler.en.apiAyariYazilamadi);
    dilKur("tr");
    expect(mesajlar.apiAyariYazilamadi("x")).toContain("Neden: x.");
    expect(mesajlar.apiAyariYazilamadi("")).toBe(metinler.tr.apiAyariYazilamadi);
  });

  test("bir eylem (durum) bildirimini etkin dilde verir", async () => {
    const iletiler: string[] = [];
    const o: Ortam = {
      mesaj: (metin) => iletiler.push(metin),
      adresAc: () => {},
      uyku: async () => {},
      secenekler: () => ({}),
      ag: {
        durumAl: async () => ({ ok: true, deger: DURUM }),
        gizle: async () => ({ ok: true, deger: null }),
      },
    };
    dilKur("en");
    await durumEylemi(o);
    dilKur("tr");
    await durumEylemi(o);
    expect(iletiler[0]).toStartWith("The tray tool is running");
    expect(iletiler[1]).toStartWith("Tepsi aracı çalışıyor");
  });
});

describe("dilKarar (saf)", () => {
  test("otomatik: Logseq dili tr ile başlıyorsa Türkçe, değilse İngilizce", () => {
    expect(dilKarar("otomatik", "tr-TR")).toBe("tr");
    expect(dilKarar("otomatik", "tr")).toBe("tr");
    expect(dilKarar("otomatik", "en")).toBe("en");
    expect(dilKarar("otomatik", "de")).toBe("en");
    expect(dilKarar("otomatik", "zh-CN")).toBe("en");
  });

  test("otomatik + okunamayan Logseq dili: İngilizce", () => {
    expect(dilKarar("otomatik", undefined)).toBe("en");
    expect(dilKarar("otomatik", null)).toBe("en");
    expect(dilKarar("otomatik", 5)).toBe("en");
    expect(dilKarar("otomatik", "")).toBe("en");
  });

  test("açık tr/en ayarı Logseq dilini yener", () => {
    expect(dilKarar("tr", "en")).toBe("tr");
    expect(dilKarar("tr", undefined)).toBe("tr");
    expect(dilKarar("en", "tr-TR")).toBe("en");
    expect(dilKarar("en", undefined)).toBe("en");
  });

  test("ayar yok ya da tanınmıyorsa otomatik sayılır (dil satırı göndermeyen eski araç)", () => {
    expect(dilKarar(undefined, "tr-TR")).toBe("tr");
    expect(dilKarar(undefined, "en")).toBe("en");
    expect(dilKarar(undefined, undefined)).toBe("en");
    expect(dilKarar("fr", "tr")).toBe("tr");
    expect(dilKarar(7, "de")).toBe("en");
  });
});

describe("dil ayarı: araç eşlemesi", () => {
  const dil = AYAR_TANIMLARI.find((a) => a.arac === "dil")!;
  const VARSAYILANLAR = Object.fromEntries(AYAR_TANIMLARI.map((t) => [t.eklenti, t.varsayilan]));

  test("tanım: enum otomatik/tr/en, varsayılan otomatik, araç ve eklenti anahtarı dil", () => {
    expect(dil).toMatchObject({ arac: "dil", eklenti: "dil", tur: "enum", varsayilan: "otomatik" });
    expect([...(dil.secenekler ?? [])]).toEqual(["otomatik", "tr", "en"]);
  });

  test("gidiş-dönüş: eklenti, araç, eklenti her değerde aynı", () => {
    for (const d of ["otomatik", "tr", "en"]) {
      const arac = aracDegeri(dil, d);
      expect(arac).toBe(d);
      expect(eklentiDegeri(dil, arac!)).toBe(d);
    }
    expect(aracDegeri(dil, "fr")).toBeUndefined();
    expect(aracDegeri(dil, true)).toBeUndefined();
    expect(eklentiDegeri(dil, "fr")).toBeUndefined();
  });

  test("araç dil=en dönerse eklenti ayarı en olur; aynıysa fark yok", () => {
    expect(eklentiFarki({ dil: "en" }, VARSAYILANLAR)).toEqual({ dil: "en" });
    expect(eklentiFarki({ dil: "otomatik" }, VARSAYILANLAR)).toEqual({});
    expect(eklentiFarki({ dil: "tr" }, { ...VARSAYILANLAR, dil: "tr" })).toEqual({});
    // Tanınmayan değer (yeni araç yeni dil eklemiş olabilir) sessizce yok sayılır.
    expect(eklentiFarki({ dil: "fr" }, VARSAYILANLAR)).toEqual({});
  });

  test("araç gövdesinde dil satırı yoksa eklenti ayarına dokunulmaz; etkin dil otomatiktir", () => {
    const eskiArac = { kucultme: "tepsi", tik: "tek" };
    expect(eklentiFarki(eskiArac, VARSAYILANLAR)).toEqual({});
    expect(VARSAYILANLAR.dil).toBe("otomatik");
    expect(dilKarar(VARSAYILANLAR.dil, "tr-TR")).toBe("tr");
    expect(dilKarar(VARSAYILANLAR.dil, "en")).toBe("en");
  });

  test("kullanıcı dili değiştirince dil=değer araca gider; araçtan gelen eşitleme geri gitmez", () => {
    const eski = { ...VARSAYILANLAR };
    const yeni = { ...VARSAYILANLAR, dil: "en" };
    // Araç henüz okunmadıysa (eski sürüm olabilir) dil gönderilmez.
    expect(degisenSatirlar(yeni, eski, null)).toEqual([]);
    expect(degisenSatirlar(yeni, eski, { dil: "otomatik" })).toEqual(["dil=en"]);
    expect(degisenSatirlar(yeni, eski, { dil: "en" })).toEqual([]);
    expect(satirlariIsle({ dil: "otomatik" }, ["dil=en"])).toEqual({ dil: "en" });
  });

  test("eski araç (GET /ayar dil satırı yok): dil ayarı araca gönderilmez, yalnız eklentide yaşar", () => {
    const eski = { ...VARSAYILANLAR };
    const yeni = { ...VARSAYILANLAR, dil: "en" };
    expect(degisenSatirlar(yeni, eski, { kucultme: "tepsi" })).toEqual([]);
    // Başka ayarlar yine gider.
    expect(degisenSatirlar({ ...yeni, tik: "cift" }, eski, { kucultme: "tepsi", tik: "tek" })).toEqual([
      "tik=cift",
    ]);
  });
});

describe("ayar şeması iki dilde", () => {
  test("her girdi iki dilde başlıklı ve açıklamalı; anahtarlar, türler ve seçenekler dilden bağımsız", () => {
    const tr = tamSema("tr");
    const en = tamSema("en");
    expect(tr.map((s) => s.key)).toEqual(en.map((s) => s.key));
    expect(tr.map((s) => s.key)).toEqual([...AYAR_ANAHTARLARI]);
    for (let i = 0; i < tr.length; i++) {
      expect(en[i].type).toBe(tr[i].type);
      expect(en[i].default).toBe(tr[i].default);
      expect(en[i].title.length).toBeGreaterThan(0);
      expect(en[i].description.length).toBeGreaterThan(0);
      expect(en[i].title).not.toBe(tr[i].title);
      expect(en[i].description).not.toBe(tr[i].description);
      expect((en[i] as { enumChoices?: string[] }).enumChoices).toEqual(
        (tr[i] as { enumChoices?: string[] }).enumChoices,
      );
    }
  });

  test("ayarSemasi() etkin dile uyar; dil ayarı seçenekleri otomatik/tr/en", () => {
    expect(ayarSemasi().find((s) => s.key === "dil")?.title).toBe(metinler.tr.ayar.dil.baslik);
    dilKur("en");
    expect(ayarSemasi().find((s) => s.key === "dil")?.title).toBe(metinler.en.ayar.dil.baslik);
    expect(ayarSemasi("tr").find((s) => s.key === "dil")?.title).toBe(metinler.tr.ayar.dil.baslik);
    const s = ayarSemasi().find((x) => x.key === "dil");
    expect(s).toMatchObject({ type: "enum", default: "otomatik", enumChoices: ["otomatik", "tr", "en"] });
  });
});
