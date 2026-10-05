# Mitwirken / Contributing

Danke für dein Interesse! Fehlerberichte, Übersetzungen und Pull Requests sind willkommen.

## Fehler melden

Bitte mit Betriebssystem, VisuTeX-Version und – wenn möglich – einem kleinen LaTeX-Beispiel, das das Problem zeigt.
Private Dokumente bitte nicht anhängen.

## Code beitragen

1. Einrichten wie in [docs/ENTWICKLUNG.md](docs/ENTWICKLUNG.md) beschrieben (`npm install`, `npm run setup`).
2. Vor dem Pull Request:
   ```bash
   npm run typecheck && npm test && npm run i18n
   cd src-tauri && cargo fmt --check && cargo test --lib && cargo test --test core_roundtrip && cargo test --test foreign_documents
   ```
3. Änderungen an LaTeX-Export oder -Import immer mit Tests absichern; neue Knotentypen in
   `tests/fixtures/full-document.json` aufnehmen.
4. Logik gehört nach Rust (`src-tauri/src/core`), die Oberfläche ruft sie über `src/api.ts` auf.

## Übersetzungen

Die Oberflächentexte stehen in `src/i18n/locales/<sprache>.json` (Schlüssel = deutscher Originaltext).
`npm run i18n` zeigt fehlende Texte; `node scripts/i18n-keys.mjs export texte.txt` liefert eine nummerierte Liste zum Übersetzen.
Neue Sprache: Datei anlegen und in `UI_LANGUAGES` (`src/i18n/index.ts`) eintragen.

## Lizenz

Beiträge stehen unter der MIT-Lizenz des Projekts.

---

*English:* set up as described in `docs/ENTWICKLUNG.md`, run the checks above before opening a pull request, cover
export/import changes with tests. UI strings live in `src/i18n/locales/<code>.json` (keys are the German source texts).
