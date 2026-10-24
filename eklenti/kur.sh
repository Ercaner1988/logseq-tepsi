#!/bin/sh
# Tepsi eklentisini Logseq eklenti klasörüne kurar (geliştirici kipi gerekmez).
# Kullanım: sh kur.sh [--sabitle]
#   --sabitle  Araç çubuğu düğmesini Logseq'in sabit listesine de ekler (Logseq KAPALI olmalı).
# Ortam: LOGSEQ_DIZIN (varsayılan ~/.logseq)
set -eu

SABITLE=0
for a in "$@"; do
  case "$a" in
    --sabitle) SABITLE=1 ;;
    *) echo "Bilinmeyen seçenek: $a" >&2; exit 64 ;;
  esac
done

KAYNAK=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
DIZIN=${LOGSEQ_DIZIN:-$HOME/.logseq}
HEDEF=$DIZIN/plugins/logseq-tepsi

command -v bun >/dev/null 2>&1 || { echo "bun bulunamadı." >&2; exit 69; }

logseq_acik() {
  command -v tasklist >/dev/null 2>&1 && tasklist 2>/dev/null | grep -qi 'Logseq\.exe'
}

if [ "$SABITLE" = 1 ] && logseq_acik; then
  echo "Logseq açık: --sabitle için önce Logseq'i kapatın (tercihler kapanırken yeniden yazılır)." >&2
  exit 75
fi

cd "$KAYNAK"
bun install --frozen-lockfile
bun test
bun run build

# Klasör adı eklenti kimliğidir: Logseq ~/.logseq/plugins altındaki her klasörü kimliği
# klasör adı olarak yükler, o yüzden ad logseq-tepsi olmalı.
mkdir -p "$HEDEF"
rm -rf "$HEDEF/dist"
cp -R dist "$HEDEF/dist"
cp package.json icon.svg README.md LICENSE "$HEDEF/"
echo "Kuruldu: $HEDEF"

if [ "$SABITLE" = 1 ]; then
  bun arac/sabitle.ts "$DIZIN/preferences.json"
fi

if logseq_acik; then
  echo "Logseq açık: eklentiyi görmek için Logseq'i yeniden başlatın."
else
  echo "Logseq'i açtığınızda eklenti yüklenir."
fi
