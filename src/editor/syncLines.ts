/**
 * Zuordnung zwischen Zeilen des kompilierten Dokuments und den obersten Blöcken im Editor
 * (für SyncTeX: Doppelklick ins PDF ↔ „Im PDF zeigen“). `blockLines` liefert der Export:
 * je Block [erste, letzte Zeile], [0, 0] für Blöcke ohne Code.
 */
export type BlockLines = Array<[number, number]>;

/**
 * Block zu einer Zeile: der letzte Block, der an oder vor dieser Zeile beginnt. TeX verbucht
 * Absätze oft auf der Zeile, die sie beendet (Leerzeile danach) – die gehört noch zum Block
 * davor. Zeilen in der Präambel ergeben den ersten Block; ohne Zuordnung −1.
 */
export function blockForLine(blockLines: BlockLines, line: number): number {
  let found = -1;
  let first = -1;
  blockLines.forEach(([start], index) => {
    if (start <= 0) return;
    if (first < 0) first = index;
    if (start <= line) found = index;
  });
  return found >= 0 ? found : first;
}

/** Erste Zeile eines Blocks; Blöcke ohne Code nehmen die des nächsten Blocks davor (sonst danach). */
export function lineForBlock(blockLines: BlockLines, index: number): number | null {
  for (let candidate = index; candidate >= 0; candidate -= 1) {
    const start = blockLines[candidate]?.[0] ?? 0;
    if (start > 0) return start;
  }
  for (let candidate = index + 1; candidate < blockLines.length; candidate += 1) {
    const start = blockLines[candidate][0];
    if (start > 0) return start;
  }
  return null;
}
