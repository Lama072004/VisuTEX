/** Add-ons verwalten und eigene Add-ons erstellen (deklarativ, ohne Code). */
import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FilePlus, FolderOpen, Package, Pencil, Plus, Trash2, Upload } from "lucide-react";
import { api, errorText } from "../api";
import type { AddonManifest, InstalledAddon, Snippet } from "../api";
import { useT } from "../i18n";
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

type EditorProps = { initial: AddonManifest; isNew: boolean; onDone: (saved: boolean) => void };

function AddonEditor({ initial, isNew, onDone }: EditorProps) {
  const t = useT();
  const dialogs = useDialogs();
  const [manifest, setManifest] = useState<AddonManifest>(initial);
  const [preambleText, setPreambleText] = useState(initial.preamble.join("\n"));
  const [newFiles, setNewFiles] = useState<Array<{ source: string; target: string }>>([]);
  const [busy, setBusy] = useState(false);
  const set = (patch: Partial<AddonManifest>) => setManifest((current) => ({ ...current, ...patch }));
  const setSnippet = (index: number, patch: Partial<Snippet>) =>
    set({ snippets: manifest.snippets.map((snippet, i) => (i === index ? { ...snippet, ...patch } : snippet)) });
  const inRibbon = (id: string) => manifest.ribbon.some((entry) => entry.snippet === id);

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
      await api.addonsSave(
        {
          ...manifest,
          id,
          snippets,
          ribbon: manifest.ribbon.filter((entry) => snippets.some((snippet) => snippet.id === entry.snippet)),
          preamble: preambleText.split("\n").map((line) => line.trimEnd()).filter((line) => line.trim()),
        },
        newFiles,
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
          {newFiles.map((file, index) => (
            <li key={file.source}>
              <code>{file.target}</code> <small>{t("(neu)")}</small>
              <button type="button" className="icon-button" onClick={() => setNewFiles(newFiles.filter((_, i) => i !== index))} aria-label={t("Entfernen")}>
                <Trash2 size={14} />
              </button>
            </li>
          ))}
          {manifest.texFiles.length + newFiles.length === 0 && <li className="muted">{t("Keine – z. B. eigene .sty-Pakete oder Logos.")}</li>}
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
  const [editing, setEditing] = useState<{ manifest: AddonManifest; isNew: boolean } | null>(null);
  const [error, setError] = useState("");
  const reload = async () => {
    try {
      setAddons(await api.addonsList());
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

  const install = async (folder: boolean) => {
    const selection = folder
      ? await open({ directory: true, title: t("Add-on-Ordner wählen") })
      : await open({ title: t("Add-on installieren"), filters: [{ name: t("Add-on (ZIP)"), extensions: ["zip"] }] });
    if (!selection || Array.isArray(selection)) return;
    try {
      const manifest = await api.addonsInstall(selection);
      await changed();
      await dialogs.alert(t("Add-on installiert"), `${manifest.name} ${manifest.version}`);
    } catch (installError) {
      await dialogs.alert(t("Installation fehlgeschlagen"), errorText(installError));
    }
  };

  if (editing) {
    return (
      <AddonEditor
        initial={editing.manifest}
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
        <button type="button" onClick={() => void install(false)}><Upload size={15} /> {t("Aus ZIP installieren …")}</button>
        <button type="button" onClick={() => void install(true)}><FolderOpen size={15} /> {t("Aus Ordner installieren …")}</button>
        <button type="button" className="primary" onClick={() => setEditing({ manifest: emptyManifest(), isNew: true })}><Plus size={15} /> {t("Eigenes Add-on erstellen")}</button>
      </div>
      {error && <p className="form-error">{error}</p>}
      <p className="muted">{t("Add-ons ergänzen Präambelzeilen, TeX-Dateien, Bausteine im Menüband und Vorlagen. Sie enthalten keinen ausführbaren Code.")}</p>
      <ul className="addon-list">
        {addons.map((addon) => (
          <li key={addon.manifest.id} className={addon.enabled ? "enabled" : ""}>
            <Package size={22} className="addon-list-icon" />
            <div className="addon-info">
              <strong>{addon.manifest.name}</strong> <small>{addon.manifest.version}{addon.builtin ? ` · ${t("mitgeliefert")}` : ""}{addon.manifest.author ? ` · ${addon.manifest.author}` : ""}</small>
              <p>{addon.manifest.description}</p>
              <small className="muted">
                {addon.manifest.snippets.length} {t("Bausteine")} · {addon.manifest.preamble.length} {t("Präambelzeilen")} · {addon.manifest.texFiles.length} {t("TeX-Dateien")}
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
              <button type="button" className="icon-button" title={t("Bearbeiten")} onClick={() => setEditing({ manifest: addon.manifest, isNew: false })}>
                <Pencil size={16} />
              </button>
            )}
            <button
              type="button"
              className="icon-button"
              title={t("Als ZIP exportieren …")}
              onClick={async () => {
                const path = await save({ title: t("Add-on exportieren"), defaultPath: `${addon.manifest.id}.zip`, filters: [{ name: "ZIP", extensions: ["zip"] }] });
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
    </div>
  );
}
