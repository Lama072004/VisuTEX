/**
 * Typisierte Aufrufe der Rust-Commands (`src-tauri/src/lib.rs`).
 * Die gesamte LaTeX-, Datei-, Kompilier-, Zotero- und Add-on-Logik liegt in Rust;
 * das Frontend ruft nur diese Funktionen auf.
 */
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { JSONContent } from "@tiptap/core";
import type { MacroDef } from "./latex/miniRender";
import type { Sketch } from "./sketch/SketchPad";
import type { DocumentSettings } from "./latex/settings";
import type { SlideDeck } from "./slides/model";

export type EmbeddedAsset = { path: string; mime: string; base64: string };

export type ExportResult = {
  latex: string;
  preamble: string;
  generatedPreamble: string;
  body: string;
  assets: EmbeddedAsset[];
  warnings: string[];
  missingPackages: string[];
};

export type SyncResult = {
  document: JSONContent;
  settings: DocumentSettings;
  customPreamble: string | null;
  warnings: string[];
  rawBlockCount: number;
  missingResources: string[];
};

export type TexMessage = { line: number | null; message: string; severity: "error" | "warning" };
export type PageSize = { width: number; height: number };
/** `missingFiles`: fehlende TeX-Dateien – das PDF wurde trotzdem erzeugt (Nachladen anbieten). */
export type CompileResult = { pdfId: number; pages: PageSize[]; messages: TexMessage[]; durationMs: number; missingFiles?: string[] };
/** `missingFiles`: TeX-Dateien, die weder im Bundle noch im Projekt gefunden wurden (z. B. `siunitx.sty`). */
export type CompileFailure = { message: string; messages: TexMessage[]; log: string; missingFiles?: string[] };

export type Project = {
  format: "visutex-project";
  version: number;
  settings: DocumentSettings;
  document: JSONContent;
  customPreamble: string | null;
  lastMode: "visual" | "code";
  code: string | null;
};

export type FileStamp = { modifiedMs: number; size: number };

export type OpenedProject = {
  project: Project;
  root: string;
  path: string | null;
  kind: "visutex" | "tex";
  warnings: string[];
  missingFiles: string[];
  stamp: FileStamp | null;
  temporary: boolean;
};

export type SavedFile = { root: string; path: string; stamp: FileStamp | null; missingFiles: string[] };

export type LabelInfo = { label: string; kind: "heading" | "figure" | "table" | "equation" | "drawing" | "raw"; title: string };
export type OutlineEntry = { level: number; title: string; blockIndex: number };
export type Analysis = {
  labels: LabelInfo[];
  outline: OutlineEntry[];
  citationKeys: string[];
  acronyms: Array<{ key: string; short: string; long: string }>;
  words: number;
  characters: number;
  duplicateLabels: string[];
};

export type BibEntry = { key: string; entryType: string; title: string; author: string; year: string; container: string; raw: string };
export type BibMerge = { added: string[]; skipped: string[]; keys: string[] };

export type ZoteroStatus = { running: boolean; betterBibtex: boolean; localApi: boolean; message: string };
export type ZoteroItem = { id: string; source: "bbt" | "local"; title: string; authors: string; year: string; itemType: string };

export type ImportedImage = { latexPath: string; convertedFrom: string | null; width: number | null; height: number | null };

export type Snippet = { id: string; name: string; group: string; kind: "inline" | "block" | "math" | "inlineMath" | "tikz"; latex: string };
export type RibbonEntry = { group: string; label: string; icon: string; snippet: string };
export type AddonManifest = {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  texFiles: string[];
  preamble: string[];
  snippets: Snippet[];
  ribbon: RibbonEntry[];
  templates: Array<{ id: string; name: string; file: string }>;
};
export type InstalledAddon = { manifest: AddonManifest; enabled: boolean; builtin: boolean; path: string };
export type TemplateInfo = { id: string; name: string; description: string; source: string };

export type AppInfo = {
  version: string;
  bundle: { kind: "embedded" | "embedded+online" | "online"; embeddedPath: string | null; fileCount: number | null };
  availableFonts: string[];
};

export type MissingResource = { command: string; path: string };

/** Skizzier-Werkzeug: Symbole aller CircuiTikZ-Bauteile und TikZ-Formen. */
export type SketchSymbol = {
  id: string;
  kind: "bipole" | "node" | "shape";
  category: string;
  de: string;
  en: string;
  image: string;
  ox: number;
  oy: number;
  w: number;
  h: number;
  anchors: Array<[string, number, number]>;
};
export type SketchCatalog = {
  pxPerCm: number;
  categories: Array<{ key: string; de: string; en: string }>;
  symbols: SketchSymbol[];
  arrowTips: string[];
  lineWidths: string[];
  dashPatterns: string[];
  fontSizes: string[];
};

/** Folien-Editor: Beamer-Export (Rust) und Bild als data:-URL. */
export type SlidesLatex = { latex: string; assets: EmbeddedAsset[]; warnings: string[] };
export type SlideImage = { src: string; width: number | null; height: number | null };

