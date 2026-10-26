/**
 * Add-ons verwalten und eigene Add-ons erstellen (deklarativ, ohne Code).
 * Installiert wird über die Sicherheitsabfrage (`AddonInstallDialog`) – aus einer Add-on-Datei
 * (`.vtxaddon`), einem ZIP oder einem Ordner. Anleitung für Ersteller: docs/ADDONS.md.
 */
import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Download, FilePlus, FolderOpen, FolderSearch, ImagePlus, Package, Pencil, Plus, RefreshCw, Trash2, Upload, X } from "lucide-react";
import { api, errorText } from "../api";
import type { AddonManifest, AddonProblem, InstalledAddon, Snippet } from "../api";
import { useT } from "../i18n";
import { AddonIcon, isSvgIcon } from "./AddonIcon";
import { AddonInstallDialog } from "./AddonInstallDialog";
import { useDialogs } from "./Dialogs";

const emptyManifest = (): AddonManifest => ({
  id: "",
  name: "",
  version: "1.0.0",
  author: "",
  description: "",
  texFiles: [],
  preamble: [],
  snippets: [],
  ribbon: [],
  templates: [],
});

function slug(text: string) {
  return text
    .toLowerCase()
    .replace(/ä/g, "ae").replace(/ö/g, "oe").replace(/ü/g, "ue").replace(/ß/g, "ss")
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}

/** Dateiname einer gewählten SVG-Datei → Pfad im Add-on (`icons/<name>.svg`). */
function iconTarget(source: string) {
  const name = slug((source.split(/[\\/]/).pop() ?? "symbol").replace(/\.svg$/i, "")) || "symbol";
  return `icons/${name}.svg`;
}

type EditorProps = { initial: AddonManifest; icons: Record<string, string>; isNew: boolean; onDone: (saved: boolean) => void };

