/**
 * Darstellung von siunitx-Größen im Editor (\SI, \qty, \si, \unit, \num, \ang,
 * Bereiche) – als Text (Unicode) und als KaTeX-Mathematik. Nur Anzeige; der
 * LaTeX-Code bleibt unverändert im Knoten.
 */
import { documentLanguage } from "./languages";

const PREFIXES: Record<string, [string, string]> = {
  yocto: ["y", "y"], zepto: ["z", "z"], atto: ["a", "a"], femto: ["f", "f"], pico: ["p", "p"],
  nano: ["n", "n"], micro: ["µ", "\\mu "], milli: ["m", "m"], centi: ["c", "c"], deci: ["d", "d"],
  deca: ["da", "da"], deka: ["da", "da"], hecto: ["h", "h"], kilo: ["k", "k"], mega: ["M", "M"],
  giga: ["G", "G"], tera: ["T", "T"], peta: ["P", "P"], exa: ["E", "E"], zetta: ["Z", "Z"],
  yotta: ["Y", "Y"], kibi: ["Ki", "Ki"], mebi: ["Mi", "Mi"], gibi: ["Gi", "Gi"],
};

const UNITS: Record<string, [string, string]> = {
  ampere: ["A", "A"], candela: ["cd", "cd"], kelvin: ["K", "K"], kilogram: ["kg", "kg"], gram: ["g", "g"],
  metre: ["m", "m"], meter: ["m", "m"], mole: ["mol", "mol"], second: ["s", "s"], becquerel: ["Bq", "Bq"],
  degreeCelsius: ["°C", "{}^{\\circ}C"], celsius: ["°C", "{}^{\\circ}C"], coulomb: ["C", "C"], farad: ["F", "F"],
  gray: ["Gy", "Gy"], hertz: ["Hz", "Hz"], henry: ["H", "H"], joule: ["J", "J"], lumen: ["lm", "lm"],
  katal: ["kat", "kat"], lux: ["lx", "lx"], newton: ["N", "N"], ohm: ["Ω", "\\Omega"], pascal: ["Pa", "Pa"],
  radian: ["rad", "rad"], siemens: ["S", "S"], sievert: ["Sv", "Sv"], steradian: ["sr", "sr"], tesla: ["T", "T"],
  volt: ["V", "V"], watt: ["W", "W"], weber: ["Wb", "Wb"], day: ["d", "d"], hectare: ["ha", "ha"],
  hour: ["h", "h"], litre: ["L", "L"], liter: ["L", "L"], minute: ["min", "min"], tonne: ["t", "t"],
  arcminute: ["′", "'"], arcsecond: ["″", "''"], degree: ["°", "{}^{\\circ}"], astronomicalunit: ["au", "au"],
  bel: ["B", "B"], dalton: ["Da", "Da"], decibel: ["dB", "dB"], electronvolt: ["eV", "eV"], neper: ["Np", "Np"],
  percent: ["%", "\\%"], bar: ["bar", "bar"], angstrom: ["Å", "\\text{Å}"], barn: ["b", "b"], mmHg: ["mmHg", "mmHg"],
  knot: ["kn", "kn"], bit: ["bit", "bit"], byte: ["B", "B"], Omega: ["Ω", "\\Omega"],
};

type UnitPart = { text: string; math: string; power: number };

const SUPERSCRIPT: Record<string, string> = {
  "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹", "-": "⁻", "+": "⁺", ".": "·",
};

function superscript(value: string): string {
  return [...value].map((character) => SUPERSCRIPT[character] ?? character).join("");
}

/** Liest `{…}` ab `start`; liefert Inhalt und Ende. */
function group(source: string, start: number): [string, number] | null {
  if (source[start] !== "{") return null;
  let depth = 0;
  for (let index = start; index < source.length; index += 1) {
    if (source[index] === "\\") {
      index += 1;
      continue;
    }
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return [source.slice(start + 1, index), index + 1];
    }
  }
  return null;
}

/** Literale Einheiten wie `mV`, `k\Omega`, `m^2`, `\mu A`. */
function literalUnit(text: string): UnitPart {
  const plain = text
    .replace(/\\Omega\b\s*/g, "Ω")
    .replace(/\\(?:mu|micro)\b\s*/g, "µ")
    .replace(/\\degree\b|\^\\circ|\\circ/g, "°")
    .replace(/\\%/g, "%")
    .replace(/\^\{?(-?\d+)\}?/g, (_, power: string) => superscript(power))
    .replace(/[{}]/g, "")
    .replace(/\\[a-zA-Z]+\s*/g, "")
    .replace(/\s*\.\s*/g, " ");
  const math = text.includes("\\") || /\^/.test(text) ? text : `\\mathrm{${text}}`;
  return { text: plain, math, power: 1 };
}

