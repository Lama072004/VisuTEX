/** Datei-Bereich (wie „Datei“ in Word): Neu/Vorlagen, Öffnen, Speichern, Export, Add-ons, Optionen, Info.
 *  Gemeinsam für Dokument und Präsentation: Speichern/Exportieren wirken auf den aktiven Bereich, Neu/Öffnen wechseln
 *  bei Bedarf zwischen Dokument und Folien. */
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  ArrowLeft,
  FileDown,
  FilePlus,
  FileText,
  FolderOpen,
  LayoutTemplate,
  GraduationCap,
  History,
  Info,
  Keyboard,
  Presentation,
  Puzzle,
  Save,
  Settings,
  SlidersHorizontal,
} from "lucide-react";
import { api } from "../api";
import type { AppInfo, TemplateInfo } from "../api";
import { useT } from "../i18n";
import type { UiLanguage } from "../i18n";
import { UI_LANGUAGES } from "../i18n";
import { AddonManager } from "./AddonManager";
import { ShortcutsPanel } from "../shortcuts/ShortcutsPanel";
import type { ShortcutOverrides } from "../shortcuts/shortcuts";
import type { SlideDeck } from "../slides/model";
import { DOCUMENT_LANGUAGES } from "../latex/languages";

export type AppPrefs = {
  language: UiLanguage;
  theme: "system" | "light" | "dark";
  accent: string;
  autosave: boolean;
  allowOnline: boolean;
  autoCompile: boolean;
  /** Speicherort für online nachgeladene TeX-Pakete (leer = Standard im App-Datenordner). */
  packageCacheDir: string;
  /** Eigene TeX-Bundle-Datei statt der mitgelieferten (leer = mitgeliefert). */
  bundlePath: string;
  /** Abweichungen von der Standardbelegung der Tastenkürzel. */
  shortcuts: ShortcutOverrides;
};

export type RecentFile = { path: string; name: string; openedAt: number };

export type BackstageView = "home" | "open" | "export" | "properties" | "addons" | "shortcuts" | "options" | "info";

type DeckMeta = Partial<Pick<SlideDeck, "title" | "subtitle" | "author" | "date" | "language">>;

/** Angaben zur offenen Präsentation, wenn der Datei-Bereich aus dem Folien-Editor geöffnet wurde. */
export type SlidesFileContext = {
  deck: SlideDeck;
  path: string | null;
  onMeta: (patch: DeckMeta) => void;
  onExportPdf: () => void;
  onExportLatex: () => void;
};

type Props = {
  initialView: BackstageView;
  /** Aktiver Bereich: Speichern/Exportieren wirken darauf. */
  mode: "document" | "slides";
  /** Folien-Bereich (nur bei mode = "slides") */
  slides: SlidesFileContext | null;
  /** Zurück möglich (Dokument offen bzw. Folien-Editor aktiv) */
  canClose: boolean;
  appInfo: AppInfo | null;
  prefs: AppPrefs;
  recent: RecentFile[];
  recovery: { name: string; savedAt: number } | null;
  onPrefsChange: (prefs: AppPrefs) => void;
  onNew: (template: TemplateInfo | null) => void;
  /** Neue Präsentation im Folien-Editor */
  onNewPresentation: (layout: "title" | "blank") => void;
  /** Zur offenen Präsentation wechseln (null: keine offen bzw. schon aktiv) */
  onOpenPresentation: (() => void) | null;
  /** Zum offenen Dokument wechseln (null: keines offen bzw. schon aktiv) */
  onShowDocument: (() => void) | null;
  onOpen: (path?: string) => void;
  onRestore: () => void;
  onDiscardRecovery: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onExportTex: () => void;
  onExportPdf: () => void;
  onAddonsChanged: () => void;
  onChoosePackageDir: () => void;
  onChooseBundle: () => void;
  onSystemCheck: () => void;
  onClose: () => void;
};

