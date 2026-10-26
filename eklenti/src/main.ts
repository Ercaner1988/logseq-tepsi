// Logseq eklentisi: tepsi aracını araç çubuğundan, komut paletinden ve kısayoldan yönetir.
// Pencereyi gizleme işini eklenti yapamaz (iframe içinde çalışır); yerel uçtan araca söyler.
import "@logseq/libs";
import { portCoz } from "./ag";
import { tamSema, VARSAYILAN_KISAYOL } from "./ayar";
import {
  acilisKontrolu,
  ayarDegisti,
  ayarEsitle,
  baslatEylemi,
  durumEylemi,
  gizleEylemi,
  type AyarDeposu,
  type Ortam,
} from "./eylemler";
import { kalinanaDon, rotaDegisti, yeniDurum, type DepoKatmani, type KalinanOrtami } from "./kalinan";
import { anindaNotEylemi, type DefterApi, type NotOrtami, type SayfaKimligi } from "./not";
import { dilKarar, dilKur, etkinDil, m, type Dil } from "./metin";
import { apiSunucusuEylemi } from "./sunucu";

const ARAC_CUBUGU_ANAHTARI = "logseq-tepsi-toolbar";

/** Aracın son bilinen ayarları (eşitleme ve başarılı gönderimle güncellenir). */
const depo: AyarDeposu = { bilinen: null };

const SIMGE_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="20" height="20" fill="none" ' +
  'stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
  '<path d="M12 3v10m0 0l-4-4m4 4l4-4"/><path d="M4 15h16v4a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"/></svg>';

function ortam(): Ortam {
  return {
    mesaj: (metin, tur, sureMs) => {
      void logseq.UI.showMsg(metin, tur, { timeout: sureMs });
    },
    adresAc: (url) => {
      // logseq.App.openExternalLink yalnız http(s) açar; özel şema için window.open gerekir.
      // Logseq bu çağrıyı yakalar ve kullanıcıya onay penceresi gösterir.
      window.open(url, "_blank");
    },
    uyku: (ms) => new Promise((coz) => setTimeout(coz, ms)),
    secenekler: () => ({ port: portCoz(logseq.settings?.port) }),
    ayarlar: () => logseq.settings,
    ayarGuncelle: (kismi) => logseq.updateSettings(kismi),
  };
}

const DEPO_ANAHTARI = "kalinan-sayfa.json";

/** Kalınan sayfa deposu: önce Logseq'in eklenti deposu, olmazsa iframe localStorage (hata fırlatan katman atlanır). */
const DEPO_KATMANLARI: DepoKatmani[] = [
  {
    oku: async () => (await logseq.FileStorage.getItem(DEPO_ANAHTARI)) ?? null,
    yaz: (deger) => logseq.FileStorage.setItem(DEPO_ANAHTARI, deger),
  },
  {
    oku: async () => localStorage.getItem(DEPO_ANAHTARI),
    yaz: async (deger) => localStorage.setItem(DEPO_ANAHTARI, deger),
  },
];

const kalinanDurum = yeniDurum();

type SayfaGirdisi = { uuid?: unknown; title?: unknown; originalName?: unknown; name?: unknown };

function sayfaKimligi(p: SayfaGirdisi | null | undefined): SayfaKimligi | null {
  if (!p) return null;
  const ad = [p.title, p.originalName, p.name].find((x) => typeof x === "string" && x !== "");
  const uuid = typeof p.uuid === "string" ? p.uuid : undefined;
  return typeof ad === "string" ? { uuid, ad } : null;
}

/** Sorgu sonucundaki uuid dizge ya da toString'i anlamlı bir nesne olabilir. */
function uuidMetni(ham: unknown): string | null {
  if (typeof ham === "string") return ham;
  if (typeof ham === "object" && ham !== null) {
    const s = String(ham);
    return s !== "[object Object]" ? s : null;
  }
  return null;
}

