// Logseq eklentisi: tepsi aracını araç çubuğundan, komut paletinden ve kısayoldan yönetir.
// Pencereyi gizleme işini eklenti yapamaz (iframe içinde çalışır); yerel uçtan araca söyler.
import "@logseq/libs";
import { portCoz } from "./ag";
import {
  acilisKontrolu,
  baslatEylemi,
  durumEylemi,
  gizleEylemi,
  type Ortam,
} from "./eylemler";

const VARSAYILAN_KISAYOL = "mod+shift+x";
const ARAC_CUBUGU_ANAHTARI = "logseq-tepsi-toolbar";

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
      "Örnek: mod+shift+x (mod, Windows'ta Ctrl tuşudur). Boş bırakırsanız kısayol atanmaz. " +
      "Değişiklik, eklenti yeniden yüklenince ya da Logseq yeniden açılınca geçerli olur.",
  },
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
  };
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

  await acilisKontrolu(ortam(), logseq.settings?.acilisUyarisi !== false);
}

logseq.ready(main).catch(console.error);
