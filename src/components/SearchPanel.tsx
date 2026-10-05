/** Suchen & Ersetzen (visueller Editor). */
import type { Editor } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp, X } from "lucide-react";
import { useT } from "../i18n";
import { searchKey } from "../editor/search";

type Props = { editor: Editor; replace: boolean; onClose: () => void };

export function SearchPanel({ editor, replace, onClose }: Props) {
  const t = useT();
  const [query, setQuery] = useState(() => {
    const { from, to, empty } = editor.state.selection;
    return empty ? "" : editor.state.doc.textBetween(from, to, " ").slice(0, 200);
  });
  const [replacement, setReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [showReplace, setShowReplace] = useState(replace);
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => setShowReplace(replace), [replace]);
  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);
  useEffect(() => {
    editor.commands.setSearch(query, { caseSensitive, wholeWord });
  }, [editor, query, caseSensitive, wholeWord]);
  useEffect(() => () => void editor.commands.clearSearch(), [editor]);

  const stats = useEditorState({
    editor,
    selector: ({ editor: current }) => {
      const state = searchKey.getState(current.state);
      return { count: state?.matches.length ?? 0, current: state?.current ?? -1 };
    },
  });

  return (
    <div
      className="search-panel"
      role="search"
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
        if (event.key === "Enter" && !(event.target instanceof HTMLButtonElement)) {
          event.preventDefault();
          editor.commands.gotoMatch(event.shiftKey ? -1 : 1);
        }
      }}
    >
      <button type="button" className="icon-button" onClick={() => setShowReplace((value) => !value)} aria-label={t("Ersetzen ein-/ausblenden")}>
        {showReplace ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
      </button>
      <div className="search-fields">
        <div className="search-row">
          <input ref={inputRef} placeholder={t("Suchen")} value={query} onChange={(event) => setQuery(event.currentTarget.value)} aria-label={t("Suchen")} />
          <span className="search-count">
            {query ? (stats?.count ? `${(stats.current >= 0 ? stats.current : 0) + 1} ${t("von")} ${stats.count}` : t("Keine Treffer")) : ""}
          </span>
          <button type="button" className={`toggle${caseSensitive ? " active" : ""}`} title={t("Groß-/Kleinschreibung beachten")} onClick={() => setCaseSensitive((value) => !value)}>Aa</button>
          <button type="button" className={`toggle${wholeWord ? " active" : ""}`} title={t("Nur ganze Wörter")} onClick={() => setWholeWord((value) => !value)}>ab|</button>
          <button type="button" className="icon-button" title={t("Vorheriger Treffer (Umschalt+Enter)")} onClick={() => editor.commands.gotoMatch(-1)}>
            <ChevronUp size={16} />
          </button>
          <button type="button" className="icon-button" title={t("Nächster Treffer (Enter)")} onClick={() => editor.commands.gotoMatch(1)}>
            <ChevronDown size={16} />
          </button>
          <button type="button" className="icon-button" title={t("Schließen")} onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        {showReplace && (
          <div className="search-row">
            <input placeholder={t("Ersetzen durch")} value={replacement} onChange={(event) => setReplacement(event.currentTarget.value)} aria-label={t("Ersetzen durch")} />
            <button type="button" className="small" disabled={!stats?.count} onClick={() => editor.chain().replaceCurrent(replacement).gotoMatch(1).run()}>
              {t("Ersetzen")}
            </button>
            <button type="button" className="small" disabled={!stats?.count} onClick={() => editor.commands.replaceAll(replacement)}>
              {t("Alle ersetzen")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
