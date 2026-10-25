# logseq-tepsi

Logseq masaüstü uygulamasını küçültünce sistem tepsisine (bildirim alanı) gizleyen küçük Windows aracı. Logseq'in kendisinde tepsi özelliği yok; araç pencere işlemleri yapar ve eklentinin konuşabileceği yalnız-yerel bir uç açar (Logseq'e başka API ya da dış ağ erişimi yok).

## Ne yapar

- Tepsiye tek simge koyar (Logseq'in kendi simgesi, ipucu "Logseq"). Logseq kapalıyken simge soluk (gri, yarı saydam) görünür ve ipucu "Logseq kapalı, tıklayınca açılır" der (tıklamak Logseq'i açar; ipucu `tik` ve `kapaliyken_tik_acar` ayarına göre değişir, kısayol kaydedilemediyse "kısayol kaydedilemedi" uyarısı ekler); durum 2 sn'de bir yoklanır, yalnız değişince simge tazelenir. Simge eklenemezse ya da küçültme kancası kurulamazsa günlüğe yazar, artan gecikmeyle (1 sn ... 30 sn) yeniden dener; kanca yokken ipucu "küçültme izlenemiyor" der.
- Logseq ana penceresi simge durumuna küçültülünce gizler (görev çubuğu düğmesi kalkar; `kucultme = gorev_cubugu` ise gizlemez). Olay `SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART)` ile yakalanır; yoklama yok, boşta CPU sıfır. Olay `IsIconic` tazelenmeden gelirse kısa gecikmeyle bir kez daha bakılır.
- Çok pencere: her pencere ayrı hatırlanır (gizlenenler kümesi). Sol tık: görünür pencere varsa hepsini gizler; yoksa en son gizleneni gösterir (geri yükler, öne getirir); Logseq hiç açık değilse Logseq'i başlatır (`kapaliyken_tik_acar = hayir` ise başlatmaz). Varsayılan `tik = tek` kipinde çift tık tek tık sayılır; `tik = cift` kipinde yalnız çift tık göster/gizle yapar, tek tık hiçbir şey yapmaz (seçilmeyen tık türü işe yaramaz).
- Açılışta tarama: araç Logseq açıkken başlarsa küçük pencereyi gizler (`kucultme = gorev_cubugu` iken gizlemez), zaten gizli olanı tepsiden geri getirilebilsin diye hatırlar.
- Sağ tık menüsü: Göster, Hepsini göster, Tepsiye gizle, Çıkış, Tepsi aracını kapat, Ayarlar alt menüsü (aşağıda). Çıkış tüm Logseq ana pencerelerini kapatır (gizliyse önce görünür yapılır, sonra `WM_CLOSE`); araç tepside kalır, simgeye tıklayınca Logseq yeniden açılır. "Tepsi aracını kapat" yalnız aracı kapatır, Logseq'e dokunmaz (gizliyse önce gösterir). Oturum kapanırken (`WM_ENDSESSION`) gizli pencere gösterilir.
- Kapatma (X) düğmesine dokunmaz: Logseq kapanır, araç ve simge kalır.
- Tek örnek: ikinci kopya hemen çıkar ama önce mevcut kopyaya "simgeyi yeniden ekle" iletisi yollar (simge, kanca ve açılış taraması tazelenir).
- Logseq yolu sabit değil. Sıra: çalışan ana pencerenin süreç yolu, `%LOCALAPPDATA%\logseq-tepsi\logseq-yolu.txt` (son bilinen yol, araç kendisi yazar), kayıt defteri Uninstall girdileri (HKCU, HKLM, WOW6432Node; `InstallLocation`, `DisplayIcon`, `UninstallString`; `DisplayVersion`ı en yüksek olan önce, sayısal karşılaştırma: `0.10.15` < `2.0.2`), son çare sabit yollar. Sürümü bilinen en yüksek kurulumdan düşük bir yol (örn. 2.x varken eski Squirrel) ayara yazılmaz, ayardan da okunmaz. İlk var olan kullanılır. Logseq ayrık süreç olarak başlatılır (`ELECTRON_RUN_AS_NODE` kaldırılır, stdio boş, DETACHED_PROCESS + yeni süreç grubu + izin varsa iş nesnesinden kopuk).
- Günlük: `%LOCALAPPDATA%\logseq-tepsi\gunluk.txt` (UTC, 64 KB'ta `gunluk.eski.txt`'ye döner; panik iletisi de buraya).
- DPI: çalışma zamanında `PER_MONITOR_AWARE_V2` istenir (manifest yok).

## Ayarlar

Tek kaynak: `%LOCALAPPDATA%\logseq-tepsi\ayar.toml` (TOML; değerler dizge, `evet`/`hayir`). Tepsi menüsündeki **Ayarlar** alt menüsü, eklenti ayarları ve `GET/POST /ayar` aynı ayarları okur/yazar; seçim anında uygulanır ve kaydedilir (geçici dosya + yeniden adlandırma ile atomik yazma).