const DEFTER: DefterApi = {
  gunlukSayfa: async (gun) => {
    const sonuc = await logseq.DB.datascriptQuery<unknown[][]>(
      `[:find ?u :where [?p :block/journal-day ${gun}] [?p :block/uuid ?u]]`,
    );
    const uuid = uuidMetni(sonuc?.[0]?.[0]);
    return uuid ? sayfaKimligi(await logseq.Editor.getPage(uuid)) : null;
  },
  gunlukOlustur: async (gun) => sayfaKimligi(await logseq.Editor.createJournalPage(gun)),
  sayfaVarMi: async (ad) => (await logseq.Editor.getPage(ad)) != null,
  sayfaOlustur: async (ad) =>
    sayfaKimligi(await logseq.Editor.createPage(ad, {}, { redirect: false, createFirstBlock: false })),
  bosBlokEkle: async (s) => (await logseq.Editor.appendBlockInPage(s.uuid ?? s.ad, ""))?.uuid ?? null,
  sayfayaGit: (s) => logseq.App.pushState("page", { name: s.uuid ?? s.ad }),
  blokDuzenle: (uuid) => logseq.Editor.editBlock(uuid),
};

/** Anında not başlayınca kalınan sayfa dönüşü iptal olur (açılışta gelen not ezilmesin). */
function notOrtami(): NotOrtami {
  return {
    ...ortam(),
    defter: DEFTER,
    notBasladi: () => {
      kalinanDurum.gezildi = true;
    },
  };
}

const KALINAN_ORTAMI: KalinanOrtami = {
  depo: DEPO_KATMANLARI,
  rota: {
    sayfaVarMi: async (ad) => (await logseq.Editor.getPage(ad)) != null,
    git: (ad) => logseq.App.pushState("page", { name: ad }),
    grafAdi: async () => (await logseq.App.getCurrentGraph())?.name ?? null,
  },
  uyku: (ms) => new Promise((coz) => setTimeout(coz, ms)),
};

/** `window.top.apis.doAction` (belgesiz): yoksa ya da iframe yalıtımı erişimi engelliyorsa undefined. */
function apiEylemi(): ((dizi: string[]) => Promise<unknown>) | undefined {
  try {
    const apis = (window.top as unknown as { apis?: { doAction?: (a: unknown) => Promise<unknown> } } | null)?.apis;
    const eylem = apis?.doAction;
    return typeof eylem === "function" ? (dizi) => eylem.call(apis, dizi) : undefined;
  } catch {
    return undefined;
  }
}

function kisayolAyari(): string {
  const ham = logseq.settings?.kisayol;
  if (typeof ham !== "string") return VARSAYILAN_KISAYOL;
  return ham.trim();
}

/** Komut paleti kayıtlarının silicileri: dil değişince silinip yeni etiketlerle yeniden kaydedilir. */
let komutSilicileri: Array<() => void> = [];

/**
 * Dile bağlı arayüz kayıtları: ayar şeması, araç çubuğu düğmesi ve komut paleti etiketleri.
 * Şema `settings:schema` (Logseq `setSchema`), araç çubuğu öğesi anahtarına göre değiştirilir; komutlar
 * `registerCommandPalette` dönüşündeki siliciyle silinip yeniden kaydedilir (Logseq
 * `unregister_plugin_simple_command` anahtara göre siler; silmeden kaydetmek yineler).
 */
function arayuzuKaydet(): void {
  const mt = m();
  logseq.useSettingsSchema(tamSema(etkinDil()) as never);

  logseq.App.registerUIItem("toolbar", {
    key: ARAC_CUBUGU_ANAHTARI,
    template:
      `<a class="button" data-on-click="tepsiGizle" title="${mt.gizleIpucu}">${SIMGE_SVG}</a>`,
  });

  for (const sil of komutSilicileri) sil();
  komutSilicileri = [];
  const kaydet = (
    opts: Parameters<typeof logseq.App.registerCommandPalette>[0],
    eylem: () => void,
  ): void => {
    const sil = logseq.App.registerCommandPalette(opts, eylem);
    if (typeof sil === "function") komutSilicileri.push(sil);
  };
  const kisayol = kisayolAyari();
  // Komut KİMLİKLERİ (key) sabittir; yalnız label dile göre değişir.
  kaydet(
    {
      key: "tepsi-gizle",
      label: mt.gizleEtiketi,
      ...(kisayol ? { keybinding: { binding: kisayol, mode: "global" as const } } : {}),
    },
    () => void gizleEylemi(ortam()),
  );
  kaydet({ key: "tepsi-durum", label: mt.durumEtiketi }, () => void durumEylemi(ortam()));
  kaydet({ key: "tepsi-baslat", label: mt.baslatEtiketi }, () => void baslatEylemi(ortam()));
  kaydet(
    { key: "tepsi-aninda-not", label: mt.notEtiketi },
    () => void anindaNotEylemi(notOrtami(), undefined),
  );
  kaydet(
    { key: "tepsi-api-baslat", label: mt.apiBaslatEtiketi },
    () => void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, "baslat"),
  );
  kaydet(
    { key: "tepsi-api-durdur", label: mt.apiDurdurEtiketi },
    () => void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, "durdur"),
  );
}

