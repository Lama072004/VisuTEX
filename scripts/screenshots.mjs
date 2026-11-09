#!/usr/bin/env node
// Screenshots für README/Doku neu erzeugen (docs/screenshots/*.png, 1600 × 1000, helles Design).
//
// Voraussetzung (Entwicklungsmodus, siehe docs/ENTWICKLUNG.md „Automatisierte Oberflächentests“):
//   npm run dev                                   (Vite, Port 1420)
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 src-tauri/target/debug/visutex.exe
// Aufruf:
//   node scripts/screenshots.mjs                  alle
//   node scripts/screenshots.mjs diagramm,versionen   nur diese
//
// Das Skript legt ein Beispieldokument in einem neutralen Ordner an (keine privaten Pfade im Bild), stellt danach
// Design, „Zuletzt verwendet“ und Wiederherstellungsdaten wieder her und löscht Beispielordner und Arbeitsdaten.
// „Literatur per DOI“ braucht Internet (doi.org).
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "docs", "screenshots");
const only = (process.argv[2] ?? "").split(",").filter(Boolean);
const want = (name) => only.length === 0 || only.includes(name);
const exampleDir =
  process.platform === "win32" ? "C:/Users/Public/Documents/VisuTeX-Beispiel" : path.join(os.tmpdir(), "VisuTeX-Beispiel");
const WIDTH = 1600;
const HEIGHT = 1000;

// ---------------------------------------------------------------- Verbindung (Chrome-DevTools-Protokoll)

const targets = await (await fetch("http://127.0.0.1:9223/json")).json();
const page = targets.find((target) => target.type === "page" && target.url.includes("localhost:1420"));
if (!page) throw new Error("VisuTeX im Entwicklungsmodus nicht gefunden (Port 9223 / Vite 1420).");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
let nextId = 1;
const pending = new Map();
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  }
};
const send = (method, params = {}) =>
  new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** JavaScript in der Seite ausführen (async-Funktionskörper, `v` = window.__visutex). */
async function run(body, args = {}) {
  const expression = `(async () => { const v = window.__visutex; const args = ${JSON.stringify(args)}; const wait = (ms) => new Promise((r) => setTimeout(r, ms)); ${body} })()`;
  const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  const error = result.result?.exceptionDetails;
  if (error) throw new Error(error.exception?.description ?? error.text);
  return result.result?.result?.value;
}

/** Warten, bis der Ausdruck in der Seite wahr ist. */
async function until(condition, timeout = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if (await run(`return Boolean(${condition});`)) return true;
    await sleep(250);
  }
  throw new Error(`Zeitüberschreitung: ${condition}`);
}

async function shot(name) {
  await sleep(600);
  const result = await send("Page.captureScreenshot", { format: "png" });
  const file = path.join(outDir, `${name}.png`);
  fs.writeFileSync(file, Buffer.from(result.result.data, "base64"));
  console.log("✓", path.relative(root, file));
}

const escape = () => run(`document.activeElement?.blur(); window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));`);
const clickText = (selector, text) =>
  run(`const el = [...document.querySelectorAll(args.selector)].find((e) => e.textContent.trim().startsWith(args.text)); if (!el) throw new Error("nicht gefunden: " + args.text); el.click(); return true;`, { selector, text });

// ---------------------------------------------------------------- Beispieldokument

