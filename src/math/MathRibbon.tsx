/**
 * Registerkarte *Formel* (kontextbezogen wie „Formel“ in Word/OneNote): Werkzeuge,
 * Symbolgalerie mit Kategorien und Strukturen (Bruch, Skript, Wurzel, Integral …) als
 * ausklappbare Galerien mit gerenderten Vorschauen. Einfügen über `mathBridge` in das
 * gerade bearbeitete Formelfeld.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import katex from "katex";
import { ChevronDown, Code2, PenLine, Radical, Sigma, Type } from "lucide-react";
import { useT } from "../i18n";
import { useMathTarget } from "./mathBridge";
import { COMMON_FORMULAS, STRUCTURES, SYMBOL_CATEGORIES, previewLatex } from "./mathCatalog";
import type { StructureGroup } from "./mathCatalog";

const cache = new Map<string, string>();

/** KaTeX-Vorschau (zwischengespeichert). */
function Tex({ latex, display = false }: { latex: string; display?: boolean }) {
  const key = `${display ? "D" : "T"}${latex}`;
  let html = cache.get(key);
  if (html === undefined) {
    try {
      html = katex.renderToString(latex, { throwOnError: false, displayMode: display, strict: "ignore" });
    } catch {
      html = latex;
    }
    cache.set(key, html);
  }
  return <span className="math-tex" dangerouslySetInnerHTML={{ __html: html }} />;
}

/** Ausklappbereich unter einer Schaltfläche (fest positioniert, Fokus bleibt im Formelfeld). */
function Popover({ label, icon, large = false, wide = false, children }: { label: string; icon: ReactNode; large?: boolean; wide?: boolean; children: (close: () => void) => ReactNode }) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => !ref.current?.contains(event.target as Node) && setOpen(false);
    const escape = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", close);
    window.addEventListener("keydown", escape);
    window.addEventListener("resize", () => setOpen(false), { once: true });
    return () => {
      document.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", escape);
    };
  }, [open]);
  return (
    <div className="ribbon-dropdown" ref={ref}>
      <button
        type="button"
        className={`ribbon-button with-caret math-structure-button${large ? " large" : ""}${open ? " active" : ""}`}
        title={label}
        aria-label={label}
        aria-expanded={open}
        onMouseDown={(event) => event.preventDefault()}
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          const width = wide ? 560 : 420;
          setPosition({ top: rect.bottom + 4, left: Math.max(4, Math.min(rect.left, window.innerWidth - width - 8)) });
          setOpen((value) => !value);
        }}
      >
        {icon}
        {large && <span className="ribbon-button-label">{label}</span>}
        <ChevronDown size={12} className="caret" />
      </button>
      {open && (
        <div className={`ribbon-popover math-gallery${wide ? " wide" : ""}`} role="menu" style={{ top: position.top, left: position.left }} onMouseDown={(event) => event.preventDefault()}>
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  );
}

function StructureGallery({ group, onPick }: { group: StructureGroup; onPick: (latex: string) => void }) {
  const t = useT();
  return (
    <Popover label={t(group.label)} large icon={<span className="math-structure-icon"><Tex latex={group.icon} /></span>}>
      {(close) =>
        group.sections.map((section) => (
          <div key={section.label} className="math-gallery-section">
            <div className="math-gallery-title">{t(section.label)}</div>
            <div className="math-gallery-grid">
              {section.items.map((item) => (
                <button
                  key={item.latex}
                  type="button"
                  className="math-gallery-item"
                  title={t(item.title)}
                  aria-label={t(item.title)}
                  onClick={() => {
                    onPick(item.latex);
                    close();
                  }}
                >
                  <Tex latex={previewLatex(item.latex)} display />
                </button>
              ))}
            </div>
          </div>
        ))
      }
    </Popover>
  );
}

