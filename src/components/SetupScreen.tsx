/**
 * Einrichtung beim ersten Start der heruntergeladenen EXE (Installationsort, Verknüpfungen,
 * Autostart, Updates) sowie Deinstallation und Update-Dialog. Die Arbeit erledigt Rust
 * (`setup.rs`, `updates.rs`); hier nur die Oberfläche.
 */
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, FolderOpen, PackageCheck, Trash2 } from "lucide-react";
import { api, errorText } from "../api";
import type { SetupStatus, UpdateInfo } from "../api";
import { useT } from "../i18n";

type SetupProps = {
  status: SetupStatus;
  checkUpdates: boolean;
  onCheckUpdatesChange: (value: boolean) => void;
  /** Ohne Installation weiter (bzw. Deinstallation abgebrochen) */
  onClose: () => void;
};

export function SetupScreen({ status, checkUpdates, onCheckUpdatesChange, onClose }: SetupProps) {
  const t = useT();
  const windows = status.platform === "windows";
  const existing = status.state?.mode === "installed" ? status.state : null;
  const [dir, setDir] = useState(existing?.installDir || status.defaultDir);
  const [desktop, setDesktop] = useState(existing ? existing.desktopShortcut : true);
  const [startMenu, setStartMenu] = useState(existing ? existing.startMenu : true);
  const [autostart, setAutostart] = useState(existing ? existing.autostart : false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const chooseDir = async () => {
    const chosen = await open({ directory: true, multiple: false, title: t("Installationsort wählen"), defaultPath: dir });
    if (typeof chosen === "string") setDir(chosen.replace(/[\\/]+$/, "").endsWith("VisuTeX") ? chosen : `${chosen.replace(/[\\/]+$/, "")}${windows ? "\\" : "/"}VisuTeX`);
  };

  const install = async () => {
    setBusy(true);
    setError("");
    try {
      await api.setupInstall({ dir, desktopShortcut: desktop, startMenu, autostart });
      // die installierte Kopie startet gleich; diese Instanz beendet sich
    } catch (failure) {
      setError(errorText(failure));
      setBusy(false);
    }
  };

  const portable = async () => {
    try {
      await api.setupUsePortable();
    } catch {
      // Entscheidung nicht speicherbar – trotzdem weiter
    }
    onClose();
  };

  const uninstall = async () => {
    setBusy(true);
    setError("");
    try {
      await api.setupUninstall();
    } catch (failure) {
      setError(errorText(failure));
      setBusy(false);
    }
  };

  if (status.uninstallRequested) {
    return (
      <div className="setup-screen" role="dialog" aria-modal="true" aria-label={t("VisuTeX deinstallieren")}>
        <div className="setup-card">
          <h1>
            <Trash2 size={22} /> {t("VisuTeX deinstallieren")}
          </h1>
          <p>{t("Entfernt das Programm, die Verknüpfungen, den Autostart und den Eintrag unter „Apps“. Ihre Dokumente bleiben unverändert.")}</p>
          {status.state?.installDir && (
            <p className="muted">
              {t("Programmordner")}: <code>{status.state.installDir}</code>
            </p>
          )}
          {error && <p className="setup-error">{error}</p>}
          <div className="setup-actions">
            <button type="button" onClick={onClose} disabled={busy}>{t("Abbrechen")}</button>
            <button type="button" className="danger" onClick={() => void uninstall()} disabled={busy}>
              {busy ? t("Wird entfernt …") : t("Deinstallieren")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="setup-screen" role="dialog" aria-modal="true" aria-label={t("VisuTeX einrichten")}>
      <div className="setup-card">
        <h1>
          <PackageCheck size={24} /> {t("VisuTeX einrichten")} <span className="setup-version">{status.version}</span>
        </h1>
        <p>{t("VisuTeX bringt alles mit, was es braucht (TeX-Engine, Pakete, Schriften) – keine weitere Installation nötig. Wählen Sie, wo das Programm gespeichert werden soll.")}</p>
        {status.sandbox && <p className="setup-note">{t("Testmodus: Verknüpfungen, Autostart und Eintrag unter „Apps“ werden nur im Testordner angelegt.")}</p>}
        {existing && !status.runningInstalled && (
          <p className="setup-note">
            {t("VisuTeX ist bereits installiert. „Installieren“ ersetzt die vorhandene Version (Update); Einstellungen und Dokumente bleiben erhalten.")}
          </p>
        )}
        <label className="form-field">
          <span>{t("Installationsort")}</span>
          <span className="path-field">
            <input value={dir} onChange={(event) => setDir(event.currentTarget.value)} spellCheck={false} />
            <button type="button" className="small" onClick={() => void chooseDir()}>
              <FolderOpen size={14} /> {t("Ändern …")}
            </button>
          </span>
        </label>
        <div className="setup-options">
          <label className="checkbox">
            <input type="checkbox" checked={desktop} onChange={(event) => setDesktop(event.currentTarget.checked)} />
            {t("Verknüpfung auf dem Desktop")}
          </label>
          <label className="checkbox">
            <input type="checkbox" checked={startMenu} onChange={(event) => setStartMenu(event.currentTarget.checked)} />
            {windows ? t("Eintrag im Startmenü") : t("Eintrag im Anwendungsmenü")}
          </label>
          <label className="checkbox">
            <input type="checkbox" checked={autostart} onChange={(event) => setAutostart(event.currentTarget.checked)} />
            {windows ? t("Mit Windows starten") : t("Beim Anmelden starten")}
          </label>
          <label className="checkbox">
            <input type="checkbox" checked={checkUpdates} onChange={(event) => onCheckUpdatesChange(event.currentTarget.checked)} />
            {t("Beim Start nach Updates suchen (GitHub)")}
          </label>
        </div>
        {windows && <p className="muted small">{t("Ohne Administratorrechte; unter „Einstellungen → Apps“ lässt sich VisuTeX wieder entfernen.")}</p>}
        {error && <p className="setup-error">{error}</p>}
        <div className="setup-actions">
          <button type="button" onClick={() => void portable()} disabled={busy}>{t("Ohne Installation verwenden")}</button>
          <button type="button" className="primary" onClick={() => void install()} disabled={busy || !dir.trim()}>
            {busy ? t("Wird installiert …") : t("Installieren und starten")}
          </button>
        </div>
      </div>
    </div>
  );
}

type UpdateProps = { info: UpdateInfo; onSkip: () => void; onClose: () => void };

/** Neue Version anzeigen, herunterladen und installieren (Neustart). */
export function UpdateDialog({ info, onSkip, onClose }: UpdateProps) {
  const t = useT();
  const [progress, setProgress] = useState<{ loaded: number; total: number } | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let stop: (() => void) | undefined;
    void api.onUpdateProgress(setProgress).then((unlisten) => (stop = unlisten));
    return () => stop?.();
  }, []);
  const install = async () => {
    if (!info.asset) return;
    setError("");
    setProgress({ loaded: 0, total: info.asset.size });
    try {
      await api.updateInstall(info.asset);
    } catch (failure) {
      setError(errorText(failure));
      setProgress(null);
    }
  };
  const percent = progress && progress.total > 0 ? Math.round((progress.loaded / progress.total) * 100) : 0;
  return (
    <div className="setup-screen overlay" role="dialog" aria-modal="true" aria-label={t("Update verfügbar")}>
      <div className="setup-card">
        <h1>
          <Download size={22} /> {t("Update verfügbar")}
        </h1>
        <p>
          {t("Installiert")}: <strong>{info.current}</strong> · {t("Neu")}: <strong>{info.latest}</strong>
          {info.prerelease && <span className="setup-version">{t("Vorabversion")}</span>}
        </p>
        {info.notes && <pre className="update-notes">{info.notes}</pre>}
        {!info.asset && <p className="muted">{t("Für diese Installation gibt es keine automatische Aktualisierung – bitte die neue Version von der Release-Seite laden.")}</p>}
        {progress && (
          <div className="update-progress" aria-label={t("Fortschritt")}>
            <div style={{ width: `${percent}%` }} />
            <span>{percent < 100 ? `${t("Lädt …")} ${percent} %` : t("Wird installiert …")}</span>
          </div>
        )}
        {error && <p className="setup-error">{error}</p>}
        <div className="setup-actions">
          <button type="button" onClick={() => void openUrl(info.page)}>{t("Release-Seite")}</button>
          <button type="button" onClick={onSkip} disabled={Boolean(progress)}>{t("Diese Version überspringen")}</button>
          <button type="button" onClick={onClose} disabled={Boolean(progress)}>{t("Später")}</button>
          {info.asset && (
            <button type="button" className="primary" onClick={() => void install()} disabled={Boolean(progress)}>
              {t("Jetzt aktualisieren")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
