// Prüft, dass das Editor-Schema (src/editor/schema.ts) genau die Dokumente
// akzeptiert, die der Rust-Kern erzeugt bzw. erwartet (Volltest-Dokument),
// und dass Suchen & Ersetzen Treffer über Formatgrenzen hinweg findet.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { getSchema } from "@tiptap/core";
import { Node } from "@tiptap/pm/model";
import { baseExtensions } from "../src/editor/schema.ts";
import { findMatches } from "../src/editor/search.ts";

const schema = getSchema(baseExtensions());
const fixture = JSON.parse(readFileSync(new URL("./fixtures/full-document.json", import.meta.url), "utf8"));

test("Volltest-Dokument ist im Editor-Schema gültig", () => {
  const doc = Node.fromJSON(schema, fixture);
  doc.check();
  const types = new Set();
  doc.descendants((node) => {
    types.add(node.type.name);
  });
  for (const expected of [
    "titlePage", "frontmatterBlock", "directoryBlock", "mathBlock", "tikzBlock", "table", "image", "citation", "footnote",
    "acronym", "crossReference", "pageBreak", "rawLatexBlock", "quantity", "verticalSpace", "environmentBlock",
    "appendixMarker", "codeBlock",
  ]) {
    assert.ok(types.has(expected), `${expected} fehlt`);
  }
});

test("Quelltreue-Attribute und Stilangaben bleiben im Editor erhalten", () => {
  const doc = Node.fromJSON(schema, {
    type: "doc",
    content: [
      { type: "paragraph", attrs: { sourceLatex: "Text\nmit Umbruch", sourceKey: "abc", sourceTight: true }, content: [{ type: "text", text: "Text mit Umbruch" }] },
      {
        type: "table",
        attrs: { caption: "T", tableStyle: "booktabs", columnSpec: "@{}ll@{}", rowRules: ["\\toprule", "\\midrule", "\\bottomrule"], captionLatex: true },
        content: [{ type: "tableRow", content: [{ type: "tableCell", content: [{ type: "paragraph" }] }, { type: "tableCell", content: [{ type: "paragraph" }] }] }],
      },
      { type: "orderedList", attrs: { listOptions: "label=\\alph*)" }, content: [{ type: "listItem", attrs: { itemLabel: "x)" }, content: [{ type: "paragraph" }] }] },
      { type: "maketitle" },
      { type: "includeBlock", attrs: { file: "kapitel1.tex", command: "input" }, content: [{ type: "paragraph" }] },
    ],
  });
  doc.check();
  const json = doc.toJSON();
  assert.equal(json.content[0].attrs.sourceLatex, "Text\nmit Umbruch");
  assert.equal(json.content[0].attrs.sourceTight, true);
  assert.deepEqual(json.content[1].attrs.rowRules, ["\\toprule", "\\midrule", "\\bottomrule"]);
  assert.equal(json.content[2].attrs.listOptions, "label=\\alph*)");
  assert.equal(json.content[2].content[0].attrs.itemLabel, "x)");
});

test("Round-Trip über JSON verliert keine Attribute", () => {
  const doc = Node.fromJSON(schema, fixture);
  const again = Node.fromJSON(schema, doc.toJSON());
  assert.ok(doc.eq(again));
});

test("Suche findet Text über Markengrenzen und respektiert Optionen", () => {
  const doc = Node.fromJSON(schema, {
    type: "doc",
    content: [
      { type: "paragraph", content: [{ type: "text", text: "Mess" }, { type: "text", text: "wert", marks: [{ type: "bold" }] }, { type: "text", text: " und Messwerte" }] },
      { type: "paragraph", content: [{ type: "text", text: "MESSWERT" }] },
    ],
  });
  assert.equal(findMatches(doc, "messwert", { caseSensitive: false, wholeWord: false }).length, 3);
  assert.equal(findMatches(doc, "Messwert", { caseSensitive: true, wholeWord: false }).length, 2);
  assert.equal(findMatches(doc, "messwert", { caseSensitive: false, wholeWord: true }).length, 2);
  const [first] = findMatches(doc, "Messwert", { caseSensitive: true, wholeWord: false });
  assert.equal(doc.textBetween(first.from, first.to), "Messwert");
});

test("Formatierungszeichen: Absatzmarken, Leerzeichen, geschützte Leerzeichen, Zeilenumbrüche", async () => {
  const { decorate } = await import("../src/editor/formattingMarks.ts");
  const doc = Node.fromJSON(schema, {
    type: "doc",
    content: [
      { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Titel" }] },
      { type: "paragraph", content: [{ type: "text", text: "a b c" }, { type: "hardBreak" }, { type: "text", text: "d" }] },
      { type: "paragraph" },
      { type: "codeBlock", content: [{ type: "text", text: "x y" }] },
    ],
  });
  const decorations = decorate(doc, 0, doc.content.size);
  const count = (predicate) => decorations.filter(predicate).length;
  assert.equal(count((d) => d.spec.key === "vtx-mark-paragraph"), 3, "Überschrift, Absatz, leerer Absatz – Code ohne ¶");
  assert.equal(count((d) => d.spec.key === "vtx-mark-break"), 1);
  assert.equal(count((d) => d.type.attrs?.class === "vtx-mark-nbsp"), 1);
  // Leerzeichen: „a b“ und „x y“ im Code
  assert.equal(count((d) => d.type.attrs?.class === "vtx-mark-space"), 2);
});
