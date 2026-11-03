/**
 * Abgabe-Check (Registerkarte „Überprüfen“): Funde aus Rust (`review_document`), gruppiert nach
 * Bereich, Fehler zuerst. Ein Klick springt zum betroffenen Block.
 */
import { CircleAlert, ClipboardCheck, Info, RefreshCw, TriangleAlert, X } from "lucide-react";
import type { ReviewFinding } from "../api";
import { useT } from "../i18n";

/** Code → (Bereich, Text) – deutsche Texte sind zugleich die Übersetzungsschlüssel. */
const FINDINGS: Record<string, { category: string; text: string }> = {
  "reference-unknown": { category: "Querverweise", text: "Querverweis auf ein unbekanntes Label" },
  "label-duplicate": { category: "Querverweise", text: "Label ist mehrfach vergeben" },
  "label-unused": { category: "Querverweise", text: "Label wird nirgends verwendet" },
  "citation-unknown": { category: "Literatur", text: "Quelle fehlt in der Literaturdatei" },
  "citation-no-bibliography": { category: "Literatur", text: "Zitate vorhanden, aber keine Literaturdatei gewählt" },
  "bibliography-missing": { category: "Literatur", text: "Literaturdatei nicht gefunden" },
  "figure-no-caption": { category: "Abbildungen und Tabellen", text: "Abbildung ohne Beschriftung" },
  "table-no-caption": { category: "Abbildungen und Tabellen", text: "Tabelle ohne Beschriftung" },
  "drawing-no-caption": { category: "Abbildungen und Tabellen", text: "Zeichnung ohne Beschriftung" },
  "image-missing": { category: "Abbildungen und Tabellen", text: "Bilddatei nicht gefunden" },
  "section-empty": { category: "Gliederung", text: "Abschnitt ohne Inhalt" },
  "heading-skip": { category: "Gliederung", text: "Gliederung überspringt eine Ebene" },
  placeholder: { category: "Inhalt", text: "Platzhalter oder Notiz im Text" },
  "title-empty": { category: "Inhalt", text: "Titelseite: Feld ist leer" },
  comment: { category: "Inhalt", text: "Kommentar im Dokument (im PDF unsichtbar)" },
  "compile-errors": { category: "Kompilieren und Seitenlayout", text: "Das letzte Kompilieren meldete Fehler" },
  overfull: { category: "Kompilieren und Seitenlayout", text: "Text ragt über den Rand" },
  "overfull-page": { category: "Kompilieren und Seitenlayout", text: "Seite ist zu voll (Inhalt ragt nach unten hinaus)" },
  "not-compiled": { category: "Kompilieren und Seitenlayout", text: "Noch nicht kompiliert – überlange Zeilen werden erst nach dem Kompilieren (F5) geprüft" },
};

const CATEGORY_ORDER = ["Querverweise", "Literatur", "Abbildungen und Tabellen", "Gliederung", "Inhalt", "Kompilieren und Seitenlayout"];

/** Feldnamen der Titelseite (Detail von `title-empty`) */
const TITLE_FIELDS: Record<string, string> = { title: "Titel", author: "Autor/in" };

type Props = {
  findings: ReviewFinding[] | null;
  busy: boolean;
  onRefresh: () => void;
  onClose: () => void;
  onJump: (block: number) => void;
};

export function ReviewPanel({ findings, busy, onRefresh, onClose, onJump }: Props) {
  const t = useT();
  const list = findings ?? [];
  const count = (severity: string) => list.filter((finding) => finding.severity === severity).length;
  const categories = CATEGORY_ORDER.map((category) => ({
    category,
    items: list.filter((finding) => (FINDINGS[finding.code]?.category ?? "Inhalt") === category),
  })).filter((group) => group.items.length > 0);

  const text = (finding: ReviewFinding) => {
    const base = t(FINDINGS[finding.code]?.text ?? finding.code);
    const detail = finding.code === "title-empty" ? t(TITLE_FIELDS[finding.detail] ?? finding.detail) : finding.detail;
    return detail ? `${base}: ${detail}` : base;
  };

  return (
    <aside className="review-pane" aria-label={t("Abgabe-Check")}>
      <div className="review-header">
        <span className="pane-title">
          <ClipboardCheck size={16} /> {t("Abgabe-Check")}
        </span>
        <button type="button" className="icon-button" title={t("Erneut prüfen")} onClick={onRefresh} disabled={busy}>
          <RefreshCw size={15} />
        </button>
        <button type="button" className="icon-button" title={t("Schließen")} onClick={onClose}>
          <X size={15} />
        </button>
      </div>
      {busy && !findings && <p className="muted review-note">{t("Dokument wird geprüft …")}</p>}
      {findings && (
        <div className="review-summary">
          <span className="review-count error"><CircleAlert size={13} /> {count("error")} {t("Fehler")}</span>
          <span className="review-count warning"><TriangleAlert size={13} /> {count("warning")} {count("warning") === 1 ? t("Warnung") : t("Warnungen")}</span>
          <span className="review-count info"><Info size={13} /> {count("info")} {count("info") === 1 ? t("Hinweis") : t("Hinweise")}</span>
        </div>
      )}
      {findings && list.length === 0 && <p className="review-ok">{t("Keine Probleme gefunden.")}</p>}
      {categories.map(({ category, items }) => (
        <section key={category} className="review-group">
          <h3>{t(category)}</h3>
          <ul>
            {items.map((finding, index) => (
              <li key={`${finding.code}-${finding.block}-${index}`} className={finding.severity}>
                <button type="button" disabled={finding.block === null} onClick={() => finding.block !== null && onJump(finding.block)}>
                  {finding.severity === "error" ? <CircleAlert size={14} /> : finding.severity === "warning" ? <TriangleAlert size={14} /> : <Info size={14} />}
                  <span>{text(finding)}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}
      <p className="muted review-note">{t("Klick auf einen Eintrag springt zur Stelle im Dokument. Nach Änderungen „Erneut prüfen“.")}</p>
    </aside>
  );
}
