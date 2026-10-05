/**
 * Dokumenteinstellungen – gemeinsame Quelle für Vorschau (CSS), LaTeX-Export
 * und Projektdatei. Alle Werte werden über `normalizeSettings` validiert, damit
 * alte localStorage-/Projektstände und manipulierte Dateien keine ungültige
 * Präambel erzeugen können.
 */
import { DOCUMENT_LANGUAGE_IDS } from "./languages";

export type PaperFormat = "a4" | "a5" | "letter" | "legal" | "custom";
export type Orientation = "portrait" | "landscape";
export type DocumentClass = "scrreprt" | "article";
/** babel-Sprache (siehe document-languages.json) */
export type DocumentLanguage = string;
export type ParagraphStyle = "indent" | "skip";
export type BibliographyStyle = "ieee" | "plainnat" | "plainnat-authoryear" | "abbrvnat" | "alpha";

export type DocumentMargins = {
  top: number;
  right: number;
  bottom: number;
  left: number;
};

export type HeaderFooterSettings = {
  enabled: boolean;
  headerLeft: string;
  headerCenter: string;
  headerRight: string;
  footerLeft: string;
  footerCenter: string;
  footerRight: string;
};

export type DocumentMetadata = {
  title: string;
  author: string;
  subject: string;
  keywords: string;
};

export type BibliographySettings = {
  /** Pfad der .bib-Datei relativ zum Projektordner, z. B. `literatur.bib`. */
  file: string;
  style: BibliographyStyle;
};

export type DocumentSettings = {
  paperFormat: PaperFormat;
  orientation: Orientation;
  margins: DocumentMargins;
  columns: 1 | 2 | 3;
  customWidth: number;
  customHeight: number;
  documentClass: DocumentClass;
  twoside: boolean;
  bindingOffset: number;
  defaultFontFamily: string;
  defaultFontSize: number;
  lineSpacing: number;
  paragraphStyle: ParagraphStyle;
  language: DocumentLanguage;
  numberingDepth: number;
  tocDepth: number;
  pageDisplayColor: string;
  pagePdfColor: string;
  headerFooter: HeaderFooterSettings;
  metadata: DocumentMetadata;
  bibliography: BibliographySettings;
};

export const availableFontFamilies = [
  "Latin Modern Roman",
  "Arial",
  "Calibri",
  "Cambria",
  "Consolas",
  "Courier New",
  "Georgia",
  "Liberation Sans",
  "Liberation Serif",
  "Palatino Linotype",
  "Segoe UI",
  "Times New Roman",
  "Verdana",
] as const;

export const DEFAULT_FONT = "Latin Modern Roman";

export const bibliographyStyles: Array<{ id: BibliographyStyle; label: string }> = [
  { id: "ieee", label: "IEEE (numerisch)" },
  { id: "plainnat", label: "Numerisch (plainnat)" },
  { id: "abbrvnat", label: "Numerisch, abgekürzt (abbrvnat)" },
  { id: "plainnat-authoryear", label: "Autor-Jahr (plainnat)" },
  { id: "alpha", label: "Alphabetisch (alpha)" },
];

export const defaultHeaderFooter: HeaderFooterSettings = {
  enabled: false,
  headerLeft: "",
  headerCenter: "",
  headerRight: "{kapitel}",
  footerLeft: "",
  footerCenter: "{seite}",
  footerRight: "",
};

export const defaultDocumentSettings: DocumentSettings = {
  paperFormat: "a4",
  orientation: "portrait",
  margins: { top: 25, right: 25, bottom: 25, left: 25 },
  columns: 1,
  customWidth: 210,
  customHeight: 297,
  documentClass: "scrreprt",
  twoside: false,
  bindingOffset: 0,
  defaultFontFamily: DEFAULT_FONT,
  defaultFontSize: 12,
  lineSpacing: 1,
  paragraphStyle: "indent",
  language: "ngerman",
  numberingDepth: 3,
  tocDepth: 3,
  pageDisplayColor: "",
  pagePdfColor: "",
  headerFooter: defaultHeaderFooter,
  metadata: { title: "", author: "", subject: "", keywords: "" },
  bibliography: { file: "", style: "ieee" },
};

const paperSizes: Record<Exclude<PaperFormat, "custom">, { width: number; height: number }> = {
  a4: { width: 210, height: 297 },
  a5: { width: 148, height: 210 },
  letter: { width: 215.9, height: 279.4 },
  legal: { width: 215.9, height: 355.6 },
};

export function getPageDimensions(settings: DocumentSettings) {
  const paper = settings.paperFormat === "custom"
    ? { width: settings.customWidth, height: settings.customHeight }
    : paperSizes[settings.paperFormat];
  return settings.orientation === "landscape"
    ? { width: paper.height, height: paper.width }
    : { ...paper };
}

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  const number = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(number)) return fallback;
  return Math.min(max, Math.max(min, number));
}

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value) ? value as T : fallback;
}

function cleanColor(value: unknown): string {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value) ? value.toLowerCase() : "";
}

function cleanText(value: unknown, maxLength = 300): string {
  return typeof value === "string" ? value.replace(/[\r\n]+/g, " ").slice(0, maxLength) : "";
}

