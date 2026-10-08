/**
 * Absätze, die nur unsichtbare Layout-Befehle enthalten (`\setcounter`, `\setlength`,
 * `\begingroup`, `\pagestyle` …), erzeugen im PDF keine eigene Ausgabe. Im Editor stehen sie
 * deshalb nicht als Codeblock im Text, sondern als kleine Markierung am linken Seitenrand.
 */

/** Befehle ohne sichtbare Ausgabe an der Stelle, an der sie stehen. */
const INVISIBLE = new Set([
  // Zähler und Längen
  "setcounter", "addtocounter", "stepcounter", "refstepcounter", "setlength", "addtolength", "settowidth",
  "settoheight", "settodepth", "newcounter", "newlength",
  // Gruppen und Schrift-/Abstandsumschaltungen
  "begingroup", "endgroup", "bgroup", "egroup", "selectfont", "linespread", "setstretch", "singlespacing",
  "onehalfspacing", "doublespacing", "normalfont", "rmfamily", "sffamily", "ttfamily", "bfseries", "mdseries",
  "itshape", "upshape", "slshape", "scshape", "tiny", "scriptsize", "footnotesize", "small", "normalsize", "large",
  "Large", "LARGE", "huge", "Huge", "fontsize", "color", "raggedright", "raggedleft", "centering", "justifying",
  "frenchspacing", "nonfrenchspacing", "raggedbottom", "flushbottom", "noindent", "indent",
  // Seitenstil und Nummerierung
  "pagestyle", "thispagestyle", "pagenumbering", "markboth", "markright", "automark",
  // Definitionen und Einstellungen
  "newcommand", "renewcommand", "providecommand", "def", "gdef", "edef", "xdef", "let", "makeatletter", "makeatother",
  "hypersetup", "captionsetup", "setkomafont", "addtokomafont", "KOMAoptions", "sisetup", "graphicspath", "setlist",
  "hyphenation", "urlstyle", "allowdisplaybreaks", "numberwithin", "counterwithin", "counterwithout",
  "DeclareMathOperator", "newtheorem", "theoremstyle", "lstset", "tikzset", "pgfplotsset", "usetikzlibrary",
  "definecolor", "colorlet", "renewenvironment", "newenvironment", "setcapindent", "deffootnote", "ofoot", "ifoot",
  "cfoot", "ohead", "ihead", "chead", "lehead", "rohead", "clearpairofpagestyles", "addcontentsline",
  "addtocontents", "phantomsection", "label", "nocite", "glsresetall", "acresetall", "FloatBarrier", "balance",
  "nopagebreak", "nolinebreak", "enlargethispage", "relax", "ignorespaces",
]);

/** Befehle, deren erstes Argument ein Befehlsname ohne Klammern sein darf (`\def\foo{…}`, `\let\a\b`). */
const TAKES_TOKEN = new Set(["def", "gdef", "edef", "xdef", "let", "newcommand", "renewcommand", "providecommand", "setlength", "addtolength", "newlength"]);

function stripComments(latex: string): string {
  return latex
    .split("\n")
    .map((line) => {
      for (let index = 0; index < line.length; index += 1) {
        if (line[index] === "\\") index += 1;
        else if (line[index] === "%") return line.slice(0, index);
      }
      return line;
    })
    .join("\n");
}

/** Ende der Gruppe ab `start` (Zeichen `open`), sonst -1. */
function groupEnd(text: string, start: number, open: string, close: string): number {
  let depth = 0;
  for (let index = start; index < text.length; index += 1) {
    const char = text[index];
    if (char === "\\") index += 1;
    else if (char === open) depth += 1;
    else if (char === close) {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

/** Zahl/Länge bei TeX-Zuweisungen (`\parindent=0pt`, `\parskip 6pt plus 2pt`). */
const ASSIGNMENT = /^(=\s*)?[-+]?(\d+(\.\d*)?|\.\d+)\s*(pt|mm|cm|in|em|ex|bp|sp|pc|dd|cc|mu)?(\s+(plus|minus)\s+[-+]?(\d+(\.\d*)?|\.\d+)\s*(pt|mm|cm|in|em|ex|bp|sp|fil+)?)*/;

/** Die Befehle eines Layout-Absatzes (ohne Argumente), oder `null`, wenn er Sichtbares enthält. */
export function layoutCommands(latex: string): string[] | null {
  const text = stripComments(latex);
  const commands: string[] = [];
  let index = 0;
  while (index < text.length) {
    const char = text[index];
    if (/\s/.test(char)) {
      index += 1;
    } else if (char === "\\") {
      const match = /^\\([A-Za-z@]+)\*?/.exec(text.slice(index));
      if (!match || !INVISIBLE.has(match[1])) return null;
      commands.push(match[1]);
      index += match[0].length;
      if (TAKES_TOKEN.has(match[1])) {
        const token = /^\s*\\[A-Za-z@]+/.exec(text.slice(index));
        if (token) index += token[0].length;
        if (match[1] === "let") {
          const target = /^\s*=?\s*\\[A-Za-z@]+/.exec(text.slice(index));
          if (target) index += target[0].length;
        }
        // Parametertext bei \def (#1#2)
        const params = /^(#\d)+/.exec(text.slice(index));
        if (params) index += params[0].length;
      }
      // Argumente: {…} und […], dazwischen Leerraum; Zuweisungen mit Zahl und Einheit
      for (;;) {
        const rest = text.slice(index);
        const space = /^\s*/.exec(rest)?.[0].length ?? 0;
        const next = text[index + space];
        if (next === "{" || next === "[") {
          const end = groupEnd(text, index + space, next, next === "{" ? "}" : "]");
          if (end < 0) return null;
          index = end + 1;
          continue;
        }
        const assignment = ASSIGNMENT.exec(rest.slice(space));
        if (assignment && assignment[0].trim()) {
          index += space + assignment[0].length;
          continue;
        }
        break;
      }
    } else {
      return null; // Text, Klammern ohne Befehl, Mathematik …
    }
  }
  return commands.length > 0 ? commands : null;
}

export function isLayoutOnly(latex: string): boolean {
  return layoutCommands(latex) !== null;
}

/** Kurzfassung für die Randmarkierung: Code in einer Zeile, Kommentare entfernt. */
export function layoutSummary(latex: string): string {
  return stripComments(latex).replace(/\s+/g, " ").trim();
}
