// Unsichtbare Layout-Befehle werden erkannt (Randmarkierung statt Codeblock), Sichtbares nicht.
import { test } from "node:test";
import assert from "node:assert/strict";
import { isLayoutOnly, layoutCommands, layoutSummary } from "../src/editor/layoutCommands.ts";

test("reine Layout-Befehle", () => {
  for (const latex of [
    String.raw`\setcounter{page}{1}`,
    String.raw`\begingroup`,
    String.raw`\endgroup`,
    String.raw`\setstretch{1.0}`,
    String.raw`\begingroup\setlength{\tabcolsep}{4pt}\renewcommand{\arraystretch}{1.2}`,
    String.raw`\pagenumbering{arabic} % ab hier arabisch`,
    String.raw`\linespread{1.5}\selectfont`,
    String.raw`\setlength\parindent{0pt}`,
    String.raw`\parindent=0pt`.replace(String.raw`\parindent`, String.raw`\setlength\parindent`),
    String.raw`\def\foo#1{\textbf{#1}}`,
    String.raw`\let\oldsection\section`,
    String.raw`\thispagestyle{empty}`,
    String.raw`\addcontentsline{toc}{chapter}{Literatur}`,
    String.raw`\newcommand*{\R}{\mathbb{R}}`,
  ]) {
    assert.ok(isLayoutOnly(latex), latex);
  }
  assert.deepEqual(layoutCommands(String.raw`\begingroup\setstretch{1}`), ["begingroup", "setstretch"]);
});

test("sichtbare Ausgabe bleibt Codeblock", () => {
  for (const latex of [
    "",
    "% nur Kommentar",
    String.raw`\vspace{1cm}`,
    String.raw`\clearpage`,
    String.raw`\newpage`,
    String.raw`\pagebreak`,
    String.raw`\noindent Text`,
    String.raw`\begin{center}x\end{center}`,
    String.raw`\includegraphics{bild}`,
    String.raw`\setcounter{page}{1} Hallo`,
    String.raw`\setcounter{page}{1`,
    String.raw`$x$`,
  ]) {
    assert.equal(isLayoutOnly(latex), false, JSON.stringify(latex));
  }
});

test("Kurzfassung ohne Kommentare in einer Zeile", () => {
  assert.equal(layoutSummary("\\setcounter{page}{1} % Seite\n\\pagestyle{plain}"), String.raw`\setcounter{page}{1} \pagestyle{plain}`);
  // escaptes Prozentzeichen ist kein Kommentar
  assert.equal(layoutSummary(String.raw`\setlength{\parskip}{0pt}\def\p{\%}`), String.raw`\setlength{\parskip}{0pt}\def\p{\%}`);
});
