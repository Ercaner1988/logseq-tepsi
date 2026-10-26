// Çok dilli metin tablosu (Türkçe + English): kullanıcıya görünen her metin burada, tek kaynak.
// Saf modül; Logseq'e bağımlı değildir. Etkin dil modül düzeyinde tutulur (`dilKur`), böylece
// bildirimler dil ayarı değişir değişmez yeni dilde çıkar. Komut ve model KİMLİKLERİ burada yok:
// onlar sabittir (araç `...models.anindaNot` adresini kullanır), yalnız görünen etiketler değişir.
import type { Durum, Hata } from "./ag";

export type Dil = "tr" | "en";
/** Eklenti ayarındaki `dil` değeri (araçla paylaşılan sözleşme). */
export type DilAyari = "otomatik" | Dil;

export const DIL_SECENEKLERI: readonly DilAyari[] = ["otomatik", "tr", "en"];

/**
 * Etkin dil kararı (saf): açık `tr`/`en` ayarı kazanır; `otomatik` (ya da tanınmayan/eksik değer,
 * örn. `dil` satırı göndermeyen eski araç) için Logseq'in arayüz dili: `tr` ile başlıyorsa Türkçe,
 * değilse ya da okunamadıysa İngilizce.
 */
export function dilKarar(ayar: unknown, logseqDili: unknown): Dil {
  if (ayar === "tr" || ayar === "en") return ayar;
  return typeof logseqDili === "string" && /^tr/i.test(logseqDili.trim()) ? "tr" : "en";
}

/** Ayar sayfasında görünen ayarların anahtarları (eklenti tarafı adları). */
export const AYAR_ANAHTARLARI = [
  "port",
  "acilisUyarisi",
  "kisayol",
  "kucultme",
  "tik",
  "kapaliykenTikAcar",
  "kisayolGosterGizle",
  "oturumdaGizliBaslat",
  "baslangictaCalis",
  "kisayolNot",
  "notHedefi",
  "kalinanSayfa",
  "apiOtomatikBaslat",
  "dil",
] as const;
export type AyarAnahtari = (typeof AYAR_ANAHTARLARI)[number];

export interface AyarMetni {
  baslik: string;
  aciklama: string;
}

/** Biçim gerektirmeyen metinler. */
export interface DuzMetinler {
  yonergeBaslatma: string;
  baslatiliyor: string;
  baslatilamadi: string;
  notSayfaYok: string;
  notBlokYok: string;
  notSayfaAdiYok: string;
  notSayfaOlusmadi: string;
  apiIzinYok: string;
  apiAyariBeklemede: string;
  apiAyariYazilamadi: string;
  // Durum iletisindeki sözcükler.
  acik: string;
  kapali: string;
  calisiyor: string;
  // Komut paleti etiketleri ve araç çubuğu ipucu (kimlikler main.ts'te sabittir).
  gizleEtiketi: string;
  durumEtiketi: string;
  baslatEtiketi: string;
  notEtiketi: string;
  apiBaslatEtiketi: string;
  apiDurdurEtiketi: string;
  gizleIpucu: string;
  ayar: Record<AyarAnahtari, AyarMetni>;
}

/** Biçimli metinler (işlev). */
export interface BicimliMetinler {
  hata: (h: Hata) => string;
  durum: (d: Durum) => string;
  zatenCalisiyor: (d: Durum) => string;
  baslatildi: (d: Durum) => string;
  /** `neden` boş olabilir. */
  ayarReddedildi: (neden: string) => string;
  /** `hataMetni`: araca ulaşılamama nedeninin (bu dilde) metni. */
  ayarIletilemedi: (hataMetni: string) => string;
  notAcilamadi: (neden: string) => string;
  apiIstekGonderildi: (baslat: boolean) => string;
  apiDurumu: (acik: boolean, istenenAcik: boolean) => string;
  /** Araç bir neden bildirdiyse ana iletiye eklenir. */
  apiAyariNedeni: (taban: string, neden: string) => string;
}

export type Metinler = DuzMetinler & BicimliMetinler;

