import { describe, expect, test } from "bun:test";
import {
  AYAR_TANIMLARI,
  ayarSemasi,
  aracDegeri,
  degisenSatirlar,
  eklentiDegeri,
  eklentiFarki,
  satirlariIsle,
} from "../src/ayar";

const ARAC = {
  kucultme: "gorev_cubugu",
  tik: "cift",
  kapaliyken_tik_acar: "hayir",
  kisayol_goster_gizle: "Ctrl+Shift+F12",
  oturumda_gizli_baslat: "evet",
  baslangicta_calis: "evet",
};

const VARSAYILANLAR = Object.fromEntries(AYAR_TANIMLARI.map((t) => [t.eklenti, t.varsayilan]));

function t(arac: string) {
  const x = AYAR_TANIMLARI.find((a) => a.arac === arac);
  if (!x) throw new Error(arac);
  return x;
}

describe("eşleme tablosu", () => {
  test("on araç anahtarının hepsi var (ilk altısı eski sırayla), eklenti anahtarları benzersiz", () => {
    expect(AYAR_TANIMLARI.map((a) => a.arac)).toEqual([
      "kucultme",
      "tik",
      "kapaliyken_tik_acar",
      "kisayol_goster_gizle",
      "oturumda_gizli_baslat",
      "baslangicta_calis",
      "kisayol_not",
      "not_hedefi",
      "kalinan_sayfa",
      "api_otomatik_baslat",
    ]);
    const anahtarlar = AYAR_TANIMLARI.map((a) => a.eklenti);
    expect(new Set(anahtarlar).size).toBe(anahtarlar.length);
    // Logseq içi kısayol ve port ayarlarıyla çakışmaz.
    expect(anahtarlar).not.toContain("kisayol");
    expect(anahtarlar).not.toContain("port");
  });

  test("varsayılanlar araçtakilerle aynı", () => {
    expect(VARSAYILANLAR).toEqual({
      kucultme: "tepsi",
      tik: "tek",
      kapaliykenTikAcar: true,
      kisayolGosterGizle: "Ctrl+Alt+L",
      oturumdaGizliBaslat: false,
      baslangictaCalis: false,
      kisayolNot: "Ctrl+Alt+N",
      notHedefi: "gunluk",
      kalinanSayfa: true,
      apiOtomatikBaslat: false,
    });
  });

  test("şema Logseq biçiminde: enum seçenekleri ve varsayılan", () => {
    const sema = ayarSemasi();
    expect(sema).toHaveLength(10);
    const k = sema.find((s) => s.key === "kucultme");
    expect(k).toMatchObject({ type: "enum", default: "tepsi", enumChoices: ["tepsi", "gorev_cubugu"] });
    const b = sema.find((s) => s.key === "kapaliykenTikAcar");
    expect(b).toMatchObject({ type: "boolean", default: true });
    expect(b && "enumChoices" in b).toBe(false);
    for (const s of sema) {
      expect(s.title.length).toBeGreaterThan(0);
      expect(s.description.length).toBeGreaterThan(0);
    }
  });
});

describe("değer çevirme", () => {
  test("araç → eklenti", () => {
    expect(eklentiDegeri(t("kapaliyken_tik_acar"), "evet")).toBe(true);
    expect(eklentiDegeri(t("kapaliyken_tik_acar"), "hayir")).toBe(false);
    expect(eklentiDegeri(t("kapaliyken_tik_acar"), "belki")).toBeUndefined();
    expect(eklentiDegeri(t("tik"), "cift")).toBe("cift");
    expect(eklentiDegeri(t("tik"), "uc")).toBeUndefined();
    expect(eklentiDegeri(t("kisayol_goster_gizle"), "")).toBe("");
  });

  test("eklenti → araç; geçersiz değer gönderilmez", () => {
    expect(aracDegeri(t("oturumda_gizli_baslat"), true)).toBe("evet");
    expect(aracDegeri(t("oturumda_gizli_baslat"), false)).toBe("hayir");
    expect(aracDegeri(t("oturumda_gizli_baslat"), "evet")).toBeUndefined();
    expect(aracDegeri(t("kucultme"), "gorev_cubugu")).toBe("gorev_cubugu");
    expect(aracDegeri(t("kucultme"), "yok")).toBeUndefined();
    expect(aracDegeri(t("kisayol_goster_gizle"), "  Ctrl+Alt+X ")).toBe("Ctrl+Alt+X");
    expect(aracDegeri(t("kisayol_goster_gizle"), 5)).toBeUndefined();
  });
});

