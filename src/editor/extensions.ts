/**
 * Editor-Erweiterungen der App: Ziel-Schema aus `schema.ts` + React-NodeViews,
 * Suchen & Ersetzen, Paginierung und Format übertragen.
 */
import type { AnyExtension } from "@tiptap/core";
import { Extension } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import {
  AcronymNode,
  AppendixMarker,
  Citation,
  CrossReference,
  DirectoryBlock,
  DocumentImage,
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
import { SearchReplace } from "./search";
import { TableCaptions } from "./tableCaptions";

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
    ExtraKeys,
  ];
}
