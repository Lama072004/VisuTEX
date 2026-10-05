# CLAUDE.md – VisuTeX

VisuTeX ist ein WYSIWYG-LaTeX-Editor („wie Word“, zusätzlich reiner LaTeX-Code) mit Folien-Editor und Skizzen-Werkzeug,
auf Basis von **Tauri 2 + Rust (Tectonic als eingebaute TeX-Engine) + React/TypeScript/Tiptap 3**.

**Vor jeder Arbeit lesen:** `docs/ARCHITEKTUR.md` (Aufbau, Datenfluss, Ordner), `docs/ENTWICKLUNG.md` (Befehle, Tests,
Werkzeuge) und `VISUTEX_IMPLEMENTATION.md` (Status: funktioniert / offen). Bedienung: `docs/BEDIENUNG.md`.

## Verbindliche Vorgaben des Nutzers
- Keine extern installierte LaTeX-Distribution nötig – **alles wird mitgeliefert** (TeX-Bundle, Editor-Bibliotheken,
  Schriften); keine CDN-/Internet-Abhängigkeit zur Laufzeit. Was nicht mitgeliefert ist, prüft die App (Systemprüfung)
  bzw. `npm run setup`, bietet es an und **fragt nach dem Speicherort**.
- Code-Ansicht = **vollständiges, portables Dokument** (pdfLaTeX/XeLaTeX/LuaLaTeX, TeX Live/MiKTeX/Overleaf);
  Bearbeitung in Code- **und** visueller Ansicht (verlustfreier Round-Trip). Beliebige fremde LaTeX-Projekte öffnen und
  bearbeiten; was dort vorkommt, soll **über die UI machbar** sein.
- Literatur über **BibTeX intern** (natbib), **Zotero-Anbindung**. **Ribbon vereinfacht** (kein Anpassungsmodus).
- **Add-on-System** (installieren + selbst erstellen, deklarativ).
- **So viel wie möglich in Rust**; im Frontend nur WebView-Pflichtteile.
- **Windows und alle Linux-Distributionen** müssen funktionieren (macOS später); keine rechnerspezifischen Pfade.
- Folien-Editor nicht „PowerPoint“ nennen, keine fremden Marken/Logos in der Oberfläche.
- Nach jeder Arbeitsrunde **`VISUTEX_IMPLEMENTATION.md` aktualisieren** (Was wurde gemacht · Funktioniert · Offen).

## Wichtige Orte (Details: docs/ARCHITEKTUR.md)
- `src-tauri/src/core/` – getesteter LaTeX-Kern (export, import, preamble, settings, languages, slides, sketch, sketch_catalog …)
- `src-tauri/src/lib.rs` – dünne Tauri-Befehle; `compile.rs`, `prepare.rs`, `texbundle.rs`, `pdf.rs`, `preview.rs`, …
- `src-tauri/resources/` – `tex-bundle.zip`, `sketch-symbols.json`, mitgelieferte Add-ons
- `src/App.tsx` (Koordination), `src/api.ts` (Befehle), `src/components/`, `src/editor/`, `src/latex/`,
  `src/slides/`, `src/sketch/`, `src/shortcuts/`, `src/i18n/` (Übersetzungen in `locales/*.json`)
- `src/latex/document-languages.json` – Dokumentsprachen, gemeinsam für Rust und Oberfläche
- `tests/` (Node) und `src-tauri/tests/` (Round-Trip, Fremddokumente, Kompilieren)
- `Vorlagen_Test_TEX/` – private Dokumente des Nutzers (**nicht versioniert, nie veröffentlichen, nicht löschen**);
  versionierte synthetische Fremddokumente in `tests/fixtures/fremd/`.
- Quelltreue: importierte Blöcke tragen `sourceLatex`/`sourceKey`/`sourceTight`; neue Blockknoten in `sourceBlockTypes`
  (`src/editor/schema.ts`) aufnehmen. Dokument-CSS: Blöcke ohne positiven `margin-top` (Paginierung).

## Befehle
```bash
npm run setup ; npm run doctor          # Einrichtung prüfen/ergänzen
npm install ; npm run tauri dev         # starten
npm run typecheck ; npm test ; npm run i18n
cd src-tauri ; cargo fmt --check ; cargo test --lib ; cargo test --test core_roundtrip ; cargo test --test foreign_documents
cargo test --test compile_pipeline -- --ignored                     # echtes Kompilieren (Bundle)
cargo run --features dev-tools --bin build-tex-bundle               # TeX-Bundle neu (Internet/Cache)
cargo run --features dev-tools --bin build-sketch-symbols           # Skizzen-Symbole neu (nach jedem Bundle-Neubau)
```
- Rechnerspezifisches nur in `src-tauri/.cargo/config.toml` (nicht versioniert). Beim Nutzer überstimmt sie die
  Benutzervariablen `VCPKGRS_TRIPLET=x64-windows`/`VCPKGRS_DYNAMIC=1` – diese **nicht ändern** (andere Projekte).
- Läuft die App, sperrt sie `target/debug/visutex.exe` → vor `cargo test` beenden.

## Konventionen
- UI-Texte, Fehlermeldungen und Doku auf **Deutsch**; Übersetzungen in `src/i18n/locales/<code>.json` (`npm run i18n`).
- Rust: `cargo fmt`, keine `unwrap()` auf Nutzereingaben, Tauri-Befehle `async` bzw. `spawn_blocking`.
- Jede Änderung am LaTeX-Export/-Import mit Tests; neue Knotentypen in `tests/fixtures/full-document.json`.
- Keine `window.prompt/confirm/alert`; eigene Dialoge.
- Ausführbare Dateien nur nach Rückfrage herunterladen; öffentliches Veröffentlichen (GitHub) nur nach Bestätigung.
