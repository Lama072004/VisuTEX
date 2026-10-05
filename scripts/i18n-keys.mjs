#!/usr/bin/env node
// Hilfswerkzeug für Übersetzer:
//   node scripts/i18n-keys.mjs export <datei.txt>
//       alle Texte nummeriert, je Zeile: Nr|"Text" (JSON-Zeichenkette)
//   node scripts/i18n-keys.mjs import <sprache> <übersetzungen.txt> [<export.txt>]
//       Zeilen Nr|"Übersetzung" → src/i18n/locales/<sprache>.json (Nummern wie im Export;
//       ohne Exportdatei werden die Texte neu gesammelt)
// Beim Import bleiben vorhandene Übersetzungen erhalten, neue überschreiben sie.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { collectKeys } from "./i18n-check.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const [, , command, first, second, snapshot] = process.argv;

function parseLine(line) {
  const match = /^(\d+)\|(".*")\s*$/.exec(line);
  return match ? [Number(match[1]), JSON.parse(match[2])] : null;
}

if (command === "export") {
  const keys = [...collectKeys().keys()].sort((a, b) => a.localeCompare(b, "de"));
  writeFileSync(first, keys.map((key, index) => `${index + 1}|${JSON.stringify(key)}`).join("\n") + "\n");
  console.log(`${keys.length} Texte → ${first}`);
} else if (command === "import") {
  const keys = snapshot
    ? readFileSync(snapshot, "utf8").split(/\r?\n/).map(parseLine).filter(Boolean).map(([, text]) => text)
    : [...collectKeys().keys()].sort((a, b) => a.localeCompare(b, "de"));
  const file = join(root, "src", "i18n", "locales", `${first}.json`);
  const existing = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : {};
  let count = 0;
  for (const line of readFileSync(second, "utf8").split(/\r?\n/)) {
    const parsed = parseLine(line);
    if (!parsed) continue;
    const key = keys[parsed[0] - 1];
    if (key === undefined) throw new Error(`Unbekannte Nummer ${parsed[0]}`);
    existing[key] = parsed[1];
    count += 1;
  }
  const sorted = Object.fromEntries(Object.entries(existing).sort(([a], [b]) => a.localeCompare(b, "de")));
  writeFileSync(file, JSON.stringify(sorted, null, 2) + "\n");
  console.log(`${count} Übersetzungen → ${file}`);
} else {
  console.log("Aufruf: node scripts/i18n-keys.mjs export <datei> | import <sprache> <datei> [<export>]");
}