/** Logseq'in arayüz dili (`preferredLanguage`); okunamazsa undefined (karar işlevi İngilizceye düşer). */
/** Logseq yanıt vermezse komutlar kayıtsız kalmasın: kısa süre sonra dil bilinmiyor sayılır. */
const LOGSEQ_DILI_ZAMAN_ASIMI_MS = 3000;

async function logseqDili(): Promise<unknown> {
  try {
    const zamanAsimi = new Promise<undefined>((coz) =>
      setTimeout(() => coz(undefined), LOGSEQ_DILI_ZAMAN_ASIMI_MS),
    );
    const ayarlar = await Promise.race([logseq.App.getUserConfigs(), zamanAsimi]);
    return ayarlar?.preferredLanguage;
  } catch {
    return undefined;
  }
}

let uygulananDil: Dil | null = null;
let dilSirasi: Promise<void> = Promise.resolve();

/** Etkin dili ayardan (gerekirse Logseq dilinden) çözer; değiştiyse bildirim dilini ve kayıtları yeniler. */
async function dilUygula(): Promise<void> {
  const ayar = logseq.settings?.dil;
  const dil = dilKarar(ayar, ayar === "tr" || ayar === "en" ? undefined : await logseqDili());
  if (dil === uygulananDil) return;
  dilKur(dil); // bildirimler bu andan itibaren yeni dilde
  document.documentElement.lang = dil;
  arayuzuKaydet();
  uygulananDil = dil; // kayıt hata fırlatırsa aynı dil bir sonraki olayda yeniden denenir
}

/** Ardışık ayar olayları sırayla işlenir (yarış olmasın). */
function dilUygulaSirayla(): Promise<void> {
  dilSirasi = dilSirasi.then(dilUygula).catch(console.error);
  return dilSirasi;
}

async function main(): Promise<void> {
  // Model KİMLİKLERİ sabittir (araç adresi ...models.anindaNot / apiSunucusu kullanır).
  logseq.provideModel({
    tepsiGizle() {
      void gizleEylemi(ortam());
    },
    // Araç `logseq://...invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=<hedef>` ile çağırır.
    anindaNot(payload: unknown) {
      void anindaNotEylemi(notOrtami(), payload);
    },
    // `...models.apiSunucusu&payload=baslat|durdur` (aracın menüsü).
    apiSunucusu(payload: unknown) {
      void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, payload);
    },
  });

  await dilUygulaSirayla(); // şema, araç çubuğu ve komutlar (dil çözülür)

  // Kalınan sayfa: her sayfa rotası saklanır; kullanıcı gezindiyse açılış dönüşü yapılmaz.
  logseq.App.onRouteChanged((r) => void rotaDegisti(KALINAN_ORTAMI, kalinanDurum, r));

  // Araç ayarlarının değişimi araca iletilir; araçtan gelen eşitleme geri POST etmez (bilinen).
  logseq.onSettingsChanged((yeni, eski) => {
    void ayarDegisti(ortam(), depo, yeni, eski);
    if (yeni?.dil !== eski?.dil) void dilUygulaSirayla();
  });

  await acilisKontrolu(ortam(), logseq.settings?.acilisUyarisi !== false);
  await ayarEsitle(ortam(), depo);
  // Eşitlemeden sonra: kalinanSayfa ayarı araçtaki değeri taşır.
  void kalinanaDon(KALINAN_ORTAMI, kalinanDurum, logseq.settings?.kalinanSayfa !== false);
}

logseq.ready(main).catch(console.error);
