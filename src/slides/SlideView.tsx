/**
 * Darstellung einer Folie in HTML/SVG – dieselbe Komponente für Arbeitsfläche,
 * Miniaturen und Bildschirmpräsentation. Maße in mm × `scale` (px pro mm);
 * Schriftgrößen in pt wie im Beamer-Export (1 pt = 25,4/72 mm).
 */
import { memo, useMemo } from "react";
import type { CSSProperties, ReactNode } from "react";
import { renderMath } from "../latex/miniRender";
import { PT, hasText, slideSize } from "./model";
import type { Slide, SlideDeck, SlideElement } from "./model";
import { slideTextHtml } from "./textExtensions";

export const FONT_STACK: Record<"sans" | "serif", string> = {
  sans: "\"Latin Modern Sans\", \"Latin Modern Roman\", sans-serif",
  serif: "\"Latin Modern Roman\", serif",
};

type Props = {
  slide: Slide;
  deck: SlideDeck;
  /** px pro mm */
  scale: number;
  /** Platzhalter-Hinweise und leere Rahmen zeigen (nur im Editor) */
  editing?: boolean;
  /** Element, dessen Text gerade bearbeitet wird: `editor` ersetzt den Inhalt */
  editingId?: string | null;
  editor?: ReactNode;
  placeholderText?: (element: SlideElement) => string;
  className?: string;
  children?: ReactNode;
};

/** Pfeilspitze wie `Stealth[length=3mm]` im Export. */
function ArrowHead({ id, color }: { id: string; color: string }) {
  return (
    <defs>
      <marker id={id} viewBox="0 0 10 10" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth="3" markerHeight="3" orient="auto-start-reverse">
        <path d="M0,0 L10,5 L0,10 L3,5 z" fill={color} />
      </marker>
    </defs>
  );
}

function ShapeGraphic({ element, scale, accent }: { element: SlideElement; scale: number; accent: string }) {
  const w = element.w * scale;
  const h = element.h * scale;
  const strokeWidth = element.stroke ? Math.max(0.5, element.strokeWidth * PT * scale) : 0;
  const isLine = element.shape === "line" || element.shape === "arrow";
  const stroke = element.stroke || (isLine ? "#1f2937" : "none");
  const fill = isLine ? "none" : element.fill || "none";
  if (!isLine && fill === "none" && stroke === "none") return null;
  const inset = strokeWidth / 2;
  let shape: ReactNode;
  switch (element.shape) {
    case "ellipse":
      shape = <ellipse cx={w / 2} cy={h / 2} rx={Math.max(0, w / 2 - inset)} ry={Math.max(0, h / 2 - inset)} />;
      break;
    case "triangle":
      shape = <polygon points={`${w / 2},${inset} ${w - inset},${h - inset} ${inset},${h - inset}`} />;
      break;
    case "diamond":
      shape = <polygon points={`${w / 2},${inset} ${w - inset},${h / 2} ${w / 2},${h - inset} ${inset},${h / 2}`} />;
      break;
    case "line":
    case "arrow": {
      const x1 = element.flipH ? w : 0;
      const x2 = element.flipH ? 0 : w;
      const y1 = element.flipV ? h : 0;
      const y2 = element.flipV ? 0 : h;
      const markerId = `arrow-${element.id}`;
      const width = Math.max(0.5, element.strokeWidth * PT * scale);
      return (
        <svg className="slide-shape" width={Math.max(w, 1)} height={Math.max(h, 1)} style={{ overflow: "visible" }} aria-hidden="true">
          {element.shape === "arrow" && <ArrowHead id={markerId} color={stroke} />}
          <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={stroke} strokeWidth={width} markerEnd={element.shape === "arrow" ? `url(#${markerId})` : undefined} />
        </svg>
      );
    }
    case "roundRect": {
      const radius = Math.max(0.5, Math.min(element.w, element.h) * 0.15) * scale;
      shape = <rect x={inset} y={inset} width={Math.max(0, w - strokeWidth)} height={Math.max(0, h - strokeWidth)} rx={radius} ry={radius} />;
      break;
    }
    default:
      shape = <rect x={inset} y={inset} width={Math.max(0, w - strokeWidth)} height={Math.max(0, h - strokeWidth)} />;
  }
  return (
    <svg className="slide-shape" width={w} height={h} aria-hidden="true" style={{ "--accent": accent } as CSSProperties}>
      <g fill={fill} stroke={stroke} strokeWidth={strokeWidth}>
        {shape}
      </g>
    </svg>
  );
}

