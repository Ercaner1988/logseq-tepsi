// Logseq eklentisi: tepsi aracını araç çubuğundan, komut paletinden ve kısayoldan yönetir.
// Pencereyi gizleme işini eklenti yapamaz (iframe içinde çalışır); yerel uçtan araca söyler.
import "@logseq/libs";
import { portCoz } from "./ag";
import { ayarSemasi } from "./ayar";
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
import { anindaNotEylemi, type DefterApi, type SayfaKimligi } from "./not";
import { apiSunucusuEylemi } from "./sunucu";

const VARSAYILAN_KISAYOL = "mod+shift+x";
const ARAC_CUBUGU_ANAHTARI = "logseq-tepsi-toolbar";

/** Aracın son bilinen ayarları (eşitleme ve başarılı gönderimle güncellenir). */
const depo: AyarDeposu = { bilinen: null };

const AYAR_SEMASI = [
  {
    key: "port",
    type: "number",
    default: 47123,
    title: "Araç portu",
    description: "logseq-tepsi aracının yerel uç portu. Aracı varsayılan portla çalıştırıyorsanız değiştirmeyin.",
  },
  {
    key: "acilisUyarisi",
    type: "boolean",
    default: true,
    title: "Açılışta araç yoksa uyar",
    description: "Logseq açılırken tepsi aracı çalışmıyorsa bir kez uyarı gösterir. Rahatsız ediyorsa kapatın.",
  },
  {
    key: "kisayol",
    type: "string",
    default: VARSAYILAN_KISAYOL,
    title: "Tepsiye gizle kısayolu",
    description:
      "Logseq içi kısayol. Örnek: mod+shift+x (mod, Windows'ta Ctrl tuşudur). Boş bırakırsanız kısayol atanmaz. " +
      "Değişiklik, eklenti yeniden yüklenince ya da Logseq yeniden açılınca geçerli olur.",
  },
  // Aşağıdakiler tepsi aracının ayarlarıdır: açılışta araçtan okunur, değişince araca yazılır.
  ...ayarSemasi(),
];

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

async function main(): Promise<void> {
  logseq.useSettingsSchema(AYAR_SEMASI as never);

  logseq.provideModel({
    tepsiGizle() {
      void gizleEylemi(ortam());
    },
    // Araç `logseq://...invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=<hedef>` ile çağırır.
    anindaNot(payload: unknown) {
      void anindaNotEylemi({ ...ortam(), defter: DEFTER }, payload);
    },
    // `...models.apiSunucusu&payload=baslat|durdur` (aracın menüsü).
    apiSunucusu(payload: unknown) {
      void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, payload);
    },
  });

  logseq.App.registerUIItem("toolbar", {
    key: ARAC_CUBUGU_ANAHTARI,
    template:
      `<a class="button" data-on-click="tepsiGizle" title="Logseq'i tepsiye gizle">${SIMGE_SVG}</a>`,
  });

  const kisayol = kisayolAyari();
  logseq.App.registerCommandPalette(
    {
      key: "tepsi-gizle",
      label: "Tepsi: Logseq'i tepsiye gizle",
      ...(kisayol ? { keybinding: { binding: kisayol, mode: "global" as const } } : {}),
    },
    () => void gizleEylemi(ortam()),
  );
  logseq.App.registerCommandPalette(
    { key: "tepsi-durum", label: "Tepsi: durum" },
    () => void durumEylemi(ortam()),
  );
  logseq.App.registerCommandPalette(
    { key: "tepsi-baslat", label: "Tepsi: aracı başlat" },
    () => void baslatEylemi(ortam()),
  );
  logseq.App.registerCommandPalette(
    { key: "tepsi-aninda-not", label: "Tepsi: anında not" },
    () => void anindaNotEylemi({ ...ortam(), defter: DEFTER }, undefined),
  );
  logseq.App.registerCommandPalette(
    { key: "tepsi-api-baslat", label: "Tepsi: API sunucusunu başlat" },
    () => void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, "baslat"),
  );
  logseq.App.registerCommandPalette(
    { key: "tepsi-api-durdur", label: "Tepsi: API sunucusunu durdur" },
    () => void apiSunucusuEylemi({ ...ortam(), apiEylemi: apiEylemi() }, "durdur"),
  );

  // Kalınan sayfa: her sayfa rotası saklanır; kullanıcı gezindiyse açılış dönüşü yapılmaz.
  logseq.App.onRouteChanged((r) => void rotaDegisti(KALINAN_ORTAMI, kalinanDurum, r));

  // Araç ayarlarının değişimi araca iletilir; araçtan gelen eşitleme geri POST etmez (bilinen).
  logseq.onSettingsChanged((yeni, eski) => void ayarDegisti(ortam(), depo, yeni, eski));

  await acilisKontrolu(ortam(), logseq.settings?.acilisUyarisi !== false);
  await ayarEsitle(ortam(), depo);
  // Eşitlemeden sonra: kalinanSayfa ayarı araçtaki değeri taşır.
  void kalinanaDon(KALINAN_ORTAMI, kalinanDurum, logseq.settings?.kalinanSayfa !== false);
}

logseq.ready(main).catch(console.error);