function NavButton({ active, onClick, icon, label }: { active?: boolean; onClick: () => void; icon: ReactNode; label: string }) {
  return (
    <button type="button" className={`backstage-nav-button${active ? " active" : ""}`} onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

export function Backstage(props: Props) {
  const t = useT();
  const [view, setView] = useState<BackstageView>(props.initialView);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  useEffect(() => {
    void api.templatesList().then(setTemplates).catch(() => setTemplates([]));
  }, []);
  const { prefs, onPrefsChange, slides } = props;
  const setPref = (patch: Partial<AppPrefs>) => onPrefsChange({ ...prefs, ...patch });
  const isSlides = props.mode === "slides" && slides !== null;
  const closeRef = useRef(props.onClose);
  closeRef.current = props.onClose;
  useEffect(() => {
    if (!props.canClose) return;
    const listener = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      // offene Dialoge (z. B. Rückfragen) zuerst
      if (document.querySelector('.modal-backdrop, [aria-modal="true"]')) return;
      event.preventDefault();
      closeRef.current();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [props.canClose]);

  const presentationCards = (
    <>
      <button type="button" className="template-card" onClick={() => props.onNewPresentation("title")}>
        <div className="template-thumb thumb-slides">
          <Presentation size={34} />
        </div>
        <strong>{t("Neue Präsentation")}</strong>
        <small>{t("Mit Titelfolie")} – {t("Export als LaTeX-Beamer bzw. PDF")}</small>
      </button>
      <button type="button" className="template-card" onClick={() => props.onNewPresentation("blank")}>
        <div className="template-thumb thumb-slides">
          <LayoutTemplate size={34} />
        </div>
        <strong>{t("Leere Präsentation")}</strong>
        <small>{t("Eine leere Folie zum freien Gestalten")}</small>
      </button>
      {props.onOpenPresentation && (
        <button type="button" className="template-card" onClick={props.onOpenPresentation}>
          <div className="template-thumb thumb-slides">
            <Presentation size={34} />
          </div>
          <strong>{t("Zur offenen Präsentation")}</strong>
          <small>{t("Folien-Editor mit der zuletzt bearbeiteten Präsentation öffnen")}</small>
        </button>
      )}
    </>
  );
  const documentCards = (
    <>
      {templates.map((template) => (
        <button key={`${template.source}-${template.id}`} type="button" className="template-card" onClick={() => props.onNew(template)}>
          <div className={`template-thumb thumb-${template.id}`}>
            {template.id === "abschlussarbeit" || template.id === "projektbericht-englisch" ? <GraduationCap size={34} /> : <FileText size={34} />}
          </div>
          <strong>{t(template.name)}</strong>
          <small>{t(template.description)}</small>
        </button>
      ))}
      {props.onShowDocument && (
        <button type="button" className="template-card" onClick={props.onShowDocument}>
          <div className="template-thumb">
            <FileText size={34} />
          </div>
          <strong>{t("Zum offenen Dokument")}</strong>
          <small>{t("Zurück zum Dokument – die Präsentation bleibt geöffnet")}</small>
        </button>
      )}
    </>
  );

  return (
    <div className="backstage" role="dialog" aria-label={t("Datei")}>
      <nav className="backstage-nav">
        <button type="button" className="backstage-back" onClick={props.onClose} disabled={!props.canClose} aria-label={isSlides ? t("Zurück zu den Folien") : t("Zurück zum Dokument")}>
          <ArrowLeft size={20} />
        </button>
        <NavButton active={view === "home"} onClick={() => setView("home")} icon={<FilePlus size={18} />} label={t("Neu")} />
        <NavButton active={view === "open"} onClick={() => setView("open")} icon={<FolderOpen size={18} />} label={t("Öffnen")} />
        <NavButton onClick={props.onSave} icon={<Save size={18} />} label={t("Speichern")} />
        <NavButton onClick={props.onSaveAs} icon={<Save size={18} />} label={t("Speichern unter")} />
        <NavButton active={view === "export"} onClick={() => setView("export")} icon={<FileDown size={18} />} label={t("Exportieren")} />
        {isSlides && (
          <NavButton active={view === "properties"} onClick={() => setView("properties")} icon={<SlidersHorizontal size={18} />} label={t("Eigenschaften")} />
        )}
        <div className="backstage-nav-spacer" />
        <NavButton active={view === "addons"} onClick={() => setView("addons")} icon={<Puzzle size={18} />} label={t("Add-ons")} />
        <NavButton active={view === "shortcuts"} onClick={() => setView("shortcuts")} icon={<Keyboard size={18} />} label={t("Tastenkürzel")} />
        <NavButton active={view === "options"} onClick={() => setView("options")} icon={<Settings size={18} />} label={t("Optionen")} />
        <NavButton active={view === "info"} onClick={() => setView("info")} icon={<Info size={18} />} label={t("Info")} />
      </nav>
      <main className="backstage-main">
        {view === "home" && (
          <>
            <h1>{t("Neu")}</h1>
            {props.recovery && (
              <div className="recovery-banner">
                <History size={20} />
                <div>
                  <strong>{t("Nicht gespeicherte Änderungen gefunden")}</strong>
                  <p>{props.recovery.name} · {new Date(props.recovery.savedAt).toLocaleString()}</p>
                </div>
                <button type="button" className="primary" onClick={props.onRestore}>{t("Wiederherstellen")}</button>
                <button type="button" onClick={props.onDiscardRecovery}>{t("Verwerfen")}</button>
              </div>
            )}
            {isSlides ? (
              <>
                <h2>{t("Präsentation")}</h2>
                <div className="template-grid">{presentationCards}</div>
                <h2>{t("Dokument")}</h2>
                <div className="template-grid">{documentCards}</div>
              </>
            ) : (
              <>
                <h2>{t("Dokument")}</h2>
                <div className="template-grid">{documentCards}</div>
                <h2>{t("Präsentation")}</h2>
                <div className="template-grid">{presentationCards}</div>
              </>
            )}
            {props.recent.length > 0 && (
              <>
                <h2>{t("Zuletzt verwendet")}</h2>
                <RecentList recent={props.recent} onOpen={props.onOpen} />
              </>
            )}
          </>
        )}
        {view === "open" && (
          <>
            <h1>{t("Öffnen")}</h1>
            <button type="button" className="primary big" onClick={() => props.onOpen()}>
              <FolderOpen size={18} /> {t("Datei auswählen …")}
            </button>
            <p className="muted">{t("VisuTeX-Projekte (.visutex, ältere .json), LaTeX-Dateien (.tex) aus beliebigen Quellen und Präsentationen (.vtxslides).")}</p>
            <h2>{t("Zuletzt verwendet")}</h2>
            <RecentList recent={props.recent} onOpen={props.onOpen} />
          </>
        )}
        {view === "export" && (
          <>
            <h1>{t("Exportieren")}</h1>
            {isSlides ? (
              <div className="export-options">
                <button type="button" className="export-card" onClick={slides.onExportPdf}>
                  <FileDown size={28} />
                  <strong>{t("PDF")}</strong>
                  <small>{t("Kompiliert die Präsentation mit der eingebauten Engine und speichert das PDF.")}</small>
                </button>
                <button type="button" className="export-card" onClick={slides.onExportLatex}>
                  <FileText size={28} />
                  <strong>{t("LaTeX-Beamer (.tex)")}</strong>
                  <small>{t("Vollständiges Beamer-Dokument inkl. Bilder – kompiliert mit pdfLaTeX, XeLaTeX und LuaLaTeX.")}</small>
                </button>
              </div>
            ) : (
            <div className="export-options">
              <button type="button" className="export-card" onClick={props.onExportTex}>
                <FileText size={28} />
                <strong>{t("LaTeX-Dokument (.tex)")}</strong>
                <small>{t("Vollständiges Dokument inkl. Bilder, Literatur und Add-on-Dateien – kompiliert mit pdfLaTeX, XeLaTeX und LuaLaTeX (TeX Live, MiKTeX, Overleaf).")}</small>
              </button>
              <button type="button" className="export-card" onClick={props.onExportPdf}>
                <FileDown size={28} />
                <strong>{t("PDF")}</strong>
                <small>{t("Kompiliert das Dokument mit der eingebauten Engine und speichert das PDF.")}</small>
              </button>
            </div>
            )}
          </>
        )}
        {view === "properties" && isSlides && (
          <>
            <h1>{t("Eigenschaften")}</h1>
            <div className="form-grid two-columns options-grid">
              {(["title", "subtitle", "author", "date"] as const).map((key) => (
                <label key={key} className="form-field">
                  <span>{t({ title: "Titel", subtitle: "Untertitel", author: "Autor/in", date: "Datum" }[key])}</span>
                  <input value={slides.deck[key]} onChange={(event) => slides.onMeta({ [key]: event.currentTarget.value })} />
                </label>
              ))}
              <label className="form-field">
                <span>{t("Sprache")}</span>
                <select value={slides.deck.language} onChange={(event) => slides.onMeta({ language: event.currentTarget.value })}>
                  {DOCUMENT_LANGUAGES.map((language) => (
                    <option key={language.id} value={language.id}>{language.name}</option>
                  ))}
                </select>
              </label>
            </div>
            <dl className="info-list">
              <dt>{t("Datei")}</dt>
              <dd>{slides.path ?? t("noch nicht gespeichert")}</dd>
              <dt>{t("Folien")}</dt>
              <dd>{slides.deck.slides.length}</dd>
              <dt>{t("Foliengröße")}</dt>
              <dd>{slides.deck.aspect}</dd>
            </dl>
          </>
        )}
        {view === "addons" && (
          <>
            <h1>{t("Add-ons")}</h1>
            <AddonManager onChanged={props.onAddonsChanged} />
          </>
        )}
        {view === "shortcuts" && (
          <>
            <h1>{t("Tastenkürzel")}</h1>
            <ShortcutsPanel overrides={prefs.shortcuts} onChange={(shortcuts) => setPref({ shortcuts })} />
          </>
        )}
        {view === "options" && (
          <>
            <h1>{t("Optionen")}</h1>
            <div className="form-grid two-columns options-grid">
              <label className="form-field">
                <span>{t("Sprache der Oberfläche")}</span>
                <select value={prefs.language} onChange={(event) => setPref({ language: event.currentTarget.value as UiLanguage })}>
                  {UI_LANGUAGES.map((language) => (
                    <option key={language.code} value={language.code}>{language.name}</option>
                  ))}
                </select>
              </label>
              <label className="form-field">
                <span>{t("Erscheinungsbild")}</span>
                <select value={prefs.theme} onChange={(event) => setPref({ theme: event.currentTarget.value as AppPrefs["theme"] })}>
                  <option value="system">{t("Wie System")}</option>
                  <option value="light">{t("Hell")}</option>
                  <option value="dark">{t("Dunkel")}</option>
                </select>
              </label>
              <label className="form-field">
                <span>{t("Akzentfarbe")}</span>
                <input type="color" value={prefs.accent} onChange={(event) => setPref({ accent: event.currentTarget.value })} />
              </label>
              <span />
              <label className="checkbox form-checkbox span-2">
                <input type="checkbox" checked={prefs.autosave} onChange={(event) => setPref({ autosave: event.currentTarget.checked })} />
                {t("Wiederherstellungsdaten automatisch sichern")}
              </label>
              <label className="checkbox form-checkbox span-2">
                <input type="checkbox" checked={prefs.autoCompile} onChange={(event) => setPref({ autoCompile: event.currentTarget.checked })} />
                {t("PDF-Vorschau automatisch aktualisieren (bei geöffneter Vorschau)")}
              </label>
              <label className="checkbox form-checkbox span-2">
                <input type="checkbox" checked={prefs.allowOnline} onChange={(event) => setPref({ allowOnline: event.currentTarget.checked })} />
                {t("Fehlende TeX-Pakete online nachladen (sonst nur mitgeliefertes Bundle, vollständig offline)")}
              </label>
              <div className="form-field span-2">
                <span>{t("Speicherort für nachgeladene TeX-Pakete")}</span>
                <span className="path-field">
                  <code>{prefs.packageCacheDir || t("Standard (App-Datenordner)")}</code>
                  <button type="button" className="small" onClick={props.onChoosePackageDir}>{t("Ändern …")}</button>
                  {prefs.packageCacheDir && (
                    <button type="button" className="small" onClick={() => setPref({ packageCacheDir: "" })}>{t("Standard")}</button>
                  )}
                </span>
              </div>
              <div className="form-field span-2">
                <span>{t("TeX-Paketbundle")}</span>
                <span className="path-field">
                  <code>{prefs.bundlePath || t("Mitgeliefert")}</code>
                  <button type="button" className="small" onClick={props.onChooseBundle}>{t("Andere Datei …")}</button>
                  {prefs.bundlePath && (
                    <button type="button" className="small" onClick={() => setPref({ bundlePath: "" })}>{t("Standard")}</button>
                  )}
                </span>
              </div>
            </div>
          </>
        )}
        {view === "info" && (
          <>
            <h1>VisuTeX</h1>
            <p>{t("WYSIWYG-LaTeX-Editor mit eingebauter TeX-Engine (Tectonic).")}</p>
            <dl className="info-list">
              <dt>{t("Version")}</dt>
              <dd>{props.appInfo?.version ?? "–"}</dd>
              <dt>{t("TeX-Bundle")}</dt>
              <dd>
                {props.appInfo?.bundle.kind === "online"
                  ? t("Kein mitgeliefertes Bundle gefunden – Pakete werden aus dem Tectonic-Cache bzw. online geladen.")
                  : `${t("Mitgeliefert")} (${props.appInfo?.bundle.fileCount ?? "?"} ${t("Dateien")})${props.appInfo?.bundle.kind === "embedded+online" ? ` + ${t("online")}` : ""}`}
              </dd>
              <dt>{t("Installierte Schriften")}</dt>
              <dd>{props.appInfo?.availableFonts.join(", ") ?? "–"}</dd>
            </dl>
            <div className="button-row left">
              <button type="button" className="primary" onClick={props.onSystemCheck}>{t("Systemprüfung …")}</button>
            </div>
          </>
        )}
      </main>
    </div>
  );
}

function RecentList({ recent, onOpen }: { recent: RecentFile[]; onOpen: (path: string) => void }) {
  const t = useT();
  if (recent.length === 0) return <p className="muted">{t("Noch keine Dateien geöffnet.")}</p>;
  return (
    <ul className="recent-list">
      {recent.map((file) => (
        <li key={file.path}>
          <button type="button" onClick={() => onOpen(file.path)}>
            <FileText size={18} />
            <span>
              <strong>{file.name}</strong>
              <small>{file.path}</small>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
