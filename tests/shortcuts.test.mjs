// Tastenkürzel: Normalisierung (auch deutsche Tastatur/AltGr), Konflikte, Umbelegung.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  SHORTCUT_COMMANDS,
  assignShortcut,
  bindingsFor,
  comboFromEvent,
  comboProblem,
  conflictFor,
  formatCombo,
  normalizeCombo,
  resolveBindings,
} from "../src/shortcuts/shortcuts.ts";

const key = (key, code, mods = {}) => ({ key, code, ctrlKey: false, altKey: false, shiftKey: false, ...mods });

test("Tastenereignisse werden normalisiert", () => {
  assert.equal(comboFromEvent(key("s", "KeyS", { ctrlKey: true })), "Ctrl+S");
  assert.equal(comboFromEvent(key("S", "KeyS", { ctrlKey: true, shiftKey: true })), "Ctrl+Shift+S");
  // QWERTZ: Taste „Z“ liegt auf KeyY – maßgeblich ist das Zeichen.
  assert.equal(comboFromEvent(key("z", "KeyY", { ctrlKey: true })), "Ctrl+Z");
  // Umschalt+Ziffer liefert „!“ – maßgeblich ist die Ziffer.
  assert.equal(comboFromEvent(key("!", "Digit1", { ctrlKey: true, shiftKey: true })), "Ctrl+Shift+1");
  assert.equal(comboFromEvent(key("+", "BracketRight", { ctrlKey: true })), "Ctrl+Plus");
  assert.equal(comboFromEvent(key("F5", "F5")), "F5");
  assert.equal(comboFromEvent(key(" ", "Space", { ctrlKey: true })), "Ctrl+Space");
  assert.equal(comboFromEvent(key("Control", "ControlLeft", { ctrlKey: true })), null);
  // macOS: ⌘ zählt als Strg.
  assert.equal(comboFromEvent(key("b", "KeyB", { metaKey: true })), "Ctrl+B");
});

test("AltGr-Zeichen bleiben Texteingabe", () => {
  assert.equal(comboFromEvent(key("@", "KeyQ", { ctrlKey: true, altKey: true })), null);
  assert.equal(comboFromEvent(key("€", "KeyE", { ctrlKey: true, altKey: true })), null);
  assert.equal(comboFromEvent(key("{", "Digit7", { ctrlKey: true, altKey: true })), null);
  assert.equal(comboFromEvent(key("1", "Digit1", { ctrlKey: true, altKey: true })), "Ctrl+Alt+1");
  assert.equal(comboFromEvent(key("f", "KeyF", { ctrlKey: true, altKey: true })), "Ctrl+Alt+F");
});

test("Normalform und Anzeige", () => {
  assert.equal(normalizeCombo("shift+ctrl+s"), "Ctrl+Shift+S");
  assert.equal(normalizeCombo("Strg++"), "Ctrl+Plus");
  assert.equal(formatCombo("Ctrl+Shift+S", "de"), "Strg+Umschalt+S");
  assert.equal(formatCombo("Ctrl+Plus", "en"), "Ctrl++");
  assert.equal(formatCombo("Ctrl+,", "de"), "Strg+,");
});

test("Standardbelegung ist eindeutig und gültig", () => {
  const seen = new Map();
  for (const command of SHORTCUT_COMMANDS) {
    for (const combo of command.defaults) {
      assert.equal(normalizeCombo(combo), combo, `${command.id}: ${combo} nicht normalisiert`);
      assert.equal(comboProblem(combo), null, `${command.id}: ${combo}`);
      assert.ok(!seen.has(combo), `${combo} doppelt: ${seen.get(combo)} und ${command.id}`);
      seen.set(combo, command.id);
    }
  }
  assert.equal(new Set(SHORTCUT_COMMANDS.map((command) => command.id)).size, SHORTCUT_COMMANDS.length);
});

test("Reservierte und modifikatorlose Kombinationen werden abgelehnt", () => {
  assert.ok(comboProblem("Ctrl+C"));
  assert.ok(comboProblem("A"));
  assert.ok(comboProblem("Shift+A"));
  assert.equal(comboProblem("F2"), null);
  assert.equal(comboProblem("Alt+Q"), null);
});

test("Umbelegen entfernt die Kombination beim bisherigen Befehl", () => {
  let overrides = assignShortcut({}, "format.italic", ["Ctrl+B"]);
  assert.deepEqual(bindingsFor("format.italic", overrides), ["Ctrl+B"]);
  assert.deepEqual(bindingsFor("format.bold", overrides), []);
  assert.equal(resolveBindings(overrides).get("Ctrl+B"), "format.italic");
  assert.equal(resolveBindings(overrides).get("Ctrl+I"), undefined);
  assert.equal(conflictFor("Ctrl+B", overrides, "format.underline")?.id, "format.italic");
  // Zurück auf Standard → keine Überschreibung mehr gespeichert.
  overrides = assignShortcut(overrides, "format.italic", ["Ctrl+I"]);
  overrides = assignShortcut(overrides, "format.bold", ["Ctrl+B"]);
  assert.deepEqual(overrides, {});
});
