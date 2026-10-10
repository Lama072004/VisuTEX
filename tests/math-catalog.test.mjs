// Formel-Galerien: jedes Symbol und jede Vorlage rendert mit KaTeX; Code-Einfügen setzt den Cursor richtig.
import { test } from "node:test";
import assert from "node:assert/strict";
import katex from "katex";
import { COMMON_FORMULAS, STRUCTURES, SYMBOL_CATEGORIES, previewLatex, templateToCode } from "../src/math/mathCatalog.ts";

const render = (latex) => katex.renderToString(latex, { throwOnError: true, displayMode: true, strict: "ignore" });

test("alle Symbole rendern", () => {
  let count = 0;
  for (const category of SYMBOL_CATEGORIES) {
    assert.ok(category.symbols.length >= 10, category.id);
    for (const symbol of category.symbols) {
      assert.doesNotThrow(() => render(symbol.latex), `${category.id}: ${symbol.latex}`);
      count += 1;
    }
  }
  assert.ok(count > 200, `nur ${count} Symbole`);
});

test("alle Strukturvorlagen rendern (Vorschau und als Code)", () => {
  for (const group of STRUCTURES) {
    assert.doesNotThrow(() => render(group.icon), group.id);
    for (const section of group.sections) {
      for (const item of section.items) {
        assert.doesNotThrow(() => render(previewLatex(item.latex)), `${group.id}: ${item.latex}`);
        assert.doesNotThrow(() => render(templateToCode(item.latex, "x").text), `${group.id} (Code): ${item.latex}`);
        assert.ok(!/#[?@0]/.test(templateToCode(item.latex).text), `Platzhalter übrig: ${item.latex}`);
      }
    }
  }
});

test("häufige Formeln rendern", () => {
  assert.ok(COMMON_FORMULAS.length >= 10);
  for (const formula of COMMON_FORMULAS) assert.doesNotThrow(() => render(formula.latex), formula.title);
});

test("Code-Einfügen: Auswahl und Cursor", () => {
  assert.deepEqual(templateToCode(String.raw`\frac{#0}{#?}`), { text: String.raw`\frac{}{}`, caret: 6 });
  // mit Auswahl landet der Cursor im ersten leeren Feld (Nenner)
  assert.deepEqual(templateToCode(String.raw`\frac{#0}{#?}`, "a+b"), { text: String.raw`\frac{a+b}{}`, caret: 11 });
  assert.deepEqual(templateToCode(String.raw`\sqrt{#0}`, "2"), { text: String.raw`\sqrt{2}`, caret: 8 });
  assert.deepEqual(templateToCode(String.raw`\alpha`), { text: String.raw`\alpha`, caret: 6 });
});

test("nur Befehle aus amsmath/amssymb (Export kompiliert überall)", () => {
  const all = [
    ...SYMBOL_CATEGORIES.flatMap((category) => category.symbols.map((symbol) => symbol.latex)),
    ...STRUCTURES.flatMap((group) => group.sections.flatMap((section) => section.items.map((item) => item.latex))),
  ].join(" ");
  for (const forbidden of ["\\coloneqq", "\\degree", "\\mleft", "\\differentialD", "\\placeholder"]) {
    assert.ok(!all.includes(forbidden), forbidden);
  }
});

test("alle Katalogtexte sind übersetzt", async () => {
  const { readFileSync } = await import("node:fs");
  const texts = new Set();
  for (const category of SYMBOL_CATEGORIES) {
    texts.add(category.label);
    category.symbols.forEach((symbol) => symbol.title && texts.add(symbol.title));
  }
  for (const group of STRUCTURES) {
    texts.add(group.label);
    group.sections.forEach((section) => {
      texts.add(section.label);
      section.items.forEach((item) => texts.add(item.title));
    });
  }
  COMMON_FORMULAS.forEach((formula) => texts.add(formula.title));
  for (const code of ["en", "es", "fr", "it", "nl", "pl", "pt"]) {
    const locale = JSON.parse(readFileSync(new URL(`../src/i18n/locales/${code}.json`, import.meta.url), "utf8"));
    const missing = [...texts].filter((text) => !(text in locale));
    assert.deepEqual(missing, [], code);
  }
});
