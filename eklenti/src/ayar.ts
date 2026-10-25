// Araç ayarları (GET/POST /ayar) ile Logseq eklenti ayarları arasındaki eşleme (saf).
// Logseq'e bağımlı değildir. Araç tek kaynaktır; eklenti ayarları onun aynasıdır.

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
  baslik: string;
  aciklama: string;
}

export const AYAR_TANIMLARI: readonly AyarTanimi[] = [
  {
    arac: "kucultme",
    eklenti: "kucultme",
    tur: "enum",
    secenekler: ["tepsi", "gorev_cubugu"],
    varsayilan: "tepsi",
    baslik: "Küçültme düğmesi (araç)",
    aciklama:
      "tepsi: küçültünce Logseq tepsiye gizlenir. gorev_cubugu: küçültünce görev çubuğunda kalır, araç gizlemez.",
  },
  {
    arac: "tik",
    eklenti: "tik",
    tur: "enum",
    secenekler: ["tek", "cift"],
    varsayilan: "tek",
    baslik: "Tepsi simgesi tıkı (araç)",
    aciklama: "tek: tek tıkla göster/gizle. cift: çift tıkla göster/gizle (tek tık bir şey yapmaz).",
  },
  {
    arac: "kapaliyken_tik_acar",
    eklenti: "kapaliykenTikAcar",
    tur: "boolean",
    varsayilan: true,
    baslik: "Logseq kapalıyken tık Logseq'i açsın (araç)",
    aciklama: "Kapalıyken tepsi simgesine tıklamak Logseq'i başlatsın mı. Menüdeki Göster her zaman başlatır.",
  },
  {
    arac: "kisayol_goster_gizle",
    eklenti: "kisayolGosterGizle",
    tur: "string",
    varsayilan: "Ctrl+Alt+L",
    baslik: "Genel kısayol: Logseq'i göster/gizle (araç)",
    aciklama:
      "Her programda çalışan genel kısayol. Örnek: Ctrl+Alt+L, Ctrl+Shift+F12 (yalnız ASCII). " +
      "Boş bırakırsanız kapalı olur. Başka program tuşu tutuyorsa araç ipucunda uyarır. " +
      "Logseq içi kısayoldan (Tepsiye gizle kısayolu) ayrıdır.",
  },
  {
    arac: "oturumda_gizli_baslat",
    eklenti: "oturumdaGizliBaslat",
    tur: "boolean",
    varsayilan: false,
    baslik: "Oturum açılınca Logseq'i gizli başlat (araç)",
    aciklama:
      "Bilgisayar açılışında araç başlangıç kısayolundan başlarsa Logseq'i başlatıp tepsiye gizler. " +
      "Kısayolu `logseq-tepsi kur` (ya da menüdeki Bilgisayar açılınca başlat) yazar.",
  },
  {
    arac: "baslangicta_calis",
    eklenti: "baslangictaCalis",
    tur: "boolean",
    varsayilan: false,
    baslik: "Bilgisayar açılınca aracı başlat (araç)",
    aciklama: "Başlangıç klasörüne araç kısayolunu koyar ya da siler. Gerçek durum aracın kısayol dosyasıdır.",
  },
  {
    arac: "kisayol_not",
    eklenti: "kisayolNot",
    tur: "string",
    varsayilan: "Ctrl+Alt+N",
    baslik: "Anında not: genel kısayol (araç)",
    aciklama:
      "Her programda çalışan genel kısayol; Logseq'i gösterir (kapalıysa başlatır) ve yeni bir not satırı açar. " +
      "Örnek: Ctrl+Alt+N (yalnız ASCII). Boş bırakırsanız kapalı olur.",
  },
  {
    arac: "not_hedefi",
    eklenti: "notHedefi",
    tur: "enum",
    secenekler: ["gunluk", "yeni_sayfa"],
    varsayilan: "gunluk",
    baslik: "Anında not nereye yazılsın (araç)",
    aciklama:
      "gunluk: bugünün günlük sayfasının sonuna boş blok. yeni_sayfa: \"Not YYYY-MM-DD HH:mm\" adlı yeni sayfa.",
  },
  {
    arac: "kalinan_sayfa",
    eklenti: "kalinanSayfa",
    tur: "boolean",
    varsayilan: true,
    baslik: "Logseq açılınca kaldığım sayfaya dön (araç)",
    aciklama:
      "Logseq açıldığında, siz henüz bir sayfaya gitmediyseniz son baktığınız sayfa açılır (sayfa silinmişse ana sayfada kalır).",
  },
  {
    arac: "api_otomatik_baslat",
    eklenti: "apiOtomatikBaslat",
    tur: "boolean",
    varsayilan: false,
    baslik: "Logseq açılınca API sunucusu otomatik başlasın (araç)",
    aciklama:
      "Logseq'in kendi yapılandırmasındaki (configs.edn) :server/autostart değerini değiştirir. Logseq açıkken yazılamaz: " +
      "istek araçta bekler, Logseq kapanınca uygulanır.",
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

/** Logseq `useSettingsSchema` girdileri (eklenti ayar sayfasında görünür). */
export function ayarSemasi() {
  return AYAR_TANIMLARI.map((t) => ({
    key: t.eklenti,
    type: t.tur,
    default: t.varsayilan,
    title: t.baslik,
    description: t.aciklama,
    ...(t.tur === "enum"
      ? { enumChoices: [...(t.secenekler ?? [])], enumPicker: "radio" as const }
      : {}),
  }));
}
