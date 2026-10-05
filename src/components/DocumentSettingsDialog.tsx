/** Dokumenteinstellungen (gespeichert im Projekt, Grundlage der Präambel). */
import { useState } from "react";
import type { ReactNode } from "react";
import { useT } from "../i18n";
import { availableFontFamilies, bibliographyStyles, normalizeSettings } from "../latex/settings";
import type { DocumentSettings } from "../latex/settings";
import { Modal } from "./Dialogs";
import { DOCUMENT_LANGUAGES } from "../latex/languages";

type Section = "page" | "text" | "headerFooter" | "bibliography" | "metadata" | "preamble";

type Props = {
  settings: DocumentSettings;
  customPreamble: string | null;
  availableFonts: string[];
  initialSection?: string;
  onChooseBibliography: () => void;
  onApply: (settings: DocumentSettings, customPreamble: string | null) => void;
  onClose: () => void;
};

function Field({ label, children, wide = false }: { label: string; children: ReactNode; wide?: boolean }) {
  return (
    <label className={`form-field${wide ? " span-2" : ""}`}>
      <span>{label}</span>
      {children}
    </label>
  );
}

function NumberInput({ value, onChange, min, max, step = 1, unit }: { value: number; onChange: (value: number) => void; min: number; max: number; step?: number; unit?: string }) {
  return (
    <span className="input-with-unit">
      <input type="number" value={value} min={min} max={max} step={step} onChange={(event) => onChange(Number(event.currentTarget.value))} />
      {unit && <span className="unit">{unit}</span>}
    </span>
  );
}

