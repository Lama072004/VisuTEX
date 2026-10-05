/**
 * React-NodeViews für die VisuTeX-Knoten. Das Schema (Attribute, Parsing)
 * kommt unverändert aus `schema.ts`; hier wird nur die Darstellung ergänzt.
 */
import { NodeViewContent, NodeViewWrapper } from "@tiptap/react";
import type { ReactNodeViewProps } from "@tiptap/react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, ReactNode } from "react";
import { api, errorText, fileUrl, projectFileUrl } from "../api";
import type { BlockPreview, ResolvedImage } from "../api";
import { useT } from "../i18n";
import { latexToHtml, renderMath } from "../latex/miniRender";
import { formatQuantity } from "../latex/siunitx";
import { getRuntime, renderContext, useRuntime } from "../state/runtime";
import { cachedPreview, isCommentOnly, requestPreview } from "./latexPreview";
import { frontmatterTitles, titlePageFields } from "./schema";
import type { FrontmatterKind, TitlePageField } from "./schema";
import { documentLanguage } from "../latex/languages";

// ---------------------------------------------------------------- Hilfen

function renderKatex(latex: string, displayMode: boolean, environment = "equation"): string {
  let source = latex.trim() || "\\square";
  if (displayMode) {
    if (environment === "align" || environment === "eqnarray") source = `\\begin{aligned}${source.replace(/&=&/g, "&=")}\\end{aligned}`;
    else if (environment === "gather" || environment === "multline") source = `\\begin{gathered}${source}\\end{gathered}`;
  }
  return renderMath(source, displayMode, renderContext());
}

/** Beschriftung: Klartext oder (bei übernommenen LaTeX-Beschriftungen) gerendertes LaTeX. */
function Caption({ text, latex, kind }: { text: string; latex: boolean; kind: "figure" | "table" }) {
  const macros = useRuntime((runtime) => runtime.macros);
  const html = useMemo(() => (latex ? latexToHtml(text, renderContext()) : null), [text, latex, macros]);
  return html === null ? (
    <div className="figure-caption" data-kind={kind}>{text}</div>
  ) : (
    <div className="figure-caption" data-kind={kind} dangerouslySetInnerHTML={{ __html: html }} />
  );
}

const imageCache = new Map<string, ResolvedImage>();

/** Bild wie LaTeX auflösen (Unterordner, \graphicspath, ohne Endung, PDF-Grafiken). */
function useResolvedImage(root: string | null, latexPath: string, src: string): { url: string; kind: string; checked: boolean } {
  const preamble = useRuntime((runtime) => runtime.customPreamble);
  const key = `${root}|${latexPath}|${preamble ?? ""}`;
  const [resolved, setResolved] = useState<ResolvedImage | null>(() => imageCache.get(key) ?? null);
  useEffect(() => {
    if (!latexPath || !root) return;
    const cached = imageCache.get(key);
    if (cached) {
      setResolved(cached);
      return;
    }
    let cancelled = false;
    api
      .resolveImage(root, latexPath, preamble)
      .then((result) => {
        imageCache.set(key, result);
        if (!cancelled) setResolved(result);
      })
      .catch(() => {
        if (!cancelled) setResolved({ file: null, relative: null, dataUrl: null, kind: "missing" });
      });
    return () => {
      cancelled = true;
    };
  }, [key, root, latexPath, preamble]);
  if (!latexPath) return { url: src, kind: src ? "image" : "missing", checked: true };
  if (!resolved) return { url: projectFileUrl(root, latexPath), kind: "image", checked: false };
  const url = resolved.dataUrl ?? (resolved.file ? fileUrl(resolved.file) : "");
  return { url, kind: resolved.kind, checked: true };
}

function stop(event: { stopPropagation: () => void }) {
  event.stopPropagation();
}

/** Bearbeitungsbereich innerhalb einer NodeView (Ereignisse nicht an ProseMirror weitergeben). */
function EditPanel({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={`node-edit-panel ${className}`}
      contentEditable={false}
      onMouseDown={stop}
      onKeyDown={stop}
      onPaste={stop}
      onCopy={stop}
      onCut={stop}
    >
      {children}
    </div>
  );
}

/**
 * Codefeld so hoch wie sein Inhalt (bis 70 % der Fensterhöhe, darüber scrollbar).
 * Läuft nach jedem Rendern – auch wenn das Feld erst beim Umschalten auf „Code
 * bearbeiten“ entsteht (sonst blieb es auf zwei Zeilen stehen).
 */
function useAutoHeight(value: string) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    element.style.height = "auto";
    const limit = Math.max(240, window.innerHeight * 0.7);
    const needed = element.scrollHeight + 2;
    element.style.height = `${Math.min(limit, needed)}px`;
    element.style.overflowY = needed > limit ? "auto" : "hidden";
  });
  void value;
  return ref;
}

const labelPattern = /^[A-Za-z0-9:._-]*$/;

// ---------------------------------------------------------------- Formeln

