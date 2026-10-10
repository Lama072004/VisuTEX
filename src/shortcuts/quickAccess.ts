/**
 * Symbolleiste für den Schnellzugriff (Titelleiste, wie in Office): Liste von Befehls-IDs aus
 * `SHORTCUT_COMMANDS`. Ohne JSX/DOM – in Node testbar (`tests/quick-access.test.mjs`).
 */
import { SHORTCUT_COMMANDS } from "./shortcuts.ts";
import type { ShortcutCommand } from "./shortcuts.ts";

/** Standardbelegung: Speichern, Rückgängig, Wiederholen. */
export const DEFAULT_QUICK_ACCESS = ["file.save", "edit.undo", "edit.redo"];

/** Vorschläge im ▾-Menü der Titelleiste (wie „Symbolleiste für den Schnellzugriff anpassen“). */
export const QUICK_ACCESS_SUGGESTIONS = [
  "file.new",
  "file.open",
  "file.save",
  "file.share",
  "file.exportPdf",
  "file.compile",
  "edit.undo",
  "edit.redo",
  "edit.find",
  "view.formattingMarks",
  "insert.inlineMath",
  "insert.table",
  "insert.image",
];

/** Höchstzahl der Schaltflächen (die Titelleiste bleibt bedienbar). */
export const QUICK_ACCESS_LIMIT = 16;

/** Befehle, die in der Titelleiste des Dokuments sinnvoll sind (nicht die des Folien-Editors). */
export function quickAccessCommands(): ShortcutCommand[] {
  return SHORTCUT_COMMANDS.filter((command) => command.category !== "Folien");
}

const known = () => new Set(quickAccessCommands().map((command) => command.id));

/** Gespeicherte Liste bereinigen: unbekannte und doppelte Einträge entfernen, Länge begrenzen. */
export function normalizeQuickAccess(value: unknown): string[] {
  if (!Array.isArray(value)) return [...DEFAULT_QUICK_ACCESS];
  const ids = known();
  const result: string[] = [];
  for (const entry of value) {
    if (typeof entry === "string" && ids.has(entry) && !result.includes(entry)) result.push(entry);
  }
  return result.slice(0, QUICK_ACCESS_LIMIT);
}

/** Befehl ein- bzw. ausblenden (neue Befehle hinten anfügen). */
export function toggleQuickAccess(list: string[], id: string): string[] {
  if (list.includes(id)) return list.filter((entry) => entry !== id);
  if (!known().has(id) || list.length >= QUICK_ACCESS_LIMIT) return list;
  return [...list, id];
}

/** Befehl um `delta` Plätze verschieben (−1 = nach links/oben). */
export function moveQuickAccess(list: string[], id: string, delta: number): string[] {
  const index = list.indexOf(id);
  const target = index + delta;
  if (index < 0 || target < 0 || target >= list.length) return list;
  const next = [...list];
  next.splice(index, 1);
  next.splice(target, 0, id);
  return next;
}
