// Araç ayarları (GET/POST /ayar) ile Logseq eklenti ayarları arasındaki eşleme (saf).
// Logseq'e bağımlı değildir. Araç tek kaynaktır; eklenti ayarları onun aynasıdır.
import { DIL_SECENEKLERI, etkinDil, metinler, type Dil } from "./metin";

/** Logseq içi "Tepsiye gizle" kısayolunun varsayılanı (araç ayarı değil, yalnız eklentide). */
export const VARSAYILAN_KISAYOL = "mod+shift+x";

export type EklentiDegeri = string | boolean;
export type EklentiAyarlari = Record<string, EklentiDegeri>;
/** Araç anahtarı → araç metni (`evet`/`hayir`, `tepsi`, `Ctrl+Alt+L` ...). */
export type AracAyarlari = Record<string, string>;

type Tur = "enum" | "boolean" | "string";

export interface AyarTanimi {
  arac: string;
  eklenti: string;
  tur: Tur;
  secenekler?: readonly string[];
  varsayilan: EklentiDegeri;
  /**
   * Eski araç sürümleri bu anahtarı hiç döndürmez (örn. `dil`): yokluğu hata değildir, ayar yalnız
   * eklentide yaşar ve araca gönderilmez (eski araç bilinmeyen anahtarı 400 ile reddederdi).
   */
  istegeBagli?: boolean;
}

export const AYAR_TANIMLARI: readonly AyarTanimi[] = [
  {
    arac: "kucultme",
    eklenti: "kucultme",
    tur: "enum",
    secenekler: ["tepsi", "gorev_cubugu"],
    varsayilan: "tepsi",
  },
  {
    arac: "tik",
    eklenti: "tik",
    tur: "enum",
    secenekler: ["tek", "cift"],
    varsayilan: "tek",
  },
  {
    arac: "kapaliyken_tik_acar",
    eklenti: "kapaliykenTikAcar",
    tur: "boolean",
    varsayilan: true,
  },
  {
    arac: "kisayol_goster_gizle",
    eklenti: "kisayolGosterGizle",
    tur: "string",
    varsayilan: "Ctrl+Alt+L",
  },
  {
    arac: "oturumda_gizli_baslat",
    eklenti: "oturumdaGizliBaslat",
    tur: "boolean",
    varsayilan: false,
  },
  {
    arac: "baslangicta_calis",
    eklenti: "baslangictaCalis",
    tur: "boolean",
    varsayilan: false,
  },
  {
    arac: "kisayol_not",
    eklenti: "kisayolNot",
    tur: "string",
    varsayilan: "Ctrl+Alt+N",
  },
  {
    arac: "not_hedefi",
    eklenti: "notHedefi",
    tur: "enum",
    secenekler: ["gunluk", "yeni_sayfa"],
    varsayilan: "gunluk",
  },
  {
    arac: "kalinan_sayfa",
    eklenti: "kalinanSayfa",
    tur: "boolean",
    varsayilan: true,
  },
  {
    arac: "api_otomatik_baslat",
    eklenti: "apiOtomatikBaslat",
    tur: "boolean",
    varsayilan: false,
  },
  {
    arac: "dil",
    eklenti: "dil",
    tur: "enum",
    secenekler: DIL_SECENEKLERI,
    varsayilan: "otomatik",
    istegeBagli: true,
  },
];

/** Araç değeri → eklenti ayar değeri; tanınmayan değerde undefined. */
export function eklentiDegeri(t: AyarTanimi, arac: string): EklentiDegeri | undefined {
  switch (t.tur) {
    case "boolean":
      return arac === "evet" ? true : arac === "hayir" ? false : undefined;
    case "enum":
      return t.secenekler?.includes(arac) ? arac : undefined;
    case "string":
      return arac;
  }
}

/** Eklenti ayar değeri → araç metni; geçersizde undefined (o anahtar gönderilmez). */
export function aracDegeri(t: AyarTanimi, deger: unknown): string | undefined {
  switch (t.tur) {
    case "boolean":
      return typeof deger === "boolean" ? (deger ? "evet" : "hayir") : undefined;
    case "enum":
      return typeof deger === "string" && t.secenekler?.includes(deger) ? deger : undefined;
    case "string":
      return typeof deger === "string" ? deger.trim() : undefined;
  }
}

