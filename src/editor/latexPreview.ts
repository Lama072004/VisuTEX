/**
 * Kompilierte Vorschau für Roh-LaTeX-Blöcke (Tabellen, Titelseiten, eigene
 * Umgebungen …). Anfragen mehrerer Blöcke werden kurz gesammelt und in einem
 * einzigen Tectonic-Lauf mit der Präambel des Dokuments kompiliert (Rust:
 * `preview.rs`). Ergebnisse bleiben im Speicher – höchstens `MAX_CACHE_BYTES`, zuletzt benutzte zuerst.
 */
import { api, errorText } from "../api";
import type { BlockPreview } from "../api";
import { getRuntime } from "../state/runtime";

type Waiting = { latex: string; resolve: (preview: BlockPreview) => void };

const cache = new Map<string, BlockPreview>();
/** Obergrenze des Vorschau-Speichers (SVG-Text; jede bearbeitete Fassung wäre sonst für immer im RAM). */
const MAX_CACHE_BYTES = 24 * 1024 * 1024;
let cacheBytes = 0;
const sizeOf = (preview: BlockPreview) => ((preview.image?.length ?? 0) + (preview.error?.length ?? 0)) * 2;

/** Eintrag merken; älteste (am längsten unbenutzte) verwerfen, bis die Grenze eingehalten ist. */
function remember(key: string, preview: BlockPreview) {
  const previous = cache.get(key);
  if (previous) {
    cacheBytes -= sizeOf(previous);
    cache.delete(key);
  }
  cache.set(key, preview);
  cacheBytes += sizeOf(preview);
  for (const [oldest, value] of cache) {
    if (cacheBytes <= MAX_CACHE_BYTES || cache.size <= 1) break;
    cache.delete(oldest);
    cacheBytes -= sizeOf(value);
  }
}
const running = new Map<string, Promise<BlockPreview>>();
let queue: Waiting[] = [];
let timer: number | undefined;
let busy = false;

/** Präambel, mit der Vorschauen kompiliert werden (von der App gesetzt). */
let preambleProvider: () => Promise<string> = async () => "\\documentclass{article}";

export function setPreviewPreambleProvider(provider: () => Promise<string>) {
  preambleProvider = provider;
}

/** Kurzer Fingerabdruck (FNV-1a) für Cache-Schlüssel. */
function hash(text: string): string {
  let value = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    value ^= text.charCodeAt(index);
    value = Math.imul(value, 0x01000193);
  }
  return (value >>> 0).toString(16);
}

let preambleKey = "";

/** Nach Änderungen an der Präambel bzw. am Projektordner neu kompilieren. */
export function invalidatePreviews() {
  cache.clear();
  cacheBytes = 0;
  running.clear();
}

async function flush() {
  timer = undefined;
  if (busy) {
    timer = window.setTimeout(flush, 200);
    return;
  }
  const batch = queue;
  queue = [];
  if (batch.length === 0) return;
  busy = true;
  try {
    const preamble = await preambleProvider();
    const key = hash(preamble + (getRuntime().projectRoot ?? ""));
    if (key !== preambleKey) {
      preambleKey = key;
    }
    const unique = [...new Set(batch.map((entry) => entry.latex))];
    let previews: BlockPreview[];
    try {
      previews = await api.renderLatexPreviews(preamble, unique, getRuntime().projectRoot, getRuntime().allowOnline);
    } catch (error) {
      previews = unique.map(() => ({ image: null, error: errorText(error) }));
    }
    const byLatex = new Map(unique.map((latex, index) => [latex, previews[index] ?? { image: null, error: "Keine Vorschau." }]));
    for (const entry of batch) {
      const preview = byLatex.get(entry.latex) ?? { image: null, error: "Keine Vorschau." };
      remember(`${preambleKey}|${entry.latex}`, preview);
      entry.resolve(preview);
    }
  } finally {
    busy = false;
    if (queue.length && timer === undefined) timer = window.setTimeout(flush, 50);
  }
}

/** Sofort verfügbare Vorschau (aus dem Cache) oder null. */
export function cachedPreview(latex: string): BlockPreview | null {
  const key = `${preambleKey}|${latex}`;
  const preview = cache.get(key);
  if (!preview) return null;
  // als zuletzt benutzt ans Ende stellen
  cache.delete(key);
  cache.set(key, preview);
  return preview;
}

/** Vorschau anfordern (gesammelt, zwischengespeichert). */
export function requestPreview(latex: string): Promise<BlockPreview> {
  const cached = cachedPreview(latex);
  if (cached) return Promise.resolve(cached);
  const key = `${preambleKey}|${latex}`;
  const pending = running.get(key);
  if (pending) return pending;
  const promise = new Promise<BlockPreview>((resolve) => {
    queue.push({ latex, resolve });
    if (timer === undefined) timer = window.setTimeout(flush, 250);
  }).finally(() => running.delete(key));
  running.set(key, promise);
  return promise;
}

export { isCommentOnly } from "./comments";
