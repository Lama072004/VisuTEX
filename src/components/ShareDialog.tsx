/**
 * Teilen der gespeicherten Datei (Rechtsklick auf die Titelleiste oder Schnellzugriff).
 * Der Dialog sagt ausdrücklich, welche Fassung geteilt wird: immer die Datei auf dem
 * Datenträger – ungespeicherte Änderungen sind nicht enthalten.
 */
import { useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { AlertTriangle, Copy, FolderOpen, Save, Share2 } from "lucide-react";
import { api, errorText } from "../api";
import { useT } from "../i18n";

const windows = typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);

export type ShareTarget = {
  /** Gespeicherte Datei (null: noch nie gespeichert) */
  path: string | null;
  name: string;
  /** Zeitpunkt der letzten Speicherung (ms) */
  savedAt: number | null;
  /** Ungespeicherte Änderungen vorhanden */
  dirty: boolean;
  /** LaTeX-Projekt (Bilder/Teildateien liegen daneben) */
  latexProject: boolean;
};

type Props = {
  target: ShareTarget;
  locale: string;
  /** Speichern (bzw. Speichern unter bei neuen Dokumenten); true bei Erfolg */
  onSave: () => Promise<boolean>;
  onShared: (message: string) => void;
  onClose: () => void;
};

export function formatSavedAt(savedAt: number | null, locale: string): string {
  if (!savedAt) return "";
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(savedAt));
}

export function ShareDialog({ target, locale, onSave, onShared, onClose }: Props) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const primary = useRef<HTMLButtonElement>(null);
  useEffect(() => primary.current?.focus(), [target.path]);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [onClose]);

  const savedAt = formatSavedAt(target.savedAt, locale);

  const share = async (saveFirst: boolean) => {
    setBusy(true);
    setError("");
    try {
      if (saveFirst && !(await onSave())) {
        setBusy(false);
        return;
      }
      // Speichern behält den Pfad bei (neue Dokumente laufen über „Speichern …“ ohne Teilen)
      const path = target.path;
      if (!path) {
        setBusy(false);
        return;
      }
      const name = await api.shareFile(path);
      onShared(
        windows
          ? `${t("Teilen geöffnet für")} „${name}“ – ${t("gespeicherte Fassung")}${savedAt && !saveFirst ? ` ${t("vom")} ${savedAt}` : ""}.`
          : `${t("E-Mail-Programm mit")} „${name}“ ${t("als Anhang geöffnet")}.`,
      );
      onClose();
    } catch (failure) {
      setError(errorText(failure));
      setBusy(false);
    }
  };

  const copyPath = async () => {
    if (!target.path) return;
    try {
      await navigator.clipboard.writeText(target.path);
      setCopied(true);
    } catch (failure) {
      setError(errorText(failure));
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <div className="modal share-dialog" role="dialog" aria-modal="true" aria-label={t("Dokument teilen")}>
        <div className="modal-header">
          <h2>
            <Share2 size={18} /> {t("Dokument teilen")}
          </h2>
        </div>
        <div className="modal-body">
          {!target.path ? (
            <p>{t("Dieses Dokument wurde noch nicht gespeichert. Geteilt werden kann nur eine gespeicherte Datei – bitte zuerst speichern.")}</p>
          ) : (
            <>
              <p>
                <strong>{t("Geteilt wird die zuletzt gespeicherte Fassung dieser Datei:")}</strong>
              </p>
              <div className="share-file">
                <span className="share-file-name">{target.name}</span>
                <code>{target.path}</code>
                {savedAt && (
                  <span className="muted">
                    {t("Gespeichert am")} {savedAt}
                  </span>
                )}
                <span className="share-secondary">
                  <button type="button" className="small" onClick={() => target.path && void revealItemInDir(target.path)}>
                    <FolderOpen size={14} /> {t("Im Ordner anzeigen")}
                  </button>
                  <button type="button" className="small" onClick={() => void copyPath()}>
                    <Copy size={14} /> {copied ? t("Pfad kopiert") : t("Pfad kopieren")}
                  </button>
                </span>
              </div>
              {target.dirty ? (
                <p className="share-warning">
                  <AlertTriangle size={16} />
                  <span>{t("Es gibt ungespeicherte Änderungen. Sie sind in der geteilten Datei nicht enthalten – außer Sie speichern vorher.")}</span>
                </p>
              ) : (
                <p className="muted">{t("Alle Änderungen sind gespeichert – die geteilte Datei entspricht dem aktuellen Stand.")}</p>
              )}
              {target.latexProject && (
                <p className="muted small">{t("Verwendet das Dokument Bilder oder Teildateien, geben Sie am besten den ganzen Ordner weiter („Im Ordner anzeigen“).")}</p>
              )}
              <p className="muted small">
                {windows
                  ? t("Es öffnet sich das Teilen-Fenster von Windows (z. B. E-Mail, Teams, Nearby Sharing).")
                  : t("Es öffnet sich Ihr E-Mail-Programm mit der Datei als Anhang.")}
              </p>
            </>
          )}
          {error && <p className="setup-error">{error}</p>}
        </div>
        <div className="modal-footer share-footer">
          <button type="button" onClick={onClose} disabled={busy}>
            {t("Abbrechen")}
          </button>
          {!target.path ? (
            <button ref={primary} type="button" className="primary" onClick={() => void onSave()} disabled={busy}>
              <Save size={14} /> {t("Speichern …")}
            </button>
          ) : target.dirty ? (
            <>
              <button type="button" onClick={() => void share(false)} disabled={busy}>
                {t("Gespeicherte Fassung teilen")}
              </button>
              <button ref={primary} type="button" className="primary" onClick={() => void share(true)} disabled={busy}>
                <Save size={14} /> {t("Speichern und teilen")}
              </button>
            </>
          ) : (
            <button ref={primary} type="button" className="primary" onClick={() => void share(false)} disabled={busy}>
              <Share2 size={14} /> {t("Teilen …")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

type MenuProps = {
  x: number;
  y: number;
  hasFile: boolean;
  onShare: () => void;
  onReveal: () => void;
  onCopyPath: () => void;
  onClose: () => void;
};

/** Kontextmenü der Titelleiste (Rechtsklick): Teilen und Dateiaktionen. */
export function TitleContextMenu({ x, y, hasFile, onShare, onReveal, onCopyPath, onClose }: MenuProps) {
  const t = useT();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: MouseEvent) => !ref.current?.contains(event.target as Node) && onClose();
    const escape = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", escape);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", escape);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);
  const run = (action: () => void) => () => {
    onClose();
    action();
  };
  return (
    <div ref={ref} className="context-menu" role="menu" style={{ left: Math.min(x, window.innerWidth - 260), top: y }}>
      <button type="button" role="menuitem" onClick={run(onShare)}>
        <Share2 size={15} /> {t("Gespeicherte Datei teilen …")}
      </button>
      <button type="button" role="menuitem" disabled={!hasFile} onClick={run(onReveal)}>
        <FolderOpen size={15} /> {t("Im Ordner anzeigen")}
      </button>
      <button type="button" role="menuitem" disabled={!hasFile} onClick={run(onCopyPath)}>
        <Copy size={15} /> {t("Dateipfad kopieren")}
      </button>
    </div>
  );
}