const LATEX = String.raw`\documentclass[11pt,a4paper]{article}
\usepackage[ngerman]{babel}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage[numbers]{natbib}
\begin{document}

\section{Frequenzgang eines RC-Tiefpasses}

Dieser Bericht untersucht das Übertragungsverhalten eines RC-Tiefpasses. Unterhalb der Grenzfrequenz $f_g = \frac{1}{2\pi RC}$ passiert das Signal nahezu ungedämpft, darüber fällt die Amplitude mit 20~dB pro Dekade ab~\cite{tietze2019}.

\begin{equation}
H(j\omega) = \frac{U_a}{U_e} = \frac{1}{1 + j\omega RC}
\end{equation}

\subsection{Messaufbau}

\begin{itemize}
\item Funktionsgenerator mit sinusförmigem Signal, 2~V Spitze-Spitze
\item Zweikanal-Oszilloskop für Ein- und Ausgangsspannung
\item Widerstand $R = 1\,\mathrm{k\Omega}$, Kondensator $C = 100\,\mathrm{nF}$
\end{itemize}

\subsection{Ergebnisse}

\begin{table}[htbp]
\centering
\begin{tabular}{lcc}
\hline
Frequenz & $U_a/U_e$ & Phase \\
\hline
100 Hz & 0,99 & $-3{,}6^\circ$ \\
1 kHz & 0,85 & $-32{,}1^\circ$ \\
1,59 kHz & 0,71 & $-45{,}0^\circ$ \\
10 kHz & 0,16 & $-81{,}0^\circ$ \\
\hline
\end{tabular}
\caption{Gemessene Amplitudenverhältnisse und Phasen}
\label{tab:messung}
\end{table}

Bei der Grenzfrequenz ist die Amplitude auf $1/\sqrt{2}$ abgefallen; der Betrag folgt
\begin{equation}
|H(j\omega)| = \frac{1}{\sqrt{1 + (\omega RC)^2}}
\end{equation}
Die Messwerte aus Tabelle~\ref{tab:messung} stimmen bis auf wenige Prozent mit der Rechnung überein (Abbildung~\ref{fig:verlauf}). TODO: Fehlerrechnung ergänzen.

\bibliographystyle{plainnat}
\bibliography{literatur}

\end{document}
`;

const BIB = `@book{tietze2019,
  author    = {Tietze, Ulrich and Schenk, Christoph and Gamm, Eberhard},
  title     = {Halbleiter-Schaltungstechnik},
  edition   = {16},
  publisher = {Springer Vieweg},
  address   = {Berlin},
  year      = {2019}
}
`;

const CHART = {
  kind: "line",
  title: "Frequenzgang",
  xLabel: "Frequenz",
  yLabel: "Ua / Ue",
  data: [
    ["Frequenz", "Ua / Ue (Messung)", "Ua / Ue (Rechnung)"],
    ["100 Hz", "0,99", "0,998"],
    ["1 kHz", "0,85", "0,847"],
    ["1,59 kHz", "0,71", "0,707"],
    ["10 kHz", "0,16", "0,157"],
  ],
  legend: "below",
  grid: true,
  width: 11,
  stacked: false,
  marks: true,
  smooth: false,
  decimalComma: true,
};

// RC-Tiefpass als Skizze (Rastereinheiten à 0,5 cm)
const SKETCH = {
  version: 1,
  elements: [
    { id: "v", kind: "component", component: "V", x1: 4, y1: 12, x2: 4, y2: 6, label: "U_e" },
    { id: "w1", kind: "line", x1: 4, y1: 6, x2: 7, y2: 6 },
    { id: "r", kind: "component", component: "R", x1: 7, y1: 6, x2: 13, y2: 6, label: "R", annotation: "1 kΩ" },
    { id: "w2", kind: "line", x1: 13, y1: 6, x2: 18, y2: 6 },
    { id: "c", kind: "component", component: "C", x1: 18, y1: 6, x2: 18, y2: 12, label: "C", annotation: "100 nF" },
    { id: "w3", kind: "line", x1: 4, y1: 12, x2: 22, y2: 12 },
    { id: "w4", kind: "line", x1: 18, y1: 6, x2: 22, y2: 6 },
    { id: "t1", kind: "terminal", x1: 22, y1: 6, x2: 22, y2: 6 },
    { id: "t2", kind: "terminal", x1: 22, y1: 12, x2: 22, y2: 12 },
    { id: "ua", kind: "text", x1: 23, y1: 9, x2: 23, y2: 9, label: "$U_a$" },
  ],
};

// ---------------------------------------------------------------- Ablauf