describe("eklentiFarki (araç → eklenti eşitleme)", () => {
  test("yalnız farklı olanları verir", () => {
    const fark = eklentiFarki(ARAC, VARSAYILANLAR);
    expect(fark).toEqual({
      kucultme: "gorev_cubugu",
      tik: "cift",
      kapaliykenTikAcar: false,
      kisayolGosterGizle: "Ctrl+Shift+F12",
      oturumdaGizliBaslat: true,
      baslangictaCalis: true,
    });
    expect(eklentiFarki({ tik: "tek" }, VARSAYILANLAR)).toEqual({});
  });
  test("tanınmayan ya da eksik araç değeri atlanır", () => {
    expect(eklentiFarki({ tik: "uc", baslangicta_calis: "evet" }, {})).toEqual({
      baslangictaCalis: true,
    });
    expect(eklentiFarki({}, undefined)).toEqual({});
  });
});

describe("degisenSatirlar (eklenti → araç, döngüsüz)", () => {
  test("kullanıcının değiştirdiği ayar satır olur", () => {
    const eski = { ...VARSAYILANLAR };
    const yeni = { ...VARSAYILANLAR, tik: "cift" };
    expect(degisenSatirlar(yeni, eski, { tik: "tek" })).toEqual(["tik=cift"]);
  });

  test("araçtan gelen eşitleme geri gönderilmez: yeni değer bilinenle aynı", () => {
    // Eşitleme eklenti ayarını araçtaki değere çekti; olay eski→yeni farkı görür ama bilinen aynı.
    const eski = { ...VARSAYILANLAR };
    const yeni = eklentiFarki(ARAC, eski);
    expect(degisenSatirlar({ ...eski, ...yeni }, eski, ARAC)).toEqual([]);
  });

  test("bilinen yoksa (araç kapalıydı) eski→yeni farkı gönderilir", () => {
    const eski = { ...VARSAYILANLAR };
    const yeni = { ...VARSAYILANLAR, kapaliykenTikAcar: false, kucultme: "gorev_cubugu" };
    expect(degisenSatirlar(yeni, eski, null)).toEqual(["kucultme=gorev_cubugu", "kapaliyken_tik_acar=hayir"]);
  });

  test("değişmeyen ve geçersiz değerler gönderilmez", () => {
    expect(degisenSatirlar(VARSAYILANLAR, VARSAYILANLAR, null)).toEqual([]);
    expect(degisenSatirlar({ ...VARSAYILANLAR, tik: "uc" }, VARSAYILANLAR, null)).toEqual([]);
    expect(degisenSatirlar(undefined, undefined, null)).toEqual([]);
    // İlgisiz ayarlar (port, kisayol) araca gitmez.
    expect(degisenSatirlar({ ...VARSAYILANLAR, port: 5000, kisayol: "x" }, VARSAYILANLAR, null)).toEqual([]);
  });

  test("boş kısayol (kapatma) satır olur", () => {
    const yeni = { ...VARSAYILANLAR, kisayolGosterGizle: "" };
    expect(degisenSatirlar(yeni, VARSAYILANLAR, ARAC)).toEqual(["kisayol_goster_gizle="]);
  });
});

describe("satirlariIsle", () => {
  test("bilinen haritayı günceller", () => {
    expect(satirlariIsle({ tik: "tek", kucultme: "tepsi" }, ["tik=cift"])).toEqual({
      tik: "cift",
      kucultme: "tepsi",
    });
    expect(satirlariIsle(null, ["kisayol_goster_gizle="])).toEqual({ kisayol_goster_gizle: "" });
  });
});
