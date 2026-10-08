// Literaturstil: IEEE bzw. IEEE (deutsch) folgt der Dokumentsprache.
import { test } from "node:test";
import assert from "node:assert/strict";
import { ieeeStyleForLanguage, withLanguageBibliography } from "../src/latex/bibliographyStyle.ts";

const withStyle = (language, style) => ({ language, numberingDepth: 3, bibliography: { file: "", style } });

test("deutsche Sprachen erhalten IEEE (deutsch)", () => {
  assert.equal(ieeeStyleForLanguage("ngerman"), "ieee-de");
  assert.equal(ieeeStyleForLanguage("naustrian"), "ieee-de");
  assert.equal(ieeeStyleForLanguage("nswissgerman"), "ieee-de");
  assert.equal(ieeeStyleForLanguage("english"), "ieee");
  assert.equal(ieeeStyleForLanguage("french"), "ieee");
});

test("Sprachwechsel schaltet nur zwischen den IEEE-Varianten um", () => {
  const german = withStyle("ngerman", "ieee-de");
  assert.equal(withLanguageBibliography(german, { ...german, language: "english" }).bibliography.style, "ieee");
  const english = withStyle("english", "ieee");
  assert.equal(withLanguageBibliography(english, { ...english, language: "naustrian" }).bibliography.style, "ieee-de");
  // andere Stile bleiben unverändert
  const natbib = withStyle("ngerman", "plainnat");
  assert.equal(withLanguageBibliography(natbib, { ...natbib, language: "english" }).bibliography.style, "plainnat");
  // ohne Sprachwechsel bleibt eine bewusste Wahl erhalten
  const chosen = withStyle("ngerman", "ieee");
  assert.equal(withLanguageBibliography(chosen, { ...chosen, numberingDepth: 2 }).bibliography.style, "ieee");
});
