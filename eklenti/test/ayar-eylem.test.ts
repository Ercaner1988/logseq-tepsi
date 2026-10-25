import { describe, expect, test } from "bun:test";
import type { Hata, Sonuc } from "../src/ag";
import { AYAR_TANIMLARI, type EklentiAyarlari } from "../src/ayar";
import { ayarDegisti, ayarEsitle, type AyarDeposu, type Ortam } from "../src/eylemler";

const ARAC = {
  kucultme: "gorev_cubugu",
  tik: "cift",
  kapaliyken_tik_acar: "hayir",
  kisayol_goster_gizle: "Ctrl+Shift+F12",
  oturumda_gizli_baslat: "evet",
  baslangicta_calis: "hayir",
};

const VARSAYILANLAR: EklentiAyarlari = Object.fromEntries(
  AYAR_TANIMLARI.map((t) => [t.eklenti, t.varsayilan]),
);

function ok<T>(deger: T): Sonuc<T> {
  return { ok: true, deger };
}
function hata(h: Hata): Sonuc<never> {
  return { ok: false, hata: h };
}

interface Kayit {
  mesajlar: { metin: string; tur: string }[];
  gonderilen: string[][];
  guncellenen: EklentiAyarlari[];
  alinan: number;
}

/** Sahte Logseq: `ayarlar` canlı eklenti ayarlarıdır, ayarGuncelle onları değiştirir. */
function kur(
  arac: Sonuc<Record<string, string>>,
  gonder: Sonuc<null> = ok(null),
  baslangic: EklentiAyarlari = VARSAYILANLAR,
) {
  const kayit: Kayit = { mesajlar: [], gonderilen: [], guncellenen: [], alinan: 0 };
  let ayarlar: EklentiAyarlari = { ...baslangic };
  const o: Ortam = {
    mesaj: (metin, tur) => kayit.mesajlar.push({ metin, tur }),
    adresAc: () => {},
    uyku: async () => {},
    secenekler: () => ({}),
    ayarlar: () => ayarlar,
    ayarGuncelle: (kismi) => {
      kayit.guncellenen.push(kismi);
      ayarlar = { ...ayarlar, ...kismi };
    },
    ayarAg: {
      ayarAl: async () => {
        kayit.alinan++;
        return arac;
      },
      ayarGonder: async (satirlar) => {
        kayit.gonderilen.push(satirlar);
        return gonder;
      },
    },
  };
  return { o, kayit, ayarlar: () => ayarlar };
}

describe("ayarEsitle (açılış)", () => {
  test("eklenti ayarlarını araçtakilere çeker, bilinen'e yazar", async () => {
    const { o, kayit, ayarlar } = kur(ok(ARAC));
    const depo: AyarDeposu = { bilinen: null };
    await ayarEsitle(o, depo);
    expect(depo.bilinen).toEqual(ARAC);
    expect(kayit.guncellenen).toHaveLength(1);
    expect(ayarlar()).toMatchObject({
      kucultme: "gorev_cubugu",
      tik: "cift",
      kapaliykenTikAcar: false,
      kisayolGosterGizle: "Ctrl+Shift+F12",
      oturumdaGizliBaslat: true,
    });
    expect(kayit.mesajlar).toEqual([]);
  });

  test("zaten eşitse güncelleme yapmaz", async () => {
    const { o, kayit } = kur(ok({ tik: "tek", kucultme: "tepsi" }));
    await ayarEsitle(o, { bilinen: null });
    expect(kayit.guncellenen).toEqual([]);
  });

  test("araç kapalıysa sessiz kalır, bilinen değişmez", async () => {
    const { o, kayit } = kur(hata({ tur: "kapali" }));
    const depo: AyarDeposu = { bilinen: null };
    await ayarEsitle(o, depo);
    expect(depo.bilinen).toBeNull();
    expect(kayit.mesajlar).toEqual([]);
    expect(kayit.guncellenen).toEqual([]);
  });
});

