/**
 * Folien-Editor: Datenmodell (Spiegel von `src-tauri/src/core/slides.rs`) und
 * reine Geometrie-Hilfen (Raster, Hilfslinien, Größenänderung). Ohne DOM – in
 * Node testbar (`tests/slides.test.mjs`). Alle Maße in Millimetern.
 */
import type { JSONContent } from "@tiptap/core";

export type SlideAspect = "16:9" | "4:3";
export type ElementKind = "text" | "shape" | "image" | "formula";
export type ShapeKind = "rect" | "roundRect" | "ellipse" | "triangle" | "diamond" | "line" | "arrow";
export type PlaceholderRole = "" | "title" | "subtitle" | "body";

export type SlideElement = {
  id: string;
  kind: ElementKind;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Grad im Uhrzeigersinn */
  rotation: number;
  shape: ShapeKind;
  flipH: boolean;
  flipV: boolean;
  fill: string;
  stroke: string;
  /** pt */
  strokeWidth: number;
  content: JSONContent | null;
  /** pt */
  fontSize: number;
  color: string;
  verticalAlign: "top" | "middle" | "bottom";
  padding: number;
  src: string;
  latex: string;
  role: PlaceholderRole;
};

export type Slide = {
  id: string;
  background: string;
  notes: string;
  hidden: boolean;
  elements: SlideElement[];
};

export type SlideTheme = { background: string; textColor: string; accent: string; font: "sans" | "serif" };
export type SlideGrid = { size: number; snap: boolean; show: boolean; guides: boolean };

export type SlideDeck = {
  format: "visutex-slides";
  version: 1;
  title: string;
  subtitle: string;
  author: string;
  date: string;
  language: string;
  aspect: SlideAspect;
  theme: SlideTheme;
  grid: SlideGrid;
  slides: Slide[];
};

/** Offene Präsentation im Folien-Editor (bleibt beim Schließen des Editors erhalten). */
export type SlidesSession = { deck: SlideDeck; path: string | null; dirty: boolean };

/** Neue Präsentation: mit Titelfolie (Standard) oder einer leeren Folie. */
export function newSlidesSession(layout: "title" | "blank" = "title"): SlidesSession {
  const deck = createDeck();
  if (layout === "blank") deck.slides = [createSlide("blank", deck.aspect)];
  return { deck, path: null, dirty: false };
}

/** Dateibefehle des Folien-Editors für den gemeinsamen Datei-Bereich. */
export type SlideFileActions = {
  save: (saveAs: boolean) => Promise<boolean>;
  exportPdf: () => Promise<void>;
  exportLatex: () => Promise<void>;
  /** Fragt bei ungespeicherten Änderungen nach (Speichern/Verwerfen/Abbrechen). */
  confirmDiscard: () => Promise<boolean>;
  /** Ersetzt die Präsentation (neu/geöffnet) und leert den Rückgängig-Verlauf. */
  replace: (session: SlidesSession) => void;
  updateMeta: (patch: Partial<Pick<SlideDeck, "title" | "subtitle" | "author" | "date" | "language">>) => void;
};

export type Box = { x: number; y: number; w: number; h: number };
export type Guide = { orientation: "vertical" | "horizontal"; position: number };

/** 1 pt in mm */
export const PT = 25.4 / 72;

export function slideSize(aspect: SlideAspect): { w: number; h: number } {
  return aspect === "4:3" ? { w: 128, h: 96 } : { w: 160, h: 90 };
}

