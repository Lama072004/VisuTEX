/**
 * Formel-Galerien (Registerkarte *Formel*, wie in Word/OneNote): Symbole und Strukturen.
 * Reine Daten, ohne DOM – in Node testbar (`tests/math-catalog.test.mjs`, prüft jede Vorlage
 * mit KaTeX).
 *
 * Vorlagen verwenden die Platzhalter von MathLive: `#?` = leeres Feld, `#0` = aktuelle
 * Auswahl (ohne Auswahl ein leeres Feld). Alle Befehle stammen aus amsmath/amssymb, die der LaTeX-Export
 * immer lädt – exportierte Formeln kompilieren so auch mit pdfLaTeX, XeLaTeX und LuaLaTeX.
 */

export type MathSymbol = { latex: string; title?: string };
export type SymbolCategory = { id: string; label: string; symbols: MathSymbol[] };
export type MathTemplate = { latex: string; title: string };
export type StructureGroup = { id: string; label: string; icon: string; sections: Array<{ label: string; items: MathTemplate[] }> };

const symbols = (list: string, titles: Record<string, string> = {}): MathSymbol[] =>
  list
    .trim()
    .split(/\s+/)
    .map((latex) => ({ latex, title: titles[latex] }));

export const SYMBOL_CATEGORIES: SymbolCategory[] = [
  {
    id: "basic",
    label: "Grundlegende Mathematik",
    symbols: symbols(
      String.raw`\pm \mp \times \div \cdot = \neq < > \le \ge \approx \sim \equiv \propto \infty \partial \nabla
      \forall \exists \in \notin \subset \subseteq \cup \cap \emptyset \angle \perp \parallel ^{\circ} \% \therefore
      \because \ldots \cdots \vdots \ddots \prime \hbar \Re \Im \aleph \ell`,
      { "^{\\circ}": "Grad" },
    ),
  },
  {
    id: "greek",
    label: "Griechische Buchstaben (klein)",
    symbols: symbols(
      String.raw`\alpha \beta \gamma \delta \epsilon \varepsilon \zeta \eta \theta \vartheta \iota \kappa \lambda \mu
      \nu \xi o \pi \varpi \rho \varrho \sigma \varsigma \tau \upsilon \phi \varphi \chi \psi \omega`,
    ),
  },
  {
    id: "greek-upper",
    label: "Griechische Buchstaben (groß)",
    symbols: symbols(String.raw`\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega A B E Z H I K M N O P T X`),
  },
  {
    id: "letters",
    label: "Buchstabenähnliche Symbole",
    symbols: symbols(
      String.raw`\mathbb{N} \mathbb{Z} \mathbb{Q} \mathbb{R} \mathbb{C} \mathcal{L} \mathcal{F} \mathcal{O} \hbar \ell \wp \Re \Im
      \aleph \beth \eth \mho \complement \imath \jmath`,
    ),
  },
  {
    id: "operators",
    label: "Operatoren",
    symbols: symbols(
      String.raw`+ - \pm \mp \times \div \cdot \ast \star \circ \bullet \oplus \ominus \otimes \oslash \odot \wedge \vee
      \neg \setminus \cap \cup \sqcap \sqcup \uplus \amalg \dagger \ddagger \wr \diamond`,
    ),
  },
  {
    id: "relations",
    label: "Relationen",
    symbols: symbols(
      String.raw`= \neq < > \le \ge \leqslant \geqslant \ll \gg \approx \cong \sim \simeq \equiv \propto \doteq
      \triangleq \prec \succ \preceq \succeq \subset \supset \subseteq \supseteq \in \ni \mid \parallel \perp \models
      \vdash \dashv`,
    ),
  },
  {
    id: "negated",
    label: "Negierte Relationen",
    symbols: symbols(
      String.raw`\neq \not\equiv \nless \ngtr \nleq \ngeq \nsim \ncong \notin \nmid \nparallel \nsubseteq
      \nsupseteq \nprec \nsucc \nexists`,
    ),
  },
  {
    id: "arrows",
    label: "Pfeile",
    symbols: symbols(
      String.raw`\to \gets \leftarrow \rightarrow \leftrightarrow \Leftarrow \Rightarrow \Leftrightarrow \uparrow
      \downarrow \updownarrow \Uparrow \Downarrow \longleftarrow \longrightarrow \Longleftarrow \Longrightarrow
      \Longleftrightarrow \mapsto \longmapsto \hookrightarrow \hookleftarrow \rightleftharpoons \nearrow \searrow
      \swarrow \nwarrow \implies \iff`,
    ),
  },
  {
    id: "logic",
    label: "Mengen und Logik",
    symbols: symbols(
      String.raw`\forall \exists \nexists \neg \land \lor \Rightarrow \Leftrightarrow \top \bot \in \notin \subset
      \subseteq \supset \supseteq \cup \cap \setminus \emptyset \varnothing \complement`,
    ),
  },
  {
    id: "geometry",
    label: "Geometrie",
    symbols: symbols(String.raw`\angle \measuredangle \triangle \square \perp \parallel \cong \sim ^{\circ} \overline{AB} \overrightarrow{AB} \widehat{ABC}`),
  },
];

