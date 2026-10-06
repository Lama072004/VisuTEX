/**
 * Randkommentare nicht übereinanderlegen: Sprechblasen, die eine vorige überdecken würden,
 * rücken nach unten (wie die Kommentarspalte in Word). Läuft nach jeder Änderung und
 * Größenänderung; die Verschiebung wirkt nur in der Anzeige (`transform`).
 */
import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

const GAP = 6;

function layoutBalloons(view: EditorView) {
  const balloons = [...view.dom.querySelectorAll<HTMLElement>(".latex-comment-balloon")];
  if (balloons.length === 0) return;
  const root = view.dom as HTMLElement;
  // Zoom der Seite (transform: scale) herausrechnen
  const scale = root.offsetHeight > 0 ? root.getBoundingClientRect().height / root.offsetHeight : 1;
  balloons.forEach((balloon) => (balloon.style.transform = ""));
  let bottom = -Infinity;
  for (const balloon of balloons) {
    const rect = balloon.getBoundingClientRect();
    const top = rect.top / (scale || 1);
    const height = rect.height / (scale || 1);
    const shift = top < bottom + GAP ? bottom + GAP - top : 0;
    if (shift > 0) balloon.style.transform = `translateY(${shift}px)`;
    bottom = top + shift + height;
  }
}

export const CommentLayout = Extension.create({
  name: "commentLayout",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("vtxCommentLayout"),
        view: (view) => {
          let frame = 0;
          const schedule = () => {
            if (frame) return;
            frame = window.requestAnimationFrame(() => {
              frame = 0;
              layoutBalloons(view);
            });
          };
          // React-NodeViews rendern teils nach dem ProseMirror-Update → auch auf DOM-Änderungen reagieren
          const mutations = new MutationObserver(schedule);
          mutations.observe(view.dom, { childList: true, subtree: true, characterData: true });
          const resize = new ResizeObserver(schedule);
          resize.observe(view.dom);
          schedule();
          return {
            update: schedule,
            destroy: () => {
              if (frame) window.cancelAnimationFrame(frame);
              mutations.disconnect();
              resize.disconnect();
            },
          };
        },
      }),
    ];
  },
});
