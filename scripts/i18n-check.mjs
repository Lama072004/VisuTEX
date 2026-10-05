#!/usr/bin/env node
// Prüft die Übersetzungen der Oberfläche: sammelt alle Texte aus t("…") bzw.
// translate(…, "…") in src/ und meldet je Sprache fehlende und überzählige
// Einträge in src/i18n/locales/*.json.
//
//   node scripts/i18n-check.mjs            Bericht
//   node scripts/i18n-check.mjs --json     fehlende Texte als JSON (für Übersetzer)
//   node scripts/i18n-check.mjs --strict   Exit-Code 1 bei fehlenden Texten (CI)
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const sourceDir = join(root, "src");

function walk(dir, files = []) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name !== "locales") walk(path, files);
    } else if (/\.(tsx?|mjs)$/.test(name) && !name.endsWith(".d.ts")) files.push(path);
  }
  return files;
}

/** Zeichenkette ab `start` (Anführungszeichen) lesen – mit Escapes. */
function readString(text, start) {
  const quote = text[start];
  let value = "";
  for (let index = start + 1; index < text.length; index += 1) {
    const character = text[index];
    if (character === "\\") {
      const next = text[index + 1];
      value += next === "n" ? "\n" : next;
      index += 1;
    } else if (character === quote) {
      return value;
    } else if (quote === "`" && character === "$" && text[index + 1] === "{") {
      return null; // Vorlagen mit Platzhaltern werden nicht übersetzt
    } else {
      value += character;
    }
  }
  return null;
}

/** Dateien mit Tabellen übersetzbarer Texte (Befehle, Layouts, Formen …). */
const TABLE_FILES = [
  "shortcuts/shortcuts.ts",
  "slides/model.ts",
  "slides/SlideEditor.tsx",
  "sketch/SketchPad.tsx",
  "components/Ribbon.tsx",
  "editor/schema.ts",
];

export function collectKeys() {
  const keys = new Map();
  const add = (value, file) => {
    if (value && /[A-Za-zÄÖÜäöüß]/.test(value) && !value.includes("t(") && !keys.has(value)) {
      keys.set(value, relative(root, file).replace(/\\/g, "/"));
    }
  };
  for (const file of walk(sourceDir)) {
    const text = readFileSync(file, "utf8");
    // t("…"), translate(sprache, "…")
    for (const match of text.matchAll(/\b(?:t\(|translate\([^,]+,)\s*(["'`])/g)) {
      add(readString(text, match.index + match[0].length - 1), file);
    }
    // t(bedingung ? "A" : "B") – nur die Ergebnisse, nicht die Vergleichswerte
    for (const match of text.matchAll(/\bt\(([^()]*\?[^()]*)\)/g)) {
      for (const literal of match[1].matchAll(/[?:]\s*"((?:[^"\\]|\\.)*)"/g)) add(JSON.parse(`"${literal[1]}"`), file);
    }
    // Tabellen: label: "…", category: "…", Werte deutscher Bezeichnungen
    const relativePath = relative(sourceDir, file).replace(/\\/g, "/");
    if (TABLE_FILES.includes(relativePath)) {
      for (const match of text.matchAll(/\b(?:label|category|[a-z]+):\s*"([A-ZÄÖÜ][a-zäöüß][^"\\]*)"/g)) add(match[1], file);
    }
  }
  return keys;
}

export function readLocale(code) {
  return new Map(Object.entries(JSON.parse(readFileSync(join(sourceDir, "i18n", "locales", `${code}.json`), "utf8"))));
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  const keys = collectKeys();
  const locales = readdirSync(join(sourceDir, "i18n", "locales")).filter((name) => name.endsWith(".json")).map((name) => name.slice(0, -5));
  const report = {};
  let missingTotal = 0;
  for (const code of locales) {
    const entries = readLocale(code);
    const missing = [...keys.keys()].filter((key) => !entries.has(key));
    const unused = [...entries.keys()].filter((key) => !keys.has(key));
    missingTotal += missing.length;
    report[code] = { missing, unused };
  }
  if (process.argv.includes("--json")) {
    console.log(JSON.stringify(Object.fromEntries(Object.entries(report).map(([code, value]) => [code, value.missing])), null, 2));
  } else {
    console.log(`${keys.size} übersetzbare Texte in src/`);
    for (const [code, { missing, unused }] of Object.entries(report)) {
      console.log(`  ${code}: ${missing.length} fehlen, ${unused.length} ungenutzt`);
      for (const key of missing.slice(0, 15)) console.log(`      – ${JSON.stringify(key)} (${keys.get(key)})`);
      if (missing.length > 15) console.log(`      … und ${missing.length - 15} weitere`);
    }
  }
  if (process.argv.includes("--strict") && missingTotal > 0) process.exit(1);
}
