/**
 * Laufzeitkontext für NodeViews (die außerhalb des React-Baums der App
 * gerendert werden können): Projektordner, Sprache, Einstellungen und
 * Rückrufe in die App (Dialoge öffnen).
 */
import { useSyncExternalStore } from "react";
import type { UiLanguage } from "../i18n";
import type { MacroDef, RenderContext } from "../latex/miniRender";
import type { DocumentSettings } from "../latex/settings";
import { defaultDocumentSettings } from "../latex/settings";
import { documentLanguage } from "../latex/languages";

export type EditRequest =
  | {
      type:
        | "citation"
        | "footnote"
        | "acronym"
        | "crossReference"
        | "rawLatexInline"
        | "image"
        | "table"
        | "pageBreak"
        | "link"
        | "quantity"
        | "verticalSpace"
        | "environmentBlock"
        | "sketch";
      pos: number;
    }
  | { type: "insertImage" };

export type Runtime = {
  projectRoot: string | null;
  language: UiLanguage;
  allowOnline: boolean;
  settings: DocumentSettings;
  /** Eigene Präambel des Dokuments (fremde Dokumente) bzw. null. */
  customPreamble: string | null;
  /** Eigene Makros aus der Präambel (nur Anzeige). */
  macros: Map<string, MacroDef>;
  editNode: (request: EditRequest) => void;
  notify: (message: string, kind?: "info" | "error") => void;
};

let state: Runtime = {
  projectRoot: null,
  language: "de",
  allowOnline: false,
  settings: defaultDocumentSettings,
  customPreamble: null,
  macros: new Map(),
  editNode: () => undefined,
  notify: () => undefined,
};

/** Kontext für `latexToHtml` aus dem aktuellen Dokument. */
export function renderContext(runtime: Runtime = state): RenderContext {
  return {
    macros: runtime.macros,
    language: runtime.settings.language,
    germanShorthands: documentLanguage(runtime.settings.language).germanShorthands,
  };
}
const listeners = new Set<() => void>();

export function getRuntime(): Runtime {
  return state;
}

export function setRuntime(patch: Partial<Runtime>) {
  const changed = (Object.keys(patch) as Array<keyof Runtime>).some((key) => state[key] !== patch[key]);
  if (!changed) return;
  state = { ...state, ...patch };
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useRuntime<T>(selector: (runtime: Runtime) => T): T {
  return useSyncExternalStore(subscribe, () => selector(state), () => selector(state));
}
