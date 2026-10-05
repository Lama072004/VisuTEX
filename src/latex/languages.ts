/**
 * Dokumentsprachen (babel) – dieselbe Tabelle wie im Rust-Kern
 * (`document-languages.json`, `core::languages`): Anführungszeichen,
 * Dezimalzeichen, Bereichswort, Beschriftungen, BCP 47.
 */
import table from "./document-languages.json";

export type DocumentLanguageLabels = {
  author: string;
  supervisor: string;
  institution: string;
  title: string;
  summary: string;
  acronyms: string;
  contents: string;
  figures: string;
  tables: string;
  bibliography: string;
  list: string;
};

export type DocumentLanguageInfo = {
  id: string;
  babel: string;
  name: string;
  bcp47: string;
  aliases: string[];
  quotes: [string, string];
  decimal: string;
  siunitx: string;
  range: string;
  germanShorthands: boolean;
  labels: DocumentLanguageLabels;
};

export const DOCUMENT_LANGUAGES = table as unknown as DocumentLanguageInfo[];

export const DOCUMENT_LANGUAGE_IDS = DOCUMENT_LANGUAGES.map((language) => language.id);

/** Sprache zu einer ID (unbekannt → Deutsch). */
export function documentLanguage(id: string | undefined | null): DocumentLanguageInfo {
  return DOCUMENT_LANGUAGES.find((language) => language.id === id) ?? DOCUMENT_LANGUAGES[0];
}
