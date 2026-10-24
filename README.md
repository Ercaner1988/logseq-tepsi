# logseq-tepsi

Logseq masaüstü uygulamasını küçültünce sistem tepsisine (bildirim alanı) gizleyen küçük Windows aracı. Logseq'in kendisinde tepsi özelliği yok; bu araç yalnız pencere işlemleri yapar (Logseq'e API, ağ ya da dosya erişimi yok).

## Ne yapar

- Tepsiye tek simge koyar (Logseq'in kendi simgesi, ipucu "Logseq").
- Logseq ana penceresi simge durumuna küçültülünce gizler (görev çubuğu düğmesi kalkar). Olay `SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART)` ile yakalanır; yoklama yok, boşta CPU sıfır.
- Simgeye sol tık: gizliyse göster + geri yükle + öne getir, görünürse gizle. Logseq hiç açık değilse `C:\Program Files\Logseq\Logseq.exe`'yi başlatır.
- Sağ tık menüsü: Göster, Tepsiye gizle, Çıkış. Çıkışta gizli pencere önce görünür yapılır.
- Kapatma (X) düğmesine dokunmaz: Logseq kapanır, araç ve simge kalır.
- Tek örnek (ikinci kopya hemen çıkar). Explorer yeniden başlarsa simge geri gelir.

## Kurulum

```
cargo install --path .
```

Başlangıçta açılması için `logseq-tepsi.exe` kısayolunu `shell:startup` klasörüne koyun (araç kendi kendine kısayol kurmaz).

## Sınama

```
cargo test
logseq-tepsi --sina-gizle-goster   # sol tık işlevini iki kez çağırır (gizle, göster)
logseq-tepsi --sina-kucult         # çalışan tepsi sürecini dışarıdan sınar: küçült, gizlenmeyi bekle, geri yükle
logseq-tepsi --sina-simge-tikla    # çalışan sürecin ileti penceresine tıklama yollar, simgenin kayıtlı olduğunu doğrular
```

Sınama komutları sonunda Logseq penceresini görünür ve geri yüklenmiş bırakır.

## Sınırlar

- Yalnız Windows; Logseq yolu sabit (`C:\Program Files\Logseq\Logseq.exe`).
- Araç görevi bitirilerek öldürülürse gizli pencere gizli kalabilir; yeniden başlatıp simgeye tıklayın.