const TR: Metinler = {
  yonergeBaslatma:
    "Aracı başlatmak için komut paletinden “Tepsi: aracı başlat” komutunu çalıştırın " +
    "ya da terminalde logseq-tepsi yazın.",
  baslatiliyor:
    "Tepsi aracı başlatılıyor… Logseq bir onay penceresi gösterirse logseq-tepsi bağlantısına izin verin.",
  baslatilamadi:
    "Tepsi aracı başlatılamadı. Onay penceresini kabul ettiniz mi? " +
    "Aracı bir kez “logseq-tepsi kur” ile kaydetmiş olmanız gerekir. " +
    "Elle başlatmak için terminalde logseq-tepsi yazın.",
  notSayfaYok: "bugünün günlük sayfası bulunamadı ya da oluşturulamadı",
  notBlokYok: "sayfaya boş blok eklenemedi",
  notSayfaAdiYok: "uygun bir sayfa adı bulunamadı",
  notSayfaOlusmadi: "sayfa oluşturulamadı",
  apiIzinYok: "Logseq bu sürümde izin vermiyor; Logseq'in kendi sunucu menüsünü kullanın.",
  apiAyariBeklemede: "Logseq açık olduğu için API sunucusu ayarı Logseq kapanınca uygulanacak.",
  apiAyariYazilamadi:
    "Araç API sunucusu ayarını Logseq'in yapılandırma dosyasına yazamadı (ayrıntı aracın günlüğünde).",
  acik: "açık",
  kapali: "kapalı",
  calisiyor: "çalışıyor",
  gizleEtiketi: "Tepsi: Logseq'i tepsiye gizle",
  durumEtiketi: "Tepsi: durum",
  baslatEtiketi: "Tepsi: aracı başlat",
  notEtiketi: "Tepsi: anında not",
  apiBaslatEtiketi: "Tepsi: API sunucusunu başlat (deneysel)",
  apiDurdurEtiketi: "Tepsi: API sunucusunu durdur (deneysel)",
  gizleIpucu: "Logseq'i tepsiye gizle",
  ayar: {
    port: {
      baslik: "Araç portu",
      aciklama: "logseq-tepsi aracının yerel uç portu. Aracı varsayılan portla çalıştırıyorsanız değiştirmeyin.",
    },
    acilisUyarisi: {
      baslik: "Açılışta araç yoksa uyar",
      aciklama: "Logseq açılırken tepsi aracı çalışmıyorsa bir kez uyarı gösterir. Rahatsız ediyorsa kapatın.",
    },
    kisayol: {
      baslik: "Tepsiye gizle kısayolu",
      aciklama:
        "Logseq içi kısayol. Örnek: mod+shift+x (mod, Windows'ta Ctrl tuşudur). Boş bırakırsanız kısayol atanmaz. " +
        "Değişiklik, eklenti yeniden yüklenince ya da Logseq yeniden açılınca geçerli olur.",
    },
    kucultme: {
      baslik: "Küçültme düğmesi (araç)",
      aciklama:
        "tepsi: küçültünce Logseq tepsiye gizlenir. gorev_cubugu: küçültünce görev çubuğunda kalır, araç gizlemez.",
    },
    tik: {
      baslik: "Tepsi simgesi tıkı (araç)",
      aciklama: "tek: tek tıkla göster/gizle. cift: çift tıkla göster/gizle (tek tık bir şey yapmaz).",
    },
    kapaliykenTikAcar: {
      baslik: "Logseq kapalıyken tık Logseq'i açsın (araç)",
      aciklama: "Kapalıyken tepsi simgesine tıklamak Logseq'i başlatsın mı. Menüdeki Göster her zaman başlatır.",
    },
    kisayolGosterGizle: {
      baslik: "Genel kısayol: Logseq'i göster/gizle (araç)",
      aciklama:
        "Her programda çalışan genel kısayol. Örnek: Ctrl+Alt+L, Ctrl+Shift+F12 (yalnız ASCII). " +
        "Boş bırakırsanız kapalı olur. Başka program tuşu tutuyorsa araç ipucunda uyarır. " +
        "Logseq içi kısayoldan (Tepsiye gizle kısayolu) ayrıdır.",
    },
    oturumdaGizliBaslat: {
      baslik: "Oturum açılınca Logseq'i gizli başlat (araç)",
      aciklama:
        "Bilgisayar açılışında araç başlangıç kısayolundan başlarsa Logseq'i başlatıp tepsiye gizler. " +
        "Kısayolu `logseq-tepsi kur` (ya da menüdeki Bilgisayar açılınca başlat) yazar.",
    },
    baslangictaCalis: {
      baslik: "Bilgisayar açılınca aracı başlat (araç)",
      aciklama: "Başlangıç klasörüne araç kısayolunu koyar ya da siler. Gerçek durum aracın kısayol dosyasıdır.",
    },
    kisayolNot: {
      baslik: "Anında not: genel kısayol (araç)",
      aciklama:
        "Her programda çalışan genel kısayol; Logseq'i gösterir (kapalıysa başlatır) ve yeni bir not satırı açar. " +
        "Örnek: Ctrl+Alt+N (yalnız ASCII). Boş bırakırsanız kapalı olur.",
    },
    notHedefi: {
      baslik: "Anında not nereye yazılsın (araç)",
      aciklama:
        "gunluk: bugünün günlük sayfasının sonuna boş blok. yeni_sayfa: \"Not YYYY-MM-DD HH:mm\" adlı yeni sayfa.",
    },
    kalinanSayfa: {
      baslik: "Logseq açılınca kaldığım sayfaya dön (araç)",
      aciklama:
        "Logseq açıldığında, siz henüz bir sayfaya gitmediyseniz son baktığınız sayfa açılır (sayfa silinmişse ana sayfada kalır).",
    },
    apiOtomatikBaslat: {
      baslik: "Logseq açılınca API sunucusu otomatik başlasın (araç)",
      aciklama:
        "Logseq'in kendi yapılandırmasındaki (configs.edn) :server/autostart değerini değiştirir. Logseq açıkken yazılamaz: " +
        "istek araçta bekler, Logseq kapanınca uygulanır.",
    },
    dil: {
      baslik: "Dil / Language (araç)",
      aciklama:
        "otomatik: eklenti Logseq'in arayüz dilini, tepsi aracı Windows görüntü dilini izler (Türkçeyse Türkçe, değilse English). tr: Türkçe. en: English. " +
        "Komut paleti etiketleri ile bu ayar sayfası dil değişince yeniden kaydedilir; açık bir ayar penceresi varsa kapatıp açın.",
    },
  },

  hata: (h) => {
    switch (h.tur) {
      case "kapali":
        return `Tepsi aracı çalışmıyor ya da erişilemiyor. ${TR.yonergeBaslatma} Port ayarını da denetleyin.`;
      case "yasak":
        return "Tepsi aracı isteği reddetti (403). Eklenti ile araç sürümleri uyuşmuyor olabilir; aracı güncelleyin.";
      case "zaman-asimi":
        return "Tepsi aracı zamanında yanıt vermedi. Araç takılmış olabilir; yeniden başlatmayı deneyin.";
      case "beklenmedik": {
        const ek =
          h.durum !== undefined ? ` (HTTP ${h.durum})` : h.ayrinti ? ` (${h.ayrinti})` : "";
        return `Tepsi aracı beklenmedik bir yanıt verdi${ek}.`;
      }
    }
  },
  durum: (d) => {
    const logseq = d.logseq === "acik" ? TR.acik : TR.kapali;
    const api = d.api === undefined ? "" : ` API sunucusu: ${d.api === "acik" ? TR.calisiyor : TR.kapali}.`;
    return `Tepsi aracı çalışıyor (sürüm ${d.surum}). Gizli pencere: ${d.gizli}. Logseq: ${logseq}.${api}`;
  },
  zatenCalisiyor: (d) => `Tepsi aracı zaten çalışıyor (sürüm ${d.surum}).`,
  baslatildi: (d) => `Tepsi aracı başlatıldı (sürüm ${d.surum}).`,
  ayarReddedildi: (neden) =>
    `Tepsi aracı ayar değişikliğini reddetti${neden ? `: ${neden}` : ""}. ` +
    "Eklenti ayarı araçtaki değere döndürüldü.",
  ayarIletilemedi: (hataMetni) =>
    `Ayar değişikliği araca iletilemedi, araçta uygulanmadı. ${hataMetni} ` +
    "Eklenti ayarı araçtakinden farklı kalır; araç çalışınca eklenti açıldığında araçtaki değer geçerli olur.",
  notAcilamadi: (neden) => `Anında not açılamadı: ${neden}`,
  apiIstekGonderildi: (baslat) =>
    `API sunucusunu ${baslat ? "başlatma" : "durdurma"} isteği Logseq'e gönderildi.`,
  apiDurumu: (acik, istenenAcik) => {
    const simdi = acik ? TR.calisiyor : TR.kapali;
    return acik === istenenAcik
      ? `API sunucusu ${simdi}.`
      : `API sunucusu henüz ${simdi} görünüyor; Logseq'in kendi sunucu menüsünden de bakabilirsiniz.`;
  },
  apiAyariNedeni: (taban, neden) => `${taban} Neden: ${neden}.`,
};

