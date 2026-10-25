# logseq-tepsi

Logseq masaüstü uygulamasını küçültünce sistem tepsisine (bildirim alanı) gizleyen küçük Windows aracı. Logseq'in kendisinde tepsi özelliği yok; araç pencere işlemleri yapar ve eklentinin konuşabileceği yalnız-yerel bir uç açar (Logseq'e başka API ya da dış ağ erişimi yok).

## Ne yapar

- Tepsiye tek simge koyar (Logseq'in kendi simgesi, ipucu "Logseq"). Logseq kapalıyken simge soluk (gri, yarı saydam) görünür ve ipucu "Logseq kapalı, tıklayınca açılır" der (tıklamak Logseq'i açar); durum 2 sn'de bir yoklanır, yalnız değişince simge tazelenir. Simge eklenemezse ya da küçültme kancası kurulamazsa günlüğe yazar, artan gecikmeyle (1 sn ... 30 sn) yeniden dener; kanca yokken ipucu "küçültme izlenemiyor" der.
- Logseq ana penceresi simge durumuna küçültülünce gizler (görev çubuğu düğmesi kalkar). Olay `SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART)` ile yakalanır; yoklama yok, boşta CPU sıfır. Olay `IsIconic` tazelenmeden gelirse kısa gecikmeyle bir kez daha bakılır.
- Çok pencere: her pencere ayrı hatırlanır (gizlenenler kümesi). Sol tık: görünür pencere varsa hepsini gizler; yoksa en son gizleneni gösterir (geri yükler, öne getirir); Logseq hiç açık değilse Logseq'i başlatır. Çift tık tek tık sayılır.
- Açılışta tarama: araç Logseq açıkken başlarsa küçük pencereyi gizler, zaten gizli olanı tepsiden geri getirilebilsin diye hatırlar.
- Sağ tık menüsü: Göster, Hepsini göster, Tepsiye gizle, Çıkış, Tepsi aracını kapat. Çıkış tüm Logseq ana pencerelerini kapatır (gizliyse önce görünür yapılır, sonra `WM_CLOSE`); araç tepside kalır, simgeye tıklayınca Logseq yeniden açılır. "Tepsi aracını kapat" yalnız aracı kapatır, Logseq'e dokunmaz (gizliyse önce gösterir). Oturum kapanırken (`WM_ENDSESSION`) gizli pencere gösterilir.
- Kapatma (X) düğmesine dokunmaz: Logseq kapanır, araç ve simge kalır.
- Tek örnek: ikinci kopya hemen çıkar ama önce mevcut kopyaya "simgeyi yeniden ekle" iletisi yollar (simge, kanca ve açılış taraması tazelenir).
- Logseq yolu sabit değil. Sıra: çalışan ana pencerenin süreç yolu, `%LOCALAPPDATA%\logseq-tepsi\logseq-yolu.txt` (son bilinen yol, araç kendisi yazar), kayıt defteri Uninstall girdileri (HKCU, HKLM, WOW6432Node; `InstallLocation`, `DisplayIcon`, `UninstallString`; `DisplayVersion`ı en yüksek olan önce, sayısal karşılaştırma: `0.10.15` < `2.0.2`), son çare sabit yollar. Sürümü bilinen en yüksek kurulumdan düşük bir yol (örn. 2.x varken eski Squirrel) ayara yazılmaz, ayardan da okunmaz. İlk var olan kullanılır. Logseq ayrık süreç olarak başlatılır (`ELECTRON_RUN_AS_NODE` kaldırılır, stdio boş, DETACHED_PROCESS + yeni süreç grubu + izin varsa iş nesnesinden kopuk).
- Günlük: `%LOCALAPPDATA%\logseq-tepsi\gunluk.txt` (UTC, 64 KB'ta `gunluk.eski.txt`'ye döner; panik iletisi de buraya).
- DPI: çalışma zamanında `PER_MONITOR_AWARE_V2` istenir (manifest yok).

## Yerel uç (eklenti için, sözleşme sabit)