/** Zerlegt eine siunitx-Einheit (`\kilo\ohm\per\second\squared`). */
function parseUnit(unit: string): UnitPart[] | null {
  const parts: UnitPart[] = [];
  let index = 0;
  let prefix: [string, string] | null = null;
  let per = false;
  let pendingPower = 1;
  const source = unit.trim();
  if (!source.includes("\\") || /\\(Omega|mu)\b/.test(source) && !/\\(per|kilo|mega|milli|micro|volt|ohm|ampere)\b/.test(source)) {
    return source ? [literalUnit(source)] : [];
  }
  while (index < source.length) {
    const character = source[index];
    if (/\s|~|\./.test(character)) {
      index += 1;
      continue;
    }
    if (character !== "\\") {
      // Literal innerhalb einer Makro-Einheit (z. B. \kilo g)
      const match = /^[A-Za-z%°]+/.exec(source.slice(index));
      if (!match) return null;
      parts.push({ ...literalUnit(match[0]), power: per ? -1 : 1 });
      index += match[0].length;
      continue;
    }
    const match = /^\\([A-Za-z]+)/.exec(source.slice(index));
    if (!match) return null;
    const name = match[1];
    index += match[0].length;
    if (name === "per") {
      per = true;
      continue;
    }
    if (name === "square") {
      pendingPower = 2;
      continue;
    }
    if (name === "cubic") {
      pendingPower = 3;
      continue;
    }
    const last = parts[parts.length - 1];
    if (name === "squared" && last) {
      last.power *= 2;
      continue;
    }
    if (name === "cubed" && last) {
      last.power *= 3;
      continue;
    }
    if (name === "tothe" && last) {
      const argument = group(source, index);
      if (!argument) return null;
      last.power *= Number(argument[0]) || 1;
      index = argument[1];
      continue;
    }
    if (PREFIXES[name]) {
      prefix = PREFIXES[name];
      continue;
    }
    const known = UNITS[name];
    if (!known) return null;
    parts.push({
      text: `${prefix?.[0] ?? ""}${known[0]}`,
      math: `\\mathrm{${prefix?.[1] ?? ""}${known[1]}}`,
      power: pendingPower * (per ? -1 : 1),
    });
    prefix = null;
    pendingPower = 1;
    per = false;
  }
  return parts;
}

