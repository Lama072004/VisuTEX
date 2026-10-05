/**
 * Vereinfachtes Menüband (ohne Drag&Drop-Anpassungsmodus): feste Tabs,
 * kontextuelle Tabs für Tabelle und Bild, Add-on-Tab, Überlaufmenü „Weitere“.
 * Aktivzustände über `useEditorState` (aktualisieren auch bei Cursorbewegung).
 */
import type { Editor } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  AlignCenter,
  AlignJustify,
  AlignLeft,
  AlignRight,
  Baseline,
  Bold,
  BookA,
  BookMarked,
  BookOpen,
  Braces,
  ChevronDown,
  CircuitBoard,
  Code2,
  Columns2,
  Columns3,
  Eraser,
  Eye,
  FileText,
  Frame,
  Hash,
  Highlighter,
  ImagePlus,
  Images,
  IndentDecrease,
  IndentIncrease,
  Italic,
  Keyboard,
  LayoutTemplate,
  Library,
  Heading,
  Link2,
  List,
  ListOrdered,
  ListTree,
  MoveVertical,
  Paperclip,
  Ruler,
  SquareDashed,
  PaintBucket,
  Paintbrush,
  PanelLeft,
  PanelRight,
  PenTool,
  Pilcrow,
  Play,
  Presentation,
  Puzzle,
  Radical,
  RectangleHorizontal,
  RectangleVertical,
  Replace,
  ScrollText,
  Search,
  SeparatorHorizontal,
  Settings,
  Shapes,
  Sigma,
  Square,
  StickyNote,
  Strikethrough,
  Subscript,
  Superscript,
  Table2,
  TableProperties,
  Trash2,
  Underline,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import type { DocumentSettings } from "../latex/settings";
import type { InstalledAddon, RibbonEntry, Snippet } from "../api";
import { useT } from "../i18n";
import { DOCUMENT_LANGUAGES } from "../latex/languages";

export type RibbonActions = {
  openBackstage: () => void;
  setMode: (mode: "visual" | "code") => void;
  formatPainter: (sticky: boolean) => void;
  openSearch: (replace: boolean) => void;
  insertTitlePage: () => void;
  insertFrontmatter: () => void;
  insertPageBreak: () => void;
  insertSectionBreak: () => void;
  insertTable: () => void;
  insertLatexTable: () => void;
  insertImage: () => void;
  insertSubfigures: () => void;
  insertTikz: (environment: "tikzpicture" | "circuitikz") => void;
  insertMath: (inline: boolean) => void;
  insertLink: () => void;
  insertCrossReference: () => void;
  insertFootnote: () => void;
  insertAcronym: () => void;
  insertRawLatex: () => void;
  insertQuantity: () => void;
  insertVerticalSpace: () => void;
  insertEnvironment: (kind: "box" | "columns" | "abstract" | "center" | "minipage" | "custom") => void;
  insertMaketitle: () => void;
  insertAppendix: () => void;
  openSketch: () => void;
  insertCodeListing: () => void;
  setTableStyle: (style: "grid" | "booktabs" | "plain") => void;
  /** enumitem-Marke, z. B. `\alph*)`; null = Standard */
  setListFormat: (label: string | null) => void;
  insertSnippet: (snippet: Snippet) => void;
  insertDirectory: (kind: "contents" | "figures" | "tables" | "acronyms" | "bibliography") => void;
  insertCitation: () => void;
  chooseBibliography: () => void;
  updateSettings: (patch: Partial<DocumentSettings>) => void;
  openDocumentSettings: (section?: string) => void;
  editTableCaption: () => void;
  editImage: () => void;
  replaceImage: () => void;
  compile: () => void;
  openAddons: () => void;
  openShortcuts: () => void;
  openSlides: () => void;
};

export type ViewState = {
  zoom: number;
  paged: boolean;
  showPdf: boolean;
  showOutline: boolean;
  showRuler: boolean;
  compiling: boolean;
};

type Props = {
  editor: Editor | null;
  mode: "visual" | "code";
  actions: RibbonActions;
  settings: DocumentSettings;
  view: ViewState;
  onViewChange: (patch: Partial<ViewState>) => void;
  addons: InstalledAddon[];
  availableFonts: string[];
  formatPainterActive: boolean;
};

type TabId = "start" | "insert" | "layout" | "references" | "view" | "table" | "image" | "addons";

const FONT_SIZES = [8, 9, 10, 10.5, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72];
const LINE_SPACINGS = ["1", "1.15", "1.5", "2", "2.5", "3"];
const TEXT_COLORS = [
  "#000000", "#404040", "#7f7f7f", "#c00000", "#ff0000", "#ffc000", "#ffff00", "#92d050",
  "#00b050", "#00b0f0", "#0070c0", "#002060", "#7030a0", "#1f4e79", "#833c0b", "#375623",
];
const MATH_TEMPLATES = [
  { label: "Bruch", latex: "\\frac{a}{b}" },
  { label: "Wurzel", latex: "\\sqrt{x}" },
  { label: "n-te Wurzel", latex: "\\sqrt[n]{x}" },
  { label: "Summe", latex: "\\sum_{i=1}^{n} x_i" },
  { label: "Produkt", latex: "\\prod_{i=1}^{n} x_i" },
  { label: "Integral", latex: "\\int_{a}^{b} f(x)\\,\\mathrm{d}x" },
  { label: "Grenzwert", latex: "\\lim_{x \\to \\infty} f(x)" },
  { label: "Ableitung", latex: "\\frac{\\mathrm{d}f}{\\mathrm{d}x}" },
  { label: "Partielle Ableitung", latex: "\\frac{\\partial f}{\\partial x}" },
  { label: "Matrix", latex: "\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}" },
  { label: "Determinante", latex: "\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}" },
  { label: "Fallunterscheidung", latex: "f(x) = \\begin{cases} 1 & x > 0 \\\\ 0 & \\text{sonst} \\end{cases}" },
  { label: "Vektor", latex: "\\vec{v} = \\begin{pmatrix} x \\\\ y \\\\ z \\end{pmatrix}" },
  { label: "Komplexe Zahl", latex: "\\underline{Z} = R + \\mathrm{j}X" },
];
const GREEK: Array<[string, string]> = [
  ["α", "\\alpha"], ["β", "\\beta"], ["γ", "\\gamma"], ["δ", "\\delta"], ["ε", "\\varepsilon"], ["ζ", "\\zeta"],
  ["η", "\\eta"], ["θ", "\\theta"], ["λ", "\\lambda"], ["μ", "\\mu"], ["ν", "\\nu"], ["π", "\\pi"],
  ["ρ", "\\rho"], ["σ", "\\sigma"], ["τ", "\\tau"], ["φ", "\\varphi"], ["χ", "\\chi"], ["ψ", "\\psi"],
  ["ω", "\\omega"], ["Γ", "\\Gamma"], ["Δ", "\\Delta"], ["Θ", "\\Theta"], ["Λ", "\\Lambda"], ["Π", "\\Pi"],
  ["Σ", "\\Sigma"], ["Φ", "\\Phi"], ["Ψ", "\\Psi"], ["Ω", "\\Omega"], ["∞", "\\infty"], ["±", "\\pm"],
  ["≤", "\\leq"], ["≥", "\\geq"], ["≠", "\\neq"], ["≈", "\\approx"], ["→", "\\rightarrow"], ["∂", "\\partial"],
];
const HIGHLIGHT_COLORS = ["#ffff00", "#00ff00", "#00ffff", "#ff00ff", "#fff2cc", "#ddebf7", "#e2efda", "#fce4d6", "#d9d9d9", "#f8cbad"];

