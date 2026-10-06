// LaTeX-Kommentare als Randkommentare: Text gewinnen, bearbeiten, Trennlinien erhalten.
import { test } from "node:test";
import assert from "node:assert/strict";
import { commentText, isCommentOnly, isDividerOnly, newComment, replaceCommentText } from "../src/editor/comments.ts";

const block = [
  "% =====================================================================",
  "% Cover page (JAMK template)",
  "% =====================================================================",
].join("\n");

test("Kommentartext ohne % und Trennlinien", () => {
  assert.equal(commentText(block), "Cover page (JAMK template)");
  assert.equal(commentText("%% Zeile 1\n%%   eingerückt"), "Zeile 1\n  eingerückt");
  assert.equal(isDividerOnly("% ====="), true);
  assert.equal(isDividerOnly(block), false);
});

test("Bearbeiten ersetzt nur den Text, Trennlinien bleiben", () => {
  const edited = replaceCommentText(block, "Titelseite\nnach JAMK-Vorlage");
  assert.equal(
    edited,
    [
      "% =====================================================================",
      "% Titelseite",
      "% nach JAMK-Vorlage",
      "% =====================================================================",
    ].join("\n"),
  );
  assert.equal(newComment("Neu\n\nzweiter Absatz"), "% Neu\n%\n% zweiter Absatz");
});

test("Nur echte Kommentarblöcke gelten als Kommentar", () => {
  assert.equal(isCommentOnly(block), true);
  assert.equal(isCommentOnly(""), false, "leerer Block ist LaTeX-Code, kein Kommentar");
  assert.equal(isCommentOnly("% Hinweis\n\\vspace{1cm}"), false);
});
