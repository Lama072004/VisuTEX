/**
 * Kleiner LaTeX→HTML-Renderer für die Anzeige im Editor: Beschriftungen mit
 * Formeln, Roh-LaTeX im Text (`\quad`, `\enquote{…}`, eigene Makros wie
 * `\teil{a}`), Box-Titel. Nur Darstellung – der LaTeX-Code bleibt unverändert.
 */
import katex from "katex";
import { formatQuantity, siunitxForKatex } from "./siunitx";
import { documentLanguage } from "./languages";

export type MacroDef = { name: string; args: number; default?: string | null; body: string };

export type RenderContext = {
  macros?: Map<string, MacroDef>;
  language?: string;
  germanShorthands?: boolean;
};

const escapeHtml = (text: string) =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** KaTeX-Makros aus den eigenen Makros (ohne optionale Argumente). */
export function katexMacros(macros?: Map<string, MacroDef>): Record<string, string> {
  const result: Record<string, string> = {};
  macros?.forEach((definition) => {
    if (definition.default === null || definition.default === undefined) result[`\\${definition.name}`] = definition.body;
  });
  return result;
}

export function renderMath(latex: string, displayMode: boolean, context: RenderContext = {}): string {
  try {
    return katex.renderToString(siunitxForKatex(latex.trim() || "\\square"), {
      displayMode,
      throwOnError: false,
      strict: "ignore",
      trust: false,
      macros: katexMacros(context.macros),
    });
  } catch {
    return `<code>${escapeHtml(latex)}</code>`;
  }
}

/** Liest `{…}` bzw. `[…]` ab `start` (Leerzeichen davor erlaubt). */
function readGroup(source: string, start: number, open: string, close: string): [string, number] | null {
  let index = start;
  while (source[index] === " ") index += 1;
  if (source[index] !== open) return null;
  let depth = 0;
  for (let position = index; position < source.length; position += 1) {
    const character = source[position];
    if (character === "\\") {
      position += 1;
      continue;
    }
    if (character === open) depth += 1;
    if (character === close) {
      depth -= 1;
      if (depth === 0) return [source.slice(index + 1, position), position + 1];
    }
  }
  return null;
}

const SPACES: Record<string, string> = {
  ",": " ",
  ";": " ",
  ":": " ",
  " ": " ",
  "!": "",
  quad: " ",
  qquad: "  ",
  enspace: " ",
  thinspace: " ",
  space: " ",
};

const SYMBOLS: Record<string, string> = {
  ldots: "…", dots: "…", textellipsis: "…", textbackslash: "\\", textbullet: "•", textdegree: "°",
  euro: "€", EUR: "€", texteuro: "€", copyright: "©", textregistered: "®", texttrademark: "™", S: "§", P: "¶",
  ss: "ß", aa: "å", AA: "Å", ae: "æ", AE: "Æ", oe: "œ", OE: "Œ", o: "ø", O: "Ø", l: "ł", L: "Ł",
  LaTeX: "LaTeX", TeX: "TeX", LaTeXe: "LaTeX2ε", textendash: "–", textemdash: "—", textquotedblleft: "“",
  textquotedblright: "”", glqq: "„", grqq: "“", glq: "‚", grq: "‘", textasciitilde: "~", textunderscore: "_",
  checkmark: "✓", textless: "<", textgreater: ">", textbar: "|", dag: "†", ddag: "‡",
};

const ACCENTS: Record<string, string> = { '"': "̈", "'": "́", "`": "̀", "^": "̂", "~": "̃", "=": "̄", ".": "̇", c: "̧", v: "̌", u: "̆", H: "̋" };

const SIZE_CLASSES = new Set(["tiny", "scriptsize", "footnotesize", "small", "normalsize", "large", "Large", "LARGE", "huge", "Huge"]);

/** Befehle ohne sichtbare Ausgabe (Layout-Anweisungen). */
const INVISIBLE = new Set([
  "noindent", "indent", "centering", "raggedright", "raggedleft", "label", "index", "protect", "relax", "nopagebreak",
  "pagebreak", "linebreak", "nolinebreak", "smallskip", "medskip", "bigskip", "vfill", "phantomsection", "leavevmode",
  "selectfont", "normalfont", "hspace", "vspace", "setlength", "addtolength", "setcounter", "addtocounter",
  "thispagestyle", "pagestyle", "pagenumbering", "addcontentsline", "begingroup", "endgroup", "color", "nocite",
  "setstretch", "linespread", "enlargethispage", "newpage", "clearpage", "FloatBarrier", "allowbreak", "sloppy",
  "fussy", "hyphenation", "selectlanguage", "captionsetup", "renewcommand", "setcounter",
]);