/** Zahl wie siunitx: `1{,}5` → „1,5“, `1.5e3` → „1.5 × 10³“, `-` → „−“. */
export function formatNumber(value: string, decimalComma = false): string {
  let text = value.trim().replace(/\{,\}/g, ",").replace(/[{}]/g, "").replace(/\\pm/g, "±").replace(/\\,/g, " ");
  if (decimalComma) text = text.replace(/(\d)\.(\d)/g, "$1,$2");
  text = text.replace(/(\d)\s*[eE]([-+]?\d+)/, (_, mantissa: string, exponent: string) => `${mantissa} × 10${superscript(exponent.replace(/^\+/, ""))}`);
  return text.replace(/(^|\s|\()-/g, "$1−");
}

export function formatUnit(unit: string): string {
  const parts = parseUnit(unit);
  if (!parts) return literalUnit(unit).text;
  return parts.map((part) => (part.power === 1 ? part.text : `${part.text}${superscript(String(part.power))}`)).join(" ");
}

export type QuantityAttrs = { command?: string; value?: string; value2?: string; unit?: string };

/** Anzeige einer Größe als Text. */
export function formatQuantity(attrs: QuantityAttrs, language = "ngerman"): string {
  const command = attrs.command || "SI";
  const decimalComma = false;
  const value = formatNumber(attrs.value ?? "", decimalComma);
  const value2 = formatNumber(attrs.value2 ?? "", decimalComma);
  const unit = formatUnit(attrs.unit ?? "");
  const range = documentLanguage(language).range;
  switch (command) {
    case "si":
    case "unit":
      return unit;
    case "num":
      return value;
    case "ang": {
      const [degrees = "", minutes = "", seconds = ""] = (attrs.value ?? "").split(";");
      return [degrees && `${formatNumber(degrees)}°`, minutes && `${formatNumber(minutes)}′`, seconds && `${formatNumber(seconds)}″`].join("");
    }
    case "numrange":
      return `${value}${range}${value2}`;
    case "SIrange":
    case "qtyrange":
      return `${value} ${unit}${range}${value2} ${unit}`;
    default:
      return unit ? `${value} ${unit}` : value;
  }
}

/** Einheit als KaTeX-Mathematik. */
function unitMath(unit: string): string {
  const parts = parseUnit(unit);
  if (!parts) return literalUnit(unit).math;
  return parts.map((part) => (part.power === 1 ? part.math : `${part.math}^{${part.power}}`)).join("\\,");
}

function numberMath(value: string): string {
  return value
    .trim()
    .replace(/(\d)\s*[eE]([-+]?\d+)/, (_, mantissa: string, exponent: string) => `${mantissa}\\times 10^{${exponent.replace(/^\+/, "")}}`);
}

const MATH_COMMANDS = ["SIrange", "qtyrange", "numrange", "SI", "qty", "si", "unit", "num", "ang"];

/**
 * Ersetzt siunitx-Befehle in einer Formel durch KaTeX-taugliche Mathematik
 * (KaTeX kennt siunitx nicht).
 */
export function siunitxForKatex(latex: string): string {
  if (!/\\(SI|qty|si|unit|num|ang|SIrange|qtyrange|numrange)\b/.test(latex)) return latex;
  let output = "";
  let index = 0;
  while (index < latex.length) {
    const match = /^\\([A-Za-z]+)/.exec(latex.slice(index));
    if (!match || !MATH_COMMANDS.includes(match[1])) {
      if (match) {
        output += match[0];
        index += match[0].length;
      } else {
        output += latex[index];
        index += 1;
      }
      continue;
    }
    const command = match[1];
    let position = index + match[0].length;
    while (latex[position] === " ") position += 1;
    if (latex[position] === "[") {
      const close = latex.indexOf("]", position);
      if (close < 0) break;
      position = close + 1;
    }
    const count = ["si", "unit", "num", "ang"].includes(command) ? 1 : command === "numrange" ? 2 : ["SIrange", "qtyrange"].includes(command) ? 3 : 2;
    const args: string[] = [];
    for (let argument = 0; argument < count; argument += 1) {
      while (latex[position] === " ") position += 1;
      const read = group(latex, position);
      if (!read) break;
      args.push(read[0]);
      position = read[1];
    }
    if (args.length < count) {
      output += match[0];
      index += match[0].length;
      continue;
    }
    let replacement: string;
    switch (command) {
      case "si":
      case "unit":
        replacement = unitMath(args[0]);
        break;
      case "num":
        replacement = numberMath(args[0]);
        break;
      case "ang":
        replacement = `${numberMath(args[0].split(";")[0])}^{\\circ}`;
        break;
      case "numrange":
        replacement = `${numberMath(args[0])}\\text{–}${numberMath(args[1])}`;
        break;
      case "SIrange":
      case "qtyrange":
        replacement = `${numberMath(args[0])}\\,${unitMath(args[2])}\\text{–}${numberMath(args[1])}\\,${unitMath(args[2])}`;
        break;
      default:
        replacement = `${numberMath(args[0])}\\,${unitMath(args[1])}`;
    }
    output += `{${replacement}}`;
    index = position;
  }
  return output;
}

/** Häufige Einheiten für die Auswahl im Dialog. */
export const COMMON_UNITS: Array<{ latex: string; label: string }> = [
  { latex: "\\volt", label: "V" },
  { latex: "\\milli\\volt", label: "mV" },
  { latex: "\\kilo\\volt", label: "kV" },
  { latex: "\\ampere", label: "A" },
  { latex: "\\milli\\ampere", label: "mA" },
  { latex: "\\micro\\ampere", label: "µA" },
  { latex: "\\ohm", label: "Ω" },
  { latex: "\\kilo\\ohm", label: "kΩ" },
  { latex: "\\mega\\ohm", label: "MΩ" },
  { latex: "\\farad", label: "F" },
  { latex: "\\micro\\farad", label: "µF" },
  { latex: "\\nano\\farad", label: "nF" },
  { latex: "\\pico\\farad", label: "pF" },
  { latex: "\\henry", label: "H" },
  { latex: "\\milli\\henry", label: "mH" },
  { latex: "\\watt", label: "W" },
  { latex: "\\kilo\\watt", label: "kW" },
  { latex: "\\hertz", label: "Hz" },
  { latex: "\\kilo\\hertz", label: "kHz" },
  { latex: "\\mega\\hertz", label: "MHz" },
  { latex: "\\second", label: "s" },
  { latex: "\\milli\\second", label: "ms" },
  { latex: "\\micro\\second", label: "µs" },
  { latex: "\\metre", label: "m" },
  { latex: "\\milli\\metre", label: "mm" },
  { latex: "\\kilo\\metre", label: "km" },
  { latex: "\\metre\\per\\second", label: "m s⁻¹" },
  { latex: "\\kilogram", label: "kg" },
  { latex: "\\newton", label: "N" },
  { latex: "\\pascal", label: "Pa" },
  { latex: "\\bar", label: "bar" },
  { latex: "\\joule", label: "J" },
  { latex: "\\kelvin", label: "K" },
  { latex: "\\degreeCelsius", label: "°C" },
  { latex: "\\decibel", label: "dB" },
  { latex: "\\percent", label: "%" },
  { latex: "\\tesla", label: "T" },
  { latex: "\\litre\\per\\second", label: "L s⁻¹" },
];

/** Zahl im Sinne von siunitx (wie `is_siunitx_number` im Rust-Export) – Text wie „test“ ist keine Zahl. */
export function isSiunitxNumber(value: string): boolean {
  const text = value.replace(/\pm/g, "±").replace(/\times/g, "x").replace(/\{,\}/g, ",").trim();
  return /\d/.test(text) && /^[\d\s.,+\-±()eEdDxij]+$/.test(text);
}
