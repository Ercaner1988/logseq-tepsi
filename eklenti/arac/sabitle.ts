// Araç çubuğu düğmesini Logseq'in "sabitlenmiş" listesine ekler (preferences.json).
// Logseq'in kodu: sabit liste BOŞSA tüm eklenti düğmeleri görünür; doluysa yalnız listedekiler.
// Bu yüzden liste boşken ekleme yapmak diğer eklentilerin düğmelerini gizler; o durumda dokunulmaz.
// Logseq açıkken çalıştırılmaz: uygulama kapanırken tercihleri bellekteki hâliyle yeniden yazar.

export const SABIT_KIMLIK = "logseq-tepsi:logseq-tepsi-toolbar";

export type SabitleSonucu =
  | { durum: "eklendi"; json: string }
  | { durum: "zaten-var" }
  | { durum: "liste-bos" }
  | { durum: "bozuk"; neden: string };

export function sabitle(metin: string): SabitleSonucu {
  let tercihler: Record<string, unknown>;
  try {
    const ayrilmis: unknown = JSON.parse(metin);
    if (typeof ayrilmis !== "object" || ayrilmis === null || Array.isArray(ayrilmis)) {
      return { durum: "bozuk", neden: "kök nesne değil" };
    }
    tercihler = ayrilmis as Record<string, unknown>;
  } catch (e) {
    return { durum: "bozuk", neden: e instanceof Error ? e.message : String(e) };
  }
  const liste = tercihler.pinnedToolbarItems;
  if (!Array.isArray(liste) || liste.length === 0) return { durum: "liste-bos" };
  if (liste.includes(SABIT_KIMLIK)) return { durum: "zaten-var" };
  tercihler.pinnedToolbarItems = [...liste, SABIT_KIMLIK];
  const son = metin.endsWith("\n") ? "\n" : "";
  return { durum: "eklendi", json: JSON.stringify(tercihler, null, 2) + son };
}

if (import.meta.main) {
  const { copyFile, readFile, rename, writeFile } = await import("node:fs/promises");
  const yol = process.argv[2];
  if (!yol) {
    console.error("Kullanım: bun arac/sabitle.ts <preferences.json yolu>");
    process.exit(64);
  }
  let metin: string;
  try {
    metin = await readFile(yol, "utf8");
  } catch (e) {
    const kod = (e as NodeJS.ErrnoException).code;
    console.error(
      kod === "ENOENT"
        ? `preferences.json bulunamadı: ${yol}
Logseq bu bilgisayarda hiç açılmamış olabilir ya da yol yanlış.`
        : `preferences.json okunamadı (${yol}): ${e instanceof Error ? e.message : String(e)}`,
    );
    process.exit(1);
  }
  const s = sabitle(metin);
  switch (s.durum) {
    case "eklendi": {
      await copyFile(yol, `${yol}.tepsi-yedek`);
      await writeFile(`${yol}.tepsi-gecici`, s.json, "utf8");
      await rename(`${yol}.tepsi-gecici`, yol);
      console.log(`Düğme sabitlendi (yedek: ${yol}.tepsi-yedek).`);
      break;
    }
    case "zaten-var":
      console.log("Düğme zaten sabitli.");
      break;
    case "liste-bos":
      console.log("Sabit liste boş: tüm eklenti düğmeleri zaten görünür, dokunulmadı.");
      break;
    case "bozuk":
      console.error(`preferences.json okunamadı: ${s.neden}`);
      process.exit(1);
  }
}