// ---------------------------------------------------------------- Bausteine

function Group({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="ribbon-group" aria-label={label}>
      <div className="ribbon-group-content">{children}</div>
      <div className="ribbon-group-label">{label}</div>
    </section>
  );
}

function Button({
  label,
  icon,
  onClick,
  active = false,
  disabled = false,
  large = false,
  title,
  onDoubleClick,
}: {
  label: string;
  icon: ReactNode;
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  large?: boolean;
  title?: string;
  onDoubleClick?: () => void;
}) {
  return (
    <button
      type="button"
      className={`ribbon-button${large ? " large" : ""}${active ? " active" : ""}`}
      title={title ?? label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
    >
      {icon}
      {large && <span className="ribbon-button-label">{label}</span>}
    </button>
  );
}

function Stack({ children }: { children: ReactNode }) {
  return <div className="ribbon-stack">{children}</div>;
}

function Row({ children }: { children: ReactNode }) {
  return <div className="ribbon-row">{children}</div>;
}

/** Popover unterhalb einer Schaltfläche (Farben, Menüs). */
function Dropdown({
  label,
  icon,
  children,
  large = false,
  swatch,
}: {
  label: string;
  icon: ReactNode;
  children: (close: () => void) => ReactNode;
  large?: boolean;
  swatch?: string;
}) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpen(false);
    };
    const closeOnResize = () => setOpen(false);
    document.addEventListener("mousedown", close);
    window.addEventListener("resize", closeOnResize);
    return () => {
      document.removeEventListener("mousedown", close);
      window.removeEventListener("resize", closeOnResize);
    };
  }, [open]);
  return (
    <div className="ribbon-dropdown" ref={ref}>
      <button
        type="button"
        className={`ribbon-button with-caret${large ? " large" : ""}${open ? " active" : ""}`}
        title={label}
        aria-label={label}
        aria-expanded={open}
        onMouseDown={(event) => event.preventDefault()}
        onClick={(event) => {
          // Fest positioniert, damit das Menü nicht vom Ribbon (overflow: hidden) abgeschnitten wird.
          const rect = event.currentTarget.getBoundingClientRect();
          setPosition({ top: rect.bottom + 4, left: Math.max(4, Math.min(rect.left, window.innerWidth - 340)) });
          setOpen((value) => !value);
        }}
      >
        <span className="icon-with-swatch">
          {icon}
          {swatch !== undefined && <span className="swatch-bar" style={{ background: swatch || "transparent" }} />}
        </span>
        {large && <span className="ribbon-button-label">{label}</span>}
        <ChevronDown size={12} className="caret" />
      </button>
      {open && (
        <div className="ribbon-popover" role="menu" style={{ top: position.top, left: position.left }} onMouseDown={(event) => event.preventDefault()}>
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  );
}

function ColorGrid({ colors, onPick, automaticLabel }: { colors: string[]; onPick: (color: string | null) => void; automaticLabel: string }) {
  return (
    <div className="color-menu">
      <button type="button" className="menu-item" onClick={() => onPick(null)}>{automaticLabel}</button>
      <div className="color-grid">
        {colors.map((color) => (
          <button key={color} type="button" className="color-cell" style={{ background: color }} title={color} aria-label={color} onClick={() => onPick(color)} />
        ))}
      </div>
      <label className="menu-item color-custom">
        <input type="color" onChange={(event) => onPick(event.currentTarget.value)} />
        …
      </label>
    </div>
  );
}

function MenuItem({ label, onClick, icon, hint }: { label: string; onClick: () => void; icon?: ReactNode; hint?: string }) {
  return (
    <button type="button" className="menu-item" onClick={onClick} title={hint}>
      {icon}
      <span>{label}</span>
      {hint && <small>{hint}</small>}
    </button>
  );
}

// ---------------------------------------------------------------- Ribbon

export function Ribbon({ editor, mode, actions, settings, view, onViewChange, addons, availableFonts, formatPainterActive }: Props) {
  const t = useT();
  const [tab, setTab] = useState<TabId>("start");
  const state = useEditorState({
    editor,
    selector: ({ editor: current }) => {
      if (!current) return null;
      const textStyle = current.getAttributes("textStyle");
      const paragraph = current.isActive("heading") ? current.getAttributes("heading") : current.getAttributes("paragraph");
      return {
        bold: current.isActive("bold"),
        italic: current.isActive("italic"),
        underline: current.isActive("underline"),
        strike: current.isActive("strike"),
        sub: current.isActive("subscript"),
        sup: current.isActive("superscript"),
        bullet: current.isActive("bulletList"),
        ordered: current.isActive("orderedList"),
        quote: current.isActive("blockquote"),
        code: current.isActive("codeBlock"),
        link: current.isActive("link"),
        heading: ([1, 2, 3, 4, 5, 6].find((level) => current.isActive("heading", { level })) ?? 0) as number,
        headingNumbered: current.getAttributes("heading").numbered !== false,
        align: (["left", "center", "right", "justify"].find((value) => current.isActive({ textAlign: value })) ?? "left") as string,
        fontFamily: String(textStyle.fontFamily ?? ""),
        fontSize: String(textStyle.fontSize ?? ""),
        color: String(textStyle.color ?? ""),
        highlight: String(textStyle.backgroundColor ?? ""),
        lineHeight: String(paragraph.lineHeight ?? ""),
        indent: Number(paragraph.indent ?? 0),
        border: Boolean(paragraph.border),
        shading: String(paragraph.paragraphShading ?? ""),
        inTable: current.isActive("table"),
        tableStyle: String(current.getAttributes("table").tableStyle || "grid"),
        tableBreaks: current.getAttributes("table").breakAcrossPages === true,
        listOptions: String(current.getAttributes(current.isActive("orderedList") ? "orderedList" : "bulletList").listOptions ?? ""),
        imageSelected: current.isActive("image"),
        canUndo: current.can().undo(),
        canRedo: current.can().redo(),
      };
    },
  });

  const addonEntries = addons
    .filter((addon) => addon.enabled)
    .flatMap((addon) =>
      addon.manifest.ribbon
        .map((entry: RibbonEntry) => ({ entry, snippet: addon.manifest.snippets.find((snippet) => snippet.id === entry.snippet) }))
        .filter((item): item is { entry: RibbonEntry; snippet: Snippet } => Boolean(item.snippet)),
    );

  const tabs: Array<{ id: TabId; label: string; contextual?: boolean }> = [
    { id: "start", label: t("Start") },
    { id: "insert", label: t("Einfügen") },
    { id: "layout", label: t("Layout") },
    { id: "references", label: t("Referenzen") },
    { id: "view", label: t("Ansicht") },
    ...(addonEntries.length ? [{ id: "addons" as TabId, label: t("Add-ons") }] : []),
    ...(state?.inTable && mode === "visual" ? [{ id: "table" as TabId, label: t("Tabelle"), contextual: true }] : []),
    ...(state?.imageSelected && mode === "visual" ? [{ id: "image" as TabId, label: t("Bild"), contextual: true }] : []),
  ];
  const activeTab = tabs.some((entry) => entry.id === tab) ? tab : "start";

  // Kontextuelle Tabs automatisch anzeigen, wenn eine Tabelle/ein Bild ausgewählt wird.
  const previousContext = useRef({ table: false, image: false });
  useEffect(() => {
    const table = Boolean(state?.inTable);
    const image = Boolean(state?.imageSelected);
    if (image && !previousContext.current.image) setTab("image");
    previousContext.current = { table, image };
  }, [state?.inTable, state?.imageSelected]);

  const chain = () => editor?.chain().focus();
  const visual = mode === "visual" && Boolean(editor);
  const setParagraphAttr = (attrs: Record<string, unknown>) => {
    if (!editor) return;
    const type = editor.isActive("heading") ? "heading" : "paragraph";
    editor.chain().focus().updateAttributes(type, attrs).run();
  };

  const groups: Array<{ id: string; label: string; content: ReactNode }> = [];
  const add = (id: string, label: string, content: ReactNode) => groups.push({ id, label, content });

  if (activeTab === "start") {
    add("clipboard", t("Format"), (
      <Stack>
        <Button
          label={t("Format übertragen")}
          title={t("Format übertragen (Doppelklick: mehrfach)")}
          icon={<Paintbrush size={18} />}
          active={formatPainterActive}
          disabled={!visual}
          onClick={() => actions.formatPainter(false)}
          onDoubleClick={() => actions.formatPainter(true)}
        />
        <Button label={t("Formatierung löschen")} icon={<Eraser size={18} />} disabled={!visual} onClick={() => chain()?.unsetAllMarks().run()} />
      </Stack>
    ));
    add("font", t("Schriftart"), (
      <Stack>
        <Row>
          <select
            className="ribbon-select font-select"
            aria-label={t("Schriftart")}
            disabled={!visual}
            value={state?.fontFamily ?? ""}
            onChange={(event) => {
              const value = event.currentTarget.value;
              if (value) chain()?.setFontFamily(value).run();
              else chain()?.unsetFontFamily().run();
            }}
          >
            <option value="">{`${t("Standard")} (${settings.defaultFontFamily})`}</option>
            {[...new Set([...availableFonts, state?.fontFamily ?? ""])].filter(Boolean).map((font) => (
              <option key={font} value={font} style={{ fontFamily: font }}>{font}</option>
            ))}
          </select>
          <select
            className="ribbon-select size-select"
            aria-label={t("Schriftgröße")}
            disabled={!visual}
            value={state?.fontSize?.replace(/pt$/, "") ?? ""}
            onChange={(event) => {
              const value = event.currentTarget.value;
              if (value) chain()?.setFontSize(`${value}pt`).run();
              else chain()?.unsetFontSize().run();
            }}
          >
            <option value="">{settings.defaultFontSize}</option>
            {FONT_SIZES.map((size) => (
              <option key={size} value={String(size)}>{size}</option>
            ))}
          </select>
        </Row>
        <Row>
          <Button label={t("Fett")} icon={<Bold size={16} />} active={state?.bold} disabled={!visual} onClick={() => chain()?.toggleBold().run()} />
          <Button label={t("Kursiv")} icon={<Italic size={16} />} active={state?.italic} disabled={!visual} onClick={() => chain()?.toggleItalic().run()} />
          <Button label={t("Unterstrichen")} icon={<Underline size={16} />} active={state?.underline} disabled={!visual} onClick={() => chain()?.toggleUnderline().run()} />
          <Button label={t("Durchgestrichen")} icon={<Strikethrough size={16} />} active={state?.strike} disabled={!visual} onClick={() => chain()?.toggleStrike().run()} />
          <Button label={t("Tiefgestellt")} icon={<Subscript size={16} />} active={state?.sub} disabled={!visual} onClick={() => chain()?.toggleSubscript().run()} />
          <Button label={t("Hochgestellt")} icon={<Superscript size={16} />} active={state?.sup} disabled={!visual} onClick={() => chain()?.toggleSuperscript().run()} />
          <Dropdown label={t("Schriftfarbe")} icon={<Baseline size={16} />} swatch={state?.color || "#c00000"}>
            {(close) => (
              <ColorGrid
                colors={TEXT_COLORS}
                automaticLabel={t("Automatisch")}
                onPick={(color) => {
                  if (color) chain()?.setColor(color).updateAttributes("textStyle", { userSetColor: true }).run();
                  else chain()?.unsetColor().run();
                  close();
                }}
              />
            )}
          </Dropdown>
          <Dropdown label={t("Hervorheben")} icon={<Highlighter size={16} />} swatch={state?.highlight || "#ffff00"}>
            {(close) => (
              <ColorGrid
                colors={HIGHLIGHT_COLORS}
                automaticLabel={t("Keine Farbe")}
                onPick={(color) => {
                  if (color) chain()?.setBackgroundColor(color).run();
                  else chain()?.unsetBackgroundColor().run();
                  close();
                }}
              />
            )}
          </Dropdown>
        </Row>
      </Stack>
    ));
    add("paragraph", t("Absatz"), (
      <Stack>
        <Row>
          <Button label={t("Aufzählung")} icon={<List size={16} />} active={state?.bullet} disabled={!visual} onClick={() => chain()?.toggleBulletList().run()} />
          <Button label={t("Nummerierung")} icon={<ListOrdered size={16} />} active={state?.ordered} disabled={!visual} onClick={() => chain()?.toggleOrderedList().run()} />
          <Dropdown label={t("Listenformat")} icon={<span className="list-format-icon">a)</span>}>
            {(close) => (
              <div className="menu-list">
                {[
                  { label: t("Standard"), value: null },
                  { label: "1.  2.  3.", value: "\\arabic*." },
                  { label: "1)  2)  3)", value: "\\arabic*)" },
                  { label: "a)  b)  c)", value: "\\alph*)" },
                  { label: "(a)  (b)  (c)", value: "(\\alph*)" },
                  { label: "A.  B.  C.", value: "\\Alph*." },
                  { label: "i.  ii.  iii.", value: "\\roman*." },
                  { label: "(i)  (ii)  (iii)", value: "(\\roman*)" },
                  { label: "–", value: "--" },
                  { label: "•", value: "\\textbullet" },
                ].map((entry) => (
                  <MenuItem
                    key={entry.label}
                    label={entry.label}
                    hint={entry.value ?? undefined}
                    onClick={() => {
                      actions.setListFormat(entry.value);
                      close();
                    }}
                  />
                ))}
              </div>
            )}
          </Dropdown>
          <Button
            label={t("Einzug verkleinern")}
            icon={<IndentDecrease size={16} />}
            disabled={!visual}
            onClick={() => {
              if (editor?.isActive("listItem")) chain()?.liftListItem("listItem").run();
              else setParagraphAttr({ indent: Math.max(0, (state?.indent ?? 0) - 1) });
            }}
          />
          <Button
            label={t("Einzug vergrößern")}
            icon={<IndentIncrease size={16} />}
            disabled={!visual}
            onClick={() => {
              if (editor?.isActive("listItem")) chain()?.sinkListItem("listItem").run();
              else setParagraphAttr({ indent: Math.min(8, (state?.indent ?? 0) + 1) });
            }}
          />
          <select
            className="ribbon-select spacing-select"
            aria-label={t("Zeilenabstand")}
            title={t("Zeilenabstand")}
            disabled={!visual}
            value={state?.lineHeight ?? ""}
            onChange={(event) => setParagraphAttr({ lineHeight: event.currentTarget.value || null })}
          >
            <option value="">{t("Zeilenabstand")}</option>
            {LINE_SPACINGS.map((value) => (
              <option key={value} value={value}>{value.replace(".", ",")}</option>
            ))}
          </select>
        </Row>
        <Row>
          <Button label={t("Linksbündig")} icon={<AlignLeft size={16} />} active={state?.align === "left"} disabled={!visual} onClick={() => chain()?.setTextAlign("left").run()} />
          <Button label={t("Zentriert")} icon={<AlignCenter size={16} />} active={state?.align === "center"} disabled={!visual} onClick={() => chain()?.setTextAlign("center").run()} />
          <Button label={t("Rechtsbündig")} icon={<AlignRight size={16} />} active={state?.align === "right"} disabled={!visual} onClick={() => chain()?.setTextAlign("right").run()} />
          <Button label={t("Blocksatz")} icon={<AlignJustify size={16} />} active={state?.align === "justify"} disabled={!visual} onClick={() => chain()?.setTextAlign("justify").run()} />
          <Button label={t("Rahmen")} icon={<Square size={16} />} active={state?.border} disabled={!visual} onClick={() => setParagraphAttr({ border: state?.border ? null : "single" })} />
          <Dropdown label={t("Schattierung")} icon={<PaintBucket size={16} />} swatch={state?.shading || "#ddebf7"}>
            {(close) => (
              <ColorGrid
                colors={HIGHLIGHT_COLORS}
                automaticLabel={t("Keine Farbe")}
                onPick={(color) => {
                  setParagraphAttr({ paragraphShading: color });
                  close();
                }}
              />
            )}
          </Dropdown>
        </Row>
      </Stack>
    ));
    add("styles", t("Formatvorlagen"), (
      <div className="style-gallery">
        {[
          { label: t("Standard"), active: !state?.heading && !state?.quote && !state?.code, run: () => chain()?.setParagraph().run(), className: "style-normal" },
          ...[1, 2, 3, 4].map((level) => ({
            label: `${t("Überschrift")} ${level}`,
            active: state?.heading === level,
            run: () => chain()?.toggleHeading({ level: level as 1 | 2 | 3 | 4 }).run(),
            className: `style-h${level}`,
          })),
          { label: t("Zitat"), active: state?.quote, run: () => chain()?.toggleBlockquote().run(), className: "style-quote" },
          { label: t("Code"), active: state?.code, run: () => chain()?.toggleCodeBlock().run(), className: "style-code" },
        ].map((style) => (
          <button
            key={style.label}
            type="button"
            className={`style-item ${style.className}${style.active ? " active" : ""}`}
            disabled={!visual}
            onMouseDown={(event) => event.preventDefault()}
            onClick={style.run}
          >
            {style.label}
          </button>
        ))}
        <button
          type="button"
          className={`style-item style-flag${state?.heading && !state?.headingNumbered ? " active" : ""}`}
          disabled={!visual || !state?.heading}
          title={t("Überschrift ohne Nummer (\\section*)")}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => editor?.chain().focus().updateAttributes("heading", { numbered: !state?.headingNumbered }).run()}
        >
          <Hash size={14} /> {t("Ohne Nummer")}
        </button>
      </div>
    ));
    add("edit", t("Bearbeiten"), (
      <Stack>
        <Button label={t("Suchen")} icon={<Search size={18} />} onClick={() => actions.openSearch(false)} />
        <Button label={t("Ersetzen")} icon={<Replace size={18} />} disabled={!visual} onClick={() => actions.openSearch(true)} />
      </Stack>
    ));
  }

  if (activeTab === "insert") {
    add("pages", t("Seiten"), (
      <>
        <Button large label={t("Titelseite")} icon={<LayoutTemplate size={22} />} disabled={!visual} onClick={actions.insertTitlePage} />
        <Button large label={t("Vorspann/Anhang")} icon={<ScrollText size={22} />} disabled={!visual} onClick={actions.insertFrontmatter} />
        <Stack>
          <Button label={t("Seitenumbruch")} icon={<SeparatorHorizontal size={18} />} disabled={!visual} onClick={actions.insertPageBreak} />
          <Button label={t("Abschnittsumbruch")} icon={<Columns2 size={18} />} disabled={!visual} onClick={actions.insertSectionBreak} />
        </Stack>
      </>
    ));
    add("tables", t("Tabellen"), (
      <>
        <Button large label={t("Tabelle")} icon={<Table2 size={22} />} disabled={!visual} onClick={actions.insertTable} />
        <Button large label={t("LaTeX-Tabelle")} icon={<TableProperties size={22} />} disabled={!visual} onClick={actions.insertLatexTable} />
      </>
    ));
    add("illustrations", t("Illustrationen"), (
      <>
        <Button large label={t("Bild")} icon={<ImagePlus size={22} />} disabled={!visual} onClick={actions.insertImage} />
        <Button large label={t("Bilder nebeneinander")} icon={<Images size={22} />} title={t("Unterabbildungen: mehrere Bilder mit eigenen Beschriftungen (a), (b) … und gemeinsamer Beschriftung")} disabled={!visual} onClick={actions.insertSubfigures} />
        <Button large label={t("Zeichnung")} icon={<PenTool size={22} />} disabled={!visual} onClick={() => actions.insertTikz("tikzpicture")} />
        <Button large label={t("Schaltung")} icon={<CircuitBoard size={22} />} disabled={!visual} onClick={() => actions.insertTikz("circuitikz")} />
        <Button large label={t("Skizze")} icon={<Shapes size={22} />} title={t("Skizzier-Werkzeug: Linien, Formen und Bauteile auf einem Raster zeichnen")} disabled={!visual} onClick={actions.openSketch} />
      </>
    ));
    add("math", t("Formeln"), (
      <>
        <Button large label={t("Formel")} icon={<Sigma size={22} />} disabled={!visual} onClick={() => actions.insertMath(false)} />
        <Button large label={t("Inline-Formel")} icon={<Radical size={22} />} disabled={!visual} onClick={() => actions.insertMath(true)} />
        <Dropdown large label={t("Vorlagen")} icon={<span className="addon-icon">∑</span>}>
          {(close) => (
            <div className="math-template-menu">
              <div className="menu-heading">{t("Formel einfügen")}</div>
              {MATH_TEMPLATES.map((template) => (
                <button
                  key={template.label}
                  type="button"
                  className="menu-item"
                  title={template.latex}
                  disabled={!visual}
                  onClick={() => {
                    actions.insertSnippet({ id: template.label, name: template.label, group: "", kind: "math", latex: template.latex });
                    close();
                  }}
                >
                  <span>{t(template.label)}</span>
                  <small><code>{template.latex.length > 34 ? `${template.latex.slice(0, 34)}…` : template.latex}</code></small>
                </button>
              ))}
              <div className="menu-heading">{t("Griechische Buchstaben")}</div>
              <div className="symbol-grid">
                {GREEK.map(([symbol, command]) => (
                  <button
                    key={command}
                    type="button"
                    className="symbol-cell"
                    title={command}
                    disabled={!visual}
                    onClick={() => {
                      actions.insertSnippet({ id: command, name: command, group: "", kind: "inlineMath", latex: command });
                      close();
                    }}
                  >
                    {symbol}
                  </button>
                ))}
              </div>
            </div>
          )}
        </Dropdown>
      </>
    ));
    add("links", t("Links"), (
      <Stack>
        <Button label={t("Hyperlink")} icon={<Link2 size={18} />} active={state?.link} disabled={!visual} onClick={actions.insertLink} />
        <Button label={t("Querverweis")} icon={<BookMarked size={18} />} disabled={!visual} onClick={actions.insertCrossReference} />
      </Stack>
    ));
    add("text", t("Text"), (
      <Stack>
        <Button label={t("Fußnote")} icon={<StickyNote size={18} />} disabled={!visual} onClick={actions.insertFootnote} />
        <Button label={t("Abkürzung")} icon={<BookA size={18} />} disabled={!visual} onClick={actions.insertAcronym} />
        <Button label={t("LaTeX-Code")} icon={<Braces size={18} />} disabled={!visual} onClick={actions.insertRawLatex} />
      </Stack>
    ));
    add("elements", t("Elemente"), (
      <>
        <Button large label={t("Einheit")} icon={<Ruler size={22} />} title={t("Größe mit Einheit (siunitx), z. B. 4,7 kΩ")} disabled={!visual} onClick={actions.insertQuantity} />
        <Dropdown large label={t("Box")} icon={<SquareDashed size={22} />}>
          {(close) => (
            <div className="menu-list">
              <MenuItem label={t("Farbige Box mit Titel (tcolorbox)")} onClick={() => { actions.insertEnvironment("box"); close(); }} />
              <MenuItem label={t("Mehrspaltiger Bereich (multicols)")} icon={<Columns3 size={16} />} onClick={() => { actions.insertEnvironment("columns"); close(); }} />
              <MenuItem label={t("Zusammenfassung (abstract)")} onClick={() => { actions.insertEnvironment("abstract"); close(); }} />
              <MenuItem label={t("Zentrierter Bereich (center)")} onClick={() => { actions.insertEnvironment("center"); close(); }} />
              <MenuItem label={t("Minipage (Teilbreite)")} onClick={() => { actions.insertEnvironment("minipage"); close(); }} />
              <MenuItem label={t("Eigene Umgebung …")} onClick={() => { actions.insertEnvironment("custom"); close(); }} />
            </div>
          )}
        </Dropdown>
        <Stack>
          <Button label={t("Vertikaler Abstand")} icon={<MoveVertical size={18} />} disabled={!visual} onClick={actions.insertVerticalSpace} />
          <Button label={t("Code-Listing")} icon={<Code2 size={18} />} disabled={!visual} onClick={actions.insertCodeListing} />
          <Button label={t("Titel (\\maketitle)")} icon={<Heading size={18} />} disabled={!visual} onClick={actions.insertMaketitle} />
        </Stack>
        <Stack>
          <Button label={t("Anhang beginnen (\\appendix)")} icon={<Paperclip size={18} />} disabled={!visual} onClick={actions.insertAppendix} />
        </Stack>
      </>
    ));
  }

  if (activeTab === "layout") {
    const marginPresets = [
      { label: t("Normal (2,5 cm)"), value: { top: 25, right: 25, bottom: 25, left: 25 } },
      { label: t("Schmal (1,27 cm)"), value: { top: 12.7, right: 12.7, bottom: 12.7, left: 12.7 } },
      { label: t("Mittel"), value: { top: 25.4, right: 19.1, bottom: 25.4, left: 19.1 } },
      { label: t("Breit"), value: { top: 25.4, right: 50.8, bottom: 25.4, left: 50.8 } },
      { label: t("Abschlussarbeit (2,5 cm)"), value: { top: 25, right: 25, bottom: 25, left: 25 } },
    ];
    add("page", t("Seite einrichten"), (
      <>
        <Dropdown large label={t("Seitenränder")} icon={<Frame size={22} />}>
          {(close) => (
            <div className="menu-list">
              {marginPresets.map((preset) => (
                <MenuItem
                  key={preset.label}
                  label={preset.label}
                  onClick={() => {
                    actions.updateSettings({ margins: preset.value });
                    close();
                  }}
                />
              ))}
              <MenuItem label={t("Benutzerdefiniert …")} onClick={() => { actions.openDocumentSettings("page"); close(); }} />
            </div>
          )}
        </Dropdown>
        <Dropdown large label={t("Ausrichtung")} icon={settings.orientation === "landscape" ? <RectangleHorizontal size={22} /> : <RectangleVertical size={22} />}>
          {(close) => (
            <div className="menu-list">
              <MenuItem label={t("Hochformat")} icon={<RectangleVertical size={16} />} onClick={() => { actions.updateSettings({ orientation: "portrait" }); close(); }} />
              <MenuItem label={t("Querformat")} icon={<RectangleHorizontal size={16} />} onClick={() => { actions.updateSettings({ orientation: "landscape" }); close(); }} />
            </div>
          )}
        </Dropdown>
        <Dropdown large label={t("Format")} icon={<FileText size={22} />}>
          {(close) => (
            <div className="menu-list">
              {([["a4", "DIN A4"], ["a5", "DIN A5"], ["letter", "US Letter"], ["legal", "US Legal"]] as const).map(([value, label]) => (
                <MenuItem key={value} label={label} onClick={() => { actions.updateSettings({ paperFormat: value }); close(); }} />
              ))}
              <MenuItem label={t("Benutzerdefiniert …")} onClick={() => { actions.openDocumentSettings("page"); close(); }} />
            </div>
          )}
        </Dropdown>
        <Dropdown large label={t("Spalten")} icon={<Columns2 size={22} />}>
          {(close) => (
            <div className="menu-list">
              {[1, 2, 3].map((columns) => (
                <MenuItem key={columns} label={`${columns}`} onClick={() => { actions.updateSettings({ columns: columns as 1 | 2 | 3 }); close(); }} />
              ))}
            </div>
          )}
        </Dropdown>
      </>
    ));
    add("document", t("Dokument"), (
      <Stack>
        <Row>
          <select
            className="ribbon-select"
            aria-label={t("Dokumentklasse")}
            value={settings.documentClass}
            onChange={(event) => actions.updateSettings({ documentClass: event.currentTarget.value as DocumentSettings["documentClass"] })}
          >
            <option value="scrreprt">{t("Bericht (mit Kapiteln)")}</option>
            <option value="article">{t("Artikel (ohne Kapitel)")}</option>
          </select>
        </Row>
        <Row>
          <select
            className="ribbon-select"
            aria-label={t("Dokumentsprache")}
            value={settings.language}
            onChange={(event) => actions.updateSettings({ language: event.currentTarget.value as DocumentSettings["language"] })}
          >
            {DOCUMENT_LANGUAGES.map((language) => (
              <option key={language.id} value={language.id}>{language.name}</option>
            ))}
          </select>
        </Row>
        <Row>
          <label className="ribbon-checkbox">
            <input type="checkbox" checked={settings.twoside} onChange={(event) => actions.updateSettings({ twoside: event.currentTarget.checked })} />
            {t("Doppelseitig")}
          </label>
        </Row>
      </Stack>
    ));
    add("pagebackground", t("Seitenhintergrund"), (
      <>
        <Button
          large
          label={t("Kopf-/Fußzeile")}
          icon={<PanelLeft size={22} />}
          active={settings.headerFooter.enabled}
          onClick={() => actions.openDocumentSettings("headerFooter")}
        />
        <Dropdown large label={t("Seitenfarbe")} icon={<PaintBucket size={22} />} swatch={settings.pagePdfColor}>
          {(close) => (
            <ColorGrid
              colors={["#ffffff", "#fffbea", "#f3f6fb", "#eef7ee", "#fdf0f0", "#f2f2f2", "#202124", "#1f2937"]}
              automaticLabel={t("Keine Farbe")}
              onPick={(color) => {
                actions.updateSettings({ pagePdfColor: color ?? "", pageDisplayColor: color ?? "" });
                close();
              }}
            />
          )}
        </Dropdown>
        <Button large label={t("Alle Einstellungen")} icon={<Settings size={22} />} onClick={() => actions.openDocumentSettings()} />
      </>
    ));
  }

  if (activeTab === "references") {
    add("directories", t("Verzeichnisse"), (
      <>
        <Button large label={t("Inhaltsverzeichnis")} icon={<ListTree size={22} />} disabled={!visual} onClick={() => actions.insertDirectory("contents")} />
        <Stack>
          <Button label={t("Abbildungsverzeichnis")} icon={<Images size={18} />} disabled={!visual} onClick={() => actions.insertDirectory("figures")} />
          <Button label={t("Tabellenverzeichnis")} icon={<Table2 size={18} />} disabled={!visual} onClick={() => actions.insertDirectory("tables")} />
          <Button label={t("Abkürzungsverzeichnis")} icon={<BookA size={18} />} disabled={!visual} onClick={() => actions.insertDirectory("acronyms")} />
        </Stack>
      </>
    ));
    add("citations", t("Zitate und Literatur"), (
      <>
        <Button large label={t("Zitat einfügen")} icon={<BookOpen size={22} />} disabled={!visual} onClick={actions.insertCitation} />
        <Button large label={t("Literaturverzeichnis")} icon={<Library size={22} />} disabled={!visual} onClick={() => actions.insertDirectory("bibliography")} />
        <Stack>
          <Button label={t("Literaturdatei (.bib)")} icon={<FileText size={18} />} onClick={actions.chooseBibliography} />
          <select
            className="ribbon-select"
            aria-label={t("Zitierstil")}
            value={settings.bibliography.style}
            onChange={(event) =>
              actions.updateSettings({ bibliography: { ...settings.bibliography, style: event.currentTarget.value as DocumentSettings["bibliography"]["style"] } })
            }
          >
            <option value="ieee">IEEE</option>
            <option value="plainnat">{t("Numerisch")}</option>
            <option value="abbrvnat">{t("Numerisch, abgekürzt")}</option>
            <option value="plainnat-authoryear">{t("Autor-Jahr")}</option>
            <option value="alpha">{t("Alphabetisch")}</option>
          </select>
        </Stack>
      </>
    ));
    add("captions", t("Verweise"), (
      <Stack>
        <Button label={t("Querverweis")} icon={<BookMarked size={18} />} disabled={!visual} onClick={actions.insertCrossReference} />
        <Button label={t("Fußnote")} icon={<StickyNote size={18} />} disabled={!visual} onClick={actions.insertFootnote} />
        <Button label={t("Abkürzung")} icon={<BookA size={18} />} disabled={!visual} onClick={actions.insertAcronym} />
      </Stack>
    ));
  }

  if (activeTab === "view") {
    add("mode", t("Ansicht"), (
      <>
        <Button large label={t("Visuell")} icon={<Eye size={22} />} active={mode === "visual"} onClick={() => actions.setMode("visual")} />
        <Button large label={t("LaTeX-Code")} icon={<Braces size={22} />} active={mode === "code"} onClick={() => actions.setMode("code")} />
      </>
    ));
    add("layoutview", t("Darstellung"), (
      <>
        <Button large label={t("Seitenlayout")} icon={<FileText size={22} />} active={view.paged} disabled={mode !== "visual"} onClick={() => onViewChange({ paged: !view.paged })} />
        <Button large label={t("Navigation")} icon={<PanelLeft size={22} />} active={view.showOutline} onClick={() => onViewChange({ showOutline: !view.showOutline })} />
        <Button large label={t("PDF-Vorschau")} icon={<PanelRight size={22} />} active={view.showPdf} onClick={() => onViewChange({ showPdf: !view.showPdf })} />
        <Button large label={t("Lineal")} icon={<Ruler size={22} />} active={view.showRuler} disabled={mode !== "visual"} onClick={() => onViewChange({ showRuler: !view.showRuler })} />
      </>
    ));
    add("zoom", t("Zoom"), (
      <Stack>
        <Row>
          <Button label={t("Verkleinern")} icon={<ZoomOut size={16} />} onClick={() => onViewChange({ zoom: Math.max(50, view.zoom - 10) })} />
          <span className="zoom-value">{view.zoom}%</span>
          <Button label={t("Vergrößern")} icon={<ZoomIn size={16} />} onClick={() => onViewChange({ zoom: Math.min(200, view.zoom + 10) })} />
        </Row>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={() => onViewChange({ zoom: 100 })}>100 %</button>
        </Row>
      </Stack>
    ));
    add("compile", t("PDF"), (
      <Button large label={view.compiling ? t("Kompiliere …") : t("Kompilieren")} icon={<Play size={22} />} disabled={view.compiling} onClick={actions.compile} />
    ));
    add("extras", t("Extras"), (
      <>
        <Button large label={t("Folien")} title={t("Folien-Editor: Präsentationen frei gestalten (Export als LaTeX-Beamer)")} icon={<Presentation size={22} />} onClick={actions.openSlides} />
        <Button large label={t("Tastenkürzel")} icon={<Keyboard size={22} />} onClick={actions.openShortcuts} />
      </>
    ));
  }

  if (activeTab === "table") {
    add("rows", t("Zeilen und Spalten"), (
      <Stack>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.addRowBefore().run()}>{t("Zeile oberhalb")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.addRowAfter().run()}>{t("Zeile unterhalb")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.deleteRow().run()}>{t("Zeile löschen")}</button>
        </Row>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.addColumnBefore().run()}>{t("Spalte links")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.addColumnAfter().run()}>{t("Spalte rechts")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.deleteColumn().run()}>{t("Spalte löschen")}</button>
        </Row>
      </Stack>
    ));
    add("merge", t("Zellen"), (
      <Stack>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.mergeCells().run()}>{t("Verbinden")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.splitCell().run()}>{t("Teilen")}</button>
        </Row>
        <Row>
          <Button label={t("Zelle links")} icon={<AlignLeft size={16} />} onClick={() => chain()?.setCellAttribute("align", "left").run()} />
          <Button label={t("Zelle zentriert")} icon={<AlignCenter size={16} />} onClick={() => chain()?.setCellAttribute("align", "center").run()} />
          <Button label={t("Zelle rechts")} icon={<AlignRight size={16} />} onClick={() => chain()?.setCellAttribute("align", "right").run()} />
        </Row>
      </Stack>
    ));
    add("tableformat", t("Tabelle"), (
      <Stack>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.toggleHeaderRow().run()}>{t("Kopfzeile")}</button>
          <button type="button" className="ribbon-text-button" onClick={() => chain()?.toggleHeaderColumn().run()}>{t("Kopfspalte")}</button>
        </Row>
        <Row>
          <button type="button" className="ribbon-text-button" onClick={actions.editTableCaption}>{t("Beschriftung …")}</button>
          <Button label={t("Tabelle löschen")} icon={<Trash2 size={16} />} onClick={() => chain()?.deleteTable().run()} />
        </Row>
        <Row>
          <label className="ribbon-checkbox" title={t("Lange Tabellen gehen auf der nächsten Seite weiter (longtable); die Kopfzeile wird wiederholt.")}>
            <input
              type="checkbox"
              checked={Boolean(state?.tableBreaks)}
              onChange={(event) => chain()?.updateAttributes("table", { breakAcrossPages: event.currentTarget.checked || null }).run()}
            />
            {t("Über Seiten umbrechen")}
          </label>
        </Row>
      </Stack>
    ));
    add("tablestyle", t("Stil"), (
      <Stack>
        {([
          ["grid", t("Gitter (alle Linien)")],
          ["booktabs", t("Professionell (booktabs)")],
          ["plain", t("Ohne Linien")],
        ] as const).map(([style, label]) => (
          <label key={style} className="ribbon-radio">
            <input type="radio" name="table-style" checked={state?.tableStyle === style} onChange={() => actions.setTableStyle(style)} />
            {label}
          </label>
        ))}
      </Stack>
    ));
  }

  if (activeTab === "image") {
    const attrs = editor?.getAttributes("image") ?? {};
    add("imagesize", t("Größe"), (
      <Stack>
        <Row>
          <label className="ribbon-label">
            {t("Breite")}
            <input
              type="number"
              className="ribbon-number"
              min={5}
              max={100}
              value={Number(attrs.widthPercent) || 80}
              onChange={(event) => chain()?.updateAttributes("image", { widthPercent: Math.min(100, Math.max(5, Number(event.currentTarget.value))) }).run()}
            />
            %
          </label>
        </Row>
        <Row>
          {[25, 50, 75, 100].map((value) => (
            <button key={value} type="button" className="ribbon-text-button" onClick={() => chain()?.updateAttributes("image", { widthPercent: value }).run()}>{value}%</button>
          ))}
        </Row>
      </Stack>
    ));
    add("imagecaption", t("Beschriftung"), (
      <>
        <Button large label={t("Beschriftung und Platzierung")} icon={<Pilcrow size={22} />} onClick={actions.editImage} />
        <Button large label={t("Bild ersetzen")} icon={<ImagePlus size={22} />} onClick={actions.replaceImage} />
      </>
    ));
  }

  if (activeTab === "addons") {
    const byGroup = new Map<string, typeof addonEntries>();
    for (const item of addonEntries) {
      const list = byGroup.get(item.entry.group) ?? [];
      list.push(item);
      byGroup.set(item.entry.group, list);
    }
    for (const [group, items] of byGroup) {
      add(`addon-${group}`, group, (
        <>
          {items.map(({ entry, snippet }) => (
            <button
              key={`${entry.label}-${snippet.id}`}
              type="button"
              className="ribbon-button large addon-button"
              title={snippet.latex}
              disabled={!visual}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => actions.insertSnippet(snippet)}
            >
              <span className="addon-icon">{entry.icon || "⧉"}</span>
              <span className="ribbon-button-label">{entry.label}</span>
            </button>
          ))}
        </>
      ));
    }
    add("addon-manage", t("Verwalten"), (
      <Button large label={t("Add-ons verwalten")} icon={<Puzzle size={22} />} onClick={actions.openAddons} />
    ));
  }

  return (
    <div className="ribbon">
      <nav className="ribbon-tabs" role="tablist">
        <button type="button" className="ribbon-tab file-tab" onClick={actions.openBackstage}>{t("Datei")}</button>
        {tabs.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={activeTab === entry.id}
            className={`ribbon-tab${activeTab === entry.id ? " active" : ""}${entry.contextual ? " contextual" : ""}`}
            onClick={() => setTab(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </nav>
      <OverflowGroups groups={groups} moreLabel={t("Weitere")} />
    </div>
  );
}

/** Gruppen, die nicht mehr in die Breite passen, wandern ins Menü „Weitere“. */
function OverflowGroups({ groups, moreLabel }: { groups: Array<{ id: string; label: string; content: ReactNode }>; moreLabel: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widths = useRef(new Map<string, number>());
  const [visibleCount, setVisibleCount] = useState(groups.length);
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState({ top: 0, right: 0 });
  const key = groups.map((group) => group.id).join("|");

  useLayoutEffect(() => {
    setVisibleCount(groups.length);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const measure = () => {
      container.querySelectorAll<HTMLElement>(":scope > .ribbon-group-wrapper").forEach((element) => {
        const id = element.dataset.groupId ?? "";
        if (element.offsetWidth > 0) widths.current.set(id, element.offsetWidth);
      });
      const width = (id: string) => (widths.current.get(id) ?? 160) + 4;
      const total = groups.reduce((sum, group) => sum + width(group.id), 0);
      let count = groups.length;
      if (total > container.clientWidth) {
        const available = container.clientWidth - 90;
        let used = 0;
        count = 0;
        for (const group of groups) {
          used += width(group.id);
          if (used > available && count > 0) break;
          count += 1;
        }
      }
      setVisibleCount((current) => (current === count ? current : count));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(container);
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, visibleCount === groups.length]);

  const hidden = groups.slice(visibleCount);
  return (
    <div className="ribbon-content" ref={containerRef}>
      {groups.slice(0, visibleCount).map((group) => (
        <div key={group.id} className="ribbon-group-wrapper" data-group-id={group.id}>
          <Group label={group.label}>{group.content}</Group>
        </div>
      ))}
      {hidden.length > 0 && (
        <div className="ribbon-more">
          <button
            type="button"
            className={`ribbon-button large${open ? " active" : ""}`}
            onClick={(event) => {
              const rect = event.currentTarget.getBoundingClientRect();
              setAnchor({ top: rect.bottom + 4, right: Math.max(4, window.innerWidth - rect.right) });
              setOpen((value) => !value);
            }}
            aria-expanded={open}
          >
            <ChevronDown size={22} />
            <span className="ribbon-button-label">{moreLabel}</span>
          </button>
          {open && (
            <div className="ribbon-more-popover" style={{ top: anchor.top, right: anchor.right }} onMouseLeave={() => setOpen(false)}>
              {hidden.map((group) => (
                <Group key={group.id} label={group.label}>{group.content}</Group>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
