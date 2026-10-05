/**
 * Eigene Dialoge statt window.prompt/confirm/alert (Projektvorgabe).
 * Promise-basiert: `const values = await dialogs.form({...})`.
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { X } from "lucide-react";
import { useT } from "../i18n";

export type FieldValue = string | number | boolean;

export type Field =
  | {
      kind: "text";
      name: string;
      label: string;
      value?: string;
      placeholder?: string;
      pattern?: RegExp;
      patternMessage?: string;
      required?: boolean;
      multiline?: boolean;
      mono?: boolean;
      autoFocus?: boolean;
      list?: string[];
    }
  | { kind: "number"; name: string; label: string; value?: number; min?: number; max?: number; step?: number; unit?: string }
  | { kind: "select"; name: string; label: string; value?: string; options: Array<{ value: string; label: string }> }
  | { kind: "checkbox"; name: string; label: string; value?: boolean }
  | { kind: "info"; text: string };

export type FormSpec = {
  title: string;
  description?: string;
  fields: Field[];
  submitLabel?: string;
  validate?: (values: Record<string, FieldValue>) => string | null;
  wide?: boolean;
};

type ConfirmSpec = { title: string; message: string; confirmLabel?: string; cancelLabel?: string; danger?: boolean; thirdLabel?: string };

export type DialogApi = {
  form: (spec: FormSpec) => Promise<Record<string, FieldValue> | null>;
  /** true = bestätigt, false = abgebrochen, "third" = dritte Option (z. B. „Nicht speichern“) */
  confirm: (spec: ConfirmSpec) => Promise<boolean | "third">;
  alert: (title: string, message: string) => Promise<void>;
};

const DialogContext = createContext<DialogApi | null>(null);

export function useDialogs(): DialogApi {
  const api = useContext(DialogContext);
  if (!api) throw new Error("DialogProvider fehlt");
  return api;
}

type Pending =
  | { kind: "form"; spec: FormSpec; resolve: (values: Record<string, FieldValue> | null) => void }
  | { kind: "confirm"; spec: ConfirmSpec; resolve: (value: boolean | "third") => void }
  | { kind: "alert"; title: string; message: string; resolve: () => void };

