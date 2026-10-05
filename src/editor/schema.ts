/**
 * Schema-Definitionen aller VisuTeX-Knoten ohne React/JSX.
 *
 * Diese Datei wird sowohl von der App (dort werden NodeViews ergänzt) als auch
 * von den Node-Tests (`getSchema(baseExtensions())`) verwendet. Dadurch prüfen
 * die Tests exakt das Schema, in das der LaTeX-Import seine Ergebnisse lädt.
 */
import { Extension, Node, mergeAttributes } from "@tiptap/core";
import type { AnyExtension } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Image from "@tiptap/extension-image";
import { Table, TableCell, TableHeader, TableRow } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import { BackgroundColor, Color, FontFamily, FontSize, TextStyle } from "@tiptap/extension-text-style";
import Superscript from "@tiptap/extension-superscript";
import Subscript from "@tiptap/extension-subscript";

type AttributeSpec = {
  default: unknown;
  parseHTML?: (element: HTMLElement) => unknown;
  renderHTML?: (attributes: Record<string, unknown>) => Record<string, unknown> | null;
  keepOnSplit?: boolean;
};

/** Attribut, das als data-* gespeichert wird. */
function dataAttribute(name: string, fallback: unknown = ""): AttributeSpec {
  const dataName = `data-${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`;
  return {
    default: fallback,
    parseHTML: (element) => {
      const value = element.getAttribute(dataName);
      if (value === null) return fallback;
      if (typeof fallback === "boolean") return value === "true";
      if (typeof fallback === "number") return Number(value);
      return value;
    },
    renderHTML: (attributes) => {
      const value = attributes[name];
      return value === "" || value === null || value === undefined || value === false
        ? {}
        : { [dataName]: String(value) };
    },
  };
}

/** Attribut, das nur im JSON lebt (nicht im DOM), z. B. Originalcode. */
function jsonAttribute(fallback: unknown = null, keepOnSplit = true): AttributeSpec {
  return { default: fallback, renderHTML: () => ({}), parseHTML: () => fallback, keepOnSplit };
}

/** Blöcke, die beim Import ihren Originalcode behalten (siehe Rust `Exporter::block`). */
export const sourceBlockTypes = [
  "paragraph",
  "heading",
  "blockquote",
  "bulletList",
  "orderedList",
  "codeBlock",
  "horizontalRule",
  "image",
  "subfigures",
  "table",
  "mathBlock",
  "tikzBlock",
  "rawLatexBlock",
  "directoryBlock",
  "pageBreak",
  "verticalSpace",
  "environmentBlock",
  "maketitle",
  "appendixMarker",
  "includeBlock",
];

/**
 * Quelltreue: `sourceLatex` (Originalcode), `sourceKey` (Fingerabdruck der daraus
 * erzeugten Ausgabe) und `sourceTight` (im Original ohne Leerzeile angeschlossen).
 * Unveränderte Blöcke schreibt der Export exakt so zurück, wie sie im Original standen.
 */
export const SourceFidelity = Extension.create({
  name: "sourceFidelity",
  addGlobalAttributes() {
    return [
      {
        types: sourceBlockTypes,
        attributes: {
          sourceLatex: jsonAttribute(null, false),
          sourceKey: jsonAttribute(null, false),
          sourceTight: jsonAttribute(null, false),
        },
      },
      {
        types: ["heading"],
        attributes: {
          /** Nicht nummeriert, aber im Inhaltsverzeichnis (\addcontentsline). */
          inToc: jsonAttribute(null),
          tocTitle: jsonAttribute(null),
        },
      },
      {
        types: ["bulletList", "orderedList"],
        attributes: {
          /** enumitem-Optionen, z. B. `label=\alph*), leftmargin=1.5em` */
          listOptions: {
            default: null,
            parseHTML: () => null,
            renderHTML: (attributes: Record<string, unknown>) => listStyleAttributes(attributes.listOptions),
          },
          /** description statt itemize */
          listEnvironment: jsonAttribute(null),
        },
      },
      {
        types: ["listItem"],
        attributes: {
          /** Eigene Marke: \item[a)] */
          itemLabel: {
            default: null,
            parseHTML: (element: HTMLElement) => element.getAttribute("data-item-label"),
            renderHTML: (attributes: Record<string, unknown>) =>
              attributes.itemLabel ? { "data-item-label": String(attributes.itemLabel) } : {},
            keepOnSplit: false,
          },
        },
      },
      {
        types: ["codeBlock"],
        attributes: {
          /** verbatim | lstlisting | minted | Verbatim */
          environment: jsonAttribute(null),
          listingOptions: jsonAttribute(null),
        },
      },
    ];
  },
});

