/**
 * VisuTeX – Hauptfenster.
 *
 * Die Logik (LaTeX-Export/-Import, Kompilieren, Dateien, Zotero, Add-ons) liegt
 * in Rust (`src-tauri`); hier werden nur Editor, Ansichten und Dialoge
 * koordiniert. Ablauf Visuell ↔ Code: siehe docs/ARCHITEKTUR.md (Datenfluss Dokument).
 */
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import type { Transaction } from "@tiptap/pm/state";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Braces, Eye, PanelRight, Play, Presentation } from "lucide-react";
import "katex/dist/katex.min.css";
import "./App.css";
import { api, errorText, isTauri } from "./api";
import type { AppInfo, CompileFailure, SetupStatus, UpdateInfo, EmbeddedAsset, InstalledAddon, OpenedProject, Project, Snippet, SystemCheck, TemplateInfo, TexMessage, Analysis } from "./api";
import { appExtensions } from "./editor/extensions";
import { FormatPainter } from "./editor/formatPainter";
import type { PageInfo } from "./editor/pagination";
import { invalidatePreviews, setPreviewPreambleProvider } from "./editor/latexPreview";
import { RUNTIME_META } from "./editor/nodeViews";
import { frontmatterKinds, frontmatterTitles, subfigureDefaultPercent } from "./editor/schema";
import { newComment } from "./editor/comments";
import { useCtrlWheel } from "./components/useCtrlWheel";
import type { FrontmatterKind } from "./editor/schema";
import { languageInfo, loadLanguage, systemLanguage, translate } from "./i18n";
import type { UiLanguage } from "./i18n";
import { documentLanguage } from "./latex/languages";
import { EDITOR_NATIVE_COMBOS, SHORTCUT_COMMANDS, comboFromEvent, resolveBindings } from "./shortcuts/shortcuts";
import { COMMON_UNITS, formatQuantity, isSiunitxNumber } from "./latex/siunitx";
import { defaultDocumentSettings, getPageDimensions, normalizeSettings, withLanguageBibliography } from "./latex/settings";
import type { DocumentSettings } from "./latex/settings";
import { getRuntime, setRuntime } from "./state/runtime";
import type { EditRequest } from "./state/runtime";
import { Backstage } from "./components/Backstage";
import type { AppPrefs, BackstageView, RecentFile } from "./components/Backstage";
import { CitationDialog } from "./components/CitationDialog";
import type { CitationValue } from "./components/CitationDialog";
import { DialogProvider, useDialogs } from "./components/Dialogs";
import { DocumentSettingsDialog } from "./components/DocumentSettingsDialog";
import { PageCanvas } from "./components/PageCanvas";
import { PdfPreview } from "./components/PdfPreview";
import type { CompileStatus } from "./components/PdfPreview";
import { Ribbon } from "./components/Ribbon";
import type { RibbonActions, ViewState } from "./components/Ribbon";
import { Ruler } from "./components/Ruler";
import { SearchPanel } from "./components/SearchPanel";
import { SketchPad } from "./sketch/SketchPad";
import type { Sketch } from "./sketch/SketchPad";
import { SystemCheckDialog } from "./components/SystemCheckDialog";
import { SetupScreen, UpdateDialog } from "./components/SetupScreen";
import { QuickAccessBar } from "./components/QuickAccess";
import { setPendingMath } from "./math/mathBridge";
import { ShareDialog, TitleContextMenu } from "./components/ShareDialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { DEFAULT_QUICK_ACCESS, normalizeQuickAccess } from "./shortcuts/quickAccess";
import { completionData } from "./monaco/completionData";
import { newSlidesSession, normalizeDeck } from "./slides/model";
import type { SlideFileActions, SlidesSession } from "./slides/model";

const CodeView = lazy(() => import("./components/CodeView"));
const SlideEditor = lazy(() => import("./slides/SlideEditor"));

const MM = 96 / 25.4;
const PAGE_GAP = 28;
const labelPattern = /^[A-Za-z0-9:._-]+$/;

// ---------------------------------------------------------------- Einstellungen der App (lokal)

const defaultPrefs: AppPrefs = {
  language: systemLanguage(),
  theme: "system",
  accent: "#2563eb",
  autosave: true,
  allowOnline: false,
  autoCompile: false,
  packageCacheDir: "",
  bundlePath: "",
  shortcuts: {},
  checkUpdates: true,
  skipUpdate: "",
  quickAccess: [...DEFAULT_QUICK_ACCESS],
};

function loadPrefs(): AppPrefs {
  try {
    const stored = JSON.parse(localStorage.getItem("visutex-prefs") ?? "{}") as Partial<AppPrefs>;
    const legacyLanguage = localStorage.getItem("visutex-language");
    return {
      ...defaultPrefs,
      ...(legacyLanguage === "en" ? { language: "en" } : {}),
      ...(localStorage.getItem("visutex-accent-color") ? { accent: localStorage.getItem("visutex-accent-color") as string } : {}),
      ...stored,
      quickAccess: normalizeQuickAccess(stored.quickAccess ?? DEFAULT_QUICK_ACCESS),
    };
  } catch {
    return defaultPrefs;
  }
}

function loadRecent(): RecentFile[] {
  try {
    const list = JSON.parse(localStorage.getItem("visutex-recent") ?? "[]") as RecentFile[];
    return Array.isArray(list) ? list.slice(0, 12) : [];
  } catch {
    return [];
  }
}

type Recovery = { name: string; savedAt: number; root: string; path: string | null; kind: "visutex" | "tex"; temporary: boolean; project: Project };

function loadRecovery(): Recovery | null {
  try {
    const value = JSON.parse(localStorage.getItem("visutex-recovery") ?? "null") as Recovery | null;
    return value?.project ? value : null;
  } catch {
    return null;
  }
}

/** TikZ-Vorschaubilder nicht in die Wiederherstellungsdaten (Größe). */
function stripPreviews(node: JSONContent): JSONContent {
  const copy: JSONContent = { ...node };
  if (copy.type === "tikzBlock" && copy.attrs) copy.attrs = { ...copy.attrs, preview: "", previewCode: "" };
  if (copy.content) copy.content = copy.content.map(stripPreviews);
  return copy;
}

function fileName(path: string | null) {
  return path ? path.split(/[\\/]/).pop() ?? path : "";
}

type DocState = { root: string; path: string | null; kind: "visutex" | "tex"; temporary: boolean; stamp: OpenedProject["stamp"]; name: string };

type Notice = { id: number; message: string; kind: "info" | "error" };

const TIKZ_TEMPLATES = {
  tikzpicture: "\\draw[->] (0,0) -- (4,0) node[right] {$x$};\n\\draw[->] (0,0) -- (0,3) node[above] {$y$};\n\\draw[thick, blue, domain=0:3.5, smooth] plot (\\x, {0.2*\\x*\\x});",
  circuitikz: "\\draw (0,0) to[V, v=$U$] (0,3) to[R, l=$R_1$] (3,3) to[C, l=$C$] (3,0) -- (0,0);",
};

const LATEX_TABLE_TEMPLATE =
  "\\begin{table}[htbp]\n  \\centering\n  \\begin{tabular}{lrr}\n    \\hline\n    Größe & Wert & Einheit \\\\\n    \\hline\n    Spannung & 230 & V \\\\\n    Strom & 10 & A \\\\\n    \\hline\n  \\end{tabular}\n  \\caption{Beschriftung}\n  \\label{tab:beispiel}\n\\end{table}";

// ---------------------------------------------------------------- Hauptkomponente

export default function App() {
  return (
    <DialogProvider>
      <Workspace />
    </DialogProvider>
  );
}

