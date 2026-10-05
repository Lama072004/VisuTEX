/**
 * Seitenansicht wie in Word: Der Editor misst die Blöcke und fügt vor Blöcken,
 * die eine Seitengrenze überschreiten, unsichtbare Abstandshalter (Widgets) ein.
 * Lange Absätze werden zeilengenau geteilt (Inline-Widget mit `display:block`).
 * Die Seiten selbst (weiße Blätter, Kopf-/Fußzeile) zeichnet die App hinter dem
 * Editor anhand von `pageCount`.
 *
 * Ablauf einer Messung (entprellt): Abstandshalter entfernen → natürliche
 * Positionen messen → Umbrüche berechnen → Abstandshalter setzen. Beides
 * geschieht synchron im selben Frame, daher ohne Flackern.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState, Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import { translate } from "../i18n";
import { getRuntime } from "../state/runtime";

export type PageGeometry = {
  enabled: boolean;
  /** Alle Maße in CSS-Pixeln. */
  pageHeight: number;
  marginTop: number;
  marginBottom: number;
  gap: number;
  /** Kapitel (Überschrift 1) beginnen auf neuer Seite (Bericht/scrreprt). */
  chapterNewPage: boolean;
};

export type PageInfo = { chapter: string; chapterStart?: boolean; titlePage?: boolean };

/**
 * Umbruch (Abstandshalter vor `pos`) bzw. mit `oversize` (Knotengröße) ein Hinweis,
 * dass ein unteilbarer Block höher als eine Seite ist.
 */
type Break = { pos: number; height: number; inline: boolean; oversize?: number };

type PaginationState = {
  breaks: Break[];
  decorations: DecorationSet;
};

export const paginationKey = new PluginKey<PaginationState>("vtxPagination");

const containerTypes = new Set(["bulletList", "orderedList", "listItem", "blockquote", "frontmatterBlock", "taskList", "taskItem"]);
const splittableTypes = new Set(["paragraph", "codeBlock"]);

function spacer(height: number, inline: boolean, row = false): HTMLElement {
  if (row) {
    // Umbruch zwischen Tabellenzeilen: eine leere Zeile über alle Spalten
    const tr = document.createElement("tr");
    tr.className = "vtx-page-gap vtx-page-gap-row";
    tr.setAttribute("contenteditable", "false");
    tr.setAttribute("aria-hidden", "true");
    const td = document.createElement("td");
    td.colSpan = 1000;
    td.style.height = `${height}px`;
    tr.appendChild(td);
    return tr;
  }
  const element = document.createElement(inline ? "span" : "div");
  element.className = inline ? "vtx-page-gap vtx-page-gap-inline" : "vtx-page-gap";
  element.style.height = `${height}px`;
  element.setAttribute("contenteditable", "false");
  element.setAttribute("aria-hidden", "true");
  return element;
}

function oversizeMessage(node: PMNode | null): string {
  const language = getRuntime().language;
  return node?.type.name === "table"
    ? translate(language, "Höher als eine Seite – im PDF würde die Tabelle abgeschnitten. Tabelle → „Über Seiten umbrechen“ aktivieren.")
    : translate(language, "Höher als eine Seite – im PDF läuft dieser Block über den Seitenrand hinaus.");
}

function buildDecorations(doc: PMNode, breaks: Break[]): DecorationSet {
  return DecorationSet.create(
    doc,
    breaks.map((entry) => {
      if (entry.oversize) {
        return Decoration.node(entry.pos, entry.pos + entry.oversize, {
          class: "vtx-oversize",
          "data-oversize": oversizeMessage(doc.nodeAt(entry.pos)),
        });
      }
      const row = doc.nodeAt(entry.pos)?.type.name === "tableRow";
      return Decoration.widget(entry.pos, () => spacer(entry.height, entry.inline, row), {
        side: -1,
        key: `gap-${entry.pos}-${Math.round(entry.height)}-${entry.inline ? "i" : row ? "r" : "b"}`,
        ignoreSelection: true,
      });
    }),
  );
}

type Container = { pos: number; node: PMNode; element: HTMLElement; depth: number };

/**
 * Messeinheit: ein Block, der nicht weiter zerlegt wird. `opens`/`closes`
 * nennen die Container (Listen, Vorspann-Teile …), die mit dieser Einheit
 * beginnen bzw. enden – Umbrüche vor dem ersten Kind werden vor den Container gesetzt.
 */