`127.0.0.1:47123` (yalnız döngü adresi; bağlanamazsa günlüğe yazılır, araç çalışır). Ayrı iş parçacığı, bloklayıcı `accept` (boşta CPU sıfır). En küçük HTTP/1.1 ayrıştırıcı: istek satırı ve başlıklar, gövde yok sayılır, başlık bloğu en çok 8 KB, toplam okuma 2 sn, her bağlantı kapatılır.

| İstek | Yanıt |
| --- | --- |
| `GET /durum` | 200 `text/plain; charset=utf-8`: `logseq-tepsi <sürüm>\ngizli=<sayı>\nlogseq=<acik\|kapali>\n` |
| `POST /gizle` | 204; görünür tüm Logseq ana pencerelerini tepsiye gizler |
| `POST /goster` | 204; tepsideki tüm ana pencereleri gösterir, en son gizlenen öne gelir |
| `OPTIONS` (ön uçuş) | izinli Origin ise 204 + `Access-Control-Allow-Origin: <aynı origin>`, `Access-Control-Allow-Methods: GET, POST, OPTIONS`, `Access-Control-Allow-Headers: X-Logseq-Tepsi`, `Access-Control-Allow-Private-Network: true`; değilse 403 |

Güvenlik: OPTIONS dışındaki her istek `X-Logseq-Tepsi: 1` taşımalı, yoksa 403. `Origin` varsa izin listesinde olmalı (yalnız `lsp://logseq.io`, `lsp://logseq.com`; `cekirdek/src/uc.rs` `IZINLI_ORIGINLER`; `null` ve `file://` bilerek yok: sandbox iframe ve yerel HTML dosyaları da onu gönderir). Origin göndermeyen yerel süreç (curl gibi) özel başlıkla geçer, değilse 403. `Host` varsa `127.0.0.1:47123` ya da `localhost:47123` olmalı (DNS yeniden bağlama savunması; sözleşmeye ek). İzinli yanıtlarda `Access-Control-Allow-Origin` yankılanır. Bilinmeyen yol 404, yanlış metot 405, bozuk istek 400, büyük istek 431, zaman aşımı 408. Pencere işleri HTTP iş parçacığında yapılmaz: ileti penceresine `PostMessageW` ile devredilir.

## Kurulum

```
cargo install --path .
logseq-tepsi kur
```

`kur` (idempotent, ne yaptığını yazar): `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Logseq-tepsi.lnk` başlangıç kısayolu yoksa oluşturur, aynı exe'yi gösteriyorsa dokunmaz, başka exe'yi gösteriyorsa (ya da bozuksa) yeniden yazar ve bunu söyler; `HKCU\Software\Classes\logseq-tepsi` URL şemasını yazar (`"<exe>" --url "%1"`). `--url` argümanı yok sayılır: yalnız aracı başlatır (çalışıyorsa mevcut kopyaya "simgeyi yeniden ekle" gider). Görev Zamanlayıcı kullanılmaz.

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

- `cekirdek/` (`logseq-tepsi-cekirdek`): Win32'den bağımsız saf kod ve sınamaları: karar tabloları (`mantik`), gizlenenler kümesi (`gizlenen`), yol adayları ve kayıt değeri ayrıştırma (`yollar`), yerel uç ayrıştırıcısı ve yetki kararı (`uc`), günlük biçimi (`gunluk`), komut satırı (`komut`).
- `src/`: Win32 kabuğu (pencere, tepsi, kayıt defteri, COM kısayolu, TCP iş parçacığı).

## Sınırlar

- Yalnız Windows.
- Araç görevi bitirilerek öldürülürse gizli pencere gizli kalabilir; yeniden başlatınca açılış taraması gizli pencereyi hatırlar, simgeye tıklayınca geri gelir.
- Birden çok Logseq ana penceresi birim sınamalıdır (karar tabloları); canlıda yalnız tek pencereyle ölçüldü.
- Tepsi simgesi sürüm 0 geri çağrısı kullanır (`NIM_SETVERSION` yok): klavye ile (Enter) seçme çalışmaz. Sürüm 4'e geçilirse `mantik::tepsi_olayi` birlikte değişmeli.
- Kayıt defterinde birden çok Logseq kurulumu varsa, çalışan pencere ve ayar dosyası yoksa ilk bulunan (HKCU, sonra HKLM) denenir.