/** Grundgerüst eines modalen Dialogs (auch von anderen Komponenten genutzt). */
export function Modal({
  title,
  onClose,
  children,
  footer,
  wide = false,
  className = "",
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
  className?: string;
}) {
  const t = useT();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const first = ref.current?.querySelector<HTMLElement>("[autofocus], input, select, textarea, button.primary");
    first?.focus();
    return () => previous?.focus?.();
  }, []);
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <div
        ref={ref}
        className={`modal${wide ? " modal-wide" : ""} ${className}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            onClose();
          }
        }}
      >
        <header className="modal-header">
          <h2>{title}</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label={t("Schließen")}>
            <X size={18} />
          </button>
        </header>
        <div className="modal-body">{children}</div>
        {footer && <footer className="modal-footer">{footer}</footer>}
      </div>
    </div>
  );
}

function initialValues(fields: Field[]): Record<string, FieldValue> {
  const values: Record<string, FieldValue> = {};
  for (const field of fields) {
    if (field.kind === "info") continue;
    if (field.kind === "checkbox") values[field.name] = field.value ?? false;
    else if (field.kind === "number") values[field.name] = field.value ?? 0;
    else if (field.kind === "select") values[field.name] = field.value ?? field.options[0]?.value ?? "";
    else values[field.name] = field.value ?? "";
  }
  return values;
}

function FormDialog({ spec, onDone }: { spec: FormSpec; onDone: (values: Record<string, FieldValue> | null) => void }) {
  const t = useT();
  const [values, setValues] = useState(() => initialValues(spec.fields));
  const [error, setError] = useState("");
  const submit = () => {
    for (const field of spec.fields) {
      if (field.kind !== "text") continue;
      const value = String(values[field.name] ?? "").trim();
      if (field.required && !value) {
        setError(`${t("Bitte ausfüllen:")} ${field.label}`);
        return;
      }
      if (value && field.pattern && !field.pattern.test(value)) {
        setError(field.patternMessage ?? `${t("Ungültige Eingabe:")} ${field.label}`);
        return;
      }
    }
    const message = spec.validate?.(values) ?? null;
    if (message) {
      setError(message);
      return;
    }
    onDone(values);
  };
  const set = (name: string, value: FieldValue) => {
    setValues((current) => ({ ...current, [name]: value }));
    setError("");
  };
  return (
    <Modal
      title={spec.title}
      wide={spec.wide}
      onClose={() => onDone(null)}
      footer={
        <>
          {error && <span className="form-error" role="alert">{error}</span>}
          <button type="button" onClick={() => onDone(null)}>{t("Abbrechen")}</button>
          <button type="button" className="primary" onClick={submit}>{spec.submitLabel ?? t("OK")}</button>
        </>
      }
    >
      <form
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        {spec.description && <p className="form-description">{spec.description}</p>}
        {spec.fields.map((field, index) => {
          if (field.kind === "info") return <p key={index} className="form-info">{field.text}</p>;
          if (field.kind === "checkbox") {
            return (
              <label key={field.name} className="checkbox form-checkbox">
                <input type="checkbox" checked={Boolean(values[field.name])} onChange={(event) => set(field.name, event.currentTarget.checked)} />
                {field.label}
              </label>
            );
          }
          return (
            <label key={field.name} className="form-field">
              <span>{field.label}</span>
              {field.kind === "select" ? (
                <select value={String(values[field.name])} onChange={(event) => set(field.name, event.currentTarget.value)}>
                  {field.options.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              ) : field.kind === "number" ? (
                <span className="input-with-unit">
                  <input
                    type="number"
                    value={Number(values[field.name])}
                    min={field.min}
                    max={field.max}
                    step={field.step ?? 1}
                    onChange={(event) => set(field.name, Number(event.currentTarget.value))}
                  />
                  {field.unit && <span className="unit">{field.unit}</span>}
                </span>
              ) : field.multiline ? (
                <textarea
                  className={field.mono ? "code-input" : ""}
                  value={String(values[field.name])}
                  placeholder={field.placeholder}
                  spellCheck={!field.mono}
                  rows={field.mono ? 8 : 4}
                  autoFocus={field.autoFocus}
                  onChange={(event) => set(field.name, event.currentTarget.value)}
                />
              ) : (
                <>
                  <input
                    className={field.mono ? "code-input" : ""}
                    value={String(values[field.name])}
                    placeholder={field.placeholder}
                    spellCheck={!field.mono}
                    autoFocus={field.autoFocus ?? index === 0}
                    list={field.list ? `list-${field.name}` : undefined}
                    onChange={(event) => set(field.name, event.currentTarget.value)}
                  />
                  {field.list && (
                    <datalist id={`list-${field.name}`}>
                      {field.list.map((item) => <option key={item} value={item} />)}
                    </datalist>
                  )}
                </>
              )}
            </label>
          );
        })}
        <button type="submit" hidden />
      </form>
    </Modal>
  );
}

export function DialogProvider({ children }: { children: ReactNode }) {
  const t = useT();
  const [queue, setQueue] = useState<Pending[]>([]);
  const push = useCallback((pending: Pending) => setQueue((current) => [...current, pending]), []);
  const finish = () => setQueue((current) => current.slice(1));

  const api = useMemo<DialogApi>(
    () => ({
      form: (spec) => new Promise((resolve) => push({ kind: "form", spec, resolve })),
      confirm: (spec) => new Promise((resolve) => push({ kind: "confirm", spec, resolve })),
      alert: (title, message) => new Promise((resolve) => push({ kind: "alert", title, message, resolve })),
    }),
    [push],
  );

  const current = queue[0];
  return (
    <DialogContext.Provider value={api}>
      {children}
      {current?.kind === "form" && (
        <FormDialog
          key={queue.length}
          spec={current.spec}
          onDone={(values) => {
            current.resolve(values);
            finish();
          }}
        />
      )}
      {current?.kind === "confirm" && (
        <Modal
          title={current.spec.title}
          onClose={() => {
            current.resolve(false);
            finish();
          }}
          footer={
            <>
              {current.spec.thirdLabel && (
                <button
                  type="button"
                  className="danger-text"
                  onClick={() => {
                    current.resolve("third");
                    finish();
                  }}
                >
                  {current.spec.thirdLabel}
                </button>
              )}
              <button
                type="button"
                onClick={() => {
                  current.resolve(false);
                  finish();
                }}
              >
                {current.spec.cancelLabel ?? t("Abbrechen")}
              </button>
              <button
                type="button"
                className={current.spec.danger ? "primary danger" : "primary"}
                autoFocus
                onClick={() => {
                  current.resolve(true);
                  finish();
                }}
              >
                {current.spec.confirmLabel ?? t("OK")}
              </button>
            </>
          }
        >
          <p className="dialog-message">{current.spec.message}</p>
        </Modal>
      )}
      {current?.kind === "alert" && (
        <Modal
          title={current.title}
          onClose={() => {
            current.resolve();
            finish();
          }}
          footer={
            <button
              type="button"
              className="primary"
              autoFocus
              onClick={() => {
                current.resolve();
                finish();
              }}
            >
              {t("OK")}
            </button>
          }
        >
          <p className="dialog-message">{current.message}</p>
        </Modal>
      )}
    </DialogContext.Provider>
  );
}
