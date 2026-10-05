/**
 * Oberflächensprachen. Deutsch ist die Quellsprache (die Texte im Code sind die
 * Schlüssel); Übersetzungen liegen als JSON in `src/locales/<code>.json`
 * (Englisch immer geladen, andere Sprachen bei Bedarf). Fehlende Übersetzungen
 * fallen auf Englisch, danach auf Deutsch zurück.
 * Prüfen/ergänzen: `npm run i18n` (scripts/i18n-check.mjs).
 * Unter macOS werden Tastenkürzel mit ⌘ statt Strg/Ctrl angezeigt.
 */
import { useCallback } from "react";
import { useRuntime } from "../state/runtime";
import en from "./locales/en.json";

export type UiLanguage = "de" | "en" | "fr" | "es" | "it" | "pt" | "nl" | "pl";

export type UiLanguageInfo = {
  code: UiLanguage;
  /** Name in der Sprache selbst */
  name: string;
  /** BCP 47 für Datums- und Zahlenformate */
  locale: string;
  /** Vorgabe für die Dokumentsprache neuer Dokumente (babel) */
  documentLanguage: string;
};

export const UI_LANGUAGES: UiLanguageInfo[] = [
  { code: "de", name: "Deutsch", locale: "de-DE", documentLanguage: "ngerman" },
  { code: "en", name: "English", locale: "en-GB", documentLanguage: "english" },
  { code: "fr", name: "Français", locale: "fr-FR", documentLanguage: "french" },
  { code: "es", name: "Español", locale: "es-ES", documentLanguage: "spanish" },
  { code: "it", name: "Italiano", locale: "it-IT", documentLanguage: "italian" },
  { code: "pt", name: "Português", locale: "pt-PT", documentLanguage: "portuguese" },
  { code: "nl", name: "Nederlands", locale: "nl-NL", documentLanguage: "dutch" },
  { code: "pl", name: "Polski", locale: "pl-PL", documentLanguage: "polish" },
];

export function languageInfo(code: string): UiLanguageInfo {
  return UI_LANGUAGES.find((entry) => entry.code === code) ?? UI_LANGUAGES[1];
}

/** Oberflächensprache aus der Systemsprache (erste unterstützte, sonst Englisch). */
export function systemLanguage(): UiLanguage {
  const candidates = typeof navigator === "undefined" ? [] : [...(navigator.languages ?? []), navigator.language];
  for (const candidate of candidates) {
    const code = String(candidate ?? "").slice(0, 2).toLowerCase();
    const match = UI_LANGUAGES.find((entry) => entry.code === code);
    if (match) return match.code;
  }
  return "en";
}

export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent || "");

const dictionaries: Partial<Record<UiLanguage, Record<string, string>>> = { en };
const loaders = import.meta.glob<{ default: Record<string, string> }>(["./locales/*.json", "!./locales/en.json"]);

/** Lädt die Übersetzungen einer Sprache (einmalig). */
export async function loadLanguage(code: UiLanguage): Promise<void> {
  if (code === "de" || dictionaries[code]) return;
  const loader = loaders[`./locales/${code}.json`];
  if (!loader) return;
  try {
    dictionaries[code] = (await loader()).default;
  } catch {
    // Sprachdatei fehlt/defekt → Englisch als Rückfall
  }
}

function platformKeys(text: string): string {
  if (!isMac) return text;
  return text.replace(/(Strg|Ctrl)\+/g, "⌘").replace(/Umschalt\+/g, "⇧").replace(/Shift\+/g, "⇧");
}

export function translate(language: UiLanguage, text: string): string {
  if (language === "de") return platformKeys(text);
  return platformKeys(dictionaries[language]?.[text] ?? en[text as keyof typeof en] ?? text);
}

export function useT() {
  const language = useRuntime((runtime) => runtime.language);
  return useCallback((text: string) => translate(language, text), [language]);
}

export function useLanguage(): UiLanguage {
  return useRuntime((runtime) => runtime.language);
}

/** Locale (BCP 47) der Oberfläche für Datums-/Zahlenformate. */
export function useLocale(): string {
  return languageInfo(useLanguage()).locale;
}