type Unit = { pos: number; node: PMNode; element: HTMLElement; depth: number; opens: Container[]; closes: Container[] };

function collectUnits(view: EditorView): Unit[] {
  const units: Unit[] = [];
  const visit = (parent: PMNode, offset: number, depth: number, opening: Container[]): Unit | undefined => {
    let last: Unit | undefined;
    let first = true;
    parent.forEach((child, childOffset) => {
      const pos = offset + childOffset;
      const opens = first ? opening : [];
      first = false;
      const dom = view.nodeDOM(pos);
      // Tabellen mit „Über Seiten umbrechen“ werden zeilenweise umbrochen (longtable).
      const container = containerTypes.has(child.type.name) || (child.type.name === "table" && child.attrs.breakAcrossPages === true);
      if (container && child.childCount > 0) {
        const container = dom instanceof HTMLElement ? { pos, node: child, element: dom, depth } : null;
        const inner = visit(child, pos + 1, depth + 1, container ? [...opens, container] : opens);
        if (inner && container) inner.closes.push(container);
        if (inner) last = inner;
        return;
      }
      if (dom instanceof HTMLElement) {
        last = { pos, node: child, element: dom, depth, opens, closes: [] };
        units.push(last);
      }
    });
    return last;
  };
  visit(view.state.doc, 0, 0, []);
  return units;
}

type Measured = Unit & { top: number; bottom: number; anchorPos: number; anchorTop: number };

const isFrontmatter = (container: Container) => container.depth === 0 && container.node.type.name === "frontmatterBlock";

/**
 * Berechnet Umbrüche aus der natürlichen Lage (ohne Abstandshalter).
 * Gibt außerdem Seitenzahl und Kapitel je Seite zurück.
 */
function computeBreaks(view: EditorView, geometry: PageGeometry): { breaks: Break[]; pages: PageInfo[] } {
  const root = view.dom as HTMLElement;
  const rootRect = root.getBoundingClientRect();
  const scale = root.offsetHeight > 0 ? rootRect.height / root.offsetHeight : 1;
  const toLocal = (clientY: number) => (clientY - rootRect.top) / (scale || 1);
  const contentHeight = Math.max(50, geometry.pageHeight - geometry.marginTop - geometry.marginBottom);
  const pitch = geometry.pageHeight + geometry.gap;

  const units: Measured[] = collectUnits(view).map((unit) => {
    const rect = unit.element.getBoundingClientRect();
    const top = toLocal(rect.top);
    const anchor = unit.opens[0];
    // Tabellenbeschriftung (Widget direkt nach der Tabelle) gehört zur Einheit.
    const following = unit.element.nextElementSibling;
    const bottom = following?.classList.contains("table-caption") ? following.getBoundingClientRect().bottom : rect.bottom;
    return {
      ...unit,
      top,
      bottom: toLocal(bottom),
      anchorPos: anchor ? anchor.pos : unit.pos,
      anchorTop: anchor ? toLocal(anchor.element.getBoundingClientRect().top) : top,
    };
  });

  const breaks: Break[] = [];
  const pages: PageInfo[] = [{ chapter: "" }];
  let page = 0;
  let shift = 0;
  const pageStart = (index: number) => index * pitch + geometry.marginTop;
  const pageEnd = (index: number) => pageStart(index) + contentHeight;
  let forceBreak = false;
  let previous: Measured | null = null;

  const newPage = () => {
    page += 1;
    pages.push({ chapter: pages[pages.length - 1]?.chapter ?? "" });
  };

  const insertBreak = (pos: number, naturalTop: number) => {
    const height = pageStart(page + 1) - (naturalTop + shift);
    if (height <= 0 || breaks.some((entry) => entry.pos === pos && !entry.inline && !entry.oversize)) return false;
    breaks.push({ pos, height, inline: false });
    shift += height;
    newPage();
    return true;
  };

  const breakBefore = (unit: Measured, allowKeepWithHeading: boolean) => {
    // Überschrift nicht allein am Seitenende lassen.
    if (
      allowKeepWithHeading &&
      previous &&
      previous.node.type.name === "heading" &&
      previous.anchorTop + shift > pageStart(page) + 1 &&
      previous.anchorTop + shift < pageEnd(page)
    ) {
      if (insertBreak(previous.anchorPos, previous.anchorTop)) return;
    }
    insertBreak(unit.anchorPos, unit.anchorTop);
  };

  for (const unit of units) {
    const type = unit.node.type.name;
    const isChapter = type === "heading" && unit.node.attrs.level === 1;
    const startsPage =
      forceBreak ||
      type === "titlePage" ||
      (unit.depth === 0 && type === "directoryBlock") ||
      unit.opens.some(isFrontmatter) ||
      (geometry.chapterNewPage && isChapter && unit.depth === 0);
    forceBreak = false;

    if (startsPage && previous && unit.anchorTop + shift > pageStart(page) + 2) {
      breakBefore(unit, false);
    }
    if (isChapter) {
      pages[page].chapter = unit.node.textContent;
      if (unit.depth === 0 && Math.abs(unit.top + shift - pageStart(page)) < 4) pages[page].chapterStart = true;
    }
    if (type === "titlePage") pages[page].titlePage = true;

    let top = unit.top + shift;
    let bottom = unit.bottom + shift;
    let guard = 0;
    while (bottom > pageEnd(page) + 0.5 && guard++ < 500) {
      const height = unit.bottom - unit.top;
      if (top >= pageEnd(page) - 1) {
        breakBefore(unit, true);
        top = unit.top + shift;
        bottom = unit.bottom + shift;
        continue;
      }
      if (splittableTypes.has(type) && unit.node.isTextblock && unit.node.content.size > 0) {
        const split = findLineBreak(view, unit, pageEnd(page) - shift, toLocal);
        if (split !== null && split.lineTop - unit.top > 1) {
          const lineTop = split.lineTop + shift;
          const gapHeight = pageStart(page + 1) - lineTop;
          breaks.push({ pos: split.pos, height: gapHeight, inline: true });
          shift += gapHeight;
          newPage();
          top = pageStart(page);
          bottom = unit.bottom + shift;
          continue;
        }
      }
      if (height > contentHeight && top <= pageStart(page) + 2) {
        // Unteilbarer Block größer als eine Seite: überlaufen lassen und markieren.
        breaks.push({ pos: unit.pos, height: 0, inline: false, oversize: unit.node.nodeSize });
        const overflowPages = Math.ceil((bottom - pageEnd(page)) / pitch);
        for (let index = 0; index < overflowPages; index += 1) newPage();
        break;
      }
      const before = breaks.length;
      breakBefore(unit, true);
      if (breaks.length === before) break;
      top = unit.top + shift;
      bottom = unit.bottom + shift;
    }

    if (
      type === "pageBreak" ||
      type === "titlePage" ||
      (unit.depth === 0 && type === "directoryBlock") ||
      unit.closes.some(isFrontmatter)
    ) {
      forceBreak = true;
    }
    previous = unit;
  }
  return { breaks, pages };
}