describe("ayarDegisti (kullanıcı eklenti ayarını değiştirince)", () => {
  test("değişen ayar araca POST edilir, bilinen güncellenir", async () => {
    const { o, kayit } = kur(ok(ARAC));
    const depo: AyarDeposu = { bilinen: { ...ARAC } };
    const eski = { ...VARSAYILANLAR, tik: "cift" };
    await ayarDegisti(o, depo, { ...eski, tik: "tek" }, eski);
    expect(kayit.gonderilen).toEqual([["tik=tek"]]);
    expect(depo.bilinen?.tik).toBe("tek");
    expect(kayit.mesajlar).toEqual([]);
  });

  test("döngü yok: açılış eşitlemesinin yarattığı olay araca geri POST etmez", async () => {
    const { o, kayit, ayarlar } = kur(ok(ARAC));
    const depo: AyarDeposu = { bilinen: null };
    const eski = { ...ayarlar() };
    await ayarEsitle(o, depo);
    // Logseq, updateSettings sonrası onSettingsChanged(yeni, eski) tetikler:
    await ayarDegisti(o, depo, ayarlar(), eski);
    expect(kayit.gonderilen).toEqual([]);
    expect(kayit.mesajlar).toEqual([]);
  });

  test("araç kapalıysa sessiz kayıp yok: uyarı verilir, bilinen değişmez", async () => {
    const { o, kayit } = kur(ok(ARAC), hata({ tur: "kapali" }));
    const depo: AyarDeposu = { bilinen: { ...ARAC } };
    await ayarDegisti(o, depo, { ...VARSAYILANLAR, tik: "tek" }, { ...VARSAYILANLAR, tik: "cift" });
    expect(kayit.gonderilen).toHaveLength(1);
    expect(kayit.mesajlar).toHaveLength(1);
    expect(kayit.mesajlar[0].tur).toBe("warning");
    expect(kayit.mesajlar[0].metin).toContain("araca iletilemedi");
    expect(kayit.mesajlar[0].metin).toContain("Tepsi aracı çalışmıyor");
    expect(depo.bilinen).toEqual(ARAC);
  });

  test("araç bilinmiyorken (açılışta kapalıydı) eski→yeni farkı yine gönderilir", async () => {
    const { o, kayit } = kur(hata({ tur: "kapali" }), hata({ tur: "kapali" }));
    await ayarDegisti(o, { bilinen: null }, { ...VARSAYILANLAR, kapaliykenTikAcar: false }, VARSAYILANLAR);
    expect(kayit.gonderilen).toEqual([["kapaliyken_tik_acar=hayir"]]);
    expect(kayit.mesajlar[0].tur).toBe("warning");
  });

  test("zaman aşımı ve 403 de uyarılır", async () => {
    for (const h of [{ tur: "zaman-asimi" }, { tur: "yasak", durum: 403 }] as Hata[]) {
      const { o, kayit } = kur(ok(ARAC), hata(h));
      await ayarDegisti(o, { bilinen: { ...ARAC } }, { ...VARSAYILANLAR, tik: "tek" }, { ...VARSAYILANLAR, tik: "cift" });
      expect(kayit.mesajlar).toHaveLength(1);
      expect(kayit.mesajlar[0].metin).toContain("araca iletilemedi");
    }
  });

  test("araç reddederse (400) nedeni gösterilir ve eklenti ayarı geri alınır", async () => {
    const red: Hata = {
      tur: "beklenmedik",
      durum: 400,
      ayrinti: "400 Bad Request\nkisayol_goster_gizle: bilinmeyen tuş: Foo",
    };
    const eski = { ...VARSAYILANLAR, kisayolGosterGizle: "Ctrl+Shift+F12" };
    const { o, kayit, ayarlar } = kur(ok(ARAC), hata(red), { ...eski, kisayolGosterGizle: "Ctrl+Foo" });
    const depo: AyarDeposu = { bilinen: { ...ARAC } };
    await ayarDegisti(o, depo, ayarlar(), eski);
    expect(kayit.gonderilen).toEqual([["kisayol_goster_gizle=Ctrl+Foo"]]);
    expect(kayit.mesajlar).toHaveLength(1);
    expect(kayit.mesajlar[0].tur).toBe("error");
    expect(kayit.mesajlar[0].metin).toContain("reddetti: kisayol_goster_gizle: bilinmeyen tuş: Foo");
    expect(kayit.mesajlar[0].metin).not.toContain("400 Bad Request");
    expect(ayarlar().kisayolGosterGizle).toBe("Ctrl+Shift+F12");
    expect(kayit.alinan).toBe(1);
  });

  test("ilgisiz ayar (port, logseq içi kısayol) değişince araca hiçbir şey gitmez", async () => {
    const { o, kayit } = kur(ok(ARAC));
    await ayarDegisti(o, { bilinen: { ...ARAC } }, { ...VARSAYILANLAR, port: 1234 }, { ...VARSAYILANLAR, port: 47123 });
    expect(kayit.gonderilen).toEqual([]);
    expect(kayit.mesajlar).toEqual([]);
  });
});

