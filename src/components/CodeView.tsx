/** LaTeX-Code-Ansicht (Monaco, lokal) mit Fehlermarkern aus dem Kompilierlog. */
import { useEffect, useRef } from "react";
import type { TexMessage } from "../api";
import { monaco, registerLatex } from "../monaco/setup";

type Props = {
  value: string;
  onChange: (value: string) => void;
  theme: "light" | "dark";
  messages: TexMessage[];
  revealLine: { line: number; token: number } | null;
  onCompile: () => void;
  onSave: () => void;
};

export default function CodeView({ value, onChange, theme, messages, revealLine, onCompile, onSave }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const callbacks = useRef({ onChange, onCompile, onSave });
  callbacks.current = { onChange, onCompile, onSave };

  useEffect(() => {
    registerLatex();
    const host = hostRef.current;
    if (!host) return;
    const editor = monaco.editor.create(host, {
      value,
      language: "latex",
      theme: theme === "dark" ? "visutex-dark" : "visutex-light",
      automaticLayout: true,
      minimap: { enabled: false },
      wordWrap: "on",
      fontSize: 14,
      lineNumbers: "on",
      renderWhitespace: "selection",
      scrollBeyondLastLine: false,
      tabSize: 2,
      unicodeHighlight: { ambiguousCharacters: false },
      padding: { top: 12 },
      fixedOverflowWidgets: true,
    });
    editorRef.current = editor;
    const subscription = editor.onDidChangeModelContent(() => callbacks.current.onChange(editor.getValue()));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => callbacks.current.onCompile());
    editor.addCommand(monaco.KeyCode.F5, () => callbacks.current.onCompile());
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => callbacks.current.onSave());
    editor.focus();
    return () => {
      subscription.dispose();
      editor.getModel()?.dispose();
      editor.dispose();
      editorRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && editor.getValue() !== value) {
      const model = editor.getModel();
      if (model) {
        editor.pushUndoStop();
        model.pushEditOperations([], [{ range: model.getFullModelRange(), text: value }], () => null);
        editor.pushUndoStop();
      }
    }
  }, [value]);

  useEffect(() => {
    monaco.editor.setTheme(theme === "dark" ? "visutex-dark" : "visutex-light");
  }, [theme]);

  useEffect(() => {
    const model = editorRef.current?.getModel();
    if (!model) return;
    const lineCount = model.getLineCount();
    monaco.editor.setModelMarkers(
      model,
      "tex",
      messages
        .filter((message) => message.line !== null && message.line >= 1 && message.line <= lineCount)
        .map((message) => ({
          severity: message.severity === "error" ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
          message: message.message,
          startLineNumber: message.line as number,
          endLineNumber: message.line as number,
          startColumn: 1,
          endColumn: model.getLineMaxColumn(message.line as number),
        })),
    );
  }, [messages, value]);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || !revealLine) return;
    editor.revealLineInCenter(revealLine.line);
    editor.setPosition({ lineNumber: revealLine.line, column: 1 });
    editor.focus();
  }, [revealLine]);

  return <div className="code-view" ref={hostRef} />;
}
