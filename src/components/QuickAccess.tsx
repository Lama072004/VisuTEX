/**
 * Symbolleiste für den Schnellzugriff (Titelleiste, wie in Office): frei wählbare Befehle
 * neben Speichern/Rückgängig. Anpassen über das ▾-Menü daneben oder *Datei → Optionen*.
 */
import { useEffect, useRef, useState } from "react";
import type { ComponentType } from "react";
import {
  AlignCenter,
  AlignJustify,
  AlignLeft,
  AlignRight,
  ArrowDown,
  ArrowUp,
  Bold,
  BookOpen,
  Braces,
  Check,
  ChevronDown,
  Code,
  Copy,
  Eye,
  FileCode,
  FileDown,
  FilePlus,
  FolderOpen,
  Footprints,
  Heading1,
  Heading2,
  Heading3,
  Image,
  Italic,
  Keyboard,
  Link,
  List,
  ListOrdered,
  ListTree,
  Minus,
  Paintbrush,
  PanelRight,
  PenTool,
  Percent,
  Pilcrow,
  Play,
  Plus,
  Presentation,
  Quote,
  Radical,
  Redo2,
  RemoveFormatting,
  Replace,
  Ruler,
  Save,
  SaveAll,
  ScrollText,
  Search,
  SeparatorHorizontal,
  Settings,
  Share2,
  Sigma,
  SquareCode,
  Strikethrough,
  Subscript,
  Superscript,
  Table,
  Type,
  Underline,
  Undo2,
  Weight,
  ZoomIn,
  ZoomOut,
  Crosshair,
  Files,
} from "lucide-react";
import type { LucideProps } from "lucide-react";
import { useT } from "../i18n";
import { SHORTCUT_CATEGORIES, bindingsFor, formatCombo } from "../shortcuts/shortcuts";
import type { ShortcutOverrides } from "../shortcuts/shortcuts";
import {
  DEFAULT_QUICK_ACCESS,
  QUICK_ACCESS_LIMIT,
  QUICK_ACCESS_SUGGESTIONS,
  moveQuickAccess,
  quickAccessCommands,
  toggleQuickAccess,
} from "../shortcuts/quickAccess";

type Icon = ComponentType<LucideProps>;

/** Symbol je Befehl (Befehle ohne eigenes Symbol erhalten ein neutrales). */
const ICONS: Record<string, Icon> = {
  "file.new": FilePlus,
  "file.open": FolderOpen,
  "file.save": Save,
  "file.saveAs": SaveAll,
  "file.share": Share2,
  "file.exportTex": FileCode,
  "file.exportPdf": FileDown,
  "file.compile": Play,
  "file.options": Settings,
  "file.shortcuts": Keyboard,
  "edit.undo": Undo2,
  "edit.redo": Redo2,
  "edit.find": Search,
  "edit.replace": Replace,
  "edit.formatPainter": Paintbrush,
  "format.bold": Bold,
  "format.italic": Italic,
  "format.underline": Underline,
  "format.strike": Strikethrough,
  "format.subscript": Subscript,
  "format.superscript": Superscript,
  "format.code": Code,
  "format.clear": RemoveFormatting,
  "paragraph.normal": Type,
  "paragraph.heading1": Heading1,
  "paragraph.heading2": Heading2,
  "paragraph.heading3": Heading3,
  "paragraph.alignLeft": AlignLeft,
  "paragraph.alignCenter": AlignCenter,
  "paragraph.alignRight": AlignRight,
  "paragraph.alignJustify": AlignJustify,
  "paragraph.bulletList": List,
  "paragraph.orderedList": ListOrdered,
  "paragraph.quote": Quote,
  "paragraph.codeBlock": SquareCode,
  "insert.pageBreak": SeparatorHorizontal,
  "insert.inlineMath": Radical,
  "insert.blockMath": Sigma,
  "insert.link": Link,
  "insert.footnote": Footprints,
  "insert.citation": BookOpen,
  "insert.crossReference": Crosshair,
  "insert.table": Table,
  "insert.image": Image,
  "insert.sketch": PenTool,
  "insert.rawLatex": Braces,
  "insert.quantity": Weight,
  "insert.codeListing": ScrollText,
  "view.toggleCode": Eye,
  "view.togglePdf": PanelRight,
  "view.toggleOutline": ListTree,
  "view.togglePaged": Files,
  "view.toggleRuler": Ruler,
  "view.formattingMarks": Pilcrow,
  "view.zoomIn": ZoomIn,
  "view.zoomOut": ZoomOut,
  "view.zoomReset": Percent,
  "view.slides": Presentation,
};

export function commandIcon(id: string): Icon {
  return ICONS[id] ?? Copy;
}

function commandLabel(id: string): string {
  return quickAccessCommands().find((command) => command.id === id)?.label ?? id;
}

type BarProps = {
  items: string[];
  shortcuts: ShortcutOverrides;
  language: string;
  /** Befehl gerade nicht anwendbar (z. B. Rückgängig in der Code-Ansicht) */
  isDisabled: (id: string) => boolean;
  onRun: (id: string) => void;
  onChange: (items: string[]) => void;
  onMoreOptions: () => void;
};