/** Einstellungen der TeX-Engine (aus den App-Optionen, in Rust gespeichert). */
export type TexOptions = { allowOnline: boolean; packageCacheDir: string | null; bundlePath: string | null };

export type SystemCheckItem = {
  id: "engine" | "bundle" | "packages" | "cache" | "data" | "fonts" | "webview" | "zotero";
  name: string;
  /** ok | warning | error | info */
  status: "ok" | "warning" | "error" | "info";
  detail: string;
  /** Vorgeschlagene Lösung: choose-bundle | choose-package-dir | enable-online */
  action: "choose-bundle" | "choose-package-dir" | "enable-online" | null;
};

export type SystemCheck = { items: SystemCheckItem[]; packageCacheDir: string; bundlePath: string | null; os: string };

/** Vorschau eines Roh-LaTeX-Blocks (PNG als data:-URL) oder Fehler. */
/** `fullPage`: ganze Seite samt Rändern (Titelseite), nicht beschnitten. */
export type BlockPreview = { image: string | null; error: string | null; fullPage?: boolean };
/** Wie LaTeX aufgelöstes Bild (Unterordner, \graphicspath, ohne Endung, PDF). */
export type ResolvedImage = {
  file: string | null;
  relative: string | null;
  dataUrl: string | null;
  kind: "image" | "pdf" | "eps" | "missing";
};

/** Läuft die Oberfläche innerhalb von Tauri (und nicht im reinen Browser)? */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Fehlertext aus einem abgelehnten invoke (String oder Objekt mit message). */
export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}

/** Aufruf eines Rust-Commands (im reinen Browser: Entwicklungs-Mock). */
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(command, args);
  return import("./devMock").then((mock) => mock.mockInvoke(command, args) as Promise<T>);
}

/** Einrichtung (Installation der heruntergeladenen EXE). */
export type SetupState = { mode: "installed" | "portable"; installDir: string | null; desktopShortcut: boolean; startMenu: boolean; autostart: boolean };
export type SetupStatus = {
  version: string;
  platform: "windows" | "linux" | "macos";
  currentExe: string;
  defaultDir: string;
  state: SetupState | null;
  runningInstalled: boolean;
  packaged: boolean;
  showSetup: boolean;
  uninstallRequested: boolean;
  /** Testmodus (nur Debug-Builds) */
  sandbox: boolean;
};
export type InstallOptions = { dir: string; desktopShortcut: boolean; startMenu: boolean; autostart: boolean };
export type UpdateAsset = { name: string; url: string; size: number; digest: string };
export type UpdateInfo = { current: string; latest: string; newer: boolean; prerelease: boolean; page: string; notes: string; asset: UpdateAsset | null };