| Anahtar | Değerler (varsayılan ilk) | Ne yapar |
| --- | --- | --- |
| `kucultme` | `tepsi`, `gorev_cubugu` | `tepsi`: küçültünce Logseq tepsiye gizlenir. `gorev_cubugu`: küçültme kancası gizlemez (menüdeki "Tepsiye gizle" ve `POST /gizle` yine gizler) |
| `tik` | `tek`, `cift` | Tepsi simgesinde hangi tık göster/gizle yapar; seçilmeyen tık türü hiçbir şey yapmaz. Sağ tık menüsü her kipte açılır |
| `kapaliyken_tik_acar` | `evet`, `hayir` | Logseq kapalıyken simge tıkı Logseq'i başlatsın mı (menüdeki Göster ve genel kısayol her zaman başlatır) |
| `kisayol_goster_gizle` | `Ctrl+Alt+L`; boş = kapalı | Genel kısayol (`RegisterHotKey`): görünürse gizler, gizliyse gösterir, kapalıysa Logseq'i başlatır. `Ctrl`, `Alt`, `Shift`, `Win` + harf/rakam/`F1`-`F24`/`Space`/`Enter`/`Tab`/`Esc`/ok ve gezinme tuşları; büyük/küçük harf duyarsız, yalnız ASCII; en az bir `Ctrl`/`Alt`/`Win` gerekir (`Shift` tek başına yalnız F tuşuyla). Dosyada ve uçta kurallı yazımla (`Ctrl+Alt+L`) saklanır. Kayıt başarısızsa (başka program kullanıyor) günlüğe yazılır ve ipucu uyarır; ayar değişince eski kayıt silinir. Menüden kapatınca boşalır, açınca varsayılan yazılır |
| `oturumda_gizli_baslat` | `hayir`, `evet` | Yalnız araç başlangıç kısayolundan `--oturum` argümanıyla başlarsa: Logseq açık değilse başlatır, ana penceresi görününce (en çok 90 sn, 1 sn aralıkla yoklanır) tepsiye gizler. Elle başlatmada etkisizdir |
| `baslangicta_calis` | `evet`, `hayir` | Dosyada TUTULMAZ: başlangıç kısayolunun (`Startup\Logseq-tepsi.lnk`) varlığıdır. Menüdeki "Bilgisayar açılınca başlat" işareti ve `GET /ayar` bunu gösterir; değiştirmek kısayolu kurar/siler (URL şemasına dokunmaz) |

Dosya eksikse varsayılanlarla çalışılır (günlüğe yazılır). Dosya bozuksa (TOML hatası ya da geçersiz değer) `ayar.toml.bozuk` adıyla kenara alınır, varsayılanlarla çalışılır ve günlüğe yazılır; kenara alınamazsa dosyanın üstüne hiç yazılmaz. Bilinmeyen anahtar uyarıyla atlanır (ilk kayıtta dosyadan düşer).

## Yerel uç (eklenti için, sözleşme sabit)

`127.0.0.1:47123` (yalnız döngü adresi; bağlanamazsa günlüğe yazılır, araç çalışır). Ayrı iş parçacığı, bloklayıcı `accept` (boşta CPU sıfır). En küçük HTTP/1.1 ayrıştırıcı: istek satırı ve başlıklar, gövde yalnız yetkili `POST /ayar` için okunur (öbür isteklerde yok sayılır), başlık bloğu en çok 8 KB, toplam okuma 2 sn, her bağlantı kapatılır.

| İstek | Yanıt |
| --- | --- |
| `GET /durum` | 200 `text/plain; charset=utf-8`: `logseq-tepsi <sürüm>\ngizli=<sayı>\nlogseq=<acik\|kapali>\n` |
| `POST /gizle` | 204; görünür tüm Logseq ana pencerelerini tepsiye gizler |
| `POST /goster` | 204; tepsideki tüm ana pencereleri gösterir, en son gizlenen öne gelir |
| `GET /ayar` | 200 `text/plain; charset=utf-8`: `anahtar=değer` satırları (`kucultme`, `tik`, `kapaliyken_tik_acar`, `kisayol_goster_gizle`, `oturumda_gizli_baslat`, `baslangicta_calis`; sabit sıra) |
| `POST /ayar` | Gövde: bir ya da birkaç `anahtar=değer` satırı (`Content-Length` zorunlu, en çok 1024 bayt). Hepsi geçerliyse 204 ve ileti penceresine devredilir (uygulama ve kayıt pencere iş parçacığında; yanıt işlenmeden önce döner); biri geçersizse hiçbiri uygulanmaz, 400 + neden (gövdede). `Content-Length` yoksa 411, 1024'ten büyükse 413 |
| `OPTIONS` (ön uçuş) | izinli Origin ise 204 + `Access-Control-Allow-Origin: <aynı origin>`, `Access-Control-Allow-Methods: GET, POST, OPTIONS`, `Access-Control-Allow-Headers: X-Logseq-Tepsi`, `Access-Control-Allow-Private-Network: true`; değilse 403 |

