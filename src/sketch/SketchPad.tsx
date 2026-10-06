/**
 * Skizzier-Werkzeug: verschiebbares, in der Größe veränderbares Unterfenster mit
 * Raster. Zeichenwerkzeuge (Linie, Pfeil, Pfad, Polygon, Kurve, Freihand,
 * Rechteck, Kreis, Ellipse, Bogen, Text) und eine Palette mit **allen**
 * CircuiTikZ-Bauteilen und TikZ-Formen (Symbole und Anschlüsse aus
 * `resources/sketch-symbols.json`, erzeugt mit der mitgelieferten CircuiTikZ-
 * Version). Leitungen docken an Bauteil-Anschlüssen an (`(Q1.G)`).
 * Der TikZ/CircuiTikZ-Code entsteht in Rust (`core::sketch`).
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent, ReactNode } from "react";
import { Hand, Search, X } from "lucide-react";
import { api, errorText } from "../api";
import type { SketchCatalog, SketchSymbol } from "../api";
import { useLanguage, useT } from "../i18n";
import { useCtrlWheel } from "../components/useCtrlWheel";
import { renderMath } from "../latex/miniRender";

export type SketchElement = {
  id: string;
  kind: "line" | "arrow" | "path" | "rect" | "circle" | "ellipse" | "arc" | "text" | "dot" | "terminal" | "ground" | "component" | "symbol";
  component?: string;
  symbol?: string;
  name?: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  points?: Array<[number, number]>;
  closed?: boolean;
  smooth?: boolean;
  sweep?: number;
  rotation?: number;
  mirror?: boolean;
  flip?: boolean;
  label?: string;
  annotation?: string;
  voltage?: string;
  current?: string;
  flow?: string;
  labelBelow?: boolean;
  voltageSide?: string;
  currentDir?: string;
  invert?: boolean;
  labelPosition?: string;
  ref1?: string;
  ref2?: string;
  color?: string;
  fill?: string;
  lineWidth?: string;
  dash?: string;
  arrowStart?: string;
  arrowEnd?: string;
  fontSize?: string;
  bold?: boolean;
  italic?: boolean;
  textMode?: string;
  dashed?: boolean;
  thick?: boolean;
};

export type Sketch = { version: number; elements: SketchElement[] };

type DrawTool = "line" | "arrow" | "path" | "polygon" | "curve" | "freehand" | "rect" | "circle" | "ellipse" | "arc" | "text" | "dot" | "terminal";
type Tool = "select" | DrawTool | `symbol:${string}`;

/** Pixel je Rastereinheit (0,5 cm) bei 100 %. */
const CELL = 22;
const COLUMNS = 64;
const ROWS = 40;
const UNIT_CM = 0.5;

let nextId = 1;
const newId = () => `e${Date.now().toString(36)}${(nextId++).toString(36)}`;

/** Zeiger festhalten (Ziehen über den Rand hinaus); ohne aktiven Zeiger wirkungslos. */
function capturePointer(element: Element, pointerId: number) {
  try {
    element.setPointerCapture(pointerId);
  } catch {
    // z. B. synthetische Ereignisse ohne aktiven Zeiger
  }
}

/** Bauteile alter Skizzen (`component`) → Katalogsymbol für die Anzeige. */
const LEGACY_COMPONENTS: Record<string, string> = {
  R: "R", C: "C", L: "L", V: "V", I: "I", D: "D", LED: "leD", switch: "nos", lamp: "lamp",
  battery: "battery1", voltmeter: "voltmeter", ammeter: "ammeter", fuse: "fuse", Rvar: "vR", short: "short",
};

const LINE_WIDTH_PX: Record<string, number> = {
  "ultra thin": 0.6, "very thin": 0.9, thin: 1.4, semithick: 2, thick: 2.6, "very thick": 3.6, "ultra thick": 5,
};
const DASH_ARRAY: Record<string, string> = {
  dashed: "7 5", "densely dashed": "5 3", "loosely dashed": "8 9", dotted: "1.5 4", "densely dotted": "1.5 2.5",
  "loosely dotted": "1.5 8", "dash dot": "7 4 1.5 4", "dash dot dot": "7 4 1.5 4 1.5 4",
};
const FONT_SIZE_PX: Record<string, number> = {
  tiny: 7, scriptsize: 9, footnotesize: 11, small: 12.5, normalsize: 14, large: 16.5, Large: 20, LARGE: 24, huge: 28.5, Huge: 34,
};
/** Formen, die direkt als SVG gezeichnet werden (Rest: Symbolbild). */
const NATIVE_SHAPES = new Set([
  "rectangle", "rounded rectangle", "chamfered rectangle", "circle", "ellipse", "diamond", "trapezium", "semicircle",
  "isosceles triangle", "regular polygon", "star", "kite", "dart", "circular sector", "cylinder", "cross out", "strike out",
  "single arrow", "double arrow", "signal", "tape",
]);

const escapeHtml = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Beschriftung wie im Rust-Export: Formeln (`R_1`, `\Omega`, `$…$`) als Formel. */
function labelHtml(label: string, mode = "auto"): string {
  const text = label.trim();
  if (!text) return "";
  const math = mode === "math" || (mode !== "text" && (text.includes("$") || /[_^\\]/.test(text) || (text.length <= 4 && !text.includes(" "))));
  if (!math) return escapeHtml(text);
  if (text.includes("$") && mode !== "math") {
    return text.split("$").map((part, index) => (index % 2 === 1 ? renderMath(part, false) : escapeHtml(part))).join("");
  }
  return renderMath(text.replace(/\$/g, ""), false);
}

function Label({ x, y, html, anchor = "middle", color, size = 13, rotate = 0 }: { x: number; y: number; html: string; anchor?: "start" | "middle" | "end"; color?: string; size?: number; rotate?: number }) {
  if (!html) return null;
  const width = 400;
  const height = size * 3;
  const left = anchor === "start" ? x : anchor === "end" ? x - width : x - width / 2;
  return (
    <foreignObject x={left} y={y - height / 2} width={width} height={height} transform={rotate ? `rotate(${rotate} ${x} ${y})` : undefined} style={{ overflow: "visible", pointerEvents: "none" }}>
      <div
        className="sketch-label"
        style={{ justifyContent: anchor === "start" ? "flex-start" : anchor === "end" ? "flex-end" : "center", fontSize: size, color }}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </foreignObject>
  );
}

// ---------------------------------------------------------------- Pfeilspitzen

const TIP_PATHS: Record<string, { d: string; open?: boolean; line?: boolean }> = {
  Stealth: { d: "M0,0 L10,5 L0,10 L3,5 z" },
  Latex: { d: "M0,0 L10,5 L0,10 z" },
  Triangle: { d: "M0,0 L10,5 L0,10 z" },
  To: { d: "M1,1 Q6,4 10,5 Q6,6 1,9", line: true },
  Kite: { d: "M0,5 L4,0 L10,5 L4,10 z" },
  Circle: { d: "M5,1 A4,4 0 1 1 4.99,1 z" },
  Square: { d: "M1,1 L9,1 L9,9 L1,9 z" },
  Diamond: { d: "M0,5 L5,0 L10,5 L5,10 z" },
  Ellipse: { d: "M5,2 A5,3 0 1 1 4.99,2 z" },
  "Turned Square": { d: "M0,5 L5,0 L10,5 L5,10 z" },
  Bar: { d: "M9,0 L9,10", line: true },
  Bracket: { d: "M6,0 L9,0 L9,10 L6,10", line: true },
  Parenthesis: { d: "M5,0 Q10,5 5,10", line: true },
  Rays: { d: "M5,0 L5,10 M0,5 L10,5 M1.5,1.5 L8.5,8.5 M1.5,8.5 L8.5,1.5", line: true },
  Hooks: { d: "M8,0 Q3,0 3,5 M8,10 Q3,10 3,5", line: true },
  "Straight Barb": { d: "M1,1 L10,5 L1,9", line: true },
  "Arc Barb": { d: "M2,0 Q10,2 10,5 Q10,8 2,10", line: true },
  "Tee Barb": { d: "M9,0 L9,10 M3,5 L9,5", line: true },
  Implies: { d: "M2,0 Q6,4 10,5 Q6,6 2,10", line: true },
  "Computer Modern Rightarrow": { d: "M1,1 Q6,4 10,5 Q6,6 1,9", line: true },
};

function tipId(tip: string, color: string): string {
  return `tip-${tip.replace(/[^A-Za-z]/g, "")}-${color.replace("#", "")}`;
}

function TipMarker({ tip, color }: { tip: string; color: string }) {
  const base = tip.replace("[open]", "");
  const shape = TIP_PATHS[base] ?? TIP_PATHS.Stealth;
  const open = tip.includes("[open]") || shape.line;
  return (
    <marker id={tipId(tip, color)} viewBox="-1 -1 12 12" refX="9" refY="5" markerWidth="9" markerHeight="9" orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d={shape.d} fill={open ? (shape.line ? "none" : "#fff") : color} stroke={color} strokeWidth={1.2} />
    </marker>
  );
}

function arrowEnds(element: SketchElement): { start?: string; end?: string } {
  const legacyArrow = element.kind === "arrow" && !element.arrowStart && !element.arrowEnd;
  return { start: element.arrowStart || undefined, end: element.arrowEnd || (legacyArrow ? "Stealth" : undefined) };
}