const t = (latex: string, title: string): MathTemplate => ({ latex, title });

export const STRUCTURES: StructureGroup[] = [
  {
    id: "fraction",
    label: "Bruch",
    icon: String.raw`\frac{x}{y}`,
    sections: [
      {
        label: "Bruch",
        items: [
          t(String.raw`\frac{#0}{#?}`, "Bruch (gestapelt)"),
          t(String.raw`#0/#?`, "Bruch (schräg)"),
          t(String.raw`\tfrac{#0}{#?}`, "Kleiner Bruch"),
          t(String.raw`\dfrac{#0}{#?}`, "Großer Bruch"),
          t(String.raw`\binom{#0}{#?}`, "Binomialkoeffizient"),
        ],
      },
      {
        label: "Häufig verwendete Brüche",
        items: [
          t(String.raw`\frac{\mathrm{d}y}{\mathrm{d}x}`, "Ableitung dy/dx"),
          t(String.raw`\frac{\Delta y}{\Delta x}`, "Differenzenquotient"),
          t(String.raw`\frac{\partial #?}{\partial #?}`, "Partielle Ableitung"),
          t(String.raw`\frac{\pi}{2}`, "Pi halbe"),
          t(String.raw`\frac{1}{2}`, "Ein Halb"),
        ],
      },
    ],
  },
  {
    id: "script",
    label: "Skript",
    icon: String.raw`e^{x}`,
    sections: [
      {
        label: "Hoch- und Tiefgestellt",
        items: [
          t(String.raw`#0^{#?}`, "Hochgestellt"),
          t(String.raw`#0_{#?}`, "Tiefgestellt"),
          t(String.raw`#0_{#?}^{#?}`, "Tief- und hochgestellt"),
          t(String.raw`{}_{#?}^{#?}#0`, "Links tief- und hochgestellt"),
        ],
      },
      {
        label: "Häufig verwendet",
        items: [
          t(String.raw`x^{2}`, "x Quadrat"),
          t(String.raw`x_{n}`, "x Index n"),
          t(String.raw`e^{-i\omega t}`, "e hoch -iωt"),
          t(String.raw`x_{1}, \ldots, x_{n}`, "x1 bis xn"),
        ],
      },
    ],
  },
  {
    id: "radical",
    label: "Wurzel",
    icon: String.raw`\sqrt[n]{x}`,
    sections: [
      {
        label: "Wurzeln",
        items: [
          t(String.raw`\sqrt{#0}`, "Quadratwurzel"),
          t(String.raw`\sqrt[#?]{#0}`, "n-te Wurzel"),
          t(String.raw`\sqrt[3]{#0}`, "Kubikwurzel"),
        ],
      },
      {
        label: "Häufig verwendet",
        items: [
          t(String.raw`\sqrt{a^{2}+b^{2}}`, "Satz des Pythagoras"),
          t(String.raw`\frac{-b\pm\sqrt{b^{2}-4ac}}{2a}`, "Lösungsformel"),
        ],
      },
    ],
  },
  {
    id: "integral",
    label: "Integral",
    icon: String.raw`\int_{a}^{b}`,
    sections: [
      {
        label: "Integrale",
        items: [
          t(String.raw`\int #0\,\mathrm{d}#?`, "Integral"),
          t(String.raw`\int_{#?}^{#?} #0\,\mathrm{d}#?`, "Bestimmtes Integral"),
          t(String.raw`\iint_{#?} #0\,\mathrm{d}A`, "Doppelintegral"),
          t(String.raw`\iiint_{#?} #0\,\mathrm{d}V`, "Dreifachintegral"),
          t(String.raw`\oint_{#?} #0\,\mathrm{d}#?`, "Kurvenintegral"),
        ],
      },
      {
        label: "Häufig verwendet",
        items: [
          t(String.raw`\int_{-\infty}^{\infty} #0\,\mathrm{d}x`, "Integral über ℝ"),
          t(String.raw`\int_{0}^{t} #0\,\mathrm{d}\tau`, "Integral von 0 bis t"),
        ],
      },
    ],
  },
  {
    id: "large-operator",
    label: "Großer Operator",
    icon: String.raw`\sum_{i=1}^{n}`,
    sections: [
      {
        label: "Summen und Produkte",
        items: [
          t(String.raw`\sum_{#?}^{#?} #0`, "Summe mit Grenzen"),
          t(String.raw`\sum_{#?} #0`, "Summe mit Index"),
          t(String.raw`\sum_{i=1}^{n} #0`, "Summe i = 1 bis n"),
          t(String.raw`\prod_{#?}^{#?} #0`, "Produkt"),
          t(String.raw`\coprod_{#?}^{#?} #0`, "Koprodukt"),
        ],
      },
      {
        label: "Vereinigung und Schnitt",
        items: [
          t(String.raw`\bigcup_{#?}^{#?} #0`, "Vereinigung"),
          t(String.raw`\bigcap_{#?}^{#?} #0`, "Schnittmenge"),
          t(String.raw`\bigvee_{#?} #0`, "Logisches Oder"),
          t(String.raw`\bigwedge_{#?} #0`, "Logisches Und"),
        ],
      },
    ],
  },
  {
    id: "bracket",
    label: "Klammer",
    icon: String.raw`\left\{x\right\}`,
    sections: [
      {
        label: "Klammern",
        items: [
          t(String.raw`\left(#0\right)`, "Runde Klammern"),
          t(String.raw`\left[#0\right]`, "Eckige Klammern"),
          t(String.raw`\left\{#0\right\}`, "Geschweifte Klammern"),
          t(String.raw`\left|#0\right|`, "Betrag"),
          t(String.raw`\left\|#0\right\|`, "Norm"),
          t(String.raw`\left\langle #0\right\rangle`, "Spitze Klammern"),
          t(String.raw`\left\lfloor #0\right\rfloor`, "Abrunden"),
          t(String.raw`\left\lceil #0\right\rceil`, "Aufrunden"),
        ],
      },
      {
        label: "Fallunterscheidung und Klammern über/unter",
        items: [
          t(String.raw`\begin{cases}#0 & #?\\ #? & #?\end{cases}`, "Fallunterscheidung"),
          t(String.raw`\underbrace{#0}_{#?}`, "Klammer unten"),
          t(String.raw`\overbrace{#0}^{#?}`, "Klammer oben"),
        ],
      },
    ],
  },
  {
    id: "function",
    label: "Funktion",
    icon: String.raw`\sin\theta`,
    sections: [
      {
        label: "Trigonometrische Funktionen",
        items: [
          t(String.raw`\sin #0`, "Sinus"),
          t(String.raw`\cos #0`, "Kosinus"),
          t(String.raw`\tan #0`, "Tangens"),
          t(String.raw`\cot #0`, "Kotangens"),
          t(String.raw`\arcsin #0`, "Arkussinus"),
          t(String.raw`\arccos #0`, "Arkuskosinus"),
          t(String.raw`\arctan #0`, "Arkustangens"),
          t(String.raw`\sin^{-1} #0`, "Inverse Sinusfunktion"),
        ],
      },
      {
        label: "Hyperbolische und weitere Funktionen",
        items: [
          t(String.raw`\sinh #0`, "Sinus hyperbolicus"),
          t(String.raw`\cosh #0`, "Kosinus hyperbolicus"),
          t(String.raw`\tanh #0`, "Tangens hyperbolicus"),
          t(String.raw`\exp #0`, "Exponentialfunktion"),
          t(String.raw`\det #0`, "Determinante"),
          t(String.raw`\operatorname{#?}#0`, "Eigener Funktionsname"),
        ],
      },
    ],
  },
  {
    id: "accent",
    label: "Akzent",
    icon: String.raw`\vec{a}`,
    sections: [
      {
        label: "Akzente",
        items: [
          t(String.raw`\dot{#0}`, "Punkt"),
          t(String.raw`\ddot{#0}`, "Zwei Punkte"),
          t(String.raw`\hat{#0}`, "Dach"),
          t(String.raw`\check{#0}`, "Hatschek"),
          t(String.raw`\tilde{#0}`, "Tilde"),
          t(String.raw`\bar{#0}`, "Querstrich"),
          t(String.raw`\vec{#0}`, "Vektorpfeil"),
          t(String.raw`\acute{#0}`, "Akut"),
          t(String.raw`\grave{#0}`, "Gravis"),
          t(String.raw`\breve{#0}`, "Brevis"),
        ],
      },
      {
        label: "Breite Akzente und Rahmen",
        items: [
          t(String.raw`\overline{#0}`, "Überstrich"),
          t(String.raw`\underline{#0}`, "Unterstrich"),
          t(String.raw`\overrightarrow{#0}`, "Pfeil nach rechts darüber"),
          t(String.raw`\overleftarrow{#0}`, "Pfeil nach links darüber"),
          t(String.raw`\widehat{#0}`, "Breites Dach"),
          t(String.raw`\widetilde{#0}`, "Breite Tilde"),
          t(String.raw`\boxed{#0}`, "Umrahmt"),
        ],
      },
    ],
  },
  {
    id: "limit",
    label: "Grenzwert und Log",
    icon: String.raw`\lim_{n\to\infty}`,
    sections: [
      {
        label: "Funktionen",
        items: [
          t(String.raw`\lim_{#?\to #?} #0`, "Grenzwert"),
          t(String.raw`\lim_{n\to\infty} #0`, "Grenzwert n gegen unendlich"),
          t(String.raw`\lim_{x\to 0} #0`, "Grenzwert x gegen 0"),
          t(String.raw`\max_{#?} #0`, "Maximum"),
          t(String.raw`\min_{#?} #0`, "Minimum"),
          t(String.raw`\sup_{#?} #0`, "Supremum"),
          t(String.raw`\inf_{#?} #0`, "Infimum"),
        ],
      },
      {
        label: "Logarithmen",
        items: [
          t(String.raw`\log_{#?} #0`, "Logarithmus zur Basis"),
          t(String.raw`\log #0`, "Logarithmus"),
          t(String.raw`\ln #0`, "Natürlicher Logarithmus"),
          t(String.raw`\lg #0`, "Dekadischer Logarithmus"),
        ],
      },
    ],
  },
  {
    id: "operator",
    label: "Operator",
    icon: String.raw`\overset{!}{=}`,
    sections: [
      {
        label: "Zusammengesetzte Operatoren",
        items: [
          t(String.raw`\stackrel{\mathrm{def}}{=}`, "Definitionsgemäß gleich"),
          t(String.raw`\mathrel{:=}`, "Ist definiert als"),
          t(String.raw`\overset{!}{=}`, "Soll gleich sein"),
          t(String.raw`\overset{#?}{#0}`, "Etwas darüber"),
          t(String.raw`\underset{#?}{#0}`, "Etwas darunter"),
          t(String.raw`\xrightarrow{#?}`, "Pfeil mit Beschriftung"),
          t(String.raw`\xleftarrow{#?}`, "Pfeil nach links mit Beschriftung"),
        ],
      },
    ],
  },
  {
    id: "matrix",
    label: "Matrix",
    icon: String.raw`\begin{pmatrix}a&b\\c&d\end{pmatrix}`,
    sections: [
      {
        label: "Leere Matrizen",
        items: [
          t(String.raw`\begin{pmatrix}#0 & #?\\ #? & #?\end{pmatrix}`, "2×2-Matrix (runde Klammern)"),
          t(String.raw`\begin{bmatrix}#0 & #?\\ #? & #?\end{bmatrix}`, "2×2-Matrix (eckige Klammern)"),
          t(String.raw`\begin{vmatrix}#0 & #?\\ #? & #?\end{vmatrix}`, "Determinante 2×2"),
          t(String.raw`\begin{pmatrix}#0 & #? & #?\\ #? & #? & #?\\ #? & #? & #?\end{pmatrix}`, "3×3-Matrix"),
          t(String.raw`\begin{pmatrix}#0\\ #?\end{pmatrix}`, "Spaltenvektor (2)"),
          t(String.raw`\begin{pmatrix}#0\\ #?\\ #?\end{pmatrix}`, "Spaltenvektor (3)"),
          t(String.raw`\begin{pmatrix}#0 & #?\end{pmatrix}`, "Zeilenvektor"),
        ],
      },
      {
        label: "Häufig verwendet",
        items: [
          t(String.raw`\begin{pmatrix}1 & 0\\ 0 & 1\end{pmatrix}`, "Einheitsmatrix 2×2"),
          t(String.raw`\begin{pmatrix}1 & 0 & 0\\ 0 & 1 & 0\\ 0 & 0 & 1\end{pmatrix}`, "Einheitsmatrix 3×3"),
          t(String.raw`\begin{pmatrix}a_{11} & \cdots & a_{1n}\\ \vdots & \ddots & \vdots\\ a_{m1} & \cdots & a_{mn}\end{pmatrix}`, "m×n-Matrix"),
        ],
      },
    ],
  },
];

