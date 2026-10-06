/**
 * Kompilierte Vorschau für Roh-LaTeX-Blöcke (Tabellen, Titelseiten, eigene
 * Umgebungen …). Anfragen mehrerer Blöcke werden kurz gesammelt und in einem
 * einzigen Tectonic-Lauf mit der Präambel des Dokuments kompiliert (Rust:
 * `preview.rs`). Ergebnisse bleiben für die Sitzung im Speicher.
 */
import { api, errorText } from "../api";
import type { BlockPreview } from "../api";
import { getRuntime } from "../state/runtime";

type Waiting = { latex: string; resolve: (preview: BlockPreview) => void };

const cache = new Map<string, BlockPreview>();
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
      cache.set(`${preambleKey}|${entry.latex}`, preview);
      entry.resolve(preview);
    }
  } finally {
    busy = false;
    if (queue.length && timer === undefined) timer = window.setTimeout(flush, 50);
  }
}

/** Sofort verfügbare Vorschau (aus dem Cache) oder null. */
export function cachedPreview(latex: string): BlockPreview | null {
  return cache.get(`${preambleKey}|${latex}`) ?? null;
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
