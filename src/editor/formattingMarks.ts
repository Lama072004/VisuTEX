/**
 * Formatierungszeichen anzeigen – wie „Alle anzeigen (¶)“ in Word: Absatzmarken (¶),
 * Leerzeichen (·), geschützte Leerzeichen (°) und Zeilenumbrüche (↵).
 *
 * Reine Anzeige über Dekorationen: Dokument, Undo-Verlauf und LaTeX-Export bleiben
 * unverändert. Die Zeichen nehmen keinen Platz ein (Breite 0 bzw. absolut positioniert),
 * damit sich Zeilen- und Seitenumbrüche beim Einschalten nicht verschieben.
 * Bei Änderungen werden nur die betroffenen Textblöcke neu dekoriert.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

type MarksState = { enabled: boolean; decorations: DecorationSet };

export const formattingMarksKey = new PluginKey<MarksState>("vtxFormattingMarks");

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    formattingMarks: {
      setFormattingMarks: (enabled: boolean) => ReturnType;
    };
  }
}

function markElement(symbol: string, kind: string): HTMLElement {
  const element = document.createElement("span");
  element.className = `vtx-mark vtx-mark-${kind}`;
  element.textContent = symbol;
  element.setAttribute("contenteditable", "false");
  element.setAttribute("aria-hidden", "true");
  return element;
}

/** Dekorationen für alle Knoten in [from, to). */
export function decorate(doc: PMNode, from: number, to: number): Decoration[] {
  const result: Decoration[] = [];
  doc.nodesBetween(from, to, (node, pos) => {
    if (node.isText) {
      const text = node.text ?? "";
      for (let index = 0; index < text.length; index += 1) {
        const character = text[index];
        if (character === " ") result.push(Decoration.inline(pos + index, pos + index + 1, { class: "vtx-mark-space" }));
        else if (character === " ") result.push(Decoration.inline(pos + index, pos + index + 1, { class: "vtx-mark-nbsp" }));
      }
      return false;
    }
    if (node.type.name === "hardBreak") {
      result.push(Decoration.widget(pos, () => markElement("↵", "break"), { side: -1, ignoreSelection: true, key: "vtx-mark-break" }));
      return false;
    }
    if (node.isTextblock && node.type.name !== "codeBlock") {
      // Absatzmarke am Ende des Inhalts (auch bei leeren Absätzen)
      result.push(
        Decoration.widget(pos + node.nodeSize - 1, () => markElement("¶", "paragraph"), {
          side: 1,
          ignoreSelection: true,
          key: "vtx-mark-paragraph",
        }),
      );
    }
    return true;
  });
  return result;
}

/** Textblöcke, die eine Transaktion berührt hat (Positionen im neuen Dokument). */
function touchedBlocks(tr: Transaction): Array<{ from: number; to: number }> {
  const doc = tr.doc;
  const blocks = new Map<number, number>();
  tr.mapping.maps.forEach((map, index) => {
    const rest = tr.mapping.slice(index + 1);
    map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
      const from = Math.max(0, Math.min(doc.content.size, rest.map(newStart, -1)));
      const to = Math.max(from, Math.min(doc.content.size, rest.map(newEnd, 1)));
      // umgebender Textblock (auch bei reinen Löschungen ohne Inhalt im Bereich)
      const $from = doc.resolve(from);
      for (let depth = $from.depth; depth > 0; depth -= 1) {
        const node = $from.node(depth);
        if (node.isTextblock) {
          blocks.set($from.before(depth), $from.after(depth));
          break;
        }
      }
      doc.nodesBetween(from, Math.min(doc.content.size, Math.max(to, from + 1)), (node, pos) => {
        if (node.isTextblock) {
          blocks.set(pos, pos + node.nodeSize);
          return false;
        }
        return true;
      });
    });
  });
  return [...blocks].map(([from, to]) => ({ from, to }));
}

export const FormattingMarks = Extension.create({
  name: "formattingMarks",

  addCommands() {
    return {
      setFormattingMarks:
        (enabled: boolean) =>
        ({ tr, dispatch }) => {
          if (dispatch) tr.setMeta(formattingMarksKey, { enabled }).setMeta("addToHistory", false);
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin<MarksState>({
        key: formattingMarksKey,
        state: {
          init: () => ({ enabled: false, decorations: DecorationSet.empty }),
          apply: (tr, value) => {
            const meta = tr.getMeta(formattingMarksKey) as { enabled: boolean } | undefined;
            const enabled = meta ? meta.enabled : value.enabled;
            if (!enabled) return value.enabled ? { enabled: false, decorations: DecorationSet.empty } : value;
            if (!value.enabled) return { enabled: true, decorations: DecorationSet.create(tr.doc, decorate(tr.doc, 0, tr.doc.content.size)) };
            if (!tr.docChanged) return value;
            let decorations = value.decorations.map(tr.mapping, tr.doc);
            for (const block of touchedBlocks(tr)) {
              decorations = decorations.remove(decorations.find(block.from, block.to));
              decorations = decorations.add(tr.doc, decorate(tr.doc, block.from, block.to));
            }
            return { enabled, decorations };
          },
        },
        props: {
          decorations: (state) => formattingMarksKey.getState(state)?.decorations,
        },
      }),
    ];
  },
});