function AddonEditor({ initial, icons, isNew, onDone }: EditorProps) {
  const t = useT();
  const dialogs = useDialogs();
  const [manifest, setManifest] = useState<AddonManifest>(initial);
  const [preambleText, setPreambleText] = useState(initial.preamble.join("\n"));
  const [newFiles, setNewFiles] = useState<Array<{ source: string; target: string }>>([]);
  const [iconUrls, setIconUrls] = useState<Record<string, string>>(icons);
  const [busy, setBusy] = useState(false);
  const set = (patch: Partial<AddonManifest>) => setManifest((current) => ({ ...current, ...patch }));
  const setSnippet = (index: number, patch: Partial<Snippet>) =>
    set({ snippets: manifest.snippets.map((snippet, i) => (i === index ? { ...snippet, ...patch } : snippet)) });
  const inRibbon = (id: string) => manifest.ribbon.some((entry) => entry.snippet === id);
  const ribbonIcon = (id: string) => manifest.ribbon.find((entry) => entry.snippet === id)?.icon ?? "";
  const setRibbonIcon = (id: string, icon: string) =>
    setManifest((current) => ({ ...current, ribbon: current.ribbon.map((entry) => (entry.snippet === id ? { ...entry, icon } : entry)) }));

  /** SVG-Datei wählen, in Rust prüfen lassen und zum Kopieren vormerken. */
  const chooseIcon = async (): Promise<string | null> => {
    const selection = await open({ title: t("SVG-Symbol wählen"), filters: [{ name: t("SVG-Grafik"), extensions: ["svg"] }] });
    if (!selection || Array.isArray(selection)) return null;
    try {
      const url = await api.addonsIconPreview(selection);
      const target = iconTarget(selection);
      setNewFiles((current) => [...current.filter((file) => file.target !== target), { source: selection, target }]);
      setIconUrls((current) => ({ ...current, [target]: url }));
      return target;
    } catch (iconError) {
      await dialogs.alert(t("SVG-Symbol wählen"), errorText(iconError));
      return null;
    }
  };

  const addFiles = async () => {
    const selection = await open({
      multiple: true,
      title: t("TeX-Dateien hinzufügen"),
      filters: [{ name: t("TeX-Dateien"), extensions: ["sty", "cls", "tex", "bst", "bib", "def", "cfg", "png", "jpg", "pdf"] }],
    });
    if (!selection) return;
    const paths = Array.isArray(selection) ? selection : [selection];
    setNewFiles((current) => [
      ...current,
      ...paths.map((source) => ({ source, target: `tex/${source.split(/[\\/]/).pop() ?? "datei"}` })),
    ]);
  };

  const submit = async () => {
    const id = manifest.id || slug(manifest.name);
    if (!id) {
      await dialogs.alert(t("Add-on speichern"), t("Bitte einen Namen für das Add-on angeben."));
      return;
    }
    setBusy(true);
    try {
      const snippets = manifest.snippets.map((snippet, index) => ({ ...snippet, id: snippet.id || `snippet-${index + 1}` }));
      const ribbon = manifest.ribbon.filter((entry) => snippets.some((snippet) => snippet.id === entry.snippet));
      // Nur noch verwendete SVG-Symbole kopieren
      const usedIcons = new Set([manifest.icon ?? "", ...ribbon.map((entry) => entry.icon)]);
      await api.addonsSave(
        {
          ...manifest,
          id,
          snippets,
          ribbon,
          preamble: preambleText.split("\n").map((line) => line.trimEnd()).filter((line) => line.trim()),
        },
        newFiles.filter((file) => !isSvgIcon(file.target) || usedIcons.has(file.target)),
      );
      onDone(true);
    } catch (error) {
      await dialogs.alert(t("Add-on speichern"), errorText(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="addon-editor">
      <h3>{isNew ? t("Neues Add-on erstellen") : `${t("Add-on bearbeiten")}: ${manifest.name}`}</h3>
      <div className="form-grid two-columns">
        <label className="form-field">
          <span>{t("Name")}</span>
          <input value={manifest.name} onChange={(event) => set({ name: event.currentTarget.value, ...(isNew ? { id: slug(event.currentTarget.value) } : {}) })} />
        </label>
        <label className="form-field">
          <span>{t("ID")}</span>
          <input value={manifest.id} disabled={!isNew} onChange={(event) => set({ id: slug(event.currentTarget.value) })} />
        </label>
        <label className="form-field">
          <span>{t("Version")}</span>
          <input value={manifest.version} onChange={(event) => set({ version: event.currentTarget.value })} />
        </label>
        <label className="form-field">
          <span>{t("Autor/in")}</span>
          <input value={manifest.author} onChange={(event) => set({ author: event.currentTarget.value })} />
        </label>
        <label className="form-field span-2">
          <span>{t("Beschreibung")}</span>
          <input value={manifest.description} onChange={(event) => set({ description: event.currentTarget.value })} />
        </label>
        <div className="form-field span-2">
          <span>{t("Symbol des Add-ons (SVG)")}</span>
          <div className="icon-picker">
            <span className="icon-preview">
              <AddonIcon icon={manifest.icon ?? ""} icons={iconUrls} fallback={<Package size={20} />} className="addon-list-icon" />
            </span>
            <button
              type="button"
              className="small"
              onClick={async () => {
                const target = await chooseIcon();
                if (target) set({ icon: target });
              }}
            >
              <ImagePlus size={14} /> {t("SVG wählen …")}
            </button>
            {manifest.icon && (
              <>
                <code>{manifest.icon}</code>
                <button type="button" className="icon-button" onClick={() => set({ icon: "" })} aria-label={t("Entfernen")}>
                  <X size={14} />
                </button>
              </>
            )}
          </div>
        </div>
        <label className="form-field span-2">
          <span>{t("Präambelzeilen (eine je Zeile, z. B. \\usepackage{siunitx})")}</span>
          <textarea className="code-input" rows={4} value={preambleText} spellCheck={false} onChange={(event) => setPreambleText(event.currentTarget.value)} />
        </label>
      </div>
      <div className="addon-section">
        <div className="addon-section-header">
          <strong>{t("TeX-Dateien")}</strong>
          <button type="button" className="small" onClick={() => void addFiles()}>
            <FilePlus size={14} /> {t("Hinzufügen …")}
          </button>
        </div>
        <ul className="plain-list">
          {manifest.texFiles.map((file) => (
            <li key={file}>
              <code>{file}</code>
              <button type="button" className="icon-button" onClick={() => set({ texFiles: manifest.texFiles.filter((entry) => entry !== file) })} aria-label={t("Entfernen")}>
                <Trash2 size={14} />
              </button>
            </li>
          ))}
          {newFiles.map((file, index) =>
            isSvgIcon(file.target) ? null : (
              <li key={file.source}>
                <code>{file.target}</code> <small>{t("(neu)")}</small>
                <button type="button" className="icon-button" onClick={() => setNewFiles(newFiles.filter((_, i) => i !== index))} aria-label={t("Entfernen")}>
                  <Trash2 size={14} />
                </button>
              </li>
            ),
          )}
          {manifest.texFiles.length + newFiles.filter((file) => !isSvgIcon(file.target)).length === 0 && (
            <li className="muted">{t("Keine – z. B. eigene .sty-Pakete oder Logos.")}</li>
          )}
        </ul>
      </div>
      <div className="addon-section">
        <div className="addon-section-header">
          <strong>{t("Bausteine (Snippets)")}</strong>
          <button
            type="button"
            className="small"
            onClick={() => set({ snippets: [...manifest.snippets, { id: `snippet-${manifest.snippets.length + 1}`, name: "", group: "", kind: "inline", latex: "" }] })}
          >
            <Plus size={14} /> {t("Baustein")}
          </button>
        </div>
        {manifest.snippets.map((snippet, index) => (
          <div key={index} className="snippet-row">
            <input placeholder={t("Name")} value={snippet.name} onChange={(event) => setSnippet(index, { name: event.currentTarget.value })} />
            <select value={snippet.kind} onChange={(event) => setSnippet(index, { kind: event.currentTarget.value as Snippet["kind"] })}>
              <option value="inline">{t("LaTeX im Text")}</option>
              <option value="block">{t("LaTeX-Block")}</option>
              <option value="inlineMath">{t("Inline-Formel")}</option>
              <option value="math">{t("Formel")}</option>
              <option value="tikz">{t("TikZ-Zeichnung")}</option>
            </select>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={inRibbon(snippet.id)}
                onChange={(event) =>
                  set({
                    ribbon: event.currentTarget.checked
                      ? [...manifest.ribbon, { group: manifest.name || t("Add-on"), label: snippet.name || snippet.id, icon: "", snippet: snippet.id }]
                      : manifest.ribbon.filter((entry) => entry.snippet !== snippet.id),
                  })
                }
              />
              {t("Im Menüband")}
            </label>
            <button type="button" className="icon-button" onClick={() => set({ snippets: manifest.snippets.filter((_, i) => i !== index), ribbon: manifest.ribbon.filter((entry) => entry.snippet !== snippet.id) })} aria-label={t("Entfernen")}>
              <Trash2 size={14} />
            </button>
            <textarea className="code-input span-all" rows={2} spellCheck={false} placeholder="\SI{230}{\volt}" value={snippet.latex} onChange={(event) => setSnippet(index, { latex: event.currentTarget.value })} />
            {inRibbon(snippet.id) && (
              <div className="icon-picker span-all">
                <span className="muted">{t("Symbol im Menüband")}</span>
                <span className="icon-preview">
                  <AddonIcon icon={ribbonIcon(snippet.id)} icons={iconUrls} />
                </span>
                <input
                  className="icon-text"
                  maxLength={4}
                  placeholder="Ω"
                  title={t("Kurzer Text (höchstens 4 Zeichen)")}
                  value={isSvgIcon(ribbonIcon(snippet.id)) ? "" : ribbonIcon(snippet.id)}
                  onChange={(event) => setRibbonIcon(snippet.id, event.currentTarget.value)}
                />
                <button
                  type="button"
                  className="small"
                  onClick={async () => {
                    const target = await chooseIcon();
                    if (target) setRibbonIcon(snippet.id, target);
                  }}
                >
                  <ImagePlus size={14} /> {t("SVG wählen …")}
                </button>
                {isSvgIcon(ribbonIcon(snippet.id)) && <code>{ribbonIcon(snippet.id)}</code>}
              </div>
            )}
          </div>
        ))}
      </div>
      <div className="button-row">
        <button type="button" onClick={() => onDone(false)}>{t("Abbrechen")}</button>
        <button type="button" className="primary" disabled={busy} onClick={() => void submit()}>{t("Add-on speichern")}</button>
      </div>
    </div>
  );
}

export function AddonManager({ onChanged }: { onChanged: () => void }) {
  const t = useT();
  const dialogs = useDialogs();
  const [addons, setAddons] = useState<InstalledAddon[]>([]);
  const [problems, setProblems] = useState<AddonProblem[]>([]);
  const [editing, setEditing] = useState<{ manifest: AddonManifest; icons: Record<string, string>; isNew: boolean } | null>(null);
  const [installPath, setInstallPath] = useState<string | null>(null);
  const [error, setError] = useState("");
  const reload = async () => {
    try {
      setAddons(await api.addonsList());
      setProblems(await api.addonsProblems());
      setError("");
    } catch (loadError) {
      setError(errorText(loadError));
    }
  };
  useEffect(() => {
    void reload();
  }, []);
  const changed = async () => {
    await reload();
    onChanged();
  };

  /** Datei bzw. Ordner wählen → Sicherheitsabfrage mit dem Inhalt → installieren. */
  const install = async (folder: boolean) => {
    const selection = folder
      ? await open({ directory: true, title: t("Add-on-Ordner wählen") })
      : await open({
          title: t("Add-on installieren"),
          filters: [
            { name: t("VisuTeX-Add-on"), extensions: ["vtxaddon", "zip"] },
            { name: "ZIP", extensions: ["zip"] },
          ],
        });
    if (!selection || Array.isArray(selection)) return;
    setInstallPath(selection);
  };

  if (editing) {
    return (
      <AddonEditor
        initial={editing.manifest}
        icons={editing.icons}
        isNew={editing.isNew}
        onDone={(saved) => {
          setEditing(null);
          if (saved) void changed();
        }}
      />
    );
  }

  return (
    <div className="addon-manager">
      <div className="button-row left">
        <button type="button" onClick={() => void install(false)}><Download size={15} /> {t("Aus Datei installieren …")}</button>
        <button type="button" onClick={() => void install(true)}><FolderOpen size={15} /> {t("Aus Ordner installieren …")}</button>
        <button type="button" className="primary" onClick={() => setEditing({ manifest: emptyManifest(), icons: {}, isNew: true })}><Plus size={15} /> {t("Eigenes Add-on erstellen")}</button>
      </div>
      <div className="button-row left">
        <button
          type="button"
          className="small"
          title={t("Eigene Add-ons hier als Ordner ablegen oder bearbeiten, danach „Neu laden“.")}
          onClick={() => void api.addonsOpenFolder().catch((openError) => dialogs.alert(t("Add-on-Ordner"), errorText(openError)))}
        >
          <FolderSearch size={14} /> {t("Add-on-Ordner öffnen")}
        </button>
        <button type="button" className="small" onClick={() => void changed()}>
          <RefreshCw size={14} /> {t("Neu laden")}
        </button>
      </div>
      {error && <p className="form-error">{error}</p>}
      <p className="muted">
        {t("Add-ons ergänzen Präambelzeilen, TeX-Dateien, Bausteine im Menüband und Vorlagen. Sie enthalten keinen ausführbaren Code.")}{" "}
        {t("Add-on-Dateien (.vtxaddon) lassen sich auch per Doppelklick oder über Datei → Öffnen installieren.")}{" "}
        {t("Eine Anleitung zum Erstellen eigener Add-ons steht in docs/ADDONS.md.")}
      </p>
      {problems.length > 0 && (
        <ul className="addon-problems">
          {problems.map((problem) => (
            <li key={problem.folder}>
              <AlertTriangle size={15} />
              <span>
                <strong>{t("Wird nicht geladen:")}</strong> <code>{problem.folder}</code>
                <br />
                {problem.error}
              </span>
            </li>
          ))}
        </ul>
      )}
      <ul className="addon-list">
        {addons.map((addon) => (
          <li key={addon.manifest.id} className={addon.enabled ? "enabled" : ""}>
            <AddonIcon icon={addon.manifest.icon ?? ""} icons={addon.icons} fallback={<Package size={22} />} className="addon-list-icon" />
            <div className="addon-info">
              <strong>{addon.manifest.name}</strong> <small>{addon.manifest.version}{addon.builtin ? ` · ${t("mitgeliefert")}` : ""}{addon.manifest.author ? ` · ${addon.manifest.author}` : ""}</small>
              <p>{addon.manifest.description}</p>
              <small className="muted">
                {addon.manifest.snippets.length} {addon.manifest.snippets.length === 1 ? t("Baustein") : t("Bausteine")} ·{" "}
                {addon.manifest.preamble.length} {addon.manifest.preamble.length === 1 ? t("Präambelzeile") : t("Präambelzeilen")} ·{" "}
                {addon.manifest.texFiles.length} {addon.manifest.texFiles.length === 1 ? t("TeX-Datei") : t("TeX-Dateien")}
              </small>
            </div>
            <label className="switch" title={addon.enabled ? t("Deaktivieren") : t("Aktivieren")}>
              <input
                type="checkbox"
                checked={addon.enabled}
                onChange={async (event) => {
                  try {
                    await api.addonsSetEnabled(addon.manifest.id, event.currentTarget.checked);
                    await changed();
                  } catch (toggleError) {
                    await dialogs.alert(t("Add-on"), errorText(toggleError));
                  }
                }}
              />
              <span />
            </label>
            {!addon.builtin && (
              <button type="button" className="icon-button" title={t("Bearbeiten")} onClick={() => setEditing({ manifest: addon.manifest, icons: addon.icons, isNew: false })}>
                <Pencil size={16} />
              </button>
            )}
            <button
              type="button"
              className="icon-button"
              title={t("Als Add-on-Datei exportieren …")}
              onClick={async () => {
                const path = await save({
                  title: t("Add-on exportieren"),
                  defaultPath: `${addon.manifest.id}-${addon.manifest.version}.vtxaddon`,
                  filters: [{ name: t("VisuTeX-Add-on"), extensions: ["vtxaddon"] }],
                });
                if (!path) return;
                try {
                  await api.addonsExport(addon.manifest.id, path);
                } catch (exportError) {
                  await dialogs.alert(t("Export fehlgeschlagen"), errorText(exportError));
                }
              }}
            >
              <Upload size={16} />
            </button>
            {!addon.builtin && (
              <button
                type="button"
                className="icon-button danger"
                title={t("Entfernen")}
                onClick={async () => {
                  const confirmed = await dialogs.confirm({ title: t("Add-on entfernen"), message: `${addon.manifest.name}?`, confirmLabel: t("Entfernen"), danger: true });
                  if (confirmed !== true) return;
                  try {
                    await api.addonsRemove(addon.manifest.id);
                    await changed();
                  } catch (removeError) {
                    await dialogs.alert(t("Add-on"), errorText(removeError));
                  }
                }}
              >
                <Trash2 size={16} />
              </button>
            )}
          </li>
        ))}
        {addons.length === 0 && <li className="muted">{t("Keine Add-ons installiert.")}</li>}
      </ul>
      {installPath && (
        <AddonInstallDialog
          path={installPath}
          onClose={() => setInstallPath(null)}
          onInstalled={(manifest) => {
            setInstallPath(null);
            void changed();
            void dialogs.alert(t("Add-on installiert"), `${manifest.name} ${manifest.version}`);
          }}
        />
      )}
    </div>
  );
}
