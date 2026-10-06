/**
 * Folien-Editor: Präsentationen frei gestalten – Textfelder, Formen, Bilder und
 * Formeln auf einer Fläche verschieben, skalieren und drehen, mit Raster,
 * Hilfslinien, Ausrichten, Ebenen, Rückgängig und Bildschirmpräsentation.
 * Gespeichert wird als `.vtxslides` (JSON), exportiert als LaTeX-Beamer bzw. PDF
 * (Übersetzung und Kompilieren in Rust: `core/slides.rs`).
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ClipboardEvent as ReactClipboardEvent, CSSProperties, MutableRefObject, DragEvent as ReactDragEvent, KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent, ReactNode } from "react";
import { Editor } from "@tiptap/core";
import type { JSONContent } from "@tiptap/core";
import { EditorContent, ReactNodeViewRenderer, useEditor, useEditorState } from "@tiptap/react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AlignCenter,
  AlignCenterHorizontal,
  AlignCenterVertical,
  AlignEndHorizontal,
  AlignEndVertical,
  AlignHorizontalSpaceAround,
  AlignLeft,
  AlignRight,
  AlignStartHorizontal,
  AlignStartVertical,
  AlignVerticalSpaceAround,
  ArrowRight,
  ArrowDownToLine,
  ArrowUpToLine,
  Bold,
  BringToFront,
  ChevronDown,
  Circle,
  Copy,
  Diamond,
  Eye,
  EyeOff,
  FileText,
  Grid3x3,
  ImagePlus,
  Italic,
  LayoutTemplate,
  List,
  ListOrdered,
  Magnet,
  Minus,
  Play,
  Plus,
  Redo2,
  RotateCw,
  SendToBack,
  Sigma,
  Square,
  SquareDashed,
  Trash2,
  Triangle,
  Type,
  Underline,
  Undo2,
} from "lucide-react";
import { api, errorText, isTauri } from "../api";
import { useDialogs } from "../components/Dialogs";
import { InlineMath } from "../editor/schema";
import { InlineMathView } from "../editor/nodeViews";
import { useT } from "../i18n";
import { useCtrlWheel } from "../components/useCtrlWheel";
import { comboFromEvent, resolveBindings } from "../shortcuts/shortcuts";
import type { ShortcutOverrides } from "../shortcuts/shortcuts";
import { renderMath } from "../latex/miniRender";
import {
  LAYOUTS,
  PT,
  alignElements,
  boundsOf,
  cloneElements,
  createElement,
  createSlide,
  distributeElements,
  hasText,
  reorderElements,
  resizeBox,
  round,
  slideSize,
  snapMove,
  textContent,
} from "./model";
import type { Alignment, Box, ElementKind, Guide, Handle, LayoutId, ShapeKind, Slide, SlideDeck, SlideElement, SlideFileActions, SlidesSession } from "./model";
import { FONT_STACK, SlideView, elementStyle } from "./SlideView";
import { slideTextExtensions } from "./textExtensions";
import "./slides.css";


type Props = {
  session: SlidesSession;
  onSessionChange: (session: SlidesSession) => void;
  onClose: () => void;
  /** Öffnet den gemeinsamen Datei-Bereich (wie im Dokument). */
  onFileMenu: () => void;
  /** Datei öffnen (Dokument oder Präsentation) – wie Strg+O im Dokument. */
  onOpenFile: () => void;
  /** Dateibefehle für den Datei-Bereich */
  actionsRef: MutableRefObject<SlideFileActions | null>;
  shortcuts: ShortcutOverrides;
  allowOnline: boolean;
  notify: (text: string) => void;
};

type Tool = { kind: ElementKind; shape?: ShapeKind } | null;

type Drag =
  | { kind: "move"; startX: number; startY: number; origin: Map<string, { x: number; y: number }>; box: Box; moved: boolean; recorded: boolean }
  | { kind: "resize"; handle: Handle; startX: number; startY: number; start: SlideElement }
  | { kind: "rotate"; start: SlideElement }
  | { kind: "endpoint"; which: "start" | "end"; start: SlideElement }
  | { kind: "marquee"; startX: number; startY: number; x: number; y: number; additive: boolean; initial: string[] }
  | { kind: "draw"; tool: NonNullable<Tool>; startX: number; startY: number; x: number; y: number };

type TabId = "start" | "insert" | "design" | "arrange" | "present";

const SHAPES: Array<{ shape: ShapeKind; label: string; icon: ReactNode }> = [
  { shape: "rect", label: "Rechteck", icon: <Square size={16} /> },
  { shape: "roundRect", label: "Abgerundetes Rechteck", icon: <SquareDashed size={16} /> },
  { shape: "ellipse", label: "Ellipse", icon: <Circle size={16} /> },
  { shape: "triangle", label: "Dreieck", icon: <Triangle size={16} /> },
  { shape: "diamond", label: "Raute", icon: <Diamond size={16} /> },
  { shape: "line", label: "Linie", icon: <Minus size={16} /> },
  { shape: "arrow", label: "Pfeil", icon: <ArrowRight size={16} /> },
];
const FONT_SIZES = [10, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 44, 54, 60, 72];
const GRID_SIZES = [1, 2, 2.5, 5, 10];
const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

/** Interne Zwischenablage für Folienelemente (über Folien hinweg). */
let clipboard: SlideElement[] = [];

function baseName(path: string | null): string {
  if (!path) return "Praesentation";
  const name = path.split(/[\\/]/).pop() ?? "Praesentation";
  return name.replace(/\.[^.]+$/, "");
}

/** Wendet einen Tiptap-Befehl auf den gesamten Text eines Elements an (ohne Bearbeitungsmodus). */
function transformContent(content: JSONContent | null, apply: (editor: Editor) => void): JSONContent {
  const editor = new Editor({ extensions: slideTextExtensions(), content: content ?? EMPTY_DOC });
  try {
    editor.commands.selectAll();
    apply(editor);
    return editor.getJSON();
  } finally {
    editor.destroy();
  }
}

/** Größe einer Formel in mm (KaTeX, gemessen in einem unsichtbaren Element). */
function measureFormula(latex: string, fontSize: number): { w: number; h: number } {
  const probe = document.createElement("div");
  probe.className = "slide-formula slide-measure";
  probe.style.fontSize = `${fontSize * PT * 10}px`;
  probe.innerHTML = renderMath(`\\displaystyle ${latex || "\\square"}`, false);
  document.body.appendChild(probe);
  const rect = probe.getBoundingClientRect();
  probe.remove();
  return { w: round(Math.max(4, rect.width / 10 + 1)), h: round(Math.max(4, rect.height / 10 + 1)) };
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

function imageSize(src: string): Promise<{ width: number; height: number }> {
  return new Promise((resolve) => {
    const image = new Image();
    image.onload = () => resolve({ width: image.naturalWidth || 400, height: image.naturalHeight || 300 });
    image.onerror = () => resolve({ width: 400, height: 300 });
    image.src = src;
  });
}

// ---------------------------------------------------------------- Bausteine der Symbolleiste

function ToolButton({ label, icon, onClick, active = false, disabled = false, large = false, title }: { label: string; icon: ReactNode; onClick: () => void; active?: boolean; disabled?: boolean; large?: boolean; title?: string }) {
  return (
    <button
      type="button"
      className={`ribbon-button${large ? " large" : ""}${active ? " active" : ""}`}
      title={title ?? label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
    >
      {icon}
      {large && <span className="ribbon-button-label">{label}</span>}
    </button>
  );
}

function Group({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="ribbon-group" aria-label={label}>
      <div className="ribbon-group-content">{children}</div>
      <div className="ribbon-group-label">{label}</div>
    </section>
  );
}

function Menu({ label, icon, large = false, children }: { label: string; icon: ReactNode; large?: boolean; children: (close: () => void) => ReactNode }) {
  const [openMenu, setOpenMenu] = useState<{ top: number; left: number } | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!openMenu) return;
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpenMenu(null);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [openMenu]);
  return (
    <div className="ribbon-dropdown" ref={ref}>
      <button
        type="button"
        className={`ribbon-button with-caret${large ? " large" : ""}${openMenu ? " active" : ""}`}
        title={label}
        aria-label={label}
        aria-expanded={Boolean(openMenu)}
        onMouseDown={(event) => event.preventDefault()}
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          setOpenMenu((value) => (value ? null : { top: rect.bottom + 4, left: Math.max(4, Math.min(rect.left, window.innerWidth - 260)) }));
        }}
      >
        {icon}
        {large && <span className="ribbon-button-label">{label}</span>}
        <ChevronDown size={12} className="caret" />
      </button>
      {openMenu && (
        <div className="ribbon-popover" role="menu" style={openMenu} onMouseDown={(event) => event.preventDefault()}>
          {children(() => setOpenMenu(null))}
        </div>
      )}
    </div>
  );
}