function TextContent({ element, scale }: { element: SlideElement; scale: number }) {
  const html = useMemo(() => slideTextHtml(element.content), [element.content]);
  return <div className="slide-text-content" dangerouslySetInnerHTML={{ __html: html }} style={{ fontSize: element.fontSize * PT * scale }} />;
}

function FormulaContent({ element, scale }: { element: SlideElement; scale: number }) {
  const html = useMemo(() => renderMath(`\\displaystyle ${element.latex || "\\square"}`, false), [element.latex]);
  return <div className="slide-formula" style={{ fontSize: element.fontSize * PT * scale, color: element.color || undefined }} dangerouslySetInnerHTML={{ __html: html }} />;
}

export function elementStyle(element: SlideElement, scale: number): CSSProperties {
  return {
    left: element.x * scale,
    top: element.y * scale,
    width: element.w * scale,
    height: element.h * scale,
    transform: element.rotation ? `rotate(${element.rotation}deg)` : undefined,
  };
}

export const SlideView = memo(function SlideView({ slide, deck, scale, editing = false, editingId = null, editor, placeholderText, className, children }: Props) {
  const size = slideSize(deck.aspect);
  const style: CSSProperties = {
    width: size.w * scale,
    height: size.h * scale,
    background: slide.background || deck.theme.background,
    color: deck.theme.textColor,
    fontFamily: FONT_STACK[deck.theme.font] ?? FONT_STACK.sans,
    ["--slide-accent" as string]: deck.theme.accent,
  };
  return (
    <div className={`slide-view${className ? ` ${className}` : ""}`} style={style}>
      {slide.elements.map((element) => {
        const textCapable = element.kind === "text" || element.kind === "shape";
        const isEditing = editingId === element.id;
        const empty = textCapable && !hasText(element.content);
        const showPlaceholder = editing && !isEditing && empty && element.role !== "";
        return (
          <div
            key={element.id}
            className={`slide-element kind-${element.kind}${showPlaceholder ? " placeholder" : ""}${editing && element.kind === "text" && !element.fill && !element.stroke ? " outline-hint" : ""}`}
            data-element-id={element.id}
            style={elementStyle(element, scale)}
          >
            {element.kind === "shape" && <ShapeGraphic element={element} scale={scale} accent={deck.theme.accent} />}
            {element.kind === "text" && (element.fill || element.stroke) && <ShapeGraphic element={{ ...element, shape: element.shape || "rect" }} scale={scale} accent={deck.theme.accent} />}
            {element.kind === "image" && element.src && <img className="slide-image" src={element.src} alt="" draggable={false} />}
            {element.kind === "formula" && <FormulaContent element={element} scale={scale} />}
            {textCapable && element.shape !== "line" && element.shape !== "arrow" && (
              <div
                className={`slide-text valign-${element.verticalAlign}`}
                style={{ padding: element.padding * scale, color: element.color || undefined }}
              >
                {isEditing ? (
                  <div className="slide-text-content" style={{ fontSize: element.fontSize * PT * scale }}>{editor}</div>
                ) : showPlaceholder ? (
                  <div className="slide-placeholder" style={{ fontSize: element.fontSize * PT * scale }}>{placeholderText?.(element)}</div>
                ) : (
                  !empty && <TextContent element={element} scale={scale} />
                )}
              </div>
            )}
          </div>
        );
      })}
      {children}
    </div>
  );
});