/** Zähler-Stil einer enumitem-Marke (`\alph*)` → lower-alpha mit „)“). */
export function listLabelStyle(options: unknown): { style: string; prefix: string; suffix: string; bold: boolean } | null {
  if (typeof options !== "string") return null;
  const match = /(?:^|,)\s*label\s*=\s*(\{[^]*\}|[^,]*)/.exec(options);
  if (!match) return null;
  let label = match[1].trim();
  if (label.startsWith("{") && label.endsWith("}")) label = label.slice(1, -1);
  const bold = /\\textbf|\\bfseries/.test(label);
  label = label.replace(/\\textbf\{([^]*)\}/, "$1").replace(/\\bfseries\s*/, "");
  const counters: Array<[RegExp, string]> = [
    [/\\arabic\*|\\arabic\{enum[iv]+\}|\\theenum[iv]+/, "decimal"],
    [/\\alph\*|\\alph\{enum[iv]+\}/, "lower-alpha"],
    [/\\Alph\*|\\Alph\{enum[iv]+\}/, "upper-alpha"],
    [/\\roman\*|\\roman\{enum[iv]+\}/, "lower-roman"],
    [/\\Roman\*|\\Roman\{enum[iv]+\}/, "upper-roman"],
  ];
  for (const [pattern, style] of counters) {
    const found = pattern.exec(label);
    if (found) {
      return { style, prefix: label.slice(0, found.index), suffix: label.slice(found.index + found[0].length), bold };
    }
  }
  // Feste Marke (z. B. „–“ oder „$\bullet$“)
  const fixed = label.replace(/\$\\bullet\$|\\textbullet/g, "•").replace(/\$-\$|--/g, "–").replace(/\\[a-zA-Z]+|[{}$]/g, "");
  return fixed ? { style: "none", prefix: fixed, suffix: "", bold } : null;
}

