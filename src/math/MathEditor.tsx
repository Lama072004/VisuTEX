/**
 * Grafische Formelbearbeitung wie in Word/OneNote: MathLive zeigt die Formel gerendert und
 * lässt direkt darin tippen (Brüche, Exponenten, Wurzeln mit Platzhaltern, Pfeiltasten
 * springen zwischen den Feldern). Umschaltbar auf den LaTeX-Code. Die Registerkarte *Formel*
 * fügt über `mathBridge` Symbole und Strukturen an der Cursorposition ein.
 *
 * Alles läuft offline: die Schriften kommen aus `mathlive/fonts.css` (von Vite eingebunden),
 * Klänge sind abgeschaltet.
 */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { MathfieldElement } from "mathlive";
import "mathlive/fonts.css";
import { useT } from "../i18n";
import { registerMathTarget, takePendingMath } from "./mathBridge";
import type { MathEditMode } from "./mathBridge";
import { templateToCode } from "./mathCatalog";

type MathLiveModule = typeof import("mathlive");
let loading: Promise<MathLiveModule> | null = null;

/** MathLive erst bei Bedarf laden und einmalig für den Offline-Betrieb einrichten. */
function loadMathLive(): Promise<MathLiveModule> {
  loading ??= import("mathlive").then((module) => {
    module.MathfieldElement.fontsDirectory = null; // Schriften kommen aus fonts.css
    module.MathfieldElement.soundsDirectory = null; // keine Klänge
    const language = (document.documentElement.lang || "de").slice(0, 2);
    module.MathfieldElement.locale = language;
    return module;
  });
  return loading;
}

/** Ausgabe für LaTeX: leere Platzhalter werden zu leeren Gruppen. */
const latexOf = (field: MathfieldElement) => field.getValue("latex-without-placeholders");

type FieldProps = {
  value: string;
  display: boolean;
  autoFocus: boolean;
  onInput: (latex: string) => void;
  onReady: (field: MathfieldElement | null) => void;
  onLeave: () => void;
};

