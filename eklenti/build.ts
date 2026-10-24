// Tek dosyalı paket: dist/main.js + dist/index.html (Logseq eklenti girişi).
import { mkdir, rm } from "node:fs/promises";

await rm("dist", { recursive: true, force: true });
await mkdir("dist", { recursive: true });

const sonuc = await Bun.build({
  entrypoints: ["src/main.ts"],
  outdir: "dist",
  target: "browser",
  format: "esm",
  minify: true,
  naming: "main.js",
});
if (!sonuc.success) {
  for (const g of sonuc.logs) console.error(g);
  process.exit(1);
}
await Bun.write("dist/index.html", Bun.file("index.html"));
console.log("dist/main.js + dist/index.html hazır");
