/**
 * Zitat einfügen/bearbeiten: Einträge der Projekt-.bib durchsuchen, aus Zotero
 * übernehmen (Better BibTeX oder Zotero-7-Local-API, Anfragen laufen in Rust)
 * oder BibTeX direkt einfügen. Zitiert wird immer mit dem BibTeX-Schlüssel.
 */
import { useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { api, errorText } from "../api";
import type { BibEntry, ZoteroItem, ZoteroStatus } from "../api";
import { useT } from "../i18n";
import { Modal } from "./Dialogs";

export type CitationValue = { keys: string; prenote: string; postnote: string; command: "cite" | "citep" | "citet" };

type Props = {
  projectRoot: string | null;
  bibFile: string;
  initial: CitationValue | null;
  onEnsureBibFile: () => Promise<string | null>;
  onDone: (value: CitationValue | null) => void;
};

type Tab = "library" | "zotero" | "bibtex";

export function CitationDialog({ projectRoot, bibFile, initial, onEnsureBibFile, onDone }: Props) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("library");
  const [entries, setEntries] = useState<BibEntry[]>([]);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string[]>(() => (initial?.keys ?? "").split(",").map((key) => key.trim()).filter(Boolean));
  const [prenote, setPrenote] = useState(initial?.prenote ?? "");
  const [postnote, setPostnote] = useState(initial?.postnote ?? "");
  const [command, setCommand] = useState<CitationValue["command"]>(initial?.command ?? "cite");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [zoteroStatus, setZoteroStatus] = useState<ZoteroStatus | null>(null);
  const [zoteroQuery, setZoteroQuery] = useState("");
  const [zoteroItems, setZoteroItems] = useState<ZoteroItem[]>([]);
  const [zoteroSelected, setZoteroSelected] = useState<string[]>([]);
  const [bibtexText, setBibtexText] = useState("");

  const loadEntries = async (file = bibFile) => {
    if (!projectRoot || !file) {
      setEntries([]);
      return;
    }
    try {
      setEntries(await api.readBib(projectRoot, file));
    } catch (error) {
      setMessage(errorText(error));
    }
  };
  useEffect(() => {
    void loadEntries();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectRoot, bibFile]);

  useEffect(() => {
    if (tab === "zotero" && !zoteroStatus) void api.zoteroStatus().then(setZoteroStatus).catch((error) => setMessage(errorText(error)));
  }, [tab, zoteroStatus]);

  const filtered = useMemo(() => {
    const query = filter.trim().toLowerCase();
    if (!query) return entries;
    return entries.filter((entry) => `${entry.key} ${entry.title} ${entry.author} ${entry.year}`.toLowerCase().includes(query));
  }, [entries, filter]);

  const toggle = (key: string) => setSelected((current) => (current.includes(key) ? current.filter((entry) => entry !== key) : [...current, key]));

  const requireBib = async () => {
    if (!projectRoot) {
      setMessage(t("Kein Projektordner verfügbar."));
      return null;
    }
    const file = bibFile || (await onEnsureBibFile());
    if (!file) setMessage(t("Bitte zuerst eine Literaturdatei festlegen."));
    return file;
  };

  const importZotero = async () => {
    const file = await requireBib();
    if (!file || !projectRoot || zoteroSelected.length === 0) return;
    setBusy(true);
    setMessage("");
    try {
      const sources = new Set(zoteroItems.filter((item) => zoteroSelected.includes(item.id)).map((item) => item.source));
      const keys: string[] = [];
      for (const source of sources) {
        const ids = zoteroItems.filter((item) => item.source === source && zoteroSelected.includes(item.id)).map((item) => item.id);
        const merged = await api.zoteroImport(ids, source, projectRoot, file);
        keys.push(...merged.keys);
        setMessage(`${merged.added.length} ${t("neu in")} ${file}${merged.skipped.length ? `, ${merged.skipped.length} ${t("bereits vorhanden")}` : ""}.`);
      }
      await loadEntries(file);
      setSelected((current) => [...new Set([...current, ...keys])]);
      setZoteroSelected([]);
      setTab("library");
    } catch (error) {
      setMessage(errorText(error));
    } finally {
      setBusy(false);
    }
  };

  const importBibtex = async () => {
    const file = await requireBib();
    if (!file || !projectRoot || !bibtexText.trim()) return;
    setBusy(true);
    try {
      const merged = await api.addBibEntries(projectRoot, file, bibtexText);
      await loadEntries(file);
      setSelected((current) => [...new Set([...current, ...merged.keys])]);
      setMessage(`${merged.added.length} ${t("neu in")} ${file}${merged.skipped.length ? `, ${merged.skipped.length} ${t("bereits vorhanden")}` : ""}.`);
      setBibtexText("");
      setTab("library");
    } catch (error) {
      setMessage(errorText(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={initial ? t("Zitat bearbeiten") : t("Zitat einfügen")}
      wide
      onClose={() => onDone(null)}
      className="citation-dialog"
      footer={
        <>
          {message && <span className="form-message">{message}</span>}
          <button type="button" onClick={() => onDone(null)}>{t("Abbrechen")}</button>
          <button
            type="button"
            className="primary"
            disabled={selected.length === 0}
            onClick={() => onDone({ keys: selected.join(","), prenote: prenote.trim(), postnote: postnote.trim(), command })}
          >
            {initial ? t("Übernehmen") : t("Einfügen")}
          </button>
        </>
      }
    >
      <div className="tab-bar">
        <button type="button" className={tab === "library" ? "active" : ""} onClick={() => setTab("library")}>
          {t("Literaturdatei")} {bibFile ? <small>({bibFile})</small> : null}
        </button>
        <button type="button" className={tab === "zotero" ? "active" : ""} onClick={() => setTab("zotero")}>Zotero</button>
        <button type="button" className={tab === "bibtex" ? "active" : ""} onClick={() => setTab("bibtex")}>{t("BibTeX einfügen")}</button>
      </div>

      {tab === "library" && (
        <div className="citation-library">
          <div className="search-field">
            <Search size={15} />
            <input autoFocus placeholder={t("Autor, Titel, Jahr oder Schlüssel")} value={filter} onChange={(event) => setFilter(event.currentTarget.value)} />
          </div>
          {!bibFile && <p className="muted">{t("Noch keine Literaturdatei festgelegt. Einträge aus Zotero oder als BibTeX hinzufügen – die Datei literatur.bib wird dann angelegt.")}</p>}
          <ul className="entry-list">
            {filtered.map((entry) => (
              <li key={entry.key}>
                <label className={selected.includes(entry.key) ? "selected" : ""}>
                  <input type="checkbox" checked={selected.includes(entry.key)} onChange={() => toggle(entry.key)} />
                  <span className="entry-main">
                    <strong>{entry.title || entry.key}</strong>
                    <small>{[entry.author, entry.year, entry.container].filter(Boolean).join(" · ")}</small>
                  </span>
                  <code>{entry.key}</code>
                </label>
              </li>
            ))}
            {bibFile && filtered.length === 0 && <li className="muted">{t("Keine passenden Einträge.")}</li>}
          </ul>
          <div className="form-grid three-columns">
            <label className="form-field">
              <span>{t("Befehl")}</span>
              <select value={command} onChange={(event) => setCommand(event.currentTarget.value as CitationValue["command"])}>
                <option value="cite">\cite – [1]</option>
                <option value="citep">\citep – (Autor, Jahr)</option>
                <option value="citet">\citet – Autor (Jahr)</option>
              </select>
            </label>
            <label className="form-field">
              <span>{t("Vorsatz (z. B. vgl.)")}</span>
              <input value={prenote} onChange={(event) => setPrenote(event.currentTarget.value)} />
            </label>
            <label className="form-field">
              <span>{t("Seite/Zusatz (z. B. S. 12)")}</span>
              <input value={postnote} onChange={(event) => setPostnote(event.currentTarget.value)} />
            </label>
          </div>
        </div>
      )}

      {tab === "zotero" && (
        <div className="citation-zotero">
          <div className="zotero-status">
            <span className={zoteroStatus?.betterBibtex || zoteroStatus?.localApi ? "ok" : "warn"}>
              {zoteroStatus ? zoteroStatus.message : t("Verbindung wird geprüft …")}
            </span>
            <button type="button" className="icon-button" title={t("Erneut prüfen")} onClick={() => { setZoteroStatus(null); }}>
              <RefreshCw size={15} />
            </button>
          </div>
          <form
            className="search-field"
            onSubmit={async (event) => {
              event.preventDefault();
              setBusy(true);
              setMessage("");
              try {
                setZoteroItems(await api.zoteroSearch(zoteroQuery));
              } catch (error) {
                setMessage(errorText(error));
              } finally {
                setBusy(false);
              }
            }}
          >
            <Search size={15} />
            <input autoFocus placeholder={t("In Zotero suchen (Titel, Autor, Jahr)")} value={zoteroQuery} onChange={(event) => setZoteroQuery(event.currentTarget.value)} />
            <button type="submit" disabled={busy}>{t("Suchen")}</button>
          </form>
          <ul className="entry-list">
            {zoteroItems.map((item) => (
              <li key={`${item.source}-${item.id}`}>
                <label className={zoteroSelected.includes(item.id) ? "selected" : ""}>
                  <input
                    type="checkbox"
                    checked={zoteroSelected.includes(item.id)}
                    onChange={() => setZoteroSelected((current) => (current.includes(item.id) ? current.filter((id) => id !== item.id) : [...current, item.id]))}
                  />
                  <span className="entry-main">
                    <strong>{item.title || item.id}</strong>
                    <small>{[item.authors, item.year, item.itemType].filter(Boolean).join(" · ")}</small>
                  </span>
                  {item.source === "bbt" && <code>{item.id}</code>}
                </label>
              </li>
            ))}
          </ul>
          <div className="button-row">
            <button type="button" className="primary" disabled={busy || zoteroSelected.length === 0} onClick={() => void importZotero()}>
              {t("In Literaturdatei übernehmen und auswählen")}
            </button>
          </div>
        </div>
      )}

      {tab === "bibtex" && (
        <div className="citation-bibtex">
          <textarea
            className="code-input"
            rows={12}
            spellCheck={false}
            placeholder={"@book{knuth1984,\n  author = {Donald E. Knuth},\n  title = {The TeXbook},\n  year = {1984}\n}"}
            value={bibtexText}
            onChange={(event) => setBibtexText(event.currentTarget.value)}
          />
          <div className="button-row">
            <button type="button" className="primary" disabled={busy || !bibtexText.trim()} onClick={() => void importBibtex()}>
              {t("Zur Literaturdatei hinzufügen")}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