function MathField({ value, display, autoFocus, onInput, onReady, onLeave }: FieldProps) {
  const t = useT();
  const host = useRef<HTMLDivElement>(null);
  const field = useRef<MathfieldElement | null>(null);
  const callbacks = useRef({ onInput, onReady, onLeave });
  callbacks.current = { onInput, onReady, onLeave };
  const initial = useRef(value);

  useEffect(() => {
    let cancelled = false;
    void loadMathLive().then(({ MathfieldElement }) => {
      if (cancelled || !host.current) return;
      const element = new MathfieldElement();
      element.className = display ? "math-field is-display" : "math-field";
      element.setAttribute("aria-label", t("Formel"));
      element.addEventListener("input", () => callbacks.current.onInput(latexOf(element)));
      element.addEventListener("keydown", (event) => {
        // Esc verlässt das Formelfeld (Änderungen bleiben erhalten)
        if (event.key === "Escape") callbacks.current.onLeave();
      });
      // Einige Einstellungen (z. B. menuItems) setzt MathLive erst im eingehängten Zustand
      host.current.appendChild(element);
      element.mathVirtualKeyboardPolicy = "manual";
      element.smartFence = true;
      element.smartSuperscript = true;
      element.menuItems = [];
      element.placeholder = t("Formel hier eingeben");
      element.value = initial.current;
      field.current = element;
      callbacks.current.onReady(element);
      if (autoFocus) requestAnimationFrame(() => element.focus());
    });
    return () => {
      cancelled = true;
      field.current?.remove();
      field.current = null;
      callbacks.current.onReady(null);
    };
    // Nur beim Anlegen – spätere Änderungen von außen übernimmt der Effekt unten.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Änderungen von außen (Rückgängig, Umschalten aus dem Code) übernehmen
  useEffect(() => {
    const element = field.current;
    if (element && latexOf(element) !== value) element.value = value;
  }, [value]);

  return <div ref={host} className="math-field-host" />;
}

type PanelProps = {
  latex: string;
  /** Abgesetzte Formel (sonst Formel im Text) */
  display: boolean;
  /** Anfangs grafisch oder als Code bearbeiten */
  initialMode: MathEditMode;
  /** Neu eingefügt → Formelfeld sofort fokussieren */
  autoFocus?: boolean;
  /** Übernehmen (beim Verlassen bzw. Abwählen); leere Formel = löschen */
  onCommit: (latex: string) => void;
  /** Zurück in den Text */
  onLeave: () => void;
  /** Vorschau im Code-Modus (HTML, z. B. KaTeX) */
  preview?: (latex: string) => string;
  /** Weitere Einstellungen (Umgebung, Nummerierung …) */
  children?: ReactNode;
};

/** Bearbeitungsbereich einer Formel: grafisch (MathLive) oder als LaTeX-Code. */
export function MathEditPanel({ latex, display, initialMode, autoFocus = true, onCommit, onLeave, preview, children }: PanelProps) {
  const t = useT();
  const [mode, setMode] = useState<MathEditMode>(initialMode);
  const [draft, setDraft] = useState(latex);
  const draftRef = useRef(latex);
  const committed = useRef(latex);
  const field = useRef<MathfieldElement | null>(null);
  const textarea = useRef<HTMLTextAreaElement>(null);

  const update = (value: string) => {
    draftRef.current = value;
    setDraft(value);
  };
  const commit = () => {
    if (draftRef.current !== committed.current) {
      committed.current = draftRef.current;
      onCommit(draftRef.current);
    }
  };
  const commitRef = useRef(commit);
  commitRef.current = commit;

  // Von außen geänderte Formel (Rückgängig) übernehmen, solange nichts Eigenes offen ist
  useEffect(() => {
    if (latex !== committed.current && draftRef.current === committed.current) {
      committed.current = latex;
      update(latex);
    }
  }, [latex]);

  // Beim Abwählen der Formel übernehmen
  useEffect(() => () => commitRef.current(), []);

  // Textfeld so hoch wie sein Inhalt
  useLayoutEffect(() => {
    const element = textarea.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${Math.min(element.scrollHeight + 2, window.innerHeight * 0.5)}px`;
  });

  // Beim Menüband anmelden: Einfügen an der Cursorposition
  const modeRef = useRef(mode);
  modeRef.current = mode;
  useEffect(() => {
    const insertCode = (template: string) => {
      const element = textarea.current;
      const text = draftRef.current;
      const start = element?.selectionStart ?? text.length;
      const end = element?.selectionEnd ?? text.length;
      const { text: snippet, caret } = templateToCode(template, text.slice(start, end));
      const next = text.slice(0, start) + snippet + text.slice(end);
      update(next);
      requestAnimationFrame(() => {
        element?.focus();
        element?.setSelectionRange(start + caret, start + caret);
      });
    };
    return registerMathTarget({
      insert: (template) => {
        if (modeRef.current === "visual" && field.current) {
          field.current.focus();
          field.current.insert(template, { format: "latex", selectionMode: "placeholder", focus: true });
          update(latexOf(field.current));
        } else {
          insertCode(template);
        }
      },
      mode,
      setMode: (next) => setMode(next),
    });
  }, [mode]);

  // Vorlage für eine neu eingefügte Formel („Häufige Formeln“)
  const ready = (element: MathfieldElement | null) => {
    field.current = element;
    const template = element ? takePendingMath() : null;
    if (element && template) {
      element.insert(template, { format: "latex", selectionMode: "placeholder", focus: true });
      update(latexOf(element));
    }
  };

  const leave = () => {
    commit();
    onLeave();
  };

  return (
    <div className="math-edit">
      <div className="math-edit-toolbar">
        <div className="segmented-light" role="group" aria-label={t("Bearbeitung")}>
          <button type="button" className={mode === "visual" ? "active" : ""} onClick={() => setMode("visual")} title={t("Formel grafisch bearbeiten (wie in Word)")}>
            {t("Grafisch")}
          </button>
          <button type="button" className={mode === "code" ? "active" : ""} onClick={() => setMode("code")} title={t("LaTeX-Code der Formel bearbeiten")}>
            LaTeX
          </button>
        </div>
        <span className="math-edit-hint">
          {mode === "visual" ? t("Symbole und Strukturen: Registerkarte „Formel“ · Tab/Pfeiltasten wechseln das Feld · Esc beendet") : t("Strg+Eingabe übernimmt · Esc beendet")}
        </span>
      </div>
      {mode === "visual" ? (
        <MathField value={draft} display={display} autoFocus={autoFocus} onInput={update} onReady={ready} onLeave={leave} />
      ) : (
        <textarea
          ref={textarea}
          className="code-input math-code"
          value={draft}
          spellCheck={false}
          autoFocus={autoFocus}
          aria-label={t("LaTeX-Formel")}
          onChange={(event) => update(event.currentTarget.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.ctrlKey || event.metaKey || !display)) {
              event.preventDefault();
              commit();
            }
            if (event.key === "Escape") leave();
          }}
        />
      )}
      {mode === "code" && preview && <div className="math-code-preview" dangerouslySetInnerHTML={{ __html: preview(draft) }} />}
      {children}
    </div>
  );
}

/** Grafische Bearbeitung möglich? Mehrzeilige Umgebungen (align …) besser als Code. */
export function defaultMathMode(environment: string, latex: string): MathEditMode {
  if (environment && environment !== "equation") return "code";
  // Zeilenumbrüche/Ausrichtung außerhalb von Matrizen und Fallunterscheidungen
  const outside = latex.replace(/\\begin\{([a-zA-Z]*matrix|cases)\}[\s\S]*?\\end\{\1\}/g, "");
  return /\\\\|(^|[^\\])&/.test(outside) ? "code" : "visual";
}