// ---------------------------------------------------------------- Geometrie

const deg = (radians: number) => (radians * 180) / Math.PI;
const rad = (degrees: number) => (degrees * Math.PI) / 180;

/** Anschlüsse eines Knotens in Rastereinheiten (Bildschirmrichtung). */
function nodePins(element: SketchElement, symbol: SketchSymbol | undefined): Array<{ name: string; x: number; y: number }> {
  if (!symbol || symbol.kind !== "node") return [];
  const angle = rad(element.rotation ?? 0);
  return symbol.anchors.map(([name, ax, ay]) => {
    let x = ax / UNIT_CM;
    let y = -ay / UNIT_CM;
    if (element.mirror) x = -x;
    if (element.flip) y = -y;
    const rx = x * Math.cos(angle) - y * Math.sin(angle);
    const ry = x * Math.sin(angle) + y * Math.cos(angle);
    return { name, x: element.x1 + rx, y: element.y1 + ry };
  });
}

/** Verweise auf Anschlüsse auflösen: Leitungsenden folgen verschobenen Knoten. */
function resolveRefs(elements: SketchElement[], symbols: Map<string, SketchSymbol>): SketchElement[] {
  const nodes = new Map(elements.filter((element) => element.kind === "symbol").map((element) => [element.id, element]));
  let changed = false;
  const result = elements.map((element) => {
    if (!element.ref1 && !element.ref2) return element;
    const patch: Partial<SketchElement> = {};
    for (const end of [1, 2] as const) {
      const reference = end === 1 ? element.ref1 : element.ref2;
      if (!reference) continue;
      const [id, ...rest] = reference.split(".");
      const anchor = rest.join(".");
      const node = nodes.get(id);
      const pin = node ? nodePins(node, symbols.get(node.symbol ?? "")).find((candidate) => candidate.name === anchor) : undefined;
      if (!pin) {
        patch[end === 1 ? "ref1" : "ref2"] = "";
        continue;
      }
      const keyX = end === 1 ? "x1" : "x2";
      const keyY = end === 1 ? "y1" : "y2";
      if (Math.abs(element[keyX] - pin.x) > 1e-6 || Math.abs(element[keyY] - pin.y) > 1e-6) {
        patch[keyX] = pin.x;
        patch[keyY] = pin.y;
      }
    }
    if (Object.keys(patch).length === 0) return element;
    changed = true;
    return { ...element, ...patch };
  });
  return changed ? result : elements;
}

// ---------------------------------------------------------------- Formen (SVG)

function shapePath(shape: string, w: number, h: number): string {
  const r = Math.min(w, h) / 2;
  switch (shape) {
    case "rounded rectangle":
      return `M${r},0 H${w - r} A${r},${r} 0 0 1 ${w - r},${h} H${r} A${r},${r} 0 0 1 ${r},0 Z`;
    case "chamfered rectangle": {
      const c = Math.min(w, h) * 0.2;
      return `M${c},0 H${w - c} L${w},${c} V${h - c} L${w - c},${h} H${c} L0,${h - c} V${c} Z`;
    }
    case "diamond":
      return `M${w / 2},0 L${w},${h / 2} L${w / 2},${h} L0,${h / 2} Z`;
    case "trapezium":
      return `M${w * 0.2},0 H${w * 0.8} L${w},${h} H0 Z`;
    case "semicircle":
      return `M0,${h} A${w / 2},${h} 0 0 1 ${w},${h} Z`;
    case "isosceles triangle":
      return `M0,0 L${w},${h / 2} L0,${h} Z`;
    case "regular polygon":
    case "star": {
      const points: string[] = [];
      const count = shape === "star" ? 10 : 5;
      for (let index = 0; index < count; index += 1) {
        const angle = -Math.PI / 2 + (index * 2 * Math.PI) / count;
        const factor = shape === "star" && index % 2 === 1 ? 0.45 : 1;
        points.push(`${w / 2 + (w / 2) * factor * Math.cos(angle)},${h / 2 + (h / 2) * factor * Math.sin(angle)}`);
      }
      return `M${points.join(" L")} Z`;
    }
    case "kite":
      return `M${w / 2},0 L${w},${h * 0.35} L${w / 2},${h} L0,${h * 0.35} Z`;
    case "dart":
      return `M0,0 L${w},${h / 2} L0,${h} L${w * 0.3},${h / 2} Z`;
    case "circular sector":
      return `M${w * 0.1},${h / 2} L${w},${h * 0.05} A${w * 0.9},${h * 0.9} 0 0 1 ${w},${h * 0.95} Z`;
    case "cylinder": {
      const e = Math.min(w * 0.15, h / 2);
      return `M${e},0 H${w - e} A${e},${h / 2} 0 0 1 ${w - e},${h} H${e} A${e},${h / 2} 0 0 1 ${e},0 Z M${w - e},0 A${e},${h / 2} 0 0 0 ${w - e},${h}`;
    }
    case "cross out":
      return `M0,0 L${w},${h} M${w},0 L0,${h}`;
    case "strike out":
      return `M0,${h} L${w},0`;
    case "single arrow":
      return `M0,${h * 0.25} H${w * 0.65} V0 L${w},${h / 2} L${w * 0.65},${h} V${h * 0.75} H0 Z`;
    case "double arrow":
      return `M0,${h / 2} L${w * 0.3},0 V${h * 0.25} H${w * 0.7} V0 L${w},${h / 2} L${w * 0.7},${h} V${h * 0.75} H${w * 0.3} V${h} Z`;
    case "signal":
      return `M0,0 H${w * 0.8} L${w},${h / 2} L${w * 0.8},${h} H0 Z`;
    case "tape":
      return `M0,${h * 0.1} Q${w * 0.25},${-h * 0.05} ${w / 2},${h * 0.1} T${w},${h * 0.1} V${h * 0.9} Q${w * 0.75},${h * 1.05} ${w / 2},${h * 0.9} T0,${h * 0.9} Z`;
    default:
      return `M0,0 H${w} V${h} H0 Z`;
  }
}

/** Symbolbild, eingefärbt (Maske) – für Bauteile und Formen ohne SVG-Nachbildung. */
function SymbolImage({ symbol, scale, color, id }: { symbol: SketchSymbol; scale: number; color: string; id: string }) {
  const width = symbol.w * scale;
  const height = symbol.h * scale;
  const x = -symbol.ox * scale;
  const y = -symbol.oy * scale;
  return (
    <g>
      <defs>
        <mask id={`mask-${id}`} maskUnits="userSpaceOnUse" x={x} y={y} width={width} height={height} style={{ maskType: "alpha" }}>
          <image href={symbol.image} x={x} y={y} width={width} height={height} />
        </mask>
      </defs>
      <rect x={x} y={y} width={width} height={height} fill={color} mask={`url(#mask-${id})`} />
    </g>
  );
}

// ---------------------------------------------------------------- Element-Darstellung

type ShapeProps = {
  element: SketchElement;
  selected: boolean;
  cell: number;
  catalog: SketchCatalog | null;
  symbols: Map<string, SketchSymbol>;
};