function SymbolGallery({ onPick }: { onPick: (latex: string) => void }) {
  const t = useT();
  const [category, setCategory] = useState(SYMBOL_CATEGORIES[0].id);
  const current = useMemo(() => SYMBOL_CATEGORIES.find((entry) => entry.id === category) ?? SYMBOL_CATEGORIES[0], [category]);
  return (
    <div className="math-symbols">
      <select className="ribbon-select math-symbol-category" value={category} aria-label={t("Symbolgruppe")} onChange={(event) => setCategory(event.currentTarget.value)}>
        {SYMBOL_CATEGORIES.map((entry) => (
          <option key={entry.id} value={entry.id}>
            {t(entry.label)}
          </option>
        ))}
      </select>
      <div className="math-symbol-grid" role="listbox" aria-label={t(current.label)}>
        {current.symbols.map((symbol) => (
          <button
            key={symbol.latex}
            type="button"
            className="math-symbol"
            title={symbol.title ? t(symbol.title) : symbol.latex}
            aria-label={symbol.title ? t(symbol.title) : symbol.latex}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onPick(symbol.latex)}
          >
            <Tex latex={symbol.latex} />
          </button>
        ))}
      </div>
    </div>
  );
}

type Props = {
  /** Neue Formel einfügen (optional mit Vorlage) */
  newFormula: (inline: boolean, template?: string) => void;
};

/** Gruppen der Registerkarte *Formel* (für `add(id, label, content)` im Menüband). */
export function useMathRibbonGroups({ newFormula }: Props): Array<{ id: string; label: string; content: ReactNode }> {
  const t = useT();
  const target = useMathTarget();
  const insert = (latex: string) => (target ? target.insert(latex) : newFormula(false, latex));
  const toolButton = (label: string, icon: ReactNode, onClick: () => void, active = false, title?: string) => (
    <button
      type="button"
      className={`ribbon-button large${active ? " active" : ""}`}
      title={title ?? label}
      aria-label={label}
      aria-pressed={active}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
    >
      {icon}
      <span className="ribbon-button-label">{label}</span>
    </button>
  );
  return [
    {
      id: "math-tools",
      label: t("Tools"),
      content: (
        <>
          {toolButton(t("Formel"), <Sigma size={22} />, () => newFormula(false), false, t("Neue abgesetzte Formel"))}
          {toolButton(t("Im Text"), <Radical size={22} />, () => newFormula(true), false, t("Neue Formel im Text"))}
          <Popover label={t("Häufige Formeln")} large wide icon={<span className="math-structure-icon"><Tex latex="\pi r^2" /></span>}>
            {(close) => (
              <div className="math-common-list">
                {COMMON_FORMULAS.map((formula) => (
                  <button
                    key={formula.latex}
                    type="button"
                    className="math-common-item"
                    onClick={() => {
                      insert(formula.latex);
                      close();
                    }}
                  >
                    <span className="math-common-title">{t(formula.title)}</span>
                    <Tex latex={formula.latex} display />
                  </button>
                ))}
              </div>
            )}
          </Popover>
        </>
      ),
    },
    {
      id: "math-edit",
      label: t("Bearbeitung"),
      content: (
        <>
          {toolButton(t("Grafisch"), <PenLine size={22} />, () => target?.setMode("visual"), target?.mode === "visual", t("Formel grafisch bearbeiten (wie in Word)"))}
          {toolButton("LaTeX", <Code2 size={22} />, () => target?.setMode("code"), target?.mode === "code", t("LaTeX-Code der Formel bearbeiten"))}
          {toolButton(t("Normaler Text"), <Type size={22} />, () => insert(String.raw`\text{#0}`), false, t("Normalen Text in die Formel einfügen"))}
        </>
      ),
    },
    {
      id: "math-symbols",
      label: t("Symbole"),
      content: <SymbolGallery onPick={insert} />,
    },
    {
      id: "math-structures",
      label: t("Strukturen"),
      content: (
        <div className="math-structures">
          {STRUCTURES.map((group) => (
            <StructureGallery key={group.id} group={group} onPick={insert} />
          ))}
        </div>
      ),
    },
  ];
}
