/**
 * LaTeX-Kommentare (`% …`) als Randkommentare wie in Word: aus reinen Kommentarblöcken
 * wird der lesbare Text gewonnen (ohne `%` und ohne Trennlinien wie `% =====`), und
 * bearbeiteter Text wird wieder zu Kommentarzeilen. Trennlinien am Anfang/Ende bleiben
 * beim Bearbeiten erhalten, unveränderter Text lässt den Originalcode unangetastet.
 */

type CommentLine = { raw: string; text: string; separator: boolean };

function parseLine(line: string): CommentLine {
  const trimmed = line.trim();
  const text = trimmed.replace(/^%+ ?/, "").replace(/\s+$/, "");
  // Trennlinien: nur Zeichen wie = - * # ~ _ . + (mindestens drei)
  const separator = /^[=\-*#~_.+\s]{3,}$/.test(text);
  return { raw: line, text, separator };
}

function lines(raw: string): CommentLine[] {
  return raw.split("\n").filter((line) => line.trim() !== "").map(parseLine);
}

/** Nur Trennlinien (z. B. `% ====`) – im Editor nicht als Kommentar anzeigen. */
export function isDividerOnly(raw: string): boolean {
  const parsed = lines(raw);
  return parsed.length > 0 && parsed.every((line) => line.separator);
}

/** Lesbarer Kommentartext ohne `%`, ohne Trennlinien. */
export function commentText(raw: string): string {
  const parsed = lines(raw).filter((line) => !line.separator);
  return parsed.map((line) => line.text).join("\n").trim();
}

/** Ersetzt den Text eines Kommentarblocks; Trennlinien am Anfang und Ende bleiben stehen. */
export function replaceCommentText(raw: string, text: string): string {
  const parsed = lines(raw);
  let start = 0;
  while (start < parsed.length && parsed[start].separator) start += 1;
  let end = parsed.length;
  while (end > start && parsed[end - 1].separator) end -= 1;
  const indent = (parsed[start] ?? parsed[0])?.raw.match(/^\s*/)?.[0] ?? "";
  const body = text
    .split("\n")
    .map((line) => line.replace(/\s+$/, ""))
    .map((line) => (line ? `${indent}% ${line}` : `${indent}%`));
  return [...parsed.slice(0, start).map((line) => line.raw), ...body, ...parsed.slice(end).map((line) => line.raw)].join("\n");
}

/** Neuer Kommentarblock aus Klartext. */
export function newComment(text: string): string {
  return replaceCommentText("", text);
}

/** Nur Kommentare (mindestens eine `%`-Zeile, sonst Leerraum) – keine Vorschau nötig. */
export function isCommentOnly(latex: string): boolean {
  const lines = latex.split("\n").map((line) => line.trim()).filter(Boolean);
  return lines.length > 0 && lines.every((line) => line.startsWith("%"));
}
