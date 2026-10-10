/**
 * Verbindung zwischen der Registerkarte *Formel* im Menüband und dem gerade bearbeiteten
 * Formelfeld (grafisch per MathLive oder als LaTeX-Code). Das Feld meldet sich an, solange
 * die Formel ausgewählt ist; das Menüband fügt Symbole und Strukturen dort ein.
 */
import { useSyncExternalStore } from "react";

export type MathEditMode = "visual" | "code";

export type MathTarget = {
  /** Vorlage mit MathLive-Platzhaltern (`#?`, `#0`) an der Cursorposition einfügen */
  insert: (template: string) => void;
  mode: MathEditMode;
  setMode: (mode: MathEditMode) => void;
};

let target: MathTarget | null = null;
let pending: string | null = null;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((listener) => listener());

/** Formelfeld anmelden; liefert die Abmeldung. */
export function registerMathTarget(next: MathTarget): () => void {
  target = next;
  emit();
  return () => {
    if (target === next) {
      target = null;
      emit();
    }
  };
}

export function activeMathTarget(): MathTarget | null {
  return target;
}

export function useMathTarget(): MathTarget | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => target,
  );
}

/** Vorlage für die nächste neu eingefügte Formel (z. B. „Häufige Formeln“). */
export function setPendingMath(template: string | null) {
  pending = template;
}

export function takePendingMath(): string | null {
  const value = pending;
  pending = null;
  return value;
}