/** Sucht die erste Zeile eines Absatzes, die über `limit` (natürliche Koordinate) hinausragt. */
function findLineBreak(
  view: EditorView,
  unit: Measured,
  limit: number,
  toLocal: (clientY: number) => number,
): { pos: number; lineTop: number } | null {
  const start = unit.pos + 1;
  const end = unit.pos + unit.node.nodeSize - 1;
  if (end <= start) return null;
  const coords = (pos: number) => {
    try {
      const rect = view.coordsAtPos(pos, 1);
      return { top: toLocal(rect.top), bottom: toLocal(rect.bottom) };
    } catch {
      return null;
    }
  };
  // Erste Position, deren Zeile unten über die Grenze hinausragt.
  let low = start;
  let high = end;
  while (low < high) {
    const middle = (low + high) >> 1;
    const rect = coords(middle);
    if (!rect) return null;
    if (rect.bottom > limit) high = middle;
    else low = middle + 1;
  }
  const crossing = coords(low);
  if (!crossing) return null;
  // Zeilenanfang dieser Zeile.
  let lineLow = start;
  let lineHigh = low;
  while (lineLow < lineHigh) {
    const middle = (lineLow + lineHigh) >> 1;
    const rect = coords(middle);
    if (!rect) return null;
    if (rect.top >= crossing.top - 1) lineHigh = middle;
    else lineLow = middle + 1;
  }
  if (lineLow <= start) return null;
  const lineRect = coords(lineLow);
  return lineRect ? { pos: lineLow, lineTop: lineRect.top } : null;
}

function sameBreaks(left: Break[], right: Break[]) {
  return (
    left.length === right.length &&
    left.every((entry, index) => {
      const other = right[index];
      return other.pos === entry.pos && other.inline === entry.inline && other.oversize === entry.oversize && Math.abs(other.height - entry.height) < 0.5;
    })
  );
}