function Workspace() {
  const dialogs = useDialogs();
  const [prefs, setPrefs] = useState<AppPrefs>(loadPrefs);
  // Oberflächensprache erst umschalten, wenn ihre Übersetzungen geladen sind.
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>(() => (prefs.language === "de" || prefs.language === "en" ? prefs.language : "en"));
  useEffect(() => {
    let cancelled = false;
    void loadLanguage(prefs.language).then(() => {
      if (!cancelled) setUiLanguage(prefs.language);
    });
    document.documentElement.lang = languageInfo(prefs.language).locale;
    return () => {
      cancelled = true;
    };
  }, [prefs.language]);
  const t = useCallback((text: string) => translate(uiLanguage, text), [uiLanguage]);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  const theme = prefs.theme === "system" ? (systemDark ? "dark" : "light") : prefs.theme;

  const [docState, setDocState] = useState<DocState | null>(null);
  const [settings, setSettingsState] = useState<DocumentSettings>(defaultDocumentSettings);
  const [customPreamble, setCustomPreamble] = useState<string | null>(null);
  const [mode, setModeState] = useState<"visual" | "code">("visual");
  const [code, setCode] = useState("");
  const [dirty, setDirty] = useState(false);
  const [view, setView] = useState<ViewState>(() => ({
    zoom: Number(localStorage.getItem("visutex-zoom")) || 100,
    paged: localStorage.getItem("visutex-paged") !== "false",
    showPdf: localStorage.getItem("visutex-show-pdf") === "true",
    showOutline: localStorage.getItem("visutex-show-outline") === "true",
    showRuler: localStorage.getItem("visutex-show-ruler") === "true",
    showMarks: localStorage.getItem("visutex-show-marks") === "true",
    compiling: false,
  }));
  const [backstage, setBackstage] = useState<BackstageView | null>("home");
  // Folien-Editor: Sitzung bleibt beim Schließen erhalten; ungespeicherte Änderungen
  // werden lokal gesichert (Wiederherstellung nach Absturz/Neustart).
  const [slidesOpen, setSlidesOpen] = useState(false);
  const [slidesSession, setSlidesSession] = useState<SlidesSession | null>(() => {
    try {
      const stored = JSON.parse(localStorage.getItem("visutex-slides-recovery") ?? "null") as SlidesSession | null;
      return stored ? { ...stored, deck: normalizeDeck(stored.deck), dirty: true } : null;
    } catch {
      return null;
    }
  });
  useEffect(() => {
    const timer = window.setTimeout(() => {
      try {
        if (slidesSession?.dirty) localStorage.setItem("visutex-slides-recovery", JSON.stringify(slidesSession));
        else localStorage.removeItem("visutex-slides-recovery");
      } catch {
        // zu groß (eingebettete Bilder) – Wiederherstellung entfällt
      }
    }, 1500);
    return () => window.clearTimeout(timer);
  }, [slidesSession]);
  const [recent, setRecent] = useState<RecentFile[]>(loadRecent);
  const [recovery, setRecovery] = useState<Recovery | null>(loadRecovery);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  // Einrichtung (erster Start der heruntergeladenen EXE), Deinstallation und Updates
  const [setupStatus, setSetupStatus] = useState<SetupStatus | null>(null);
  const [setupView, setSetupView] = useState<"setup" | "uninstall" | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  // Teilen: Dialog und Kontextmenü der Titelleiste
  const [shareOpen, setShareOpen] = useState(false);
  const [titleMenu, setTitleMenu] = useState<{ x: number; y: number } | null>(null);
  const [addons, setAddons] = useState<InstalledAddon[]>([]);
  const [compileStatus, setCompileStatus] = useState<CompileStatus>({ state: "idle" });
  const [codeMessages, setCodeMessages] = useState<TexMessage[]>([]);
  const [revealLine, setRevealLine] = useState<{ line: number; token: number } | null>(null);
  const [search, setSearch] = useState<{ replace: boolean } | null>(null);
  const [settingsDialog, setSettingsDialog] = useState<{ section?: string } | null>(null);
  const [citationDialog, setCitationDialog] = useState<{ initial: CitationValue | null; pos: number | null } | null>(null);
  const [notices, setNotices] = useState<Notice[]>([]);
  const [pages, setPages] = useState<PageInfo[]>([{ chapter: "" }]);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [formatPainterActive, setFormatPainterActive] = useState(false);
  const [paperHeight, setPaperHeight] = useState(0);
  const [systemCheck, setSystemCheck] = useState<SystemCheck | null>(null);
  const [systemCheckOpen, setSystemCheckOpen] = useState(false);
  const [systemCheckBusy, setSystemCheckBusy] = useState(false);
  /** Skizzier-Werkzeug: offen, Ausgangsmodell, ggf. Position des bearbeiteten TikZ-Blocks */
  const [sketch, setSketch] = useState<{ initial: Sketch | null; pos: number | null } | null>(null);
  const systemCheckOpenRef = useRef(false);
  systemCheckOpenRef.current = systemCheckOpen;
  /** Wird nach programmatischem Laden erhöht (setContent ohne update-Ereignis). */
  const [contentVersion, setContentVersion] = useState(0);

  // Abgleich Visuell ↔ Code
  /** Exakter LaTeX-Quelltext, der dem aktuellen visuellen Dokument entspricht (z. B. geöffnete .tex-Datei). */
  const docCode = useRef<string | null>(null);
  /** Stand beim Wechsel in die Code-Ansicht. */
  const codeBase = useRef<{ code: string; generatedPreamble: string; assets: EmbeddedAsset[] } | null>(null);
  const lastCompiled = useRef<{ pdfId: number } | null>(null);
  const noticeId = useRef(0);
  const stateRef = useRef({ settings, customPreamble, mode, code, docState, dirty, prefs });
  stateRef.current = { settings, customPreamble, mode, code, docState, dirty, prefs };

  const notify = useCallback((message: string, kind: "info" | "error" = "info") => {
    const id = ++noticeId.current;
    setNotices((current) => [...current.slice(-3), { id, message, kind }]);
    window.setTimeout(() => setNotices((current) => current.filter((notice) => notice.id !== id)), kind === "error" ? 12000 : 6000);
  }, []);

  const markDirty = useCallback(() => setDirty(true), []);
  const insertImageRef = useRef<(files: File[]) => Promise<void>>(async () => undefined);

  // ---------------------------------------------------------------- Editor

  const editor = useEditor({
    extensions: appExtensions(),
    content: { type: "doc", content: [{ type: "paragraph" }] },
    editorProps: {
      attributes: { class: "document-body", spellcheck: "true", lang: "de-DE", "aria-label": "Dokument" },
      handlePaste: (_view, event) => {
        const files = Array.from(event.clipboardData?.files ?? []).filter((file) => file.type.startsWith("image/"));
        if (files.length === 0) return false;
        void insertImageRef.current(files);
        return true;
      },
      handleDrop: (_view, event) => {
        const files = Array.from(event.dataTransfer?.files ?? []).filter((file) => file.type.startsWith("image/"));
        if (files.length === 0) return false;
        event.preventDefault();
        void insertImageRef.current(files);
        return true;
      },
    },
    onUpdate: ({ transaction }: { transaction: Transaction }) => {
      if (transaction.getMeta(RUNTIME_META)) return;
      docCode.current = null;
      markDirty();
    },
  });

  const painter = useMemo(() => (editor ? new FormatPainter(editor) : null), [editor]);
  useEffect(() => painter?.onChange(setFormatPainterActive), [painter]);

  // Laufzeitkontext für NodeViews
  useEffect(() => {
    setRuntime({ projectRoot: docState?.root ?? null, language: uiLanguage, allowOnline: prefs.allowOnline, settings, notify, customPreamble });
  }, [docState?.root, uiLanguage, prefs.allowOnline, settings, notify, customPreamble]);

  // Eigene Makros der Präambel (Anzeige von \REcv, \teil{a}, Formeln mit eigenen Befehlen)
  useEffect(() => {
    let cancelled = false;
    if (!customPreamble) {
      setRuntime({ macros: new Map() });
      return;
    }
    api
      .preambleMacros(customPreamble)
      .then((list) => {
        if (!cancelled) setRuntime({ macros: new Map(list.map((definition) => [definition.name, definition])) });
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [customPreamble]);

  // Vorschau für Roh-LaTeX: mit der Präambel, die auch beim Kompilieren gilt
  useEffect(() => {
    invalidatePreviews();
  }, [customPreamble, docState?.root]);
  useEffect(() => {
    setPreviewPreambleProvider(async () => {
      const current = stateRef.current;
      if (current.customPreamble) return current.customPreamble;
      const exported = await api.exportDocument(editor?.getJSON() ?? { type: "doc" }, current.settings, null);
      return exported.preamble;
    });
  }, [editor]);

  // Sprache für die Rechtschreibprüfung des WebView
  useEffect(() => {
    if (!editor) return;
    const lang = documentLanguage(settings.language).bcp47;
    editor.setOptions({ editorProps: { ...editor.options.editorProps, attributes: { class: "document-body", spellcheck: "true", lang, "aria-label": t("Dokument") } } });
  }, [editor, settings.language, t]);

  // Seitengeometrie für die Paginierung
  const dimensions = getPageDimensions(settings);
  const geometry = useMemo(
    () => ({
      enabled: view.paged && mode === "visual" && settings.columns === 1,
      pageHeight: dimensions.height * MM,
      marginTop: settings.margins.top * MM,
      marginBottom: settings.margins.bottom * MM,
      gap: PAGE_GAP,
      chapterNewPage: settings.documentClass === "scrreprt",
    }),
    [view.paged, mode, settings.columns, dimensions.height, settings.margins.top, settings.margins.bottom, settings.documentClass],
  );
  useEffect(() => {
    if (!editor) return;
    editor.commands.setPageGeometry(geometry);
  }, [editor, geometry]);
  useEffect(() => {
    if (!editor) return;
    const listener = (next: PageInfo[]) => setPages(next);
    editor.storage.pagination.listeners.add(listener);
    return () => {
      editor.storage.pagination.listeners.delete(listener);
    };
  }, [editor]);
  useEffect(() => {
    editor?.storage.pagination.remeasure();
  }, [editor, settings.defaultFontFamily, settings.defaultFontSize, settings.lineSpacing, settings.paragraphStyle, settings.margins.left, settings.margins.right, dimensions.width]);

  // Analyse (Labels, Gliederung, Wortzahl) in Rust, entprellt
  useEffect(() => {
    if (!editor) return;
    let timer: number | undefined;
    const run = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        void api
          .analyzeDocument(editor.getJSON())
          .then((result) => {
            setAnalysis(result);
            completionData.labels = result.labels.map((label) => label.label);
            completionData.citations = result.citationKeys;
          })
          .catch(() => undefined);
      }, 500);
    };
    run();
    editor.on("update", run);
    return () => {
      window.clearTimeout(timer);
      editor.off("update", run);
    };
  }, [editor, docState?.root, contentVersion]);

  // ---------------------------------------------------------------- Darstellung

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    const rootStyle = document.documentElement.style;
    rootStyle.setProperty("--accent", prefs.accent);
    const hex = /^#[0-9a-f]{6}$/i.test(prefs.accent) ? prefs.accent : "#2563eb";
    const [red, green, blue] = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16));
    rootStyle.setProperty("--accent-soft", `rgba(${red}, ${green}, ${blue}, 0.14)`);
    rootStyle.setProperty("--accent-border", `rgba(${red}, ${green}, ${blue}, 0.35)`);
    rootStyle.setProperty("--accent-strong", `rgb(${Math.round(red * 0.85)}, ${Math.round(green * 0.85)}, ${Math.round(blue * 0.85)})`);
    localStorage.setItem("visutex-prefs", JSON.stringify(prefs));
  }, [theme, prefs]);
  useEffect(() => {
    localStorage.setItem("visutex-zoom", String(view.zoom));
    localStorage.setItem("visutex-paged", String(view.paged));
    localStorage.setItem("visutex-show-pdf", String(view.showPdf));
    localStorage.setItem("visutex-show-outline", String(view.showOutline));
    localStorage.setItem("visutex-show-ruler", String(view.showRuler));
    localStorage.setItem("visutex-show-marks", String(view.showMarks));
  }, [view.zoom, view.paged, view.showPdf, view.showOutline, view.showRuler, view.showMarks]);
  // Formatierungszeichen (¶) im Editor ein-/ausblenden
  useEffect(() => {
    editor?.commands.setFormattingMarks(view.showMarks);
  }, [editor, view.showMarks]);

  const reloadAddons = useCallback(() => {
    void api.addonsList().then(setAddons).catch(() => setAddons([]));
  }, []);
  // TeX-Optionen (Online-Nachladen, Speicherort, eigenes Bundle) an Rust übergeben.
  const texOptions = useCallback(
    (next: AppPrefs) => ({ allowOnline: next.allowOnline, packageCacheDir: next.packageCacheDir || null, bundlePath: next.bundlePath || null }),
    [],
  );
  const runSystemCheck = useCallback(async (openIfProblems: boolean) => {
    setSystemCheckBusy(true);
    try {
      const result = await api.systemCheck();
      setSystemCheck(result);
      if (openIfProblems && result.items.some((item) => item.status === "error")) setSystemCheckOpen(true);
    } catch (error) {
      notify(errorText(error), "error");
    } finally {
      setSystemCheckBusy(false);
    }
  }, [notify]);
  const startupCheckDone = useRef(false);
  useEffect(() => {
    void api
      .setTexOptions(texOptions(prefs))
      .then(() => api.appInfo(prefs.allowOnline))
      .then(setAppInfo)
      .catch(() => setAppInfo(null))
      .then(() => {
        if (!startupCheckDone.current) {
          startupCheckDone.current = true;
          void runSystemCheck(true);
        } else if (systemCheckOpenRef.current) {
          void runSystemCheck(false);
        }
      });
    reloadAddons();
  }, [prefs, texOptions, reloadAddons, runSystemCheck]);

  const refreshSetup = useCallback(async () => {
    const status = await api.setupStatus();
    setSetupStatus(status);
    return status;
  }, []);
  // Beim Start: Einrichtung bzw. Deinstallation anzeigen, sonst (falls gewünscht) nach Updates suchen
  const startupUpdateDone = useRef(false);
  useEffect(() => {
    if (!isTauri) return;
    void refreshSetup()
      .then((status) => {
        if (status.uninstallRequested) setSetupView("uninstall");
        else if (status.showSetup) setSetupView("setup");
        else if (!import.meta.env.DEV && !startupUpdateDone.current && stateRef.current.prefs.checkUpdates) {
          startupUpdateDone.current = true;
          void api
            .updateCheck()
            .then((info) => {
              if (info.newer && info.latest !== stateRef.current.prefs.skipUpdate) setUpdateInfo(info);
            })
            .catch(() => undefined); // offline – beim nächsten Start erneut
        }
      })
      .catch(() => undefined);
  }, [refreshSetup]);

  /** Manuelle Update-Suche (Datei → Info); liefert eine Meldung, wenn kein Dialog folgt. */
  const checkForUpdates = useCallback(async (): Promise<string> => {
    try {
      const info = await api.updateCheck();
      if (info.newer) {
        setUpdateInfo(info);
        return "";
      }
      return `${t("VisuTeX ist aktuell")} (${info.current}).`;
    } catch (error) {
      return `${t("Update-Suche fehlgeschlagen")}: ${errorText(error)}`;
    }
  }, [t]);

  /** Fragt nach einem Ordner für nachgeladene TeX-Pakete; aktiviert das Nachladen. */
  const choosePackageDir = useCallback(async (): Promise<boolean> => {
    const dir = await open({ directory: true, title: t("Speicherort für nachgeladene TeX-Pakete wählen") });
    if (!dir || Array.isArray(dir)) return false;
    const next = { ...stateRef.current.prefs, packageCacheDir: dir, allowOnline: true };
    await api.setTexOptions(texOptions(next)).catch(() => undefined);
    setPrefs(next);
    return true;
  }, [t, texOptions]);

  const chooseBundle = useCallback(async () => {
    const file = await open({ title: t("TeX-Bundle wählen"), filters: [{ name: t("TeX-Bundle (ZIP)"), extensions: ["zip"] }] });
    if (!file || Array.isArray(file)) return;
    const next = { ...stateRef.current.prefs, bundlePath: file };
    await api.setTexOptions(texOptions(next)).catch(() => undefined);
    setPrefs(next);
  }, [t, texOptions]);

  const enableOnline = useCallback(async () => {
    const next = { ...stateRef.current.prefs, allowOnline: true };
    await api.setTexOptions(texOptions(next)).catch(() => undefined);
    setPrefs(next);
  }, [texOptions]);

  // Fenstertitel
  useEffect(() => {
    if (!isTauri) return;
    const name = docState?.name ?? "VisuTeX";
    void getCurrentWindow().setTitle(`${dirty ? "● " : ""}${name} – VisuTeX`).catch(() => undefined);
  }, [docState?.name, dirty]);

  // ---------------------------------------------------------------- Einstellungen

  const setSettings = useCallback(
    (next: DocumentSettings) => {
      setSettingsState(normalizeSettings(withLanguageBibliography(stateRef.current.settings, next)));
      docCode.current = null;
      markDirty();
    },
    [markDirty],
  );
  const updateSettings = useCallback(
    (patch: Partial<DocumentSettings>) => setSettings({ ...stateRef.current.settings, ...patch }),
    [setSettings],
  );

  // ---------------------------------------------------------------- Projekte

  const rememberRecent = useCallback((path: string) => {
    setRecent((current) => {
      const next = [{ path, name: fileName(path), openedAt: Date.now() }, ...current.filter((entry) => entry.path !== path)].slice(0, 12);
      localStorage.setItem("visutex-recent", JSON.stringify(next));
      return next;
    });
  }, []);

  // ---------------------------------------------------------------- Präsentationen (Folien-Editor)

  /** Öffnet den Folien-Editor mit einer neuen bzw. der zuletzt bearbeiteten Präsentation. */
  const showSlides = useCallback(() => {
    setSlidesSession((value) => value ?? newSlidesSession());
    setSlidesOpen(true);
    setBackstage(null);
  }, []);

  const slidesRef = useRef(slidesSession);
  slidesRef.current = slidesSession;
  /** Dateibefehle des Folien-Editors (gesetzt, solange er angezeigt wird). */
  const slideActions = useRef<SlideFileActions | null>(null);
  /** Ungespeicherte Präsentation verwerfen? (nur wenn eine andere geladen werden soll) */
  const confirmDiscardSlides = useCallback(async () => {
    if (slideActions.current) return slideActions.current.confirmDiscard();
    if (!slidesRef.current?.dirty) return true;
    const answer = await dialogs.confirm({
      title: t("Präsentation"),
      message: t("Die Präsentation enthält ungespeicherte Änderungen. Verwerfen?"),
      confirmLabel: t("Verwerfen"),
      cancelLabel: t("Abbrechen"),
      danger: true,
    });
    return answer === true;
  }, [dialogs, t]);

  /** Setzt eine neue bzw. geöffnete Präsentation (im laufenden Editor ohne alten Rückgängig-Verlauf). */
  const replaceSlides = useCallback((next: SlidesSession) => {
    if (slideActions.current) slideActions.current.replace(next);
    else setSlidesSession(next);
    setSlidesOpen(true);
    setBackstage(null);
  }, []);

  const newPresentation = useCallback(
    async (layout: "title" | "blank" = "title") => {
      if (!(await confirmDiscardSlides())) return;
      replaceSlides(newSlidesSession(layout));
    },
    [confirmDiscardSlides, replaceSlides],
  );

  const openPresentation = useCallback(
    async (path: string) => {
      if (slidesRef.current?.path !== path && !(await confirmDiscardSlides())) return;
      try {
        const deck = normalizeDeck(await api.slidesLoad(path));
        replaceSlides({ deck, path, dirty: false });
        rememberRecent(path);
      } catch (error) {
        await dialogs.alert(t("Öffnen fehlgeschlagen"), errorText(error));
      }
    },
    [confirmDiscardSlides, dialogs, rememberRecent, replaceSlides, t],
  );

  const applyOpened = useCallback(
    (opened: OpenedProject, name?: string) => {
      if (!editor) return;
      const project = opened.project;
      setSlidesOpen(false);
      setSettingsState(normalizeSettings(project.settings));
      setCustomPreamble(project.customPreamble);
      editor.commands.setContent(project.document, { emitUpdate: false });
      setContentVersion((value) => value + 1);
      editor.commands.focus("start");
      docCode.current = project.code;
      codeBase.current = null;
      setDocState({
        root: opened.root,
        path: opened.path,
        kind: opened.kind,
        temporary: opened.temporary,
        stamp: opened.stamp,
        name: name ?? (fileName(opened.path) || t("Unbenannt")),
      });
      setDirty(false);
      setCompileStatus({ state: "idle" });
      setCodeMessages([]);
      lastCompiled.current = null;
      if (project.lastMode === "code" && project.code) {
        setCode(project.code);
        setModeState("code");
      } else {
        setModeState("visual");
      }
      for (const warning of opened.warnings.slice(0, 3)) notify(warning);
      if (opened.missingFiles.length) notify(`${t("Fehlende Dateien im Projektordner:")} ${opened.missingFiles.join(", ")}`, "error");
      if (opened.path) rememberRecent(opened.path);
      setBackstage(null);
    },
    [editor, notify, rememberRecent, t],
  );

  /** Fragt bei ungespeicherten Änderungen nach; false = abbrechen. */
  const confirmDiscard = useCallback(async () => {
    if (!stateRef.current.dirty) return true;
    const answer = await dialogs.confirm({
      title: t("Ungespeicherte Änderungen"),
      message: t("Das Dokument enthält ungespeicherte Änderungen. Jetzt speichern?"),
      confirmLabel: t("Speichern"),
      thirdLabel: t("Nicht speichern"),
    });
    if (answer === "third") return true;
    if (answer === true) return saveRef.current();
    return false;
  }, [dialogs, t]);

  const newDocument = useCallback(
    async (template: TemplateInfo | null) => {
      if (!(await confirmDiscard())) return;
      try {
        const addon = template?.source.startsWith("addon:") ? template.source.slice(6) : null;
        const opened = await api.newProject(template?.id ?? null, addon, languageInfo(stateRef.current.prefs.language).documentLanguage);
        applyOpened(opened, t("Unbenannt"));
        setDirty(false);
        localStorage.removeItem("visutex-recovery");
        setRecovery(null);
      } catch (error) {
        await dialogs.alert(t("Neues Dokument"), errorText(error));
      }
    },
    [applyOpened, confirmDiscard, dialogs, t],
  );

  const openDocument = useCallback(
    async (path?: string) => {
      const selected =
        path ??
        (await open({
          title: t("Dokument öffnen"),
          multiple: false,
          filters: [
            { name: t("VisuTeX, LaTeX und Präsentationen"), extensions: ["visutex", "json", "tex", "vtxslides"] },
            { name: t("VisuTeX-Projekt"), extensions: ["visutex", "json"] },
            { name: "LaTeX", extensions: ["tex"] },
            { name: t("VisuTeX-Präsentation"), extensions: ["vtxslides"] },
          ],
        }));
      if (!selected || Array.isArray(selected)) return;
      // Präsentationen öffnen im Folien-Editor; das Dokument bleibt geöffnet.
      if (selected.toLowerCase().endsWith(".vtxslides")) {
        await openPresentation(selected);
        return;
      }
      if (!(await confirmDiscard())) return;
      try {
        applyOpened(await api.loadProject(selected));
      } catch (error) {
        await dialogs.alert(t("Öffnen fehlgeschlagen"), errorText(error));
      }
    },
    [applyOpened, confirmDiscard, dialogs, openPresentation, t],
  );

  /** Aktuelles LaTeX (Code-Ansicht: Code; sonst exakter Quelltext oder Export). */
  const currentLatex = useCallback(async (): Promise<{ latex: string; assets: EmbeddedAsset[]; generatedPreamble: string }> => {
    const current = stateRef.current;
    if (current.mode === "code") {
      return { latex: current.code, assets: codeBase.current?.assets ?? [], generatedPreamble: codeBase.current?.generatedPreamble ?? "" };
    }
    const exported = await api.exportDocument(editor?.getJSON() ?? { type: "doc" }, current.settings, current.customPreamble);
    if (docCode.current !== null) return { latex: docCode.current, assets: exported.assets, generatedPreamble: exported.generatedPreamble };
    for (const warning of exported.warnings.slice(0, 2)) notify(warning);
    return { latex: exported.latex, assets: exported.assets, generatedPreamble: exported.generatedPreamble };
  }, [editor, notify]);

  /** Übernimmt geänderten Code in das visuelle Dokument (ohne die Ansicht zu wechseln). */
  const syncFromCode = useCallback(async () => {
    const current = stateRef.current;
    if (!editor || current.mode !== "code") return true;
    let base = codeBase.current;
    try {
      if (!base) {
        // Projekt wurde direkt in der Code-Ansicht geöffnet: Vergleichsstand nachträglich bilden.
        if (docCode.current !== null && current.code === docCode.current) return true;
        const exported = await api.exportDocument(editor.getJSON(), current.settings, current.customPreamble);
        base = { code: docCode.current ?? exported.latex, generatedPreamble: exported.generatedPreamble, assets: exported.assets };
        codeBase.current = base;
      }
      if (base.code === current.code) return true;
      const result = await api.codeToDocument({
        code: current.code,
        settings: current.settings,
        generatedPreamble: base.generatedPreamble,
        customPreamble: current.customPreamble,
        projectRoot: current.docState?.root ?? null,
      });
      editor.commands.setContent(result.document, { emitUpdate: false });
      setContentVersion((value) => value + 1);
      setSettingsState(normalizeSettings(result.settings));
      setCustomPreamble(result.customPreamble);
      docCode.current = current.code;
      codeBase.current = { ...base, code: current.code };
      if (result.rawBlockCount > 0) notify(`${result.rawBlockCount} ${t("Abschnitt(e) bleiben als LaTeX-Block erhalten (keine visuelle Entsprechung).")}`);
      if (result.missingResources.length) notify(`${t("Fehlende Dateien:")} ${result.missingResources.join(", ")}`, "error");
      return true;
    } catch (error) {
      await dialogs.alert(t("Code konnte nicht übernommen werden"), errorText(error));
      return false;
    }
  }, [dialogs, editor, notify, t]);

  const saveAs = useCallback(async (): Promise<boolean> => {
    const current = stateRef.current.docState;
    if (!editor || !current) return false;
    const suggested = current.path ?? `${t("Dokument")}.visutex`;
    const path = await save({
      title: t("Speichern unter"),
      defaultPath: suggested,
      filters: [
        { name: t("VisuTeX-Projekt"), extensions: ["visutex"] },
        { name: t("LaTeX-Dokument"), extensions: ["tex"] },
      ],
    });
    if (!path) return false;
    const kind = path.toLowerCase().endsWith(".tex") ? "tex" : "visutex";
    return saveToRef.current(path, kind, current.root);
  }, [editor, t]);

  const saveTo = useCallback(
    async (path: string, kind: "visutex" | "tex", sourceRoot: string | null): Promise<boolean> => {
      if (!editor) return false;
      if (!(await syncFromCode())) return false;
      const current = stateRef.current;
      try {
        let saved;
        if (kind === "tex") {
          const { latex, assets } = await currentLatex();
          saved = await api.exportTex(path, latex, assets, sourceRoot);
          docCode.current = latex;
        } else {
          const project: Project = {
            format: "visutex-project",
            version: 2,
            settings: current.settings,
            document: editor.getJSON(),
            customPreamble: current.customPreamble,
            lastMode: current.mode,
            code: current.mode === "code" ? current.code : docCode.current,
          };
          saved = await api.saveProject(path, project, sourceRoot);
        }
        setDocState({ root: saved.root, path: saved.path, kind, temporary: false, stamp: saved.stamp, name: fileName(saved.path) });
        setDirty(false);
        rememberRecent(saved.path);
        localStorage.removeItem("visutex-recovery");
        if (saved.missingFiles.length) notify(`${t("Nicht gefundene Dateien (nicht kopiert):")} ${saved.missingFiles.join(", ")}`, "error");
        else notify(`${t("Gespeichert:")} ${fileName(saved.path)}`);
        return true;
      } catch (error) {
        await dialogs.alert(t("Speichern fehlgeschlagen"), errorText(error));
        return false;
      }
    },
    [currentLatex, dialogs, editor, notify, rememberRecent, syncFromCode, t],
  );
  const saveToRef = useRef(saveTo);
  saveToRef.current = saveTo;

  const saveDocument = useCallback(async (): Promise<boolean> => {
    const current = stateRef.current.docState;
    if (!current) return false;
    if (!current.path || current.temporary) return saveAs();
    return saveTo(current.path, current.kind, current.root);
  }, [saveAs, saveTo]);
  const saveRef = useRef(saveDocument);
  saveRef.current = saveDocument;

  const exportTex = useCallback(async () => {
    const current = stateRef.current.docState;
    if (!current) return;
    const path = await save({ title: t("Als LaTeX exportieren"), defaultPath: `${(current.name || "dokument").replace(/\.[^.]+$/, "")}.tex`, filters: [{ name: "LaTeX", extensions: ["tex"] }] });
    if (!path) return;
    try {
      if (!(await syncFromCode())) return;
      const { latex, assets } = await currentLatex();
      const saved = await api.exportTex(path, latex, assets, current.root);
      notify(saved.missingFiles.length ? `${t("Exportiert, aber Dateien fehlen:")} ${saved.missingFiles.join(", ")}` : `${t("Exportiert:")} ${fileName(path)}`, saved.missingFiles.length ? "error" : "info");
      setBackstage(null);
    } catch (error) {
      await dialogs.alert(t("Export fehlgeschlagen"), errorText(error));
    }
  }, [currentLatex, dialogs, notify, syncFromCode, t]);

  // ---------------------------------------------------------------- Kompilieren

  /** Fehlen TeX-Dateien im mitgelieferten Bundle: Nachladen anbieten und nach dem Speicherort fragen. */
  const offerPackageDownload = useCallback(
    async (files: string[]) => {
      const location = stateRef.current.prefs.packageCacheDir || systemCheck?.packageCacheDir || t("Standard (App-Datenordner)");
      const answer = await dialogs.confirm({
        title: t("Fehlende TeX-Pakete"),
        message: `${t("Diese Dateien sind nicht im mitgelieferten TeX-Bundle enthalten:")}\n${files.slice(0, 12).join(", ")}\n\n${t("Sollen fehlende Pakete aus dem Internet nachgeladen werden? Sie werden dauerhaft gespeichert in:")}\n${location}`,
        confirmLabel: t("Nachladen"),
        cancelLabel: t("Nicht nachladen"),
        thirdLabel: t("Anderen Speicherort …"),
      });
      if (answer === "third") {
        if (await choosePackageDir()) void compileRef.current();
      } else if (answer === true) {
        await enableOnline();
        void compileRef.current();
      }
    },
    [choosePackageDir, dialogs, enableOnline, systemCheck?.packageCacheDir, t],
  );
  const offerPackageDownloadRef = useRef(offerPackageDownload);
  offerPackageDownloadRef.current = offerPackageDownload;

  const compile = useCallback(async (): Promise<number | null> => {
    const current = stateRef.current;
    if (!current.docState) return null;
    setView((value) => ({ ...value, compiling: true }));
    setCompileStatus({ state: "compiling", progress: t("Bereite vor …") });
    const unlisten = await api.onCompileProgress((text) => setCompileStatus({ state: "compiling", progress: text })).catch(() => null);
    try {
      const { latex, assets } = await currentLatex();
      const result = await api.compileDocument(latex, current.docState.root, assets, current.prefs.allowOnline);
      lastCompiled.current = { pdfId: result.pdfId };
      setCompileStatus({ state: "done", result });
      setCodeMessages(result.messages);
      // PDF ohne fehlende Pakete erzeugt → Nachladen anbieten
      if (result.missingFiles?.length && !stateRef.current.prefs.allowOnline) {
        void offerPackageDownloadRef.current(result.missingFiles);
      }
      return result.pdfId;
    } catch (error) {
      const failure: CompileFailure =
        error && typeof error === "object" && "message" in error ? (error as CompileFailure) : { message: errorText(error), messages: [], log: "" };
      setCompileStatus((previous) => ({
        state: "failed",
        failure,
        previous: previous.state === "done" ? previous.result : previous.state === "failed" ? previous.previous : null,
      }));
      setCodeMessages(failure.messages);
      setView((value) => ({ ...value, showPdf: true }));
      if (failure.missingFiles?.length && !stateRef.current.prefs.allowOnline) {
        void offerPackageDownloadRef.current(failure.missingFiles);
      }
      return null;
    } finally {
      unlisten?.();
      setView((value) => ({ ...value, compiling: false }));
    }
  }, [currentLatex, t]);
  const compileRef = useRef(compile);
  compileRef.current = compile;

  const exportPdf = useCallback(async () => {
    const pdfId = await compile();
    if (pdfId === null) {
      notify(t("Das PDF konnte nicht erzeugt werden – siehe Meldungen in der PDF-Vorschau."), "error");
      return;
    }
    const name = (stateRef.current.docState?.name || "dokument").replace(/\.[^.]+$/, "");
    const path = await save({ title: t("PDF speichern"), defaultPath: `${name}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return;
    try {
      await api.savePdf(pdfId, path);
      notify(`${t("PDF gespeichert:")} ${fileName(path)}`);
      setBackstage(null);
    } catch (error) {
      await dialogs.alert(t("PDF speichern"), errorText(error));
    }
  }, [compile, dialogs, notify, t]);

  const savePdf = useCallback(async () => {
    const pdfId = lastCompiled.current?.pdfId;
    if (!pdfId) return exportPdf();
    const name = (stateRef.current.docState?.name || "dokument").replace(/\.[^.]+$/, "");
    const path = await save({ title: t("PDF speichern"), defaultPath: `${name}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return;
    try {
      await api.savePdf(pdfId, path);
      notify(`${t("PDF gespeichert:")} ${fileName(path)}`);
    } catch (error) {
      await dialogs.alert(t("PDF speichern"), errorText(error));
    }
  }, [dialogs, exportPdf, notify, t]);

  // Automatisch kompilieren (optional, bei geöffneter Vorschau)
  useEffect(() => {
    if (!prefs.autoCompile || !view.showPdf || !editor) return;
    let timer: number | undefined;
    const schedule = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void compileRef.current(), 2500);
    };
    editor.on("update", schedule);
    return () => {
      window.clearTimeout(timer);
      editor.off("update", schedule);
    };
  }, [editor, prefs.autoCompile, view.showPdf]);
  useEffect(() => {
    if (!prefs.autoCompile || !view.showPdf || mode !== "code") return;
    const timer = window.setTimeout(() => void compileRef.current(), 2500);
    return () => window.clearTimeout(timer);
  }, [code, mode, prefs.autoCompile, view.showPdf]);

  // ---------------------------------------------------------------- Ansicht wechseln

  const setMode = useCallback(
    async (next: "visual" | "code") => {
      const current = stateRef.current;
      if (!editor || next === current.mode) return;
      if (next === "code") {
        try {
          const exported = await api.exportDocument(editor.getJSON(), current.settings, current.customPreamble);
          const text = docCode.current ?? exported.latex;
          codeBase.current = { code: text, generatedPreamble: exported.generatedPreamble, assets: exported.assets };
          setCode(text);
          setSearch(null);
          painter?.stop();
          setModeState("code");
        } catch (error) {
          await dialogs.alert(t("Code-Ansicht"), errorText(error));
        }
        return;
      }
      if (await syncFromCode()) {
        setModeState("visual");
        window.setTimeout(() => editor.storage.pagination.remeasure(), 50);
      }
    },
    [dialogs, editor, painter, syncFromCode, t],
  );

  const onCodeChange = useCallback(
    (value: string) => {
      setCode(value);
      if (value !== codeBase.current?.code) markDirty();
    },
    [markDirty],
  );

  // ---------------------------------------------------------------- Wiederherstellung, externe Änderungen, Schließen

  useEffect(() => {
    if (!prefs.autosave || !dirty || !editor || !docState) return;
    const timer = window.setTimeout(() => {
      const current = stateRef.current;
      const value: Recovery = {
        name: docState.name,
        savedAt: Date.now(),
        root: docState.root,
        path: docState.path,
        kind: docState.kind,
        temporary: docState.temporary,
        project: {
          format: "visutex-project",
          version: 2,
          settings: current.settings,
          document: stripPreviews(editor.getJSON()),
          customPreamble: current.customPreamble,
          lastMode: current.mode,
          code: current.mode === "code" ? current.code : docCode.current,
        },
      };
      try {
        localStorage.setItem("visutex-recovery", JSON.stringify(value));
      } catch {
        // Speicher voll – Wiederherstellung ist optional.
      }
    }, 2000);
    return () => window.clearTimeout(timer);
  }, [dirty, editor, docState, prefs.autosave, code, settings, customPreamble]);

  const restoreRecovery = useCallback(async () => {
    const value = recovery;
    if (!value || !editor) return;
    if (isTauri) await api.allowProjectDir(value.root).catch(() => undefined);
    applyOpened(
      { project: value.project, root: value.root, path: value.path, kind: value.kind, warnings: [], missingFiles: [], stamp: null, temporary: value.temporary },
      value.name,
    );
    setDirty(true);
    setRecovery(null);
  }, [applyOpened, editor, recovery]);

  useEffect(() => {
    if (!isTauri) return;
    const onFocus = async () => {
      const current = stateRef.current.docState;
      if (!current?.path || current.temporary || !current.stamp) return;
      const stamp = await api.fileStamp(current.path).catch(() => null);
      if (!stamp || (stamp.modifiedMs === current.stamp.modifiedMs && stamp.size === current.stamp.size)) return;
      setDocState({ ...current, stamp });
      const answer = await dialogs.confirm({
        title: t("Datei wurde extern geändert"),
        message: `${fileName(current.path)} ${t("wurde außerhalb von VisuTeX geändert. Externe Version laden?")}${stateRef.current.dirty ? ` ${t("Ungespeicherte Änderungen gehen verloren.")}` : ""}`,
        confirmLabel: t("Externe Version laden"),
        cancelLabel: t("Behalten"),
      });
      if (answer === true) {
        try {
          applyOpened(await api.loadProject(current.path));
        } catch (error) {
          notify(errorText(error), "error");
        }
      }
    };
    const listener = () => void onFocus();
    window.addEventListener("focus", listener);
    return () => window.removeEventListener("focus", listener);
  }, [applyOpened, dialogs, notify, t]);

  useEffect(() => {
    if (!isTauri) return;
    const windowHandle = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    void windowHandle
      .onCloseRequested(async (event) => {
        if (!stateRef.current.dirty) return;
        event.preventDefault();
        if (await confirmDiscard()) {
          localStorage.removeItem("visutex-recovery");
          await windowHandle.destroy();
        }
      })
      .then((handler) => {
        unlisten = handler;
      });
    return () => unlisten?.();
  }, [confirmDiscard]);

  // Erststart: Startseite mit Vorlagen; im Hintergrund ein leeres Dokument bereitstellen.
  const initialized = useRef(false);
  useEffect(() => {
    if (!editor || initialized.current) return;
    initialized.current = true;
    void api
      .newProject(null, null)
      .then((opened) => {
        applyOpened(opened, t("Unbenannt"));
        setBackstage("home");
      })
      .catch((error) => notify(errorText(error), "error"));
  }, [applyOpened, editor, notify, t]);

  // ---------------------------------------------------------------- Einfügen und Bearbeiten

  const projectRoot = docState?.root ?? null;

  const insertImageFiles = useCallback(
    async (files: File[]) => {
      const root = getRuntime().projectRoot;
      if (!editor || !root) return;
      for (const file of files) {
        const dataUrl = await new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result));
          reader.onerror = () => reject(reader.error);
          reader.readAsDataURL(file);
        });
        try {
          const imported = await api.importImageData(dataUrl, root);
          editor.chain().focus().insertContent({ type: "image", attrs: { latexPath: imported.latexPath, widthPercent: 80 } }).run();
          if (imported.convertedFrom) notify(`${t("Bild wurde nach")} ${imported.latexPath.split(".").pop()?.toUpperCase()} ${t("konvertiert.")}`);
        } catch (error) {
          notify(errorText(error), "error");
        }
      }
    },
    [editor, notify, t],
  );
  insertImageRef.current = insertImageFiles;

  const chooseImage = useCallback(async (): Promise<{ latexPath: string; widthPercent: number } | null> => {
    if (!projectRoot) return null;
    const path = await open({
      title: t("Bild einfügen"),
      multiple: false,
      filters: [{ name: t("Bilder"), extensions: ["png", "jpg", "jpeg", "pdf", "svg", "gif", "webp", "bmp", "tif", "tiff"] }],
    });
    if (!path || Array.isArray(path)) return null;
    try {
      const imported = await api.importImage(path, projectRoot);
      if (imported.convertedFrom) notify(`${imported.convertedFrom.toUpperCase()} ${t("wurde für LaTeX konvertiert:")} ${imported.latexPath}`);
      const widthPercent = imported.width ? Math.max(10, Math.min(100, Math.round((imported.width / 605) * 100))) : 80;
      return { latexPath: imported.latexPath, widthPercent };
    } catch (error) {
      await dialogs.alert(t("Bild einfügen"), errorText(error));
      return null;
    }
  }, [dialogs, notify, projectRoot, t]);

  const nodeAt = (pos: number) => editor?.state.doc.nodeAt(pos) ?? null;
  const setNodeAttrs = (pos: number, attrs: Record<string, unknown>) => {
    if (!editor) return;
    const node = editor.state.doc.nodeAt(pos);
    if (!node) return;
    editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, ...attrs }));
    editor.commands.focus();
  };

  const footnoteForm = (text = "") =>
    dialogs.form({ title: t("Fußnote"), fields: [{ kind: "text", name: "text", label: t("Fußnotentext"), value: text, multiline: true, required: true, autoFocus: true }] });

  const acronymForm = async (initial?: { key: string; short: string; long: string; command: string }) => {
    const known = analysis?.acronyms ?? [];
    const values = await dialogs.form({
      title: t("Abkürzung"),
      description: t("Beim ersten Vorkommen wird die Langform ausgeschrieben; das Abkürzungsverzeichnis entsteht automatisch."),
      fields: [
        { kind: "text", name: "short", label: t("Abkürzung"), value: initial?.short ?? "", required: true, list: known.map((entry) => entry.short) },
        { kind: "text", name: "long", label: t("Langform"), value: initial?.long ?? "" },
        { kind: "text", name: "key", label: t("LaTeX-Schlüssel (optional)"), value: initial?.key ?? "", pattern: labelPattern, patternMessage: t("Nur Buchstaben, Ziffern, : . _ -") },
        {
          kind: "select",
          name: "command",
          label: t("Darstellung"),
          value: initial?.command ?? "ac",
          options: [
            { value: "ac", label: t("Automatisch (\\ac)") },
            { value: "acs", label: t("Immer kurz (\\acs)") },
            { value: "acl", label: t("Immer lang (\\acl)") },
            { value: "acf", label: t("Lang mit Kurzform (\\acf)") },
            { value: "acp", label: t("Plural (\\acp)") },
          ],
        },
      ],
    });
    if (!values) return null;
    const short = String(values.short).trim();
    const existing = known.find((entry) => entry.short === short);
    const long = String(values.long).trim() || existing?.long || "";
    if (!long) {
      await dialogs.alert(t("Abkürzung"), t("Bitte die Langform angeben."));
      return null;
    }
    const key = String(values.key).trim() || existing?.key || short.replace(/[^A-Za-z0-9:._-]/g, "") || "abk";
    return { key, short, long, command: String(values.command) };
  };

  const crossReferenceForm = async (initial?: { label: string; kind: string }) => {
    const labels = analysis?.labels ?? [];
    const kindNames: Record<string, string> = { heading: t("Kapitel"), figure: t("Abbildung"), table: t("Tabelle"), equation: t("Formel"), drawing: t("Zeichnung"), raw: "LaTeX" };
    const values = await dialogs.form({
      title: t("Querverweis"),
      description: labels.length ? undefined : t("Noch keine Labels vorhanden – Labels bei Überschriften, Bildern, Tabellen oder nummerierten Formeln vergeben."),
      fields: [
        labels.length
          ? {
              kind: "select",
              name: "label",
              label: t("Ziel"),
              value: initial?.label ?? labels[0]?.label,
              options: labels.map((entry) => ({ value: entry.label, label: `${kindNames[entry.kind] ?? entry.kind}: ${entry.title || entry.label} (${entry.label})` })),
            }
          : { kind: "text", name: "label", label: t("Label"), value: initial?.label ?? "", required: true, pattern: labelPattern },
        {
          kind: "select",
          name: "kind",
          label: t("Verweis auf"),
          value: initial?.kind ?? "ref",
          options: [
            { value: "ref", label: t("Nummer (\\ref)") },
            { value: "pageref", label: t("Seitenzahl (\\pageref)") },
            { value: "eqref", label: t("Formelnummer in Klammern (\\eqref)") },
            { value: "autoref", label: t("Mit Bezeichnung, z. B. „Abbildung 3“ (\\autoref)") },
          ],
        },
      ],
    });
    return values ? { label: String(values.label), kind: String(values.kind) } : null;
  };

  const sectionBreakForm = (initial?: { columns: number; orientation: string }) =>
    dialogs.form({
      title: t("Abschnittsumbruch"),
      description: t("Ab hier gelten neue Spalten bzw. eine neue Seitenausrichtung (neue Seite)."),
      fields: [
        { kind: "select", name: "columns", label: t("Spalten"), value: String(initial?.columns ?? 2), options: ["1", "2", "3"].map((value) => ({ value, label: value })) },
        {
          kind: "select",
          name: "orientation",
          label: t("Ausrichtung"),
          value: initial?.orientation ?? "keep",
          options: [
            { value: "keep", label: t("Unverändert") },
            { value: "portrait", label: t("Hochformat") },
            { value: "landscape", label: t("Querformat") },
          ],
        },
      ],
    });

  /** Einheiten-Kurzformen in siunitx-Befehle (Ω → \ohm, µ → \micro, …). */
  const normalizeUnit = (unit: string) =>
    unit
      .trim()
      .replace(/Ω|Ohm\b|ohm\b/g, "\\ohm")
      .replace(/µ|μ/g, "\\micro")
      .replace(/°C/g, "\\degreeCelsius")
      .replace(/°/g, "\\degree")
      .replace(/%/g, "\\percent");

  const quantityForm = async (initial?: Record<string, unknown>) => {
    const values = await dialogs.form({
      title: t("Größe mit Einheit"),
      description: t("Zahl und Einheit werden wie in LaTeX (siunitx) gesetzt, z. B. 4,7 kΩ. Einheiten als siunitx-Befehle (\\kilo\\ohm) oder Kurzform (kΩ, mV)."),
      fields: [
        {
          kind: "select",
          name: "command",
          label: t("Art"),
          value: String(initial?.command ?? (customPreamble?.includes("\\SI{") ? "SI" : "qty")),
          options: [
            { value: "qty", label: t("Zahl mit Einheit (\\qty)") },
            { value: "SI", label: t("Zahl mit Einheit (\\SI)") },
            { value: "num", label: t("Nur Zahl (\\num)") },
            { value: "unit", label: t("Nur Einheit (\\unit)") },
            { value: "si", label: t("Nur Einheit (\\si)") },
            { value: "qtyrange", label: t("Bereich (\\qtyrange)") },
            { value: "ang", label: t("Winkel (\\ang)") },
          ],
        },
        { kind: "text", name: "value", label: t("Zahl"), value: String(initial?.value ?? ""), placeholder: "4.7  ·  1e-3  ·  1{,}5", autoFocus: true },
        { kind: "text", name: "value2", label: t("Bis (nur bei Bereich)"), value: String(initial?.value2 ?? "") },
        { kind: "text", name: "unit", label: t("Einheit"), value: String(initial?.unit ?? "\\volt"), mono: true, list: COMMON_UNITS.map((unit) => unit.latex) },
        { kind: "info", text: COMMON_UNITS.slice(0, 20).map((unit) => `${unit.label} = ${unit.latex}`).join("   ") },
      ],
      validate: (input) => {
        const command = String(input.command);
        if (!["unit", "si"].includes(command) && !String(input.value).trim()) return t("Bitte eine Zahl angeben.");
        const numbers = command === "ang" ? String(input.value).split(";").filter((part) => part.trim()) : [String(input.value), ...(command === "qtyrange" ? [String(input.value2)] : [])];
        if (!["unit", "si"].includes(command) && !numbers.every(isSiunitxNumber)) {
          return t("Bitte eine Zahl angeben (z. B. 4.7, 1,5 oder 1e-3) – siunitx akzeptiert hier keinen Text.");
        }
        if (["qty", "SI", "unit", "si", "qtyrange"].includes(command) && !String(input.unit).trim()) return t("Bitte eine Einheit angeben.");
        return null;
      },
    });
    if (!values) return null;
    const attrs = {
      command: String(values.command),
      value: String(values.value).trim(),
      value2: String(values.value2).trim(),
      unit: normalizeUnit(String(values.unit)),
    };
    return { ...attrs, preview: formatQuantity(attrs, settings.language) };
  };

  const verticalSpaceForm = (initial?: Record<string, unknown>) =>
    dialogs.form({
      title: t("Vertikaler Abstand"),
      fields: [
        {
          kind: "select",
          name: "command",
          label: t("Art"),
          value: String(initial?.command ?? "vspace"),
          options: [
            { value: "vspace", label: t("Fester Abstand (\\vspace)") },
            { value: "vspace*", label: t("Auch am Seitenanfang (\\vspace*)") },
            { value: "smallskip", label: t("Klein (\\smallskip)") },
            { value: "medskip", label: t("Mittel (\\medskip)") },
            { value: "bigskip", label: t("Groß (\\bigskip)") },
            { value: "vfill", label: t("Rest der Seite füllen (\\vfill)") },
          ],
        },
        { kind: "text", name: "size", label: t("Größe (bei festem Abstand)"), value: String(initial?.size ?? "1cm"), mono: true, list: ["0.5cm", "1cm", "2cm", "1em", "\\baselineskip", "-0.5cm"] },
      ],
    });

  /** Umgebung einfügen bzw. die markierten Absätze damit umschließen. */
  const insertEnvironment = async (kind: "box" | "columns" | "abstract" | "center" | "minipage" | "custom") => {
    if (!editor) return;
    let name = "";
    let args = "";
    if (kind === "box") {
      const values = await dialogs.form({
        title: t("Box (tcolorbox)"),
        fields: [
          { kind: "text", name: "title", label: t("Titel (optional)"), value: "", autoFocus: true },
          {
            kind: "select",
            name: "color",
            label: t("Farbe"),
            value: "blue",
            options: [
              { value: "blue", label: t("Blau") },
              { value: "green", label: t("Grün") },
              { value: "red", label: t("Rot") },
              { value: "orange", label: t("Orange") },
              { value: "gray", label: t("Grau") },
            ],
          },
        ],
      });
      if (!values) return;
      const title = String(values.title).trim().replace(/[{}\\]/g, "");
      const color = String(values.color);
      args = `[${title ? `title={${title}}, ` : ""}colback=${color}!5!white, colframe=${color}!60!black]`;
      name = "tcolorbox";
    } else if (kind === "columns") {
      const values = await dialogs.form({
        title: t("Mehrspaltiger Bereich"),
        description: t("Nur dieser Bereich wird mehrspaltig gesetzt (multicols)."),
        fields: [{ kind: "select", name: "columns", label: t("Spalten"), value: "2", options: ["2", "3", "4", "5"].map((value) => ({ value, label: value })) }],
      });
      if (!values) return;
      name = "multicols";
      args = `{${values.columns}}`;
    } else if (kind === "minipage") {
      const values = await dialogs.form({
        title: t("Minipage"),
        fields: [{ kind: "number", name: "width", label: t("Breite"), value: 48, min: 5, max: 100, unit: "%" }],
      });
      if (!values) return;
      name = "minipage";
      args = `{${(Number(values.width) / 100).toFixed(2)}\\linewidth}`;
    } else if (kind === "custom") {
      const values = await dialogs.form({
        title: t("Eigene Umgebung"),
        description: t("Beliebige LaTeX-Umgebung, z. B. eine Box aus der eigenen Präambel."),
        fields: [
          { kind: "text", name: "name", label: t("Name"), value: "", required: true, pattern: /^[A-Za-z]+\*?$/, patternMessage: t("Nur Buchstaben (optional *)"), autoFocus: true },
          { kind: "text", name: "args", label: t("Argumente (optional)"), value: "", mono: true, placeholder: "[Titel] oder {2}" },
        ],
      });
      if (!values) return;
      name = String(values.name);
      args = String(values.args).trim();
    } else {
      name = kind;
    }
    const attrs = { name, args };
    const { empty } = editor.state.selection;
    if (!empty && editor.can().wrapIn("environmentBlock", attrs)) {
      editor.chain().focus().wrapIn("environmentBlock", attrs).run();
    } else {
      editor.chain().focus().insertContent({ type: "environmentBlock", attrs, content: [{ type: "paragraph" }] }).run();
    }
  };

  const ensureBibFile = useCallback(async (): Promise<string | null> => {
    const current = stateRef.current.settings.bibliography.file;
    if (current) return current;
    const values = await dialogs.form({
      title: t("Literaturdatei"),
      description: t("Für Zitate wird eine BibTeX-Datei im Projektordner verwendet."),
      fields: [{ kind: "text", name: "file", label: t("Dateiname"), value: "literatur.bib", pattern: /^[A-Za-z0-9_\-/]+\.bib$/, patternMessage: t("Bitte einen Dateinamen mit Endung .bib angeben.") }],
    });
    if (!values) return null;
    const file = String(values.file);
    updateSettings({ bibliography: { ...stateRef.current.settings.bibliography, file } });
    return file;
  }, [dialogs, t, updateSettings]);

  const chooseBibliography = useCallback(async () => {
    if (!projectRoot) return;
    const path = await open({ title: t("Literaturdatei wählen"), multiple: false, filters: [{ name: "BibTeX", extensions: ["bib"] }] });
    if (!path || Array.isArray(path)) return;
    const normalizedRoot = projectRoot.replace(/\\/g, "/").replace(/\/$/, "").toLowerCase();
    const normalizedPath = path.replace(/\\/g, "/");
    let relative = normalizedPath.toLowerCase().startsWith(`${normalizedRoot}/`) ? normalizedPath.slice(normalizedRoot.length + 1) : null;
    if (!relative) {
      // Datei liegt außerhalb des Projekts: Einträge in eine Projekt-.bib gleichen Namens übernehmen.
      relative = fileName(path).replace(/[^A-Za-z0-9_.-]/g, "-");
      try {
        const text = await api.readTextFile(path);
        const merged = await api.addBibEntries(projectRoot, relative, text);
        notify(`${merged.added.length} ${t("Einträge in den Projektordner übernommen:")} ${relative}`);
      } catch (error) {
        await dialogs.alert(t("Literaturdatei"), errorText(error));
        return;
      }
    }
    updateSettings({ bibliography: { ...stateRef.current.settings.bibliography, file: relative } });
  }, [dialogs, notify, projectRoot, t, updateSettings]);

  const insertCitationValue = (value: CitationValue, pos: number | null) => {
    if (!editor) return;
    const attrs = { keys: value.keys, prenote: value.prenote, postnote: value.postnote, command: value.command };
    if (pos !== null) setNodeAttrs(pos, attrs);
    else editor.chain().focus().insertContent({ type: "citation", attrs }).run();
  };

  // Bearbeitungsanfragen aus NodeViews (Doppelklick auf Chips)
  const editNode = useCallback(
    async (request: EditRequest) => {
      if (!editor || request.type === "insertImage") return;
      const pos = request.pos;
      const node = nodeAt(pos);
      if (!node) return;
      const attrs = node.attrs as Record<string, unknown>;
      switch (request.type) {
        case "citation":
          setCitationDialog({
            initial: { keys: String(attrs.keys ?? ""), prenote: String(attrs.prenote ?? ""), postnote: String(attrs.postnote ?? ""), command: (attrs.command as CitationValue["command"]) || "cite" },
            pos,
          });
          break;
        case "footnote": {
          const values = await footnoteForm(String(attrs.text ?? ""));
          if (values) setNodeAttrs(pos, { text: String(values.text) });
          break;
        }
        case "acronym": {
          const value = await acronymForm({ key: String(attrs.key ?? ""), short: String(attrs.short ?? ""), long: String(attrs.long ?? ""), command: String(attrs.command ?? "ac") });
          if (value) setNodeAttrs(pos, value);
          break;
        }
        case "crossReference": {
          const value = await crossReferenceForm({ label: String(attrs.label ?? ""), kind: String(attrs.kind ?? "ref") });
          if (value) setNodeAttrs(pos, value);
          break;
        }
        case "rawLatexInline": {
          const values = await dialogs.form({ title: t("LaTeX im Text"), fields: [{ kind: "text", name: "latex", label: "LaTeX", value: String(attrs.latex ?? ""), mono: true, required: true }] });
          if (values) setNodeAttrs(pos, { latex: String(values.latex) });
          break;
        }
        case "pageBreak": {
          if (attrs.breakType !== "section") return;
          const values = await sectionBreakForm({ columns: Number(attrs.columns) || 1, orientation: String(attrs.orientation ?? "keep") });
          if (values) setNodeAttrs(pos, { columns: Number(values.columns), orientation: String(values.orientation) });
          break;
        }
        case "quantity": {
          const value = await quantityForm(attrs);
          if (value) setNodeAttrs(pos, { command: value.command, value: value.value, value2: value.value2, unit: value.unit });
          break;
        }
        case "verticalSpace": {
          const values = await verticalSpaceForm(attrs);
          if (values) setNodeAttrs(pos, { command: String(values.command), size: String(values.size).trim() });
          break;
        }
        case "sketch": {
          const model = await api.sketchFromCode(String(attrs.code ?? "")).catch(() => null);
          setSketch({ initial: model, pos });
          break;
        }
        case "environmentBlock": {
          const values = await dialogs.form({
            title: t("Umgebung"),
            fields: [
              { kind: "text", name: "name", label: t("Name"), value: String(attrs.name ?? ""), required: true, pattern: /^[A-Za-z]+\*?$/, patternMessage: t("Nur Buchstaben (optional *)") },
              { kind: "text", name: "args", label: t("Argumente/Optionen"), value: String(attrs.args ?? ""), mono: true, multiline: true },
              { kind: "checkbox", name: "remove", label: t("Umgebung entfernen (Inhalt behalten)"), value: false },
            ],
          });
          if (!values) break;
          if (values.remove) {
            const current = nodeAt(pos);
            if (current) editor.view.dispatch(editor.state.tr.replaceWith(pos, pos + current.nodeSize, current.content));
          } else {
            setNodeAttrs(pos, { name: String(values.name), args: String(values.args).trim() });
          }
          break;
        }
        default:
          break;
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [editor, analysis, dialogs, t],
  );
  useEffect(() => {
    setRuntime({ editNode: (request) => void editNode(request), chooseImage });
  }, [editNode, chooseImage]);

  const actions: RibbonActions = {
    openBackstage: () => setBackstage("home"),
    setMode: (next) => void setMode(next),
    formatPainter: (sticky) => painter?.start(sticky),
    openSearch: (replace) => {
      if (mode === "code") {
        notify(t("In der Code-Ansicht: Strg+F (Suchen) bzw. Strg+H (Ersetzen) im Editor."));
        return;
      }
      setSearch({ replace });
    },
    insertTitlePage: () => {
      if (!editor) return;
      let exists = false;
      editor.state.doc.forEach((node) => {
        if (node.type.name === "titlePage") exists = true;
      });
      if (exists) {
        notify(t("Das Dokument hat bereits eine Titelseite."));
        return;
      }
      editor
        .chain()
        .focus()
        .insertContentAt(0, {
          type: "titlePage",
          attrs: {
            title: settings.metadata.title || documentLanguage(settings.language).labels.title,
            author: settings.metadata.author,
            institution: documentLanguage(settings.language).labels.institution,
            placeDate: new Date().toLocaleDateString(documentLanguage(settings.language).bcp47, { month: "long", year: "numeric" }),
            authorLabel: documentLanguage(settings.language).labels.author,
            supervisorLabel: documentLanguage(settings.language).labels.supervisor,
          },
        })
        .run();
    },
    insertFrontmatter: async () => {
      const values = await dialogs.form({
        title: t("Vorspann- oder Anhangteil"),
        fields: [
          { kind: "select", name: "kind", label: t("Art"), value: "abstract", options: frontmatterKinds.map((kind) => ({ value: kind, label: t(frontmatterTitles[kind]) })) },
          { kind: "checkbox", name: "inToc", label: t("Im Inhaltsverzeichnis aufführen"), value: false },
        ],
      });
      if (!values || !editor) return;
      const kind = String(values.kind) as FrontmatterKind;
      const title = kind === "summary" ? documentLanguage(settings.language).labels.summary : settings.language === "ngerman" || settings.language === "naustrian" || settings.language === "nswissgerman" ? frontmatterTitles[kind] : t(frontmatterTitles[kind]);
      editor
        .chain()
        .focus()
        .insertContent({ type: "frontmatterBlock", attrs: { kind, title, inToc: Boolean(values.inToc) }, content: [{ type: "paragraph" }] })
        .run();
    },
    insertPageBreak: () => editor?.chain().focus().insertContent({ type: "pageBreak", attrs: { breakType: "page" } }).run(),
    insertSectionBreak: async () => {
      const values = await sectionBreakForm();
      if (!values) return;
      editor?.chain().focus().insertContent({ type: "pageBreak", attrs: { breakType: "section", columns: Number(values.columns), orientation: String(values.orientation) } }).run();
    },
    insertTable: async () => {
      const values = await dialogs.form({
        title: t("Tabelle einfügen"),
        fields: [
          { kind: "number", name: "rows", label: t("Zeilen"), value: 3, min: 1, max: 60 },
          { kind: "number", name: "cols", label: t("Spalten"), value: 3, min: 1, max: 20 },
          { kind: "checkbox", name: "header", label: t("Kopfzeile"), value: true },
          { kind: "text", name: "caption", label: t("Beschriftung (optional)"), value: "" },
          { kind: "text", name: "label", label: t("Label (optional)"), value: "", pattern: labelPattern, placeholder: "tab:messwerte" },
        ],
        validate: (input) => (Number(input.rows) < 1 || Number(input.cols) < 1 || Number(input.rows) > 60 || Number(input.cols) > 20 ? t("Zeilen 1–60, Spalten 1–20.") : null),
      });
      if (!values || !editor) return;
      editor.chain().focus().insertTable({ rows: Number(values.rows), cols: Number(values.cols), withHeaderRow: Boolean(values.header) }).run();
      if (values.caption || values.label) editor.chain().updateAttributes("table", { caption: String(values.caption), label: String(values.label) }).run();
    },
    insertLatexTable: () => editor?.chain().focus().insertContent({ type: "rawLatexBlock", attrs: { rawLatex: LATEX_TABLE_TEMPLATE } }).run(),
    insertImage: async () => {
      const image = await chooseImage();
      if (image) editor?.chain().focus().insertContent({ type: "image", attrs: { ...image, placement: "htbp" } }).run();
    },
    insertSubfigures: () => {
      const item = (letter: string) => ({ latexPath: "", src: null, caption: "", label: `fig:teil-${letter}`, widthPercent: 100, boxPercent: subfigureDefaultPercent(2) });
      editor
        ?.chain()
        .focus()
        .insertContent({ type: "subfigures", attrs: { items: [item("a"), item("b")], caption: "", label: "fig:vergleich", placement: "htbp" } })
        .run();
    },
    insertTikz: (environment) =>
      editor?.chain().focus().insertContent({ type: "tikzBlock", attrs: { code: TIKZ_TEMPLATES[environment], environment } }).run(),
    insertMath: (inline, template) => {
      if (!editor) return;
      const { from, to, empty } = editor.state.selection;
      const selected = empty ? "" : editor.state.doc.textBetween(from, to, " ");
      const type = inline ? "inlineMath" : "mathBlock";
      // Wie in Word: neue Formel leer (bzw. markierter Text) und sofort zum Bearbeiten geöffnet;
      // eine Vorlage („Häufige Formeln“, Strukturen) fügt das Formelfeld beim Öffnen ein.
      setPendingMath(template ?? null);
      editor.chain().focus().insertContent({ type, attrs: inline ? { latex: selected } : { latex: selected, numbered: true } }).run();
      const doc = editor.state.doc;
      let found = -1;
      doc.nodesBetween(Math.max(0, from - 1), Math.min(doc.content.size, from + 4), (node, pos) => {
        if (found < 0 && node.type.name === type && String(node.attrs.latex ?? "") === selected) found = pos;
        return found < 0;
      });
      if (found >= 0) editor.chain().setNodeSelection(found).run();
    },
    insertLink: async () => {
      if (!editor) return;
      const current = String(editor.getAttributes("link").href ?? "");
      const empty = editor.state.selection.empty;
      const values = await dialogs.form({
        title: t("Hyperlink"),
        fields: [
          { kind: "text", name: "url", label: t("Adresse (URL)"), value: current || "https://", required: true },
          ...(empty && !current ? [{ kind: "text" as const, name: "text", label: t("Anzuzeigender Text"), value: "" }] : []),
        ],
        validate: (input) => {
          try {
            const url = new URL(String(input.url));
            return ["http:", "https:", "mailto:"].includes(url.protocol) ? null : t("Nur http-, https- und mailto-Links sind erlaubt.");
          } catch {
            return t("Ungültige URL.");
          }
        },
      });
      if (!values) return;
      const href = new URL(String(values.url)).href;
      if (empty && !current) {
        const text = String(values.text ?? "").trim() || href;
        editor.chain().focus().insertContent({ type: "text", text, marks: [{ type: "link", attrs: { href } }] }).run();
      } else {
        editor.chain().focus().extendMarkRange("link").setLink({ href }).run();
      }
    },
    insertCrossReference: async () => {
      const value = await crossReferenceForm();
      if (value) editor?.chain().focus().insertContent({ type: "crossReference", attrs: value }).run();
    },
    insertFootnote: async () => {
      const values = await footnoteForm();
      if (values) editor?.chain().focus().insertContent({ type: "footnote", attrs: { text: String(values.text) } }).run();
    },
    insertAcronym: async () => {
      const value = await acronymForm();
      if (value) editor?.chain().focus().insertContent({ type: "acronym", attrs: value }).run();
    },
    insertRawLatex: () => editor?.chain().focus().insertContent({ type: "rawLatexBlock", attrs: { rawLatex: "" } }).run(),
    insertComment: () => {
      if (!editor) return;
      // nach dem aktuellen Block einfügen (nicht mitten im Absatz – das würde ihn teilen)
      const { $from } = editor.state.selection;
      const pos = $from.depth > 0 ? $from.after(1) : editor.state.selection.to;
      editor.chain().focus().insertContentAt(pos, { type: "rawLatexBlock", attrs: { rawLatex: newComment(t("Kommentar")) } }).run();
    },
    insertQuantity: async () => {
      const value = await quantityForm();
      if (value) editor?.chain().focus().insertContent({ type: "quantity", attrs: { command: value.command, value: value.value, value2: value.value2, unit: value.unit } }).run();
    },
    insertVerticalSpace: async () => {
      const values = await verticalSpaceForm();
      if (values) editor?.chain().focus().insertContent({ type: "verticalSpace", attrs: { command: String(values.command), size: String(values.size).trim() } }).run();
    },
    insertEnvironment: (kind) => void insertEnvironment(kind),
    insertMaketitle: async () => {
      if (!editor) return;
      if (!settings.metadata.title) {
        const values = await dialogs.form({
          title: t("Titel (\\maketitle)"),
          fields: [
            { kind: "text", name: "title", label: t("Titel"), value: "", required: true, autoFocus: true },
            { kind: "text", name: "author", label: t("Autor/in"), value: settings.metadata.author },
          ],
        });
        if (!values) return;
        updateSettings({ metadata: { ...settings.metadata, title: String(values.title), author: String(values.author) } });
      }
      editor.chain().focus().insertContent({ type: "maketitle" }).run();
    },
    insertAppendix: () => editor?.chain().focus().insertContent({ type: "appendixMarker" }).run(),
    openSketch: () => setSketch({ initial: null, pos: null }),
    insertCodeListing: async () => {
      if (!editor) return;
      const inCode = editor.isActive("codeBlock");
      const current = inCode ? editor.getAttributes("codeBlock") : {};
      const values = await dialogs.form({
        title: t("Code-Listing"),
        fields: [
          {
            kind: "select",
            name: "language",
            label: t("Sprache"),
            value: String(current.language ?? "python"),
            options: [
              { value: "", label: t("(keine)") },
              ...["python", "c", "c++", "java", "matlab", "javascript", "bash", "sql", "html", "xml", "vhdl", "verilog", "r", "tex"].map((value) => ({ value, label: value })),
            ],
          },
          { kind: "text", name: "caption", label: t("Beschriftung (optional)"), value: "" },
          {
            kind: "select",
            name: "environment",
            label: t("Umgebung"),
            value: String(current.environment ?? "lstlisting"),
            options: [
              { value: "lstlisting", label: t("lstlisting (mit Hervorhebung)") },
              { value: "verbatim", label: t("verbatim (einfach)") },
            ],
          },
        ],
      });
      if (!values) return;
      const language = String(values.language) || null;
      const caption = String(values.caption).trim().replace(/[{}]/g, "");
      const environment = String(values.environment) === "verbatim" ? null : "lstlisting";
      const listingOptions = environment ? [language ? `language=${language}` : "", caption ? `caption={${caption}}` : ""].filter(Boolean).join(", ") : null;
      if (inCode) editor.chain().focus().updateAttributes("codeBlock", { language, environment, listingOptions }).run();
      else editor.chain().focus().insertContent({ type: "codeBlock", attrs: { language, environment, listingOptions }, content: [{ type: "text", text: "# Code" }] }).run();
    },
    setTableStyle: (style) => {
      if (!editor) return;
      const attrs = editor.getAttributes("table");
      const spec = typeof attrs.columnSpec === "string" ? attrs.columnSpec : null;
      editor
        .chain()
        .focus()
        .updateAttributes("table", {
          tableStyle: style,
          rowRules: null,
          // Gitter: Spalten von VisuTeX (mit Linien); sonst die übernommene Definition ohne |
          columnSpec: style === "grid" ? null : spec?.replace(/\|/g, "") ?? null,
          tableEnvironment: style === "grid" ? null : attrs.tableEnvironment ?? null,
        })
        .run();
    },
    setListFormat: (label) => {
      if (!editor) return;
      const type = editor.isActive("orderedList") ? "orderedList" : editor.isActive("bulletList") ? "bulletList" : null;
      if (!type) {
        notify(t("Bitte zuerst den Cursor in eine Liste setzen."));
        return;
      }
      const options = String(editor.getAttributes(type).listOptions ?? "");
      const rest = options
        .split(",")
        .map((part) => part.trim())
        .filter((part) => part && !part.startsWith("label"));
      const next = label ? [`label=${label}`, ...rest].join(", ") : rest.join(", ");
      editor.chain().focus().updateAttributes(type, { listOptions: next || null }).run();
    },
    insertSnippet: (snippet: Snippet) => {
      if (!editor) return;
      const chain = editor.chain().focus();
      switch (snippet.kind) {
        case "block":
          chain.insertContent({ type: "rawLatexBlock", attrs: { rawLatex: snippet.latex } }).run();
          break;
        case "math":
          chain.insertContent({ type: "mathBlock", attrs: { latex: snippet.latex, numbered: false } }).run();
          break;
        case "inlineMath":
          chain.insertContent({ type: "inlineMath", attrs: { latex: snippet.latex } }).run();
          break;
        case "tikz":
          chain.insertContent({ type: "tikzBlock", attrs: { code: snippet.latex, environment: /to\[/.test(snippet.latex) ? "circuitikz" : "tikzpicture" } }).run();
          break;
        default:
          chain.insertContent({ type: "rawLatexInline", attrs: { latex: snippet.latex } }).run();
      }
    },
    insertDirectory: (kind) => {
      if (kind === "bibliography" && !settings.bibliography.file) void ensureBibFile();
      editor?.chain().focus().insertContent({ type: "directoryBlock", attrs: { kind } }).run();
    },
    insertCitation: () => setCitationDialog({ initial: null, pos: null }),
    chooseBibliography: () => void chooseBibliography(),
    updateSettings,
    openDocumentSettings: (section) => setSettingsDialog({ section }),
    editTableCaption: async () => {
      if (!editor) return;
      const attrs = editor.getAttributes("table");
      const values = await dialogs.form({
        title: t("Tabellenbeschriftung"),
        fields: [
          { kind: "text", name: "caption", label: t("Beschriftung"), value: String(attrs.caption ?? ""), mono: attrs.captionLatex === true },
          { kind: "checkbox", name: "captionLatex", label: t("Beschriftung enthält LaTeX (Formeln, Befehle)"), value: attrs.captionLatex === true },
          { kind: "text", name: "label", label: t("Label"), value: String(attrs.label ?? ""), placeholder: "tab:messwerte" },
          { kind: "checkbox", name: "captionAbove", label: t("Beschriftung über der Tabelle"), value: attrs.captionAbove === true },
          {
            kind: "select",
            name: "placement",
            label: t("Platzierung"),
            value: String(attrs.placement || "htbp"),
            options: [
              { value: "htbp", label: t("Automatisch (Gleitumgebung)") },
              { value: "H", label: t("Genau hier (H)") },
              { value: "h", label: t("Möglichst hier (h)") },
              { value: "t", label: t("Seitenanfang") },
              { value: "b", label: t("Seitenende") },
            ],
          },
        ],
      });
      if (values)
        editor
          .chain()
          .focus()
          .updateAttributes("table", {
            caption: String(values.caption),
            label: String(values.label).trim(),
            captionLatex: values.captionLatex ? true : null,
            captionAbove: values.captionAbove ? true : null,
            placement: String(values.placement),
          })
          .run();
    },
    editImage: async () => {
      if (!editor) return;
      const attrs = editor.getAttributes("image");
      const values = await dialogs.form({
        title: t("Bild"),
        fields: [
          { kind: "text", name: "caption", label: t("Beschriftung"), value: String(attrs.caption ?? "") },
          { kind: "text", name: "label", label: t("Label"), value: String(attrs.label ?? ""), pattern: labelPattern, placeholder: "fig:aufbau" },
          {
            kind: "select",
            name: "placement",
            label: t("Platzierung"),
            value: String(attrs.placement || "htbp"),
            options: [
              { value: "htbp", label: t("Automatisch (Gleitumgebung)") },
              { value: "H", label: t("Genau hier (H)") },
              { value: "t", label: t("Seitenanfang") },
              { value: "b", label: t("Seitenende") },
              { value: "here", label: t("Ohne Gleitumgebung (zentriert)") },
            ],
          },
          { kind: "number", name: "widthPercent", label: t("Breite"), value: Number(attrs.widthPercent) || 80, min: 5, max: 100, unit: "%" },
          { kind: "text", name: "latexPath", label: t("Datei im Projekt"), value: String(attrs.latexPath ?? ""), mono: true },
        ],
      });
      if (values) {
        editor
          .chain()
          .focus()
          .updateAttributes("image", {
            caption: String(values.caption),
            label: String(values.label),
            placement: String(values.placement),
            widthPercent: Math.min(100, Math.max(5, Number(values.widthPercent))),
            latexPath: String(values.latexPath).replace(/\\/g, "/"),
          })
          .run();
      }
    },
    replaceImage: async () => {
      const image = await chooseImage();
      if (image) editor?.chain().focus().updateAttributes("image", { latexPath: image.latexPath, src: null }).run();
    },
    compile: () => {
      setView((value) => ({ ...value, showPdf: true }));
      void compile();
    },
    openAddons: () => setBackstage("addons"),
    openShortcuts: () => setBackstage("shortcuts"),
    openSlides: showSlides,
  };

  // Nur Entwicklung: Zugang für automatisierte Tests im echten Fenster (WebView2-Remote-Debugging).
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    (window as unknown as { __visutex: unknown }).__visutex = {
      editor,
      state: () => stateRef.current,
      actions,
      setMode,
      setCode: onCodeChange,
      saveTo: saveToRef.current,
      compile: compileRef.current,
      exportTex,
      applyOpened,
    };
  });

  // Tastenkürzel (frei belegbar unter Datei → Tastenkürzel, siehe src/shortcuts.ts)
  const bindings = useMemo(() => resolveBindings(prefs.shortcuts), [prefs.shortcuts]);
  /** Führt einen Befehl aus; `false`, wenn er im aktuellen Zustand nicht anwendbar ist. */
  const runShortcut = (id: string): boolean => {
    const zoom = (delta: number | null) =>
      setView((value) => ({ ...value, zoom: delta === null ? 100 : Math.min(200, Math.max(50, value.zoom + delta)) }));
    switch (id) {
      case "file.new": setBackstage("home"); return true;
      case "file.open": void openDocument(); return true;
      case "file.save": void saveRef.current(); return true;
      case "file.saveAs": void saveAs(); return true;
      case "file.share": setShareOpen(true); return true;
      case "file.exportTex": void exportTex(); return true;
      case "file.exportPdf": void exportPdf(); return true;
      case "file.compile": actions.compile(); return true;
      case "file.options": setBackstage("options"); return true;
      case "file.shortcuts": setBackstage("shortcuts"); return true;
      case "view.toggleCode": void setMode(stateRef.current.mode === "visual" ? "code" : "visual"); return true;
      case "view.togglePdf": setView((value) => ({ ...value, showPdf: !value.showPdf })); return true;
      case "view.toggleOutline": setView((value) => ({ ...value, showOutline: !value.showOutline })); return true;
      case "view.togglePaged": setView((value) => ({ ...value, paged: !value.paged })); return true;
      case "view.toggleRuler": setView((value) => ({ ...value, showRuler: !value.showRuler })); return true;
      case "view.formattingMarks": setView((value) => ({ ...value, showMarks: !value.showMarks })); return true;
      case "view.zoomIn": zoom(10); return true;
      case "view.zoomOut": zoom(-10); return true;
      case "view.zoomReset": zoom(null); return true;
      case "view.slides": actions.openSlides(); return true;
    }
    if (id.startsWith("slides.")) return false; // im Folien-Editor selbst behandelt
    if (stateRef.current.mode !== "visual" || !editor) return false;
    const chain = () => editor.chain().focus();
    switch (id) {
      case "edit.undo": return chain().undo().run();
      case "edit.redo": return chain().redo().run();
      case "edit.find": setSearch({ replace: false }); return true;
      case "edit.replace": setSearch({ replace: true }); return true;
      case "edit.formatPainter": actions.formatPainter(false); return true;
      case "format.bold": return chain().toggleBold().run();
      case "format.italic": return chain().toggleItalic().run();
      case "format.underline": return chain().toggleUnderline().run();
      case "format.strike": return chain().toggleStrike().run();
      case "format.subscript": return chain().toggleSubscript().run();
      case "format.superscript": return chain().toggleSuperscript().run();
      case "format.code": return chain().toggleCode().run();
      case "format.clear": return chain().unsetAllMarks().run();
      case "paragraph.normal": return chain().setParagraph().run();
      case "paragraph.heading1": return chain().toggleHeading({ level: 1 }).run();
      case "paragraph.heading2": return chain().toggleHeading({ level: 2 }).run();
      case "paragraph.heading3": return chain().toggleHeading({ level: 3 }).run();
      case "paragraph.alignLeft": return chain().setTextAlign("left").run();
      case "paragraph.alignCenter": return chain().setTextAlign("center").run();
      case "paragraph.alignRight": return chain().setTextAlign("right").run();
      case "paragraph.alignJustify": return chain().setTextAlign("justify").run();
      case "paragraph.bulletList": return chain().toggleBulletList().run();
      case "paragraph.orderedList": return chain().toggleOrderedList().run();
      case "paragraph.quote": return chain().toggleBlockquote().run();
      case "paragraph.codeBlock": return chain().toggleCodeBlock().run();
      case "insert.pageBreak": actions.insertPageBreak(); return true;
      case "insert.inlineMath": actions.insertMath(true); return true;
      case "insert.blockMath": actions.insertMath(false); return true;
      case "insert.link": actions.insertLink(); return true;
      case "insert.footnote": actions.insertFootnote(); return true;
      case "insert.citation": actions.insertCitation(); return true;
      case "insert.crossReference": actions.insertCrossReference(); return true;
      case "insert.table": actions.insertTable(); return true;
      case "insert.image": actions.insertImage(); return true;
      case "insert.sketch": actions.openSketch(); return true;
      case "insert.rawLatex": actions.insertRawLatex(); return true;
      case "insert.quantity": actions.insertQuantity(); return true;
      case "insert.codeListing": actions.insertCodeListing(); return true;
    }
    return false;
  };
  const runShortcutRef = useRef(runShortcut);
  runShortcutRef.current = runShortcut;
  useEffect(() => {
    const scopes = new Map(SHORTCUT_COMMANDS.map((command) => [command.id, command.scope]));
    // Capture-Phase: vor ProseMirror/Monaco, damit umbelegte Standardkürzel nicht doppelt wirken.
    const listener = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing) return;
      // Offene Dialoge, Datei-Bereich, Skizze und Folien-Editor behandeln Tasten selbst.
      if (document.querySelector('[role="dialog"]')) return;
      const combo = comboFromEvent(event);
      if (!combo) return;
      const target = event.target instanceof Element ? event.target : null;
      const inMainEditor = Boolean(target && editor && editor.view.dom.contains(target));
      // Formelfelder (MathLive) haben eigene Tasten (Strg+Z, Pfeile …)
      const inField = Boolean(target?.closest("input, textarea, select, .monaco-editor, math-field"));
      const editorContext = !inField && (inMainEditor || !target || target === document.body || Boolean(target.closest(".ribbon, .statusbar")));
      const id = bindings.get(combo);
      if (id && (scopes.get(id) === "global" || editorContext)) {
        if (runShortcutRef.current(id)) {
          event.preventDefault();
          event.stopPropagation();
        }
        return;
      }
      // Nicht (mehr) belegte Standardkürzel des Editors abfangen.
      if (!id && inMainEditor && !inField && EDITOR_NATIVE_COMBOS.includes(combo)) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    window.addEventListener("keydown", listener, true);
    return () => window.removeEventListener("keydown", listener, true);
  }, [bindings, editor]);

  // ---------------------------------------------------------------- Layout

  const paperRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = paperRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setPaperHeight(element.offsetHeight));
    observer.observe(element);
    return () => observer.disconnect();
  }, [mode, backstage]);

  const zoom = view.zoom / 100;
  // Strg + Mausrad (bzw. Touchpad-Zoom) im Dokument: in 10-%-Schritten wie im Menüband
  const scrollerRef = useRef<HTMLDivElement>(null);
  useCtrlWheel(scrollerRef, (direction) =>
    setView((value) => ({ ...value, zoom: Math.min(200, Math.max(50, value.zoom + direction * 10)) })),
  );
  const paged = geometry.enabled;
  const pageWidthPx = dimensions.width * MM;
  const pageCount = paged ? pages.length : 1;
  const paperStyle = {
    "--page-width": `${pageWidthPx}px`,
    "--page-height": `${geometry.pageHeight}px`,
    "--page-gap": `${PAGE_GAP}px`,
    "--margin-top": `${settings.margins.top * MM}px`,
    "--margin-right": `${(settings.margins.right + (settings.twoside ? 0 : 0)) * MM}px`,
    "--margin-bottom": `${settings.margins.bottom * MM}px`,
    "--margin-left": `${(settings.margins.left + settings.bindingOffset) * MM}px`,
    "--doc-font": settings.defaultFontFamily === "Latin Modern Roman" ? "\"Latin Modern Roman\", \"Latin Modern\", serif" : `"${settings.defaultFontFamily}", "Latin Modern Roman", serif`,
    "--doc-font-size": `${settings.defaultFontSize}pt`,
    "--doc-line-height": String(1.2 * (settings.lineSpacing === 1.5 ? 1.25 : settings.lineSpacing === 2 ? 1.667 : settings.lineSpacing)),
    "--paragraph-indent": settings.paragraphStyle === "indent" ? "1.5em" : "0",
    "--paragraph-skip": settings.paragraphStyle === "skip" ? "0.6em" : "0",
    "--page-bg": settings.pageDisplayColor || "#ffffff",
    minHeight: paged ? `${pageCount * geometry.pageHeight + (pageCount - 1) * PAGE_GAP}px` : undefined,
  } as CSSProperties;

  const currentPage = useMemo(() => {
    if (!editor || !paged) return 1;
    try {
      const coords = editor.view.coordsAtPos(editor.state.selection.head);
      const root = editor.view.dom.getBoundingClientRect();
      const y = (coords.top - root.top) / zoom;
      return Math.min(pages.length, Math.max(1, Math.floor(y / (geometry.pageHeight + PAGE_GAP)) + 1));
    } catch {
      return 1;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor, paged, pages.length, analysis, zoom, geometry.pageHeight]);

  const numberingClass = `numbering-depth-${settings.numberingDepth} class-${settings.documentClass}${settings.columns > 1 ? ` columns-${settings.columns}` : ""}`;

  return (
    <div className={`app${theme === "dark" ? " theme-dark" : ""}`}>
      <header
        className="titlebar"
        onContextMenu={(event) => {
          // Rechtsklick auf die Titelleiste (nicht in Eingabefeldern): Teilen und Dateiaktionen
          if ((event.target as HTMLElement).closest("input, textarea, select")) return;
          event.preventDefault();
          setTitleMenu({ x: event.clientX, y: event.clientY });
        }}
      >
        <div className="titlebar-actions">
          <QuickAccessBar
            items={prefs.quickAccess}
            shortcuts={prefs.shortcuts}
            language={prefs.language}
            isDisabled={(id) => SHORTCUT_COMMANDS.find((command) => command.id === id)?.scope === "editor" && mode !== "visual"}
            onRun={(id) => void runShortcut(id)}
            onChange={(quickAccess) => setPrefs((current) => ({ ...current, quickAccess }))}
            onMoreOptions={() => setBackstage("options")}
          />
        </div>
        <div className="titlebar-name">
          <strong>{docState?.name ?? "VisuTeX"}</strong>
          {dirty && <span className="dirty-dot" title={t("Ungespeicherte Änderungen")}>●</span>}
          {customPreamble && <span className="badge" title={t("Dieses Dokument verwendet eine eigene Präambel.")}>{t("eigene Präambel")}</span>}
        </div>
        <div className="titlebar-actions right">
          <div className="segmented" role="group" aria-label={t("Ansicht")}>
            <button type="button" className={mode === "visual" ? "active" : ""} onClick={() => void setMode("visual")}>
              <Eye size={15} /> {t("Visuell")}
            </button>
            <button type="button" className={mode === "code" ? "active" : ""} onClick={() => void setMode("code")}>
              <Braces size={15} /> LaTeX
            </button>
            <button type="button" title={t("Folien-Editor: Präsentationen frei gestalten (Export als LaTeX-Beamer)")} onClick={showSlides}>
              <Presentation size={15} /> {t("Folien")}
            </button>
          </div>
          <button type="button" className={`icon-button${view.showPdf ? " active" : ""}`} title={t("PDF-Vorschau ein/aus")} onClick={() => setView((value) => ({ ...value, showPdf: !value.showPdf }))}>
            <PanelRight size={17} />
          </button>
          <button type="button" className="primary small" title={t("Kompilieren (F5)")} disabled={view.compiling} onClick={() => actions.compile()}>
            <Play size={14} /> {view.compiling ? t("Kompiliere …") : t("PDF")}
          </button>
        </div>
      </header>

      <Ribbon
        editor={editor}
        mode={mode}
        actions={actions}
        settings={settings}
        view={view}
        onViewChange={(patch) => setView((value) => ({ ...value, ...patch }))}
        addons={addons}
        availableFonts={appInfo?.availableFonts ?? ["Latin Modern Roman"]}
        formatPainterActive={formatPainterActive}
      />

      <div className={`workspace${view.showPdf ? " with-pdf" : ""}${view.showOutline ? " with-outline" : ""}`}>
        {view.showOutline && (
          <aside className="outline-pane" aria-label={t("Navigation")}>
            <div className="pane-title">{t("Navigation")}</div>
            <ul>
              {(analysis?.outline ?? []).map((entry, index) => (
                <li key={index} className={`level-${entry.level}`}>
                  <button
                    type="button"
                    onClick={() => {
                      if (!editor || mode !== "visual") return;
                      let pos = 0;
                      for (let child = 0; child < entry.blockIndex; child += 1) pos += editor.state.doc.child(child).nodeSize;
                      const dom = editor.view.nodeDOM(pos);
                      if (dom instanceof HTMLElement) dom.scrollIntoView({ block: "start", behavior: "smooth" });
                      editor.commands.setTextSelection(pos + 1);
                    }}
                  >
                    {entry.title || t("(ohne Titel)")}
                  </button>
                </li>
              ))}
              {(analysis?.outline.length ?? 0) === 0 && <li className="muted">{t("Keine Überschriften.")}</li>}
            </ul>
          </aside>
        )}

        <main className="editor-pane">
          {mode === "visual" && search && editor && <SearchPanel editor={editor} replace={search.replace} onClose={() => setSearch(null)} />}
          {sketch && (
            <SketchPad
              initial={sketch.initial}
              editing={sketch.pos !== null}
              allowOnline={prefs.allowOnline}
              onClose={() => setSketch(null)}
              onInsert={(code, environment) => {
                if (!editor) return;
                const pos = sketch.pos;
                const node = pos !== null ? editor.state.doc.nodeAt(pos) : null;
                if (pos !== null && node?.type.name === "tikzBlock") {
                  editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, code, environment, preview: "", previewCode: "" }));
                } else {
                  editor.chain().focus().insertContent({ type: "tikzBlock", attrs: { code, environment, options: "" } }).run();
                }
                setSketch(null);
                notify(t("Skizze eingefügt – Vorschau wird kompiliert."));
              }}
            />
          )}
          <div className="document-scroller" ref={scrollerRef} style={{ display: mode === "visual" ? undefined : "none" }}>
            {view.showRuler && (
              <Ruler
                pageWidthMm={dimensions.width}
                marginLeftMm={settings.margins.left + settings.bindingOffset}
                marginRightMm={settings.margins.right}
                bindingOffsetMm={settings.bindingOffset}
                zoom={zoom}
                onChange={({ left, right }) => updateSettings({ margins: { ...settings.margins, left, right } })}
              />
            )}
            <div className="paper-outer" style={{ width: pageWidthPx * zoom, height: paperHeight ? paperHeight * zoom : undefined, ["--zoom" as string]: zoom }}>
              <div ref={paperRef} data-doc-lang={settings.language} className={`paper${paged ? " paged" : " continuous"} ${numberingClass}`} style={{ ...paperStyle, transform: zoom === 1 ? undefined : `scale(${zoom})` }}>
                {paged && (
                  <PageCanvas
                    pages={pages}
                    pageHeight={geometry.pageHeight}
                    gap={PAGE_GAP}
                    marginTop={settings.margins.top * MM}
                    marginBottom={settings.margins.bottom * MM}
                    marginLeft={(settings.margins.left + settings.bindingOffset) * MM}
                    marginRight={settings.margins.right * MM}
                    settings={settings}
                  />
                )}
                <EditorContent editor={editor} className="editor-host" />
              </div>
            </div>
          </div>
          {mode === "code" && (
            <Suspense fallback={<div className="loading">{t("Code-Editor wird geladen …")}</div>}>
              <CodeView
                value={code}
                onChange={onCodeChange}
                theme={theme}
                messages={codeMessages}
                revealLine={revealLine}
                onCompile={() => actions.compile()}
                onSave={() => void saveRef.current()}
              />
            </Suspense>
          )}
        </main>

        {view.showPdf && (
          <aside className="pdf-pane" aria-label={t("PDF-Vorschau")}>
            <PdfPreview
              status={compileStatus}
              onCompile={() => actions.compile()}
              onSavePdf={() => void savePdf()}
              onMessageClick={(message) => {
                if (message.line === null) return;
                void setMode("code").then(() => setRevealLine({ line: message.line as number, token: Date.now() }));
              }}
            />
          </aside>
        )}
      </div>

      <footer className="statusbar">
        <span>
          {mode === "visual" ? (paged ? `${t("Seite")} ${currentPage} ${t("von")} ${pageCount}` : t("Endlosansicht")) : t("LaTeX-Code")}
        </span>
        <span>{analysis ? `${analysis.words.toLocaleString()} ${t("Wörter")}` : ""}</span>
        <span title={t("Dokumentsprache")}>{documentLanguage(settings.language).name}</span>
        {analysis?.duplicateLabels.length ? <span className="status-error">{t("Doppelte Labels:")} {analysis.duplicateLabels.join(", ")}</span> : null}
        <span className="spacer" />
        <span>{appInfo?.bundle.kind === "online" ? t("TeX: Online-Bundle") : appInfo ? t("TeX: offline") : ""}</span>
        <label className="zoom-control">
          <input type="range" min={50} max={200} step={10} value={view.zoom} onChange={(event) => {
            // Wert sofort lesen – im verzögerten Updater ist event.currentTarget bereits null (Absturz)
            const zoom = Number(event.currentTarget.value);
            setView((value) => ({ ...value, zoom }));
          }} aria-label={t("Zoom")} />
          {view.zoom}%
        </label>
      </footer>

      <div className="notices" aria-live="polite">
        {notices.map((notice) => (
          <div key={notice.id} className={`notice ${notice.kind}`} onClick={() => setNotices((current) => current.filter((entry) => entry.id !== notice.id))}>
            {notice.message}
          </div>
        ))}
      </div>

      {settingsDialog && (
        <DocumentSettingsDialog
          settings={settings}
          customPreamble={customPreamble}
          availableFonts={appInfo?.availableFonts ?? ["Latin Modern Roman"]}
          initialSection={settingsDialog.section}
          onChooseBibliography={() => void chooseBibliography()}
          onApply={(next, preamble) => {
            setSettings(next);
            setCustomPreamble(preamble);
          }}
          onClose={() => setSettingsDialog(null)}
        />
      )}

      {citationDialog && (
        <CitationDialog
          projectRoot={projectRoot}
          bibFile={settings.bibliography.file}
          initial={citationDialog.initial}
          onEnsureBibFile={ensureBibFile}
          onDone={(value) => {
            const pos = citationDialog.pos;
            setCitationDialog(null);
            if (value) insertCitationValue(value, pos);
          }}
        />
      )}

      {setupView && setupStatus && (
        <SetupScreen
          status={{ ...setupStatus, uninstallRequested: setupView === "uninstall" }}
          checkUpdates={prefs.checkUpdates}
          onCheckUpdatesChange={(checkUpdates) => setPrefs((current) => ({ ...current, checkUpdates }))}
          onClose={() => {
            setSetupView(null);
            void refreshSetup().catch(() => undefined);
          }}
        />
      )}
      {titleMenu && (
        <TitleContextMenu
          x={titleMenu.x}
          y={titleMenu.y}
          hasFile={Boolean(docState?.path && !docState.temporary)}
          onShare={() => setShareOpen(true)}
          onReveal={() => docState?.path && void revealItemInDir(docState.path)}
          onCopyPath={() => {
            if (!docState?.path) return;
            void navigator.clipboard.writeText(docState.path).then(() => notify(t("Dateipfad kopiert.")));
          }}
          onClose={() => setTitleMenu(null)}
        />
      )}
      {shareOpen && (
        <ShareDialog
          target={{
            path: docState?.path && !docState.temporary ? docState.path : null,
            name: docState?.name ?? "VisuTeX",
            savedAt: docState?.stamp?.modifiedMs ?? null,
            dirty,
            latexProject: docState?.kind === "tex",
          }}
          locale={languageInfo(prefs.language).locale}
          onSave={() => saveRef.current()}
          onShared={(message) => notify(message)}
          onClose={() => setShareOpen(false)}
        />
      )}
      {updateInfo && (
        <UpdateDialog
          info={updateInfo}
          onSkip={() => {
            setPrefs((current) => ({ ...current, skipUpdate: updateInfo.latest }));
            setUpdateInfo(null);
          }}
          onClose={() => setUpdateInfo(null)}
        />
      )}

      {systemCheckOpen && (
        <SystemCheckDialog
          check={systemCheck}
          busy={systemCheckBusy}
          onRecheck={() => void runSystemCheck(false)}
          onChooseBundle={() => void chooseBundle()}
          onChoosePackageDir={() => void choosePackageDir()}
          onEnableOnline={() => void enableOnline()}
          onClose={() => setSystemCheckOpen(false)}
        />
      )}

      {slidesOpen && slidesSession && (
        <Suspense fallback={<div className="slide-editor loading">{t("Folien-Editor wird geladen …")}</div>}>
          <SlideEditor
            session={slidesSession}
            onSessionChange={(next) => {
              if (next.path && next.path !== slidesRef.current?.path) rememberRecent(next.path);
              setSlidesSession(next);
            }}
            onClose={() => setSlidesOpen(false)}
            onFileMenu={() => setBackstage("home")}
            onOpenFile={() => void openDocument()}
            actionsRef={slideActions}
            shortcuts={prefs.shortcuts}
            allowOnline={prefs.allowOnline}
            notify={notify}
          />
        </Suspense>
      )}
      {backstage && (
        <Backstage
          key={backstage}
          initialView={backstage}
          mode={slidesOpen && slidesSession ? "slides" : "document"}
          slides={
            slidesOpen && slidesSession
              ? {
                  deck: slidesSession.deck,
                  path: slidesSession.path,
                  onMeta: (patch) => slideActions.current?.updateMeta(patch),
                  onExportPdf: () => {
                    setBackstage(null);
                    void slideActions.current?.exportPdf();
                  },
                  onExportLatex: () => {
                    setBackstage(null);
                    void slideActions.current?.exportLatex();
                  },
                }
              : null
          }
          canClose={Boolean(docState) || (slidesOpen && Boolean(slidesSession))}
          appInfo={appInfo}
          setup={setupStatus}
          onCheckUpdates={checkForUpdates}
          onSetupChanged={() => void refreshSetup().catch(() => undefined)}
          onShowSetup={(kind) => setSetupView(kind)}
          prefs={prefs}
          recent={recent}
          recovery={recovery ? { name: recovery.name, savedAt: recovery.savedAt } : null}
          onPrefsChange={setPrefs}
          onNew={(template) => void newDocument(template)}
          onNewPresentation={(layout) => void newPresentation(layout)}
          onOpenPresentation={slidesSession && !slidesOpen ? showSlides : null}
          onShowDocument={
            slidesOpen && docState
              ? () => {
                  setSlidesOpen(false);
                  setBackstage(null);
                }
              : null
          }
          onOpen={(path) => void openDocument(path)}
          onRestore={() => void restoreRecovery()}
          onDiscardRecovery={() => {
            localStorage.removeItem("visutex-recovery");
            setRecovery(null);
          }}
          onSave={() => {
            const saving = slidesOpen && slideActions.current ? slideActions.current.save(false) : saveRef.current();
            void saving.then((ok) => ok && setBackstage(null));
          }}
          onSaveAs={() => {
            const saving = slidesOpen && slideActions.current ? slideActions.current.save(true) : saveAs();
            void saving.then((ok) => ok && setBackstage(null));
          }}
          onExportTex={() => void exportTex()}
          onExportPdf={() => void exportPdf()}
          onAddonsChanged={reloadAddons}
          onChoosePackageDir={() => void choosePackageDir()}
          onChooseBundle={() => void chooseBundle()}
          onSystemCheck={() => {
            setSystemCheckOpen(true);
            void runSystemCheck(false);
          }}
          onClose={() => setBackstage(null)}
        />
      )}
    </div>
  );
}

