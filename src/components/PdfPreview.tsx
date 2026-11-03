/**
 * PDF-Vorschau: Seiten werden in Rust (hayro) als PNG gerendert und nur bei
 * Sichtbarkeit angefordert. Kein PDF-Viewer/iframe im WebView nötig.
 * Doppelklick springt an die Stelle im Dokument (SyncTeX); „Im PDF zeigen“ scrollt hierher
 * und markiert die Stelle kurz (`reveal`).
 */
import { useEffect, useRef, useState } from "react";
import { CircleAlert, Download, LocateFixed, Maximize2, Play, TriangleAlert, ZoomIn, ZoomOut } from "lucide-react";
import { api } from "../api";
import type { CompileFailure, CompileResult, PdfPoint, TexMessage } from "../api";
import { useT } from "../i18n";
import { useCtrlWheel } from "./useCtrlWheel";

export type CompileStatus =
  | { state: "idle" }
  | { state: "compiling"; progress: string }
  | { state: "done"; result: CompileResult }
  | { state: "failed"; failure: CompileFailure; previous: CompileResult | null };

/** Kurz eingeblendete Markierung einer Stelle („Im PDF zeigen“) */
export type PdfReveal = PdfPoint & { token: number };

type PageProps = {
  pdfId: number;
  page: number;
  width: number;
  height: number;
  scale: number;
  /** CSS-Pixel je PDF-Punkt */
  cssScale: number;
  marker: PdfReveal | null;
  title: string;
  onDoubleClick: (page: number, x: number, y: number) => void;
};

function PageImage({ pdfId, page, width, height, scale, cssScale, marker, title, onDoubleClick }: PageProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [url, setUrl] = useState("");
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver((entries) => setVisible(entries.some((entry) => entry.isIntersecting)), { rootMargin: "600px" });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    let objectUrl = "";
    const timer = window.setTimeout(() => {
      void api
        .renderPdfPage(pdfId, page, scale)
        .then((blob) => {
          if (cancelled) return;
          objectUrl = URL.createObjectURL(blob);
          setUrl(objectUrl);
        })
        .catch(() => undefined);
    }, 30);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [visible, pdfId, page, scale]);
  return (
    <div
      className="pdf-page"
      ref={ref}
      style={{ width, height }}
      data-page={page}
      title={title}
      onDoubleClick={(event) => {
        const rect = event.currentTarget.getBoundingClientRect();
        onDoubleClick(page, (event.clientX - rect.left) / cssScale, (event.clientY - rect.top) / cssScale);
      }}
    >
      {url ? <img src={url} alt={`Seite ${page + 1}`} draggable={false} /> : <div className="pdf-page-loading" />}
      {marker && (
        <div
          key={marker.token}
          className="pdf-sync-marker"
          style={{ top: Math.max(0, marker.y * cssScale - 3), height: Math.max(14, marker.height * cssScale + 6) }}
        />
      )}
    </div>
  );
}

type Props = {
  status: CompileStatus;
  onCompile: () => void;
  onSavePdf: () => void;
  onMessageClick: (message: TexMessage) => void;
  /** Doppelklick auf eine Seite (Koordinaten in bp ab links oben) */
  onPageDoubleClick: (page: number, x: number, y: number) => void;
  /** Cursorstelle im PDF zeigen */
  onShowInPdf: () => void;
  reveal: PdfReveal | null;
};

