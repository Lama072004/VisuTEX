/**
 * Zeigt Tabellenbeschriftungen (Attribut `caption` der Tabelle) unter der
 * Tabelle an – nummeriert per CSS-Zähler wie im PDF („Tabelle 1.2: …“).
 * Bearbeitet wird die Beschriftung über den Kontext-Tab „Tabelle“.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { latexToHtml } from "../latex/miniRender";
import { renderContext } from "../state/runtime";

const key = new PluginKey<DecorationSet>("vtxTableCaptions");

function build(doc: PMNode): DecorationSet {
  const decorations: Decoration[] = [];
  doc.descendants((node, pos) => {
    // Tabellen stehen nie in Absätzen – nicht in Text hinabsteigen
    if (node.isTextblock) return false;
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
          // Vor einem eventuellen Seitenumbruch-Abstandshalter (side -1) einordnen. Schlüssel ohne
          // Position: Tippen davor verschiebt nur die Position, das Widget (mit CSS-Zähler) bleibt –
          // ein neues Widget ließe den Browser die Stile des ganzen Dokuments neu berechnen.
          { side: above ? 1 : -2, key: `caption-${above ? "a" : "b"}-${latex ? "l" : "t"}-${caption}`, ignoreSelection: true },
        ),
      );
    }
    return false;
  });
  return DecorationSet.create(doc, decorations);
}

/** Berührt eine Änderung eine Tabelle (vorher oder nachher)? Dann muss neu aufgebaut werden. */
function touchesTable(tr: Transaction, before: PMNode): boolean {
  const isTable = (node: PMNode) => node.type.name === "table";
  const check = (doc: PMNode, from: number, to: number) => {
    const $from = doc.resolve(Math.max(0, Math.min(from, doc.content.size)));
    for (let depth = $from.depth; depth > 0; depth--) if (isTable($from.node(depth))) return true;
    let found = false;
    doc.nodesBetween($from.pos, Math.min(Math.max(to, $from.pos), doc.content.size), (node) => {
      if (found || node.isTextblock) return false;
      if (isTable(node)) found = true;
      return !found;
    });
    return found;
  };
  return tr.steps.some((step, index) => {
    const docBefore = tr.docs[index] ?? before;
    const docAfter = tr.docs[index + 1] ?? tr.doc;
    // Attributänderungen (AttrStep) verschieben nichts und haben keinen Bereich – Position prüfen
    const pos = (step as unknown as { pos?: number }).pos;
    if (typeof pos === "number") {
      const node = docBefore.nodeAt(pos);
      return (node !== null && isTable(node)) || check(docBefore, pos, pos);
    }
    let touched = false;
    step.getMap().forEach((oldStart, oldEnd, newStart, newEnd) => {
      touched ||= check(docBefore, oldStart, oldEnd) || check(docAfter, newStart, newEnd);
    });
    return touched;
  });
}

export const TableCaptions = Extension.create({
  name: "tableCaptions",
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key,
        state: {
          init: (_, state) => build(state.doc),
          // Nur neu aufbauen, wenn eine Änderung eine Tabelle berührt; sonst Positionen fortschreiben.
          apply: (tr, value, oldState) => (!tr.docChanged ? value : touchesTable(tr, oldState.doc) ? build(tr.doc) : value.map(tr.mapping, tr.doc)),
        },
        props: {
          decorations: (state) => key.getState(state),
        },
      }),
    ];
  },
});
