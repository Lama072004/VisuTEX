/**
 * Monaco lokal (kein CDN): Kern-API + ausgewählte Funktionen, eigener
 * Web-Worker, LaTeX-Syntaxhervorhebung (Monarch) und Vervollständigung für
 * Befehle, Umgebungen, Labels und Zitierschlüssel.
 */
import * as monaco from "monaco-editor/editor/editor.api";
import "monaco-editor/features/find/register";
import "monaco-editor/features/folding/register";
import "monaco-editor/features/bracketMatching/register";
import "monaco-editor/features/wordHighlighter/register";
import "monaco-editor/features/contextmenu/register";
import "monaco-editor/features/clipboard/register";
import "monaco-editor/features/comment/register";
import "monaco-editor/features/linesOperations/register";
import "monaco-editor/features/multicursor/register";
import "monaco-editor/features/suggest/register";
import "monaco-editor/features/hover/register";
import "monaco-editor/features/gotoError/register";
import "monaco-editor/features/smartSelect/register";
import "monaco-editor/features/wordOperations/register";
import "monaco-editor/features/indentation/register";
import "monaco-editor/features/cursorUndo/register";
import "monaco-editor/features/dnd/register";
import "../../node_modules/monaco-editor/esm/vs/base/browser/ui/codicons/codicon/codicon.css";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";

declare global {
  interface Window {
    MonacoEnvironment?: { getWorker: (workerId: string, label: string) => Worker };
  }
}

window.MonacoEnvironment = { getWorker: () => new EditorWorker() };

import { completionData } from "./completionData";

const COMMANDS = [
  "section", "subsection", "subsubsection", "chapter", "paragraph", "textbf", "textit", "emph", "underline",
  "texttt", "textsc", "footnote", "cite", "citep", "citet", "ref", "eqref", "pageref", "autoref", "label",
  "includegraphics", "caption", "centering", "item", "frac", "sqrt", "sum", "int", "alpha", "beta", "gamma",
  "delta", "varepsilon", "lambda", "mu", "pi", "sigma", "omega", "Omega", "cdot", "times", "leq", "geq",
  "neq", "approx", "infty", "partial", "nabla", "mathbb", "mathrm", "mathbf", "left", "right", "hline",
  "multicolumn", "multirow", "newpage", "clearpage", "tableofcontents", "listoffigures", "listoftables",
  "href", "url", "ac", "acs", "acl", "acp", "SI", "si", "usepackage", "documentclass", "vspace", "hspace",
  "noindent", "par", "textcolor", "color", "today",
];
const ENVIRONMENTS = [
  "itemize", "enumerate", "description", "figure", "table", "tabular", "tabularx", "equation", "equation*",
  "align", "align*", "gather", "multline", "center", "flushleft", "flushright", "quote", "quotation",
  "minipage", "tikzpicture", "circuitikz", "verbatim", "abstract", "titlepage", "landscape", "multicols",
];

let registered = false;