Güvenlik: OPTIONS dışındaki her istek `X-Logseq-Tepsi: 1` taşımalı, yoksa 403. `Origin` varsa izin listesinde olmalı (yalnız `lsp://logseq.io`, `lsp://logseq.com`; `cekirdek/src/uc.rs` `IZINLI_ORIGINLER`; `null` ve `file://` bilerek yok: sandbox iframe ve yerel HTML dosyaları da onu gönderir). Origin göndermeyen yerel süreç (curl gibi) özel başlıkla geçer, değilse 403. `Host` varsa `127.0.0.1:47123` ya da `localhost:47123` olmalı (DNS yeniden bağlama savunması; sözleşmeye ek). İzinli yanıtlarda `Access-Control-Allow-Origin` yankılanır. Bilinmeyen yol 404, yanlış metot 405, bozuk istek 400, büyük istek 431, zaman aşımı 408. Pencere ve ayar işleri HTTP iş parçacığında yapılmaz: ileti penceresine `PostMessageW` ile devredilir (`/ayar` değerleri bir kuyruktan okunur). `/ayar` için de aynı güvenlik denetimleri geçerlidir; yetkisiz isteğin gövdesi okunmaz.

## Kurulum

```
cargo install --path .
logseq-tepsi kur
```

`kur` (idempotent, ne yaptığını yazar): `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Logseq-tepsi.lnk` başlangıç kısayolunu `--oturum` argümanıyla oluşturur (`oturumda_gizli_baslat` yalnız bu argümanla başlayınca geçerlidir); kısayol varsa ve aynı exe'yi aynı argümanla gösteriyorsa dokunmaz, başka exe'yi gösteriyorsa, argümanı farklıysa (eski kurulum) ya da bozuksa yeniden yazar ve bunu söyler; `HKCU\Software\Classes\logseq-tepsi` URL şemasını yazar (`"<exe>" --url "%1"`). `--url` argümanı yok sayılır: yalnız aracı başlatır (çalışıyorsa mevcut kopyaya "simgeyi yeniden ekle" gider). Görev Zamanlayıcı kullanılmaz.

`logseq-tepsi kaldir`: yalnız kendi eklediği kısayolu ve şema anahtarını siler.

## Sınama

```
cargo test                        # çekirdek ve ikili crate (workspace)
logseq-tepsi --sina-gizle-goster   # sol tık işlevini iki kez çağırır (gizle, göster)
logseq-tepsi --sina-kucult         # çalışan tepsi sürecini dışarıdan sınar: küçült, gizlenmeyi bekle, geri yükle
logseq-tepsi --sina-simge-tikla    # çalışan sürecin ileti penceresine tıklama yollar, simgenin kayıtlı olduğunu doğrular
logseq-tepsi --sina-tik            # tek sol tık iletisi yollar (Logseq açık olmasa da)
logseq-tepsi --sina-pencereler     # Logseq ana pencerelerini ve durumlarını yazar
```

Sınama komutları sonunda Logseq penceresini görünür ve geri yüklenmiş bırakır. Alt komutlar çıktıyı üst sürecin konsoluna yazar.

## Yapı

- `cekirdek/` (`logseq-tepsi-cekirdek`): Win32'den bağımsız saf kod ve sınamaları: karar tabloları (`mantik`), ayara bağlı davranış kararları (`davranis`), gizlenenler kümesi (`gizlenen`), yol adayları ve kayıt değeri ayrıştırma (`yollar`), ayarlar yapısı ve `ayar.toml` okuma/yazma (`ayar`), genel kısayol dizgesi ayrıştırıcısı (`kisayol`), Ayarlar alt menüsü modeli (`menu`), günlük biçimi (`gunluk`), komut satırı (`komut`).
- `uc/` (`logseq-tepsi-uc`): yerel uç: HTTP ayrıştırıcı ve yetki kararı (`uc`), TCP sunucusu (`sunucu`), `/durum` önbelleği (`onbellek`).
- `sistem/` (`logseq-tepsi-sistem`): Windows kurulum katmanı: kayıt defteri (`kayit`), başlangıç kısayolu (COM) ve `kur`/`kaldir` (`kurulum`).
- `src/`: Win32 kabuğu (pencere, tepsi, genel kısayol kaydı `tus`, ayarların uygulanması `ayarlar`, TCP sunucusuna köprü).

## Sınırlar

- Yalnız Windows.
- Araç görevi bitirilerek öldürülürse gizli pencere gizli kalabilir; yeniden başlatınca açılış taraması gizli pencereyi hatırlar, simgeye tıklayınca geri gelir.
- Birden çok Logseq ana penceresi birim sınamalıdır (karar tabloları); canlıda yalnız tek pencereyle ölçüldü.
- Tepsi simgesi sürüm 0 geri çağrısı kullanır (`NIM_SETVERSION` yok): klavye ile (Enter) seçme çalışmaz. Sürüm 4'e geçilirse `mantik::tepsi_olayi` birlikte değişmeli.
- Kayıt defterinde birden çok Logseq kurulumu varsa, çalışan pencere ve ayar dosyası yoksa ilk bulunan (HKCU, sonra HKLM) denenir.
