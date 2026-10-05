/**
 * Zeigt Tabellenbeschriftungen (Attribut `caption` der Tabelle) unter der
 * Tabelle an – nummeriert per CSS-Zähler wie im PDF („Tabelle 1.2: …“).
 * Bearbeitet wird die Beschriftung über den Kontext-Tab „Tabelle“.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { latexToHtml } from "../latex/miniRender";
import { renderContext } from "../state/runtime";

const key = new PluginKey<DecorationSet>("vtxTableCaptions");

function build(doc: PMNode): DecorationSet {
  const decorations: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== "table") return true;
    const caption = String(node.attrs.caption ?? "").trim();
    if (caption) {
      const latex = node.attrs.captionLatex === true;
      const above = node.attrs.captionAbove === true;
      decorations.push(
        Decoration.widget(
          above ? pos : pos + node.nodeSize,
          () => {
            const element = document.createElement("div");
            element.className = above ? "table-caption is-above" : "table-caption";
            element.setAttribute("contenteditable", "false");
            // Übernommene LaTeX-Beschriftungen (Formeln, Einheiten …) gerendert anzeigen.
            if (latex) element.innerHTML = latexToHtml(caption, renderContext());
            else element.textContent = caption;
            return element;
          },
          // Vor einem eventuellen Seitenumbruch-Abstandshalter (side -1) einordnen.
          { side: above ? 1 : -2, key: `caption-${pos}-${above ? "a" : "b"}-${caption}`, ignoreSelection: true },
        ),
      );
    }
    return false;
  });
  return DecorationSet.create(doc, decorations);
}

export const TableCaptions = Extension.create({
  name: "tableCaptions",
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key,
        state: {
          init: (_, state) => build(state.doc),
          apply: (tr, value) => (tr.docChanged ? build(tr.doc) : value),
        },
        props: {
          decorations: (state) => key.getState(state),
        },
      }),
    ];
  },
});