const WRAPPERS: Record<string, [string, string]> = {
  textbf: ["<strong>", "</strong>"],
  textit: ["<em>", "</em>"],
  emph: ["<em>", "</em>"],
  textsl: ["<em>", "</em>"],
  underline: ["<u>", "</u>"],
  uline: ["<u>", "</u>"],
  sout: ["<s>", "</s>"],
  texttt: ["<code>", "</code>"],
  textsf: ['<span class="tex-sf">', "</span>"],
  textsc: ['<span class="tex-sc">', "</span>"],
  textrm: ["<span>", "</span>"],
  textup: ["<span>", "</span>"],
  textmd: ["<span>", "</span>"],
  textnormal: ["<span>", "</span>"],
  textsuperscript: ["<sup>", "</sup>"],
  textsubscript: ["<sub>", "</sub>"],
  mbox: ["<span>", "</span>"],
  hbox: ["<span>", "</span>"],
  text: ["<span>", "</span>"],
  fbox: ['<span class="tex-fbox">', "</span>"],
  framebox: ['<span class="tex-fbox">', "</span>"],
  phantom: ['<span class="tex-phantom">', "</span>"],
  hphantom: ['<span class="tex-phantom">', "</span>"],
  url: ['<span class="tex-url">', "</span>"],
  nolinkurl: ['<span class="tex-url">', "</span>"],
};

const DECLARATIONS: Record<string, [string, string]> = {
  bfseries: ["<strong>", "</strong>"],
  itshape: ["<em>", "</em>"],
  em: ["<em>", "</em>"],
  slshape: ["<em>", "</em>"],
  ttfamily: ["<code>", "</code>"],
  sffamily: ['<span class="tex-sf">', "</span>"],
  scshape: ['<span class="tex-sc">', "</span>"],
};

type Frame = { closers: string[] };

/**
 * Wandelt LaTeX-Text in HTML. Unbekannte Befehle mit Argumenten zeigen ihr
 * letztes Argument, ohne Argumente einen dezenten Platzhalter.
 */
