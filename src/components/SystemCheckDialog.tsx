/**
 * Systemprüfung (beim Start und über Datei → Info): zeigt, was mitgeliefert ist,
 * was auf dem System gefunden wurde und bietet für Fehlendes eine Lösung an –
 * inklusive Auswahl des Speicherorts.
 */
import { CircleAlert, CircleCheck, Info, RefreshCw, TriangleAlert } from "lucide-react";
import type { SystemCheck } from "../api";
import { useT } from "../i18n";
import { Modal } from "./Dialogs";

type Props = {
  check: SystemCheck | null;
  busy: boolean;
  onRecheck: () => void;
  onChooseBundle: () => void;
  onChoosePackageDir: () => void;
  onEnableOnline: () => void;
  onClose: () => void;
};

export function SystemCheckDialog({ check, busy, onRecheck, onChooseBundle, onChoosePackageDir, onEnableOnline, onClose }: Props) {
  const t = useT();
  const icon = (status: string) =>
    status === "ok" ? <CircleCheck size={18} className="status-ok" />
    : status === "warning" ? <TriangleAlert size={18} className="status-warning" />
    : status === "error" ? <CircleAlert size={18} className="status-failed" />
    : <Info size={18} className="status-info" />;
  const actionButton = (action: string | null) => {
    if (action === "choose-bundle") return <button type="button" className="small" onClick={onChooseBundle}>{t("Bundle-Datei wählen …")}</button>;
    if (action === "choose-package-dir") return <button type="button" className="small" onClick={onChoosePackageDir}>{t("Speicherort wählen …")}</button>;
    if (action === "enable-online") return <button type="button" className="small" onClick={onEnableOnline}>{t("Online-Nachladen erlauben")}</button>;
    return null;
  };
  return (
    <Modal
      title={t("Systemprüfung")}
      wide
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onRecheck} disabled={busy}>
            <RefreshCw size={14} /> {t("Erneut prüfen")}
          </button>
          <button type="button" className="primary" onClick={onClose}>{t("Schließen")}</button>
        </>
      }
    >
      <p className="form-description">
        {t("VisuTeX bringt TeX-Engine, TeX-Pakete und Schriften mit. Hier siehst du, was auf diesem System gefunden wurde und was fehlt.")}
      </p>
      {!check ? (
        <p className="muted">{t("Prüfe …")}</p>
      ) : (
        <ul className="system-check-list">
          {check.items.map((item) => (
            <li key={item.id} className={`status-${item.status}`}>
              {icon(item.status)}
              <div>
                <strong>{t(item.name)}</strong>
                <small>{item.detail}</small>
              </div>
              {actionButton(item.action)}
            </li>
          ))}
        </ul>
      )}
      {check && (
        <p className="muted small-print">
          {t("Speicherort für nachgeladene TeX-Pakete:")} <code>{check.packageCacheDir}</code>
        </p>
      )}
    </Modal>
  );
}