const EN: Metinler = {
  yonergeBaslatma:
    "To start the tool, run the command “Tepsi: start tool” from the command palette " +
    "or type logseq-tepsi in a terminal.",
  baslatiliyor:
    "Starting the tray tool… If Logseq shows a confirmation dialog, allow the logseq-tepsi link.",
  baslatilamadi:
    "Could not start the tray tool. Did you accept the confirmation dialog? " +
    "You must register the tool once with “logseq-tepsi kur”. " +
    "To start it manually, type logseq-tepsi in a terminal.",
  notSayfaYok: "today's journal page could not be found or created",
  notBlokYok: "could not add an empty block to the page",
  notSayfaAdiYok: "no suitable page name was found",
  notSayfaOlusmadi: "could not create the page",
  apiIzinYok: "Logseq does not allow this in this version; use Logseq's own server menu.",
  apiAyariBeklemede: "Logseq is open, so the API server setting will be applied when Logseq closes.",
  apiAyariYazilamadi:
    "The tool could not write the API server setting to Logseq's configuration file (details are in the tool's log).",
  acik: "open",
  kapali: "closed",
  calisiyor: "running",
  gizleEtiketi: "Tepsi: hide Logseq to the tray",
  durumEtiketi: "Tepsi: status",
  baslatEtiketi: "Tepsi: start tool",
  notEtiketi: "Tepsi: quick note",
  apiBaslatEtiketi: "Tepsi: start API server (experimental)",
  apiDurdurEtiketi: "Tepsi: stop API server (experimental)",
  gizleIpucu: "Hide Logseq to the tray",
  ayar: {
    port: {
      baslik: "Tool port",
      aciklama: "Local port of the logseq-tepsi tool. Leave it alone if you run the tool on its default port.",
    },
    acilisUyarisi: {
      baslik: "Warn on startup if the tool is missing",
      aciklama: "Shows a one-time warning when Logseq starts and the tray tool is not running. Turn it off if it bothers you.",
    },
    kisayol: {
      baslik: "Hide-to-tray shortcut",
      aciklama:
        "Shortcut inside Logseq. Example: mod+shift+x (mod is the Ctrl key on Windows). Leave empty to assign no shortcut. " +
        "A change takes effect when the plugin is reloaded or Logseq is restarted.",
    },
    kucultme: {
      baslik: "Minimize button (tool)",
      aciklama:
        "tepsi: minimizing hides Logseq to the tray. gorev_cubugu: minimizing keeps it on the taskbar; the tool does not hide it.",
    },
    tik: {
      baslik: "Tray icon click (tool)",
      aciklama: "tek: a single click shows/hides. cift: a double click shows/hides (a single click does nothing).",
    },
    kapaliykenTikAcar: {
      baslik: "Clicking the icon opens Logseq when it is closed (tool)",
      aciklama: "Whether clicking the tray icon starts Logseq while it is closed. Show in the menu always starts it.",
    },
    kisayolGosterGizle: {
      baslik: "Global shortcut: show/hide Logseq (tool)",
      aciklama:
        "Global shortcut that works in every program. Example: Ctrl+Alt+L, Ctrl+Shift+F12 (ASCII only). " +
        "Leave empty to turn it off. If another program holds the key, the tool warns in its tooltip. " +
        "Separate from the shortcut inside Logseq (Hide-to-tray shortcut).",
    },
    oturumdaGizliBaslat: {
      baslik: "Start Logseq hidden at sign-in (tool)",
      aciklama:
        "When the tool starts from its startup shortcut at boot, it starts Logseq and hides it to the tray. " +
        "The shortcut is written by `logseq-tepsi kur` (or Start with Windows in the tray menu).",
    },
    baslangictaCalis: {
      baslik: "Start the tool with the computer (tool)",
      aciklama: "Adds or removes the tool's shortcut in the Startup folder. The real state is the tool's shortcut file.",
    },
    kisayolNot: {
      baslik: "Quick note: global shortcut (tool)",
      aciklama:
        "Global shortcut that works in every program; shows Logseq (starts it if closed) and opens a new note line. " +
        "Example: Ctrl+Alt+N (ASCII only). Leave empty to turn it off.",
    },
    notHedefi: {
      baslik: "Where quick notes go (tool)",
      aciklama:
        "gunluk: an empty block at the end of today's journal page. yeni_sayfa: a new page named \"Not YYYY-MM-DD HH:mm\".",
    },
    kalinanSayfa: {
      baslik: "Return to my last page when Logseq opens (tool)",
      aciklama:
        "When Logseq opens and you have not navigated to a page yet, the last page you viewed is opened (if the page was deleted, it stays on the home page).",
    },
    apiOtomatikBaslat: {
      baslik: "Start the API server automatically when Logseq opens (tool)",
      aciklama:
        "Changes the :server/autostart value in Logseq's own configuration (configs.edn). It cannot be written while Logseq is open: " +
        "the request waits in the tool and is applied when Logseq closes.",
    },
    dil: {
      baslik: "Dil / Language (tool)",
      aciklama:
        "otomatik: the plugin follows Logseq's interface language, the tray tool follows the Windows display language (Türkçe if Turkish, otherwise English). tr: Türkçe. en: English. " +
        "Command palette labels and this settings page are re-registered when the language changes; if a settings window is open, close and reopen it.",
    },
  },

  hata: (h) => {
    switch (h.tur) {
      case "kapali":
        return `The tray tool is not running or cannot be reached. ${EN.yonergeBaslatma} Also check the port setting.`;
      case "yasak":
        return "The tray tool rejected the request (403). The plugin and tool versions may not match; update the tool.";
      case "zaman-asimi":
        return "The tray tool did not respond in time. It may be stuck; try restarting it.";
      case "beklenmedik": {
        const ek =
          h.durum !== undefined ? ` (HTTP ${h.durum})` : h.ayrinti ? ` (${h.ayrinti})` : "";
        return `The tray tool gave an unexpected response${ek}.`;
      }
    }
  },
  durum: (d) => {
    const logseq = d.logseq === "acik" ? EN.acik : EN.kapali;
    const api = d.api === undefined ? "" : ` API server: ${d.api === "acik" ? EN.calisiyor : EN.kapali}.`;
    return `The tray tool is running (version ${d.surum}). Hidden windows: ${d.gizli}. Logseq: ${logseq}.${api}`;
  },
  zatenCalisiyor: (d) => `The tray tool is already running (version ${d.surum}).`,
  baslatildi: (d) => `The tray tool has started (version ${d.surum}).`,
  ayarReddedildi: (neden) =>
    `The tray tool rejected the setting change${neden ? `: ${neden}` : ""}. ` +
    "The plugin setting was reverted to the tool's value.",
  ayarIletilemedi: (hataMetni) =>
    `The setting change could not be sent to the tool and was not applied there. ${hataMetni} ` +
    "The plugin setting stays different from the tool's; once the tool is running, the tool's value takes effect when the plugin opens.",
  notAcilamadi: (neden) => `Could not open quick note: ${neden}`,
  apiIstekGonderildi: (baslat) =>
    `The request to ${baslat ? "start" : "stop"} the API server was sent to Logseq.`,
  apiDurumu: (acik, istenenAcik) => {
    const simdi = acik ? EN.calisiyor : EN.kapali;
    return acik === istenenAcik
      ? `The API server is ${simdi}.`
      : `The API server still appears ${simdi}; you can also check Logseq's own server menu.`;
  },
  apiAyariNedeni: (taban, neden) => `${taban} Reason: ${neden}.`,
};

export const metinler: Record<Dil, Metinler> = { tr: TR, en: EN };

// Başlangıç Türkçe: Logseq hazır olmadan gösterilen bir şey yok; `dilKur` açılışta ayarla çağrılır.
let etkin: Dil = "tr";

export function dilKur(d: Dil): void {
  etkin = d;
}

export function etkinDil(): Dil {
  return etkin;
}

/** Etkin dilin metin tablosu. */
export function m(): Metinler {
  return metinler[etkin];
}
