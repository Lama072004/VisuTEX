/**
 * Strg + Mausrad (bzw. Zwei-Finger-Zoom auf dem Touchpad) zum Vergrößern/Verkleinern – wie in
 * Word. Tauri schaltet den Browser-Zoom ab, deshalb behandelt jeder Bereich das selbst.
 * Touchpad-Gesten liefern viele kleine Schritte; sie werden gesammelt, bis ein Zoomschritt erreicht ist.
 */
import { useEffect, useRef } from "react";
import type { RefObject } from "react";

const STEP = 50;

export function useCtrlWheel(ref: RefObject<HTMLElement | null>, onStep: (direction: 1 | -1) => void, active = true) {
  const handler = useRef(onStep);
  handler.current = onStep;
  useEffect(() => {
    const element = ref.current;
    if (!element || !active) return;
    let accumulated = 0;
    const listener = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      // Mausrad: ein Raster ≈ 100 Pixel → ein Schritt; Touchpad: kleine Werte sammeln
      accumulated += event.deltaMode === 1 ? event.deltaY * 40 : event.deltaY;
      if (Math.abs(accumulated) < STEP) return;
      handler.current(accumulated < 0 ? 1 : -1);
      accumulated = 0;
    };
    element.addEventListener("wheel", listener, { passive: false });
    return () => element.removeEventListener("wheel", listener);
  }, [ref, active]);
}
