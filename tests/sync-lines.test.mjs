// SyncTeX-Zuordnung Zeile ↔ oberster Block (Doppelklick ins PDF, „Im PDF zeigen“).
import { test } from "node:test";
import assert from "node:assert/strict";
import { blockForLine, lineForBlock } from "../src/editor/syncLines.ts";

// Präambel bis Zeile 30; Blöcke: Überschrift 32, Absatz 34–35, leerer Block, Formel 37–39, Absatz 41
const lines = [[32, 32], [34, 35], [0, 0], [37, 39], [41, 41]];

test("Zeile → Block", () => {
  assert.equal(blockForLine(lines, 32), 0);
  assert.equal(blockForLine(lines, 35), 1);
  // Leerzeile nach dem Absatz (dort verbucht TeX ihn) gehört noch zum Absatz
  assert.equal(blockForLine(lines, 36), 1);
  assert.equal(blockForLine(lines, 38), 3);
  assert.equal(blockForLine(lines, 99), 4);
  // Präambel → erster Block
  assert.equal(blockForLine(lines, 5), 0);
  assert.equal(blockForLine([], 5), -1);
});

test("Block → Zeile", () => {
  assert.equal(lineForBlock(lines, 1), 34);
  // Block ohne Code → Block davor
  assert.equal(lineForBlock(lines, 2), 34);
  assert.equal(lineForBlock([[0, 0], [12, 14]], 0), 12);
  assert.equal(lineForBlock([[0, 0]], 0), null);
});