let sequence = 0;
export function uid(prefix: string): string {
  sequence += 1;
  return `${prefix}${Date.now().toString(36)}${sequence.toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

export function createElement(kind: ElementKind, patch: Partial<SlideElement> = {}): SlideElement {
  return {
    id: uid("e"),
    kind,
    x: 10,
    y: 10,
    w: 60,
    h: 20,
    rotation: 0,
    shape: "rect",
    flipH: false,
    flipV: false,
    fill: "",
    stroke: "",
    strokeWidth: 1,
    content: null,
    fontSize: 18,
    color: "",
    verticalAlign: "top",
    padding: 2,
    src: "",
    latex: "",
    role: "",
    ...patch,
  };
}

/** Textinhalt (Tiptap-JSON) aus einfachen Absätzen bzw. einer Aufzählung. */
export function textContent(paragraphs: string[], options: { align?: "left" | "center" | "right"; bold?: boolean; list?: boolean } = {}): JSONContent {
  const paragraph = (text: string): JSONContent => ({
    type: "paragraph",
    ...(options.align && options.align !== "left" ? { attrs: { textAlign: options.align } } : {}),
    ...(text ? { content: [{ type: "text", text, ...(options.bold ? { marks: [{ type: "bold" }] } : {}) }] } : {}),
  });
  if (options.list) {
    return { type: "doc", content: [{ type: "bulletList", content: paragraphs.map((text) => ({ type: "listItem", content: [paragraph(text)] })) }] };
  }
  return { type: "doc", content: (paragraphs.length ? paragraphs : [""]).map(paragraph) };
}

/** Enthält der Textinhalt sichtbaren Text oder Formeln? */
export function hasText(content: JSONContent | null | undefined): boolean {
  if (!content) return false;
  if (typeof content.text === "string" && content.text.trim()) return true;
  if (content.type === "inlineMath") return true;
  return (content.content ?? []).some(hasText);
}

export type LayoutId = "title" | "titleContent" | "twoContent" | "titleOnly" | "section" | "blank";

export const LAYOUTS: Array<{ id: LayoutId; label: string }> = [
  { id: "title", label: "Titelfolie" },
  { id: "titleContent", label: "Titel und Inhalt" },
  { id: "twoContent", label: "Zwei Inhalte" },
  { id: "titleOnly", label: "Nur Titel" },
  { id: "section", label: "Abschnitt" },
  { id: "blank", label: "Leer" },
];

/** Neue Folie mit Platzhaltern des Layouts (Textfelder ohne Inhalt). */
export function createSlide(layout: LayoutId, aspect: SlideAspect): Slide {
  const { w, h } = slideSize(aspect);
  const margin = 10;
  const elements: SlideElement[] = [];
  const title = (patch: Partial<SlideElement>) =>
    elements.push(createElement("text", { role: "title", fontSize: 28, verticalAlign: "middle", content: textContent([""]), ...patch }));
  const body = (patch: Partial<SlideElement>) =>
    elements.push(createElement("text", { role: "body", fontSize: 18, content: textContent([""], { list: true }), ...patch }));
  switch (layout) {
    case "title":
      title({ x: margin * 1.5, y: h * 0.28, w: w - margin * 3, h: h * 0.24, fontSize: 36, verticalAlign: "bottom", content: textContent([""], { align: "center" }) });
      elements.push(
        createElement("text", { role: "subtitle", x: margin * 2.5, y: h * 0.55, w: w - margin * 5, h: h * 0.16, fontSize: 20, content: textContent([""], { align: "center" }) }),
      );
      break;
    case "titleContent":
      title({ x: margin, y: 6, w: w - 2 * margin, h: 15 });
      body({ x: margin, y: 24, w: w - 2 * margin, h: h - 24 - margin });
      break;
    case "twoContent": {
      const column = (w - 2 * margin - 6) / 2;
      title({ x: margin, y: 6, w: w - 2 * margin, h: 15 });
      body({ x: margin, y: 24, w: column, h: h - 24 - margin });
      body({ x: margin + column + 6, y: 24, w: column, h: h - 24 - margin });
      break;
    }
    case "titleOnly":
      title({ x: margin, y: 6, w: w - 2 * margin, h: 15 });
      break;
    case "section":
      title({ x: margin * 1.5, y: h * 0.38, w: w - margin * 3, h: h * 0.2, fontSize: 32, verticalAlign: "middle", content: textContent([""], { align: "center" }) });
      break;
    case "blank":
      break;
  }
  return { id: uid("s"), background: "", notes: "", hidden: false, elements };
}

export function createDeck(): SlideDeck {
  return {
    format: "visutex-slides",
    version: 1,
    title: "",
    subtitle: "",
    author: "",
    date: "",
    language: "ngerman",
    aspect: "16:9",
    theme: { background: "#ffffff", textColor: "#1f2937", accent: "#2563eb", font: "sans" },
    grid: { size: 5, snap: true, show: true, guides: true },
    slides: [createSlide("title", "16:9")],
  };
}

/** Ergänzt fehlende Felder (Dateien älterer Versionen bzw. von Hand bearbeitet). */
export function normalizeDeck(raw: Partial<SlideDeck>): SlideDeck {
  const base = createDeck();
  const deck: SlideDeck = {
    ...base,
    ...raw,
    format: "visutex-slides",
    version: 1,
    aspect: raw.aspect === "4:3" ? "4:3" : "16:9",
    theme: { ...base.theme, ...(raw.theme ?? {}) },
    grid: { ...base.grid, ...(raw.grid ?? {}) },
    slides: (raw.slides ?? base.slides).map((slide) => ({
      id: slide.id || uid("s"),
      background: slide.background ?? "",
      notes: slide.notes ?? "",
      hidden: Boolean(slide.hidden),
      elements: (slide.elements ?? []).map((item) => createElement(item.kind ?? "text", { ...item, id: item.id || uid("e") })),
    })),
  };
  if (deck.slides.length === 0) deck.slides = [createSlide("blank", deck.aspect)];
  return deck;
}

// ---------------------------------------------------------------- Geometrie

export function snapToGrid(value: number, grid: number): number {
  if (!(grid > 0)) return value;
  return Math.round(value / grid) * grid;
}

export function round(value: number, digits = 2): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

/** Umschließendes Rechteck mehrerer Elemente (ohne Drehung). */
export function boundsOf(boxes: Box[]): Box {
  if (boxes.length === 0) return { x: 0, y: 0, w: 0, h: 0 };
  const left = Math.min(...boxes.map((box) => box.x));
  const top = Math.min(...boxes.map((box) => box.y));
  const right = Math.max(...boxes.map((box) => box.x + box.w));
  const bottom = Math.max(...boxes.map((box) => box.y + box.h));
  return { x: left, y: top, w: right - left, h: bottom - top };
}

type SnapContext = {
  /** Rasterweite (null = kein Rasterfang) */
  grid: number | null;
  /** Hilfslinien an Folienrändern/-mitte und anderen Elementen */
  guides: boolean;
  targets: Box[];
  slide: { w: number; h: number };
  /** Fangradius in mm */
  threshold: number;
};

function bestSnap(candidates: number[], lines: number[], threshold: number): { offset: number; line: number } | null {
  let best: { offset: number; line: number } | null = null;
  for (const candidate of candidates) {
    for (const line of lines) {
      const offset = line - candidate;
      if (Math.abs(offset) <= threshold && (!best || Math.abs(offset) < Math.abs(best.offset))) best = { offset, line };
    }
  }
  return best;
}

/**
 * Verschiebung einer Auswahl (`box` = Ausgangslage) um `dx`/`dy` mit Fang:
 * Hilfslinien (Kanten/Mitten der Folie und anderer Elemente) haben Vorrang,
 * sonst rastet die linke/obere Kante im Raster ein.
 */
export function snapMove(box: Box, dx: number, dy: number, context: SnapContext): { dx: number; dy: number; guides: Guide[] } {
  const moved = { ...box, x: box.x + dx, y: box.y + dy };
  const guides: Guide[] = [];
  let resultX = dx;
  let resultY = dy;
  let snappedX = false;
  let snappedY = false;
  if (context.guides) {
    const verticalLines = [0, context.slide.w / 2, context.slide.w, ...context.targets.flatMap((target) => [target.x, target.x + target.w / 2, target.x + target.w])];
    const horizontalLines = [0, context.slide.h / 2, context.slide.h, ...context.targets.flatMap((target) => [target.y, target.y + target.h / 2, target.y + target.h])];
    const snapX = bestSnap([moved.x, moved.x + moved.w / 2, moved.x + moved.w], verticalLines, context.threshold);
    const snapY = bestSnap([moved.y, moved.y + moved.h / 2, moved.y + moved.h], horizontalLines, context.threshold);
    if (snapX) {
      resultX += snapX.offset;
      snappedX = true;
      guides.push({ orientation: "vertical", position: snapX.line });
    }
    if (snapY) {
      resultY += snapY.offset;
      snappedY = true;
      guides.push({ orientation: "horizontal", position: snapY.line });
    }
  }
  if (context.grid) {
    if (!snappedX) resultX = snapToGrid(box.x + dx, context.grid) - box.x;
    if (!snappedY) resultY = snapToGrid(box.y + dy, context.grid) - box.y;
  }
  return { dx: round(resultX), dy: round(resultY), guides };
}

export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

/**
 * Neue Größe beim Ziehen eines Anfassers (ungedrehte Elemente). `keepAspect`:
 * Seitenverhältnis halten (Umschalt bzw. Bilder an Ecken). Rasterfang wirkt auf
 * die gezogene Kante.
 */
export function resizeBox(start: Box, handle: Handle, dx: number, dy: number, options: { keepAspect: boolean; grid: number | null; min: number }): Box {
  let left = start.x;
  let top = start.y;
  let right = start.x + start.w;
  let bottom = start.y + start.h;
  const snap = (value: number) => (options.grid ? snapToGrid(value, options.grid) : value);
  if (handle.includes("w")) left = snap(left + dx);
  if (handle.includes("e")) right = snap(right + dx);
  if (handle.includes("n")) top = snap(top + dy);
  if (handle.includes("s")) bottom = snap(bottom + dy);
  if (right - left < options.min) {
    if (handle.includes("w")) left = right - options.min;
    else right = left + options.min;
  }
  if (bottom - top < options.min) {
    if (handle.includes("n")) top = bottom - options.min;
    else bottom = top + options.min;
  }
  let w = right - left;
  let h = bottom - top;
  if (options.keepAspect && start.w > 0 && start.h > 0 && handle.length === 2) {
    const ratio = start.w / start.h;
    if (w / h > ratio) w = h * ratio;
    else h = w / ratio;
    if (handle.includes("w")) left = right - w;
    if (handle.includes("n")) top = bottom - h;
  }
  return { x: round(left), y: round(top), w: round(w), h: round(h) };
}

/** Kopien mit neuen IDs, versetzt um `offset` mm. */
export function cloneElements(elements: SlideElement[], offset: number): SlideElement[] {
  return elements.map((item) => ({ ...structuredClone(item), id: uid("e"), x: round(item.x + offset), y: round(item.y + offset) }));
}

/** Ausrichten an der Folie (eine Auswahl) bzw. an der Auswahl (mehrere Elemente). */
export type Alignment = "left" | "center" | "right" | "top" | "middle" | "bottom";

export function alignElements(elements: SlideElement[], ids: string[], alignment: Alignment, slide: { w: number; h: number }): SlideElement[] {
  const selected = elements.filter((item) => ids.includes(item.id));
  const frame = selected.length > 1 ? boundsOf(selected) : { x: 0, y: 0, w: slide.w, h: slide.h };
  return elements.map((item) => {
    if (!ids.includes(item.id)) return item;
    switch (alignment) {
      case "left": return { ...item, x: round(frame.x) };
      case "center": return { ...item, x: round(frame.x + (frame.w - item.w) / 2) };
      case "right": return { ...item, x: round(frame.x + frame.w - item.w) };
      case "top": return { ...item, y: round(frame.y) };
      case "middle": return { ...item, y: round(frame.y + (frame.h - item.h) / 2) };
      case "bottom": return { ...item, y: round(frame.y + frame.h - item.h) };
    }
  });
}

/** Gleichmäßig verteilen (mindestens drei Elemente). */
export function distributeElements(elements: SlideElement[], ids: string[], axis: "horizontal" | "vertical"): SlideElement[] {
  const selected = elements.filter((item) => ids.includes(item.id));
  if (selected.length < 3) return elements;
  const key = axis === "horizontal" ? "x" : "y";
  const size = axis === "horizontal" ? "w" : "h";
  const sorted = [...selected].sort((a, b) => a[key] - b[key]);
  const first = sorted[0];
  const last = sorted[sorted.length - 1];
  const total = sorted.reduce((sum, item) => sum + item[size], 0);
  const gap = (last[key] + last[size] - first[key] - total) / (sorted.length - 1);
  const positions = new Map<string, number>();
  let cursor = first[key];
  for (const item of sorted) {
    positions.set(item.id, round(cursor));
    cursor += item[size] + gap;
  }
  return elements.map((item) => (positions.has(item.id) ? { ...item, [key]: positions.get(item.id) } : item));
}

/** Ebenen: in den Vordergrund/Hintergrund, eine Ebene nach vorn/hinten. */
export function reorderElements(elements: SlideElement[], ids: string[], direction: "front" | "back" | "forward" | "backward"): SlideElement[] {
  const selected = elements.filter((item) => ids.includes(item.id));
  const others = elements.filter((item) => !ids.includes(item.id));
  if (direction === "front") return [...others, ...selected];
  if (direction === "back") return [...selected, ...others];
  const result = [...elements];
  const indices = result.map((item, index) => (ids.includes(item.id) ? index : -1)).filter((index) => index >= 0);
  if (direction === "forward") {
    for (const index of indices.reverse()) {
      if (index < result.length - 1 && !ids.includes(result[index + 1].id)) [result[index], result[index + 1]] = [result[index + 1], result[index]];
    }
  } else {
    for (const index of indices) {
      if (index > 0 && !ids.includes(result[index - 1].id)) [result[index], result[index - 1]] = [result[index - 1], result[index]];
    }
  }
  return result;
}