function ElementShape({ element, selected, cell, catalog, symbols }: ShapeProps) {
  const p1 = { x: element.x1 * cell, y: element.y1 * cell };
  const p2 = { x: element.x2 * cell, y: element.y2 * cell };
  const baseColor = element.color || "#1f2937";
  const color = selected ? "var(--accent)" : baseColor;
  const markerColor = selected ? "#2563eb" : baseColor;
  const dash = DASH_ARRAY[element.dash ?? ""] ?? (element.dashed ? DASH_ARRAY.dashed : undefined);
  const width = LINE_WIDTH_PX[element.lineWidth ?? ""] ?? (element.thick ? 2.6 : 2);
  const fill = element.fill || "none";
  const label = element.label?.trim() ?? "";
  const ends = arrowEnds(element);
  const markers = {
    markerStart: ends.start ? `url(#${tipId(ends.start, markerColor)})` : undefined,
    markerEnd: ends.end ? `url(#${tipId(ends.end, markerColor)})` : undefined,
  };
  const markerDefs = (
    <defs>
      {ends.start && <TipMarker tip={ends.start} color={markerColor} />}
      {ends.end && ends.end !== ends.start && <TipMarker tip={ends.end} color={markerColor} />}
    </defs>
  );
  const pxPerCm = cell / UNIT_CM;
  const imageScale = catalog ? pxPerCm / catalog.pxPerCm : 0.5;

  switch (element.kind) {
    case "line":
    case "arrow":
      return (
        <g style={{ color }}>
          {markerDefs}
          <line x1={p1.x} y1={p1.y} x2={p2.x} y2={p2.y} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} {...markers} />
          <Label x={(p1.x + p2.x) / 2} y={(p1.y + p2.y) / 2 - 12} html={labelHtml(label)} color={baseColor} />
        </g>
      );
    case "path": {
      const points = element.points ?? [];
      if (points.length < 2) return null;
      const scaled = points.map(([x, y]) => [x * cell, y * cell] as const);
      let d = `M${scaled[0][0]},${scaled[0][1]}`;
      if (element.smooth && scaled.length > 2) {
        // Catmull-Rom → kubische Bézierkurven (ähnlich TikZ `plot[smooth]`)
        const list = element.closed ? [scaled[scaled.length - 1], ...scaled, scaled[0], scaled[1]] : [scaled[0], ...scaled, scaled[scaled.length - 1]];
        for (let index = 1; index < list.length - 2; index += 1) {
          const [p0, a, b, p3] = [list[index - 1], list[index], list[index + 1], list[index + 2]];
          const c1 = [a[0] + (b[0] - p0[0]) / 6, a[1] + (b[1] - p0[1]) / 6];
          const c2 = [b[0] - (p3[0] - a[0]) / 6, b[1] - (p3[1] - a[1]) / 6];
          d += ` C${c1[0]},${c1[1]} ${c2[0]},${c2[1]} ${b[0]},${b[1]}`;
        }
      } else {
        d += scaled.slice(1).map(([x, y]) => ` L${x},${y}`).join("");
      }
      if (element.closed) d += " Z";
      return (
        <g style={{ color }}>
          {!element.closed && markerDefs}
          <path d={d} fill={element.closed ? fill : "none"} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} strokeLinejoin="round" {...(element.closed ? {} : markers)} />
        </g>
      );
    }
    case "rect":
      return (
        <g style={{ color }}>
          <rect x={Math.min(p1.x, p2.x)} y={Math.min(p1.y, p2.y)} width={Math.abs(p2.x - p1.x)} height={Math.abs(p2.y - p1.y)} fill={fill} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} />
          <Label x={(p1.x + p2.x) / 2} y={(p1.y + p2.y) / 2} html={labelHtml(label)} color={baseColor} />
        </g>
      );
    case "circle": {
      const radius = Math.hypot(p2.x - p1.x, p2.y - p1.y);
      return (
        <g style={{ color }}>
          <circle cx={p1.x} cy={p1.y} r={Math.max(2, radius)} fill={fill} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} />
          <Label x={p1.x} y={p1.y} html={labelHtml(label)} color={baseColor} />
        </g>
      );
    }
    case "ellipse":
      return (
        <g style={{ color }}>
          <ellipse cx={(p1.x + p2.x) / 2} cy={(p1.y + p2.y) / 2} rx={Math.max(2, Math.abs(p2.x - p1.x) / 2)} ry={Math.max(2, Math.abs(p2.y - p1.y) / 2)} fill={fill} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} />
        </g>
      );
    case "arc": {
      const radius = Math.max(2, Math.hypot(p2.x - p1.x, p2.y - p1.y));
      const sweep = Math.abs(element.sweep ?? 90) < 0.5 ? 90 : (element.sweep ?? 90);
      const start = Math.atan2(p2.y - p1.y, p2.x - p1.x);
      // TikZ zählt gegen den Uhrzeigersinn (y nach oben) → am Bildschirm negativ
      const end = start - rad(sweep);
      const endPoint = { x: p1.x + radius * Math.cos(end), y: p1.y + radius * Math.sin(end) };
      const large = Math.abs(sweep) > 180 ? 1 : 0;
      const direction = sweep > 0 ? 0 : 1;
      return (
        <g style={{ color }}>
          {markerDefs}
          <path d={`M${p2.x},${p2.y} A${radius},${radius} 0 ${large} ${direction} ${endPoint.x},${endPoint.y}`} fill="none" stroke="currentColor" strokeWidth={width} strokeDasharray={dash} {...markers} />
          <circle cx={p1.x} cy={p1.y} r={2} fill="currentColor" opacity={0.35} />
        </g>
      );
    }
    case "text": {
      const size = FONT_SIZE_PX[element.fontSize ?? ""] ?? 14;
      return (
        <g>
          <Label
            x={p1.x}
            y={p1.y}
            html={`<span style="font-weight:${element.bold ? 700 : 400};font-style:${element.italic ? "italic" : "normal"}">${labelHtml(label || "Text", element.textMode || "auto")}</span>`}
            color={selected ? "var(--accent)" : element.color || "#1f2937"}
            size={size}
            rotate={element.rotation ?? 0}
          />
          <circle cx={p1.x} cy={p1.y} r={2} fill={color} opacity={selected ? 0.6 : 0} />
        </g>
      );
    }
    case "dot":
      return <circle cx={p1.x} cy={p1.y} r={4} fill={color} />;
    case "terminal":
      return (
        <g style={{ color }}>
          <circle cx={p1.x} cy={p1.y} r={4} fill="white" stroke="currentColor" strokeWidth={2} />
          <Label x={p1.x - 8} y={p1.y} html={labelHtml(label)} anchor="end" color={baseColor} />
        </g>
      );
    case "ground":
      return (
        <g style={{ color }} stroke="currentColor" strokeWidth={2}>
          <line x1={p1.x} y1={p1.y} x2={p1.x} y2={p1.y + 8} />
          <line x1={p1.x - 10} y1={p1.y + 8} x2={p1.x + 10} y2={p1.y + 8} />
          <line x1={p1.x - 6} y1={p1.y + 12} x2={p1.x + 6} y2={p1.y + 12} />
          <line x1={p1.x - 2} y1={p1.y + 16} x2={p1.x + 2} y2={p1.y + 16} />
        </g>
      );
    case "component":
    case "symbol": {
      const symbolId = element.kind === "component" ? LEGACY_COMPONENTS[element.component ?? "R"] ?? "R" : element.symbol ?? "";
      const symbol = symbols.get(symbolId);
      if (!symbol) {
        // Katalog (noch) nicht geladen: Platzhalter
        return <line x1={p1.x} y1={p1.y} x2={p2.x} y2={p2.y} stroke={color} strokeWidth={2} strokeDasharray="3 3" />;
      }
      if (symbol.kind === "bipole") {
        const length = Math.hypot(p2.x - p1.x, p2.y - p1.y);
        const angle = deg(Math.atan2(p2.y - p1.y, p2.x - p1.x));
        const middle = { x: (p1.x + p2.x) / 2, y: (p1.y + p2.y) / 2 };
        const half = pxPerCm; // Symbol wurde zwischen −1 cm und +1 cm erzeugt
        const scaleY = element.mirror ? -1 : 1;
        const scaleX = element.invert ? -1 : 1;
        const normal = { x: -Math.sin(rad(angle)), y: Math.cos(rad(angle)) };
        const side = (below: boolean) => (below ? 1 : -1) * 22;
        const labelAt = (below: boolean) => ({ x: middle.x + normal.x * side(below), y: middle.y + normal.y * side(below) });
        const top = labelAt(Boolean(element.labelBelow));
        const bottom = labelAt(!element.labelBelow);
        const extras = [element.voltage && `U: ${element.voltage}`, element.current && `I: ${element.current}`, element.flow && `f: ${element.flow}`].filter(Boolean).join("  ");
        return (
          <g style={{ color }}>
            <g transform={`translate(${middle.x} ${middle.y}) rotate(${angle})`}>
              {length / 2 > half && (
                <>
                  <line x1={-length / 2} y1={0} x2={-half} y2={0} stroke="currentColor" strokeWidth={1.6} />
                  <line x1={half} y1={0} x2={length / 2} y2={0} stroke="currentColor" strokeWidth={1.6} />
                </>
              )}
              <g transform={`scale(${scaleX} ${scaleY})`}>
                <SymbolImage symbol={symbol} scale={imageScale} color={selected ? "#2563eb" : baseColor} id={element.id} />
              </g>
            </g>
            <Label x={top.x} y={top.y} html={labelHtml(label)} color={baseColor} />
            <Label x={bottom.x} y={bottom.y} html={[labelHtml(element.annotation ?? ""), extras && escapeHtml(extras)].filter(Boolean).join(" · ")} color={baseColor} size={12} />
          </g>
        );
      }
      if (symbol.kind === "node") {
        const transform = `translate(${p1.x} ${p1.y}) rotate(${element.rotation ?? 0}) scale(${element.mirror ? -1 : 1} ${element.flip ? -1 : 1})`;
        const radius = Math.max(symbol.w, symbol.h) * imageScale * 0.5;
        const offsets: Record<string, [number, number, "start" | "middle" | "end"]> = {
          above: [0, -radius - 8, "middle"],
          below: [0, radius + 8, "middle"],
          left: [-radius - 6, 0, "end"],
          right: [radius + 6, 0, "start"],
          inside: [0, 0, "middle"],
        };
        const [dx, dy, anchor] = offsets[element.labelPosition || "above"] ?? offsets.above;
        return (
          <g style={{ color }}>
            <g transform={transform}>
              <SymbolImage symbol={symbol} scale={imageScale} color={selected ? "#2563eb" : baseColor} id={element.id} />
            </g>
            <Label x={p1.x + dx} y={p1.y + dy} html={labelHtml(label)} anchor={anchor} color={baseColor} />
          </g>
        );
      }
      // TikZ-Form im Rechteck
      const left = Math.min(p1.x, p2.x);
      const top = Math.min(p1.y, p2.y);
      const w = Math.max(4, Math.abs(p2.x - p1.x));
      const h = Math.max(4, Math.abs(p2.y - p1.y));
      const center = { x: left + w / 2, y: top + h / 2 };
      const native = NATIVE_SHAPES.has(symbolId);
      const textSize = FONT_SIZE_PX[element.fontSize ?? ""] ?? 14;
      return (
        <g style={{ color }} transform={element.rotation ? `rotate(${element.rotation} ${center.x} ${center.y})` : undefined}>
          {native ? (
            symbolId === "circle" || symbolId === "ellipse" ? (
              <ellipse cx={center.x} cy={center.y} rx={symbolId === "circle" ? Math.max(w, h) / 2 : w / 2} ry={symbolId === "circle" ? Math.max(w, h) / 2 : h / 2} fill={fill} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} />
            ) : (
              <path d={shapePath(symbolId, w, h)} transform={`translate(${left} ${top})`} fill={symbolId === "cross out" || symbolId === "strike out" ? "none" : fill} stroke="currentColor" strokeWidth={width} strokeDasharray={dash} strokeLinejoin="round" />
            )
          ) : (
            <g transform={`translate(${center.x} ${center.y}) scale(${w / Math.max(1, symbol.w * imageScale)} ${h / Math.max(1, symbol.h * imageScale)})`}>
              <SymbolImage symbol={{ ...symbol, ox: symbol.w / 2, oy: symbol.h / 2 }} scale={imageScale} color={selected ? "#2563eb" : baseColor} id={element.id} />
            </g>
          )}
          <Label
            x={center.x}
            y={center.y}
            html={`<span style="font-weight:${element.bold ? 700 : 400};font-style:${element.italic ? "italic" : "normal"}">${labelHtml(label, element.textMode || "text")}</span>`}
            color={baseColor}
            size={textSize}
          />
        </g>
      );
    }
    default:
      return null;
  }
}

