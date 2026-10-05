/**
 * Tastenkürzel: Befehlsliste mit Standardbelegung, Normalisierung von
 * Tastenereignissen und Konfliktprüfung. Ohne JSX/DOM – in Node testbar
 * (`tests/shortcuts.test.mjs`).
 *
 * Kombinationen werden als Text gespeichert: Modifikatoren in fester Reihenfolge
 * `Ctrl+Alt+Shift+Taste`, z. B. `Ctrl+Shift+S`, `F5`, `Ctrl+Plus`. „Ctrl“ steht
 * für Strg bzw. ⌘ (macOS).
 */

/** `global`: überall (auch Code-Ansicht); `editor`: nur im visuellen Editor. */
export type ShortcutScope = "global" | "editor";

export type ShortcutCategory = "Datei" | "Bearbeiten" | "Format" | "Absatz" | "Einfügen" | "Ansicht" | "Folien";

export type ShortcutCommand = {
  id: string;
  label: string;
  category: ShortcutCategory;
  scope: ShortcutScope;
  defaults: string[];
};

export const SHORTCUT_CATEGORIES: ShortcutCategory[] = ["Datei", "Bearbeiten", "Format", "Absatz", "Einfügen", "Ansicht", "Folien"];

export const SHORTCUT_COMMANDS: ShortcutCommand[] = [
  { id: "file.new", label: "Neues Dokument", category: "Datei", scope: "global", defaults: ["Ctrl+N"] },
  { id: "file.open", label: "Öffnen", category: "Datei", scope: "global", defaults: ["Ctrl+O"] },
  { id: "file.save", label: "Speichern", category: "Datei", scope: "global", defaults: ["Ctrl+S"] },
  { id: "file.saveAs", label: "Speichern unter", category: "Datei", scope: "global", defaults: ["Ctrl+Shift+S"] },
  { id: "file.exportTex", label: "Als LaTeX exportieren", category: "Datei", scope: "global", defaults: [] },
  { id: "file.exportPdf", label: "Als PDF exportieren", category: "Datei", scope: "global", defaults: ["Ctrl+P"] },
  { id: "file.compile", label: "PDF kompilieren", category: "Datei", scope: "global", defaults: ["F5"] },
  { id: "file.options", label: "Optionen", category: "Datei", scope: "global", defaults: [] },
  { id: "file.shortcuts", label: "Tastenkürzel anpassen", category: "Datei", scope: "global", defaults: [] },

  { id: "edit.undo", label: "Rückgängig", category: "Bearbeiten", scope: "editor", defaults: ["Ctrl+Z"] },
  { id: "edit.redo", label: "Wiederholen", category: "Bearbeiten", scope: "editor", defaults: ["Ctrl+Y", "Ctrl+Shift+Z"] },
  { id: "edit.find", label: "Suchen", category: "Bearbeiten", scope: "editor", defaults: ["Ctrl+F"] },
  { id: "edit.replace", label: "Ersetzen", category: "Bearbeiten", scope: "editor", defaults: ["Ctrl+H"] },
  { id: "edit.formatPainter", label: "Format übertragen", category: "Bearbeiten", scope: "editor", defaults: ["Ctrl+Shift+C"] },

  { id: "format.bold", label: "Fett", category: "Format", scope: "editor", defaults: ["Ctrl+B"] },
  { id: "format.italic", label: "Kursiv", category: "Format", scope: "editor", defaults: ["Ctrl+I"] },
  { id: "format.underline", label: "Unterstrichen", category: "Format", scope: "editor", defaults: ["Ctrl+U"] },
  { id: "format.strike", label: "Durchgestrichen", category: "Format", scope: "editor", defaults: ["Ctrl+Shift+X"] },
  { id: "format.subscript", label: "Tiefgestellt", category: "Format", scope: "editor", defaults: ["Ctrl+,"] },
  { id: "format.superscript", label: "Hochgestellt", category: "Format", scope: "editor", defaults: ["Ctrl+."] },
  { id: "format.code", label: "Code (Schreibmaschine)", category: "Format", scope: "editor", defaults: [] },
  { id: "format.clear", label: "Formatierung löschen", category: "Format", scope: "editor", defaults: ["Ctrl+Space"] },

  { id: "paragraph.normal", label: "Standardtext", category: "Absatz", scope: "editor", defaults: ["Ctrl+Alt+0", "Ctrl+Shift+0"] },
  { id: "paragraph.heading1", label: "Überschrift 1", category: "Absatz", scope: "editor", defaults: ["Ctrl+Alt+1", "Ctrl+Shift+1"] },
  { id: "paragraph.heading2", label: "Überschrift 2", category: "Absatz", scope: "editor", defaults: ["Ctrl+Alt+2", "Ctrl+Shift+2"] },
  { id: "paragraph.heading3", label: "Überschrift 3", category: "Absatz", scope: "editor", defaults: ["Ctrl+Alt+3", "Ctrl+Shift+3"] },
  { id: "paragraph.alignLeft", label: "Linksbündig", category: "Absatz", scope: "editor", defaults: ["Ctrl+L", "Ctrl+Shift+L"] },
  { id: "paragraph.alignCenter", label: "Zentriert", category: "Absatz", scope: "editor", defaults: ["Ctrl+E", "Ctrl+Shift+E"] },
  { id: "paragraph.alignRight", label: "Rechtsbündig", category: "Absatz", scope: "editor", defaults: ["Ctrl+R", "Ctrl+Shift+R"] },
  { id: "paragraph.alignJustify", label: "Blocksatz", category: "Absatz", scope: "editor", defaults: ["Ctrl+J", "Ctrl+Shift+J"] },
  { id: "paragraph.bulletList", label: "Aufzählung", category: "Absatz", scope: "editor", defaults: ["Ctrl+Shift+8"] },
  { id: "paragraph.orderedList", label: "Nummerierung", category: "Absatz", scope: "editor", defaults: ["Ctrl+Shift+7"] },
  { id: "paragraph.quote", label: "Zitat", category: "Absatz", scope: "editor", defaults: ["Ctrl+Shift+B"] },
  { id: "paragraph.codeBlock", label: "Codeblock", category: "Absatz", scope: "editor", defaults: ["Ctrl+Alt+C"] },

  { id: "insert.pageBreak", label: "Seitenumbruch", category: "Einfügen", scope: "editor", defaults: ["Ctrl+Enter"] },
  { id: "insert.inlineMath", label: "Formel im Text", category: "Einfügen", scope: "editor", defaults: ["Ctrl+M"] },
  { id: "insert.blockMath", label: "Abgesetzte Formel", category: "Einfügen", scope: "editor", defaults: ["Ctrl+Shift+M"] },
  { id: "insert.link", label: "Link", category: "Einfügen", scope: "editor", defaults: ["Ctrl+K"] },
  { id: "insert.footnote", label: "Fußnote", category: "Einfügen", scope: "editor", defaults: ["Ctrl+Alt+F"] },
  { id: "insert.citation", label: "Zitat/Quelle (Literatur)", category: "Einfügen", scope: "editor", defaults: ["Ctrl+Alt+Z"] },
  { id: "insert.crossReference", label: "Querverweis", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.table", label: "Tabelle", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.image", label: "Bild", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.sketch", label: "Skizze", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.rawLatex", label: "LaTeX-Code", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.quantity", label: "Größe mit Einheit", category: "Einfügen", scope: "editor", defaults: [] },
  { id: "insert.codeListing", label: "Quellcode-Listing", category: "Einfügen", scope: "editor", defaults: [] },

  { id: "view.toggleCode", label: "Visuell ↔ Code umschalten", category: "Ansicht", scope: "global", defaults: ["Ctrl+Alt+V"] },
  { id: "view.togglePdf", label: "PDF-Vorschau ein/aus", category: "Ansicht", scope: "global", defaults: ["Ctrl+Alt+P"] },
  { id: "view.toggleOutline", label: "Navigation ein/aus", category: "Ansicht", scope: "global", defaults: [] },
  { id: "view.togglePaged", label: "Seitenansicht ein/aus", category: "Ansicht", scope: "global", defaults: [] },
  { id: "view.toggleRuler", label: "Lineal ein/aus", category: "Ansicht", scope: "global", defaults: [] },
  { id: "view.zoomIn", label: "Vergrößern", category: "Ansicht", scope: "global", defaults: ["Ctrl+Plus", "Ctrl+="] },
  { id: "view.zoomOut", label: "Verkleinern", category: "Ansicht", scope: "global", defaults: ["Ctrl+-"] },
  { id: "view.zoomReset", label: "Zoom 100 %", category: "Ansicht", scope: "global", defaults: ["Ctrl+0"] },
  { id: "view.slides", label: "Folien-Editor öffnen", category: "Ansicht", scope: "global", defaults: [] },

  { id: "slides.newSlide", label: "Neue Folie", category: "Folien", scope: "global", defaults: ["Ctrl+Shift+N"] },
  { id: "slides.textBox", label: "Textfeld einfügen", category: "Folien", scope: "global", defaults: [] },
  { id: "slides.duplicate", label: "Auswahl duplizieren", category: "Folien", scope: "global", defaults: ["Ctrl+D"] },
  { id: "slides.present", label: "Bildschirmpräsentation", category: "Folien", scope: "global", defaults: ["F11"] },
  { id: "slides.grid", label: "Raster ein/aus", category: "Folien", scope: "global", defaults: [] },
];

/** Nicht belegbar: Zwischenablage/Alles markieren und Kombinationen ohne Modifikator (außer F-Tasten). */
export const RESERVED_COMBOS = ["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+A", "Ctrl+Shift+V", "Alt+F4", "Ctrl+Shift+I"];

export type KeyLike = {
  key: string;
  code?: string;
  ctrlKey: boolean;
  metaKey?: boolean;
  altKey: boolean;
  shiftKey: boolean;
};

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph", "OS", "CapsLock", "Dead", "Unidentified"]);

/** Tastenname unabhängig von der Tastaturbelegung bei Ziffern, sonst das erzeugte Zeichen. */
function keyName(event: KeyLike): string | null {
  const { key, code = "" } = event;
  if (MODIFIER_KEYS.has(key)) return null;
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return code.slice(6);
  if (key === "+" || code === "NumpadAdd") return "Plus";
  if (code === "NumpadSubtract") return "-";
  if (key === " " || code === "Space") return "Space";
  if (key.length === 1) return key.toUpperCase();
  if (key === "Esc") return "Escape";
  return key;
}

/**
 * Normalisierte Kombination eines Tastenereignisses oder `null` (reiner Modifikator,
 * AltGr-Zeichen wie `@`, `€`, `{`). Mit Strg+Alt erzeugte Sonderzeichen bleiben so
 * Texteingabe – auf deutschen Tastaturen ist Strg+Alt = AltGr.
 */
export function comboFromEvent(event: KeyLike): string | null {
  const name = keyName(event);
  if (!name) return null;
  const ctrl = event.ctrlKey || Boolean(event.metaKey);
  if (ctrl && event.altKey && event.key.length === 1 && !/^[a-z0-9 ]$/i.test(event.key) && event.key !== "+") {
    return null;
  }
  const parts: string[] = [];
  if (ctrl) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  parts.push(name);
  return parts.join("+");
}

/** Bringt eine gespeicherte bzw. getippte Kombination in die Normalform. */
export function normalizeCombo(combo: string): string {
  const pieces = combo.split("+").map((piece) => piece.trim());
  // „Ctrl++“ → Taste „+“
  const key = combo.endsWith("++") ? "Plus" : pieces[pieces.length - 1];
  const modifiers = new Set(pieces.slice(0, -1).map((piece) => piece.toLowerCase()));
  const parts: string[] = [];
  if (modifiers.has("ctrl") || modifiers.has("strg") || modifiers.has("cmd") || modifiers.has("mod")) parts.push("Ctrl");
  if (modifiers.has("alt")) parts.push("Alt");
  if (modifiers.has("shift") || modifiers.has("umschalt")) parts.push("Shift");
  const normalizedKey = key.length === 1 ? key.toUpperCase() : key === "+" ? "Plus" : key;
  parts.push(normalizedKey);
  return parts.join("+");
}

/** Darf die Kombination belegt werden? Liefert sonst den Grund. */
export function comboProblem(combo: string): string | null {
  const normalized = normalizeCombo(combo);
  if (RESERVED_COMBOS.includes(normalized)) return "Diese Kombination ist für die Zwischenablage bzw. das System reserviert.";
  const pieces = normalized.split("+");
  const key = pieces[pieces.length - 1];
  const hasModifier = pieces.includes("Ctrl") || pieces.includes("Alt");
  if (!hasModifier && !/^F([1-9]|1[0-2])$/.test(key)) {
    return "Bitte eine Kombination mit Strg oder Alt verwenden (oder eine Funktionstaste F1–F12).";
  }
  return null;
}

/** Anzeige: „Strg+Umschalt+S“ (Deutsch) bzw. „Ctrl+Shift+S“ (Englisch). */
/** Tastennamen, wie sie auf der Tastatur der jeweiligen Sprache stehen (Rest: Englisch). */
const KEY_NAMES: Record<string, Record<string, string>> = {
  de: { Ctrl: "Strg", Shift: "Umschalt", Space: "Leertaste", Enter: "Eingabe", Delete: "Entf", Backspace: "Rücktaste", PageUp: "Bild ↑", PageDown: "Bild ↓" },
  en: { Space: "Space", Delete: "Del", PageUp: "PgUp", PageDown: "PgDn" },
  fr: { Shift: "Maj", Space: "Espace", Enter: "Entrée", Delete: "Suppr", Backspace: "Retour arrière", PageUp: "Pg préc", PageDown: "Pg suiv" },
  es: { Shift: "Mayús", Space: "Espacio", Enter: "Intro", Delete: "Supr", Backspace: "Retroceso", PageUp: "RePág", PageDown: "AvPág" },
  it: { Shift: "Maiusc", Space: "Spazio", Enter: "Invio", Delete: "Canc", Backspace: "Backspace", PageUp: "PagSu", PageDown: "PagGiù" },
  pt: { Shift: "Shift", Space: "Espaço", Enter: "Enter", Delete: "Del", PageUp: "PgUp", PageDown: "PgDn" },
  nl: { Space: "Spatie", Delete: "Del", PageUp: "PgUp", PageDown: "PgDn" },
  pl: { Space: "Spacja", Delete: "Del", PageUp: "PgUp", PageDown: "PgDn" },
};
const COMMON_KEYS: Record<string, string> = {
  Ctrl: "Ctrl", Shift: "Shift", Alt: "Alt", Plus: "+", Enter: "Enter", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→",
  Backspace: "Backspace", Escape: "Esc",
};

export function formatCombo(combo: string, language: string = "de"): string {
  const names = KEY_NAMES[language] ?? KEY_NAMES.en;
  return combo
    .split(/\+(?!$)/)
    .map((part) => names[part] ?? COMMON_KEYS[part] ?? part)
    .join("+");
}

/** Gespeicherte Abweichungen von der Standardbelegung: Befehl → Kombinationen. */
export type ShortcutOverrides = Record<string, string[]>;

export function bindingsFor(id: string, overrides: ShortcutOverrides): string[] {
  const command = SHORTCUT_COMMANDS.find((entry) => entry.id === id);
  const list = Object.prototype.hasOwnProperty.call(overrides, id) ? overrides[id] : command?.defaults ?? [];
  return list.map(normalizeCombo);
}

/** Kombination → Befehl (bei Doppelbelegung gewinnt der zuerst gelistete Befehl). */
export function resolveBindings(overrides: ShortcutOverrides): Map<string, string> {
  const map = new Map<string, string>();
  for (const command of SHORTCUT_COMMANDS) {
    for (const combo of bindingsFor(command.id, overrides)) {
      if (!map.has(combo)) map.set(combo, command.id);
    }
  }
  return map;
}

/** Befehl, der `combo` bereits verwendet (außer `exceptId`). */
export function conflictFor(combo: string, overrides: ShortcutOverrides, exceptId?: string): ShortcutCommand | null {
  const normalized = normalizeCombo(combo);
  return (
    SHORTCUT_COMMANDS.find(
      (command) => command.id !== exceptId && bindingsFor(command.id, overrides).includes(normalized),
    ) ?? null
  );
}

/**
 * Neue Belegung: setzt `combos` für `id` und entfernt dieselben Kombinationen bei
 * anderen Befehlen. Abweichungen, die wieder der Standardbelegung entsprechen,
 * werden aus den Überschreibungen entfernt.
 */
export function assignShortcut(overrides: ShortcutOverrides, id: string, combos: string[]): ShortcutOverrides {
  const normalized = [...new Set(combos.map(normalizeCombo))];
  const next: ShortcutOverrides = { ...overrides, [id]: normalized };
  for (const command of SHORTCUT_COMMANDS) {
    if (command.id === id) continue;
    const current = bindingsFor(command.id, next);
    const filtered = current.filter((combo) => !normalized.includes(combo));
    if (filtered.length !== current.length) next[command.id] = filtered;
  }
  for (const command of SHORTCUT_COMMANDS) {
    if (!Object.prototype.hasOwnProperty.call(next, command.id)) continue;
    const defaults = command.defaults.map(normalizeCombo);
    const value = next[command.id];
    if (value.length === defaults.length && value.every((combo, index) => combo === defaults[index])) {
      delete next[command.id];
    }
  }
  return next;
}

/**
 * Kombinationen, die der Editor (Tiptap/ProseMirror) selbst kennt. Sind sie gerade
 * keinem Befehl zugeordnet, werden sie abgefangen, damit umbelegte Standardkürzel
 * nicht weiterhin wirken.
 */
export const EDITOR_NATIVE_COMBOS = [
  "Ctrl+B", "Ctrl+I", "Ctrl+U", "Ctrl+Shift+S", "Ctrl+Shift+X", "Ctrl+,", "Ctrl+.", "Ctrl+E",
  "Ctrl+Shift+L", "Ctrl+Shift+E", "Ctrl+Shift+R", "Ctrl+Shift+J",
  "Ctrl+Alt+0", "Ctrl+Alt+1", "Ctrl+Alt+2", "Ctrl+Alt+3", "Ctrl+Alt+4", "Ctrl+Alt+5", "Ctrl+Alt+6",
  "Ctrl+Shift+7", "Ctrl+Shift+8", "Ctrl+Shift+9", "Ctrl+Shift+B", "Ctrl+Alt+C", "Ctrl+Shift+H",
  "Ctrl+Z", "Ctrl+Y", "Ctrl+Shift+Z",
];