describe("api_otomatik_baslat sonucu", () => {
  const eski = { ...VARSAYILANLAR, apiOtomatikBaslat: false };
  const yeni = { ...VARSAYILANLAR, apiOtomatikBaslat: true };

  test("Logseq açıkken araç beklemeye aldıysa kullanıcıya 'Logseq kapanınca uygulanacak' denir", async () => {
    const arac = { ...ARAC, api_otomatik_baslat: "evet", api_otomatik_bekleyen: "evet" };
    const { o, kayit } = kur(ok(arac));
    await ayarDegisti(o, { bilinen: { ...ARAC } }, yeni, eski);
    expect(kayit.gonderilen).toEqual([["api_otomatik_baslat=evet"]]);
    expect(kayit.mesajlar).toHaveLength(1);
    expect(kayit.mesajlar[0].tur).toBe("success");
    expect(kayit.mesajlar[0].metin).toContain("Logseq kapanınca uygulanacak");
  });

  test("araç hemen yazdıysa (Logseq kapalıydı) ileti yok", async () => {
    const arac = { ...ARAC, api_otomatik_baslat: "evet", api_otomatik_bekleyen: "" };
    const { o, kayit } = kur(ok(arac));
    await ayarDegisti(o, { bilinen: { ...ARAC } }, yeni, eski);
    expect(kayit.gonderilen).toHaveLength(1);
    expect(kayit.mesajlar).toEqual([]);
  });

  test("araç yazamadıysa (istenen ≠ araçtaki, bekleyen yok) uyarır ve ayarı geri alır", async () => {
    const arac = { ...ARAC, api_otomatik_baslat: "hayir", api_otomatik_bekleyen: "" };
    const { o, kayit, ayarlar } = kur(ok(arac), ok(null), yeni);
    const depo: AyarDeposu = { bilinen: { ...ARAC } };
    await ayarDegisti(o, depo, yeni, eski);
    expect(kayit.mesajlar).toHaveLength(1);
    expect(kayit.mesajlar[0].tur).toBe("warning");
    expect(kayit.mesajlar[0].metin).toContain("yapılandırma dosyasına yazamadı");
    expect(ayarlar().apiOtomatikBaslat).toBe(false);
    expect(depo.bilinen?.api_otomatik_baslat).toBe("hayir");
  });

  test("başka ayar değişince api sorgusu yapılmaz", async () => {
    const { o, kayit } = kur(ok(ARAC));
    await ayarDegisti(o, { bilinen: { ...ARAC } }, { ...VARSAYILANLAR, tik: "tek" }, { ...VARSAYILANLAR, tik: "cift" });
    expect(kayit.alinan).toBe(0);
  });
});

describe("yeni araç ayarları eşitlemesi", () => {
  test("açılışta not ve kalınan sayfa ayarları araçtakilere çekilir", async () => {
    const arac = {
      ...ARAC,
      kisayol_not: "Ctrl+Shift+F9",
      not_hedefi: "yeni_sayfa",
      kalinan_sayfa: "hayir",
      api_otomatik_baslat: "evet",
      api_otomatik_bekleyen: "evet",
    };
    const { o, ayarlar } = kur(ok(arac));
    await ayarEsitle(o, { bilinen: null });
    expect(ayarlar()).toMatchObject({
      kisayolNot: "Ctrl+Shift+F9",
      notHedefi: "yeni_sayfa",
      kalinanSayfa: false,
      apiOtomatikBaslat: true,
    });
  });

  test("eklentide değişen yeni ayarlar araç satırlarına çevrilir", async () => {
    const { o, kayit } = kur(ok(ARAC));
    await ayarDegisti(
      o,
      { bilinen: { ...ARAC } },
      { ...VARSAYILANLAR, kisayolNot: "Alt+Win+N", notHedefi: "yeni_sayfa", kalinanSayfa: false },
      VARSAYILANLAR,
    );
    expect(kayit.gonderilen).toEqual([["kisayol_not=Alt+Win+N", "not_hedefi=yeni_sayfa", "kalinan_sayfa=hayir"]]);
  });
});
