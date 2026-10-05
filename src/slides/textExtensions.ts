/**
 * Tiptap-Erweiterungen für Textfelder auf Folien: Absätze, Listen, Zeichen-
 * formate, Farbe/Größe/Schrift, Ausrichtung und Formeln im Text. Knoten- und
 * Markennamen entsprechen dem Dokumenteditor, damit der Rust-Exporter den Text
 * unverändert nach LaTeX übersetzt.
 */
import type { AnyExtension, JSONContent } from "@tiptap/core";
import { generateHTML } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import TextAlign from "@tiptap/extension-text-align";
import { BackgroundColor, Color, FontFamily, FontSize } from "@tiptap/extension-text-style";
import Superscript from "@tiptap/extension-superscript";
import Subscript from "@tiptap/extension-subscript";
import { DocumentTextStyle, InlineMath } from "../editor/schema";
import { renderMath } from "../latex/miniRender";

export function slideTextExtensions(inlineMath: AnyExtension = InlineMath): AnyExtension[] {
  return [
    StarterKit.configure({
      heading: false,
      codeBlock: false,
      blockquote: false,
      horizontalRule: false,
      code: false,
      link: false,
    }),
    DocumentTextStyle,
    Color,
    BackgroundColor,
    FontFamily,
    FontSize,
    TextAlign.configure({ types: ["paragraph"] }),
    Superscript,
    Subscript,
    inlineMath,
  ];
}

const staticExtensions = slideTextExtensions();
const cache = new Map<string, string>();

/** HTML eines Textfelds (für Anzeige, Miniaturen und Präsentation); Formeln mit KaTeX. */
export function slideTextHtml(content: JSONContent | null | undefined): string {
  if (!content) return "";
  const key = JSON.stringify(content);
  const cached = cache.get(key);
  if (cached !== undefined) return cached;
  let html: string;
  try {
    html = generateHTML(content, staticExtensions);
  } catch {
    html = "";
  }
  html = html.replace(/<span([^>]*)data-inline-math=""([^>]*)><\/span>/g, (_match, before: string, after: string) => {
    const attributes = `${before} ${after}`;
    const latex = /data-latex="([^"]*)"/.exec(attributes)?.[1] ?? "";
    const decoded = latex.replace(/&quot;/g, "\"").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
    return `<span class="inline-math">${renderMath(decoded, false)}</span>`;
  });
  if (cache.size > 500) cache.clear();
  cache.set(key, html);
  return html;
}