/** Relative Projektpfade: keine absoluten Pfade, kein `..`, keine LaTeX-Sonderzeichen. */
export function isSafeRelativePath(path: string): boolean {
  const normalized = path.replace(/\\/g, "/");
  return normalized.length > 0
    && !normalized.startsWith("/")
    && !/^[A-Za-z]:/.test(normalized)
    && !normalized.split("/").includes("..")
    && !/[{}\\%#$&~^\r\n]/.test(normalized);
}

/**
 * Validiert und ergänzt beliebige (z. B. aus alten Versionen stammende)
 * Einstellungsobjekte. Unbekannte Felder werden verworfen.
 */
export function normalizeSettings(input: unknown): DocumentSettings {
  const raw = (input && typeof input === "object" ? input : {}) as Record<string, unknown>;
  const defaults = defaultDocumentSettings;
  const margins = (raw.margins && typeof raw.margins === "object" ? raw.margins : {}) as Record<string, unknown>;
  const headerFooter = (raw.headerFooter && typeof raw.headerFooter === "object" ? raw.headerFooter : {}) as Record<string, unknown>;
  const metadata = (raw.metadata && typeof raw.metadata === "object" ? raw.metadata : {}) as Record<string, unknown>;
  const bibliography = (raw.bibliography && typeof raw.bibliography === "object" ? raw.bibliography : {}) as Record<string, unknown>;
  // Frühere Versionen speicherten nur den Dateinamen unter `bibliographyFile`.
  const legacyBibFile = typeof raw.bibliographyFile === "string" ? raw.bibliographyFile : "";
  const bibFile = typeof bibliography.file === "string" ? bibliography.file : legacyBibFile;
  const font = typeof raw.defaultFontFamily === "string" && raw.defaultFontFamily.trim()
    && /^[A-Za-z0-9 _-]+$/.test(raw.defaultFontFamily)
    ? raw.defaultFontFamily.trim()
    : defaults.defaultFontFamily;
  const columns = clampNumber(raw.columns, 1, 3, 1);

  return {
    paperFormat: pick(raw.paperFormat, ["a4", "a5", "letter", "legal", "custom"] as const, defaults.paperFormat),
    orientation: pick(raw.orientation, ["portrait", "landscape"] as const, defaults.orientation),
    margins: {
      top: clampNumber(margins.top, 0, 100, defaults.margins.top),
      right: clampNumber(margins.right, 0, 100, defaults.margins.right),
      bottom: clampNumber(margins.bottom, 0, 100, defaults.margins.bottom),
      left: clampNumber(margins.left, 0, 100, defaults.margins.left),
    },
    columns: Math.round(columns) as 1 | 2 | 3,
    customWidth: clampNumber(raw.customWidth, 50, 1000, defaults.customWidth),
    customHeight: clampNumber(raw.customHeight, 50, 1000, defaults.customHeight),
    documentClass: pick(raw.documentClass, ["scrreprt", "article"] as const, defaults.documentClass),
    twoside: raw.twoside === true,
    bindingOffset: clampNumber(raw.bindingOffset, 0, 50, defaults.bindingOffset),
    defaultFontFamily: font,
    defaultFontSize: Math.round(clampNumber(raw.defaultFontSize, 8, 20, defaults.defaultFontSize)),
    lineSpacing: clampNumber(raw.lineSpacing, 1, 3, defaults.lineSpacing),
    paragraphStyle: pick(raw.paragraphStyle, ["indent", "skip"] as const, defaults.paragraphStyle),
    language: typeof raw.language === "string" && DOCUMENT_LANGUAGE_IDS.includes(raw.language) ? raw.language : defaults.language,
    numberingDepth: Math.round(clampNumber(raw.numberingDepth, 0, 5, defaults.numberingDepth)),
    tocDepth: Math.round(clampNumber(raw.tocDepth, 0, 5, defaults.tocDepth)),
    pageDisplayColor: cleanColor(raw.pageDisplayColor),
    pagePdfColor: cleanColor(raw.pagePdfColor),
    headerFooter: {
      enabled: headerFooter.enabled === true || raw.headerFooterEnabled === true,
      headerLeft: cleanText(headerFooter.headerLeft ?? defaultHeaderFooter.headerLeft),
      headerCenter: cleanText(headerFooter.headerCenter ?? defaultHeaderFooter.headerCenter),
      headerRight: cleanText(headerFooter.headerRight ?? defaultHeaderFooter.headerRight),
      footerLeft: cleanText(headerFooter.footerLeft ?? defaultHeaderFooter.footerLeft),
      footerCenter: cleanText(headerFooter.footerCenter ?? defaultHeaderFooter.footerCenter),
      footerRight: cleanText(headerFooter.footerRight ?? defaultHeaderFooter.footerRight),
    },
    metadata: {
      title: cleanText(metadata.title),
      author: cleanText(metadata.author),
      subject: cleanText(metadata.subject),
      keywords: cleanText(metadata.keywords),
    },
    bibliography: {
      file: bibFile && isSafeRelativePath(bibFile) && /\.bib$/i.test(bibFile) ? bibFile.replace(/\\/g, "/") : "",
      style: pick(bibliography.style, ["ieee", "plainnat", "plainnat-authoryear", "abbrvnat", "alpha"] as const, defaults.bibliography.style),
    },
  };
}

/** Klassen mit `\chapter` (Bericht/Buch). */
export function classHasChapters(documentClass: DocumentClass): boolean {
  return documentClass === "scrreprt";
}
