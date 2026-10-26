/**
 * Markierung wie bei Text – auch für Blöcke ohne eigenen Text: Skizzen/TikZ, Bilder, Formeln,
 * LaTeX-Blöcke, Verzeichnisse, Umbrüche, Zitate, Fußnoten … Der Browser hebt nur Text hervor;
 * alles, was vollständig in einer Markierung liegt, bekommt deshalb die Klasse
 * `vtx-in-selection` (CSS legt dieselbe Markierungsfarbe darüber). Tabellen werden als Ganzes
 * eingefärbt, sobald sie vollständig markiert sind.
 *
 * Reine Anzeige über Dekorationen: Dokument, Rückgängig-Verlauf und LaTeX-Export bleiben unverändert.
 * Ein einzeln angeklickter Block (NodeSelection) behält seinen Auswahlrahmen.
 */
import { Extension } from "@tiptap/core";
import { NodeSelection, Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState } from "@tiptap/pm/state";
import { CellSelection } from "@tiptap/pm/tables";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

/** Container, die vollständig markiert als Ganzes eingefärbt werden (sonst nur ihr Text). */
const WHOLE_CONTAINERS = new Set(["table", "environmentBlock", "frontmatterBlock", "includeBlock"]);

export const selectionHighlightKey = new PluginKey("vtxSelectionHighlight");

/** Dekorationen für alle vollständig markierten Elemente ohne eigenen Text. */
export function selectionDecorations(state: EditorState): DecorationSet {
  const { selection } = state;
  if (selection.empty || selection instanceof NodeSelection || selection instanceof CellSelection) return DecorationSet.empty;
  const decorations: Decoration[] = [];
  state.doc.nodesBetween(selection.from, selection.to, (node, pos) => {
    if (pos < selection.from || pos + node.nodeSize > selection.to) return true; // nur teilweise markiert
    if (node.isText || node.type.name === "hardBreak") return false;
    if (node.isAtom) {
      decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: "vtx-in-selection" }));
      return false;
    }
    if (WHOLE_CONTAINERS.has(node.type.name)) {
      // als Ganzes einfärben; der Text darin bekommt keine zweite (dunklere) Markierung
      decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: "vtx-in-selection vtx-whole" }));
      return false;
    }
    return true;
  });
  return DecorationSet.create(state.doc, decorations);
}

export const SelectionHighlight = Extension.create({
  name: "selectionHighlight",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: selectionHighlightKey,
        props: { decorations: selectionDecorations },
      }),
    ];
  },
});
