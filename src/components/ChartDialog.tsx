/**
 * Diagramm einfügen (wie in Word): Daten in einer Tabelle eingeben, aus Excel/LibreOffice
 * einfügen (Strg+V in eine Zelle) oder als CSV laden, Diagrammart wählen – die Vorschau
 * erzeugt die eingebaute TeX-Engine. Der pgfplots-Code entsteht in Rust (`core::chart`).
 */
import { useEffect, useRef, useState } from "react";
import type { ClipboardEvent as ReactClipboardEvent, KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeftRight, ChartArea, ChartBar, ChartColumn, ChartLine, ChartPie, ChartScatter, FileSpreadsheet, Minus, Plus } from "lucide-react";
import { api, errorText } from "../api";
import type { Chart, ChartKind } from "../api";
import { useT } from "../i18n";
import { Modal } from "./Dialogs";

/** `preview`: fertiges Vorschaubild zu `code` (leer: wird im Dokument erzeugt) */
export type ChartResult = { chart: Chart; code: string; preview: string; caption: string; label: string };
export type ChartInitial = { chart: Chart | null; caption: string; label: string };

const labelPattern = /^[A-Za-z0-9:_\-.]*$/;

function defaultChart(decimalComma: boolean, t: (text: string) => string): Chart {
  const number = (value: number) => (decimalComma ? String(value).replace(".", ",") : String(value));
  const values = [
    [4.3, 2.4, 2],
    [2.5, 4.4, 2],
    [3.5, 1.8, 3],
    [4.5, 2.8, 5],
  ];
  return {
    kind: "bar",
    title: "",
    xLabel: "",
    yLabel: "",
    data: [
      ["", ...[1, 2, 3].map((index) => `${t("Datenreihe")} ${index}`)],
      ...values.map((row, index) => [`${t("Kategorie")} ${index + 1}`, ...row.map(number)]),
    ],
    // wie in Word unter dem Diagramm – oben rechts verdeckt die Legende oft Säulen
    legend: "below",
    grid: true,
    width: 12,
    stacked: false,
    marks: true,
    smooth: false,
    decimalComma,
  };
}

/** Rechteckige Tabelle (fehlende Zellen leer, mindestens 2 × 2). */
function normalize(data: string[][]): string[][] {
  const columns = Math.max(2, ...data.map((row) => row.length));
  const rows = data.length < 2 ? [...data, ...Array.from({ length: 2 - data.length }, () => [])] : data;
  return rows.map((row) => Array.from({ length: columns }, (_, index) => row[index] ?? ""));
}

