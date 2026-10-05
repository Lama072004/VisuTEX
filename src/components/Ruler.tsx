/**
 * Horizontales Lineal über dem Blatt (wie in Textverarbeitungen): Zentimeter ab
 * dem linken Satzspiegelrand, graue Seitenränder, Ränder per Ziehen ändern
 * (ganze Millimeter, mit Alt in 0,1-mm-Schritten). Übernommen wird beim Loslassen,
 * damit die Paginierung nicht bei jeder Mausbewegung neu läuft.
 */
import { useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useT } from "../i18n";

const MM = 96 / 25.4;
const MIN_MARGIN = 5;
const MIN_TEXT_WIDTH = 30;

type Props = {
  pageWidthMm: number;
  /** linker Rand inkl. Bindekorrektur */
  marginLeftMm: number;
  marginRightMm: number;
  bindingOffsetMm: number;
  zoom: number;
  onChange: (margins: { left: number; right: number }) => void;
};

type Drag = { side: "left" | "right"; startX: number; startValue: number; value: number };

export function Ruler({ pageWidthMm, marginLeftMm, marginRightMm, bindingOffsetMm, zoom, onChange }: Props) {
  const t = useT();
  const [drag, setDrag] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  const scale = MM * zoom;
  const width = pageWidthMm * scale;
  const left = drag?.side === "left" ? drag.value + bindingOffsetMm : marginLeftMm;
  const right = drag?.side === "right" ? drag.value : marginRightMm;

  const start = (side: "left" | "right") => (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // ältere WebViews: ohne Capture weiterziehen
    }
    const value = side === "left" ? marginLeftMm - bindingOffsetMm : marginRightMm;
    const next = { side, startX: event.clientX, startValue: value, value };
    dragRef.current = next;
    setDrag(next);
  };
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = dragRef.current;
    if (!current) return;
    const deltaMm = (event.clientX - current.startX) / scale;
    const raw = current.startValue + (current.side === "left" ? deltaMm : -deltaMm);
    const step = event.altKey ? 0.1 : 1;
    const other = current.side === "left" ? marginRightMm : marginLeftMm;
    const offset = current.side === "left" ? bindingOffsetMm : 0;
    const max = pageWidthMm - other - offset - MIN_TEXT_WIDTH;
    const value = Math.min(max, Math.max(MIN_MARGIN, Math.round(raw / step) * step));
    const next = { ...current, value: Number(value.toFixed(1)) };
    dragRef.current = next;
    setDrag(next);
  };
  const end = () => {
    const current = dragRef.current;
    dragRef.current = null;
    setDrag(null);
    if (!current || current.value === current.startValue) return;
    onChange({
      left: current.side === "left" ? current.value : marginLeftMm - bindingOffsetMm,
      right: current.side === "right" ? current.value : marginRightMm,
    });
  };

  // Teilstriche: alle Millimeter ab dem linken Satzspiegelrand in beide Richtungen.
  const ticks: Array<{ x: number; size: "cm" | "half" | "mm"; label?: string }> = [];
  for (let mm = -Math.floor(left); mm <= pageWidthMm - left; mm += 1) {
    const x = (left + mm) * scale;
    if (x < 0 || x > width) continue;
    if (mm % 10 === 0) ticks.push({ x, size: "cm", label: mm === 0 ? undefined : String(Math.abs(mm / 10)) });
    else if (mm % 5 === 0) ticks.push({ x, size: "half" });
    else if (scale >= 3) ticks.push({ x, size: "mm" });
  }
  const formatMm = (value: number) => `${(value / 10).toLocaleString(undefined, { maximumFractionDigits: 2 })} cm`;

  return (
    <div className="ruler" style={{ width }} role="group" aria-label={t("Lineal")} onPointerMove={move} onPointerUp={end} onPointerCancel={end}>
      <div className="ruler-margin" style={{ left: 0, width: left * scale }} />
      <div className="ruler-margin" style={{ right: 0, width: right * scale }} />
      <svg className="ruler-ticks" width={width} height={22} aria-hidden="true">
        {ticks.map((tick) =>
          tick.label ? (
            <text key={tick.x} x={tick.x} y={15} textAnchor="middle">{tick.label}</text>
          ) : (
            <line key={tick.x} x1={tick.x} x2={tick.x} y1={tick.size === "cm" ? 5 : tick.size === "half" ? 8 : 10} y2={tick.size === "cm" ? 17 : tick.size === "half" ? 14 : 12} />
          ),
        )}
      </svg>
      <div
        className={`ruler-handle left${drag?.side === "left" ? " dragging" : ""}`}
        style={{ left: left * scale }}
        title={`${t("Linker Rand")}: ${formatMm(left - bindingOffsetMm)}`}
        role="slider"
        aria-label={t("Linker Rand")}
        aria-valuenow={Number((left - bindingOffsetMm).toFixed(1))}
        aria-valuemin={MIN_MARGIN}
        onPointerDown={start("left")}
      />
      <div
        className={`ruler-handle right${drag?.side === "right" ? " dragging" : ""}`}
        style={{ left: width - right * scale }}
        title={`${t("Rechter Rand")}: ${formatMm(right)}`}
        role="slider"
        aria-label={t("Rechter Rand")}
        aria-valuenow={Number(right.toFixed(1))}
        aria-valuemin={MIN_MARGIN}
        onPointerDown={start("right")}
      />
      {drag && (
        <div className="ruler-tooltip" style={{ left: drag.side === "left" ? left * scale : width - right * scale }}>
          {formatMm(drag.value)}
        </div>
      )}
    </div>
  );
}