function MenuItem({ label, icon, onClick }: { label: string; icon?: ReactNode; onClick: () => void }) {
  return (
    <button type="button" className="menu-item" role="menuitem" onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

function ColorField({ label, value, onChange, allowNone = true, noneLabel }: { label: string; value: string; onChange: (value: string) => void; allowNone?: boolean; noneLabel: string }) {
  return (
    <label className="slide-field color">
      <span>{label}</span>
      <span className="slide-color-row">
        <input type="color" value={/^#[0-9a-f]{6}$/i.test(value) ? value : "#000000"} onChange={(event) => onChange(event.currentTarget.value)} aria-label={label} />
        {allowNone && (
          <button type="button" className={`small${value ? "" : " active"}`} onClick={() => onChange("")}>
            {noneLabel}
          </button>
        )}
      </span>
    </label>
  );
}

function NumberField({ label, value, onChange, step = 0.5, min, max, unit }: { label: string; value: number; onChange: (value: number) => void; step?: number; min?: number; max?: number; unit?: string }) {
  const [draft, setDraft] = useState(String(round(value)));
  useEffect(() => setDraft(String(round(value))), [value]);
  const commit = () => {
    const parsed = Number(draft.replace(",", "."));
    if (Number.isFinite(parsed)) onChange(min !== undefined || max !== undefined ? Math.min(max ?? Infinity, Math.max(min ?? -Infinity, parsed)) : parsed);
    else setDraft(String(round(value)));
  };
  return (
    <label className="slide-field">
      <span>{label}</span>
      <span className="slide-number">
        <input
          type="number"
          value={draft}
          step={step}
          min={min}
          max={max}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") commit();
          }}
        />
        {unit && <small>{unit}</small>}
      </span>
    </label>
  );
}

// ---------------------------------------------------------------- Bildschirmpräsentation

function Presenter({ deck, start, onClose }: { deck: SlideDeck; start: number; onClose: () => void }) {
  const t = useT();
  const slides = deck.slides.filter((slide) => !slide.hidden);
  const [index, setIndex] = useState(() => Math.max(0, Math.min(slides.length - 1, start)));
  const [viewport, setViewport] = useState({ w: window.innerWidth, h: window.innerHeight });
  const rootRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const size = slideSize(deck.aspect);
  useEffect(() => {
    const element = rootRef.current;
    element?.focus();
    void element?.requestFullscreen?.().catch(() => undefined);
    const resize = () => setViewport({ w: window.innerWidth, h: window.innerHeight });
    const leave = () => {
      if (!document.fullscreenElement) closeRef.current();
    };
    window.addEventListener("resize", resize);
    document.addEventListener("fullscreenchange", leave);
    return () => {
      window.removeEventListener("resize", resize);
      document.removeEventListener("fullscreenchange", leave);
      if (document.fullscreenElement) void document.exitFullscreen().catch(() => undefined);
    };
  }, []);
  const next = () => setIndex((value) => Math.min(slides.length - 1, value + 1));
  const previous = () => setIndex((value) => Math.max(0, value - 1));
  const scale = Math.min(viewport.w / size.w, viewport.h / size.h);
  const slide = slides[index];
  return (
    <div
      ref={rootRef}
      className="slide-presenter"
      role="dialog"
      aria-label={t("Bildschirmpräsentation")}
      tabIndex={-1}
      onClick={next}
      onContextMenu={(event) => {
        event.preventDefault();
        previous();
      }}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (["ArrowRight", "ArrowDown", "PageDown", " ", "Enter", "n"].includes(event.key)) next();
        else if (["ArrowLeft", "ArrowUp", "PageUp", "Backspace", "p"].includes(event.key)) previous();
        else if (event.key === "Home") setIndex(0);
        else if (event.key === "End") setIndex(slides.length - 1);
        else if (event.key === "Escape") onClose();
        else return;
        event.preventDefault();
      }}
    >
      {slide ? <SlideView slide={slide} deck={deck} scale={scale} /> : <p>{t("Keine sichtbaren Folien.")}</p>}
      <div className="slide-presenter-bar" onClick={(event) => event.stopPropagation()}>
        <button type="button" onClick={previous} aria-label={t("Vorherige Folie")}>‹</button>
        <span>{index + 1} / {slides.length}</span>
        <button type="button" onClick={next} aria-label={t("Nächste Folie")}>›</button>
        <button type="button" onClick={onClose}>{t("Beenden")}</button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- Editor

export default function SlideEditor({ session, onSessionChange, onClose, onFileMenu, onOpenFile, actionsRef, shortcuts, allowOnline, notify }: Props) {
  const t = useT();
  const dialogs = useDialogs();
  const deck = session.deck;
  const size = slideSize(deck.aspect);

  const sessionRef = useRef(session);
  sessionRef.current = session;
  const history = useRef<{ past: SlideDeck[]; future: SlideDeck[] }>({ past: [], future: [] });

  const [current, setCurrent] = useState(0);
  const index = Math.min(current, deck.slides.length - 1);
  const slide = deck.slides[index];
  const currentRef = useRef(index);
  currentRef.current = index;

  const [selection, setSelection] = useState<string[]>([]);
  const selectionRef = useRef(selection);
  selectionRef.current = selection;
  const [editingId, setEditingId] = useState<string | null>(null);
  const editingRef = useRef(editingId);
  editingRef.current = editingId;
  const [tool, setTool] = useState<Tool>(null);
  const [tab, setTab] = useState<TabId>("start");
  const [guides, setGuides] = useState<Guide[]>([]);
  const [marquee, setMarquee] = useState<Box | null>(null);
  const [drawing, setDrawing] = useState<Box | null>(null);
  const [presenting, setPresenting] = useState<number | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const dragRef = useRef<Drag | null>(null);
  const formulaInputRef = useRef<HTMLTextAreaElement>(null);

  // ------------------------------------------------------------ Zustand ändern

  const commit = useCallback(
    (next: SlideDeck, record = true) => {
      const previous = sessionRef.current;
      if (record) {
        history.current.past.push(previous.deck);
        if (history.current.past.length > 200) history.current.past.shift();
        history.current.future = [];
      }
      const updated = { ...previous, deck: next, dirty: true };
      sessionRef.current = updated;
      onSessionChange(updated);
    },
    [onSessionChange],
  );

  const updateSlide = useCallback(
    (update: (slide: Slide) => Slide, record = true) => {
      const current = sessionRef.current.deck;
      const position = currentRef.current;
      commit({ ...current, slides: current.slides.map((item, itemIndex) => (itemIndex === position ? update(item) : item)) }, record);
    },
    [commit],
  );

  const updateElements = useCallback(
    (ids: string[], patch: Partial<SlideElement> | ((element: SlideElement) => Partial<SlideElement>), record = true) => {
      updateSlide(
        (item) => ({
          ...item,
          elements: item.elements.map((element) => (ids.includes(element.id) ? { ...element, ...(typeof patch === "function" ? patch(element) : patch) } : element)),
        }),
        record,
      );
    },
    [updateSlide],
  );

  const undo = () => {
    const previous = history.current.past.pop();
    if (!previous) return;
    history.current.future.push(sessionRef.current.deck);
    setEditingId(null);
    commit(previous, false);
  };
  const redo = () => {
    const next = history.current.future.pop();
    if (!next) return;
    history.current.past.push(sessionRef.current.deck);
    setEditingId(null);
    commit(next, false);
  };

  // Auswahl auf vorhandene Elemente beschränken (nach Rückgängig, Folienwechsel …)
  useEffect(() => {
    const ids = new Set(slide?.elements.map((element) => element.id));
    setSelection((value) => (value.every((id) => ids.has(id)) ? value : value.filter((id) => ids.has(id))));
    if (editingId && !ids.has(editingId)) setEditingId(null);
  }, [slide, editingId]);

  const selectedElements = useMemo(() => slide?.elements.filter((element) => selection.includes(element.id)) ?? [], [slide, selection]);
  const single = selectedElements.length === 1 ? selectedElements[0] : null;

  // ------------------------------------------------------------ Textbearbeitung

  const textExtensions = useMemo(
    () => slideTextExtensions(InlineMath.extend({ addNodeView: () => ReactNodeViewRenderer(InlineMathView) })),
    [],
  );
  const textEditor = useEditor(
    {
      extensions: textExtensions,
      content: EMPTY_DOC,
      onUpdate: ({ editor }) => {
        const id = editingRef.current;
        if (!id) return;
        updateElements([id], { content: editor.getJSON() }, false);
        // Textfelder wachsen mit dem Inhalt (wie „Form an Text anpassen“).
        requestAnimationFrame(() => {
          const element = sessionRef.current.deck.slides[currentRef.current]?.elements.find((item) => item.id === id);
          const dom = editor.view.dom as HTMLElement;
          if (!element || element.kind !== "text" || !dom.isConnected) return;
          const scaleNow = dom.getBoundingClientRect().width / Math.max(1, element.w - 2 * element.padding);
          const needed = round(dom.scrollHeight / Math.max(0.1, scaleNow) + 2 * element.padding + 0.5, 1);
          if (needed > element.h) updateElements([id], { h: needed }, false);
        });
      },
    },
    [textExtensions],
  );
  const textState = useEditorState({
    editor: textEditor,
    selector: ({ editor }) =>
      editor
        ? {
            bold: editor.isActive("bold"),
            italic: editor.isActive("italic"),
            underline: editor.isActive("underline"),
            bullet: editor.isActive("bulletList"),
            ordered: editor.isActive("orderedList"),
          }
        : null,
  });

  const startEditing = useCallback(
    (id: string) => {
      const element = sessionRef.current.deck.slides[currentRef.current]?.elements.find((item) => item.id === id);
      if (!element || !textEditor || element.kind === "image" || element.kind === "formula" || element.shape === "line" || element.shape === "arrow") return;
      // Ein Rückgängig-Schritt für die gesamte Bearbeitung
      history.current.past.push(sessionRef.current.deck);
      history.current.future = [];
      textEditor.commands.setContent(element.content ?? EMPTY_DOC, { emitUpdate: false });
      setSelection([id]);
      setEditingId(id);
      setTool(null);
      requestAnimationFrame(() => textEditor.commands.focus("end"));
    },
    [textEditor],
  );
  const stopEditing = useCallback(() => setEditingId(null), []);

  /** Textformat: im Bearbeitungsmodus auf die Auswahl, sonst auf den ganzen Text der Elemente. */
  const formatText = (apply: (editor: Editor) => void) => {
    if (editingId && textEditor) {
      apply(textEditor);
      return;
    }
    const targets = selectedElements.filter((element) => element.kind === "text" || element.kind === "shape");
    if (targets.length === 0) return;
    updateElements(
      targets.map((element) => element.id),
      (element) => ({ content: transformContent(element.content, apply) }),
    );
  };

  // ------------------------------------------------------------ Folien

  const goTo = (position: number) => {
    setEditingId(null);
    setSelection([]);
    setCurrent(Math.max(0, Math.min(sessionRef.current.deck.slides.length - 1, position)));
  };

  const addSlide = (layout: LayoutId) => {
    const current = sessionRef.current.deck;
    const created = createSlide(layout, current.aspect);
    const slides = [...current.slides];
    slides.splice(currentRef.current + 1, 0, created);
    commit({ ...current, slides });
    goTo(currentRef.current + 1);
  };
  const duplicateSlide = () => {
    const current = sessionRef.current.deck;
    const copy: Slide = { ...structuredClone(current.slides[currentRef.current]), id: `${Date.now().toString(36)}d` };
    copy.elements = cloneElements(copy.elements, 0);
    const slides = [...current.slides];
    slides.splice(currentRef.current + 1, 0, copy);
    commit({ ...current, slides });
    goTo(currentRef.current + 1);
  };
  const deleteSlide = (position = currentRef.current) => {
    const current = sessionRef.current.deck;
    if (current.slides.length <= 1) {
      commit({ ...current, slides: [createSlide("blank", current.aspect)] });
      goTo(0);
      return;
    }
    commit({ ...current, slides: current.slides.filter((_, itemIndex) => itemIndex !== position) });
    goTo(Math.min(position, current.slides.length - 2));
  };
  const moveSlide = (from: number, to: number) => {
    const current = sessionRef.current.deck;
    if (from === to || to < 0 || to >= current.slides.length) return;
    const slides = [...current.slides];
    const [moved] = slides.splice(from, 1);
    slides.splice(to, 0, moved);
    commit({ ...current, slides });
    goTo(to);
  };
  const applyLayout = (layout: LayoutId) => {
    // Vorhandene Inhalte bleiben, Platzhalter des Layouts kommen hinzu, leere alte Platzhalter entfallen.
    const fresh = createSlide(layout, sessionRef.current.deck.aspect);
    updateSlide((item) => ({
      ...item,
      elements: [...item.elements.filter((element) => !(element.role && !hasText(element.content))), ...fresh.elements],
    }));
  };

  // ------------------------------------------------------------ Elemente

  const addElement = (element: SlideElement, edit = false) => {
    updateSlide((item) => ({ ...item, elements: [...item.elements, element] }));
    setSelection([element.id]);
    if (edit) requestAnimationFrame(() => startEditing(element.id));
  };

  const defaultsFor = (kind: ElementKind, shape?: ShapeKind): Partial<SlideElement> => {
    if (kind === "text") return { content: textContent([""]), fontSize: 18 };
    if (shape === "line" || shape === "arrow") return { shape, stroke: "#1f2937", strokeWidth: 2, h: 0 };
    return { shape: shape ?? "rect", fill: deck.theme.accent, stroke: "", fontSize: 18, verticalAlign: "middle", content: textContent([""], { align: "center" }), color: "#ffffff" };
  };

  const createAt = (kind: ElementKind, shape: ShapeKind | undefined, box: Box | null) => {
    const defaults = defaultsFor(kind, shape);
    const w = box && box.w > 2 ? box.w : kind === "text" ? 60 : shape === "line" || shape === "arrow" ? 40 : 30;
    const h = box && (box.h > 2 || shape === "line" || shape === "arrow") ? box.h : kind === "text" ? 12 : 20;
    const x = box ? box.x : (size.w - w) / 2;
    const y = box ? box.y : (size.h - h) / 2;
    const element = createElement(kind, { ...defaults, x: round(x), y: round(y), w: round(w), h: round(h) });
    addElement(element, kind === "text");
  };

  const insertImage = async () => {
    if (!isTauri) {
      notify(t("Bilder einfügen ist nur in der Desktop-App möglich."));
      return;
    }
    const selected = await open({
      multiple: false,
      filters: [{ name: t("Bilder"), extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "svg", "pdf"] }],
    });
    if (typeof selected !== "string") return;
    try {
      const image = await api.slidesReadImage(selected);
      await addImage(image.src, image.width, image.height);
    } catch (error) {
      notify(errorText(error));
    }
  };

  const addImage = async (src: string, width: number | null, height: number | null) => {
    const natural = width && height ? { width, height } : await imageSize(src);
    const ratio = natural.width / Math.max(1, natural.height);
    let w = Math.min(size.w * 0.6, 80);
    let h = w / ratio;
    if (h > size.h * 0.7) {
      h = size.h * 0.7;
      w = h * ratio;
    }
    addElement(createElement("image", { src, x: round((size.w - w) / 2), y: round((size.h - h) / 2), w: round(w), h: round(h) }));
  };

  const insertFormula = () => {
    const latex = "a^2 + b^2 = c^2";
    const measured = measureFormula(latex, 24);
    addElement(createElement("formula", { latex, fontSize: 24, x: round((size.w - measured.w) / 2), y: round((size.h - measured.h) / 2), ...measured }));
    requestAnimationFrame(() => formulaInputRef.current?.focus());
  };

  const deleteSelection = () => {
    if (selection.length === 0) return;
    updateSlide((item) => ({ ...item, elements: item.elements.filter((element) => !selection.includes(element.id)) }));
    setSelection([]);
  };
  const copySelection = () => {
    clipboard = structuredClone(selectedElements);
  };
  const paste = () => {
    if (clipboard.length === 0) return;
    const copies = cloneElements(clipboard, 5);
    updateSlide((item) => ({ ...item, elements: [...item.elements, ...copies] }));
    setSelection(copies.map((element) => element.id));
    clipboard = structuredClone(copies);
  };
  const duplicateSelection = () => {
    if (selectedElements.length === 0) return;
    const copies = cloneElements(selectedElements, 5);
    updateSlide((item) => ({ ...item, elements: [...item.elements, ...copies] }));
    setSelection(copies.map((element) => element.id));
  };
  const nudge = (dx: number, dy: number) => {
    if (selection.length === 0) return;
    updateElements(selection, (element) => ({ x: round(element.x + dx), y: round(element.y + dy) }));
  };
  const align = (alignment: Alignment) => {
    if (selection.length === 0) return;
    updateSlide((item) => ({ ...item, elements: alignElements(item.elements, selection, alignment, size) }));
  };
  const distribute = (axis: "horizontal" | "vertical") => {
    updateSlide((item) => ({ ...item, elements: distributeElements(item.elements, selection, axis) }));
  };
  const reorder = (direction: "front" | "back" | "forward" | "backward") => {
    if (selection.length === 0) return;
    updateSlide((item) => ({ ...item, elements: reorderElements(item.elements, selection, direction) }));
  };

  // ------------------------------------------------------------ Dateien

  const confirmDiscard = async (): Promise<boolean> => {
    if (!sessionRef.current.dirty) return true;
    const answer = await dialogs.confirm({
      title: t("Präsentation"),
      message: t("Die Präsentation enthält ungespeicherte Änderungen. Jetzt speichern?"),
      confirmLabel: t("Speichern"),
      cancelLabel: t("Abbrechen"),
      thirdLabel: t("Verwerfen"),
    });
    if (answer === "third") return true;
    if (!answer) return false;
    return saveDeck(false);
  };

  const saveDeck = async (saveAs: boolean): Promise<boolean> => {
    if (!isTauri) {
      notify(t("Speichern ist nur in der Desktop-App möglich."));
      return false;
    }
    let path = sessionRef.current.path;
    if (!path || saveAs) {
      const chosen = await save({ defaultPath: `${baseName(path)}.vtxslides`, filters: [{ name: t("VisuTeX-Präsentation"), extensions: ["vtxslides"] }] });
      if (!chosen) return false;
      path = chosen;
    }
    try {
      await api.slidesSave(path, sessionRef.current.deck);
      const updated = { ...sessionRef.current, path, dirty: false };
      sessionRef.current = updated;
      onSessionChange(updated);
      setStatus(t("Gespeichert."));
      return true;
    } catch (error) {
      notify(errorText(error));
      return false;
    }
  };

  /** Neue bzw. geöffnete Präsentation übernehmen (Rückfrage erledigt der Aufrufer). */
  const replaceSession = (next: SlidesSession) => {
    history.current = { past: [], future: [] };
    setEditingId(null);
    setSelection([]);
    sessionRef.current = next;
    onSessionChange(next);
    goTo(0);
  };

  const exportLatex = async () => {
    if (!isTauri) return;
    const path = await save({ defaultPath: `${baseName(sessionRef.current.path)}.tex`, filters: [{ name: "LaTeX", extensions: ["tex"] }] });
    if (!path) return;
    try {
      const result = await api.slidesToLatex(sessionRef.current.deck);
      await api.exportTex(path, result.latex, result.assets, null);
      setStatus(t("LaTeX-Beamer-Datei exportiert."));
      for (const warning of result.warnings) notify(warning);
    } catch (error) {
      notify(errorText(error));
    }
  };

  const exportPdf = async () => {
    if (!isTauri || busy) return;
    const path = await save({ defaultPath: `${baseName(sessionRef.current.path)}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return;
    setBusy(true);
    setStatus(t("Kompiliere …"));
    try {
      const result = await api.slidesCompile(sessionRef.current.deck, allowOnline);
      await api.savePdf(result.pdfId, path);
      const errors = result.messages.filter((message) => message.severity === "error");
      setStatus(errors.length ? `${t("PDF gespeichert – mit Fehlern:")} ${errors[0].message}` : `${t("PDF gespeichert")} (${(result.durationMs / 1000).toFixed(1)} s).`);
    } catch (error) {
      setStatus("");
      notify(errorText(error));
    } finally {
      setBusy(false);
    }
  };

  const fileActions: SlideFileActions = {
    save: saveDeck,
    exportPdf,
    exportLatex,
    confirmDiscard,
    replace: replaceSession,
    updateMeta: (patch) => commit({ ...sessionRef.current.deck, ...patch }),
  };
  actionsRef.current = fileActions;
  useEffect(() => {
    actionsRef.current = fileActions;
    return () => {
      actionsRef.current = null;
    };
  });

  const setAspect = (aspect: "16:9" | "4:3") => {
    const current = sessionRef.current.deck;
    if (current.aspect === aspect) return;
    const from = slideSize(current.aspect);
    const to = slideSize(aspect);
    const fx = to.w / from.w;
    const fy = to.h / from.h;
    commit({
      ...current,
      aspect,
      slides: current.slides.map((item) => ({
        ...item,
        elements: item.elements.map((element) => ({ ...element, x: round(element.x * fx), y: round(element.y * fy), w: round(element.w * fx), h: round(element.h * fy) })),
      })),
    });
  };

  /** Zurück zum Dokument: die Präsentation bleibt geöffnet (auch ungespeichert, mit Wiederherstellung). */
  const close = async () => {
    setEditingId(null);
    onClose();
  };

  // ------------------------------------------------------------ Arbeitsfläche

  const stageRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const [stage, setStage] = useState({ w: 900, h: 520 });
  const [zoom, setZoom] = useState<number | null>(null);
  // Strg + Mausrad auf der Arbeitsfläche (Zoom relativ zu „Anpassen“ = 1)
  useCtrlWheel(stageRef, (direction) =>
    setZoom((value) => {
      const steps = [0.5, 0.75, 1, 1.25, 1.5, 2];
      const current = value ?? 1;
      const next = direction > 0 ? steps.find((step) => step > current + 0.01) : [...steps].reverse().find((step) => step < current - 0.01);
      if (next === undefined) return value;
      return next === 1 ? null : next;
    }),
  );
  useEffect(() => {
    const element = stageRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setStage({ w: element.clientWidth, h: element.clientHeight }));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const fitScale = Math.max(1, Math.min((stage.w - 48) / size.w, (stage.h - 48) / size.h));
  const scale = zoom ? fitScale * zoom : fitScale;
  const scaleRef = useRef(scale);
  scaleRef.current = scale;

  const toSlide = (event: { clientX: number; clientY: number }) => {
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return { x: 0, y: 0 };
    return { x: (event.clientX - rect.left) / scaleRef.current, y: (event.clientY - rect.top) / scaleRef.current };
  };

  const gridValue = deck.grid.snap ? deck.grid.size : null;

  const onPointerMove = useCallback(
    (event: PointerEvent) => {
      const drag = dragRef.current;
      if (!drag) return;
      const point = toSlide(event);
      const current = sessionRef.current.deck;
      const activeSlide = current.slides[currentRef.current];
      switch (drag.kind) {
        case "move": {
          const dx = point.x - drag.startX;
          const dy = point.y - drag.startY;
          if (!drag.moved && Math.hypot(dx, dy) * scaleRef.current < 3) return;
          if (!drag.recorded) {
            history.current.past.push(current);
            history.current.future = [];
            drag.recorded = true;
          }
          drag.moved = true;
          const others = activeSlide.elements.filter((element) => !drag.origin.has(element.id));
          const snapped = event.altKey
            ? { dx, dy, guides: [] }
            : snapMove(drag.box, dx, dy, { grid: gridValue, guides: current.grid.guides, targets: others, slide: slideSize(current.aspect), threshold: 6 / scaleRef.current });
          setGuides(snapped.guides);
          updateElements(
            [...drag.origin.keys()],
            (element) => {
              const origin = drag.origin.get(element.id) ?? { x: element.x, y: element.y };
              return { x: round(origin.x + snapped.dx), y: round(origin.y + snapped.dy) };
            },
            false,
          );
          break;
        }
        case "resize": {
          const start = drag.start;
          const angle = (start.rotation * Math.PI) / 180;
          const rawDx = point.x - drag.startX;
          const rawDy = point.y - drag.startY;
          // Zug in Elementkoordinaten (gedrehte Elemente)
          const dx = rawDx * Math.cos(angle) + rawDy * Math.sin(angle);
          const dy = -rawDx * Math.sin(angle) + rawDy * Math.cos(angle);
          const keepAspect = event.shiftKey || (start.kind === "image" && drag.handle.length === 2) || start.kind === "formula";
          const box = resizeBox(start, drag.handle, dx, dy, { keepAspect, grid: start.rotation || event.altKey ? null : gridValue, min: 2 });
          // Gegenüberliegenden Punkt festhalten (bei Drehung um die Mitte)
          const anchor = (b: Box) => ({
            x: drag.handle.includes("w") ? b.x + b.w : drag.handle.includes("e") ? b.x : b.x + b.w / 2,
            y: drag.handle.includes("n") ? b.y + b.h : drag.handle.includes("s") ? b.y : b.y + b.h / 2,
          });
          const rotate = (x: number, y: number) => ({ x: x * Math.cos(angle) - y * Math.sin(angle), y: x * Math.sin(angle) + y * Math.cos(angle) });
          const oldCenter = { x: start.x + start.w / 2, y: start.y + start.h / 2 };
          const oldAnchor = anchor(start);
          const world = rotate(oldAnchor.x - oldCenter.x, oldAnchor.y - oldCenter.y);
          const worldAnchor = { x: oldCenter.x + world.x, y: oldCenter.y + world.y };
          const newAnchor = anchor(box);
          const local = rotate(newAnchor.x - (box.x + box.w / 2), newAnchor.y - (box.y + box.h / 2));
          const center = { x: worldAnchor.x - local.x, y: worldAnchor.y - local.y };
          const patch: Partial<SlideElement> = { x: round(center.x - box.w / 2), y: round(center.y - box.h / 2), w: box.w, h: box.h };
          if (start.kind === "formula") patch.fontSize = round(Math.max(6, start.fontSize * (box.h / Math.max(1, start.h))), 1);
          updateElements([start.id], patch, false);
          break;
        }
        case "rotate": {
          const start = drag.start;
          const center = { x: start.x + start.w / 2, y: start.y + start.h / 2 };
          let angle = (Math.atan2(point.y - center.y, point.x - center.x) * 180) / Math.PI + 90;
          if (angle > 180) angle -= 360;
          angle = event.shiftKey ? Math.round(angle / 15) * 15 : Math.round(angle);
          if (!event.shiftKey && Math.abs(angle % 90) < 3) angle = Math.round(angle / 90) * 90;
          updateElements([start.id], { rotation: angle === -0 ? 0 : angle }, false);
          break;
        }
        case "endpoint": {
          const start = drag.start;
          const p1 = { x: start.flipH ? start.x + start.w : start.x, y: start.flipV ? start.y + start.h : start.y };
          const p2 = { x: start.flipH ? start.x : start.x + start.w, y: start.flipV ? start.y : start.y + start.h };
          const snap = (value: number) => (gridValue && !event.altKey ? Math.round(value / gridValue) * gridValue : value);
          let moved = { x: snap(point.x), y: snap(point.y) };
          const fixed = drag.which === "start" ? p2 : p1;
          if (event.shiftKey) {
            // waagrecht/senkrecht/45°
            const dx = moved.x - fixed.x;
            const dy = moved.y - fixed.y;
            const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
            const length = Math.hypot(dx, dy);
            moved = { x: fixed.x + Math.cos(angle) * length, y: fixed.y + Math.sin(angle) * length };
          }
          const a = drag.which === "start" ? moved : p1;
          const b = drag.which === "start" ? p2 : moved;
          updateElements(
            [start.id],
            { x: round(Math.min(a.x, b.x)), y: round(Math.min(a.y, b.y)), w: round(Math.abs(b.x - a.x)), h: round(Math.abs(b.y - a.y)), flipH: a.x > b.x, flipV: a.y > b.y },
            false,
          );
          break;
        }
        case "marquee": {
          drag.x = point.x;
          drag.y = point.y;
          const box = { x: Math.min(drag.startX, point.x), y: Math.min(drag.startY, point.y), w: Math.abs(point.x - drag.startX), h: Math.abs(point.y - drag.startY) };
          setMarquee(box);
          const inside = activeSlide.elements
            .filter((element) => element.x >= box.x && element.y >= box.y && element.x + element.w <= box.x + box.w && element.y + element.h <= box.y + box.h)
            .map((element) => element.id);
          setSelection(drag.additive ? [...new Set([...drag.initial, ...inside])] : inside);
          break;
        }
        case "draw": {
          const snap = (value: number) => (gridValue && !event.altKey ? Math.round(value / gridValue) * gridValue : value);
          drag.x = snap(point.x);
          drag.y = snap(point.y);
          const line = drag.tool.shape === "line" || drag.tool.shape === "arrow";
          setDrawing(
            line
              ? { x: drag.startX, y: drag.startY, w: drag.x - drag.startX, h: drag.y - drag.startY }
              : { x: Math.min(drag.startX, drag.x), y: Math.min(drag.startY, drag.y), w: Math.abs(drag.x - drag.startX), h: Math.abs(drag.y - drag.startY) },
          );
          break;
        }
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [gridValue, updateElements],
  );

  const onPointerUp = useCallback(() => {
    const drag = dragRef.current;
    dragRef.current = null;
    setGuides([]);
    setMarquee(null);
    setDrawing(null);
    if (drag?.kind === "draw") {
      const line = drag.tool.shape === "line" || drag.tool.shape === "arrow";
      const tiny = Math.abs(drag.x - drag.startX) < 2 && Math.abs(drag.y - drag.startY) < 2;
      if (line && !tiny) {
        const defaults = defaultsFor(drag.tool.kind, drag.tool.shape);
        addElement(
          createElement(drag.tool.kind, {
            ...defaults,
            x: round(Math.min(drag.startX, drag.x)),
            y: round(Math.min(drag.startY, drag.y)),
            w: round(Math.abs(drag.x - drag.startX)),
            h: round(Math.abs(drag.y - drag.startY)),
            flipH: drag.startX > drag.x,
            flipV: drag.startY > drag.y,
          }),
        );
      } else {
        createAt(drag.tool.kind, drag.tool.shape, tiny ? { x: drag.startX, y: drag.startY, w: 0, h: 0 } : { x: Math.min(drag.startX, drag.x), y: Math.min(drag.startY, drag.y), w: Math.abs(drag.x - drag.startX), h: Math.abs(drag.y - drag.startY) });
      }
      setTool(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onPointerMove]);

  // Stabile Listener: Die Handler ändern sich bei jedem Rendern (neuer Zustand), die
  // registrierten Funktionen dürfen es nicht – sonst endet das Ziehen nach dem ersten Schritt.
  const moveHandlerRef = useRef(onPointerMove);
  moveHandlerRef.current = onPointerMove;
  const upHandlerRef = useRef(onPointerUp);
  upHandlerRef.current = onPointerUp;
  const stableMove = useCallback((event: PointerEvent) => moveHandlerRef.current(event), []);
  const stableUp = useCallback(() => {
    window.removeEventListener("pointermove", stableMove);
    window.removeEventListener("pointerup", stableUp);
    window.removeEventListener("pointercancel", stableUp);
    upHandlerRef.current();
  }, [stableMove]);
  const beginDrag = (drag: Drag) => {
    dragRef.current = drag;
    window.addEventListener("pointermove", stableMove);
    window.addEventListener("pointerup", stableUp);
    window.addEventListener("pointercancel", stableUp);
  };
  useEffect(
    () => () => {
      window.removeEventListener("pointermove", stableMove);
      window.removeEventListener("pointerup", stableUp);
      window.removeEventListener("pointercancel", stableUp);
    },
    [stableMove, stableUp],
  );

  const onCanvasPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    const point = toSlide(event);
    // Klick in das gerade bearbeitete Textfeld: Tiptap übernimmt.
    if (editingId && target.closest(`[data-element-id="${editingId}"]`)) return;
    if (editingId) stopEditing();
    // preventDefault unten verhindert den Fokus – für Entf, Pfeiltasten usw. selbst setzen.
    canvasRef.current?.focus({ preventScroll: true });
    const handle = target.closest<HTMLElement>("[data-handle]");
    if (handle && single) {
      event.preventDefault();
      const kind = handle.dataset.handle ?? "";
      history.current.past.push(sessionRef.current.deck);
      history.current.future = [];
      if (kind === "rotate") beginDrag({ kind: "rotate", start: single });
      else if (kind === "start" || kind === "end") beginDrag({ kind: "endpoint", which: kind, start: single });
      else beginDrag({ kind: "resize", handle: kind as Handle, startX: point.x, startY: point.y, start: single });
      return;
    }
    if (tool) {
      event.preventDefault();
      const snap = (value: number) => (gridValue ? Math.round(value / gridValue) * gridValue : value);
      const start = { x: snap(point.x), y: snap(point.y) };
      beginDrag({ kind: "draw", tool, startX: start.x, startY: start.y, x: start.x, y: start.y });
      return;
    }
    const elementNode = target.closest<HTMLElement>("[data-element-id]");
    const id = elementNode?.dataset.elementId ?? null;
    if (id) {
      event.preventDefault();
      const additive = event.shiftKey || event.ctrlKey || event.metaKey;
      let next = selection;
      if (additive) next = selection.includes(id) ? selection.filter((item) => item !== id) : [...selection, id];
      else if (!selection.includes(id)) next = [id];
      setSelection(next);
      const moving = slide.elements.filter((element) => next.includes(element.id));
      if (moving.length === 0) return;
      beginDrag({
        kind: "move",
        startX: point.x,
        startY: point.y,
        origin: new Map(moving.map((element) => [element.id, { x: element.x, y: element.y }])),
        box: boundsOf(moving),
        moved: false,
        recorded: false,
      });
      return;
    }
    // Leere Fläche: Auswahlrechteck
    event.preventDefault();
    const additive = event.shiftKey || event.ctrlKey || event.metaKey;
    if (!additive) setSelection([]);
    beginDrag({ kind: "marquee", startX: point.x, startY: point.y, x: point.x, y: point.y, additive, initial: additive ? selection : [] });
  };

  const onCanvasDoubleClick = (event: ReactMouseEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement;
    const id = target.closest<HTMLElement>("[data-element-id]")?.dataset.elementId;
    if (!id || id === editingId) return;
    const element = slide.elements.find((item) => item.id === id);
    if (!element) return;
    if (element.kind === "formula") {
      setSelection([id]);
      requestAnimationFrame(() => formulaInputRef.current?.focus());
    } else startEditing(id);
  };

  // ------------------------------------------------------------ Tastatur

  const bindings = useMemo(() => resolveBindings(shortcuts), [shortcuts]);

  const runCommand = (id: string): boolean => {
    switch (id) {
      case "file.save": void saveDeck(false); return true;
      case "file.saveAs": void saveDeck(true); return true;
      case "file.open": onOpenFile(); return true;
      case "file.exportPdf": void exportPdf(); return true;
      case "file.exportTex": void exportLatex(); return true;
      case "file.compile": void exportPdf(); return true;
      case "slides.newSlide": addSlide("titleContent"); return true;
      case "slides.textBox": setTool({ kind: "text" }); return true;
      case "slides.duplicate": duplicateSelection(); return true;
      case "slides.present": setPresenting(currentRef.current); return true;
      case "slides.grid": commit({ ...sessionRef.current.deck, grid: { ...sessionRef.current.deck.grid, show: !sessionRef.current.deck.grid.show } }, false); return true;
      case "edit.undo": undo(); return true;
      case "edit.redo": redo(); return true;
    }
    return false;
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement;
    const inField = Boolean(target.closest("input, textarea, select"));
    const inText = Boolean(editingId && target.closest(".ProseMirror"));
    const combo = comboFromEvent(event.nativeEvent);
    const command = combo ? bindings.get(combo) : undefined;
    if (event.key === "Escape") {
      if (editingId) {
        stopEditing();
        event.preventDefault();
      } else if (tool) setTool(null);
      else if (selection.length) setSelection([]);
      return;
    }
    // Dateibefehle und Präsentation auch aus Text/Feldern heraus
    if (command && (command.startsWith("file.") || command === "slides.present") && runCommand(command)) {
      event.preventDefault();
      return;
    }
    if (inField || inText) return;
    if (command && runCommand(command)) {
      event.preventDefault();
      return;
    }
    const mod = event.ctrlKey || event.metaKey;
    const step = event.altKey ? 0.5 : mod ? 1 : deck.grid.size;
    switch (event.key) {
      case "Delete":
      case "Backspace":
        deleteSelection();
        break;
      case "ArrowLeft":
        if (selection.length) nudge(-step, 0);
        else goTo(index - 1);
        break;
      case "ArrowRight":
        if (selection.length) nudge(step, 0);
        else goTo(index + 1);
        break;
      case "ArrowUp":
        if (selection.length) nudge(0, -step);
        else goTo(index - 1);
        break;
      case "ArrowDown":
        if (selection.length) nudge(0, step);
        else goTo(index + 1);
        break;
      case "PageUp":
        goTo(index - 1);
        break;
      case "PageDown":
        goTo(index + 1);
        break;
      case "Enter":
      case "F2":
        if (single) startEditing(single.id);
        break;
      default:
        if (mod && event.key.toLowerCase() === "c") copySelection();
        else if (mod && event.key.toLowerCase() === "x") {
          copySelection();
          deleteSelection();
        } else if (mod && event.key.toLowerCase() === "v") paste();
        else if (mod && event.key.toLowerCase() === "a") setSelection(slide.elements.map((element) => element.id));
        else return;
    }
    event.preventDefault();
  };

  const onPaste = async (event: ReactClipboardEvent<HTMLDivElement>) => {
    if (editingId || (event.target as HTMLElement).closest("input, textarea")) return;
    const file = [...event.clipboardData.files].find((item) => item.type.startsWith("image/"));
    if (!file) return;
    event.preventDefault();
    if (!["image/png", "image/jpeg", "image/svg+xml"].includes(file.type)) {
      notify(t("Eingefügte Bilder bitte als PNG, JPEG oder SVG – andere Formate über „Bild“ einfügen."));
      return;
    }
    const src = await readFileAsDataUrl(file);
    await addImage(src, null, null);
  };

  // ------------------------------------------------------------ Miniaturen

  const thumbScale = 168 / size.w;
  const [dragSlide, setDragSlide] = useState<number | null>(null);
  const onThumbDrop = (event: ReactDragEvent<HTMLLIElement>, target: number) => {
    event.preventDefault();
    if (dragSlide !== null) moveSlide(dragSlide, target);
    setDragSlide(null);
  };

  // ------------------------------------------------------------ Ansicht

  const selectionOverlay = selectedElements.map((element) => {
    const isLine = element.shape === "line" || element.shape === "arrow";
    const style: CSSProperties = elementStyle(element, scale);
    if (isLine) {
      const p1 = { x: element.flipH ? element.w : 0, y: element.flipV ? element.h : 0 };
      const p2 = { x: element.flipH ? 0 : element.w, y: element.flipV ? 0 : element.h };
      return (
        <div key={element.id} className="slide-selection line" style={style}>
          {single && (
            <>
              <span className="slide-handle endpoint" data-handle="start" style={{ left: p1.x * scale, top: p1.y * scale }} />
              <span className="slide-handle endpoint" data-handle="end" style={{ left: p2.x * scale, top: p2.y * scale }} />
            </>
          )}
        </div>
      );
    }
    return (
      <div key={element.id} className={`slide-selection${editingId === element.id ? " editing" : ""}`} style={style}>
        {single && editingId !== element.id && (
          <>
            {HANDLES.map((handle) => (
              <span key={handle} className={`slide-handle ${handle}`} data-handle={handle} />
            ))}
            <span className="slide-handle rotate" data-handle="rotate" title={t("Drehen (Umschalt: 15°-Schritte)")}>
              <RotateCw size={10} />
            </span>
          </>
        )}
      </div>
    );
  });

  const gridStyle: CSSProperties | undefined = deck.grid.show
    ? {
        backgroundImage:
          "linear-gradient(to right, var(--slide-grid) 1px, transparent 1px), linear-gradient(to bottom, var(--slide-grid) 1px, transparent 1px)",
        backgroundSize: `${deck.grid.size * scale}px ${deck.grid.size * scale}px`,
      }
    : undefined;

  const textTargets = editingId ? true : selectedElements.some((element) => element.kind === "text" || element.kind === "shape");
  const placeholderText = (element: SlideElement) =>
    element.role === "title" ? t("Titel hinzufügen") : element.role === "subtitle" ? t("Untertitel hinzufügen") : t("Text hinzufügen");

  const fontSizeValue = single?.fontSize ?? 18;
  const setFontSize = (value: number) => {
    if (editingId && textEditor && !textEditor.state.selection.empty) {
      textEditor.chain().focus().setFontSize(`${value}pt`).run();
      return;
    }
    if (selection.length) updateElements(selection, { fontSize: value });
  };

  const tabs: Array<{ id: TabId; label: string }> = [
    { id: "start", label: t("Start") },
    { id: "insert", label: t("Einfügen") },
    { id: "design", label: t("Entwurf") },
    { id: "arrange", label: t("Anordnen") },
    { id: "present", label: t("Bildschirmpräsentation") },
  ];

  const historyGroup = (
    <Group label={t("Verlauf")}>
      <div className="ribbon-stack">
        <ToolButton label={t("Rückgängig")} icon={<Undo2 size={16} />} disabled={history.current.past.length === 0} onClick={undo} />
        <ToolButton label={t("Wiederholen")} icon={<Redo2 size={16} />} disabled={history.current.future.length === 0} onClick={redo} />
      </div>
    </Group>
  );

  const shapeMenu = (
    <Menu label={t("Formen")} icon={<Square size={20} />} large>
      {(closeMenu) => (
        <div className="slide-shape-menu">
          {SHAPES.map((entry) => (
            <MenuItem key={entry.shape} label={t(entry.label)} icon={entry.icon} onClick={() => { closeMenu(); setTool({ kind: "shape", shape: entry.shape }); }} />
          ))}
        </div>
      )}
    </Menu>
  );

  const newSlideMenu = (
    <Menu label={t("Neue Folie")} icon={<Plus size={20} />} large>
      {(closeMenu) => (
        <>
          {LAYOUTS.map((layout) => (
            <MenuItem key={layout.id} label={t(layout.label)} icon={<LayoutTemplate size={16} />} onClick={() => { closeMenu(); addSlide(layout.id); }} />
          ))}
        </>
      )}
    </Menu>
  );

  let groups: ReactNode;
  if (tab === "start") {
    groups = (
      <>
        {historyGroup}
        <Group label={t("Folien")}>
          {newSlideMenu}
          <Menu label={t("Layout")} icon={<LayoutTemplate size={20} />} large>
            {(closeMenu) => (
              <>
                {LAYOUTS.map((layout) => (
                  <MenuItem key={layout.id} label={t(layout.label)} onClick={() => { closeMenu(); applyLayout(layout.id); }} />
                ))}
              </>
            )}
          </Menu>
          <div className="ribbon-stack">
            <ToolButton label={t("Folie duplizieren")} icon={<Copy size={16} />} onClick={duplicateSlide} />
            <ToolButton label={t("Folie löschen")} icon={<Trash2 size={16} />} onClick={() => deleteSlide()} />
          </div>
        </Group>
        <Group label={t("Schriftart")}>
          <div className="ribbon-stack">
            <div className="ribbon-row">
              <select
                className="ribbon-select narrow"
                value={FONT_SIZES.includes(fontSizeValue) ? String(fontSizeValue) : ""}
                disabled={!textTargets}
                aria-label={t("Schriftgröße")}
                onChange={(event) => setFontSize(Number(event.currentTarget.value))}
              >
                {!FONT_SIZES.includes(fontSizeValue) && <option value="">{fontSizeValue}</option>}
                {FONT_SIZES.map((value) => (
                  <option key={value} value={value}>{value}</option>
                ))}
              </select>
              <label className="ribbon-color" title={t("Schriftfarbe")}>
                <span className="slide-color-letter" style={{ borderBottomColor: single?.color || deck.theme.textColor }}>A</span>
                <input
                  type="color"
                  disabled={!textTargets}
                  aria-label={t("Schriftfarbe")}
                  onChange={(event) => {
                    const color = event.currentTarget.value;
                    if (editingId && textEditor && !textEditor.state.selection.empty) textEditor.chain().focus().setColor(color).run();
                    else if (selection.length) updateElements(selection, { color });
                  }}
                />
              </label>
            </div>
            <div className="ribbon-row">
              <ToolButton label={t("Fett")} icon={<Bold size={16} />} active={Boolean(editingId && textState?.bold)} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().toggleBold().run())} />
              <ToolButton label={t("Kursiv")} icon={<Italic size={16} />} active={Boolean(editingId && textState?.italic)} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().toggleItalic().run())} />
              <ToolButton label={t("Unterstrichen")} icon={<Underline size={16} />} active={Boolean(editingId && textState?.underline)} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().toggleUnderline().run())} />
              <ToolButton
                label={t("Formel im Text")}
                icon={<Sigma size={16} />}
                disabled={!editingId}
                onClick={() => textEditor?.chain().focus().insertContent({ type: "inlineMath", attrs: { latex: "x^2" } }).run()}
              />
            </div>
          </div>
        </Group>
        <Group label={t("Absatz")}>
          <div className="ribbon-stack">
            <div className="ribbon-row">
              <ToolButton label={t("Linksbündig")} icon={<AlignLeft size={16} />} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().setTextAlign("left").run())} />
              <ToolButton label={t("Zentriert")} icon={<AlignCenter size={16} />} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().setTextAlign("center").run())} />
              <ToolButton label={t("Rechtsbündig")} icon={<AlignRight size={16} />} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().setTextAlign("right").run())} />
            </div>
            <div className="ribbon-row">
              <ToolButton label={t("Aufzählung")} icon={<List size={16} />} active={Boolean(editingId && textState?.bullet)} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().toggleBulletList().run())} />
              <ToolButton label={t("Nummerierung")} icon={<ListOrdered size={16} />} active={Boolean(editingId && textState?.ordered)} disabled={!textTargets} onClick={() => formatText((editor) => editor.chain().focus().toggleOrderedList().run())} />
              <ToolButton label={t("Oben")} icon={<ArrowUpToLine size={16} />} disabled={!single} onClick={() => single && updateElements([single.id], { verticalAlign: "top" })} />
              <ToolButton label={t("Unten")} icon={<ArrowDownToLine size={16} />} disabled={!single} onClick={() => single && updateElements([single.id], { verticalAlign: "bottom" })} />
            </div>
          </div>
        </Group>
        <Group label={t("Zeichnen")}>
          <ToolButton large label={t("Textfeld")} icon={<Type size={20} />} active={tool?.kind === "text"} onClick={() => setTool(tool?.kind === "text" ? null : { kind: "text" })} />
          {shapeMenu}
        </Group>
      </>
    );
  } else if (tab === "insert") {
    groups = (
      <>
        <Group label={t("Folien")}>{newSlideMenu}</Group>
        <Group label={t("Text")}>
          <ToolButton large label={t("Textfeld")} icon={<Type size={20} />} active={tool?.kind === "text"} onClick={() => setTool(tool?.kind === "text" ? null : { kind: "text" })} />
          <ToolButton large label={t("Formel")} icon={<Sigma size={20} />} onClick={insertFormula} />
        </Group>
        <Group label={t("Illustrationen")}>
          <ToolButton large label={t("Bild")} icon={<ImagePlus size={20} />} onClick={() => void insertImage()} />
          {shapeMenu}
        </Group>
      </>
    );
  } else if (tab === "design") {
    groups = (
      <>
        <Group label={t("Foliengröße")}>
          <ToolButton large label="16:9" icon={<span className="slide-aspect wide" />} active={deck.aspect === "16:9"} onClick={() => setAspect("16:9")} />
          <ToolButton large label="4:3" icon={<span className="slide-aspect" />} active={deck.aspect === "4:3"} onClick={() => setAspect("4:3")} />
        </Group>
        <Group label={t("Design")}>
          <div className="slide-theme-fields">
            <ColorField label={t("Hintergrund")} value={deck.theme.background} allowNone={false} noneLabel="" onChange={(background) => commit({ ...deck, theme: { ...deck.theme, background } })} />
            <ColorField label={t("Text")} value={deck.theme.textColor} allowNone={false} noneLabel="" onChange={(textColor) => commit({ ...deck, theme: { ...deck.theme, textColor } })} />
            <ColorField label={t("Akzent")} value={deck.theme.accent} allowNone={false} noneLabel="" onChange={(accent) => commit({ ...deck, theme: { ...deck.theme, accent } })} />
            <label className="slide-field">
              <span>{t("Schrift")}</span>
              <select value={deck.theme.font} onChange={(event) => commit({ ...deck, theme: { ...deck.theme, font: event.currentTarget.value as "sans" | "serif" } })}>
                <option value="sans">{t("Serifenlos")}</option>
                <option value="serif">{t("Mit Serifen")}</option>
              </select>
            </label>
          </div>
        </Group>
        <Group label={t("Diese Folie")}>
          <div className="slide-theme-fields">
            <ColorField label={t("Folienhintergrund")} value={slide.background} noneLabel={t("Design")} onChange={(background) => updateSlide((item) => ({ ...item, background }))} />
          </div>
        </Group>
      </>
    );
  } else if (tab === "arrange") {
    groups = (
      <>
        <Group label={t("Anordnen")}>
          <div className="ribbon-stack">
            <div className="ribbon-row">
              <ToolButton label={t("In den Vordergrund")} icon={<BringToFront size={16} />} disabled={!selection.length} onClick={() => reorder("front")} />
              <ToolButton label={t("Eine Ebene nach vorne")} icon={<ArrowUpToLine size={16} />} disabled={!selection.length} onClick={() => reorder("forward")} />
            </div>
            <div className="ribbon-row">
              <ToolButton label={t("In den Hintergrund")} icon={<SendToBack size={16} />} disabled={!selection.length} onClick={() => reorder("back")} />
              <ToolButton label={t("Eine Ebene nach hinten")} icon={<ArrowDownToLine size={16} />} disabled={!selection.length} onClick={() => reorder("backward")} />
            </div>
          </div>
        </Group>
        <Group label={t("Ausrichten")}>
          <div className="ribbon-stack">
            <div className="ribbon-row">
              <ToolButton label={t("Links ausrichten")} icon={<AlignStartVertical size={16} />} disabled={!selection.length} onClick={() => align("left")} />
              <ToolButton label={t("Horizontal zentrieren")} icon={<AlignCenterVertical size={16} />} disabled={!selection.length} onClick={() => align("center")} />
              <ToolButton label={t("Rechts ausrichten")} icon={<AlignEndVertical size={16} />} disabled={!selection.length} onClick={() => align("right")} />
              <ToolButton label={t("Horizontal verteilen")} icon={<AlignHorizontalSpaceAround size={16} />} disabled={selection.length < 3} onClick={() => distribute("horizontal")} />
            </div>
            <div className="ribbon-row">
              <ToolButton label={t("Oben ausrichten")} icon={<AlignStartHorizontal size={16} />} disabled={!selection.length} onClick={() => align("top")} />
              <ToolButton label={t("Vertikal zentrieren")} icon={<AlignCenterHorizontal size={16} />} disabled={!selection.length} onClick={() => align("middle")} />
              <ToolButton label={t("Unten ausrichten")} icon={<AlignEndHorizontal size={16} />} disabled={!selection.length} onClick={() => align("bottom")} />
              <ToolButton label={t("Vertikal verteilen")} icon={<AlignVerticalSpaceAround size={16} />} disabled={selection.length < 3} onClick={() => distribute("vertical")} />
            </div>
          </div>
        </Group>
        <Group label={t("Raster")}>
          <ToolButton large label={t("Raster")} icon={<Grid3x3 size={20} />} active={deck.grid.show} onClick={() => commit({ ...deck, grid: { ...deck.grid, show: !deck.grid.show } }, false)} />
          <ToolButton large label={t("Am Raster ausrichten")} icon={<Magnet size={20} />} active={deck.grid.snap} onClick={() => commit({ ...deck, grid: { ...deck.grid, snap: !deck.grid.snap } }, false)} />
          <div className="ribbon-stack">
            <label className="slide-field compact">
              <span>{t("Rasterweite")}</span>
              <select value={String(deck.grid.size)} onChange={(event) => commit({ ...deck, grid: { ...deck.grid, size: Number(event.currentTarget.value) } }, false)}>
                {GRID_SIZES.map((value) => (
                  <option key={value} value={value}>{String(value).replace(".", ",")} mm</option>
                ))}
              </select>
            </label>
            <label className="checkbox compact">
              <input type="checkbox" checked={deck.grid.guides} onChange={(event) => commit({ ...deck, grid: { ...deck.grid, guides: event.currentTarget.checked } }, false)} />
              {t("Intelligente Hilfslinien")}
            </label>
          </div>
        </Group>
      </>
    );
  } else {
    groups = (
      <Group label={t("Bildschirmpräsentation")}>
        <ToolButton large label={t("Von Beginn an")} icon={<Play size={20} />} onClick={() => setPresenting(0)} />
        <ToolButton large label={t("Ab aktueller Folie")} icon={<Play size={20} />} onClick={() => setPresenting(index)} />
        <ToolButton large label={slide.hidden ? t("Folie einblenden") : t("Folie ausblenden")} icon={slide.hidden ? <Eye size={20} /> : <EyeOff size={20} />} onClick={() => updateSlide((item) => ({ ...item, hidden: !item.hidden }))} />
      </Group>
    );
  }

  // ------------------------------------------------------------ Seitenleiste „Format“

  const formatPanel = (
    <aside className="slide-format-panel" aria-label={t("Format")}>
      {selectedElements.length === 0 ? (
        <>
          <h3>{t("Folie")} {index + 1}</h3>
          <ColorField label={t("Hintergrund")} value={slide.background} noneLabel={t("Design")} onChange={(background) => updateSlide((item) => ({ ...item, background }))} />
          <label className="checkbox">
            <input type="checkbox" checked={slide.hidden} onChange={(event) => { const hidden = event.currentTarget.checked; updateSlide((item) => ({ ...item, hidden })); }} />
            {t("In der Präsentation ausblenden")}
          </label>
          <p className="muted small">{t("Doppelklick auf ein Textfeld bearbeitet den Text. Ziehen verschiebt (Alt: ohne Raster), Umschalt beim Skalieren hält das Seitenverhältnis.")}</p>
        </>
      ) : single ? (
        <>
          <h3>{t(single.kind === "text" ? "Textfeld" : single.kind === "shape" ? "Form" : single.kind === "image" ? "Bild" : "Formel")}</h3>
          <div className="slide-field-grid">
            <NumberField label="X" unit="mm" value={single.x} onChange={(x) => updateElements([single.id], { x })} />
            <NumberField label="Y" unit="mm" value={single.y} onChange={(y) => updateElements([single.id], { y })} />
            <NumberField label={t("Breite")} unit="mm" min={0} value={single.w} onChange={(w) => updateElements([single.id], { w })} />
            <NumberField label={t("Höhe")} unit="mm" min={0} value={single.h} onChange={(h) => updateElements([single.id], { h })} />
            <NumberField label={t("Drehung")} unit="°" step={1} min={-360} max={360} value={single.rotation} onChange={(rotation) => updateElements([single.id], { rotation })} />
          </div>
          {single.kind === "formula" && (
            <>
              <label className="slide-field block">
                <span>{t("LaTeX-Formel")}</span>
                <textarea
                  ref={formulaInputRef}
                  rows={3}
                  spellCheck={false}
                  defaultValue={single.latex}
                  key={single.id}
                  onBlur={(event) => {
                    const latex = event.currentTarget.value;
                    if (latex === single.latex) return;
                    const measured = measureFormula(latex, single.fontSize);
                    const cx = single.x + single.w / 2;
                    const cy = single.y + single.h / 2;
                    updateElements([single.id], { latex, ...measured, x: round(cx - measured.w / 2), y: round(cy - measured.h / 2) });
                  }}
                />
              </label>
              <NumberField
                label={t("Schriftgröße")}
                unit="pt"
                step={1}
                min={6}
                max={200}
                value={single.fontSize}
                onChange={(fontSize) => {
                  const measured = measureFormula(single.latex, fontSize);
                  updateElements([single.id], { fontSize, ...measured });
                }}
              />
              <ColorField label={t("Farbe")} value={single.color} noneLabel={t("Design")} onChange={(color) => updateElements([single.id], { color })} />
            </>
          )}
          {(single.kind === "shape" || single.kind === "text") && (
            <>
              {single.shape !== "line" && single.shape !== "arrow" && (
                <ColorField label={t("Füllung")} value={single.fill} noneLabel={t("Keine")} onChange={(fill) => updateElements([single.id], { fill })} />
              )}
              <ColorField label={t("Linie")} value={single.stroke} noneLabel={t("Keine")} onChange={(stroke) => updateElements([single.id], { stroke })} />
              <NumberField label={t("Linienstärke")} unit="pt" step={0.25} min={0} max={20} value={single.strokeWidth} onChange={(strokeWidth) => updateElements([single.id], { strokeWidth })} />
              {single.kind === "shape" && (
                <label className="slide-field">
                  <span>{t("Form")}</span>
                  <select value={single.shape} onChange={(event) => updateElements([single.id], { shape: event.currentTarget.value as ShapeKind })}>
                    {SHAPES.map((entry) => (
                      <option key={entry.shape} value={entry.shape}>{t(entry.label)}</option>
                    ))}
                  </select>
                </label>
              )}
              {single.shape !== "line" && single.shape !== "arrow" && (
                <>
                  <NumberField label={t("Schriftgröße")} unit="pt" step={1} min={4} max={200} value={single.fontSize} onChange={(fontSize) => updateElements([single.id], { fontSize })} />
                  <ColorField label={t("Textfarbe")} value={single.color} noneLabel={t("Design")} onChange={(color) => updateElements([single.id], { color })} />
                  <label className="slide-field">
                    <span>{t("Vertikal")}</span>
                    <select value={single.verticalAlign} onChange={(event) => updateElements([single.id], { verticalAlign: event.currentTarget.value as SlideElement["verticalAlign"] })}>
                      <option value="top">{t("Oben")}</option>
                      <option value="middle">{t("Mitte")}</option>
                      <option value="bottom">{t("Unten")}</option>
                    </select>
                  </label>
                  <NumberField label={t("Innenabstand")} unit="mm" step={0.5} min={0} max={20} value={single.padding} onChange={(padding) => updateElements([single.id], { padding })} />
                </>
              )}
            </>
          )}
          {single.kind === "image" && (
            <div className="button-row left">
              <button
                type="button"
                className="small"
                onClick={async () => {
                  if (!isTauri) return;
                  const selected = await open({ multiple: false, filters: [{ name: t("Bilder"), extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "svg", "pdf"] }] });
                  if (typeof selected !== "string") return;
                  try {
                    const image = await api.slidesReadImage(selected);
                    updateElements([single.id], { src: image.src });
                  } catch (error) {
                    notify(errorText(error));
                  }
                }}
              >
                {t("Bild ersetzen …")}
              </button>
            </div>
          )}
          <div className="button-row left">
            <button type="button" className="small" onClick={duplicateSelection}>{t("Duplizieren")}</button>
            <button type="button" className="small danger" onClick={deleteSelection}>{t("Löschen")}</button>
          </div>
        </>
      ) : (
        <>
          <h3>{selectedElements.length} {t("Elemente ausgewählt")}</h3>
          <ColorField label={t("Füllung")} value={selectedElements[0].fill} noneLabel={t("Keine")} onChange={(fill) => updateElements(selection, { fill })} />
          <ColorField label={t("Linie")} value={selectedElements[0].stroke} noneLabel={t("Keine")} onChange={(stroke) => updateElements(selection, { stroke })} />
          <div className="button-row left">
            <button type="button" className="small" onClick={duplicateSelection}>{t("Duplizieren")}</button>
            <button type="button" className="small danger" onClick={deleteSelection}>{t("Löschen")}</button>
          </div>
        </>
      )}
    </aside>
  );

  const fileName = session.path ? baseName(session.path) : t("Neue Präsentation");

  return (
    <div className="slide-editor" role="dialog" aria-label={t("Folien-Editor")} onKeyDown={onKeyDown} onPaste={(event) => void onPaste(event)}>
      <header className="slide-editor-header">
        <strong>{t("Folien")}</strong>
        <span className="slide-editor-file">{fileName}{session.dirty ? " •" : ""}</span>
        <span className="spacer" />
        <button type="button" className="primary small" onClick={() => setPresenting(index)}>
          <Play size={14} /> {t("Präsentieren")}
        </button>
        <button type="button" className="slide-back" onClick={() => void close()} title={t("Folien-Editor schließen – die Präsentation bleibt geöffnet")}>
          <FileText size={15} /> {t("Zum Dokument")}
        </button>
      </header>
      <div className="ribbon slide-ribbon">
        <nav className="ribbon-tabs slide-tabs" role="tablist">
          <button type="button" className="ribbon-tab file-tab" onClick={onFileMenu}>
            {t("Datei")}
          </button>
          {tabs.map((entry) => (
            <button key={entry.id} type="button" role="tab" aria-selected={tab === entry.id} className={`ribbon-tab${tab === entry.id ? " active" : ""}`} onClick={() => setTab(entry.id)}>
              {entry.label}
            </button>
          ))}
        </nav>
        <div className="ribbon-content">{groups}</div>
      </div>

      <div className="slide-workspace">
        <ol className="slide-thumbnails" aria-label={t("Folien")}>
          {deck.slides.map((item, itemIndex) => (
            <li
              key={item.id}
              className={`slide-thumb${itemIndex === index ? " active" : ""}${item.hidden ? " hidden-slide" : ""}${dragSlide !== null && dragSlide !== itemIndex ? " drop-target" : ""}`}
              draggable
              onDragStart={() => setDragSlide(itemIndex)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => onThumbDrop(event, itemIndex)}
              onDragEnd={() => setDragSlide(null)}
              onClick={() => goTo(itemIndex)}
              onKeyDown={(event) => {
                if (event.key === "Delete") {
                  event.stopPropagation();
                  deleteSlide(itemIndex);
                }
              }}
              tabIndex={0}
              aria-current={itemIndex === index}
              aria-label={`${t("Folie")} ${itemIndex + 1}`}
            >
              <span className="slide-thumb-number">{itemIndex + 1}</span>
              <div className="slide-thumb-frame">
                <SlideView slide={item} deck={deck} scale={thumbScale} />
              </div>
              {item.hidden && <EyeOff size={12} className="slide-thumb-hidden" />}
            </li>
          ))}
          <li className="slide-thumb-add">
            <button type="button" onClick={() => addSlide("titleContent")}>
              <Plus size={14} /> {t("Neue Folie")}
            </button>
          </li>
        </ol>
        <div className="slide-main">
          <div className="slide-stage" ref={stageRef}>
            <div
              ref={canvasRef}
              className={`slide-canvas${tool ? " drawing" : ""}`}
              style={{ width: size.w * scale, height: size.h * scale, ["--slide-scale" as string]: scale }}
              onPointerDown={onCanvasPointerDown}
              onDoubleClick={onCanvasDoubleClick}
              tabIndex={0}
              aria-label={t("Folie bearbeiten")}
            >
              <SlideView
                slide={slide}
                deck={deck}
                scale={scale}
                editing
                editingId={editingId}
                editor={textEditor ? <EditorContent editor={textEditor} /> : null}
                placeholderText={placeholderText}
              />
              {gridStyle && <div className="slide-grid" style={gridStyle} />}
              <div className="slide-overlay">
                {selectionOverlay}
                {guides.map((guide, guideIndex) => (
                  <div
                    key={guideIndex}
                    className={`slide-guide ${guide.orientation}`}
                    style={guide.orientation === "vertical" ? { left: guide.position * scale } : { top: guide.position * scale }}
                  />
                ))}
                {marquee && <div className="slide-marquee" style={{ left: marquee.x * scale, top: marquee.y * scale, width: marquee.w * scale, height: marquee.h * scale }} />}
                {drawing && (
                  drawing.w < 0 || drawing.h < 0 || tool?.shape === "line" || tool?.shape === "arrow" ? (
                    <svg className="slide-drawing-line" width={size.w * scale} height={size.h * scale}>
                      <line x1={drawing.x * scale} y1={drawing.y * scale} x2={(drawing.x + drawing.w) * scale} y2={(drawing.y + drawing.h) * scale} />
                    </svg>
                  ) : (
                    <div className="slide-drawing" style={{ left: drawing.x * scale, top: drawing.y * scale, width: drawing.w * scale, height: drawing.h * scale }} />
                  )
                )}
              </div>
            </div>
          </div>
          <label className="slide-notes">
            <span>{t("Notizen")}</span>
            <textarea
              value={slide.notes}
              placeholder={t("Notizen zur Folie (werden als \\note exportiert)")}
              onChange={(event) => {
                const notes = event.currentTarget.value;
                updateSlide((item) => ({ ...item, notes }), false);
              }}
            />
          </label>
        </div>
        {formatPanel}
      </div>
      <footer className="slide-statusbar">
        <span>{t("Folie")} {index + 1} {t("von")} {deck.slides.length}</span>
        {tool && <span className="slide-tool-hint">{t("Ziehen, um das Element aufzuziehen (Esc bricht ab)")}</span>}
        <span className="spacer" />
        {status && <span className="slide-status">{status}</span>}
        <span>{deck.grid.snap ? `${t("Raster")} ${String(deck.grid.size).replace(".", ",")} mm` : t("Raster aus")}</span>
        <select value={zoom === null ? "fit" : String(zoom)} onChange={(event) => setZoom(event.currentTarget.value === "fit" ? null : Number(event.currentTarget.value))} aria-label={t("Zoom")}>
          <option value="fit">{t("Anpassen")}</option>
          <option value="0.5">50 %</option>
          <option value="0.75">75 %</option>
          <option value="1.25">125 %</option>
          <option value="1.5">150 %</option>
          <option value="2">200 %</option>
        </select>
      </footer>
      {presenting !== null && (
        <Presenter
          deck={deck}
          start={deck.slides.slice(0, presenting).filter((item) => !item.hidden).length}
          onClose={() => setPresenting(null)}
        />
      )}
      <style>{`.slide-editor { --slide-font: ${FONT_STACK[deck.theme.font]}; }`}</style>
    </div>
  );
}
