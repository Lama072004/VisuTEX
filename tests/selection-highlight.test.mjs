// Markierung wie bei Text: Blöcke ohne eigenen Text (Skizzen, Formeln, Bilder …) und ganze
// Tabellen/Umgebungen werden hervorgehoben, wenn sie vollständig in einer Markierung liegen.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { getSchema } from "@tiptap/core";
import { AllSelection, EditorState, NodeSelection, TextSelection } from "@tiptap/pm/state";
import { baseExtensions } from "../src/editor/schema.ts";
import { selectionDecorations } from "../src/editor/selectionHighlight.ts";

const schema = getSchema(baseExtensions());
const paragraph = (text) => ({ type: "paragraph", content: [{ type: "text", text }] });
const doc = schema.nodeFromJSON({
  type: "doc",
  content: [
    paragraph("Anfang"),
    { type: "tikzBlock", attrs: { code: "\\draw (0,0) -- (1,1);" } },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Formel " },
        { type: "inlineMath", attrs: { latex: "x^2" } },
        { type: "hardBreak" },
        { type: "citation", attrs: { keys: "knuth1984" } },
      ],
    },
    { type: "mathBlock", attrs: { latex: "a+b" } },
    { type: "environmentBlock", attrs: { name: "tcolorbox" }, content: [paragraph("Box mit "), { type: "mathBlock", attrs: { latex: "c" } }] },
    paragraph("Ende"),
  ],
});

/** Hervorgehobene Knoten als „Typ:Klasse“ in Dokumentreihenfolge. */
function highlighted(selection) {
  const state = EditorState.create({ doc, selection });
  return selectionDecorations(state)
    .find()
    .sort((left, right) => left.from - right.from)
    .map((decoration) => `${state.doc.nodeAt(decoration.from).type.name}:${decoration.type.attrs.class}`);
}

test("alles in einer Markierung wird hervorgehoben, Container als Ganzes", () => {
  const all = highlighted(new AllSelection(doc));
  assert.deepEqual(all, [
    "tikzBlock:vtx-in-selection",
    "inlineMath:vtx-in-selection",
    "citation:vtx-in-selection",
    "mathBlock:vtx-in-selection",
    // Formel in der Box nicht einzeln – die Box wird als Ganzes eingefärbt
    "environmentBlock:vtx-in-selection vtx-whole",
  ]);
  // Text und Zeilenumbrüche markiert der Browser selbst
  assert.ok(!all.some((entry) => entry.startsWith("hardBreak") || entry.startsWith("paragraph")));
});

test("nur vollständig markierte Elemente; Klick-Auswahl und leere Auswahl ohne Färbung", () => {
  // von „Anfang“ bis in den Absatz mit der Formel – endet vor der Formel
  const formula = doc.child(0).nodeSize + doc.child(1).nodeSize + 1 + "Formel ".length;
  assert.deepEqual(highlighted(TextSelection.create(doc, 3, formula)), ["tikzBlock:vtx-in-selection"]);
  assert.deepEqual(highlighted(TextSelection.create(doc, 3, formula + 1)), ["tikzBlock:vtx-in-selection", "inlineMath:vtx-in-selection"]);
  assert.deepEqual(highlighted(TextSelection.create(doc, 3)), []);
  // ein einzeln angeklickter Block behält seinen Auswahlrahmen (keine Färbung)
  assert.deepEqual(highlighted(NodeSelection.create(doc, doc.child(0).nodeSize)), []);
});

test("Volltest-Dokument: alle Blöcke ohne eigenen Text werden bei Strg+A hervorgehoben", () => {
  const fixture = schema.nodeFromJSON(JSON.parse(readFileSync(new URL("./fixtures/full-document.json", import.meta.url), "utf8")));
  const state = EditorState.create({ doc: fixture, selection: new AllSelection(fixture) });
  const types = new Set(selectionDecorations(state).find().map((decoration) => state.doc.nodeAt(decoration.from).type.name));
  for (const type of ["titlePage", "directoryBlock", "image", "subfigures", "table", "mathBlock", "tikzBlock", "rawLatexBlock", "pageBreak", "verticalSpace", "horizontalRule", "environmentBlock"]) {
    assert.ok(types.has(type), `${type} nicht hervorgehoben`);
  }
});