/** Vorschau-LaTeX (KaTeX) einer Vorlage: Platzhalter als graues Kästchen. */
export function previewLatex(template: string): string {
  return template.replace(/#[?@0]/g, String.raw`{\color{#8a8f98}\square}`);
}

/** Vorlage als einfacher LaTeX-Code (für die Code-Bearbeitung): Platzhalter werden leer,
 * `#0` wird zur Auswahl. Liefert den Text und die Position des ersten Platzhalters. */
export function templateToCode(template: string, selection = ""): { text: string; caret: number } {
  let caret = -1;
  let text = "";
  const parts = template.split(/(#[?@0])/);
  for (const part of parts) {
    if (part === "#?" || part === "#0" || part === "#0") {
      if (caret < 0 && (part === "#?" || !selection)) caret = text.length;
      if (part !== "#?") text += selection;
    } else {
      text += part;
    }
  }
  return { text, caret: caret < 0 ? text.length : caret };
}

/** Häufige Formeln (wie *Einfügen → Formel* in Word): fertige Formeln zum Anpassen. */
export const COMMON_FORMULAS: MathTemplate[] = [
  t(String.raw`a^{2}+b^{2}=c^{2}`, "Satz des Pythagoras"),
  t(String.raw`x=\frac{-b\pm\sqrt{b^{2}-4ac}}{2a}`, "Quadratische Gleichung"),
  t(String.raw`A=\pi r^{2}`, "Kreisfläche"),
  t(String.raw`(x+a)^{n}=\sum_{k=0}^{n}\binom{n}{k}x^{k}a^{n-k}`, "Binomischer Lehrsatz"),
  t(String.raw`e^{i\pi}+1=0`, "Eulersche Identität"),
  t(String.raw`e^{x}=1+\frac{x}{1!}+\frac{x^{2}}{2!}+\frac{x^{3}}{3!}+\cdots`, "Taylor-Reihe der e-Funktion"),
  t(String.raw`f(x)=a_{0}+\sum_{n=1}^{\infty}\left(a_{n}\cos\frac{n\pi x}{L}+b_{n}\sin\frac{n\pi x}{L}\right)`, "Fourier-Reihe"),
  t(String.raw`\sin\alpha\pm\sin\beta=2\sin\frac{1}{2}(\alpha\pm\beta)\cos\frac{1}{2}(\alpha\mp\beta)`, "Trigonometrische Identität"),
  t(String.raw`U=R\cdot I`, "Ohmsches Gesetz"),
  t(String.raw`P=U\cdot I\cdot\cos\varphi`, "Wirkleistung"),
  t(String.raw`F=m\cdot a`, "Newtonsches Grundgesetz"),
  t(String.raw`E=m c^{2}`, "Masse-Energie-Äquivalenz"),
];
