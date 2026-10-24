import { describe, expect, test } from "bun:test";
import { SABIT_KIMLIK, sabitle } from "../arac/sabitle";

const DOLU = '{\n  "theme": {"a": "😌"},\n  "pinnedToolbarItems": [\n    "x:y"\n  ]\n}\n';

describe("sabitle", () => {
  test("dolu listeye ekler, öteki alanları ve sondaki satırı korur", () => {
    const s = sabitle(DOLU);
    expect(s.durum).toBe("eklendi");
    if (s.durum !== "eklendi") return;
    const geri = JSON.parse(s.json);
    expect(geri.pinnedToolbarItems).toEqual(["x:y", SABIT_KIMLIK]);
    expect(geri.theme.a).toBe("😌");
    expect(s.json.endsWith("\n")).toBe(true);
  });
  test("zaten varsa dokunmaz", () => {
    const m = JSON.stringify({ pinnedToolbarItems: ["x:y", SABIT_KIMLIK] });
    expect(sabitle(m).durum).toBe("zaten-var");
  });
  test("liste boşsa ya da yoksa dokunmaz (yoksa öteki düğmeler gizlenirdi)", () => {
    expect(sabitle('{"pinnedToolbarItems": []}').durum).toBe("liste-bos");
    expect(sabitle("{}").durum).toBe("liste-bos");
  });
  test("bozuk JSON bildirilir", () => {
    expect(sabitle("{").durum).toBe("bozuk");
    expect(sabitle("[]").durum).toBe("bozuk");
  });
});
