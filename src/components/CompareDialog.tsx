/**
 * Vergleich zweier Fassungen (Versionsverlauf, „Mit Datei vergleichen“): geänderte Abschnitte
 * mit Kontext, gelöschte Zeilen rot, neue grün, geänderte Wörter hervorgehoben. Die Berechnung
 * erfolgt in Rust (`core::diff`).
 */
import { useRef, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { TextDiff } from "../api";
import { useLocale, useT } from "../i18n";
import { Modal } from "./Dialogs";

type Props = {
  oldLabel: string;
  newLabel: string;
  diff: TextDiff;
  onClose: () => void;
  /** angeboten beim Vergleich mit einer Version */
  onRestore?: () => void;
};

export function CompareDialog({ oldLabel, newLabel, diff, onClose, onRestore }: Props) {
  const t = useT();
  const number = new Intl.NumberFormat(useLocale());
  const [current, setCurrent] = useState(0);
  const viewRef = useRef<HTMLDivElement>(null);
  const count = diff.hunks.length;
  const { summary } = diff;

  const go = (index: number) => {
    const next = Math.max(0, Math.min(count - 1, index));
    setCurrent(next);
    viewRef.current?.querySelector(`[data-hunk="${next}"]`)?.scrollIntoView({ block: "start", behavior: "smooth" });
  };

  return (
    <Modal
      title={t("Fassungen vergleichen")}
      wide
      className="compare-dialog"
      onClose={onClose}
      footer={
        <>
          {onRestore && (
            <button type="button" onClick={onRestore}>
              {t("Diese Version wiederherstellen")}
            </button>
          )}
          <button type="button" className="primary" onClick={onClose}>
            {t("Schließen")}
          </button>
        </>
      }
    >
      <div className="compare-head">
        <div className="compare-labels">
          <span className="compare-old">− {oldLabel}</span>
          <span className="compare-new">+ {newLabel}</span>
        </div>
        {!diff.identical && (
          <div className="compare-summary">
            <span>
              {count} {count === 1 ? t("Änderung") : t("Änderungen")}
            </span>
            <span className="plus">+{number.format(summary.addedWords)} {t("Wörter")}</span>
            <span className="minus">−{number.format(summary.removedWords)} {t("Wörter")}</span>
            <span className="muted">
              {number.format(summary.addedLines)} {t("Zeilen neu")}, {number.format(summary.removedLines)} {t("entfernt")}
            </span>
            <span className="compare-nav">
              <button type="button" className="icon-button" title={t("Vorherige Änderung")} disabled={current === 0} onClick={() => go(current - 1)}>
                <ChevronUp size={15} />
              </button>
              <span>
                {current + 1}/{count}
              </span>
              <button type="button" className="icon-button" title={t("Nächste Änderung")} disabled={current >= count - 1} onClick={() => go(current + 1)}>
                <ChevronDown size={15} />
              </button>
            </span>
          </div>
        )}
      </div>
      {diff.identical ? (
        <p className="review-ok">{t("Keine Unterschiede – beide Fassungen sind gleich.")}</p>
      ) : (
        <div className="compare-view" ref={viewRef}>
          {diff.hunks.map((hunk, index) => (
            <section key={index} className={`compare-hunk${index === current ? " is-current" : ""}`} data-hunk={index} onClick={() => setCurrent(index)}>
              {hunk.lines.map((line, lineIndex) => (
                <div key={lineIndex} className={`compare-line ${line.kind}`}>
                  <span className="compare-number">{line.oldLine ?? ""}</span>
                  <span className="compare-number">{line.newLine ?? ""}</span>
                  <span className="compare-marker">{line.kind === "added" ? "+" : line.kind === "removed" ? "−" : ""}</span>
                  <span className="compare-text">
                    {line.segments.map((segment, segmentIndex) => (segment.changed ? <mark key={segmentIndex}>{segment.text}</mark> : <span key={segmentIndex}>{segment.text}</span>))}
                    {line.segments.every((segment) => segment.text === "") && " "}
                  </span>
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
    </Modal>
  );
}