export function ChartDialog({
  initial,
  decimalComma,
  allowOnline,
  onDone,
}: {
  initial: ChartInitial | null;
  /** Dezimalkomma der Dokumentsprache (für neue Diagramme) */
  decimalComma: boolean;
  allowOnline: boolean;
  onDone: (result: ChartResult | null) => void;
}) {
  const t = useT();
  const [chart, setChart] = useState<Chart>(() => {
    const start = initial?.chart ?? defaultChart(decimalComma, t);
    return { ...start, data: normalize(start.data) };
  });
  const [caption, setCaption] = useState(initial?.caption ?? "");
  const [label, setLabel] = useState(initial?.label ?? "");
  const [preview, setPreview] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const gridRef = useRef<HTMLTableElement>(null);
  const run = useRef(0);

  const kinds: Array<{ kind: ChartKind; name: string; icon: ReactNode }> = [
    { kind: "bar", name: t("Säulendiagramm"), icon: <ChartColumn size={22} /> },
    { kind: "barh", name: t("Balkendiagramm"), icon: <ChartBar size={22} /> },
    { kind: "line", name: t("Liniendiagramm"), icon: <ChartLine size={22} /> },
    { kind: "area", name: t("Flächendiagramm"), icon: <ChartArea size={22} /> },
    { kind: "scatter", name: t("Punktdiagramm (XY)"), icon: <ChartScatter size={22} /> },
    { kind: "pie", name: t("Kreisdiagramm"), icon: <ChartPie size={22} /> },
  ];
  const update = (patch: Partial<Chart>) => setChart((current) => ({ ...current, ...patch }));
  const setData = (data: string[][]) => update({ data: normalize(data) });
  const isPie = chart.kind === "pie";
  const isBar = chart.kind === "bar" || chart.kind === "barh";
  const isLine = chart.kind === "line" || chart.kind === "area";

  // Vorschau: Code in Rust erzeugen, dann mit der eingebauten Engine zeichnen (verzögert, letzte Anfrage gewinnt)
  useEffect(() => {
    const id = ++run.current;
    const timer = window.setTimeout(async () => {
      setBusy(true);
      try {
        const latex = await api.chartToLatex(chart);
        if (id !== run.current) return;
        setCode(latex);
        const image = await api.renderTikz(latex, "tikzpicture", "", allowOnline);
        if (id !== run.current) return;
        setPreview(image);
        setError("");
      } catch (renderError) {
        if (id !== run.current) return;
        setError(errorText(renderError));
      } finally {
        if (id === run.current) setBusy(false);
      }
    }, 500);
    return () => window.clearTimeout(timer);
  }, [chart, allowOnline]);

  const setCell = (row: number, column: number, value: string) =>
    setChart((current) => ({ ...current, data: current.data.map((cells, r) => (r === row ? cells.map((cell, c) => (c === column ? value : cell)) : cells)) }));

  const focusCell = (row: number, column: number) =>
    window.requestAnimationFrame(() => gridRef.current?.querySelector<HTMLInputElement>(`input[data-cell="${row}:${column}"]`)?.focus());

  /** Mehrere Zellen aus Excel/LibreOffice (Tabulatoren, Zeilenumbrüche) ab der Zelle einfügen. */
  const pasteCells = async (event: ReactClipboardEvent<HTMLInputElement>, row: number, column: number) => {
    const text = event.clipboardData.getData("text/plain");
    if (!/[\t\n]/.test(text.replace(/\r?\n$/, ""))) return;
    event.preventDefault();
    try {
      const rows = await api.chartParseTable(text);
      if (rows.length === 0) return;
      // In die erste Zelle eingefügt: ganze Tabelle ersetzen (Beispieldaten fallen weg)
      const data = row === 0 && column === 0 ? [] : chart.data.map((cells) => [...cells]);
      rows.forEach((cells, r) => {
        const target = (data[row + r] ??= []);
        cells.forEach((cell, c) => {
          target[column + c] = cell;
        });
      });
      setData(data);
      setMessage(`${rows.length} × ${Math.max(...rows.map((cells) => cells.length))} ${t("Zellen eingefügt")}`);
    } catch (pasteError) {
      setMessage(errorText(pasteError));
    }
  };

  const loadCsv = async () => {
    const path = await open({ multiple: false, filters: [{ name: t("Tabellen (CSV, TSV, TXT)"), extensions: ["csv", "tsv", "txt"] }] });
    if (typeof path !== "string") return;
    try {
      const rows = await api.chartParseTable(await api.readTextFile(path));
      if (rows.length === 0) {
        setMessage(t("Die Datei enthält keine Tabelle."));
        return;
      }
      setData(rows);
      setMessage(`${rows.length - 1} ${t("Datenzeilen geladen")}`);
    } catch (loadError) {
      setMessage(errorText(loadError));
    }
  };

  const onCellKey = (event: ReactKeyboardEvent<HTMLInputElement>, row: number, column: number) => {
    if (event.key === "Enter") {
      event.preventDefault();
      const next = event.shiftKey ? row - 1 : row + 1;
      if (next >= chart.data.length) setData([...chart.data, []]);
      if (next >= 0) focusCell(next, column);
    } else if (event.key === "ArrowDown" && row + 1 < chart.data.length) {
      event.preventDefault();
      focusCell(row + 1, column);
    } else if (event.key === "ArrowUp" && row > 0) {
      event.preventDefault();
      focusCell(row - 1, column);
    }
  };

  /** Code zum aktuellen Stand (die Vorschau läuft verzögert und kann einen Schritt zurück sein). */
  const finish = async () => {
    try {
      const latest = await api.chartToLatex(chart);
      onDone({ chart, code: latest, preview: latest === code ? preview : "", caption: caption.trim(), label: label.trim() });
    } catch (finishError) {
      setError(errorText(finishError));
    }
  };

  const columns = chart.data[0]?.length ?? 2;
  const transpose = () => setData(Array.from({ length: columns }, (_, c) => chart.data.map((cells) => cells[c] ?? "")));

  let body: ReactNode = null;
  if (preview) body = <img className={`chart-preview-image${busy ? " is-stale" : ""}`} src={preview} alt={t("Diagrammvorschau")} />;
  else body = <div className="tikz-placeholder">{busy ? t("Diagramm wird erzeugt …") : t("Vorschau folgt")}</div>;

  return (
    <Modal
      title={initial ? t("Diagramm bearbeiten") : t("Diagramm einfügen")}
      wide
      className="chart-dialog"
      onClose={() => onDone(null)}
      footer={
        <>
          {message && <span className="form-message">{message}</span>}
          <button type="button" onClick={() => onDone(null)}>{t("Abbrechen")}</button>
          <button
            type="button"
            className="primary"
            disabled={!code || Boolean(error) || busy}
            onClick={() => void finish()}
          >
            {initial ? t("Übernehmen") : t("Einfügen")}
          </button>
        </>
      }
    >
      <div className="chart-kinds" role="radiogroup" aria-label={t("Diagrammart")}>
        {kinds.map((entry) => (
          <button
            key={entry.kind}
            type="button"
            role="radio"
            aria-checked={chart.kind === entry.kind}
            className={chart.kind === entry.kind ? "active" : ""}
            onClick={() => update({ kind: entry.kind })}
          >
            {entry.icon}
            <span>{entry.name}</span>
          </button>
        ))}
      </div>

      <div className="chart-layout">
        <section className="chart-data">
          <div className="chart-data-toolbar">
            <strong>{t("Daten")}</strong>
            <button type="button" className="icon-text" onClick={() => setData([...chart.data, []])} title={t("Zeile anfügen")}>
              <Plus size={14} /> {t("Datenzeile")}
            </button>
            <button type="button" className="icon-text" disabled={chart.data.length <= 2} onClick={() => setData(chart.data.slice(0, -1))} title={t("Letzte Zeile entfernen")}>
              <Minus size={14} /> {t("Datenzeile")}
            </button>
            <button type="button" className="icon-text" onClick={() => setData(chart.data.map((cells) => [...cells, ""]))} title={t("Datenreihe anfügen")}>
              <Plus size={14} /> {t("Datenreihe")}
            </button>
            <button type="button" className="icon-text" disabled={columns <= 2} onClick={() => setData(chart.data.map((cells) => cells.slice(0, -1)))} title={t("Letzte Datenreihe entfernen")}>
              <Minus size={14} /> {t("Datenreihe")}
            </button>
            <button type="button" className="icon-text" onClick={transpose} title={t("Zeilen und Spalten tauschen")}>
              <ArrowLeftRight size={14} /> {t("Tauschen")}
            </button>
            <button type="button" className="icon-text" onClick={() => void loadCsv()}>
              <FileSpreadsheet size={14} /> {t("CSV laden …")}
            </button>
          </div>
          <div className="chart-grid-scroll">
            <table className="chart-grid" ref={gridRef}>
              <tbody>
                {chart.data.map((cells, row) => (
                  <tr key={row} className={row === 0 ? "chart-grid-head" : undefined}>
                    {cells.map((cell, column) => (
                      <td key={column} className={column === 0 ? "chart-grid-first" : undefined}>
                        <input
                          value={cell}
                          data-cell={`${row}:${column}`}
                          aria-label={row === 0 ? (column === 0 ? t("Überschrift der ersten Spalte") : `${t("Name der Datenreihe")} ${column}`) : `${t("Datenzeile")} ${row}, ${t("Spalte")} ${column + 1}`}
                          placeholder={row === 0 && column === 0 ? t("x / Kategorie") : ""}
                          onChange={(event) => setCell(row, column, event.currentTarget.value)}
                          onPaste={(event) => void pasteCells(event, row, column)}
                          onKeyDown={(event) => onCellKey(event, row, column)}
                        />
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="muted chart-hint">
            {isPie
              ? t("Erste Spalte: Beschriftungen der Segmente, zweite Spalte: Werte.")
              : t("Erste Zeile: Namen der Datenreihen. Erste Spalte: Kategorien bzw. x-Werte. Bereich aus Excel kopieren und mit Strg+V in eine Zelle einfügen.")}
          </p>
        </section>

        <section className="chart-side">
          <div className="chart-preview" aria-live="polite">
            {body}
            {busy && preview && <span className="tikz-busy">{t("Aktualisiere …")}</span>}
          </div>
          {error && <div className="node-error">{error.split("\n")[0]}</div>}
          <div className="form-grid chart-options">
            <label className="form-field span-2">
              {t("Diagrammtitel")}
              <input value={chart.title} onChange={(event) => update({ title: event.currentTarget.value })} />
            </label>
            {!isPie && (
              <>
                <label className="form-field">
                  {chart.kind === "barh" ? t("Beschriftung vertikale Achse") : t("Beschriftung x-Achse")}
                  <input value={chart.xLabel} placeholder={chart.data[0]?.[0] ?? ""} onChange={(event) => update({ xLabel: event.currentTarget.value })} />
                </label>
                <label className="form-field">
                  {chart.kind === "barh" ? t("Beschriftung horizontale Achse") : t("Beschriftung y-Achse")}
                  <input value={chart.yLabel} onChange={(event) => update({ yLabel: event.currentTarget.value })} />
                </label>
              </>
            )}
            <label className="form-field">
              {t("Legende")}
              <select value={chart.legend} onChange={(event) => update({ legend: event.currentTarget.value })}>
                <option value="north east">{t("oben rechts")}</option>
                <option value="north west">{t("oben links")}</option>
                <option value="south east">{t("unten rechts")}</option>
                <option value="south west">{t("unten links")}</option>
                <option value="below">{t("unter dem Diagramm")}</option>
                <option value="none">{t("keine")}</option>
              </select>
            </label>
            <label className="form-field">
              {t("Breite (cm)")}
              <input type="number" min={4} max={30} step={0.5} value={chart.width} onChange={(event) => update({ width: Number(event.currentTarget.value) || 12 })} />
            </label>
            <div className="chart-checks span-2">
              {!isPie && (
                <label>
                  <input type="checkbox" checked={chart.grid} onChange={(event) => update({ grid: event.currentTarget.checked })} /> {t("Gitternetz")}
                </label>
              )}
              {isBar && (
                <label>
                  <input type="checkbox" checked={chart.stacked} onChange={(event) => update({ stacked: event.currentTarget.checked })} /> {t("Gestapelt")}
                </label>
              )}
              {isLine && (
                <>
                  <label>
                    <input type="checkbox" checked={chart.marks} onChange={(event) => update({ marks: event.currentTarget.checked })} /> {t("Datenpunkte markieren")}
                  </label>
                  <label>
                    <input type="checkbox" checked={chart.smooth} onChange={(event) => update({ smooth: event.currentTarget.checked })} /> {t("Linien glätten")}
                  </label>
                </>
              )}
              <label>
                <input type="checkbox" checked={chart.decimalComma} onChange={(event) => update({ decimalComma: event.currentTarget.checked })} /> {t("Dezimalkomma")}
              </label>
            </div>
            <label className="form-field">
              {t("Beschriftung (Abbildungsunterschrift)")}
              <input value={caption} onChange={(event) => setCaption(event.currentTarget.value)} />
            </label>
            <label className="form-field">
              {t("Label (für Querverweise)")}
              <input value={label} placeholder="fig:diagramm" onChange={(event) => labelPattern.test(event.currentTarget.value) && setLabel(event.currentTarget.value)} />
            </label>
          </div>
        </section>
      </div>
    </Modal>
  );
}
