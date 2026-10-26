# Tepsi (Logseq eklentisi)

Logseq penceresini Windows bildirim alanına (tepsiye) gizleyen [logseq-tepsi](../README.md) aracını Logseq'in içinden yönetir. Eklenti pencereyi kendisi gizleyemez (eklentiler bir iframe içinde çalışır); aracın yerel ucuna (`127.0.0.1`) istek atar.

> **English.** Controls the [logseq-tepsi](https://github.com/Ercaner1988/logseq-tepsi) Windows tray tool from inside Logseq: a toolbar button and command to hide Logseq to the system tray, tool status, quick note into today's journal or a new page, reopening the last visited page on startup, and the tool's settings mirrored in the plugin settings. Requires the tray tool (`logseq-tepsi.exe`, Windows) from the repository's Releases; without it the plugin loads and only shows a warning. Documentation is in Turkish.

![Tepsi menüsü ve Ayarlar alt menüsü](/tepsi-menusu.png)

## Ne yapar

- **Araç çubuğu düğmesi**: tıklayınca Logseq'i tepsiye gizler.
- **Komut paleti** (`Ctrl+Shift+P`):
  - `Tepsi: Logseq'i tepsiye gizle`
  - `Tepsi: durum` (araç çalışıyor mu, sürüm, gizli pencere sayısı)
  - `Tepsi: aracı başlat` (araç kapalıysa başlatmayı dener)
  - `Tepsi: anında not` (hedefe göre bugünün günlük sayfasına ya da yeni "Not <tarih saat>" sayfasına boş blok açar)
  - `Tepsi: API sunucusunu başlat (deneysel)` / `Tepsi: API sunucusunu durdur (deneysel)` (belgesiz kanal, canlı doğrulanmadı; aşağıda)
- **Klavye kısayolu**: varsayılan `Ctrl+Shift+X` (Mac'te `Cmd+Shift+X`), ayarlardan değişir.
- **Açılış denetimi**: Logseq açılırken araç çalışmıyorsa bir kez uyarır; ayarlardan kapatılabilir.
- **Anında not**: tepsi aracının genel kısayolu (`Ctrl+Alt+N`) ya da tepsi menüsündeki "Anında not" Logseq'e `logseq://x-callback-url/invokeCommand?action=plugin.logseq-tepsi.models.anindaNot&payload=<hedef>` adresini gönderir; eklenti `provideModel({ anindaNot })` ile karşılar. Aynı iş komut paletinden de çalışır.
- **Kalınan sayfa**: eklenti `onRouteChanged` ile son açık sayfanın adını (grafla birlikte) kendi deposunda tutar; Logseq açılışında ayar açıksa, siz henüz bir sayfaya gitmediyseniz ve sayfa hâlâ varsa oraya gider. Sayfa silinmişse sessizce ana sayfada kalınır.

## Gereksinim

`logseq-tepsi` aracı çalışıyor olmalı. Eklenti araç olmadan da yüklenir, yalnızca uyarı verir. Aracı bir kez `logseq-tepsi kur` ile kaydederseniz `Tepsi: aracı başlat` komutu çalışır (araç `logseq-tepsi://baslat` adresini kaydeder).

## Kurulum

### Pazardan

Logseq'te Eklentiler, Pazar, "Tepsi" araması, Kur. Pazar sürümü [Sürümler](https://github.com/Ercaner1988/logseq-tepsi/releases) sayfasındaki `logseq-tepsi-v*.zip`'tir; aynı zip elle de kurulabilir (açıp `logseq-tepsi` klasörünü `~/.logseq/plugins/` altına koyun).

### Betikle (geliştirici kipi gerekmez)

```sh
sh kur.sh            # derler, sınar, ~/.logseq/plugins/logseq-tepsi altına kopyalar
sh kur.sh --sabitle  # ayrıca araç çubuğu düğmesini sabitler (Logseq kapalıyken)
```

Ardından Logseq'i (yeniden) başlatın. Logseq, `~/.logseq/plugins` altındaki her klasörü açılışta eklenti olarak yükler; klasör adı (`logseq-tepsi`) eklenti kimliği olur.

### Elle

1. `bun install --frozen-lockfile && bun run build`
2. `package.json`, `icon.svg`, `README.md`, `LICENSE` ve `dist/` klasörünü `~/.logseq/plugins/logseq-tepsi/` içine kopyalayın.
3. Logseq'i yeniden başlatın.

Ya da Logseq'te Ayarlar, Gelişmiş bölümünden geliştirici kipini açıp Eklentiler sayfasında "Paketlenmemiş eklenti yükle" ile bu klasörü seçin.

## Araç çubuğunda düğme görünmüyor

Logseq, sabitlenmiş eklenti düğmesi bir kez bile seçilmişse yalnızca sabitlenmişleri araç çubuğunda gösterir; öbürleri araç çubuğundaki eklenti menüsünde durur. Tepsi düğmesini orada bulup sabitleyin ya da `sh kur.sh --sabitle` kullanın. Komut paleti ve kısayol bundan etkilenmez.

## Ayarlar

| Ayar | Varsayılan | Açıklama |
| --- | --- | --- |
| Araç portu | `47123` | Aracın yerel uç portu |
| Açılışta araç yoksa uyar | açık | Logseq açılırken tek seferlik uyarı |
| Tepsiye gizle kısayolu | `mod+shift+x` | Logseq içi kısayol. Boş bırakılırsa kısayol atanmaz; değişiklik eklenti yeniden yüklenince geçerli olur |
| Küçültme düğmesi (araç) | `tepsi` | `tepsi` ya da `gorev_cubugu`: küçültünce Logseq tepsiye mi gizlensin, görev çubuğunda mı kalsın |
| Tepsi simgesi tıkı (araç) | `tek` | `tek` ya da `cift`: hangi tık göster/gizle yapar |
| Logseq kapalıyken tık Logseq'i açsın (araç) | açık | Kapalıyken tepsi simgesine tıklamak Logseq'i başlatsın mı |
| Genel kısayol: Logseq'i göster/gizle (araç) | `Ctrl+Alt+L` | Her programda çalışan genel kısayol (yalnız ASCII); boş = kapalı. Logseq içi kısayoldan ayrıdır |
| Oturum açılınca Logseq'i gizli başlat (araç) | kapalı | Araç başlangıç kısayolundan başlarsa Logseq'i başlatıp tepsiye gizler |
| Bilgisayar açılınca aracı başlat (araç) | kapalı | Başlangıç klasöründeki araç kısayolunu koyar ya da siler |
| Anında not: genel kısayol (araç) | `Ctrl+Alt+N` | Her programda çalışan genel kısayol; Logseq'i gösterir (kapalıysa başlatır) ve not satırı açar. Boş = kapalı |
| Anında not nereye yazılsın (araç) | `gunluk` | `gunluk`: bugünün günlük sayfasının sonuna boş blok. `yeni_sayfa`: "Not YYYY-MM-DD HH:mm" adlı sayfa (çakışırsa sonuna sayı) |
| Logseq açılınca kaldığım sayfaya dön (araç) | açık | Açılışta, henüz gezinmediyseniz, son sayfaya gider |
| Logseq açılınca API sunucusu otomatik başlasın (araç) | kapalı | Logseq'in `configs.edn` `:server/autostart` değeri. Logseq açıkken yazılamaz: araç bekletir, Logseq kapanınca uygular; eklenti bunu "Logseq kapanınca uygulanacak" diye bildirir. Araç sonucu `POST /ayar` yanıtında söyler (yazıldı / beklemede / hata): hata olursa (beklenmeyen dosya yapısı) nedenle birlikte uyarır ve ayarı araçtaki değere geri alır |

"(araç)" ile biten ayarlar tepsi aracının ayarlarıdır: tek kaynak aracın `ayar.toml` dosyasıdır. Eklenti açılışta `GET /ayar` ile bunları okuyup ayar sayfasını araçtakine eşitler; sayfada değiştirdiğinizde `POST /ayar` ile araca yazar (araçtan gelen eşitleme geri yazmaz, döngü olmaz). Araç çalışmıyorsa değişiklik iletilemez: eklenti bunu bir uyarıyla bildirir ve ayar, eklenti bir sonraki açılışta araçtaki değere döner. Araç değeri reddederse (örn. geçersiz kısayol) nedeni gösterilir ve ayar araçtaki değere geri alınır; yanıt zaman aşımına düşerse ya da 503 gelirse (araç değişikliği uygulamış olabilir) de ayar araçtaki gerçek değerle eşitlenir.

## Anında not, kalınan sayfa ve API sunucusu: ayrıntılar

**Model adları ve payload.** Araç adresle şu işlevleri çağırır (`logseq.provideModel`): `anindaNot(payload)` (`gunluk` | `yeni_sayfa`; geçersizse eklenti ayarı `notHedefi`, o da yoksa günlük) ve `apiSunucusu(payload)` (`baslat` | `durdur`). Eklenti kimliği klasör adıyla aynıdır (`logseq-tepsi`). Logseq 2.0.2 asar'ında doğrulandı: `invokeCommand` `action` değerini `.` ile bölüp `plugin.<kimlik>.models.<işlev>` biçimini ister ve modeli adı olduğu gibi, `payload` dizgesini ilk argüman vererek çağırır (ayrıntı: [kök README](../README.md#logseq-202de-doğrulananlar-asar-okuması)).

**Anında not akışı** (`src/not.ts`, Logseq API'si `DefterApi` ile enjekte, birim sınamalı): günlük için önce `:block/journal-day` sorgusu (bugünün `YYYYMMDD` değeri), yoksa `createJournalPage("YYYY-MM-DD")`; yeni sayfa için `createPage` ("Not YYYY-MM-DD HH:mm", yerel saat; ad doluysa ` 2`, ` 3` ...). Sonra `appendBlockInPage(sayfa, "")` ile boş blok, `pushState('page', {name: <uuid>})` ile sayfaya gidiş, 300 ms bekleme ve `editBlock(uuid)`. Herhangi bir Logseq çağrısı hata verirse Türkçe ileti gösterilir, eklenti çökmez.

**Kalınan sayfa** (`src/kalinan.ts`): yalnız `/page/:name` rotaları saklanır. Depo: önce `logseq.FileStorage` (eklenti deposu), olmazsa iframe `localStorage`; erişim hatası fırlatan katman atlanır, hiçbiri çalışmazsa kayıt sessizce atlanır. Açılışta ayar açıksa 1,5 sn beklenir, graf yüklenene kadar (en çok 10 sn) yoklanır; kullanıcı bu arada bir sayfaya gittiyse (`gezildi`) ya da kayıt başka grafa aitse hiçbir şey yapılmaz. Anında not başlar başlamaz (ilk eşzamansız işten önce) `gezildi` kurulur, sayfa sorgusu sürerken kurulursa da dönüş `git`ten hemen önce yeniden denetlenip iptal edilir: açılıştaki anında not, kalınan sayfaya dönüşle ezilmez. Sayfa yoksa (silinmiş) ileti vermeden ana sayfada kalınır.

**API sunucusu (deneysel)** (`src/sunucu.ts`): Logseq'in kendi arayüzü sunucuyu `window.apis.doAction` ile yönetir (`ipc :server/do :restart` / `:stop`). Eklenti özellik algılamayla `window.top.apis.doAction` varsa `["server/do", "restart"|"stop", "js-obj"]` dizisini çağırır (asar'daki `decode-main-ipc-message` dizi biçimini kabul eder); yoksa, erişim `SecurityError` verirse ya da çağrı hata verirse "Logseq bu sürümde izin vermiyor; Logseq'in kendi sunucu menüsünü kullanın." der. İstek gittikten 1,5 sn sonra aracın `/durum` yanıtındaki `api=` satırıyla sunucunun gerçek durumunu bildirir. **Bu kanal canlı doğrulanmadı**; eklenti iframe'i ana pencereden yalıtılmışsa kullanılamaz (uyarı yolu devreye girer).

## `Tepsi: aracı başlat` ve onay penceresi

Logseq'in `logseq.App.openExternalLink` işlevi yalnızca `http(s)` adreslerini açar; özel şemaları sessizce yok sayar. Bu yüzden eklenti `logseq-tepsi://baslat` adresini `window.open` ile açar. Logseq bu adresi yakalar ve `http`, `https`, `mailto` dışındaki şemalar için bir **onay penceresi** gösterir. Onaylarsanız Windows adresi kayıtlı uygulamaya (aracın kendisine) yönlendirir. Eklenti ardından birkaç saniye `/durum` ucunu yoklayıp sonucu bildirir.

## Yerel uç sözleşmesi

Adres `http://127.0.0.1:<port>`; her istekte `X-Logseq-Tepsi: 1` başlığı zorunludur (araç başlıksız istekleri reddeder).

| İstek | Yanıt |
| --- | --- |
| `GET /durum` | `200 text/plain`: `logseq-tepsi <sürüm>`, `gizli=<sayı>`, `logseq=<acik\|kapali>` satırları; araç ayrıca `api=<acik\|kapali>` satırı ekler (Logseq API sunucusu; eski yanıtlarda yoktur, eklenti ikisini de kabul eder) |
| `POST /gizle` | `204` |
| `POST /goster` | `204` |
| `GET /ayar` | `200 text/plain`: `anahtar=değer` satırları (`kucultme`, `tik`, `kapaliyken_tik_acar`, `kisayol_goster_gizle`, `oturumda_gizli_baslat`, `baslangicta_calis`, `kisayol_not`, `not_hedefi`, `kalinan_sayfa`, `api_otomatik_baslat`, `api_otomatik_bekleyen`) |
| `POST /ayar` | Gövde: bir ya da birkaç `anahtar=değer` satırı. Araç ayarı eşzamanlı uygular: `204`; gövde `api_otomatik_baslat` içeriyorsa `200` + `api_otomatik_baslat=yazildi\|beklemede\|hata` (+ hatada `api_otomatik_neden=...`); geçersizse (iki kısayolun aynı olması dahil; iki eşzamanlı istek arasındaki çakışma da) `400` + neden (gövdede); pencere sahibi zamanında yanıt vermezse `503` |

Eklentinin iframe'i `Origin: lsp://logseq.io` ile istek atar (bazı kurulumlarda `lsp://logseq.com`). Özel başlık gönderildiği için tarayıcı önce bir `OPTIONS` ön denetimi yapar; araç buna `Access-Control-Allow-Origin` (izinli kökenler için), `Access-Control-Allow-Headers: X-Logseq-Tepsi` ve `Access-Control-Allow-Methods: GET, POST, OPTIONS` ile yanıt vermelidir.

Hata sınıfları: `kapali` (bağlantı reddi; CORS ön denetiminin reddi de buna benzer görünür), `yasak` (403), `zaman-asimi` (3 sn), `beklenmedik`.

## Geliştirme

```sh
bun install --frozen-lockfile
bun test          # ağ katmanı, ayar eşlemesi ve eylemler (sahte fetch)
bun run build     # dist/main.js + dist/index.html
```

Ağ mantığı `src/ag.ts` içinde saftır; anında not `src/not.ts`, kalınan sayfa `src/kalinan.ts`, API başlat/durdur `src/sunucu.ts` içinde Logseq API'sini enjekte alan saf mantıktır; araç ayarlarının eklenti ayarlarıyla eşlemesi `src/ayar.ts` içinde saftır; eylemler `src/eylemler.ts` içinde Logseq'e bağımlılık almadan sınanır; `src/main.ts` yalnızca Logseq API'sine bağlar. `node_modules` ve `dist` depoya girmez; `dist` sürüm paketine (GitHub release zip'i) konur.

## Sınırlar

- Windows odaklıdır; tepsi aracının kendisi Windows'a özgüdür.
- Logseq içi kısayol değişikliği eklenti yeniden yüklenmeden uygulanmaz (genel kısayol hemen uygulanır).
- CORS ön denetimi reddedilirse eklenti bunu "araç çalışmıyor" diye gösterir (tarayıcı ikisini ayırt ettirmez).

## Lisans

MIT