export function DocumentSettingsDialog({ settings, customPreamble, availableFonts, initialSection, onChooseBibliography, onApply, onClose }: Props) {
  const t = useT();
  const [draft, setDraft] = useState<DocumentSettings>(settings);
  const [preamble, setPreamble] = useState(customPreamble ?? "");
  const [section, setSection] = useState<Section>((initialSection as Section) || "page");
  const set = (patch: Partial<DocumentSettings>) => setDraft((current) => ({ ...current, ...patch }));
  const setMargin = (key: keyof DocumentSettings["margins"], value: number) => set({ margins: { ...draft.margins, [key]: value } });
  const setHeader = (patch: Partial<DocumentSettings["headerFooter"]>) => set({ headerFooter: { ...draft.headerFooter, ...patch } });
  const setMeta = (patch: Partial<DocumentSettings["metadata"]>) => set({ metadata: { ...draft.metadata, ...patch } });
  const fonts = [...new Set([...availableFonts, draft.defaultFontFamily])];

  const sections: Array<{ id: Section; label: string }> = [
    { id: "page", label: t("Seite") },
    { id: "text", label: t("Text und Gliederung") },
    { id: "headerFooter", label: t("Kopf-/Fußzeile") },
    { id: "bibliography", label: t("Literatur") },
    { id: "metadata", label: t("PDF-Metadaten") },
    { id: "preamble", label: t("Präambel") },
  ];

  return (
    <Modal
      title={t("Dokumenteinstellungen")}
      wide
      onClose={onClose}
      className="settings-dialog"
      footer={
        <>
          <button type="button" onClick={onClose}>{t("Abbrechen")}</button>
          <button
            type="button"
            className="primary"
            onClick={() => {
              onApply(normalizeSettings(draft), preamble.trim() ? preamble : null);
              onClose();
            }}
          >
            {t("Übernehmen")}
          </button>
        </>
      }
    >
      <div className="settings-layout">
        <nav className="settings-nav">
          {sections.map((entry) => (
            <button key={entry.id} type="button" className={section === entry.id ? "active" : ""} onClick={() => setSection(entry.id)}>
              {entry.label}
            </button>
          ))}
        </nav>
        <div className="settings-content form-grid two-columns">
          {section === "page" && (
            <>
              <Field label={t("Papierformat")}>
                <select value={draft.paperFormat} onChange={(event) => set({ paperFormat: event.currentTarget.value as DocumentSettings["paperFormat"] })}>
                  <option value="a4">DIN A4</option>
                  <option value="a5">DIN A5</option>
                  <option value="letter">US Letter</option>
                  <option value="legal">US Legal</option>
                  <option value="custom">{t("Benutzerdefiniert")}</option>
                </select>
              </Field>
              <Field label={t("Ausrichtung")}>
                <select value={draft.orientation} onChange={(event) => set({ orientation: event.currentTarget.value as DocumentSettings["orientation"] })}>
                  <option value="portrait">{t("Hochformat")}</option>
                  <option value="landscape">{t("Querformat")}</option>
                </select>
              </Field>
              {draft.paperFormat === "custom" && (
                <>
                  <Field label={t("Breite")}><NumberInput value={draft.customWidth} min={50} max={1000} unit="mm" onChange={(value) => set({ customWidth: value })} /></Field>
                  <Field label={t("Höhe")}><NumberInput value={draft.customHeight} min={50} max={1000} unit="mm" onChange={(value) => set({ customHeight: value })} /></Field>
                </>
              )}
              <Field label={t("Rand oben")}><NumberInput value={draft.margins.top} min={0} max={100} step={0.5} unit="mm" onChange={(value) => setMargin("top", value)} /></Field>
              <Field label={t("Rand unten")}><NumberInput value={draft.margins.bottom} min={0} max={100} step={0.5} unit="mm" onChange={(value) => setMargin("bottom", value)} /></Field>
              <Field label={draft.twoside ? t("Rand innen") : t("Rand links")}><NumberInput value={draft.margins.left} min={0} max={100} step={0.5} unit="mm" onChange={(value) => setMargin("left", value)} /></Field>
              <Field label={draft.twoside ? t("Rand außen") : t("Rand rechts")}><NumberInput value={draft.margins.right} min={0} max={100} step={0.5} unit="mm" onChange={(value) => setMargin("right", value)} /></Field>
              <Field label={t("Bundsteg (Bindekorrektur)")}><NumberInput value={draft.bindingOffset} min={0} max={50} step={0.5} unit="mm" onChange={(value) => set({ bindingOffset: value })} /></Field>
              <Field label={t("Spalten")}>
                <select value={draft.columns} onChange={(event) => set({ columns: Number(event.currentTarget.value) as 1 | 2 | 3 })}>
                  <option value={1}>1</option>
                  <option value={2}>2</option>
                  <option value={3}>3</option>
                </select>
              </Field>
              <label className="checkbox form-checkbox span-2">
                <input type="checkbox" checked={draft.twoside} onChange={(event) => set({ twoside: event.currentTarget.checked })} />
                {t("Doppelseitiger Druck (gerade/ungerade Seiten)")}
              </label>
              <Field label={t("Seitenfarbe im PDF")}>
                <span className="color-field">
                  <input type="color" value={draft.pagePdfColor || "#ffffff"} onChange={(event) => set({ pagePdfColor: event.currentTarget.value })} />
                  <button type="button" className="small" onClick={() => set({ pagePdfColor: "" })}>{t("Keine")}</button>
                </span>
              </Field>
              <Field label={t("Seitenfarbe in der Ansicht")}>
                <span className="color-field">
                  <input type="color" value={draft.pageDisplayColor || "#ffffff"} onChange={(event) => set({ pageDisplayColor: event.currentTarget.value })} />
                  <button type="button" className="small" onClick={() => set({ pageDisplayColor: "" })}>{t("Keine")}</button>
                </span>
              </Field>
            </>
          )}
          {section === "text" && (
            <>
              <Field label={t("Dokumentklasse")}>
                <select value={draft.documentClass} onChange={(event) => set({ documentClass: event.currentTarget.value as DocumentSettings["documentClass"] })}>
                  <option value="scrreprt">{t("Bericht/Abschlussarbeit (KOMA scrreprt, mit Kapiteln)")}</option>
                  <option value="article">{t("Artikel (ohne Kapitel)")}</option>
                </select>
              </Field>
              <Field label={t("Sprache")}>
                <select value={draft.language} onChange={(event) => set({ language: event.currentTarget.value as DocumentSettings["language"] })}>
                  {DOCUMENT_LANGUAGES.map((language) => (
                    <option key={language.id} value={language.id}>{language.name}</option>
                  ))}
                </select>
              </Field>
              <Field label={t("Grundschrift")}>
                <select value={draft.defaultFontFamily} onChange={(event) => set({ defaultFontFamily: event.currentTarget.value })}>
                  {fonts.map((font) => (
                    <option key={font} value={font}>{font}{availableFontFamilies.includes(font as (typeof availableFontFamilies)[number]) ? "" : " *"}</option>
                  ))}
                </select>
              </Field>
              <Field label={t("Schriftgröße")}>
                <select value={draft.defaultFontSize} onChange={(event) => set({ defaultFontSize: Number(event.currentTarget.value) })}>
                  {[8, 9, 10, 11, 12, 13, 14, 16, 18, 20].map((size) => <option key={size} value={size}>{size} pt</option>)}
                </select>
              </Field>
              <Field label={t("Zeilenabstand")}>
                <select value={String(draft.lineSpacing)} onChange={(event) => set({ lineSpacing: Number(event.currentTarget.value) })}>
                  {["1", "1.15", "1.25", "1.5", "2"].map((value) => <option key={value} value={value}>{value.replace(".", ",")}</option>)}
                </select>
              </Field>
              <Field label={t("Absätze")}>
                <select value={draft.paragraphStyle} onChange={(event) => set({ paragraphStyle: event.currentTarget.value as DocumentSettings["paragraphStyle"] })}>
                  <option value="indent">{t("Erstzeileneinzug")}</option>
                  <option value="skip">{t("Abstand zwischen Absätzen")}</option>
                </select>
              </Field>
              <Field label={t("Nummerierungstiefe")}>
                <select value={draft.numberingDepth} onChange={(event) => set({ numberingDepth: Number(event.currentTarget.value) })}>
                  {[0, 1, 2, 3, 4, 5].map((value) => <option key={value} value={value}>{value === 0 ? t("keine") : `${t("bis Ebene")} ${value}`}</option>)}
                </select>
              </Field>
              <Field label={t("Tiefe im Inhaltsverzeichnis")}>
                <select value={draft.tocDepth} onChange={(event) => set({ tocDepth: Number(event.currentTarget.value) })}>
                  {[1, 2, 3, 4, 5].map((value) => <option key={value} value={value}>{`${t("bis Ebene")} ${value}`}</option>)}
                </select>
              </Field>
              <p className="form-info span-2">{t("Mit * markierte Schriften sind auf diesem Rechner nicht installiert; pdfLaTeX verwendet eine ähnliche Standardschrift.")}</p>
            </>
          )}
          {section === "headerFooter" && (
            <>
              <label className="checkbox form-checkbox span-2">
                <input type="checkbox" checked={draft.headerFooter.enabled} onChange={(event) => setHeader({ enabled: event.currentTarget.checked })} />
                {t("Kopf- und Fußzeile verwenden")}
              </label>
              {(["headerLeft", "headerCenter", "headerRight", "footerLeft", "footerCenter", "footerRight"] as const).map((key) => (
                <Field
                  key={key}
                  label={t({ headerLeft: "Kopfzeile links", headerCenter: "Kopfzeile Mitte", headerRight: "Kopfzeile rechts", footerLeft: "Fußzeile links", footerCenter: "Fußzeile Mitte", footerRight: "Fußzeile rechts" }[key])}
                >
                  <input value={draft.headerFooter[key]} disabled={!draft.headerFooter.enabled} onChange={(event) => setHeader({ [key]: event.currentTarget.value })} />
                </Field>
              ))}
              <p className="form-info span-2">{t("Platzhalter: {seite} {seiten} {kapitel} {titel} {autor} {datum}")}</p>
            </>
          )}
          {section === "bibliography" && (
            <>
              <Field label={t("Literaturdatei (BibTeX, relativ zum Projektordner)")} wide>
                <span className="color-field">
                  <input
                    value={draft.bibliography.file}
                    placeholder="literatur.bib"
                    onChange={(event) => set({ bibliography: { ...draft.bibliography, file: event.currentTarget.value } })}
                  />
                  <button type="button" className="small" onClick={onChooseBibliography}>{t("Auswählen …")}</button>
                </span>
              </Field>
              <Field label={t("Zitierstil")} wide>
                <select value={draft.bibliography.style} onChange={(event) => set({ bibliography: { ...draft.bibliography, style: event.currentTarget.value as DocumentSettings["bibliography"]["style"] } })}>
                  {bibliographyStyles.map((style) => <option key={style.id} value={style.id}>{t(style.label)}</option>)}
                </select>
              </Field>
              <p className="form-info span-2">{t("VisuTeX verwendet BibTeX mit natbib (in der eingebauten Engine enthalten). Einträge können aus Zotero übernommen werden (Referenzen → Zitat einfügen).")}</p>
            </>
          )}
          {section === "metadata" && (
            <>
              <Field label={t("Titel")} wide><input value={draft.metadata.title} onChange={(event) => setMeta({ title: event.currentTarget.value })} /></Field>
              <Field label={t("Autor/in")} wide><input value={draft.metadata.author} onChange={(event) => setMeta({ author: event.currentTarget.value })} /></Field>
              <Field label={t("Thema")} wide><input value={draft.metadata.subject} onChange={(event) => setMeta({ subject: event.currentTarget.value })} /></Field>
              <Field label={t("Schlüsselwörter")} wide><input value={draft.metadata.keywords} onChange={(event) => setMeta({ keywords: event.currentTarget.value })} /></Field>
              <Field label={t("PDF-Standard")} wide>
                <select value={draft.metadata.pdfStandard} onChange={(event) => setMeta({ pdfStandard: event.currentTarget.value as DocumentSettings["metadata"]["pdfStandard"] })}>
                  <option value="">{t("Normales PDF")}</option>
                  <option value="a-2b">{t("PDF/A-2b (Archiv, z. B. für Abschlussarbeiten)")}</option>
                  <option value="a-3b">{t("PDF/A-3b (Archiv mit eingebetteten Dateien)")}</option>
                </select>
              </Field>
              <p className="form-info span-2">{t("PDF/A gilt für den LaTeX-Export: TeX Live, MiKTeX und Overleaf (ab 2022) erzeugen daraus ein archivfähiges PDF mit eingebetteten Schriften, Metadaten und Farbprofil. Die eingebaute Engine erzeugt weiterhin ein normales PDF.")}</p>
            </>
          )}
          {section === "preamble" && (
            <>
              <p className="form-info span-2">
                {preamble.trim()
                  ? t("Dieses Dokument verwendet eine eigene Präambel. Einstellungen, die die Präambel betreffen, wirken erst nach „Verwaltete Präambel verwenden“.")
                  : t("Die Präambel wird aus den Einstellungen erzeugt (verwaltet). Eine eigene Präambel entsteht automatisch, wenn sie in der Code-Ansicht geändert wird.")}
              </p>
              {preamble.trim() && (
                <>
                  <textarea className="code-input span-2" rows={14} spellCheck={false} value={preamble} onChange={(event) => setPreamble(event.currentTarget.value)} />
                  <div className="span-2">
                    <button type="button" onClick={() => setPreamble("")}>{t("Verwaltete Präambel verwenden")}</button>
                  </div>
                </>
              )}
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
