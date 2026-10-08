/**
 * Absätze, die nur aus unsichtbaren Layout-Befehlen bestehen (z. B. `\setcounter{page}{1}
 * \begingroup \setstretch{0.95}` als Inline-Code), ergeben im PDF keine Zeile. Im Editor
 * nehmen sie deshalb keine Höhe ein; die Befehle stehen als Markierung am linken Rand.
 * Bei eingeschalteten Formatierungszeichen (¶) erscheinen sie wieder als normale Zeile.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { formattingMarksKey } from "./formattingMarks";
import { isLayoutOnly } from "./layoutCommands";

/** Nur Layout-Befehle (als Inline-Code) und Leerraum. */
export function isLayoutOnlyParagraph(node: PMNode): boolean {
  if (node.type.name !== "paragraph" || node.childCount === 0) return false;
  let commands = 0;
  let other = false;
  node.forEach((child) => {
    if (child.type.name === "rawLatexInline" && isLayoutOnly(String(child.attrs.latex ?? ""))) commands += 1;
    else if (!(child.isText && !(child.text ?? "").trim())) other = true;
  });
  return commands > 0 && !other;
}

function decorate(doc: PMNode): DecorationSet {
  const decorations: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (node.isTextblock) {
      if (isLayoutOnlyParagraph(node)) decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: "layout-only-paragraph" }));
      return false;
    }
    return true;
  });
  return DecorationSet.create(doc, decorations);
}

const key = new PluginKey<DecorationSet>("vtxLayoutParagraphs");

export const LayoutParagraphs = Extension.create({
  name: "layoutParagraphs",

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key,
        state: {
          init: (_, state) => decorate(state.doc),
          apply: (tr, value) => (tr.docChanged ? decorate(tr.doc) : value),
        },
        props: {
          decorations: (state) => key.getState(state),
          // Formatierungszeichen an → Layout-Absätze als normale Zeilen zeigen
          attributes: (state): Record<string, string> => (formattingMarksKey.getState(state)?.enabled ? { class: "vtx-show-marks" } : {}),
        },
      }),
    ];
  },
});
