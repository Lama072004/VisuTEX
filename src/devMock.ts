/**
 * Nur für die Entwicklung im reinen Browser (`npm run dev` ohne Tauri):
 * minimale Antworten der Rust-Commands, damit sich die Oberfläche ansehen
 * lässt. In der Desktop-App wird dieses Modul nie geladen.
 */
import type { JSONContent } from "@tiptap/core";
import { defaultDocumentSettings } from "./latex/settings";

function outline(document: JSONContent) {
  const result: Array<{ level: number; title: string; blockIndex: number }> = [];
  const labels: Array<{ label: string; kind: string; title: string }> = [];
  let words = 0;
  const text = (node: JSONContent): string => (node.text ?? "") + (node.content ?? []).map(text).join(" ");
  (document.content ?? []).forEach((block, blockIndex) => {
    if (block.type === "heading") {
      result.push({ level: Number(block.attrs?.level ?? 1), title: text(block), blockIndex });
      if (block.attrs?.label) labels.push({ label: String(block.attrs.label), kind: "heading", title: text(block) });
    }
    words += text(block).split(/\s+/).filter(Boolean).length;
  });
  return { outline: result, labels, citationKeys: [], acronyms: [], words, characters: words * 6, duplicateLabels: [] };
}

export async function mockInvoke(command: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (command) {
    case "app_info":
      return { version: "dev", bundle: { kind: "online", embeddedPath: null, fileCount: null }, availableFonts: ["Latin Modern Roman", "Arial", "Times New Roman", "Georgia"] };
    case "templates_list":
      return [
        { id: "leer-bericht", name: "Leerer Bericht", description: "KOMA-Bericht (scrreprt) mit Kapiteln, A4", source: "builtin" },
        { id: "demo", name: "Volltest-Dokument", description: "Alle Knotentypen (Entwicklung)", source: "builtin" },
      ];
    case "new_project": {
      const demo = args.template === "demo" || args.template === null;
      const document = demo
        ? ((await import("../tests/fixtures/full-document.json")).default as JSONContent)
        : { type: "doc", content: [{ type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Einleitung" }] }, { type: "paragraph" }] };
      return {
        project: { format: "visutex-project", version: 2, settings: { ...defaultDocumentSettings, headerFooter: { ...defaultDocumentSettings.headerFooter, enabled: true } }, document, customPreamble: null, lastMode: "visual", code: null },
        root: "/mock",
        path: null,
        kind: "visutex",
        warnings: [],
        missingFiles: [],
        stamp: null,
        temporary: true,
      };
    }
    case "analyze_document":
      return outline(args.document as JSONContent);
    case "export_document":
      return {
        latex: "% Im Browser-Modus steht der Rust-Kern nicht zur Verfügung.\n\\documentclass{scrreprt}\n\\begin{document}\n\\end{document}\n",
        preamble: "",
        generatedPreamble: "",
        body: "",
        assets: [],
        warnings: [],
        missingPackages: [],
      };
    case "addons_list":
    case "preamble_macros":
      return [];
    case "render_latex_previews":
      return ((args.blocks as string[]) ?? []).map(() => ({ image: null, error: "Vorschau nur in der Desktop-App" }));
    case "resolve_image":
      return { file: null, relative: null, dataUrl: null, kind: "missing" };
    case "sketch_to_latex":
      return { code: "% Skizze (nur in der Desktop-App)", environment: "tikzpicture" };
    case "sketch_from_code":
      return null;
    case "allow_project_dir":
      return null;
    case "sketch_catalog":
      return { pxPerCm: 85, categories: [], symbols: [], arrowTips: ["Stealth", "Latex"], lineWidths: ["thin", "thick"], dashPatterns: ["dashed", "dotted"], fontSizes: ["small", "large"] };
    case "slides_to_latex":
      return { latex: "% Beamer-Export nur in der Desktop-App", assets: [], warnings: [] };
    default:
      throw new Error(`„${command}“ ist nur in der Desktop-App verfügbar.`);
  }
}