export function PdfPreview({ status, onCompile, onSavePdf, onMessageClick, onPageDoubleClick, onShowInPdf, reveal }: Props) {
  const t = useT();
  const [zoom, setZoom] = useState(1);
  const [fitWidth, setFitWidth] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(600);
  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setContainerWidth(element.clientWidth));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const result = status.state === "done" ? status.result : status.state === "failed" ? status.previous : null;
  const messages = status.state === "done" ? status.result.messages : status.state === "failed" ? status.failure.messages : [];
  const errors = messages.filter((message) => message.severity === "error");
  const warnings = messages.filter((message) => message.severity === "warning");
  const [showMessages, setShowMessages] = useState(false);
  useEffect(() => {
    // Fehler anzeigen – auch wenn das PDF trotzdem entstanden ist (wie in TeXstudio/Overleaf)
    if (status.state === "failed" || (status.state === "done" && errors.length > 0)) setShowMessages(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status.state, errors.length]);

  const pageWidthPt = result?.pages[0]?.width ?? 595;
  const cssScale = fitWidth ? Math.max(0.2, (containerWidth - 32) / pageWidthPt) : zoom * (96 / 72);
  // Strg + Mausrad zoomt die Vorschau
  useCtrlWheel(containerRef, (direction) => {
    setFitWidth(false);
    setZoom((value) => Math.min(4, Math.max(0.3, (fitWidth ? cssScale / (96 / 72) : value) + direction * 0.1)));
  });
  const renderScale = Math.min(6, cssScale * (window.devicePixelRatio || 1));

  // „Im PDF zeigen“: Stelle ins obere Drittel scrollen (Markierung zeichnet die Seite)
  useEffect(() => {
    const container = containerRef.current;
    if (!reveal || !container) return;
    const page = container.querySelector<HTMLElement>(`.pdf-page[data-page="${reveal.page}"]`);
    if (!page) return;
    const top = page.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop;
    container.scrollTo({ top: Math.max(0, top + reveal.y * cssScale - container.clientHeight / 3), behavior: "smooth" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reveal?.token]);

  return (
    <div className="pdf-preview">
      <div className="pane-toolbar">
        <button type="button" className="primary small" onClick={onCompile} disabled={status.state === "compiling"} title={t("Kompilieren (F5)")}>
          <Play size={14} /> {status.state === "compiling" ? t("Kompiliere …") : t("Kompilieren")}
        </button>
        <span className="pane-status">
          {status.state === "compiling" && status.progress}
          {status.state === "done" && `${status.result.pages.length} ${t(status.result.pages.length === 1 ? "Seite" : "Seiten")} · ${(status.result.durationMs / 1000).toFixed(1)} s`}
          {status.state === "done" && errors.length > 0 && <span className="status-error"> · {t("PDF trotz Fehlern erzeugt")}</span>}
          {status.state === "failed" && <span className="status-error">{t("Fehler beim Kompilieren")}</span>}
        </span>
        <span className="spacer" />
        {(errors.length > 0 || warnings.length > 0) && (
          <button type="button" className={`small message-toggle${errors.length ? " has-errors" : ""}`} onClick={() => setShowMessages((value) => !value)}>
            {errors.length > 0 && (<><CircleAlert size={14} /> {errors.length}</>)}
            {warnings.length > 0 && (<><TriangleAlert size={14} /> {warnings.length}</>)}
          </button>
        )}
        <button type="button" className="icon-button" title={t("Verkleinern")} onClick={() => { setFitWidth(false); setZoom((value) => Math.max(0.3, (fitWidth ? cssScale / (96 / 72) : value) - 0.1)); }}>
          <ZoomOut size={16} />
        </button>
        <button type="button" className="icon-button" title={t("Vergrößern")} onClick={() => { setFitWidth(false); setZoom((value) => Math.min(4, (fitWidth ? cssScale / (96 / 72) : value) + 0.1)); }}>
          <ZoomIn size={16} />
        </button>
        <button type="button" className="icon-button" title={t("Cursorstelle im PDF zeigen (Strg+Alt+J)")} disabled={!result} onClick={onShowInPdf}>
          <LocateFixed size={16} />
        </button>
        <button type="button" className={`icon-button${fitWidth ? " active" : ""}`} title={t("An Breite anpassen")} onClick={() => setFitWidth(true)}>
          <Maximize2 size={16} />
        </button>
        <button type="button" className="icon-button" title={t("PDF speichern …")} disabled={!result} onClick={onSavePdf}>
          <Download size={16} />
        </button>
      </div>
      {showMessages && messages.length > 0 && (
        <div className="compile-messages">
          {status.state === "failed" && <div className="compile-headline">{status.failure.message}</div>}
          <ul>
            {[...errors, ...warnings].slice(0, 200).map((message, index) => (
              <li key={index} className={message.severity}>
                <button type="button" onClick={() => onMessageClick(message)} disabled={message.line === null}>
                  {message.severity === "error" ? <CircleAlert size={13} /> : <TriangleAlert size={13} />}
                  {message.line !== null && <span className="line">{t("Zeile")} {message.line}</span>}
                  <span>{message.message}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
      {status.state === "failed" && messages.length === 0 && (
        <div className="compile-messages">
          <div className="compile-headline">{status.failure.message}</div>
          {status.failure.log && <pre className="compile-log">{status.failure.log.split("\n").slice(-60).join("\n")}</pre>}
        </div>
      )}
      <div className="pdf-pages" ref={containerRef}>
        {result ? (
          result.pages.map((page, index) => (
            <PageImage
              key={`${result.pdfId}-${index}`}
              pdfId={result.pdfId}
              page={index}
              width={Math.round(page.width * cssScale)}
              height={Math.round(page.height * cssScale)}
              scale={renderScale}
              cssScale={cssScale}
              marker={reveal && reveal.page === index ? reveal : null}
              title={t("Doppelklick: zur Stelle im Dokument springen")}
              onDoubleClick={onPageDoubleClick}
            />
          ))
        ) : (
          <div className="pdf-empty">
            <strong>{t("PDF-Vorschau")}</strong>
            <p>{t("Mit „Kompilieren“ (F5) wird das Dokument mit der eingebauten TeX-Engine gesetzt – ohne externe LaTeX-Installation.")}</p>
          </div>
        )}
      </div>
    </div>
  );
}