export type PaginationStorage = {
  pages: PageInfo[];
  geometry: PageGeometry;
  listeners: Set<(pages: PageInfo[]) => void>;
  remeasure: () => void;
};

declare module "@tiptap/core" {
  interface Storage {
    pagination: PaginationStorage;
  }
  interface Commands<ReturnType> {
    pagination: {
      setPageGeometry: (geometry: PageGeometry) => ReturnType;
    };
  }
}

export const Pagination = Extension.create({
  name: "pagination",

  addStorage(): PaginationStorage {
    return {
      pages: [{ chapter: "" }],
      geometry: { enabled: false, pageHeight: 1123, marginTop: 94, marginBottom: 94, gap: 24, chapterNewPage: true },
      listeners: new Set(),
      remeasure: () => undefined,
    };
  },

  addCommands() {
    return {
      setPageGeometry:
        (geometry: PageGeometry) =>
        ({ tr, dispatch }) => {
          this.storage.geometry = geometry;
          if (dispatch) tr.setMeta(paginationKey, { geometryChanged: true });
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    const storage = this.storage as PaginationStorage;
    const publish = (pages: PageInfo[]) => {
      const changed =
        pages.length !== storage.pages.length ||
        pages.some((page, index) => JSON.stringify(page) !== JSON.stringify(storage.pages[index]));
      storage.pages = pages;
      if (changed) storage.listeners.forEach((listener) => listener(pages));
    };

    return [
      new Plugin<PaginationState>({
        key: paginationKey,
        state: {
          init: (_, state: EditorState) => ({ breaks: [], decorations: DecorationSet.create(state.doc, []) }),
          apply: (tr: Transaction, value: PaginationState) => {
            const meta = tr.getMeta(paginationKey) as { breaks?: Break[] } | undefined;
            if (meta?.breaks) return { breaks: meta.breaks, decorations: buildDecorations(tr.doc, meta.breaks) };
            if (!tr.docChanged) return value;
            return {
              breaks: value.breaks.map((entry) => ({ ...entry, pos: tr.mapping.map(entry.pos, -1) })),
              decorations: value.decorations.map(tr.mapping, tr.doc),
            };
          },
        },
        props: {
          decorations: (state) => paginationKey.getState(state)?.decorations,
        },
        view: (view) => {
          let timer: number | undefined;
          let measuring = false;
          const run = () => {
            timer = undefined;
            if (measuring || !view.dom.isConnected) return;
            const geometry = storage.geometry;
            const current = paginationKey.getState(view.state)?.breaks ?? [];
            if (!geometry.enabled) {
              if (current.length) view.dispatch(view.state.tr.setMeta(paginationKey, { breaks: [] }).setMeta("addToHistory", false));
              publish([{ chapter: "" }]);
              return;
            }
            measuring = true;
            try {
              if (current.length) view.dispatch(view.state.tr.setMeta(paginationKey, { breaks: [] }).setMeta("addToHistory", false));
              const { breaks, pages } = computeBreaks(view, geometry);
              if (!sameBreaks(breaks, [])) {
                view.dispatch(view.state.tr.setMeta(paginationKey, { breaks }).setMeta("addToHistory", false));
              }
              publish(pages);
            } finally {
              measuring = false;
            }
          };
          const schedule = (delay = 120) => {
            if (timer !== undefined) window.clearTimeout(timer);
            timer = window.setTimeout(run, delay);
          };
          storage.remeasure = () => schedule(0);
          const onLoad = () => schedule(60);
          view.dom.addEventListener("load", onLoad, true);
          const resize = new ResizeObserver(() => schedule(80));
          resize.observe(view.dom);
          if (document.fonts?.ready) void document.fonts.ready.then(() => schedule(0));
          schedule(0);
          return {
            update: (updatedView, previousState) => {
              if (measuring) return;
              if (updatedView.state.doc !== previousState.doc) schedule();
            },
            destroy: () => {
              if (timer !== undefined) window.clearTimeout(timer);
              view.dom.removeEventListener("load", onLoad, true);
              resize.disconnect();
              storage.remeasure = () => undefined;
            },
          };
        },
        appendTransaction: (transactions) => {
          if (transactions.some((tr) => (tr.getMeta(paginationKey) as { geometryChanged?: boolean } | undefined)?.geometryChanged)) {
            window.setTimeout(() => storage.remeasure(), 0);
          }
          return null;
        },
      }),
    ];
  },
});
