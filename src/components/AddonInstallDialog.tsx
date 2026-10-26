/**
 * Sicherheitsabfrage vor dem Installieren eines Add-ons (wie bei Office-Add-ins): zeigt, was die
 * Add-on-Datei (`.vtxaddon`), das ZIP bzw. der Ordner mitbringt – geprüft in Rust, noch nicht
 * installiert. Geöffnet per Doppelklick auf die Datei, über Datei → Öffnen oder die Add-on-Verwaltung.
 */
import { useEffect, useRef, useState } from "react";
import { AlertTriangle, Package, Puzzle, ShieldCheck } from "lucide-react";
import { api, errorText } from "../api";
import type { AddonManifest, AddonPreview } from "../api";
import { useT } from "../i18n";
import { AddonIcon } from "./AddonIcon";

type Props = {
  path: string;
  onInstalled: (manifest: AddonManifest) => void;
  onClose: () => void;
};

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function AddonInstallDialog({ path, onInstalled, onClose }: Props) {
  const t = useT();
  const [preview, setPreview] = useState<AddonPreview | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const primary = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    let cancelled = false;
    setPreview(null);
    setError("");
    api
      .addonsInspect(path)
      .then((result) => !cancelled && setPreview(result))
      .catch((failure) => !cancelled && setError(errorText(failure)));
    return () => {
      cancelled = true;
    };
  }, [path]);
  useEffect(() => primary.current?.focus(), [preview]);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => event.key === "Escape" && !busy && onClose();
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [busy, onClose]);

  const install = async () => {
    setBusy(true);
    setError("");
    try {
      onInstalled(await api.addonsInstall(path));
    } catch (failure) {
      setError(errorText(failure));
      setBusy(false);
    }
  };

  const manifest = preview?.manifest;
  /** Anzahl mit Einzahl bzw. Mehrzahl */
  const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <div className="modal addon-install-dialog" role="dialog" aria-modal="true" aria-label={t("Add-on installieren")}>
        <div className="modal-header">
          <h2>
            <Puzzle size={18} /> {t("Add-on installieren")}
          </h2>
        </div>
        <div className="modal-body">
          <p className="muted small">
            <code>{fileName(path)}</code>
          </p>
          {!preview && !error && <p className="muted">{t("Add-on wird geprüft …")}</p>}
          {!preview && error && (
            <p className="share-warning">
              <AlertTriangle size={16} />
              <span>
                {t("Diese Datei ist kein gültiges VisuTeX-Add-on:")} {error}
              </span>
            </p>
          )}
          {preview && manifest && (
            <>
              <div className="addon-install-card">
                <span className="icon-preview large">
                  <AddonIcon icon={manifest.icon ?? ""} icons={preview.icons} fallback={<Package size={26} />} className="addon-list-icon" />
                </span>
                <div>
                  <strong>{manifest.name}</strong> <small>{manifest.version}</small>
                  {manifest.author && (
                    <div className="muted small">
                      {t("von")} {manifest.author}
                    </div>
                  )}
                  {manifest.description && <p>{manifest.description}</p>}
                </div>
              </div>
              <p className="addon-install-heading">{t("Das Add-on bringt mit:")}</p>
              <ul className="addon-install-contents">
                <li>
                  {count(manifest.ribbon.length, t("Schaltfläche im Menüband (Reiter „Add-ons“)"), t("Schaltflächen im Menüband (Reiter „Add-ons“)"))}
                  {manifest.ribbon.length > 0 && (
                    <span className="addon-install-ribbon">
                      {manifest.ribbon.map((entry) => (
                        <span key={`${entry.snippet}-${entry.label}`} title={entry.label}>
                          <AddonIcon icon={entry.icon} icons={preview.icons} /> {entry.label}
                        </span>
                      ))}
                    </span>
                  )}
                </li>
                <li>{count(manifest.snippets.length, t("Baustein"), t("Bausteine"))}</li>
                <li>
                  {count(manifest.templates.length, t("Vorlage unter Datei → Neu"), t("Vorlagen unter Datei → Neu"))}
                  {manifest.templates.length > 0 && `: ${manifest.templates.map((template) => template.name).join(", ")}`}
                </li>
                <li>
                  {count(manifest.texFiles.length, t("TeX-Datei"), t("TeX-Dateien"))}
                  {manifest.texFiles.length > 0 && (
                    <>
                      {": "}
                      <code>{manifest.texFiles.map(fileName).join(", ")}</code>
                    </>
                  )}
                </li>
                <li>
                  {count(manifest.preamble.length, t("Präambelzeile für jedes Dokument"), t("Präambelzeilen für jedes Dokument"))}
                  {manifest.preamble.length > 0 && (
                    <details>
                      <summary>{t("Anzeigen")}</summary>
                      <pre className="addon-install-preamble">{manifest.preamble.join("\n")}</pre>
                    </details>
                  )}
                </li>
                <li className="muted">
                  {count(preview.fileCount, t("Datei"), t("Dateien"))} · {formatSize(preview.size)}
                </li>
              </ul>
              {preview.inPlace ? (
                <p className="muted">{t("Dieses Add-on liegt bereits im Add-on-Ordner – es wird nur geprüft und neu geladen.")}</p>
              ) : preview.installedVersion !== null ? (
                <p className="share-warning">
                  <AlertTriangle size={16} />
                  <span>
                    {preview.installedBuiltin
                      ? `${t("Ein mitgeliefertes Add-on mit derselben ID ist vorhanden – das neue hat Vorrang. Mitgelieferte Version:")} ${preview.installedVersion}`
                      : `${t("Ersetzt die installierte Version")} ${preview.installedVersion}.`}
                  </span>
                </p>
              ) : null}
              <p className="addon-install-trust">
                <ShieldCheck size={16} />
                <span>
                  {t("Add-ons enthalten keinen ausführbaren Code. Die TeX-Dateien laufen aber beim Kompilieren mit – installieren Sie nur Add-ons aus vertrauenswürdigen Quellen.")}
                </span>
              </p>
            </>
          )}
          {preview && error && <p className="setup-error">{error}</p>}
        </div>
        <div className="modal-footer">
          <button type="button" onClick={onClose} disabled={busy}>
            {t("Abbrechen")}
          </button>
          <button ref={primary} type="button" className="primary" onClick={() => void install()} disabled={!preview || busy}>
            {preview?.inPlace ? t("Neu laden") : preview && preview.installedVersion !== null && !preview.installedBuiltin ? t("Ersetzen") : t("Installieren")}
          </button>
        </div>
      </div>
    </div>
  );
}
