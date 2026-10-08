/** Literaturstil passend zur Dokumentsprache (ohne Abhängigkeiten, damit auch in Node-Tests ladbar). */
import type { BibliographyStyle, DocumentSettings } from "./settings";

/** Wie `ieee_style_for_language` in Rust (`core/preamble.rs`). */
const GERMAN = new Set(["ngerman", "naustrian", "nswissgerman", "german", "austrian", "swissgerman"]);

/** IEEE-Stil passend zur Dokumentsprache (deutsche Dokumente: „IEEE (deutsch)“). */
export function ieeeStyleForLanguage(language: string): BibliographyStyle {
  return GERMAN.has(language) ? "ieee-de" : "ieee";
}

/** Beim Wechsel der Dokumentsprache zwischen IEEE und IEEE (deutsch) umschalten; andere Stile bleiben. */
export function withLanguageBibliography(previous: DocumentSettings, next: DocumentSettings): DocumentSettings {
  const style = next.bibliography.style;
  if (previous.language === next.language || (style !== "ieee" && style !== "ieee-de")) return next;
  const adjusted = ieeeStyleForLanguage(next.language);
  return adjusted === style ? next : { ...next, bibliography: { ...next.bibliography, style: adjusted } };
}
