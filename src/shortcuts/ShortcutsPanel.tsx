/**
 * Reiter „Tastenkürzel“ im Datei-Bereich: alle Befehle mit ihrer Belegung, neue
 * Kombination aufnehmen (nächster Tastendruck), Konflikte anzeigen, zurücksetzen.
 */
import { useEffect, useMemo, useState } from "react";
import { Keyboard, Plus, RotateCcw, Search, X } from "lucide-react";
import {
  SHORTCUT_CATEGORIES,
  SHORTCUT_COMMANDS,
  assignShortcut,
  bindingsFor,
  comboFromEvent,
  comboProblem,
  conflictFor,
  formatCombo,
} from "./shortcuts";
import type { ShortcutCommand, ShortcutOverrides } from "./shortcuts";
import { useLanguage, useT } from "../i18n";

type Props = {
  overrides: ShortcutOverrides;
  onChange: (overrides: ShortcutOverrides) => void;
};

type Pending = { id: string; combo: string; conflict: ShortcutCommand | null; problem: string | null };

export function ShortcutsPanel({ overrides, onChange }: Props) {
  const t = useT();
  const language = useLanguage();
  const [query, setQuery] = useState("");
  const [recording, setRecording] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);

  // Aufnahme: den nächsten Tastendruck (mit Modifikatoren) abfangen.
  useEffect(() => {
    if (!recording) return;
    const listener = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.shiftKey) {
        setRecording(null);
        return;
      }
      const combo = comboFromEvent(event);
      if (!combo) return;
      setRecording(null);
      setPending({ id: recording, combo, conflict: conflictFor(combo, overrides, recording), problem: comboProblem(combo) });
    };
    window.addEventListener("keydown", listener, true);
    return () => window.removeEventListener("keydown", listener, true);
  }, [recording, overrides]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return SHORTCUT_COMMANDS;
    return SHORTCUT_COMMANDS.filter((command) => {
      const combos = bindingsFor(command.id, overrides).map((combo) => formatCombo(combo, language).toLowerCase());
      return (
        t(command.label).toLowerCase().includes(needle) ||
        t(command.category).toLowerCase().includes(needle) ||
        combos.some((combo) => combo.includes(needle))
      );
    });
  }, [query, overrides, language, t]);

  const confirmPending = () => {
    if (!pending || pending.problem) return;
    const current = bindingsFor(pending.id, overrides);
    onChange(assignShortcut(overrides, pending.id, [...current, pending.combo]));
    setPending(null);
  };

  const removeCombo = (id: string, combo: string) => {
    onChange(assignShortcut(overrides, id, bindingsFor(id, overrides).filter((entry) => entry !== combo)));
  };

  const resetCommand = (command: ShortcutCommand) => {
    // Standard wiederherstellen; dabei Standardkombinationen anderen Befehlen wieder abnehmen.
    onChange(assignShortcut(overrides, command.id, command.defaults));
  };

  const modified = (command: ShortcutCommand) => Object.prototype.hasOwnProperty.call(overrides, command.id);

  return (
    <div className="shortcuts-panel">
      <div className="shortcuts-toolbar">
        <label className="shortcuts-search">
          <Search size={16} />
          <input
            type="search"
            value={query}
            placeholder={t("Befehl oder Tastenkürzel suchen …")}
            onChange={(event) => setQuery(event.currentTarget.value)}
            aria-label={t("Befehl oder Tastenkürzel suchen …")}
          />
        </label>
        <button type="button" onClick={() => onChange({})} disabled={Object.keys(overrides).length === 0}>
          <RotateCcw size={16} /> {t("Alle zurücksetzen")}
        </button>
      </div>
      <p className="muted">
        {t("„+“ klicken und die gewünschte Tastenkombination drücken (Esc bricht ab). Kombinationen mit Strg oder Alt bzw. F1–F12; Zeichen, die mit AltGr erzeugt werden (z. B. @ oder €), bleiben Texteingabe.")}
      </p>
      {pending && (
        <div className={`shortcut-pending${pending.problem ? " invalid" : ""}`} role="alert">
          <Keyboard size={18} />
          <span>
            <kbd>{formatCombo(pending.combo, language)}</kbd>{" "}
            {pending.problem
              ? t(pending.problem)
              : pending.conflict
                ? `${t("ist bereits belegt durch")} „${t(pending.conflict.label)}“. ${t("Neu zuweisen?")}`
                : `${t("für")} „${t(SHORTCUT_COMMANDS.find((command) => command.id === pending.id)?.label ?? "")}“ ${t("übernehmen?")}`}
          </span>
          {!pending.problem && (
            <button type="button" className="primary small" onClick={confirmPending}>
              {pending.conflict ? t("Neu zuweisen") : t("Übernehmen")}
            </button>
          )}
          <button type="button" className="small" onClick={() => setPending(null)}>{t("Abbrechen")}</button>
        </div>
      )}
      {SHORTCUT_CATEGORIES.map((category) => {
        const commands = filtered.filter((command) => command.category === category);
        if (commands.length === 0) return null;
        return (
          <section key={category} className="shortcut-category">
            <h2>{t(category)}</h2>
            <table className="shortcut-table">
              <tbody>
                {commands.map((command) => {
                  const combos = bindingsFor(command.id, overrides);
                  return (
                    <tr key={command.id} className={modified(command) ? "modified" : undefined}>
                      <th scope="row">{t(command.label)}</th>
                      <td>
                        <span className="shortcut-chips">
                          {combos.map((combo) => (
                            <span key={combo} className="shortcut-chip">
                              <kbd>{formatCombo(combo, language)}</kbd>
                              <button
                                type="button"
                                className="icon-button tiny"
                                title={t("Tastenkürzel entfernen")}
                                aria-label={`${t("Tastenkürzel entfernen")}: ${formatCombo(combo, language)}`}
                                onClick={() => removeCombo(command.id, combo)}
                              >
                                <X size={12} />
                              </button>
                            </span>
                          ))}
                          {recording === command.id ? (
                            <span className="shortcut-recording">{t("Tastenkombination drücken …")}</span>
                          ) : (
                            <button
                              type="button"
                              className="icon-button tiny"
                              title={t("Tastenkürzel hinzufügen")}
                              aria-label={`${t("Tastenkürzel hinzufügen")}: ${t(command.label)}`}
                              onClick={() => {
                                setPending(null);
                                setRecording(command.id);
                              }}
                            >
                              <Plus size={14} />
                            </button>
                          )}
                        </span>
                      </td>
                      <td className="shortcut-reset">
                        {modified(command) && (
                          <button type="button" className="small" onClick={() => resetCommand(command)} title={t("Standard wiederherstellen")}>
                            <RotateCcw size={14} /> {t("Standard")}
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </section>
        );
      })}
    </div>
  );
}