/** Eklenti ayarlarının (logseq.settings), araç değerlerinden FARKLI olan kısmı: updateSettings'e verilir. */
export function eklentiFarki(
  arac: AracAyarlari,
  mevcut: Record<string, unknown> | undefined,
): EklentiAyarlari {
  const fark: EklentiAyarlari = {};
  for (const t of AYAR_TANIMLARI) {
    const ham = arac[t.arac];
    if (ham === undefined) continue;
    const hedef = eklentiDegeri(t, ham);
    if (hedef !== undefined && mevcut?.[t.eklenti] !== hedef) fark[t.eklenti] = hedef;
  }
  return fark;
}

/**
 * Kullanıcının değiştirdiği (eski → yeni) ve aracın bildiği değerden de farklı olan ayarlar,
 * `anahtar=değer` satırları olarak. Araçtan gelen eşitleme `bilinen` ile aynı değerleri yazar,
 * bu yüzden geri POST tetiklemez (döngü yok). `bilinen` yoksa (araç kapalıydı) yalnız eski→yeni
 * farkına bakılır.
 */
export function degisenSatirlar(
  yeni: Record<string, unknown> | undefined,
  eski: Record<string, unknown> | undefined,
  bilinen: AracAyarlari | null,
): string[] {
  const satirlar: string[] = [];
  for (const t of AYAR_TANIMLARI) {
    const simdi = aracDegeri(t, yeni?.[t.eklenti]);
    if (simdi === undefined) continue;
    const once = aracDegeri(t, eski?.[t.eklenti]);
    if (simdi === once) continue;
    if (bilinen !== null && bilinen[t.arac] === simdi) continue;
    // Araç bu anahtarı bilmiyor (eski sürüm) ya da henüz okunmadı: ayar yalnız eklentide kalır,
    // ilk başarılı GET /ayar eşitler.
    if (t.istegeBagli && (bilinen === null || !(t.arac in bilinen))) continue;
    satirlar.push(`${t.arac}=${simdi}`);
  }
  return satirlar;
}

/** `anahtar=değer` satırlarını araç ayar haritasına (bilinen) işler. */
export function satirlariIsle(bilinen: AracAyarlari | null, satirlar: string[]): AracAyarlari {
  const sonuc = { ...(bilinen ?? {}) };
  for (const s of satirlar) {
    const i = s.indexOf("=");
    if (i > 0) sonuc[s.slice(0, i)] = s.slice(i + 1);
  }
  return sonuc;
}

/** Logseq `useSettingsSchema` girdileri (eklenti ayar sayfasında görünür); başlık ve açıklama `dil`de. */
export function ayarSemasi(dil: Dil = etkinDil()) {
  const tablo = metinler[dil].ayar;
  return AYAR_TANIMLARI.map((t) => ({
    key: t.eklenti,
    type: t.tur,
    default: t.varsayilan,
    title: tablo[t.eklenti as keyof typeof tablo].baslik,
    description: tablo[t.eklenti as keyof typeof tablo].aciklama,
    // enumChoices Logseq'te değerin kendisidir (ayrı görünen etiket yok): değerler dilden bağımsızdır.
    ...(t.tur === "enum"
      ? { enumChoices: [...(t.secenekler ?? [])], enumPicker: "radio" as const }
      : {}),
  }));
}

/** Eklentinin tüm ayar şeması: yalnız eklentiye ait üç ayar + araç ayarlarının aynası. */
export function tamSema(dil: Dil = etkinDil()) {
  const tablo = metinler[dil].ayar;
  return [
    {
      key: "port",
      type: "number",
      default: 47123,
      title: tablo.port.baslik,
      description: tablo.port.aciklama,
    },
    {
      key: "acilisUyarisi",
      type: "boolean",
      default: true,
      title: tablo.acilisUyarisi.baslik,
      description: tablo.acilisUyarisi.aciklama,
    },
    {
      key: "kisayol",
      type: "string",
      default: VARSAYILAN_KISAYOL,
      title: tablo.kisayol.baslik,
      description: tablo.kisayol.aciklama,
    },
    // Aşağıdakiler tepsi aracının ayarlarıdır: açılışta araçtan okunur, değişince araca yazılır.
    ...ayarSemasi(dil),
  ];
}