/** Unsichtbare, breite Trefferfläche für die Auswahl. */
function HitArea({ element, cell, symbols, onPointerDown }: { element: SketchElement; cell: number; symbols: Map<string, SketchSymbol>; onPointerDown: (event: ReactPointerEvent) => void }) {
  const p1 = { x: element.x1 * cell, y: element.y1 * cell };
  const p2 = { x: element.x2 * cell, y: element.y2 * cell };
  const common = { stroke: "transparent", fill: "transparent", style: { cursor: "grab" } as CSSProperties, onPointerDown };
  const symbol = element.kind === "symbol" ? symbols.get(element.symbol ?? "") : undefined;
  if (["text", "dot", "terminal", "ground"].includes(element.kind) || symbol?.kind === "node") {
    return <circle cx={p1.x} cy={p1.y + (element.kind === "ground" ? 8 : 0)} r={symbol ? 22 : 14} {...common} />;
  }
  if (element.kind === "path") {
    const points = (element.points ?? []).map(([x, y]) => `${x * cell},${y * cell}`).join(" ");
    return <polyline points={points} {...common} fill={element.closed ? "transparent" : "none"} strokeWidth={14} />;
  }
  if (element.kind === "rect" || element.kind === "ellipse" || symbol?.kind === "shape") {
    return <rect x={Math.min(p1.x, p2.x) - 6} y={Math.min(p1.y, p2.y) - 6} width={Math.abs(p2.x - p1.x) + 12} height={Math.abs(p2.y - p1.y) + 12} {...common} />;
  }
  if (element.kind === "circle" || element.kind === "arc") {
    return <circle cx={p1.x} cy={p1.y} r={Math.max(8, Math.hypot(p2.x - p1.x, p2.y - p1.y))} {...common} fill="none" strokeWidth={12} />;
  }
  return <line x1={p1.x} y1={p1.y} x2={p2.x} y2={p2.y} {...common} strokeWidth={16} />;
}

type Drag =
  | { mode: "draw"; element: SketchElement }
  | { mode: "freehand"; element: SketchElement }
  | { mode: "move"; id: string; startX: number; startY: number; original: SketchElement }
  | { mode: "handle"; id: string; end: 1 | 2 | number }
  | { mode: "window"; startX: number; startY: number; left: number; top: number };

export type SketchPadProps = {
  initial: Sketch | null;
  editing: boolean;
  allowOnline: boolean;
  onInsert: (code: string, environment: string) => void;
  onClose: () => void;
};

let catalogCache: SketchCatalog | null = null;
/** Zwischenablage für Skizzenelemente (Strg+C/X/V). */
let sketchClipboard: SketchElement | null = null;

