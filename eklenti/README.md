# Tepsi (Logseq eklentisi)

Logseq penceresini Windows bildirim alanına (tepsiye) gizleyen [logseq-tepsi](../README.md) aracını Logseq'in içinden yönetir. Eklenti pencereyi kendisi gizleyemez (eklentiler bir iframe içinde çalışır); aracın yerel ucuna (`127.0.0.1`) istek atar.

## Ne yapar

- **Araç çubuğu düğmesi**: tıklayınca Logseq'i tepsiye gizler.
- **Komut paleti** (`Ctrl+Shift+P`):
  - `Tepsi: Logseq'i tepsiye gizle`
  - `Tepsi: durum` (araç çalışıyor mu, sürüm, gizli pencere sayısı)
  - `Tepsi: aracı başlat` (araç kapalıysa başlatmayı dener)
- **Klavye kısayolu**: varsayılan `Ctrl+Shift+X` (Mac'te `Cmd+Shift+X`), ayarlardan değişir.
- **Açılış denetimi**: Logseq açılırken araç çalışmıyorsa bir kez uyarır; ayarlardan kapatılabilir.

## Gereksinim

`logseq-tepsi` aracı çalışıyor olmalı. Eklenti araç olmadan da yüklenir, yalnızca uyarı verir. Aracı bir kez `logseq-tepsi kur` ile kaydederseniz `Tepsi: aracı başlat` komutu çalışır (araç `logseq-tepsi://baslat` adresini kaydeder).

## Kurulum

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

"(araç)" ile biten ayarlar tepsi aracının ayarlarıdır: tek kaynak aracın `ayar.toml` dosyasıdır. Eklenti açılışta `GET /ayar` ile bunları okuyup ayar sayfasını araçtakine eşitler; sayfada değiştirdiğinizde `POST /ayar` ile araca yazar (araçtan gelen eşitleme geri yazmaz, döngü olmaz). Araç çalışmıyorsa değişiklik iletilemez: eklenti bunu bir uyarıyla bildirir ve ayar, eklenti bir sonraki açılışta araçtaki değere döner. Araç değeri reddederse (örn. geçersiz kısayol) nedeni gösterilir ve ayar araçtaki değere geri alınır.

## `Tepsi: aracı başlat` ve onay penceresi

Logseq'in `logseq.App.openExternalLink` işlevi yalnızca `http(s)` adreslerini açar; özel şemaları sessizce yok sayar. Bu yüzden eklenti `logseq-tepsi://baslat` adresini `window.open` ile açar. Logseq bu adresi yakalar ve `http`, `https`, `mailto` dışındaki şemalar için bir **onay penceresi** gösterir. Onaylarsanız Windows adresi kayıtlı uygulamaya (aracın kendisine) yönlendirir. Eklenti ardından birkaç saniye `/durum` ucunu yoklayıp sonucu bildirir.

## Yerel uç sözleşmesi

Adres `http://127.0.0.1:<port>`; her istekte `X-Logseq-Tepsi: 1` başlığı zorunludur (araç başlıksız istekleri reddeder).

| İstek | Yanıt |
| --- | --- |
| `GET /durum` | `200 text/plain`: `logseq-tepsi <sürüm>`, `gizli=<sayı>`, `logseq=<acik\|kapali>` satırları |
| `POST /gizle` | `204` |
| `POST /goster` | `204` |
| `GET /ayar` | `200 text/plain`: `anahtar=değer` satırları (`kucultme`, `tik`, `kapaliyken_tik_acar`, `kisayol_goster_gizle`, `oturumda_gizli_baslat`, `baslangicta_calis`) |
| `POST /ayar` | Gövde: bir ya da birkaç `anahtar=değer` satırı; `204`, geçersizse `400` + neden (gövdede) |

Eklentinin iframe'i `Origin: lsp://logseq.io` ile istek atar (bazı kurulumlarda `lsp://logseq.com`). Özel başlık gönderildiği için tarayıcı önce bir `OPTIONS` ön denetimi yapar; araç buna `Access-Control-Allow-Origin` (izinli kökenler için), `Access-Control-Allow-Headers: X-Logseq-Tepsi` ve `Access-Control-Allow-Methods: GET, POST, OPTIONS` ile yanıt vermelidir.

Hata sınıfları: `kapali` (bağlantı reddi; CORS ön denetiminin reddi de buna benzer görünür), `yasak` (403), `zaman-asimi` (3 sn), `beklenmedik`.

## Geliştirme

```sh
bun install --frozen-lockfile
bun test          # ağ katmanı, ayar eşlemesi ve eylemler (sahte fetch)
bun run build     # dist/main.js + dist/index.html
```

Ağ mantığı `src/ag.ts` içinde saftır; araç ayarlarının eklenti ayarlarıyla eşlemesi `src/ayar.ts` içinde saftır; eylemler `src/eylemler.ts` içinde Logseq'e bağımlılık almadan sınanır; `src/main.ts` yalnızca Logseq API'sine bağlar. `node_modules` ve `dist` depoya girmez; `dist` sürüm paketine (GitHub release zip'i) konur.

## Sınırlar

- Windows odaklıdır; tepsi aracının kendisi Windows'a özgüdür.
- Logseq içi kısayol değişikliği eklenti yeniden yüklenmeden uygulanmaz (genel kısayol hemen uygulanır).
- CORS ön denetimi reddedilirse eklenti bunu "araç çalışmıyor" diye gösterir (tarayıcı ikisini ayırt ettirmez).

## Lisans

MIT