export const api = {
  appInfo: (allowOnline: boolean) => call<AppInfo>("app_info", { allowOnline }),
  setTexOptions: (options: TexOptions) => call<void>("set_tex_options", { options }),
  systemCheck: () => call<SystemCheck>("system_check"),

  exportDocument: (document: JSONContent, settings: DocumentSettings, customPreamble: string | null) =>
    call<ExportResult>("export_document", { document, settings, customPreamble }),
  codeToDocument: (args: {
    code: string;
    settings: DocumentSettings;
    generatedPreamble: string | null;
    customPreamble: string | null;
    projectRoot: string | null;
  }) => call<SyncResult>("code_to_document", args),
  analyzeDocument: (document: JSONContent) => call<Analysis>("analyze_document", { document }),
  checkResources: (latex: string, projectRoot: string | null) =>
    call<MissingResource[]>("check_resources", { latex, projectRoot }),

  compileDocument: (latex: string, projectRoot: string | null, assets: EmbeddedAsset[], allowOnline: boolean) =>
    call<CompileResult>("compile_document", { latex, projectRoot, assets, allowOnline }),
  renderPdfPage: async (pdfId: number, page: number, scale: number): Promise<Blob> => {
    const bytes = await call<ArrayBuffer>("render_pdf_page", { pdfId, page, scale });
    return new Blob([bytes], { type: "image/png" });
  },
  savePdf: (pdfId: number, path: string) => call<void>("save_pdf", { pdfId, path }),
  renderTikz: (code: string, environment: string, options: string, allowOnline: boolean) =>
    call<string>("render_tikz", { code, environment, options, allowOnline }),
  renderLatexPreviews: (preamble: string, blocks: string[], projectRoot: string | null, allowOnline: boolean) =>
    call<BlockPreview[]>("render_latex_previews", { preamble, blocks, projectRoot, allowOnline }),
  resolveImage: (projectRoot: string, path: string, preamble: string | null) =>
    call<ResolvedImage>("resolve_image", { projectRoot, path, preamble }),
  preambleMacros: (preamble: string) => call<MacroDef[]>("preamble_macros", { preamble }),
  sketchToLatex: (sketch: Sketch) => call<{ code: string; environment: string }>("sketch_to_latex", { sketch }),
  sketchFromCode: (code: string) => call<Sketch | null>("sketch_from_code", { code }),
  sketchCatalog: () => call<SketchCatalog>("sketch_catalog"),
  setupStatus: () => call<SetupStatus>("setup_status"),
  setupInstall: (options: InstallOptions) => call<string>("setup_install", { options }),
  setupUsePortable: () => call<void>("setup_use_portable"),
  setupSetAutostart: (enabled: boolean) => call<void>("setup_set_autostart", { enabled }),
  setupUninstall: () => call<void>("setup_uninstall"),
  updateCheck: () => call<UpdateInfo>("update_check"),
  /** Gespeicherte Datei teilen (Windows: Teilen-Fenster, Linux: E-Mail mit Anhang); liefert den Dateinamen */
  shareFile: (path: string) => call<string>("share_file", { path }),
  updateInstall: (asset: UpdateAsset) => call<void>("update_install", { asset }),
  onUpdateProgress: (handler: (progress: { loaded: number; total: number }) => void) =>
    isTauri ? listen<{ loaded: number; total: number }>("update-progress", (event) => handler(event.payload)) : Promise.resolve(() => undefined),
  onCompileProgress: (handler: (text: string) => void) =>
    isTauri ? listen<string>("compile-progress", (event) => handler(event.payload)) : Promise.resolve(() => undefined),

  newProject: (template: string | null, addon: string | null, language: string | null = null) =>
    call<OpenedProject>("new_project", { template, addon, language }),
  loadProject: (path: string) => call<OpenedProject>("load_project", { path }),
  saveProject: (path: string, project: Project, sourceRoot: string | null) =>
    call<SavedFile>("save_project", { path, project, sourceRoot }),
  exportTex: (path: string, latex: string, assets: EmbeddedAsset[], sourceRoot: string | null) =>
    call<SavedFile>("export_tex", { path, latex, assets, sourceRoot }),
  fileStamp: (path: string) => call<FileStamp | null>("file_stamp", { path }),
  readTextFile: (path: string) => call<string>("read_text_file", { path }),
  allowProjectDir: (root: string) => call<void>("allow_project_dir", { root }),

  importImage: (source: string, projectRoot: string) => call<ImportedImage>("import_image", { source, projectRoot }),
  importImageData: (dataUrl: string, projectRoot: string) =>
    call<ImportedImage>("import_image_data", { dataUrl, projectRoot }),

  readBib: (projectRoot: string, file: string) => call<BibEntry[]>("read_bib", { projectRoot, file }),
  addBibEntries: (projectRoot: string, file: string, bibtex: string) =>
    call<BibMerge>("add_bib_entries", { projectRoot, file, bibtex }),
  zoteroStatus: () => call<ZoteroStatus>("zotero_status"),
  zoteroSearch: (query: string) => call<ZoteroItem[]>("zotero_search", { query }),
  zoteroImport: (ids: string[], source: string, projectRoot: string, file: string) =>
    call<BibMerge>("zotero_import", { ids, source, projectRoot, file }),

  addonsList: () => call<InstalledAddon[]>("addons_list"),
  addonsInstall: (path: string) => call<AddonManifest>("addons_install", { path }),
  addonsRemove: (id: string) => call<void>("addons_remove", { id }),
  addonsSetEnabled: (id: string, enabled: boolean) => call<void>("addons_set_enabled", { id, enabled }),
  addonsSave: (manifest: AddonManifest, newFiles: Array<{ source: string; target: string }>) =>
    call<AddonManifest>("addons_save", { manifest, newFiles }),
  addonsExport: (id: string, path: string) => call<void>("addons_export", { id, path }),
  templatesList: () => call<TemplateInfo[]>("templates_list"),

  slidesToLatex: (deck: SlideDeck) => call<SlidesLatex>("slides_to_latex", { deck }),
  slidesLoad: (path: string) => call<SlideDeck>("slides_load", { path }),
  slidesSave: (path: string, deck: SlideDeck) => call<FileStamp | null>("slides_save", { path, deck }),
  slidesReadImage: (path: string) => call<SlideImage>("slides_read_image", { path }),
  slidesCompile: (deck: SlideDeck, allowOnline: boolean) => call<CompileResult>("slides_compile", { deck, allowOnline }),
};

/** URL einer absoluten Datei für <img> (Tauri-Asset-Protokoll). */
export function fileUrl(path: string): string {
  return path ? convertFileSrc(path) : "";
}

/** URL einer Projektdatei für <img> (Tauri-Asset-Protokoll). */
export function projectFileUrl(root: string | null, relative: string): string {
  if (!root || !relative) return "";
  const separator = root.includes("\\") ? "\\" : "/";
  const path = `${root.replace(/[\\/]+$/, "")}${separator}${relative.replace(/[\\/]/g, separator)}`;
  return convertFileSrc(path);
}