export function MathBlockView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const latex = String(node.attrs.latex ?? "");
  const environment = String(node.attrs.environment || "equation");
  const numbered = Boolean(node.attrs.numbered) && node.attrs.numbered !== "false";
  const label = String(node.attrs.label ?? "");
  const [draft, setDraft] = useState(latex);
  const textareaRef = useAutoHeight(draft);
  useEffect(() => setDraft(latex), [latex]);
  const preview = useMemo(() => renderKatex(selected ? draft : latex, true, environment), [draft, latex, selected, environment]);

  return (
    <NodeViewWrapper className={`math-block${selected ? " is-selected" : ""}`} data-numbered={numbered ? "true" : undefined} data-label={label || undefined}>
      <div className="math-rendered" dangerouslySetInnerHTML={{ __html: preview }} />
      {numbered && <span className="equation-number" aria-hidden="true" />}
      {selected && (
        <EditPanel>
          <textarea
            ref={textareaRef}
            className="code-input"
            value={draft}
            spellCheck={false}
            autoFocus
            aria-label={t("LaTeX-Formel")}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onBlur={() => draft !== latex && updateAttributes({ latex: draft })}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) updateAttributes({ latex: draft });
            }}
          />
          <div className="node-edit-row">
            <label>
              {t("Umgebung")}
              <select value={environment} onChange={(event) => updateAttributes({ latex: draft, environment: event.currentTarget.value })}>
                <option value="equation">equation</option>
                <option value="align">align</option>
                <option value="gather">gather</option>
                <option value="multline">multline</option>
                <option value="eqnarray">eqnarray</option>
              </select>
            </label>
            <label className="checkbox">
              <input type="checkbox" checked={numbered} onChange={(event) => updateAttributes({ latex: draft, numbered: event.currentTarget.checked })} />
              {t("Nummeriert")}
            </label>
            {numbered && (
              <label>
                {t("Label")}
                <input
                  value={label}
                  placeholder="eq:name"
                  onChange={(event) => labelPattern.test(event.currentTarget.value) && updateAttributes({ label: event.currentTarget.value })}
                />
              </label>
            )}
          </div>
        </EditPanel>
      )}
    </NodeViewWrapper>
  );
}