export function SketchPad({ initial, editing, allowOnline, onInsert, onClose }: SketchPadProps) {
  const t = useT();
  const language = useLanguage();
  const [elements, setElements] = useState<SketchElement[]>(initial?.elements ?? []);
  const [history, setHistory] = useState<SketchElement[][]>([]);
  const [future, setFuture] = useState<SketchElement[][]>([]);
  const [tool, setTool] = useState<Tool>("select");
  const [selected, setSelected] = useState<string | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [pathDraft, setPathDraft] = useState<SketchElement | null>(null);
  const [hover, setHover] = useState<{ x: number; y: number } | null>(null);
  const [zoom, setZoom] = useState(1);
  const canvasAreaRef = useRef<HTMLDivElement>(null);
  useCtrlWheel(canvasAreaRef, (direction) => setZoom((value) => Math.min(2.5, Math.max(0.5, value + direction * 0.25))));
  const [position, setPosition] = useState(() => ({ left: Math.max(16, window.innerWidth - 1180), top: 90 }));
  const [code, setCode] = useState({ code: "", environment: "tikzpicture" });
  const [preview, setPreview] = useState<{ image: string; error: string; busy: boolean }>({ image: "", error: "", busy: false });
  const [catalog, setCatalog] = useState<SketchCatalog | null>(catalogCache);
  const [catalogError, setCatalogError] = useState("");
  const [query, setQuery] = useState("");
  const [openCategories, setOpenCategories] = useState<Set<string>>(() => new Set(["resistors", "sources", "shapes"]));
  const svgRef = useRef<SVGSVGElement>(null);
  const padRef = useRef<HTMLDivElement>(null);
  /** Fokus ins Skizzen-Fenster (Tasten wie Entf wirken auf die Zeichnung, nicht auf ein Eingabefeld). */
  const focusPad = () => padRef.current?.focus({ preventScroll: true });
  const cell = CELL * zoom;
  const english = language !== "de";

  useEffect(() => {
    if (catalogCache) return;
    api
      .sketchCatalog()
      .then((loaded) => {
        catalogCache = loaded;
        setCatalog(loaded);
      })
      .catch((error) => setCatalogError(errorText(error)));
  }, []);

  const symbols = useMemo(() => new Map((catalog?.symbols ?? []).map((symbol) => [symbol.id, symbol])), [catalog]);
  const symbolsRef = useRef(symbols);
  symbolsRef.current = symbols;
  const symbolName = (symbol: SketchSymbol) => (english ? symbol.en : symbol.de);

  // Code in Rust erzeugen (entprellt)
  useEffect(() => {
    const timer = window.setTimeout(() => {
      api
        .sketchToLatex({ version: 2, elements })
        .then(setCode)
        .catch((error) => setCode({ code: `% ${errorText(error)}`, environment: "tikzpicture" }));
    }, 150);
    return () => window.clearTimeout(timer);
  }, [elements]);

  // Aktueller Stand auch zwischen zwei Renderdurchläufen (schnelle Zeigerereignisse).
  const elementsRef = useRef<SketchElement[]>(elements);
  const dragRef = useRef<Drag | null>(null);
  const toolRef = useRef<Tool>(tool);
  toolRef.current = tool;
  const pathRef = useRef<SketchElement | null>(null);
  const setElementsNow = (next: SketchElement[]) => {
    const resolved = resolveRefs(next, symbolsRef.current);
    elementsRef.current = resolved;
    setElements(resolved);
  };
  const setDragNow = (next: Drag | null) => {
    dragRef.current = next;
    setDrag(next);
  };
  const setPathNow = (next: SketchElement | null) => {
    pathRef.current = next;
    setPathDraft(next);
  };
  // Verlauf in Refs (Tastatur-Handler laufen außerhalb von React; keine Nebenwirkungen in Update-Funktionen).
  const historyRef = useRef<SketchElement[][]>([]);
  const futureRef = useRef<SketchElement[][]>([]);
  const setHistoryNow = (past: SketchElement[][], next: SketchElement[][]) => {
    historyRef.current = past;
    futureRef.current = next;
    setHistory(past);
    setFuture(next);
  };
  const remember = () => setHistoryNow([...historyRef.current.slice(-80), elementsRef.current], []);
  const commit = (next: SketchElement[]) => {
    remember();
    setElementsNow(next);
  };

  const undo = () => {
    const past = historyRef.current;
    if (past.length === 0) return;
    const previous = past[past.length - 1];
    setHistoryNow(past.slice(0, -1), [elementsRef.current, ...futureRef.current]);
    elementsRef.current = previous;
    setElements(previous);
  };
  const redo = () => {
    const [next, ...rest] = futureRef.current;
    if (!next) return;
    setHistoryNow([...historyRef.current, elementsRef.current], rest);
    elementsRef.current = next;
    setElements(next);
  };

  const selectedElement = elements.find((element) => element.id === selected) ?? null;
  const selectedSymbol = selectedElement
    ? symbols.get(selectedElement.kind === "component" ? LEGACY_COMPONENTS[selectedElement.component ?? "R"] ?? "R" : selectedElement.symbol ?? "")
    : undefined;
  const update = (patch: Partial<SketchElement>) => {
    if (!selectedElement) return;
    commit(elementsRef.current.map((element) => (element.id === selectedElement.id ? { ...element, ...patch } : element)));
  };
  const remove = () => {
    if (!selectedElement) return;
    commit(elementsRef.current.filter((element) => element.id !== selectedElement.id));
    setSelected(null);
  };
  const duplicate = () => {
    if (!selectedElement) return;
    const copy: SketchElement = {
      ...structuredClone(selectedElement),
      id: newId(),
      name: selectedElement.name ? `${selectedElement.name.replace(/\d+$/, "")}${nextNodeNumber(selectedElement.name.replace(/\d+$/, ""))}` : selectedElement.name,
      x1: selectedElement.x1 + 2,
      y1: selectedElement.y1 + 2,
      x2: selectedElement.x2 + 2,
      y2: selectedElement.y2 + 2,
      points: selectedElement.points?.map(([x, y]) => [x + 2, y + 2]),
      ref1: "",
      ref2: "",
    };
    commit([...elementsRef.current, copy]);
    setSelected(copy.id);
  };
  const rotateSelected = (degrees: number) => {
    if (!selectedElement) return;
    const symbol = selectedSymbol;
    if (selectedElement.kind === "text" || (selectedElement.kind === "symbol" && symbol?.kind !== "bipole")) {
      update({ rotation: (((selectedElement.rotation ?? 0) + degrees) % 360 + 360) % 360 });
    } else {
      // Zweipunkt-Elemente um den Anfangspunkt drehen
      const angle = rad(degrees);
      const dx = selectedElement.x2 - selectedElement.x1;
      const dy = selectedElement.y2 - selectedElement.y1;
      update({ x2: selectedElement.x1 + Math.round(dx * Math.cos(angle) - dy * Math.sin(angle)), y2: selectedElement.y1 + Math.round(dx * Math.sin(angle) + dy * Math.cos(angle)) });
    }
  };

  function nextNodeNumber(prefix: string): number {
    const used = elementsRef.current
      .map((element) => element.name ?? "")
      .filter((name) => name.startsWith(prefix))
      .map((name) => Number(name.slice(prefix.length)))
      .filter(Number.isFinite);
    return used.length ? Math.max(...used) + 1 : 1;
  }

  /** Namensvorschlag nach Bauteilart (Q für Transistoren, U für ICs/Verstärker …). */
  const defaultName = (symbol: SketchSymbol): string => {
    const prefix =
      symbol.category === "transistors" ? "Q"
        : symbol.category === "amplifiers" || symbol.category === "digital" ? "U"
          : symbol.category.startsWith("logic") ? "G"
            : symbol.category === "tubes" ? "V"
              : symbol.category === "switches" ? "S"
                : symbol.category === "transformers" ? "T"
                  : "N";
    return `${prefix}${nextNodeNumber(prefix)}`;
  };

  /** Auswahl verschieben (Pfeiltasten); Leitungsenden lösen sich dabei von Anschlüssen. */
  const nudge = (dx: number, dy: number) => {
    if (!selectedElement) return;
    update({
      x1: selectedElement.x1 + dx,
      y1: selectedElement.y1 + dy,
      x2: selectedElement.x2 + dx,
      y2: selectedElement.y2 + dy,
      points: selectedElement.points?.map(([x, y]) => [x + dx, y + dy]),
      ref1: "",
      ref2: "",
    });
  };
  const copySelected = () => {
    if (selectedElement) sketchClipboard = structuredClone(selectedElement);
  };
  const paste = () => {
    if (!sketchClipboard) return;
    const copy: SketchElement = {
      ...structuredClone(sketchClipboard),
      id: newId(),
      x1: sketchClipboard.x1 + 2,
      y1: sketchClipboard.y1 + 2,
      x2: sketchClipboard.x2 + 2,
      y2: sketchClipboard.y2 + 2,
      points: sketchClipboard.points?.map(([x, y]) => [x + 2, y + 2]),
      ref1: "",
      ref2: "",
    };
    sketchClipboard = copy;
    commit([...elementsRef.current, copy]);
    setSelected(copy.id);
  };

  // Tastatur wie in üblichen Zeichenprogrammen:
  // Entf/Rücktaste löschen · Pfeile verschieben (Umschalt ×5, Alt ½) · Strg+C/X/V/D · Strg+Z/Y ·
  // R drehen · Eingabe/Doppelklick beendet Linienzüge · Esc bricht ab · V Auswahl, L Linie, T Text · Strg+±/0 Zoom
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT" || target.isContentEditable)) return;
      // Fokus im Skizzen-Fenster oder (nach Klick auf die Zeichenfläche) auf der Seite selbst
      if (target && target !== document.body && !target.closest(".sketch-pad")) return;
      const mod = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();
      const step = event.shiftKey ? 5 : event.altKey ? 0.5 : 1;
      let handled = true;
      if ((event.key === "Delete" || event.key === "Backspace") && selected) remove();
      else if (mod && key === "z") (event.shiftKey ? redo : undo)();
      else if (mod && key === "y") redo();
      else if (mod && key === "d") duplicate();
      else if (mod && key === "c") copySelected();
      else if (mod && key === "x") {
        copySelected();
        remove();
      } else if (mod && key === "v") paste();
      else if (mod && (event.key === "+" || event.key === "=")) setZoom((value) => Math.min(2.5, value + 0.25));
      else if (mod && event.key === "-") setZoom((value) => Math.max(0.5, value - 0.25));
      else if (mod && event.key === "0") setZoom(1);
      else if (event.key === "ArrowLeft" && selected) nudge(-step, 0);
      else if (event.key === "ArrowRight" && selected) nudge(step, 0);
      else if (event.key === "ArrowUp" && selected) nudge(0, -step);
      else if (event.key === "ArrowDown" && selected) nudge(0, step);
      else if (!mod && key === "r" && selected) rotateSelected(event.shiftKey ? -90 : 90);
      else if (event.key === "Enter" && pathRef.current) finishPath();
      else if (event.key === "Escape") {
        if (pathRef.current) finishPath();
        else if (dragRef.current?.mode === "draw") setDragNow(null);
        setSelected(null);
        setTool("select");
      } else if (!mod && !event.altKey && key === "v") setTool("select");
      else if (!mod && !event.altKey && key === "l") setTool("line");
      else if (!mod && !event.altKey && key === "t") setTool("text");
      else handled = false;
      if (handled) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const rawPoint = (event: { clientX: number; clientY: number }) => {
    const rect = svgRef.current?.getBoundingClientRect();
    if (!rect) return { x: 0, y: 0 };
    return { x: (event.clientX - rect.left) / cell, y: (event.clientY - rect.top) / cell };
  };

  /** Rasterpunkt bzw. nächster Anschluss (innerhalb von 0,6 Rastereinheiten). */
  const snapPoint = (event: { clientX: number; clientY: number; altKey?: boolean }, exclude?: string) => {
    const raw = rawPoint(event);
    let best: { x: number; y: number; ref: string; distance: number } | null = null;
    for (const element of elementsRef.current) {
      if (element.kind !== "symbol" || element.id === exclude) continue;
      for (const pin of nodePins(element, symbolsRef.current.get(element.symbol ?? ""))) {
        const distance = Math.hypot(pin.x - raw.x, pin.y - raw.y);
        if (distance < 0.6 && (!best || distance < best.distance)) best = { x: pin.x, y: pin.y, ref: `${element.id}.${pin.name}`, distance };
      }
    }
    if (best) return { x: best.x, y: best.y, ref: best.ref };
    const step = event.altKey ? 0.5 : 1;
    return {
      x: Math.max(0, Math.min(COLUMNS, Math.round(raw.x / step) * step)),
      y: Math.max(0, Math.min(ROWS, Math.round(raw.y / step) * step)),
      ref: "",
    };
  };

  const finishPath = () => {
    const draft = pathRef.current;
    setPathNow(null);
    if (!draft || (draft.points?.length ?? 0) < 2) return;
    commit([...elementsRef.current, draft]);
    setSelected(draft.id);
  };

  const startDraw = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (event.button !== 0) return;
    focusPad();
    const point = snapPoint(event);
    const current = toolRef.current;
    if (current === "select") {
      setSelected(null);
      return;
    }
    if (current === "path" || current === "polygon" || current === "curve") {
      const draft = pathRef.current;
      if (draft) {
        const last = draft.points?.[draft.points.length - 1];
        if (event.detail >= 2 || (last && last[0] === point.x && last[1] === point.y)) {
          finishPath();
          return;
        }
        setPathNow({ ...draft, points: [...(draft.points ?? []), [point.x, point.y]] });
      } else {
        setPathNow({ id: newId(), kind: "path", x1: point.x, y1: point.y, x2: point.x, y2: point.y, points: [[point.x, point.y]], closed: current === "polygon", smooth: current === "curve" });
      }
      return;
    }
    if (current === "freehand") {
      const raw = rawPoint(event);
      capturePointer(event.currentTarget as Element, event.pointerId);
      setDragNow({ mode: "freehand", element: { id: newId(), kind: "path", x1: raw.x, y1: raw.y, x2: raw.x, y2: raw.y, points: [[raw.x, raw.y]], smooth: true } });
      return;
    }
    if (current.startsWith("symbol:")) {
      const symbol = symbolsRef.current.get(current.slice("symbol:".length));
      if (!symbol) return;
      const base: SketchElement = { id: newId(), kind: "symbol", symbol: symbol.id, x1: point.x, y1: point.y, x2: point.x, y2: point.y, label: "" };
      if (symbol.kind === "node") {
        const element = { ...base, name: defaultName(symbol), labelPosition: "above" };
        commit([...elementsRef.current, element]);
        setSelected(element.id);
        return;
      }
      capturePointer(event.currentTarget as Element, event.pointerId);
      setDragNow({ mode: "draw", element: { ...base, ref1: point.ref } });
      return;
    }
    const single = current === "text" || current === "dot" || current === "terminal";
    const element: SketchElement = {
      id: newId(),
      kind: current as SketchElement["kind"],
      x1: point.x,
      y1: point.y,
      x2: point.x,
      y2: point.y,
      label: current === "text" ? "Text" : "",
      ref1: current === "line" || current === "arrow" ? point.ref : "",
      ...(current === "arc" ? { sweep: 90 } : {}),
    };
    if (single) {
      commit([...elementsRef.current, element]);
      setSelected(element.id);
      return;
    }
    capturePointer(event.currentTarget as Element, event.pointerId);
    setDragNow({ mode: "draw", element });
  };

  const onMove = (event: ReactPointerEvent) => {
    const drag = dragRef.current;
    if (!drag) {
      if (pathRef.current) setHover(snapPoint(event));
      return;
    }
    if (drag.mode === "window") {
      setPosition({ left: Math.max(0, drag.left + event.clientX - drag.startX), top: Math.max(0, drag.top + event.clientY - drag.startY) });
      return;
    }
    if (drag.mode === "freehand") {
      const raw = rawPoint(event);
      const points = drag.element.points ?? [];
      const last = points[points.length - 1];
      if (!last || Math.hypot(last[0] - raw.x, last[1] - raw.y) > 0.35) {
        setDragNow({ ...drag, element: { ...drag.element, points: [...points, [Math.round(raw.x * 4) / 4, Math.round(raw.y * 4) / 4]] } });
      }
      return;
    }
    if (drag.mode === "draw") {
      const point = snapPoint(event);
      const lineLike = ["line", "arrow"].includes(drag.element.kind) || drag.element.kind === "symbol";
      setDragNow({ ...drag, element: { ...drag.element, x2: point.x, y2: point.y, ref2: lineLike ? point.ref : "" } });
    } else if (drag.mode === "move") {
      const point = snapPoint(event, drag.id);
      const dx = Math.round(point.x - drag.startX);
      const dy = Math.round(point.y - drag.startY);
      setElementsNow(
        elementsRef.current.map((element) =>
          element.id === drag.id
            ? {
                ...element,
                x1: drag.original.x1 + dx,
                y1: drag.original.y1 + dy,
                x2: drag.original.x2 + dx,
                y2: drag.original.y2 + dy,
                points: drag.original.points?.map(([x, y]) => [x + dx, y + dy]),
                ref1: dx || dy ? "" : drag.original.ref1,
                ref2: dx || dy ? "" : drag.original.ref2,
              }
            : element,
        ),
      );
    } else if (drag.mode === "handle") {
      const point = snapPoint(event, drag.id);
      setElementsNow(
        elementsRef.current.map((element) => {
          if (element.id !== drag.id) return element;
          if (drag.end >= 10) {
            const index = drag.end - 10;
            const points = [...(element.points ?? [])];
            points[index] = [point.x, point.y];
            return { ...element, points };
          }
          return drag.end === 1 ? { ...element, x1: point.x, y1: point.y, ref1: point.ref } : { ...element, x2: point.x, y2: point.y, ref2: point.ref };
        }),
      );
    }
  };

  const onUp = () => {
    const drag = dragRef.current;
    if (!drag) return;
    if (drag.mode === "draw") {
      const { element } = drag;
      if (element.x1 !== element.x2 || element.y1 !== element.y2) {
        commit([...elementsRef.current, element]);
        setSelected(element.id);
      }
    } else if (drag.mode === "freehand") {
      if ((drag.element.points?.length ?? 0) > 2) {
        commit([...elementsRef.current, drag.element]);
        setSelected(drag.element.id);
      }
    }
    setDragNow(null);
  };

  const startMove = (element: SketchElement) => (event: ReactPointerEvent) => {
    if (toolRef.current !== "select") return;
    event.stopPropagation();
    focusPad();
    if (padRef.current) capturePointer(padRef.current, event.pointerId);
    const point = snapPoint(event, element.id);
    setSelected(element.id);
    remember();
    setDragNow({ mode: "move", id: element.id, startX: point.x, startY: point.y, original: element });
  };

  const startHandle = (element: SketchElement, end: number) => (event: ReactPointerEvent) => {
    event.stopPropagation();
    if (padRef.current) capturePointer(padRef.current, event.pointerId);
    remember();
    setDragNow({ mode: "handle", id: element.id, end });
  };

  const renderPreview = async () => {
    setPreview({ image: "", error: "", busy: true });
    try {
      const image = await api.renderTikz(code.code, code.environment, "", allowOnline);
      setPreview({ image, error: "", busy: false });
    } catch (error) {
      setPreview({ image: "", error: errorText(error), busy: false });
    }
  };

  const grid = useMemo(() => {
    const lines: ReactNode[] = [];
    for (let x = 0; x <= COLUMNS; x += 1) {
      lines.push(<line key={`v${x}`} x1={x * cell} y1={0} x2={x * cell} y2={ROWS * cell} className={x % 2 === 0 ? "sketch-grid major" : "sketch-grid"} />);
    }
    for (let y = 0; y <= ROWS; y += 1) {
      lines.push(<line key={`h${y}`} x1={0} y1={y * cell} x2={COLUMNS * cell} y2={y * cell} className={y % 2 === 0 ? "sketch-grid major" : "sketch-grid"} />);
    }
    return lines;
  }, [cell]);

  const toolButton = (id: Tool, label: string, symbol: ReactNode) => (
    <button
      key={id}
      type="button"
      className={`sketch-tool${tool === id ? " active" : ""}`}
      title={label}
      aria-label={label}
      aria-pressed={tool === id}
      onClick={() => {
        if (pathRef.current) finishPath();
        setTool(id);
      }}
    >
      {symbol}
    </button>
  );

  // Palette: Kategorien mit Symbolen, Suche über Namen (de/en) und CircuiTikZ-Namen
  const palette = useMemo(() => {
    if (!catalog) return [];
    const needle = query.trim().toLowerCase();
    return catalog.categories
      .map((category) => ({
        category,
        items: catalog.symbols.filter(
          (symbol) =>
            symbol.category === category.key &&
            (!needle || symbol.de.toLowerCase().includes(needle) || symbol.en.toLowerCase().includes(needle) || symbol.id.toLowerCase().includes(needle)),
        ),
      }))
      .filter((group) => group.items.length > 0);
  }, [catalog, query]);

  const drawing = drag?.mode === "draw" || drag?.mode === "freehand" ? drag.element : pathDraft ? { ...pathDraft, points: hover ? [...(pathDraft.points ?? []), [hover.x, hover.y] as [number, number]] : pathDraft.points } : null;
  const twoPoint = selectedElement && !["text", "dot", "terminal", "ground", "path"].includes(selectedElement.kind) && !(selectedElement.kind === "symbol" && selectedSymbol?.kind === "node");
  const pins = useMemo(
    () => elements.filter((element) => element.kind === "symbol").flatMap((element) => nodePins(element, symbols.get(element.symbol ?? "")).map((pin) => ({ ...pin, id: element.id }))),
    [elements, symbols],
  );
  const wireTool = tool === "line" || tool === "arrow" || tool === "path" || tool.startsWith("symbol:");

  const kindTitle = (element: SketchElement): string => {
    if (element.kind === "symbol" || element.kind === "component") return selectedSymbol ? symbolName(selectedSymbol) : t("Bauteil");
    const names: Record<string, string> = {
      line: "Linie", arrow: "Pfeil", path: element.closed ? "Polygon" : element.smooth ? "Kurve" : "Pfad", rect: "Rechteck", circle: "Kreis",
      ellipse: "Ellipse", arc: "Bogen", text: "Text", dot: "Verbindungspunkt", terminal: "Anschluss", ground: "Masse",
    };
    return t(names[element.kind] ?? "Element");
  };

  const styleFields = (element: SketchElement) => {
    const lineLike = ["line", "arrow", "path", "arc", "rect", "circle", "ellipse"].includes(element.kind) || selectedSymbol?.kind === "shape";
    const fillable = ["rect", "circle", "ellipse"].includes(element.kind) || (element.kind === "path" && element.closed) || selectedSymbol?.kind === "shape";
    const arrows = ["line", "arrow", "arc"].includes(element.kind) || (element.kind === "path" && !element.closed);
    return (
      <>
        <div className="sketch-color-row">
          <label>
            {t("Farbe")}
            <input type="color" value={element.color || "#1f2937"} onChange={(event) => update({ color: event.currentTarget.value })} />
          </label>
          {fillable && (
            <label>
              {t("Füllung")}
              <input type="color" value={element.fill || "#ffffff"} onChange={(event) => update({ fill: event.currentTarget.value })} />
            </label>
          )}
          {(element.color || element.fill) && (
            <button type="button" className="small" onClick={() => update({ color: "", fill: "" })}>{t("Standard")}</button>
          )}
        </div>
        {lineLike && catalog && (
          <div className="sketch-two">
            <label>
              {t("Linienstärke")}
              <select value={element.lineWidth || (element.thick ? "thick" : "")} onChange={(event) => update({ lineWidth: event.currentTarget.value, thick: false })}>
                <option value="">{t("Standard")}</option>
                {catalog.lineWidths.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
            <label>
              {t("Linienart")}
              <select value={element.dash || (element.dashed ? "dashed" : "")} onChange={(event) => update({ dash: event.currentTarget.value, dashed: false })}>
                <option value="">{t("durchgezogen")}</option>
                {catalog.dashPatterns.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          </div>
        )}
        {arrows && catalog && (
          <div className="sketch-two">
            <label>
              {t("Pfeil am Anfang")}
              <select value={element.arrowStart ?? ""} onChange={(event) => update({ arrowStart: event.currentTarget.value })}>
                <option value="">{t("keiner")}</option>
                {catalog.arrowTips.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
            <label>
              {t("Pfeil am Ende")}
              <select
                value={element.arrowEnd || (element.kind === "arrow" && !element.arrowStart ? "Stealth" : "")}
                onChange={(event) => update({ arrowEnd: event.currentTarget.value, kind: element.kind === "arrow" ? "line" : element.kind })}
              >
                <option value="">{t("keiner")}</option>
                {catalog.arrowTips.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          </div>
        )}
      </>
    );
  };

  const textFields = (element: SketchElement, defaultMode: string) => (
    <>
      <div className="sketch-two">
        <label>
          {t("Schriftgröße")}
          <select value={element.fontSize ?? ""} onChange={(event) => update({ fontSize: event.currentTarget.value })}>
            <option value="">{t("Standard")}</option>
            {(catalog?.fontSizes ?? []).map((value) => <option key={value} value={value}>{value}</option>)}
          </select>
        </label>
        <label>
          {t("Satz")}
          <select value={element.textMode || defaultMode} onChange={(event) => update({ textMode: event.currentTarget.value })}>
            <option value="auto">{t("automatisch")}</option>
            <option value="text">{t("Text")}</option>
            <option value="math">{t("Formel")}</option>
          </select>
        </label>
      </div>
      <div className="sketch-checks">
        <label className="checkbox">
          <input type="checkbox" checked={Boolean(element.bold)} onChange={(event) => update({ bold: event.currentTarget.checked })} />
          {t("Fett")}
        </label>
        <label className="checkbox">
          <input type="checkbox" checked={Boolean(element.italic)} onChange={(event) => update({ italic: event.currentTarget.checked })} />
          {t("Kursiv")}
        </label>
      </div>
    </>
  );

  const properties = (element: SketchElement) => {
    const symbol = selectedSymbol;
    const isBipole = (element.kind === "symbol" && symbol?.kind === "bipole") || element.kind === "component";
    const isNode = element.kind === "symbol" && symbol?.kind === "node";
    const isShape = element.kind === "symbol" && symbol?.kind === "shape";
    return (
      <div className="sketch-properties">
        <h4>
          {kindTitle(element)}
          {symbol && <code className="sketch-symbol-id">{symbol.id}</code>}
        </h4>
        {element.kind === "text" && (
          <>
            <label>
              {t("Text")}
              <textarea rows={2} value={element.label ?? ""} onChange={(event) => update({ label: event.currentTarget.value })} />
            </label>
            {textFields(element, "auto")}
          </>
        )}
        {(isShape || ["rect", "circle", "line", "arrow", "terminal"].includes(element.kind)) && (
          <label>
            {isShape ? t("Text in der Form") : t("Beschriftung")}
            <input value={element.label ?? ""} onChange={(event) => update({ label: event.currentTarget.value })} />
          </label>
        )}
        {isShape && textFields(element, "text")}
        {isBipole && (
          <>
            <label>
              {t("Beschriftung (Name)")}
              <input value={element.label ?? ""} placeholder="R_1" onChange={(event) => update({ label: event.currentTarget.value })} />
            </label>
            <label className="checkbox">
              <input type="checkbox" checked={Boolean(element.labelBelow)} onChange={(event) => update({ labelBelow: event.currentTarget.checked })} />
              {t("Beschriftung auf der anderen Seite")}
            </label>
            <label>
              {t("Wert/Beschreibung")}
              <input value={element.annotation ?? ""} placeholder="10 kΩ" onChange={(event) => update({ annotation: event.currentTarget.value })} />
            </label>
            <div className="sketch-two">
              <label>
                {t("Spannungspfeil")}
                <input value={element.voltage ?? ""} placeholder="U_R" onChange={(event) => update({ voltage: event.currentTarget.value })} />
              </label>
              <label>
                {t("Seite")}
                <select value={element.voltageSide ?? ""} onChange={(event) => update({ voltageSide: event.currentTarget.value })}>
                  <option value="">{t("Standard")}</option>
                  <option value="^">{t("oben")}</option>
                  <option value="_">{t("unten")}</option>
                </select>
              </label>
            </div>
            <div className="sketch-two">
              <label>
                {t("Strompfeil")}
                <input value={element.current ?? ""} placeholder="I" onChange={(event) => update({ current: event.currentTarget.value })} />
              </label>
              <label>
                {t("Richtung")}
                <select value={element.currentDir ?? ""} onChange={(event) => update({ currentDir: event.currentTarget.value })}>
                  <option value="">{t("Standard")}</option>
                  <option value=">^">→ {t("oben")}</option>
                  <option value=">_">→ {t("unten")}</option>
                  <option value="<^">← {t("oben")}</option>
                  <option value="<_">← {t("unten")}</option>
                </select>
              </label>
            </div>
            <label>
              {t("Flusspfeil")}
              <input value={element.flow ?? ""} placeholder="P" onChange={(event) => update({ flow: event.currentTarget.value })} />
            </label>
            <div className="sketch-checks">
              <label className="checkbox">
                <input type="checkbox" checked={Boolean(element.invert)} onChange={(event) => update({ invert: event.currentTarget.checked })} />
                {t("Invertieren")}
              </label>
              <label className="checkbox">
                <input type="checkbox" checked={Boolean(element.mirror)} onChange={(event) => update({ mirror: event.currentTarget.checked })} />
                {t("Spiegeln")}
              </label>
            </div>
            <button type="button" onClick={() => update({ x1: element.x2, y1: element.y2, x2: element.x1, y2: element.y1, ref1: element.ref2, ref2: element.ref1 })}>
              {t("Richtung umkehren")}
            </button>
          </>
        )}
        {isNode && symbol && (
          <>
            <div className="sketch-two">
              <label>
                {t("Name")}
                <input value={element.name ?? ""} placeholder="Q1" onChange={(event) => update({ name: event.currentTarget.value.replace(/[^A-Za-z0-9]/g, "") })} />
              </label>
              <label>
                {t("Beschriftung")}
                <input value={element.label ?? ""} placeholder="Q_1" onChange={(event) => update({ label: event.currentTarget.value })} />
              </label>
            </div>
            <label>
              {t("Position der Beschriftung")}
              <select value={element.labelPosition || "above"} onChange={(event) => update({ labelPosition: event.currentTarget.value })}>
                <option value="above">{t("oben")}</option>
                <option value="below">{t("unten")}</option>
                <option value="left">{t("links")}</option>
                <option value="right">{t("rechts")}</option>
                <option value="inside">{t("im Symbol")}</option>
              </select>
            </label>
            <div className="sketch-checks">
              <label className="checkbox">
                <input type="checkbox" checked={Boolean(element.mirror)} onChange={(event) => update({ mirror: event.currentTarget.checked })} />
                {t("Spiegeln")}
              </label>
              <label className="checkbox">
                <input type="checkbox" checked={Boolean(element.flip)} onChange={(event) => update({ flip: event.currentTarget.checked })} />
                {t("Kippen")}
              </label>
            </div>
            {symbol.anchors.length > 0 && (
              <p className="sketch-pins">
                {t("Anschlüsse")}: {symbol.anchors.map(([name]) => <code key={name}>{name}</code>)}
              </p>
            )}
          </>
        )}
        {(element.kind === "text" || isNode || isShape) && (
          <label>
            {t("Drehung")}
            <span className="sketch-rotation">
              <input type="number" step={15} value={element.rotation ?? 0} onChange={(event) => update({ rotation: Number(event.currentTarget.value) || 0 })} />
              <button type="button" className="small" title={t("90° drehen (R)")} onClick={() => rotateSelected(90)}>↻ 90°</button>
            </span>
          </label>
        )}
        {element.kind === "path" && (
          <div className="sketch-checks">
            <label className="checkbox">
              <input type="checkbox" checked={Boolean(element.closed)} onChange={(event) => update({ closed: event.currentTarget.checked })} />
              {t("Geschlossen")}
            </label>
            <label className="checkbox">
              <input type="checkbox" checked={Boolean(element.smooth)} onChange={(event) => update({ smooth: event.currentTarget.checked })} />
              {t("Geglättet")}
            </label>
          </div>
        )}
        {element.kind === "arc" && (
          <label>
            {t("Winkel (°, gegen den Uhrzeigersinn)")}
            <input type="number" step={15} value={element.sweep ?? 90} onChange={(event) => update({ sweep: Number(event.currentTarget.value) || 90 })} />
          </label>
        )}
        {element.kind !== "ground" && element.kind !== "dot" && styleFields(element)}
        <div className="sketch-buttons">
          <button type="button" onClick={duplicate}>{t("Duplizieren")}</button>
          <button type="button" className="danger" onClick={remove}>{t("Löschen")}</button>
        </div>
      </div>
    );
  };

  return (
    <div
      className="sketch-pad"
      ref={padRef}
      tabIndex={-1}
      style={{ left: position.left, top: position.top }}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
      role="dialog"
      aria-label={t("Skizze")}
    >
      <header
        className="sketch-header"
        onPointerDown={(event) => {
          if ((event.target as HTMLElement).closest("button")) return;
          capturePointer(event.currentTarget as Element, event.pointerId);
          setDragNow({ mode: "window", startX: event.clientX, startY: event.clientY, left: position.left, top: position.top });
        }}
      >
        <strong>{t("Skizze (TikZ/CircuiTikZ)")}</strong>
        <span className="sketch-hint">{t("Raster 0,5 cm (Alt: halbe Schritte) · Entf: löschen · R: drehen · Strg+D: duplizieren · Strg+Z/Y")}</span>
        <button type="button" className="icon-button" onClick={onClose} aria-label={t("Schließen")}>
          <X size={16} />
        </button>
      </header>
      <div className="sketch-toolbar">
        {toolButton("select", t("Auswählen/Verschieben (V)"), <Hand size={16} />)}
        <span className="sketch-separator" />
        {toolButton("line", t("Linie/Leitung"), "╱")}
        {toolButton("arrow", t("Pfeil"), "→")}
        {toolButton("path", t("Linienzug (Doppelklick oder Eingabe beendet)"), "⌇")}
        {toolButton("polygon", t("Vieleck"), "⬠")}
        {toolButton("curve", t("Kurve"), "∿")}
        {toolButton("freehand", t("Freihand"), "✎")}
        <span className="sketch-separator" />
        {toolButton("rect", t("Rechteck"), "▭")}
        {toolButton("circle", t("Kreis"), "◯")}
        {toolButton("ellipse", t("Ellipse"), "⬭")}
        {toolButton("arc", t("Bogen"), "◜")}
        {toolButton("text", t("Text"), "T")}
        <span className="sketch-separator" />
        {toolButton("dot", t("Verbindungspunkt"), "●")}
        {toolButton("terminal", t("Anschluss"), "○")}
        <span className="sketch-separator" />
        <button type="button" className="sketch-tool" title={t("Verkleinern")} onClick={() => setZoom((value) => Math.max(0.5, value - 0.25))}>−</button>
        <span className="sketch-zoom">{Math.round(zoom * 100)} %</span>
        <button type="button" className="sketch-tool" title={t("Vergrößern")} onClick={() => setZoom((value) => Math.min(2.5, value + 0.25))}>+</button>
        <button type="button" className="sketch-tool" title={t("Rückgängig")} disabled={history.length === 0} onClick={undo}>↶</button>
        <button type="button" className="sketch-tool" title={t("Wiederholen")} disabled={future.length === 0} onClick={redo}>↷</button>
      </div>
      <div className="sketch-body">
        <aside className="sketch-palette" aria-label={t("Bauteile und Formen")}>
          <label className="sketch-search">
            <Search size={14} />
            <input value={query} placeholder={t("Bauteil oder Form suchen …")} onChange={(event) => setQuery(event.currentTarget.value)} />
          </label>
          {!catalog && !catalogError && <p className="sketch-help">{t("Symbole werden geladen …")}</p>}
          {catalogError && <p className="node-error">{catalogError}</p>}
          {palette.map(({ category, items }) => {
            const open = query.trim() !== "" || openCategories.has(category.key);
            return (
              <section key={category.key} className="sketch-category">
                <button
                  type="button"
                  className="sketch-category-title"
                  aria-expanded={open}
                  onClick={() =>
                    setOpenCategories((value) => {
                      const next = new Set(value);
                      if (next.has(category.key)) next.delete(category.key);
                      else next.add(category.key);
                      return next;
                    })
                  }
                >
                  {open ? "▾" : "▸"} {english ? category.en : category.de} <small>{items.length}</small>
                </button>
                {open && (
                  <div className="sketch-symbols">
                    {items.map((symbol) => (
                      <button
                        key={symbol.id}
                        type="button"
                        className={`sketch-symbol${tool === `symbol:${symbol.id}` ? " active" : ""}`}
                        title={`${symbolName(symbol)} (${symbol.id})`}
                        aria-label={symbolName(symbol)}
                        onClick={() => {
                          if (pathRef.current) finishPath();
                          setTool(`symbol:${symbol.id}`);
                        }}
                      >
                        <img src={symbol.image} alt="" draggable={false} />
                      </button>
                    ))}
                  </div>
                )}
              </section>
            );
          })}
        </aside>
        <div className="sketch-canvas" ref={canvasAreaRef}>
          <svg
            ref={svgRef}
            width={COLUMNS * cell}
            height={ROWS * cell}
            className={`tool-${tool.split(":")[0]}${drag?.mode === "move" || drag?.mode === "handle" ? " dragging" : ""}`}
            onPointerDown={startDraw}
            onDoubleClick={() => pathRef.current && finishPath()}
          >
            <rect width={COLUMNS * cell} height={ROWS * cell} fill="#fff" />
            {grid}
            {elements.map((element) => (
              <g key={element.id}>
                <ElementShape element={element} selected={element.id === selected} cell={cell} catalog={catalog} symbols={symbols} />
                <HitArea element={element} cell={cell} symbols={symbols} onPointerDown={startMove(element)} />
              </g>
            ))}
            {drawing && <ElementShape element={drawing} selected cell={cell} catalog={catalog} symbols={symbols} />}
            {(wireTool || drag?.mode === "handle") &&
              pins.map((pin) => <circle key={`${pin.id}-${pin.name}`} className="sketch-pin" cx={pin.x * cell} cy={pin.y * cell} r={4} />)}
            {selectedElement && tool === "select" && (
              <g className="sketch-handles">
                {selectedElement.kind === "path" ? (
                  (selectedElement.points ?? []).map(([x, y], index) => (
                    <circle key={index} cx={x * cell} cy={y * cell} r={5} onPointerDown={startHandle(selectedElement, 10 + index)} />
                  ))
                ) : (
                  <>
                    <circle cx={selectedElement.x1 * cell} cy={selectedElement.y1 * cell} r={6} onPointerDown={startHandle(selectedElement, 1)} />
                    {twoPoint && <circle cx={selectedElement.x2 * cell} cy={selectedElement.y2 * cell} r={6} onPointerDown={startHandle(selectedElement, 2)} />}
                  </>
                )}
              </g>
            )}
          </svg>
        </div>
        <aside className="sketch-sidebar">
          {selectedElement ? (
            properties(selectedElement)
          ) : (
            <p className="sketch-help">
              {t("Werkzeug oder Symbol wählen und auf dem Raster ziehen bzw. klicken. Zweipole (Widerstand, Quelle …) werden zwischen Anfangs- und Endpunkt gezeichnet, Mehrpole (Transistor, OPV …) per Klick gesetzt. Leitungen docken an den blauen Anschlusspunkten an.")}
            </p>
          )}
          <div className="sketch-code">
            <div className="sketch-code-header">
              <span>{code.environment}</span>
              <button type="button" onClick={() => void navigator.clipboard?.writeText(`\\begin{${code.environment}}\n${code.code}\n\\end{${code.environment}}`)}>{t("Kopieren")}</button>
            </div>
            <pre>{code.code.split("\n").slice(1).join("\n") || t("(noch leer)")}</pre>
          </div>
          {preview.image && <img className="sketch-preview" src={preview.image} alt={t("Vorschau")} />}
          {preview.error && <div className="node-error">{preview.error.split("\n")[0]}</div>}
        </aside>
      </div>
      <footer className="sketch-footer">
        <button type="button" onClick={() => void renderPreview()} disabled={preview.busy || elements.length === 0}>
          {preview.busy ? t("Kompiliere …") : t("Vorschau kompilieren")}
        </button>
        <button type="button" onClick={() => commit([])} disabled={elements.length === 0}>{t("Alles löschen")}</button>
        <span className="grow" />
        <button type="button" onClick={onClose}>{t("Schließen")}</button>
        <button type="button" className="primary" disabled={elements.length === 0} onClick={() => onInsert(code.code, code.environment)}>
          {editing ? t("Skizze übernehmen") : t("An Cursorposition einfügen")}
        </button>
      </footer>
    </div>
  );
}
