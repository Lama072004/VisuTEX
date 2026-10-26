/**
 * Editor-Erweiterungen der App: Ziel-Schema aus `schema.ts` + React-NodeViews,
 * Suchen & Ersetzen, Paginierung und Format übertragen.
 */
import type { AnyExtension } from "@tiptap/core";
import { Extension } from "@tiptap/core";
import { NodeSelection, Plugin } from "@tiptap/pm/state";
import { ReactNodeViewRenderer } from "@tiptap/react";
import {
  AcronymNode,
  AppendixMarker,
  Citation,
  CrossReference,
  DirectoryBlock,
  DocumentImage,
  Subfigures,
  EnvironmentBlock,
  Footnote,
  FrontmatterBlock,
  IncludeBlock,
  InlineMath,
  Maketitle,
  MathBlock,
  PageBreak,
  Quantity,
  RawLatexBlock,
  RawLatexInline,
  TikzBlock,
  TitlePage,
  VerticalSpace,
  baseExtensions,
} from "./schema";
import {
  AcronymView,
  AppendixView,
  CitationView,
  CrossReferenceView,
  DirectoryView,
  EnvironmentBlockView,
  FootnoteView,
  FrontmatterView,
  ImageView,
  SubfiguresView,
  IncludeBlockView,
  InlineMathView,
  MaketitleView,
  MathBlockView,
  PageBreakView,
  QuantityView,
  RawLatexBlockView,
  RawLatexInlineView,
  TikzBlockView,
  TitlePageView,
  VerticalSpaceView,
} from "./nodeViews";
import { Pagination } from "./pagination";
import { FormattingMarks } from "./formattingMarks";
import { SelectionHighlight } from "./selectionHighlight";
import { CommentLayout } from "./commentLayout";
import { LayoutParagraphs } from "./layoutParagraphs";
import { SearchReplace } from "./search";
import { TableCaptions } from "./tableCaptions";

/**
 * Ausgewählte Blöcke wie Objekte in Word: Entf/Rücktaste löschen sie (ProseMirror-Standard,
 * der Fokus bleibt im Dokument). Eingabetaste bzw. F2 öffnet die Bearbeitung (Element mit
 * `data-edit-focus` im Block: Codefeld fokussieren, sonst anklicken). Lostippen schreibt in das
 * Codefeld bzw. öffnet die Bearbeitung – statt den Block durch den Buchstaben zu ersetzen.
 */
const ObjectEditKeys = Extension.create({
  name: "objectEditKeys",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleKeyDown(view, event) {
            const selection = view.state.selection;
            if (!(selection instanceof NodeSelection) || event.ctrlKey || event.metaKey || event.altKey) return false;
            const dom = view.nodeDOM(selection.from);
            const target = dom instanceof HTMLElement ? dom.querySelector<HTMLElement>("[data-edit-focus]") : null;
            if (!target) return false;
            const activate = () => (target instanceof HTMLTextAreaElement ? target.focus() : target.click());
            if (event.key === "Enter" || event.key === "F2") {
              activate();
              return true;
            }
            if (event.key.length === 1) {
              activate();
              if (target instanceof HTMLTextAreaElement) {
                target.setSelectionRange(target.value.length, target.value.length);
                // als echte Eingabe einfügen (löst das React-onChange aus)
                document.execCommand("insertText", false, event.key);
              }
              return true;
            }
            return false;
          },
        },
      }),
    ];
  },
});

/** Tastenkürzel, die nicht von StarterKit kommen. */
const ExtraKeys = Extension.create({
  name: "visutexKeys",
  addKeyboardShortcuts() {
    return {
      "Mod-Alt-1": () => this.editor.commands.toggleHeading({ level: 1 }),
      "Mod-Alt-2": () => this.editor.commands.toggleHeading({ level: 2 }),
      "Mod-Alt-3": () => this.editor.commands.toggleHeading({ level: 3 }),
      "Mod-Alt-0": () => this.editor.commands.setParagraph(),
      "Mod-Enter": () => this.editor.commands.insertContent({ type: "pageBreak", attrs: { breakType: "page" } }),
    };
  },
});

export function appExtensions(): AnyExtension[] {
  return [
    ...baseExtensions({
      mathBlock: MathBlock.extend({ addNodeView: () => ReactNodeViewRenderer(MathBlockView) }),
      inlineMath: InlineMath.extend({ addNodeView: () => ReactNodeViewRenderer(InlineMathView) }),
      tikzBlock: TikzBlock.extend({ addNodeView: () => ReactNodeViewRenderer(TikzBlockView) }),
      image: DocumentImage.extend({ addNodeView: () => ReactNodeViewRenderer(ImageView) }),
      subfigures: Subfigures.extend({ addNodeView: () => ReactNodeViewRenderer(SubfiguresView) }),
      titlePage: TitlePage.extend({ addNodeView: () => ReactNodeViewRenderer(TitlePageView) }),
      frontmatterBlock: FrontmatterBlock.extend({ addNodeView: () => ReactNodeViewRenderer(FrontmatterView) }),
      directoryBlock: DirectoryBlock.extend({ addNodeView: () => ReactNodeViewRenderer(DirectoryView) }),
      rawLatexBlock: RawLatexBlock.extend({ addNodeView: () => ReactNodeViewRenderer(RawLatexBlockView) }),
      rawLatexInline: RawLatexInline.extend({ addNodeView: () => ReactNodeViewRenderer(RawLatexInlineView) }),
      pageBreak: PageBreak.extend({ addNodeView: () => ReactNodeViewRenderer(PageBreakView) }),
      citation: Citation.extend({ addNodeView: () => ReactNodeViewRenderer(CitationView) }),
      footnote: Footnote.extend({ addNodeView: () => ReactNodeViewRenderer(FootnoteView) }),
      acronym: AcronymNode.extend({ addNodeView: () => ReactNodeViewRenderer(AcronymView) }),
      crossReference: CrossReference.extend({ addNodeView: () => ReactNodeViewRenderer(CrossReferenceView) }),
      quantity: Quantity.extend({ addNodeView: () => ReactNodeViewRenderer(QuantityView) }),
      verticalSpace: VerticalSpace.extend({ addNodeView: () => ReactNodeViewRenderer(VerticalSpaceView) }),
      environmentBlock: EnvironmentBlock.extend({ addNodeView: () => ReactNodeViewRenderer(EnvironmentBlockView) }),
      maketitle: Maketitle.extend({ addNodeView: () => ReactNodeViewRenderer(MaketitleView) }),
      appendixMarker: AppendixMarker.extend({ addNodeView: () => ReactNodeViewRenderer(AppendixView) }),
      includeBlock: IncludeBlock.extend({ addNodeView: () => ReactNodeViewRenderer(IncludeBlockView) }),
    }),
    SearchReplace,
    TableCaptions,
    Pagination,
    FormattingMarks,
    CommentLayout,
    LayoutParagraphs,
    ObjectEditKeys,
    SelectionHighlight,
    ExtraKeys,
  ];
}
