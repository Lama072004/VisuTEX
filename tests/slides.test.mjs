// Folien-Editor: Modell und Geometrie (Raster, Hilfslinien, Größe, Ausrichten, Ebenen).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  alignElements,
  cloneElements,
  createDeck,
  createElement,
  createSlide,
  distributeElements,
  normalizeDeck,
  reorderElements,
  resizeBox,
  slideSize,
  snapMove,
} from "../src/slides/model.ts";

const context = (patch = {}) => ({ grid: 5, guides: true, targets: [], slide: { w: 160, h: 90 }, threshold: 1.5, ...patch });

test("Layouts erzeugen Platzhalter innerhalb der Folie", () => {
  for (const aspect of ["16:9", "4:3"]) {
    const { w, h } = slideSize(aspect);
    for (const layout of ["title", "titleContent", "twoContent", "titleOnly", "section", "blank"]) {
      const slide = createSlide(layout, aspect);
      for (const element of slide.elements) {
        assert.ok(element.x >= 0 && element.y >= 0, `${layout}: ${element.x}/${element.y}`);
        assert.ok(element.x + element.w <= w + 0.01 && element.y + element.h <= h + 0.01, `${layout} ${aspect}`);
        assert.ok(element.role, `${layout}: Platzhalter ohne Rolle`);
      }
    }
  }
  assert.equal(createSlide("twoContent", "16:9").elements.filter((item) => item.role === "body").length, 2);
});

test("Verschieben rastet im Raster ein, Hilfslinien haben Vorrang", () => {
  const box = { x: 10, y: 10, w: 20, h: 10 };
  // Raster: 10 + 7.3 → 15 bzw. 20
  assert.deepEqual(snapMove(box, 7.3, 0.4, context({ guides: false })), { dx: 5, dy: 0, guides: [] });
  // Folienmitte (80): Elementmitte 10+10+59.4 = 79.4 → fängt auf 80
  const centered = snapMove(box, 59.4, 0, context());
  assert.equal(centered.dx, 60);
  assert.deepEqual(centered.guides[0], { orientation: "vertical", position: 80 });
  // Kante eines anderen Elements (x = 47) hat Vorrang vor dem Raster
  const edge = snapMove(box, 36.6, 0, context({ targets: [{ x: 47, y: 50, w: 10, h: 10 }] }));
  assert.equal(edge.dx, 37);
  // Ohne Fang: frei
  assert.deepEqual(snapMove(box, 1.234, 2.345, context({ grid: null, guides: false })), { dx: 1.23, dy: 2.35, guides: [] });
});

test("Größe ändern: Mindestgröße, Raster, Seitenverhältnis", () => {
  const start = { x: 10, y: 10, w: 40, h: 20 };
  assert.deepEqual(resizeBox(start, "e", 12, 0, { keepAspect: false, grid: 5, min: 2 }), { x: 10, y: 10, w: 50, h: 20 });
  assert.deepEqual(resizeBox(start, "w", 100, 0, { keepAspect: false, grid: null, min: 2 }), { x: 48, y: 10, w: 2, h: 20 });
  const corner = resizeBox(start, "se", 40, 0, { keepAspect: true, grid: null, min: 2 });
  assert.equal(corner.w / corner.h, 2);
  const nw = resizeBox(start, "nw", -10, -10, { keepAspect: true, grid: null, min: 2 });
  assert.equal(nw.x + nw.w, 50);
  assert.equal(nw.y + nw.h, 30);
});

test("Ausrichten, Verteilen, Ebenen, Duplizieren", () => {
  const a = createElement("shape", { x: 0, y: 0, w: 10, h: 10 });
  const b = createElement("shape", { x: 30, y: 20, w: 20, h: 10 });
  const c = createElement("shape", { x: 100, y: 40, w: 10, h: 10 });
  const ids = [a.id, b.id, c.id];
  // eine Auswahl: an der Folie
  assert.equal(alignElements([a], [a.id], "center", { w: 160, h: 90 })[0].x, 75);
  // mehrere: an der Auswahl
  const right = alignElements([a, b, c], ids, "right", { w: 160, h: 90 });
  assert.deepEqual(right.map((item) => item.x + item.w), [110, 110, 110]);
  const distributed = distributeElements([a, b, c], ids, "horizontal");
  assert.equal(distributed[1].x, 45);
  assert.deepEqual(reorderElements([a, b, c], [a.id], "front").map((item) => item.id), [b.id, c.id, a.id]);
  assert.deepEqual(reorderElements([a, b, c], [c.id], "backward").map((item) => item.id), [a.id, c.id, b.id]);
  const copies = cloneElements([a], 5);
  assert.notEqual(copies[0].id, a.id);
  assert.equal(copies[0].x, 5);
});

test("Beispieldatei und neue Präsentation sind gültig", () => {
  const fixture = JSON.parse(readFileSync(new URL("./fixtures/slides.json", import.meta.url), "utf8"));
  const deck = normalizeDeck(fixture);
  assert.equal(deck.slides.length, 3);
  assert.equal(deck.slides[1].elements.find((item) => item.id === "e8").kind, "formula");
  // fehlende Felder werden ergänzt
  assert.equal(deck.slides[0].elements[0].padding, 2);
  const fresh = createDeck();
  assert.equal(fresh.slides[0].elements.length, 2);
  assert.equal(normalizeDeck({ slides: [] }).slides.length, 1);
});
