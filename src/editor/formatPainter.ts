/**
 * „Format übertragen“: übernimmt Zeichen- und Absatzformat der aktuellen
 * Position und wendet es auf die nächste Markierung an – erst wenn die Maus
 * losgelassen bzw. eine Tastatur-Markierung abgeschlossen wurde (nicht schon
 * beim ersten Klick). Doppelklick auf die Schaltfläche = mehrfach anwenden.
 */
import type { Editor } from "@tiptap/core";
import type { Mark } from "@tiptap/pm/model";

type Captured = { marks: readonly Mark[]; blockAttrs: Record<string, unknown> };

const BLOCK_ATTRS = ["textAlign", "lineHeight", "indent", "border", "paragraphShading"];

export class FormatPainter {
  private captured: Captured | null = null;
  private sticky = false;
  private cleanup: (() => void) | null = null;
  private listeners = new Set<(active: boolean) => void>();

  constructor(private editor: Editor) {}

  get active() {
    return this.captured !== null;
  }

  onChange(listener: (active: boolean) => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private emit() {
    this.listeners.forEach((listener) => listener(this.active));
  }

  start(sticky: boolean) {
    if (this.active && !sticky) {
      this.stop();
      return;
    }
    const { state } = this.editor;
    const { $from } = state.selection;
    const marks = state.selection.empty ? state.storedMarks ?? $from.marks() : $from.marks();
    const parent = $from.parent;
    const blockAttrs: Record<string, unknown> = {};
    for (const key of BLOCK_ATTRS) if (key in parent.attrs) blockAttrs[key] = parent.attrs[key];
    this.captured = { marks, blockAttrs };
    this.sticky = sticky;
    this.attach();
    this.emit();
  }

  stop() {
    this.captured = null;
    this.cleanup?.();
    this.cleanup = null;
    this.emit();
  }

  private attach() {
    this.cleanup?.();
    const dom = this.editor.view.dom;
    dom.classList.add("format-painter-active");
    const apply = () => {
      // Erst nach Abschluss der Markierung anwenden.
      window.setTimeout(() => {
        if (!this.captured || this.editor.state.selection.empty) return;
        this.applyTo();
        if (!this.sticky) this.stop();
      }, 0);
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === "Escape") this.stop();
      else if (event.shiftKey || event.key === "Shift") apply();
    };
    dom.addEventListener("mouseup", apply);
    dom.addEventListener("keyup", onKeyUp);
    this.cleanup = () => {
      dom.classList.remove("format-painter-active");
      dom.removeEventListener("mouseup", apply);
      dom.removeEventListener("keyup", onKeyUp);
    };
  }

  private applyTo() {
    const captured = this.captured;
    if (!captured) return;
    const { state, view } = this.editor;
    const { from, to } = state.selection;
    const tr = state.tr;
    for (const markType of Object.values(state.schema.marks)) {
      if (markType.name === "link") continue;
      tr.removeMark(from, to, markType);
    }
    for (const mark of captured.marks) {
      if (mark.type.name !== "link") tr.addMark(from, to, mark);
    }
    state.doc.nodesBetween(from, to, (node, pos) => {
      if (!node.isTextblock) return true;
      const attrs = { ...node.attrs };
      let changed = false;
      for (const [key, value] of Object.entries(captured.blockAttrs)) {
        if (key in attrs && attrs[key] !== value) {
          attrs[key] = value;
          changed = true;
        }
      }
      if (changed) tr.setNodeMarkup(pos, undefined, attrs);
      return false;
    });
    view.dispatch(tr);
  }
}