export function registerLatex() {
  if (registered) return;
  registered = true;
  monaco.languages.register({ id: "latex", extensions: [".tex", ".sty", ".cls"], aliases: ["LaTeX"] });
  monaco.languages.setLanguageConfiguration("latex", {
    comments: { lineComment: "%" },
    brackets: [["{", "}"], ["[", "]"], ["(", ")"]],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: "$", close: "$" },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "$", close: "$" },
    ],
    folding: {
      markers: { start: /^\s*(\\begin\{|%%\s*VisuTeX-begin)/, end: /^\s*(\\end\{|%%\s*VisuTeX-end)/ },
    },
    wordPattern: /\\?[A-Za-zÄÖÜäöüß@]+|[^\s\\{}[\]()$%]+/,
  });
  monaco.languages.setMonarchTokensProvider("latex", {
    defaultToken: "",
    tokenizer: {
      root: [
        [/%%\s*VisuTeX-(begin|end):.*$/, "comment.marker"],
        [/%.*$/, "comment"],
        [/(\\(?:begin|end))(\s*)(\{)([^}]*)(\})/, ["keyword", "", "delimiter.curly", "type", "delimiter.curly"]],
        [/\\(?:section|subsection|subsubsection|chapter|paragraph|part)\*?/, "keyword.heading"],
        [/\\(?:documentclass|usepackage|RequirePackage|input|include|bibliography|bibliographystyle)\b/, "keyword.preamble"],
        [/\\(?:label|ref|eqref|pageref|autoref|cite[pt]?|ac[slfp]?)\b/, "keyword.reference"],
        [/\\[A-Za-z@]+\*?/, "tag"],
        [/\\./, "tag"],
        [/\$\$/, { token: "string.math", next: "@displayMath" }],
        [/\$/, { token: "string.math", next: "@inlineMath" }],
        [/\\\[/, { token: "string.math", next: "@bracketMath" }],
        [/[{}]/, "delimiter.curly"],
        [/[[\]]/, "delimiter.square"],
        [/&/, "operator"],
      ],
      inlineMath: [
        [/\$/, { token: "string.math", next: "@pop" }],
        [/\\[A-Za-z]+/, "string.math.command"],
        [/[^$\\]+/, "string.math"],
        [/./, "string.math"],
      ],
      displayMath: [
        [/\$\$/, { token: "string.math", next: "@pop" }],
        [/\\[A-Za-z]+/, "string.math.command"],
        [/[^$\\]+/, "string.math"],
        [/./, "string.math"],
      ],
      bracketMath: [
        [/\\\]/, { token: "string.math", next: "@pop" }],
        [/\\[A-Za-z]+/, "string.math.command"],
        [/[^\\]+/, "string.math"],
        [/./, "string.math"],
      ],
    },
  });
  for (const [name, base] of [["visutex-light", "vs"], ["visutex-dark", "vs-dark"]] as const) {
    const dark = base === "vs-dark";
    monaco.editor.defineTheme(name, {
      base,
      inherit: true,
      rules: [
        { token: "comment", foreground: dark ? "6a9955" : "008000", fontStyle: "italic" },
        { token: "comment.marker", foreground: dark ? "8b949e" : "8c959f", fontStyle: "italic" },
        { token: "keyword", foreground: dark ? "c586c0" : "af00db" },
        { token: "keyword.heading", foreground: dark ? "4fc1ff" : "0451a5", fontStyle: "bold" },
        { token: "keyword.preamble", foreground: dark ? "c586c0" : "af00db" },
        { token: "keyword.reference", foreground: dark ? "dcdcaa" : "795e26" },
        { token: "tag", foreground: dark ? "569cd6" : "0000ff" },
        { token: "type", foreground: dark ? "4ec9b0" : "267f99" },
        { token: "string.math", foreground: dark ? "ce9178" : "a31515" },
        { token: "string.math.command", foreground: dark ? "d7ba7d" : "811f3f" },
        { token: "operator", foreground: dark ? "d4d4d4" : "000000", fontStyle: "bold" },
      ],
      colors: {},
    });
  }
  monaco.languages.registerCompletionItemProvider("latex", {
    triggerCharacters: ["\\", "{", ","],
    provideCompletionItems: (model, position) => {
      const line = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
      const argument = line.match(/\\(ref|eqref|pageref|autoref|cite[pt]?|begin|end)\{([^}]*)$/);
      if (argument) {
        const [, command, typed] = argument;
        const lastPart = typed.split(",").pop() ?? "";
        const argRange = new monaco.Range(position.lineNumber, position.column - lastPart.length, position.lineNumber, position.column);
        const source = command.startsWith("cite") ? completionData.citations : command === "begin" || command === "end" ? ENVIRONMENTS : completionData.labels;
        return {
          suggestions: source.map((label) => ({
            label,
            kind: monaco.languages.CompletionItemKind.Reference,
            insertText: label,
            range: argRange,
          })),
        };
      }
      const commandMatch = line.match(/\\([A-Za-z]*)$/);
      if (!commandMatch) return { suggestions: [] };
      const commandRange = new monaco.Range(position.lineNumber, position.column - commandMatch[1].length, position.lineNumber, position.column);
      const suggestions: monaco.languages.CompletionItem[] = COMMANDS.map((command) => ({
        label: `\\${command}`,
        filterText: command,
        kind: monaco.languages.CompletionItemKind.Function,
        insertText: command,
        range: commandRange,
      }));
      for (const environment of ENVIRONMENTS) {
        suggestions.push({
          label: `\\begin{${environment}}`,
          filterText: `begin${environment}`,
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: `begin{${environment}}\n\t$0\n\\end{${environment}}`,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range: commandRange,
        });
      }
      return { suggestions };
    },
  });
}

export { monaco };