export function latexToHtml(source: string, context: RenderContext = {}, depth = 0): string {
  if (depth > 8) return escapeHtml(source);
  let output = "";
  const frames: Frame[] = [{ closers: [] }];
  const open = (start: string, end: string) => {
    output += start;
    frames[frames.length - 1].closers.unshift(end);
  };
  const render = (text: string) => latexToHtml(text, context, depth + 1);
  let index = 0;
  while (index < source.length) {
    const character = source[index];
    // Kommentar
    if (character === "%") {
      const end = source.indexOf("\n", index);
      index = end < 0 ? source.length : end + 1;
      continue;
    }
    // Mathematik
    if (character === "$") {
      const display = source[index + 1] === "$";
      const delimiter = display ? "$$" : "$";
      let end = index + delimiter.length;
      while (end < source.length) {
        if (source[end] === "\\") {
          end += 2;
          continue;
        }
        if (source.startsWith(delimiter, end)) break;
        end += 1;
      }
      output += renderMath(source.slice(index + delimiter.length, end), display, context);
      index = end + delimiter.length;
      continue;
    }
    if (character === "{") {
      const group = readGroup(source, index, "{", "}");
      if (group) {
        output += render(group[0]);
        index = group[1];
        continue;
      }
    }
    if (character === "}") {
      index += 1;
      continue;
    }
    if (character === "~") {
      output += " ";
      index += 1;
      continue;
    }
    if (character === "-" && source.startsWith("---", index)) {
      output += "—";
      index += 3;
      continue;
    }
    if (character === "-" && source.startsWith("--", index)) {
      output += "–";
      index += 2;
      continue;
    }
    if (source.startsWith("``", index) || source.startsWith("''", index)) {
      output += source[index] === "`" ? "“" : "”";
      index += 2;
      continue;
    }
    if (character === '"' && context.germanShorthands) {
      const next = source[index + 1] ?? "";
      const shorthand: Record<string, string> = { a: "ä", o: "ö", u: "ü", A: "Ä", O: "Ö", U: "Ü", s: "ß", "`": "„", "'": "“", "-": "", "=": "-", "~": "-", "|": "" };
      if (next in shorthand) {
        output += shorthand[next];
        index += 2;
        continue;
      }
    }
    if (character !== "\\") {
      output += escapeHtml(character);
      index += 1;
      continue;
    }

    // Befehle
    const match = /^\\([A-Za-z@]+|.)/.exec(source.slice(index));
    if (!match) {
      index += 1;
      continue;
    }
    const name = match[1];
    let position = index + match[0].length;
    const isWord = /^[A-Za-z@]+$/.test(name);
    const skipSpace = () => {
      if (isWord) while (source[position] === " ") position += 1;
    };

    if (name === "\\") {
      if (source[position] === "*") position += 1;
      const optional = readGroup(source, position, "[", "]");
      if (optional) position = optional[1];
      output += "<br>";
      index = position;
      continue;
    }
    if ("%&#_{}$".includes(name)) {
      output += escapeHtml(name);
      index = position;
      continue;
    }
    if (name in SPACES) {
      output += SPACES[name];
      index = position;
      skipSpace();
      index = position;
      continue;
    }
    if (name in ACCENTS && (name.length === 1 || source[position] === "{")) {
      let letter = source[position] ?? "";
      if (letter === "{") {
        const group = readGroup(source, position, "{", "}");
        letter = group ? group[0] : "";
        position = group ? group[1] : position + 1;
      } else {
        position += 1;
      }
      output += escapeHtml(`${letter.replace("\\i", "ı")}${ACCENTS[name]}`.normalize("NFC"));
      index = position;
      continue;
    }
    if (name === "newline" || name === "par") {
      output += "<br>";
      index = position;
      continue;
    }
    if (name === "hfill" || name === "hfil") {
      output += '<span class="tex-hfill"></span>';
      index = position;
      continue;
    }
    if (name === "today") {
      output += new Date().toLocaleDateString(documentLanguage(context.language).bcp47, { day: "numeric", month: "long", year: "numeric" });
      index = position;
      continue;
    }
    if (name in SYMBOLS && !(name === "EUR" && source[position] === "{")) {
      output += SYMBOLS[name];
      if (source.startsWith("{}", position)) position += 2;
      index = position;
      skipSpace();
      index = position;
      continue;
    }
    if (name === "(" || name === "[") {
      const end = source.indexOf(name === "(" ? "\\)" : "\\]", position);
      const stop = end < 0 ? source.length : end;
      output += renderMath(source.slice(position, stop), name === "[", context);
      index = stop + 2;
      continue;
    }
    if (DECLARATIONS[name]) {
      open(...DECLARATIONS[name]);
      index = position;
      skipSpace();
      index = position;
      continue;
    }
    if (SIZE_CLASSES.has(name)) {
      open(`<span class="tex-size-${name.toLowerCase()}${name === "LARGE" ? "-xl" : ""}">`, "</span>");
      index = position;
      skipSpace();
      index = position;
      continue;
    }

    // Argumente einlesen
    const optional = readGroup(source, position, "[", "]");
    const readArgs = (count: number) => {
      const args: string[] = [];
      for (let argument = 0; argument < count; argument += 1) {
        const group = readGroup(source, position, "{", "}");
        if (!group) break;
        args.push(group[0]);
        position = group[1];
      }
      return args;
    };

    // Eigene Makros aus der Präambel
    const definition = context.macros?.get(name);
    if (definition) {
      let args: string[] = [];
      let required = definition.args;
      if (definition.default !== null && definition.default !== undefined && definition.args > 0) {
        if (optional) {
          args.push(optional[0]);
          position = optional[1];
        } else {
          args.push(definition.default);
        }
        required -= 1;
      }
      args = [...args, ...readArgs(required)];
      const expanded = definition.body.replace(/#(\d)/g, (_, number: string) => args[Number(number) - 1] ?? "");
      output += render(expanded);
      index = position;
      if (definition.args === 0) skipSpace();
      if (definition.args === 0) index = position;
      continue;
    }

    if (WRAPPERS[name]) {
      const [argument] = readArgs(1);
      const [start, end] = WRAPPERS[name];
      output += `${start}${argument === undefined ? "" : render(argument)}${end}`;
      index = position;
      continue;
    }
    switch (name) {
      case "enquote": {
        const [argument = ""] = readArgs(1);
        const [left, right] = documentLanguage(context.language).quotes;
        output += `${left}${render(argument)}${right}`;
        index = position;
        continue;
      }
      case "EUR": {
        const [argument = ""] = readArgs(1);
        output += `${render(argument)} €`;
        index = position;
        continue;
      }
      case "textcolor":
      case "colorbox": {
        if (optional) position = optional[1];
        const [color = "", text = ""] = readArgs(2);
        const css = optional?.[0] === "HTML" ? `#${color}` : color.replace(/!.*/, "");
        const style = name === "textcolor" ? `color:${escapeHtml(css)}` : `background:${escapeHtml(css)}`;
        output += `<span style="${style}">${render(text)}</span>`;
        index = position;
        continue;
      }
      case "href": {
        const [, text = ""] = readArgs(2);
        output += `<span class="tex-url">${render(text)}</span>`;
        index = position;
        continue;
      }
      case "footnote": {
        readArgs(1);
        output += "<sup>*</sup>";
        index = position;
        continue;
      }
      case "cite":
      case "citep":
      case "citet":
      case "parencite":
      case "textcite":
      case "autocite": {
        if (optional) position = optional[1];
        const second = readGroup(source, position, "[", "]");
        if (second) position = second[1];
        const [keys = ""] = readArgs(1);
        output += `[${escapeHtml(keys)}]`;
        index = position;
        continue;
      }
      case "ref":
      case "eqref":
      case "pageref":
      case "autoref": {
        const [label = ""] = readArgs(1);
        output += `<span class="tex-ref">${escapeHtml(name === "eqref" ? `(${label})` : label)}</span>`;
        index = position;
        continue;
      }
      case "SI":
      case "qty":
      case "si":
      case "unit":
      case "num":
      case "ang":
      case "SIrange":
      case "qtyrange":
      case "numrange": {
        if (optional) position = optional[1];
        const count = ["si", "unit", "num", "ang"].includes(name) ? 1 : name === "numrange" ? 2 : ["SIrange", "qtyrange"].includes(name) ? 3 : 2;
        const args = readArgs(count);
        const [first = "", second = "", third = ""] = args;
        const attrs =
          name === "si" || name === "unit"
            ? { command: name, unit: first }
            : name === "num" || name === "ang"
              ? { command: name, value: first }
              : name === "numrange"
                ? { command: name, value: first, value2: second }
                : count === 3
                  ? { command: name, value: first, value2: second, unit: third }
                  : { command: name, value: first, unit: second };
        output += escapeHtml(formatQuantity(attrs, context.language));
        index = position;
        continue;
      }
      default:
        break;
    }
    if (INVISIBLE.has(name)) {
      if (optional) position = optional[1];
      if (["label", "index", "hspace", "vspace", "setlength", "addtolength", "setcounter", "addtocounter", "thispagestyle", "pagestyle", "pagenumbering", "addcontentsline", "color", "nocite", "setstretch", "linespread", "enlargethispage", "hyphenation", "selectlanguage", "captionsetup", "renewcommand"].includes(name)) {
        if (source[position] === "*") position += 1;
        const count = ["setlength", "addtolength", "setcounter", "addtocounter", "renewcommand"].includes(name) ? 2 : name === "addcontentsline" ? 3 : 1;
        readArgs(count);
      }
      if (name === "hspace") output += " ";
      index = position;
      skipSpace();
      index = position;
      continue;
    }
    // Unbekannt: letztes Argument anzeigen, sonst Platzhalter
    if (optional) position = optional[1];
    const args = readArgs(3);
    if (args.length) {
      output += `<span class="tex-macro" title="\\${escapeHtml(name)}">${render(args[args.length - 1])}</span>`;
    } else {
      output += `<span class="tex-cmd">\\${escapeHtml(name)}</span>`;
      skipSpace();
    }
    index = position;
  }
  for (const frame of frames) output += frame.closers.join("");
  return output;
}

/** Text ohne Formatierung (für Verzeichnisse, Tooltips). */
export function latexToPlain(source: string, context: RenderContext = {}): string {
  const html = latexToHtml(source, context);
  const element = document.createElement("div");
  element.innerHTML = html;
  return (element.textContent ?? "").replace(/\s+/g, " ").trim();
}
