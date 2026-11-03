/**
 * Kurzes Aufleuchten eines Blocks (Doppelklick ins PDF springt dorthin). Als Dekoration, damit
 * ProseMirror die Hervorhebung beim Neuzeichnen nicht entfernt; Dokument und Verlauf bleiben
 * unberührt.
 */
import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const syncFlashKey = new PluginKey<DecorationSet>("vtxSyncFlash");

export const SyncFlash = Extension.create({
  name: "syncFlash",
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: syncFlashKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, set) {
            const meta = tr.getMeta(syncFlashKey) as { from: number; to: number } | null | undefined;
            if (meta === null) return DecorationSet.empty;
            if (meta) return DecorationSet.create(tr.doc, [Decoration.node(meta.from, meta.to, { class: "sync-flash" })]);
            return set.map(tr.mapping, tr.doc);
          },
        },
        props: {
          decorations: (state) => syncFlashKey.getState(state),
        },
      }),
    ];
  },
});

/** Block [from, to) kurz hervorheben. */
export function flashBlock(view: EditorView, from: number, to: number, duration = 1800) {
  view.dispatch(view.state.tr.setMeta(syncFlashKey, { from, to }).setMeta("addToHistory", false));
  window.setTimeout(() => {
    if (!view.isDestroyed) view.dispatch(view.state.tr.setMeta(syncFlashKey, null).setMeta("addToHistory", false));
  }, duration);
}