/** Schaltflächen in der Titelleiste plus ▾-Menü zum Anpassen. */
export function QuickAccessBar({ items, shortcuts, language, isDisabled, onRun, onChange, onMoreOptions }: BarProps) {
  const t = useT();
  const [menu, setMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!menu) return;
    const close = (event: MouseEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) setMenu(false);
    };
    const escape = (event: KeyboardEvent) => event.key === "Escape" && setMenu(false);
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", escape);
    };
  }, [menu]);
  const title = (id: string) => {
    const combo = bindingsFor(id, shortcuts)[0];
    return combo ? `${t(commandLabel(id))} (${formatCombo(combo, language)})` : t(commandLabel(id));
  };
  const suggestions = [...new Set([...QUICK_ACCESS_SUGGESTIONS, ...items])];
  return (
    <div className="quick-access" role="toolbar" aria-label={t("Symbolleiste für den Schnellzugriff")}>
      {items.map((id) => {
        const IconComponent = commandIcon(id);
        return (
          <button key={id} type="button" className="icon-button" title={title(id)} aria-label={t(commandLabel(id))} disabled={isDisabled(id)} onClick={() => onRun(id)}>
            <IconComponent size={17} />
          </button>
        );
      })}
      <div className="quick-access-menu-anchor" ref={menuRef}>
        <button
          type="button"
          className="icon-button quick-access-more"
          title={t("Symbolleiste für den Schnellzugriff anpassen")}
          aria-label={t("Symbolleiste für den Schnellzugriff anpassen")}
          aria-expanded={menu}
          onClick={() => setMenu((open) => !open)}
        >
          <ChevronDown size={14} />
        </button>
        {menu && (
          <div className="quick-access-menu" role="menu">
            <div className="quick-access-menu-title">{t("Symbolleiste für den Schnellzugriff anpassen")}</div>
            {suggestions.map((id) => {
              const active = items.includes(id);
              const IconComponent = commandIcon(id);
              return (
                <button key={id} type="button" role="menuitemcheckbox" aria-checked={active} onClick={() => onChange(toggleQuickAccess(items, id))}>
                  <span className="quick-access-check">{active && <Check size={14} />}</span>
                  <IconComponent size={15} />
                  <span>{t(commandLabel(id))}</span>
                </button>
              );
            })}
            <hr />
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setMenu(false);
                onMoreOptions();
              }}
            >
              <span className="quick-access-check" />
              <Settings size={15} />
              <span>{t("Weitere Befehle …")}</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

type PanelProps = { items: string[]; onChange: (items: string[]) => void };

/** Bereich in *Datei → Optionen*: alle Befehle links, Leiste rechts (wie in Word). */
export function QuickAccessPanel({ items, onChange }: PanelProps) {
  const t = useT();
  const commands = quickAccessCommands();
  const [available, setAvailable] = useState<string>("");
  const [chosen, setChosen] = useState<string>(items[0] ?? "");
  const full = items.length >= QUICK_ACCESS_LIMIT;
  const add = () => {
    if (!available || items.includes(available) || full) return;
    onChange(toggleQuickAccess(items, available));
    setChosen(available);
  };
  return (
    <div className="quick-access-panel">
      <div className="quick-access-column">
        <span>{t("Befehle")}</span>
        <select size={12} value={available} onChange={(event) => setAvailable(event.currentTarget.value)} onDoubleClick={add} aria-label={t("Befehle")}>
          {SHORTCUT_CATEGORIES.filter((category) => category !== "Folien").map((category) => (
            <optgroup key={category} label={t(category)}>
              {commands
                .filter((command) => command.category === category)
                .map((command) => (
                  <option key={command.id} value={command.id} disabled={items.includes(command.id)}>
                    {t(command.label)}
                  </option>
                ))}
            </optgroup>
          ))}
        </select>
      </div>
      <div className="quick-access-buttons">
        <button type="button" className="small" onClick={add} disabled={!available || items.includes(available) || full}>
          <Plus size={14} /> {t("Hinzufügen")}
        </button>
        <button type="button" className="small" onClick={() => chosen && onChange(toggleQuickAccess(items, chosen))} disabled={!items.includes(chosen)}>
          <Minus size={14} /> {t("Entfernen")}
        </button>
        <button type="button" className="small" onClick={() => onChange(moveQuickAccess(items, chosen, -1))} disabled={items.indexOf(chosen) <= 0} title={t("Nach oben")} aria-label={t("Nach oben")}>
          <ArrowUp size={14} />
        </button>
        <button
          type="button"
          className="small"
          onClick={() => onChange(moveQuickAccess(items, chosen, 1))}
          disabled={!items.includes(chosen) || items.indexOf(chosen) === items.length - 1}
          title={t("Nach unten")}
          aria-label={t("Nach unten")}
        >
          <ArrowDown size={14} />
        </button>
        <button type="button" className="small" onClick={() => onChange([...DEFAULT_QUICK_ACCESS])}>
          {t("Zurücksetzen")}
        </button>
      </div>
      <div className="quick-access-column">
        <span>{t("In der Titelleiste")}</span>
        <select size={12} value={chosen} onChange={(event) => setChosen(event.currentTarget.value)} onDoubleClick={() => chosen && onChange(toggleQuickAccess(items, chosen))} aria-label={t("In der Titelleiste")}>
          {items.map((id) => (
            <option key={id} value={id}>
              {t(commandLabel(id))}
            </option>
          ))}
        </select>
        {full && <small className="muted">{t("Höchstens 16 Befehle.")}</small>}
      </div>
    </div>
  );
}