const cssString = (text: string) => `"${text.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;

function listStyleAttributes(options: unknown): Record<string, string> {
  const label = listLabelStyle(options);
  if (!label) return typeof options === "string" && options ? { "data-list-options": options } : {};
  return {
    "data-list-options": String(options),
    "data-label-format": "",
    style: `--vtx-label-style: ${label.style}; --vtx-label-prefix: ${cssString(label.prefix)}; --vtx-label-suffix: ${cssString(label.suffix)}; --vtx-label-weight: ${label.bold ? 700 : 400}`,
  };
}

/** Absatz-/Überschriftenattribute: Zeilenabstand, Einzug, Rahmen, Schattierung. */
export const ParagraphFormat = Extension.create({
  name: "paragraphFormat",
  addGlobalAttributes() {
    return [
      {
        types: ["paragraph", "heading"],
        attributes: {
          lineHeight: {
            default: null,
            parseHTML: (element: HTMLElement) => {
              const value = element.style.lineHeight;
              return /^\d*\.?\d+$/.test(value) ? value : null;
            },
            renderHTML: (attributes: Record<string, unknown>) =>
              attributes.lineHeight ? { style: `--vtx-line-height: ${attributes.lineHeight}` } : {},
          },
          indent: {
            default: 0,
            parseHTML: (element: HTMLElement) => Number(element.getAttribute("data-indent") ?? 0) || 0,
            renderHTML: (attributes: Record<string, unknown>) =>
              Number(attributes.indent) > 0
                ? { "data-indent": String(attributes.indent), style: `margin-left: ${Number(attributes.indent) * 1.25}cm` }
                : {},
          },
          border: {
            default: null,
            parseHTML: (element: HTMLElement) => element.getAttribute("data-border") ? "single" : null,
            renderHTML: (attributes: Record<string, unknown>) =>
              attributes.border ? { "data-border": "single" } : {},
          },
          paragraphShading: {
            default: null,
            parseHTML: (element: HTMLElement) => element.getAttribute("data-shading"),
            renderHTML: (attributes: Record<string, unknown>) =>
              typeof attributes.paragraphShading === "string" && /^#[0-9a-f]{6}$/i.test(attributes.paragraphShading)
                ? { "data-shading": attributes.paragraphShading, style: `background-color: ${attributes.paragraphShading}` }
                : {},
          },
        },
      },
      {
        types: ["heading"],
        attributes: {
          numbered: {
            default: true,
            parseHTML: (element: HTMLElement) => element.getAttribute("data-numbered") !== "false",
            renderHTML: (attributes: Record<string, unknown>) =>
              attributes.numbered === false ? { "data-numbered": "false" } : {},
          },
          label: dataAttribute("label"),
        },
      },
    ];
  },
});

export const DocumentTextStyle = TextStyle.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      userSetColor: {
        default: false,
        parseHTML: (element: HTMLElement) => element.getAttribute("data-user-set-color") === "true",
        renderHTML: (attributes: Record<string, unknown>) =>
          attributes.userSetColor ? { "data-user-set-color": "true" } : {},
      },
    };
  },
});

export const MathBlock = Node.create({
  name: "mathBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      latex: dataAttribute("latex"),
      numbered: dataAttribute("numbered", false),
      label: dataAttribute("label"),
      /** equation | align | gather | multline | eqnarray */
      environment: dataAttribute("environment", "equation"),
    };
  },
  parseHTML() {
    return [{ tag: "div[data-math-block]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-math-block": "" })];
  },
});

export const InlineMath = Node.create({
  name: "inlineMath",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return { latex: dataAttribute("latex") };
  },
  parseHTML() {
    return [{ tag: "span[data-inline-math]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-inline-math": "" })];
  },
});

/**
 * Angaben einer Gleitumgebung, die aus fremden Dokumenten übernommen werden:
 * Platzierung, `\centering`, Beschriftung oben, LaTeX-Beschriftung (mit Formeln),
 * Kurzbeschriftung fürs Verzeichnis, `wrapper` (float/center/none).
 */
function floatAttributes(): Record<string, AttributeSpec> {
  return {
    wrapper: jsonAttribute(null),
    centered: jsonAttribute(null),
    captionAbove: jsonAttribute(null),
    captionLatex: jsonAttribute(null),
    shortCaption: jsonAttribute(null),
  };
}

export const TikzBlock = Node.create({
  name: "tikzBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      code: { ...dataAttribute("tikzCode"), default: "\\draw[thick] (0,0) -- (3,1);" },
      /** tikzpicture | circuitikz */
      environment: dataAttribute("environment", "tikzpicture"),
      options: dataAttribute("options"),
      caption: dataAttribute("caption"),
      label: dataAttribute("label"),
      placement: jsonAttribute(null),
      ...floatAttributes(),
      /** Gerenderte Vorschau (PNG als data-URL) und der Code, zu dem sie gehört. */
      preview: { default: "", renderHTML: () => ({}) },
      previewCode: { default: "", renderHTML: () => ({}) },
    };
  },
  parseHTML() {
    return [{ tag: "div[data-tikz-block]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-tikz-block": "" })];
  },
});

export type FrontmatterKind = "confidentiality" | "dedication" | "summary" | "abstract" | "preface" | "appendix" | "declaration";

export const frontmatterTitles: Record<FrontmatterKind, string> = {
  confidentiality: "Sperrvermerk",
  dedication: "Widmung",
  summary: "Kurzreferat",
  abstract: "Abstract",
  preface: "Vorwort",
  appendix: "Anhang",
  declaration: "Eidesstattliche Erklärung",
};

export const frontmatterKinds = Object.keys(frontmatterTitles) as FrontmatterKind[];

/** Vorspann-/Nachspannteil mit eigenem Titel und frei editierbarem Inhalt. */
export const FrontmatterBlock = Node.create({
  name: "frontmatterBlock",
  group: "block",
  content: "block+",
  defining: true,
  isolating: true,
  draggable: true,
  addAttributes() {
    return {
      kind: dataAttribute("frontmatterKind", "abstract"),
      title: dataAttribute("title", "Abstract"),
      /** Eintrag im Inhaltsverzeichnis */
      inToc: dataAttribute("inToc", false),
    };
  },
  parseHTML() {
    return [{ tag: "section[data-frontmatter]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["section", mergeAttributes(HTMLAttributes, { "data-frontmatter": "" }), 0];
  },
});

export const titlePageFields = [
  "title",
  "subtitle",
  "thesisType",
  "degreeIntro",
  "degree",
  "institution",
  "program",
  "authorLabel",
  "author",
  "placeDate",
  "supervisorLabel",
  "supervisor",
  "logoPath",
] as const;
export type TitlePageField = typeof titlePageFields[number];

export const TitlePage = Node.create({
  name: "titlePage",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    const attributes: Record<string, AttributeSpec> = {};
    for (const field of titlePageFields) attributes[field] = dataAttribute(field);
    attributes.logoSrc = { default: "", renderHTML: () => ({}) };
    return attributes;
  },
  parseHTML() {
    return [{ tag: "div[data-title-page]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-title-page": "" })];
  },
});

export type DirectoryKind = "contents" | "figures" | "tables" | "acronyms" | "bibliography";

export const DirectoryBlock = Node.create({
  name: "directoryBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      kind: {
        default: "contents",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-directory-list") ?? "contents",
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-directory-list": String(attributes.kind) }),
      },
      /** Eintrag im Inhaltsverzeichnis (\phantomsection\addcontentsline) */
      inToc: jsonAttribute(null),
      tocTitle: jsonAttribute(null),
      tocLevel: jsonAttribute(null),
      /** Literaturverzeichnis mit biblatex: printbibliography */
      command: jsonAttribute(null),
      options: jsonAttribute(null),
    };
  },
  parseHTML() {
    return [{ tag: "div[data-directory-list]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", HTMLAttributes];
  },
});

export const RawLatexBlock = Node.create({
  name: "rawLatexBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      rawLatex: dataAttribute("rawLatex"),
      missingResources: { default: [], renderHTML: () => ({}), parseHTML: () => [] },
    };
  },
  parseHTML() {
    return [{ tag: "div[data-raw-latex]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { class: "raw-latex-block" })];
  },
});

export const RawLatexInline = Node.create({
  name: "rawLatexInline",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return { latex: dataAttribute("rawInline") };
  },
  parseHTML() {
    return [{ tag: "span[data-raw-inline]" }];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["span", mergeAttributes(HTMLAttributes, { class: "raw-latex-inline" }), String(node.attrs.latex)];
  },
});

export const PageBreak = Node.create({
  name: "pageBreak",
  group: "block",
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      breakType: dataAttribute("breakType", "page"),
      columns: dataAttribute("columns", 1),
      orientation: dataAttribute("orientation", "keep"),
    };
  },
  parseHTML() {
    return [{ tag: "div[data-page-break]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-page-break": "", class: "document-break" })];
  },
});

export const Citation = Node.create({
  name: "citation",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      /** Kommagetrennte BibTeX-Schlüssel */
      keys: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-citation") ?? "",
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-citation": String(attributes.keys ?? "") }),
      },
      label: dataAttribute("label"),
      prenote: dataAttribute("prenote"),
      postnote: dataAttribute("postnote"),
      /** cite | citep | citet */
      command: dataAttribute("command", "cite"),
    };
  },
  parseHTML() {
    return [{ tag: "span[data-citation]" }];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["span", mergeAttributes(HTMLAttributes, { class: "citation-chip" }), `[${node.attrs.label || node.attrs.keys}]`];
  },
});

export const Footnote = Node.create({
  name: "footnote",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      text: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-footnote") ?? "",
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-footnote": String(attributes.text ?? "") }),
      },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-footnote]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { class: "footnote-chip" })];
  },
});

export const AcronymNode = Node.create({
  name: "acronym",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      key: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-acronym") ?? "",
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-acronym": String(attributes.key ?? "") }),
      },
      short: dataAttribute("short"),
      long: dataAttribute("long"),
      /** ac | acs | acl | acf | acp */
      command: dataAttribute("command", "ac"),
    };
  },
  parseHTML() {
    return [{ tag: "span[data-acronym]" }];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["span", mergeAttributes(HTMLAttributes, { class: "acronym-chip" }), String(node.attrs.short || node.attrs.key)];
  },
});

export const CrossReference = Node.create({
  name: "crossReference",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      label: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-reference") ?? "",
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-reference": String(attributes.label ?? "") }),
      },
      /** ref | pageref | eqref | autoref */
      kind: {
        default: "ref",
        parseHTML: (element: HTMLElement) => {
          const kind = element.getAttribute("data-reference-kind");
          if (kind) return kind;
          return element.getAttribute("data-reference-type") === "page" ? "pageref" : "ref";
        },
        renderHTML: (attributes: Record<string, unknown>) => ({ "data-reference-kind": String(attributes.kind ?? "ref") }),
      },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-reference]" }];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["span", mergeAttributes(HTMLAttributes, { class: "inline-reference" }), `\\${node.attrs.kind}{${node.attrs.label}}`];
  },
});

export const DocumentImage = Image.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      latexPath: dataAttribute("latexPath"),
      caption: dataAttribute("caption"),
      label: dataAttribute("label"),
      /** Breite in Prozent der Zeilenbreite (1–100). */
      widthPercent: dataAttribute("widthPercent", 80),
      /** Unveränderte \includegraphics-Optionen aus importiertem Code (haben Vorrang). */
      graphicsOptions: dataAttribute("graphicsOptions"),
      /** htbp | H | t | b | here(center, nicht gleitend) */
      placement: dataAttribute("placement", "htbp"),
      missingResource: dataAttribute("missingResource"),
      ...floatAttributes(),
    };
  },
}).configure({ allowBase64: true });

/** Ein Bild innerhalb von `subfigures` (Felder wie beim Bildknoten). */
export type SubfigureItem = {
  src?: string | null;
  alt?: string;
  title?: string;
  latexPath?: string;
  caption?: string;
  label?: string;
  captionLatex?: boolean;
  /** Bildbreite in Prozent der Unterabbildung */
  widthPercent?: number;
  /** Breite der Unterabbildung in Prozent der Zeilenbreite */
  boxPercent?: number;
  /** vertikale Ausrichtung: t (Standard), c, b */
  position?: string;
  graphicsOptions?: string;
  centered?: boolean;
};

/** Standardbreite je Unterabbildung in Prozent (wie Rust `subfigure_default_share`). */
export function subfigureDefaultPercent(count: number): number {
  return Math.floor((0.96 / Math.max(1, count)) * 100);
}

/** Mehrere Bilder nebeneinander mit eigenen Unterbeschriftungen (subcaption). */
export const Subfigures = Node.create({
  name: "subfigures",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      items: jsonAttribute([]),
      caption: dataAttribute("caption"),
      label: dataAttribute("label"),
      placement: jsonAttribute(null),
      ...floatAttributes(),
    };
  },
  parseHTML() {
    return [{ tag: "figure[data-subfigures]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["figure", mergeAttributes(HTMLAttributes, { "data-subfigures": "" })];
  },
});

export const DocumentTable = Table.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      caption: dataAttribute("caption"),
      label: dataAttribute("label"),
      placement: jsonAttribute(null),
      ...floatAttributes(),
      /** grid (Gitter) | booktabs | plain (ohne Linien) */
      tableStyle: {
        default: null,
        parseHTML: (element: HTMLElement) => element.getAttribute("data-table-style"),
        renderHTML: (attributes: Record<string, unknown>) =>
          attributes.tableStyle ? { "data-table-style": String(attributes.tableStyle) } : {},
      },
      /** Übernommene Spaltendefinition, Umgebung und Breite (tabular/tabularx) */
      columnSpec: jsonAttribute(null),
      tableEnvironment: jsonAttribute(null),
      tableWidth: jsonAttribute(null),
      /** Linien zwischen den Zeilen (\toprule, \hline …) */
      rowRules: jsonAttribute(null),
      /** \small, \footnotesize … */
      fontSize: jsonAttribute(null),
      arrayStretch: jsonAttribute(null),
      /** Über mehrere Seiten umbrechen (longtable) – Seitenansicht teilt zeilenweise */
      breakAcrossPages: {
        default: null,
        parseHTML: (element: HTMLElement) => (element.getAttribute("data-break-across-pages") === "true" ? true : null),
        renderHTML: (attributes: Record<string, unknown>) =>
          attributes.breakAcrossPages ? { "data-break-across-pages": "true" } : {},
      },
    };
  },
});

/** siunitx-Größe: \SI{1}{\mega\ohm}, \qty, \si, \num, Bereiche. */
export const Quantity = Node.create({
  name: "quantity",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      command: dataAttribute("quantityCommand", "SI"),
      value: dataAttribute("value"),
      value2: dataAttribute("value2"),
      unit: dataAttribute("unit"),
      options: dataAttribute("options"),
    };
  },
  parseHTML() {
    return [{ tag: "span[data-quantity]" }];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-quantity": "", class: "quantity-chip" }), `${node.attrs.value} ${node.attrs.unit}`];
  },
});

/** Vertikaler Abstand: \vspace{1cm}, \vspace*{…}, \medskip, \bigskip, \smallskip, \vfill. */
export const VerticalSpace = Node.create({
  name: "verticalSpace",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      command: dataAttribute("spaceCommand", "vspace"),
      size: dataAttribute("size", "1cm"),
    };
  },
  parseHTML() {
    return [{ tag: "div[data-vertical-space]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-vertical-space": "" })];
  },
});

/**
 * Umgebung mit bearbeitbarem Inhalt: tcolorbox, multicols, abstract, minipage,
 * center, eigene Boxen aus der Präambel (\newtcolorbox, \newenvironment) …
 * `args` sind die Argumente direkt nach `\begin{name}` (z. B. `[title=Hinweis]`).
 */
export const EnvironmentBlock = Node.create({
  name: "environmentBlock",
  group: "block",
  content: "block+",
  defining: true,
  draggable: true,
  addAttributes() {
    return {
      name: dataAttribute("environmentName", "tcolorbox"),
      args: dataAttribute("environmentArgs"),
    };
  },
  parseHTML() {
    return [{ tag: "div[data-environment-block]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-environment-block": "" }), 0];
  },
});

/** \maketitle (Titel, Autor, Datum aus der Präambel bzw. den Dokumenteigenschaften). */
export const Maketitle = Node.create({
  name: "maketitle",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  parseHTML() {
    return [{ tag: "div[data-maketitle]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-maketitle": "" })];
  },
});

/** \appendix – folgende Kapitel/Abschnitte werden mit Buchstaben nummeriert. */
export const AppendixMarker = Node.create({
  name: "appendixMarker",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  parseHTML() {
    return [{ tag: "div[data-appendix-marker]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-appendix-marker": "" })];
  },
});

/** Inhalt einer eingebundenen Datei (\input/\include) eines mehrteiligen Projekts. */
export const IncludeBlock = Node.create({
  name: "includeBlock",
  group: "block",
  content: "block+",
  defining: true,
  isolating: true,
  addAttributes() {
    return {
      file: dataAttribute("includeFile"),
      /** input | include | subfile */
      command: dataAttribute("includeCommand", "input"),
      /** \subfile: Präambel/Schluss der Teildatei (unverändert zurückgeschrieben) */
      preamble: jsonAttribute(null),
      postamble: jsonAttribute(null),
    };
  },
  parseHTML() {
    return [{ tag: "section[data-include-block]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["section", mergeAttributes(HTMLAttributes, { "data-include-block": "" }), 0];
  },
});

const cellAlignment: AttributeSpec = {
  default: null,
  parseHTML: (element) => {
    const value = element.style.textAlign || element.getAttribute("data-align");
    return value === "center" || value === "right" || value === "left" ? value : null;
  },
  renderHTML: (attributes) =>
    attributes.align ? { "data-align": String(attributes.align), style: `text-align: ${attributes.align}` } : {},
};

export const DocumentTableCell = TableCell.extend({
  addAttributes() {
    return { ...this.parent?.(), align: cellAlignment };
  },
});

export const DocumentTableHeader = TableHeader.extend({
  addAttributes() {
    return { ...this.parent?.(), align: cellAlignment };
  },
});

/**
 * Basiserweiterungen. Die App ersetzt einzelne Knoten über `overrides` durch
 * Varianten mit React-NodeViews (gleicher Name ⇒ gleiches Schema).
 */
export function baseExtensions(overrides: Record<string, AnyExtension> = {}): AnyExtension[] {
  const extensions: AnyExtension[] = [
    StarterKit.configure({
      heading: { levels: [1, 2, 3, 4, 5, 6] },
      link: { openOnClick: false, autolink: true, protocols: ["http", "https", "mailto"] },
    }),
    ParagraphFormat,
    DocumentTextStyle,
    Color,
    BackgroundColor,
    FontFamily,
    FontSize,
    TextAlign.configure({ types: ["heading", "paragraph"] }),
    Superscript,
    Subscript,
    DocumentImage,
    Subfigures,
    DocumentTable.configure({ resizable: true }),
    TableRow,
    DocumentTableHeader,
    DocumentTableCell,
    MathBlock,
    InlineMath,
    TikzBlock,
    FrontmatterBlock,
    TitlePage,
    DirectoryBlock,
    RawLatexBlock,
    RawLatexInline,
    PageBreak,
    Citation,
    Footnote,
    AcronymNode,
    CrossReference,
    Quantity,
    VerticalSpace,
    EnvironmentBlock,
    Maketitle,
    AppendixMarker,
    IncludeBlock,
    SourceFidelity,
  ];
  return extensions.map((extension) => overrides[extension.name] ?? extension);
}
