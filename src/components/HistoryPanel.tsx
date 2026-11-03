/**
 * Versionsverlauf (Registerkarte „Überprüfen“): beim Speichern angelegte und benannte Versionen
 * (Ablage in Rust, `history.rs`), nach Tagen gruppiert. Vergleichen, wiederherstellen,
 * umbenennen, löschen.
 */
import { ArchiveRestore, BookmarkPlus, FileDiff, GitCompareArrows, History, Pencil, RefreshCw, Trash2, X } from "lucide-react";
import type { VersionInfo } from "../api";
import { useLocale, useT } from "../i18n";

type Props = {
  /** Pfad des gespeicherten Dokuments (null: noch nicht gespeichert) */
  documentPath: string | null;
  versions: VersionInfo[] | null;
  busy: boolean;
  onRefresh: () => void;
  onClose: () => void;
  onSaveVersion: () => void;
  onCompareFile: () => void;
  onCompare: (version: VersionInfo) => void;
  onRestore: (version: VersionInfo) => void;
  onRename: (version: VersionInfo) => void;
  onDelete: (version: VersionInfo) => void;
};

const DAY = 24 * 60 * 60 * 1000;

export function HistoryPanel({ documentPath, versions, busy, onRefresh, onClose, onSaveVersion, onCompareFile, onCompare, onRestore, onRename, onDelete }: Props) {
  const t = useT();
  const locale = useLocale();
  const list = versions ?? [];
  const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" });
  const date = new Intl.DateTimeFormat(locale, { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  const number = new Intl.NumberFormat(locale);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dayLabel = (created: number) => {
    const day = new Date(created);
    day.setHours(0, 0, 0, 0);
    const difference = Math.round((today.getTime() - day.getTime()) / DAY);
    if (difference === 0) return t("Heute");
    if (difference === 1) return t("Gestern");
    return date.format(day);
  };
  const groups: Array<{ label: string; items: Array<{ version: VersionInfo; older: VersionInfo | undefined }> }> = [];
  list.forEach((version, index) => {
    const label = dayLabel(version.created);
    const item = { version, older: list[index + 1] };
    const last = groups[groups.length - 1];
    if (last?.label === label) last.items.push(item);
    else groups.push({ label, items: [item] });
  });

  return (
    <aside className="review-pane history-pane" aria-label={t("Versionsverlauf")}>
      <div className="review-header">
        <span className="pane-title">
          <History size={16} /> {t("Versionsverlauf")}
        </span>
        <button type="button" className="icon-button" title={t("Aktualisieren")} onClick={onRefresh} disabled={busy || !documentPath}>
          <RefreshCw size={15} />
        </button>
        <button type="button" className="icon-button" title={t("Schließen")} onClick={onClose}>
          <X size={15} />
        </button>
      </div>
      <div className="history-actions">
        <button type="button" onClick={onSaveVersion} title={t("Aktuellen Stand als benannte Version sichern (z. B. „Abgabe“)")}>
          <BookmarkPlus size={15} /> {t("Version speichern …")}
        </button>
        <button type="button" onClick={onCompareFile} title={t("Aktuellen Stand mit einer anderen .tex-Datei vergleichen")}>
          <FileDiff size={15} /> {t("Mit Datei vergleichen …")}
        </button>
      </div>
      {!documentPath && <p className="muted review-note">{t("Versionen entstehen beim Speichern. Bitte das Dokument zuerst speichern.")}</p>}
      {documentPath && busy && !versions && <p className="muted review-note">{t("Versionen werden geladen …")}</p>}
      {documentPath && versions && list.length === 0 && <p className="muted review-note">{t("Noch keine Versionen – jedes Speichern legt eine an.")}</p>}
      {groups.map((group) => (
        <section key={group.label} className="review-group history-group">
          <h3>{group.label}</h3>
          <ul>
            {group.items.map(({ version, older }) => {
              const delta = older ? version.words - older.words : 0;
              return (
                <li key={version.id} className={version.name ? "named" : undefined}>
                  <button type="button" className="history-entry" onClick={() => onCompare(version)} title={t("Mit dem aktuellen Stand vergleichen")}>
                    <span className="history-time">{time.format(new Date(version.created))}</span>
                    <span className="history-name">{version.name || t("Automatisch beim Speichern")}</span>
                    <span className="history-words">
                      {number.format(version.words)} {t("Wörter")}
                      {delta !== 0 && <span className={delta > 0 ? "plus" : "minus"}> ({delta > 0 ? "+" : "−"}{number.format(Math.abs(delta))})</span>}
                    </span>
                  </button>
                  <span className="history-tools">
                    <button type="button" className="icon-button" title={t("Vergleichen")} onClick={() => onCompare(version)}>
                      <GitCompareArrows size={14} />
                    </button>
                    <button type="button" className="icon-button" title={t("Wiederherstellen")} onClick={() => onRestore(version)}>
                      <ArchiveRestore size={14} />
                    </button>
                    <button type="button" className="icon-button" title={t("Umbenennen")} onClick={() => onRename(version)}>
                      <Pencil size={14} />
                    </button>
                    <button type="button" className="icon-button" title={t("Version löschen")} onClick={() => onDelete(version)}>
                      <Trash2 size={14} />
                    </button>
                  </span>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
      {documentPath && list.length > 0 && (
        <p className="muted review-note">{t("Benannte Versionen bleiben erhalten; von den automatischen werden die letzten 100 aufbewahrt.")}</p>
      )}
    </aside>
  );
}