fs.mkdirSync(outDir, { recursive: true });
fs.rmSync(exampleDir, { recursive: true, force: true });
fs.mkdirSync(exampleDir, { recursive: true });
fs.writeFileSync(path.join(exampleDir, "Messbericht.tex"), LATEX);
fs.writeFileSync(path.join(exampleDir, "literatur.bib"), BIB);
fs.copyFileSync(path.join(root, "tests", "fixtures", "slides.json"), path.join(exampleDir, "Beispielvortrag.vtxslides"));
const projectPath = `${exampleDir}/Messbericht.visutex`;

// Zustand merken, helles Design, Fenstergröße
const saved = await run(`return { prefs: localStorage.getItem("visutex-prefs"), recent: localStorage.getItem("visutex-recent") };`);
await run(`const prefs = JSON.parse(localStorage.getItem("visutex-prefs") || "{}"); prefs.theme = "light"; localStorage.setItem("visutex-prefs", JSON.stringify(prefs)); localStorage.removeItem("visutex-recovery"); location.reload(); return true;`).catch(() => undefined);
await sleep(4000);
await until("window.__visutex && document.querySelector('.ProseMirror')", 60000);
await send("Emulation.setDeviceMetricsOverride", { width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false });
await sleep(800);

try {
  // Dokument öffnen, als Projektdatei speichern (Titel „Messbericht.visutex“)
  await run(`document.querySelector(".backstage-back")?.click(); await v.openDocument(args.file); return true;`, { file: `${exampleDir}/Messbericht.tex` });
  await until("window.__visutex.state().docState?.name === 'Messbericht.tex'");
  await run(`if (document.querySelector(".vtx-show-marks")) v.runShortcut("view.formattingMarks"); return true;`);

  // Diagramm und Skizze einfügen (vor dem Literaturverzeichnis)
  const codes = await run(
    `const chart = await window.__TAURI_INTERNALS__.invoke("chart_to_latex", { chart: args.chart });
     const sketch = await window.__TAURI_INTERNALS__.invoke("sketch_to_latex", { sketch: args.sketch });
     return { chart, sketch };`,
    { chart: CHART, sketch: SKETCH },
  );
  await run(
    `const e = v.editor;
     let anchor = null;
     e.state.doc.forEach((node, offset) => { if (anchor === null && node.textContent.startsWith("Bei der Grenzfrequenz")) anchor = offset; });
     const insertAt = anchor ?? e.state.doc.content.size;
     e.chain().insertContentAt(insertAt, [
       { type: "tikzBlock", attrs: { code: args.chart, environment: "tikzpicture", options: "", caption: "Gemessener und berechneter Frequenzgang", label: "fig:verlauf" } },
     ]).run();
     let messaufbau = null;
     e.state.doc.forEach((node, offset) => { if (messaufbau === null && node.type.name === "bulletList") messaufbau = offset + node.nodeSize; });
     e.chain().insertContentAt(messaufbau ?? insertAt, [
       { type: "tikzBlock", attrs: { code: args.sketch.code, environment: args.sketch.environment, options: "" } },
     ]).run();
     return true;`,
    { chart: codes.chart, sketch: codes.sketch },
  );
  await run(`await v.saveTo(args.path, "visutex", v.state().docState.root); return true;`, { path: projectPath });
  await until("window.__visutex.state().docState?.name === 'Messbericht.visutex'");
  await until("document.querySelectorAll('.tikz-block img.tikz-preview').length >= 2", 90000);

  // Kompilieren → Hauptansicht (nichts ausgewählt, oben im Dokument)
  await run(`v.editor.commands.setTextSelection(3); v.editor.commands.blur(); return true;`);
  await run(`await v.compile(); return true;`);
  await until("document.querySelector('.pdf-page img')?.complete", 60000);
  await run(`document.querySelector(".document-scroller")?.scrollTo(0, 0); document.querySelectorAll(".pdf-scroll, .pdf-pages").forEach((el) => el.scrollTo?.(0, 0)); return true;`);
  if (want("hauptansicht")) await shot("hauptansicht");

  // PDF ↔ Dokument: Cursor in einen Absatz, „Im PDF zeigen“ (Zuordnung ist absatzweise)
  if (want("pdf-sync")) {
    await run(
      `const e = v.editor; let pos = null;
       e.state.doc.forEach((node, offset) => { if (pos === null && node.textContent.startsWith("Die Messwerte")) pos = offset + 5; });
       e.commands.setTextSelection(pos ?? 1);
       e.view.domAtPos(pos ?? 1).node.parentElement?.scrollIntoView({ block: "center" });
       await wait(300); v.actions.showInPdf(); return true;`,
    );
    await until("document.querySelector('.pdf-sync-marker')");
    await shot("pdf-sync");
  }

  // Formel-Editor
  if (want("formel-editor")) {
    await run(
      `const e = v.editor; let pos = null;
       e.state.doc.forEach((node, offset) => { if (pos === null && node.type.name === "mathBlock") pos = offset; });
       e.commands.setNodeSelection(pos); e.commands.scrollIntoView(); return true;`,
    );
    await sleep(1500);
    await run(`const b = document.querySelector("button[title='Bruch']"); b?.click(); return Boolean(b);`);
    await shot("formel-editor");
    await escape();
    await run(`v.editor.commands.setTextSelection(1); return true;`);
  }

  // Code-Ansicht
  if (want("code-ansicht")) {
    await run(`await v.setMode("code"); return true;`);
    await until("document.querySelector('.monaco-editor .view-lines')", 30000);
    await shot("code-ansicht");
    await run(`await v.setMode("visual"); return true;`);
    await sleep(800);
  }

  // Skizze: „In Skizze bearbeiten“ an der eingefügten Skizze
  if (want("skizzen")) {
    await run(
      `const e = v.editor; let pos = null;
       e.state.doc.forEach((node, offset) => { if (pos === null && node.type.name === "tikzBlock" && String(node.attrs.code).startsWith("% VisuTeX-Skizze")) pos = offset; });
       e.commands.setNodeSelection(pos); e.view.nodeDOM(pos)?.scrollIntoView({ block: "center" }); await wait(600);
       return true;`,
    );
    await clickText(".tikz-block button", "In Skizze bearbeiten");
    await until("document.querySelector('.sketch-pad .sketch-toolbar')");
    await sleep(1200);
    await shot("skizzen");
    await clickText(".sketch-pad button", "Schließen").catch(() => escape());
    await sleep(500);
  }

  // Diagramm-Dialog: Doppelklick auf das Diagramm
  if (want("diagramm")) {
    await run(
      `const frame = [...document.querySelectorAll(".tikz-block .tikz-preview-frame")].find((f) => f.title.includes("Diagramm"));
       frame.scrollIntoView({ block: "center" }); frame.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); return true;`,
    );
    await until("document.querySelector('.chart-preview-image:not(.is-stale)')", 60000);
    await shot("diagramm");
    await clickText(".chart-dialog .modal-footer button", "Abbrechen");
  }

  // Abgabe-Check
  if (want("abgabe-check")) {
    await run(`v.actions.openReview(); return true;`);
    await until("document.querySelector('.review-summary')", 30000);
    await clickText(".ribbon-tabs button, [role=tab]", "Überprüfen").catch(() => undefined);
    await shot("abgabe-check");
    await run(`document.querySelector(".review-pane .review-header button[title='Schließen']")?.click(); return true;`);
  }

  // Versionsverlauf und Vergleich: zweite Fassung speichern, ältere Version vergleichen
  if (want("versionen")) {
    await run(
      `const e = v.editor; let pos = null;
       e.state.doc.descendants((node, p) => { if (pos === null && node.isText && node.text.includes("TODO: Fehlerrechnung ergänzen.")) pos = p + node.text.indexOf("TODO"); });
       if (pos !== null) e.chain().insertContentAt({ from: pos, to: pos + "TODO: Fehlerrechnung ergänzen.".length }, "Die Abweichung liegt unter 3 %, bedingt durch Bauteiltoleranzen.").run();
       await v.saveTo(v.state().docState.path, "visutex", v.state().docState.root); await wait(500);
       v.actions.openHistory(); return true;`,
    );
    await until("document.querySelectorAll('.history-group li').length >= 2");
    await clickText(".ribbon-tabs button, [role=tab]", "Überprüfen").catch(() => undefined);
    await run(`const items = document.querySelectorAll(".history-group li"); items[items.length - 1].querySelector(".history-entry").click(); return true;`);
    await until("document.querySelector('.compare-dialog .compare-view')");
    await shot("versionen");
    await clickText(".compare-dialog .modal-footer button", "Schließen");
    await run(`document.querySelector(".history-pane .review-header button[title='Schließen']")?.click(); return true;`);
  }

  // Teilen (mit ungespeicherter Änderung)
  if (want("teilen")) {
    await run(`v.editor.chain().setTextSelection(3).insertContent(" ").run(); v.runShortcut("file.share"); return true;`);
    await until("document.querySelector('.share-dialog')");
    await shot("teilen");
    await clickText(".share-dialog .modal-footer button", "Abbrechen");
    await run(`v.editor.commands.undo(); return true;`);
  }

  // Literatur per DOI (Internet)
  if (want("literatur-doi")) {
    await run(
      `v.actions.insertCitationByIdentifier(); await wait(500);
       const input = document.querySelector(".citation-doi input");
       Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, "10.1038/nature14539");
       input.dispatchEvent(new Event("input", { bubbles: true })); await wait(100);
       document.querySelector(".citation-doi form button[type=submit]").click(); return true;`,
    );
    await until("document.querySelector('.found-reference')", 30000).catch(() => console.warn("DOI-Suche ohne Ergebnis (Internet?)"));
    await shot("literatur-doi");
    await clickText(".citation-dialog .modal-footer button", "Abbrechen");
  }

  // Schnellzugriff (Datei → Optionen)
  if (want("schnellzugriff")) {
    await run(`v.runShortcut("file.options"); await wait(800); document.getElementById("optionen-schnellzugriff")?.scrollIntoView({ block: "start" }); return true;`);
    await sleep(600);
    await shot("schnellzugriff");
    await run(`document.querySelector(".backstage-back")?.click(); return true;`);
  }

  // Exportieren (Projekt-ZIP für Overleaf)
  if (want("exportieren")) {
    await run(`v.runShortcut("file.new"); await wait(500); return true;`);
    await clickText(".backstage-nav button, .backstage button", "Exportieren");
    await sleep(600);
    await shot("exportieren");
    await run(`document.querySelector(".backstage-back")?.click(); return true;`);
  }

  // Folien
  if (want("folien")) {
    await run(`await v.openPresentation(args.file); return true;`, { file: `${exampleDir}/Beispielvortrag.vtxslides` });
    await until("document.querySelector('.slide-editor:not(.loading)')", 30000);
    await sleep(1500);
    await shot("folien");
  }
} finally {
  // Aufräumen: Arbeitsdaten der Projektdatei, Einstellungen, „Zuletzt verwendet“, Wiederherstellung
  const workDir = await run(`return await window.__TAURI_INTERNALS__.invoke("history_location", { store: { kind: "project", path: args.path } });`, { path: projectPath }).catch(() => null);
  await send("Emulation.clearDeviceMetricsOverride");
  await run(
    `localStorage.removeItem("visutex-recovery");
     if (args.prefs === null) localStorage.removeItem("visutex-prefs"); else localStorage.setItem("visutex-prefs", args.prefs);
     if (args.recent === null) localStorage.removeItem("visutex-recent"); else localStorage.setItem("visutex-recent", args.recent);
     return true;`,
    saved,
  ).catch(() => undefined);
  if (workDir) fs.rmSync(path.dirname(workDir), { recursive: true, force: true });
  fs.rmSync(exampleDir, { recursive: true, force: true });
  console.log("Aufgeräumt. App neu starten, damit Design und Liste „Zuletzt verwendet“ wieder gelten.");
  ws.close();
}
process.exit(0);
