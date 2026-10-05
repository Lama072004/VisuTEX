/** Weiße Seiten mit Kopf-/Fußzeilen-Vorschau hinter dem paginierten Editor. */
import type { PageInfo } from "../editor/pagination";
import type { DocumentSettings } from "../latex/settings";
import { documentLanguage } from "../latex/languages";

type Props = {
  pages: PageInfo[];
  pageHeight: number;
  gap: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
  settings: DocumentSettings;
};

function fill(text: string, page: number, total: number, info: PageInfo, settings: DocumentSettings) {
  const locale = documentLanguage(settings.language).bcp47;
  return text
    .replace(/\{seite\}/g, String(page))
    .replace(/\{seiten\}/g, String(total))
    .replace(/\{kapitel\}/g, info.chapter)
    .replace(/\{titel\}/g, settings.metadata.title)
    .replace(/\{autor\}/g, settings.metadata.author)
    .replace(/\{datum\}/g, new Date().toLocaleDateString(locale, { day: "numeric", month: "long", year: "numeric" }));
}

export function PageCanvas({ pages, pageHeight, gap, marginTop, marginBottom, marginLeft, marginRight, settings }: Props) {
  const hf = settings.headerFooter;
  const total = pages.length;
  return (
    <div className="page-canvas" aria-hidden="true">
      {pages.map((info, index) => {
        const page = index + 1;
        const showHeader = hf.enabled && !info.titlePage && !info.chapterStart;
        const showFooter = hf.enabled && !info.titlePage;
        const mirrored = settings.twoside && page % 2 === 0;
        const left = mirrored ? marginRight : marginLeft;
        const right = mirrored ? marginLeft : marginRight;
        return (
          <div key={index} className="page-sheet" style={{ top: index * (pageHeight + gap), height: pageHeight }}>
            {showHeader && (
              <div className="page-header" style={{ height: marginTop, left, right }}>
                <span>{fill(hf.headerLeft, page, total, info, settings)}</span>
                <span>{fill(hf.headerCenter, page, total, info, settings)}</span>
                <span>{fill(hf.headerRight, page, total, info, settings)}</span>
              </div>
            )}
            {showFooter && (
              <div className="page-footer" style={{ height: marginBottom, left, right }}>
                <span>{fill(hf.footerLeft, page, total, info, settings)}</span>
                <span>{fill(hf.footerCenter, page, total, info, settings)}</span>
                <span>{fill(hf.footerRight, page, total, info, settings)}</span>
              </div>
            )}
            {!hf.enabled && !info.titlePage && <div className="page-number" style={{ height: marginBottom }}>{page}</div>}
          </div>
        );
      })}
    </div>
  );
}
