// Symbolleiste für den Schnellzugriff: Bereinigen, Ein-/Ausblenden, Verschieben.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_QUICK_ACCESS,
  QUICK_ACCESS_LIMIT,
  QUICK_ACCESS_SUGGESTIONS,
  moveQuickAccess,
  normalizeQuickAccess,
  quickAccessCommands,
  toggleQuickAccess,
} from "../src/shortcuts/quickAccess.ts";

test("gespeicherte Liste wird bereinigt", () => {
  assert.deepEqual(normalizeQuickAccess(undefined), DEFAULT_QUICK_ACCESS);
  assert.deepEqual(normalizeQuickAccess("kaputt"), DEFAULT_QUICK_ACCESS);
  assert.deepEqual(normalizeQuickAccess(["file.save", "gibt.es.nicht", "file.save", 3, "edit.undo"]), ["file.save", "edit.undo"]);
  // Befehle des Folien-Editors gehören nicht in die Titelleiste des Dokuments
  assert.deepEqual(normalizeQuickAccess(["slides.present", "file.share"]), ["file.share"]);
  assert.deepEqual(normalizeQuickAccess([]), []);
});

test("ein- und ausblenden, Höchstzahl", () => {
  assert.deepEqual(toggleQuickAccess(DEFAULT_QUICK_ACCESS, "file.share"), [...DEFAULT_QUICK_ACCESS, "file.share"]);
  assert.deepEqual(toggleQuickAccess(["file.save", "edit.undo"], "file.save"), ["edit.undo"]);
  assert.deepEqual(toggleQuickAccess(["file.save"], "unbekannt"), ["file.save"]);
  const all = quickAccessCommands().map((command) => command.id);
  const full = all.slice(0, QUICK_ACCESS_LIMIT);
  assert.deepEqual(toggleQuickAccess(full, all[QUICK_ACCESS_LIMIT]), full);
});

test("verschieben", () => {
  const list = ["a", "b", "c"];
  assert.deepEqual(moveQuickAccess(list, "c", -1), ["a", "c", "b"]);
  assert.deepEqual(moveQuickAccess(list, "a", -1), list);
  assert.deepEqual(moveQuickAccess(list, "a", 2), ["b", "c", "a"]);
  assert.deepEqual(moveQuickAccess(list, "x", 1), list);
});

test("alle Vorschläge sind gültige Befehle", () => {
  const ids = new Set(quickAccessCommands().map((command) => command.id));
  for (const id of [...QUICK_ACCESS_SUGGESTIONS, ...DEFAULT_QUICK_ACCESS]) assert.ok(ids.has(id), id);
});