export function InlineMathView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const latex = String(node.attrs.latex ?? "");
  const [draft, setDraft] = useState(latex);
  useEffect(() => setDraft(latex), [latex]);
  const html = useMemo(() => renderKatex(selected ? draft : latex, false), [draft, latex, selected]);
  return (
    <NodeViewWrapper as="span" className={`inline-math${selected ? " is-selected" : ""}`}>
      <span dangerouslySetInnerHTML={{ __html: html }} />
      {selected && (
        <span className="inline-math-editor" contentEditable={false} onMouseDown={stop} onKeyDown={stop}>
          <input
            value={draft}
            spellCheck={false}
            autoFocus
            aria-label={t("LaTeX-Formel")}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onBlur={() => draft !== latex && updateAttributes({ latex: draft })}
            onKeyDown={(event) => {
              if (event.key === "Enter") updateAttributes({ latex: draft });
            }}
          />
        </span>
      )}
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- TikZ

/** Meta-Kennzeichen für Transaktionen, die nur Laufzeitdaten ändern (keine Dokumentänderung im Sinne von LaTeX). */
export const RUNTIME_META = "visutex:runtime";

export function TikzBlockView({ node, updateAttributes, selected, editor, getPos }: ReactNodeViewProps) {
  const t = useT();
  const code = String(node.attrs.code ?? "");
  const environment = String(node.attrs.environment || "tikzpicture");
  const options = String(node.attrs.options ?? "");
  const caption = String(node.attrs.caption ?? "");
  const label = String(node.attrs.label ?? "");
  const preview = String(node.attrs.preview ?? "");
  const previewKey = `${environment}|${options}|${code}`;
  const upToDate = preview && node.attrs.previewCode === previewKey;
  // Skizzen tragen ihr Modell als erste Kommentarzeile – im Code-Feld ausgeblendet.
  const sketchHeader = code.startsWith("% VisuTeX-Skizze:") ? code.split("\n")[0] : "";
  const body = sketchHeader ? code.slice(sketchHeader.length).replace(/^\r?\n/, "") : code;
  const [draft, setDraft] = useState(body);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const allowOnline = useRuntime((runtime) => runtime.allowOnline);
  const textareaRef = useAutoHeight(draft);
  useEffect(() => setDraft(body), [body]);
  /** Vollständiger Code zum Entwurf (von Hand geänderte Skizze verliert ihr Modell). */
  const fullCode = (text: string) => (text === body ? code : text);
  const editing = draft !== body;

  const render = async (source = fullCode(draft)) => {
    setBusy(true);
    setError("");
    try {
      const png = await api.renderTikz(source, environment, options, allowOnline);
      const previewCode = `${environment}|${options}|${source}`;
      const pos = typeof getPos === "function" ? getPos() : undefined;
      if (source === code && typeof pos === "number") {
        // Nur die Vorschau ändert sich – nicht als Bearbeitung zählen.
        editor
          .chain()
          .command(({ tr }) => {
            const current = tr.doc.nodeAt(pos);
            if (!current || current.type.name !== "tikzBlock") return false;
            tr.setNodeMarkup(pos, undefined, { ...current.attrs, preview: png, previewCode });
            tr.setMeta(RUNTIME_META, true).setMeta("addToHistory", false);
            return true;
          })
          .run();
      } else {
        updateAttributes({ code: source, preview: png, previewCode });
      }
    } catch (renderError) {
      setError(errorText(renderError));
    } finally {
      setBusy(false);
    }
  };

  // Vorschau automatisch erzeugen, wenn sie fehlt oder veraltet ist und der Code gerade nicht bearbeitet wird.
  useEffect(() => {
    if (!upToDate && !editing && !busy && !error && code.trim()) void render(code);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [upToDate, editing]);

  return (
    <NodeViewWrapper className={`tikz-block${selected ? " is-selected" : ""}`} data-label={label || undefined} data-caption={caption ? "true" : undefined}>
      <div className="tikz-preview-frame">
        {preview ? (
          <img className={`tikz-preview${upToDate ? "" : " is-stale"}`} src={preview} alt={caption || t("TikZ-Zeichnung")} draggable={false} />
        ) : (
          <div className="tikz-placeholder">{busy ? t("Zeichnung wird erzeugt …") : t("TikZ-Zeichnung (Vorschau folgt)")}</div>
        )}
        {busy && preview && <span className="tikz-busy">{t("Aktualisiere …")}</span>}
      </div>
      {caption && <Caption text={caption} latex={node.attrs.captionLatex === true} kind="figure" />}
      {error && !selected && <div className="node-error">{error.split("\n")[0]}</div>}
      {selected && (
        <EditPanel>
          <textarea
            ref={textareaRef}
            className="code-input"
            value={draft}
            spellCheck={false}
            autoFocus
            aria-label={t("TikZ-Code")}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onBlur={() => editing && updateAttributes({ code: draft })}
          />
          {sketchHeader && editing && <div className="node-hint">{t("Von Hand geänderte Skizzen lassen sich danach nicht mehr im Skizzier-Werkzeug bearbeiten.")}</div>}
          <div className="node-edit-row">
            <label>
              {t("Umgebung")}
              <select value={environment} onChange={(event) => updateAttributes({ code: fullCode(draft), environment: event.currentTarget.value })}>
                <option value="tikzpicture">tikzpicture</option>
                <option value="circuitikz">circuitikz</option>
              </select>
            </label>
            <label>
              {t("Optionen")}
              <input value={options} placeholder="scale=1" onChange={(event) => updateAttributes({ options: event.currentTarget.value })} />
            </label>
            <label className="grow">
              {t("Beschriftung")}
              <input value={caption} onChange={(event) => updateAttributes({ caption: event.currentTarget.value })} />
            </label>
            <label>
              {t("Label")}
              <input value={label} placeholder="fig:name" onChange={(event) => labelPattern.test(event.currentTarget.value) && updateAttributes({ label: event.currentTarget.value })} />
            </label>
            <button type="button" className="primary" disabled={busy} onClick={() => void render()}>
              {busy ? t("Kompiliere …") : t("Vorschau aktualisieren")}
            </button>
            {sketchHeader && !editing && (
              <button
                type="button"
                onClick={() => {
                  const pos = typeof getPos === "function" ? getPos() : undefined;
                  if (typeof pos === "number") getRuntime().editNode({ type: "sketch", pos });
                }}
              >
                {t("In Skizze bearbeiten")}
              </button>
            )}
          </div>
          {error && <pre className="node-error">{error}</pre>}
        </EditPanel>
      )}
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Bilder

export function ImageView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const root = useRuntime((runtime) => runtime.projectRoot);
  const latexPath = String(node.attrs.latexPath ?? "");
  const src = String(node.attrs.src ?? "");
  const resolved = useResolvedImage(root, latexPath, src);
  const url = resolved.url;
  const width = Math.min(100, Math.max(5, Number(node.attrs.widthPercent) || 80));
  const caption = String(node.attrs.caption ?? "");
  const captionAbove = node.attrs.captionAbove === true;
  const [failed, setFailed] = useState(false);
  const notDisplayable = resolved.kind === "eps" || (resolved.kind === "pdf" && !url.startsWith("data:"));
  const missing = resolved.checked && resolved.kind === "missing";
  useEffect(() => setFailed(false), [url]);
  const frameRef = useRef<HTMLDivElement>(null);

  const startResize = (event: ReactPointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const frame = frameRef.current;
    const container = frame?.parentElement;
    if (!frame || !container) return;
    const startX = event.clientX;
    const startWidth = frame.getBoundingClientRect().width;
    const containerWidth = container.getBoundingClientRect().width || 1;
    const move = (moveEvent: PointerEvent) => {
      const next = Math.round(((startWidth + moveEvent.clientX - startX) / containerWidth) * 100);
      frame.style.width = `${Math.min(100, Math.max(5, next))}%`;
    };
    const up = (upEvent: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      const next = Math.round(((startWidth + upEvent.clientX - startX) / containerWidth) * 100);
      updateAttributes({ widthPercent: Math.min(100, Math.max(5, next)) });
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const captionElement = caption ? <Caption text={caption} latex={node.attrs.captionLatex === true} kind="figure" /> : null;
  return (
    <NodeViewWrapper className={`image-block${selected ? " is-selected" : ""}`} data-caption={caption ? "true" : undefined}>
      {captionAbove && captionElement}
      <div className="image-frame" ref={frameRef} style={{ width: `${width}%` }} contentEditable={false}>
        {!url || failed || missing || notDisplayable ? (
          <div className={`image-placeholder${failed || missing ? " is-missing" : ""}`}>
            <strong>
              {failed || missing || !url
                ? t("Bild nicht gefunden")
                : resolved.kind === "eps"
                  ? t("EPS-Grafik (Vorschau nicht möglich)")
                  : t("PDF-Grafik")}
            </strong>
            <code>{latexPath || t("(eingebettet)")}</code>
          </div>
        ) : (
          <img src={url} alt={String(node.attrs.alt || caption || latexPath)} draggable={false} onError={() => setFailed(true)} />
        )}
        {selected && <span className="resize-handle" onPointerDown={startResize} title={t("Breite ändern")} />}
      </div>
      {!captionAbove && captionElement}
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Titelseite und Vorspann

const titleFieldLabels: Record<TitlePageField, string> = {
  title: "Titel",
  subtitle: "Untertitel",
  thesisType: "Art der Arbeit",
  degreeIntro: "Einleitung Grad",
  degree: "Akademischer Grad",
  institution: "Hochschule",
  program: "Studiengang",
  authorLabel: "Beschriftung Autor",
  author: "Autor/in",
  placeDate: "Ort, Datum",
  supervisorLabel: "Beschriftung Betreuung",
  supervisor: "Betreuung",
  logoPath: "Logo (Pfad im Projekt)",
};

function TitleField({ field, value, onChange, className }: { field: TitlePageField; value: string; onChange: (value: string) => void; className: string }) {
  const t = useT();
  return (
    <input
      className={`title-field ${className}`}
      value={value}
      placeholder={t(titleFieldLabels[field])}
      aria-label={t(titleFieldLabels[field])}
      onChange={(event) => onChange(event.currentTarget.value)}
      onKeyDown={stop}
      onMouseDown={stop}
      onPaste={stop}
    />
  );
}

export function TitlePageView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const root = useRuntime((runtime) => runtime.projectRoot);
  const value = (field: TitlePageField) => String(node.attrs[field] ?? "");
  const field = (name: TitlePageField, className: string) => (
    <TitleField field={name} value={value(name)} className={className} onChange={(next) => updateAttributes({ [name]: next })} />
  );
  const logo = value("logoPath");
  return (
    <NodeViewWrapper className={`title-page${selected ? " is-selected" : ""}`} contentEditable={false}>
      <div className="title-page-badge">{t("Titelseite")}</div>
      <div className="title-page-logo">
        {logo ? <img src={projectFileUrl(root, logo)} alt="" onError={(event) => (event.currentTarget.style.display = "none")} /> : null}
        {selected && field("logoPath", "small")}
      </div>
      {/* Reihenfolge und Abstände wie im Export (Layout einer Abschlussarbeit, linksbündig) */}
      {field("title", "huge")}
      {field("subtitle", "large")}
      <div className="title-page-gap" style={{ height: "1cm" }} />
      {field("thesisType", "normal")}
      {field("degreeIntro", "normal")}
      <div className="title-page-gap" style={{ height: "0.5cm" }} />
      {field("degree", "normal bold")}
      <div className="title-page-gap" style={{ height: "1cm" }} />
      {field("institution", "normal")}
      {field("program", "normal")}
      <div className="title-page-gap" style={{ height: "2.5cm" }} />
      {field("authorLabel", "normal")}
      {field("author", "normal")}
      {field("placeDate", "normal")}
      <div className="title-page-gap" style={{ height: "3cm" }} />
      {field("supervisorLabel", "normal")}
      {field("supervisor", "normal")}
    </NodeViewWrapper>
  );
}

export function FrontmatterView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const kind = String(node.attrs.kind || "abstract") as FrontmatterKind;
  return (
    <NodeViewWrapper className={`frontmatter-block frontmatter-${kind}${selected ? " is-selected" : ""}`}>
      <div className="frontmatter-header" contentEditable={false}>
        <span className="frontmatter-kind">{t(frontmatterTitles[kind] ?? kind)}</span>
        <input
          className="frontmatter-title"
          value={String(node.attrs.title ?? "")}
          aria-label={t("Überschrift")}
          onChange={(event) => updateAttributes({ title: event.currentTarget.value })}
          onKeyDown={stop}
          onMouseDown={stop}
        />
        <label className="checkbox" onMouseDown={stop}>
          <input
            type="checkbox"
            checked={node.attrs.inToc === true || node.attrs.inToc === "true"}
            onChange={(event) => updateAttributes({ inToc: event.currentTarget.checked })}
          />
          {t("Im Inhaltsverzeichnis")}
        </label>
      </div>
      <NodeViewContent className="frontmatter-content" />
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Verzeichnisse

type DirectoryEntry = { level: number; text: string; number: string };

function useDocumentEntries(editor: ReactNodeViewProps["editor"], kind: string) {
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  useEffect(() => {
    const compute = () => {
      const result: DirectoryEntry[] = [];
      const counters = [0, 0, 0, 0, 0, 0];
      let figure = 0;
      let table = 0;
      const acronyms = new Map<string, string>();
      editor.state.doc.descendants((child) => {
        const type = child.type.name;
        if (kind === "contents" && type === "heading") {
          const level = Number(child.attrs.level) || 1;
          const numbered = child.attrs.numbered !== false;
          if (numbered) {
            counters[level - 1] += 1;
            for (let index = level; index < counters.length; index += 1) counters[index] = 0;
          }
          if (level <= getRuntime().settings.tocDepth + 1) {
            result.push({ level, text: child.textContent, number: numbered ? counters.slice(0, level).join(".") : "" });
          }
          return false;
        }
        if (kind === "figures" && (type === "image" || type === "tikzBlock") && child.attrs.caption) {
          figure += 1;
          result.push({ level: 1, text: String(child.attrs.caption), number: String(figure) });
        }
        if (kind === "tables" && type === "table" && child.attrs.caption) {
          table += 1;
          result.push({ level: 1, text: String(child.attrs.caption), number: String(table) });
        }
        if (kind === "acronyms" && type === "acronym") {
          const short = String(child.attrs.short || child.attrs.key);
          if (!acronyms.has(short)) acronyms.set(short, String(child.attrs.long ?? ""));
        }
        return true;
      });
      if (kind === "acronyms") {
        [...acronyms.entries()]
          .sort(([left], [right]) => left.localeCompare(right))
          .forEach(([short, long]) => result.push({ level: 1, number: short, text: long }));
      }
      setEntries(result);
    };
    compute();
    editor.on("update", compute);
    return () => {
      editor.off("update", compute);
    };
  }, [editor, kind]);
  return entries;
}

const directoryTitles: Record<string, string> = {
  contents: "Inhaltsverzeichnis",
  figures: "Abbildungsverzeichnis",
  tables: "Tabellenverzeichnis",
  acronyms: "Abkürzungsverzeichnis",
  bibliography: "Literaturverzeichnis",
};

export function DirectoryView({ node, editor, selected, updateAttributes }: ReactNodeViewProps) {
  const t = useT();
  const kind = String(node.attrs.kind || "contents");
  const entries = useDocumentEntries(editor, kind);
  const language = useRuntime((runtime) => runtime.settings.language);
  const labels = documentLanguage(language).labels;
  const title = (labels as Record<string, string>)[kind] ?? labels.list;
  const inToc = node.attrs.inToc === true;
  return (
    <NodeViewWrapper className={`directory-block directory-${kind}${selected ? " is-selected" : ""}`} contentEditable={false}>
      <div className="directory-title">{title ?? t(directoryTitles[kind] ?? kind)}</div>
      {selected && kind !== "acronyms" && (
        <EditPanel>
          <div className="node-edit-row">
            <label className="checkbox">
              <input type="checkbox" checked={inToc} onChange={(event) => updateAttributes({ inToc: event.currentTarget.checked || null })} />
              {t("Im Inhaltsverzeichnis")}
            </label>
            {kind === "bibliography" && (
              <label>
                {t("Befehl")}
                <select
                  value={node.attrs.command === "printbibliography" ? "printbibliography" : "bibtex"}
                  onChange={(event) => updateAttributes({ command: event.currentTarget.value === "printbibliography" ? "printbibliography" : null })}
                >
                  <option value="bibtex">BibTeX (\bibliography)</option>
                  <option value="printbibliography">biblatex (\printbibliography)</option>
                </select>
              </label>
            )}
          </div>
        </EditPanel>
      )}
      {kind === "bibliography" ? (
        <p className="directory-hint">{t("Das Literaturverzeichnis wird beim Kompilieren aus der BibTeX-Datei erzeugt (nur zitierte Einträge).")}</p>
      ) : entries.length === 0 ? (
        <p className="directory-hint">{t("Noch keine Einträge – das Verzeichnis wird beim Kompilieren erzeugt.")}</p>
      ) : (
        <ol className="directory-entries">
          {entries.slice(0, 400).map((entry, index) => (
            <li key={index} className={`level-${entry.level}`}>
              {entry.number && <span className="directory-number">{entry.number}</span>}
              <span className="directory-text">{entry.text}</span>
            </li>
          ))}
        </ol>
      )}
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Roh-LaTeX und Umbrüche

/**
 * Roh-LaTeX-Block: zeigt die kompilierte Vorschau (Tabellen, Titelseiten, eigene
 * Umgebungen …); „Code bearbeiten“ bzw. Doppelklick öffnet den Quelltext.
 * Reine Kommentarblöcke erscheinen als dezente Notiz.
 */
export function RawLatexBlockView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const t = useT();
  const raw = String(node.attrs.rawLatex ?? "");
  const [draft, setDraft] = useState(raw);
  const [editing, setEditing] = useState(false);
  const textareaRef = useAutoHeight(draft);
  useEffect(() => setDraft(raw), [raw]);
  const missing = Array.isArray(node.attrs.missingResources) ? (node.attrs.missingResources as string[]) : [];
  const comment = isCommentOnly(raw);
  const [preview, setPreview] = useState<BlockPreview | null>(() => (comment ? null : cachedPreview(raw)));
  const [loading, setLoading] = useState(false);
  const root = useRuntime((runtime) => runtime.projectRoot);
  const preamble = useRuntime((runtime) => runtime.customPreamble);

  useEffect(() => {
    if (comment || !raw.trim()) return;
    const cached = cachedPreview(raw);
    if (cached) {
      setPreview(cached);
      return;
    }
    let cancelled = false;
    setLoading(true);
    void requestPreview(raw).then((result) => {
      if (cancelled) return;
      setPreview(result);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [raw, comment, root, preamble]);

  const showCode = editing || comment || (!preview?.image && !loading);
  const finish = () => {
    if (draft !== raw) updateAttributes({ rawLatex: draft });
    setEditing(false);
  };

  if (comment) {
    return (
      <NodeViewWrapper className={`raw-latex-block is-comment${selected ? " is-selected" : ""}`}>
        <textarea
          ref={textareaRef}
          className="code-input raw-latex-input comment-input"
          value={draft}
          spellCheck={false}
          aria-label={t("LaTeX-Kommentar")}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onBlur={finish}
          onKeyDown={stop}
          onMouseDown={stop}
          onPaste={stop}
          onCopy={stop}
          onCut={stop}
        />
      </NodeViewWrapper>
    );
  }

  return (
    <NodeViewWrapper className={`raw-latex-block${selected ? " is-selected" : ""}${preview?.image && !showCode ? " has-preview" : ""}`}>
      <div className="raw-latex-label" contentEditable={false}>
        <span>LaTeX</span>
        <button type="button" className="raw-latex-toggle" onMouseDown={stop} onClick={() => (editing ? finish() : setEditing(true))}>
          {editing ? t("Vorschau") : t("Code bearbeiten")}
        </button>
      </div>
      {!showCode && preview?.image && (
        <div className="raw-latex-preview" contentEditable={false} onDoubleClick={() => setEditing(true)} title={t("Doppelklick: Code bearbeiten")}>
          <img src={preview.image} alt={t("Vorschau")} draggable={false} />
        </div>
      )}
      {!editing && loading && !preview?.image && <div className="raw-latex-loading" contentEditable={false}>{t("Vorschau wird erzeugt …")}</div>}
      {showCode && (
        <textarea
          ref={textareaRef}
          className="code-input raw-latex-input"
          value={draft}
          spellCheck={false}
          autoFocus={editing}
          aria-label={t("LaTeX-Code")}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onBlur={finish}
          onKeyDown={stop}
          onMouseDown={stop}
          onPaste={stop}
          onCopy={stop}
          onCut={stop}
        />
      )}
      {preview?.error && showCode && !editing && <div className="node-error">{t("Vorschau nicht möglich: ")}{preview.error.split("\n")[0]}</div>}
      {missing.length > 0 && <div className="node-error">{t("Fehlende Dateien: ")}{missing.join(", ")}</div>}
    </NodeViewWrapper>
  );
}

export function PageBreakView({ node, selected, getPos }: ReactNodeViewProps) {
  const t = useT();
  const section = node.attrs.breakType === "section";
  const columns = Number(node.attrs.columns) || 1;
  const orientation = String(node.attrs.orientation || "keep");
  const details = section
    ? [
        `${columns} ${columns === 1 ? t("Spalte") : t("Spalten")}`,
        orientation === "landscape" ? t("Querformat") : orientation === "portrait" ? t("Hochformat") : "",
      ].filter(Boolean).join(", ")
    : "";
  return (
    <NodeViewWrapper
      className={`document-break${section ? " is-section" : ""}${selected ? " is-selected" : ""}`}
      contentEditable={false}
      onDoubleClick={() => {
        const pos = typeof getPos === "function" ? getPos() : undefined;
        if (typeof pos === "number") getRuntime().editNode({ type: "pageBreak", pos });
      }}
    >
      <span>{section ? `${t("Abschnittsumbruch")} – ${details}` : t("Seitenumbruch")}</span>
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Inline-Chips

function useEditOnClick(
  type: "citation" | "footnote" | "acronym" | "crossReference" | "rawLatexInline" | "quantity" | "verticalSpace" | "environmentBlock",
  getPos: ReactNodeViewProps["getPos"],
) {
  return () => {
    const pos = typeof getPos === "function" ? getPos() : undefined;
    if (typeof pos === "number") getRuntime().editNode({ type, pos });
  };
}

export function CitationView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("citation", getPos);
  const keys = String(node.attrs.keys ?? "");
  const prenote = String(node.attrs.prenote ?? "");
  const postnote = String(node.attrs.postnote ?? "");
  const command = String(node.attrs.command || "cite");
  const text = [prenote, keys.split(",").map((key) => key.trim()).filter(Boolean).join("; "), postnote].filter(Boolean).join(", ");
  return (
    <NodeViewWrapper as="span" className={`citation-chip${selected ? " is-selected" : ""}`} title={`\\${command}{${keys}}`} onDoubleClick={edit}>
      {command === "citet" ? text : `[${text}]`}
    </NodeViewWrapper>
  );
}

export function FootnoteView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("footnote", getPos);
  return (
    <NodeViewWrapper as="span" className={`footnote-chip${selected ? " is-selected" : ""}`} title={String(node.attrs.text ?? "")} onDoubleClick={edit}>
      <sup className="footnote-marker" />
    </NodeViewWrapper>
  );
}

export function AcronymView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("acronym", getPos);
  const short = String(node.attrs.short || node.attrs.key);
  const long = String(node.attrs.long ?? "");
  const command = String(node.attrs.command || "ac");
  const text = command === "acl" ? long : command === "acf" ? `${long} (${short})` : command === "acp" ? `${short}s` : short;
  return (
    <NodeViewWrapper as="span" className={`acronym-chip${selected ? " is-selected" : ""}`} title={`${short}: ${long}`} onDoubleClick={edit}>
      {text}
    </NodeViewWrapper>
  );
}

export function CrossReferenceView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("crossReference", getPos);
  const label = String(node.attrs.label ?? "");
  const kind = String(node.attrs.kind || "ref");
  return (
    <NodeViewWrapper as="span" className={`inline-reference${selected ? " is-selected" : ""}`} title={`\\${kind}{${label}}`} onDoubleClick={edit}>
      {kind === "pageref" ? `S. ${label}` : kind === "eqref" ? `(${label})` : label}
    </NodeViewWrapper>
  );
}

/** Befehle, die im Text als Abstand bzw. unsichtbar erscheinen (keine Code-Marke). */
const QUIET_INLINE = /^\\(quad|qquad|,|;|:|!| |enspace|hfill|hfil|noindent|indent|par|newline|smallskip|medskip|bigskip|\\(\[[^\]]*\])?)\s*$/;

/**
 * Roh-LaTeX im Text: wird gerendert angezeigt (z. B. `\enquote{…}`, eigene
 * Makros wie `\teil{a}`, Abstände). Doppelklick bearbeitet den Code.
 */
export function RawLatexInlineView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("rawLatexInline", getPos);
  const latex = String(node.attrs.latex ?? "");
  const macros = useRuntime((runtime) => runtime.macros);
  const html = useMemo(() => latexToHtml(latex, renderContext()), [latex, macros]);
  const visible = html.replace(/<[^>]*>/g, "").trim();
  const quiet = QUIET_INLINE.test(latex.trim());
  const commandOnly = !visible || /^<span class="tex-cmd">[^<]*<\/span>$/.test(html.trim());
  const className = quiet ? "raw-latex-inline is-quiet" : commandOnly ? "raw-latex-inline" : "raw-latex-inline is-rendered";
  return (
    <NodeViewWrapper as="span" className={`${className}${selected ? " is-selected" : ""}`} onDoubleClick={edit} title={latex}>
      {quiet || !commandOnly ? <span dangerouslySetInnerHTML={{ __html: quiet && !visible ? " " : html }} /> : latex}
    </NodeViewWrapper>
  );
}

// ---------------------------------------------------------------- Neue Elemente (siunitx, Abstand, Umgebungen …)

export function QuantityView({ node, selected, getPos }: ReactNodeViewProps) {
  const edit = useEditOnClick("quantity", getPos);
  const language = useRuntime((runtime) => runtime.settings.language);
  const text = formatQuantity(
    { command: node.attrs.command, value: node.attrs.value, value2: node.attrs.value2, unit: node.attrs.unit },
    language,
  );
  const command = String(node.attrs.command || "SI");
  return (
    <NodeViewWrapper
      as="span"
      className={`quantity-chip${selected ? " is-selected" : ""}`}
      onDoubleClick={edit}
      title={`\\${command}{${node.attrs.value ?? ""}}{${node.attrs.unit ?? ""}}`}
    >
      {text || "?"}
    </NodeViewWrapper>
  );
}

/** Länge in Pixel (für die Anzeige eines Abstands). */
function lengthToPixels(command: string, size: string): number {
  const fixed: Record<string, number> = { smallskip: 4, medskip: 8, bigskip: 16, vfill: 28 };
  if (fixed[command]) return fixed[command];
  const match = /^\s*(-?\d*[.,]?\d+)?\s*\\?([a-z]+)/i.exec(size);
  if (!match) return 16;
  const value = Number((match[1] ?? "1").replace(",", "."));
  const units: Record<string, number> = { cm: 37.8, mm: 3.78, in: 96, pt: 1.33, bp: 1.33, em: 16, ex: 8, baselineskip: 20, pc: 16 };
  return Math.max(4, Math.min(600, value * (units[match[2]] ?? 16)));
}

export function VerticalSpaceView({ node, selected, getPos }: ReactNodeViewProps) {
  const t = useT();
  const edit = useEditOnClick("verticalSpace", getPos);
  const command = String(node.attrs.command || "vspace");
  const size = String(node.attrs.size ?? "");
  const height = lengthToPixels(command, size);
  const label = command.startsWith("vspace") ? `${t("Abstand")} ${size}` : `\\${command}`;
  return (
    <NodeViewWrapper className={`vertical-space${selected ? " is-selected" : ""}`} contentEditable={false} onDoubleClick={edit} title={t("Doppelklick: Abstand ändern")}>
      <div className="vertical-space-gap" style={{ height }}>
        <span className="vertical-space-label">{label}</span>
      </div>
    </NodeViewWrapper>
  );
}

/** Titel einer Umgebung aus ihren Argumenten (`title=…`, erstes {…}, […]). */
function environmentTitle(name: string, args: string): string {
  const title = /title\s*=\s*(\{((?:[^{}]|\{[^{}]*\})*)\}|[^,\]]+)/.exec(args);
  if (title) return (title[2] ?? title[1]).trim();
  if (name === "multicols" || name === "multicols*") return "";
  const group = /\{((?:[^{}]|\{[^{}]*\})*)\}/.exec(args);
  if (group && !/^[\d.]+\\(line|text|column)width$/.test(group[1].trim())) return group[1];
  const optional = /^\s*\[([^\]]*)\]/.exec(args);
  if (optional && !optional[1].includes("=") && !["minipage", "multicols", "multicols*", "mdframed", "tcolorbox"].includes(name)) return optional[1];
  return "";
}

const environmentLabels: Record<string, string> = {
  tcolorbox: "Box",
  abstract: "Zusammenfassung (abstract)",
  multicols: "Spalten",
  "multicols*": "Spalten",
  minipage: "Minipage",
  center: "Zentriert",
  flushleft: "Linksbündig",
  flushright: "Rechtsbündig",
  quotation: "Zitat",
  verse: "Vers",
  landscape: "Querformat",
  proof: "Beweis",
};

export function EnvironmentBlockView({ node, selected, getPos }: ReactNodeViewProps) {
  const t = useT();
  const edit = useEditOnClick("environmentBlock", getPos);
  const name = String(node.attrs.name || "");
  const args = String(node.attrs.args ?? "");
  const macros = useRuntime((runtime) => runtime.macros);
  const title = environmentTitle(name, args);
  const titleHtml = useMemo(() => (title ? latexToHtml(title, renderContext()) : ""), [title, macros]);
  const columns = /^multicols\*?$/.test(name) ? Number(/\{(\d+)\}/.exec(args)?.[1] ?? 2) : 0;
  const width = name === "minipage" ? /\{([\d.]*)\\(?:line|text|column)width\}/.exec(args)?.[1] : undefined;
  const colors = name === "tcolorbox" || getRuntime().customPreamble?.includes(`{${name}}`) ? /colframe\s*=\s*([a-zA-Z]+)/.exec(args)?.[1] : undefined;
  return (
    <NodeViewWrapper
      className={`environment-block env-${name.replace(/\*$/, "-star").replace(/[^a-zA-Z-]/g, "")}${selected ? " is-selected" : ""}`}
      data-environment={name}
      style={width ? { width: `${Math.round(Number(width || 1) * 100)}%` } : undefined}
    >
      <div className="environment-header" contentEditable={false} onDoubleClick={edit} title={t("Doppelklick: Umgebung bearbeiten")} style={colors ? { borderColor: colors } : undefined}>
        <span className="environment-name">{t(environmentLabels[name] ?? name)}</span>
        {titleHtml && <span className="environment-title" dangerouslySetInnerHTML={{ __html: titleHtml }} />}
        {columns > 1 && <span className="environment-meta">{columns} {t("Spalten")}</span>}
      </div>
      <NodeViewContent className="environment-content" />
    </NodeViewWrapper>
  );
}

export function MaketitleView({ selected }: ReactNodeViewProps) {
  const t = useT();
  const metadata = useRuntime((runtime) => runtime.settings.metadata);
  return (
    <NodeViewWrapper className={`maketitle-block${selected ? " is-selected" : ""}`} contentEditable={false}>
      <div className="maketitle-title">{metadata.title || t("[Titel]")}</div>
      <div className="maketitle-author">{metadata.author || t("[Autor]")}</div>
      <div className="maketitle-date">{new Date().toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" })}</div>
      <div className="maketitle-hint">{t("\\maketitle – Titel und Autor unter Datei → Dokumenteigenschaften bzw. in der Präambel")}</div>
    </NodeViewWrapper>
  );
}

export function AppendixView({ selected }: ReactNodeViewProps) {
  const t = useT();
  return (
    <NodeViewWrapper className={`document-break is-appendix${selected ? " is-selected" : ""}`} contentEditable={false}>
      <span>{t("Anhang (\\appendix) – folgende Kapitel werden mit Buchstaben nummeriert")}</span>
    </NodeViewWrapper>
  );
}

export function IncludeBlockView({ node, selected }: ReactNodeViewProps) {
  const t = useT();
  const file = String(node.attrs.file ?? "");
  const command = String(node.attrs.command || "input");
  return (
    <NodeViewWrapper className={`include-block${selected ? " is-selected" : ""}`}>
      <div className="include-header" contentEditable={false}>
        <span className="include-file">{file}</span>
        <span className="include-command">{`\\${command}{${file.replace(/\.tex$/, "")}}`}</span>
        <span className="include-hint">{t("eingebundene Datei – wird beim Speichern wieder in diese Datei geschrieben")}</span>
      </div>
      <NodeViewContent className="include-content" />
    </NodeViewWrapper>
  );
}

export { titlePageFields };
