# Architektur

VisuTeX ist eine Desktop-App aus zwei Teilen:

| Teil | Technik | Aufgabe |
|---|---|---|
| **Backend** (`src-tauri/`) | Rust, Tauri 2, Tectonic (TeX-Engine) | LaTeX-Export/-Import, Kompilieren, PDF-Rendering, Dateien, Zotero, Add-ons, Folien- und Skizzen-Export |
| **Oberfläche** (`src/`) | React 19, TypeScript, Tiptap 3 (ProseMirror), Monaco | Editor-Ansichten, Dialoge, Folien- und Skizzen-Editor |

Grundsatz: **Logik in Rust**, im Frontend nur, was der WebView braucht (Bearbeiten, DOM-Messung, Anzeige).
Die Oberfläche ruft Rust ausschließlich über typisierte Befehle in `src/api.ts` auf.

```
┌──────────────────────── Oberfläche (WebView) ─────────────────────────┐
│ Ribbon · Datei-Bereich · Dialoge                                      │
│ Dokument-Editor (Tiptap) ⇄ Code-Ansicht (Monaco) · PDF-Vorschau      │
│ Folien-Editor (src/slides) · Skizzen-Werkzeug (src/sketch)            │
└──────────────── src/api.ts (invoke) ──────────────────────────────────┘
┌──────────────────────── Rust (src-tauri) ─────────────────────────────┐
│ lib.rs: dünne Tauri-Befehle                                           │
│ core/: reine, getestete Logik (Export, Import, Präambel, Folien …)    │
│ compile.rs + prepare.rs + texbundle.rs → Tectonic → PDF → pdf.rs      │
└───────────────────────────────────────────────────────────────────────┘
```

## Datenfluss Dokument

1. **Visuell bearbeiten**: Tiptap hält das Dokument als JSON (Knoten wie `heading`, `mathBlock`, `figure`, `rawLatexBlock` …; Schema in `src/editor/schema.ts`).
2. **Export** (`core/export.rs`): JSON → vollständiges, portables LaTeX (Präambel aus `core/preamble.rs`, kompiliert mit pdfLaTeX, XeLaTeX, LuaLaTeX).
3. **Import** (`core/import.rs`): beliebiges LaTeX → JSON. Unbekanntes bleibt als Roh-LaTeX-Block erhalten.
4. **Quelltreue**: Importierte Blöcke merken sich ihren Originalcode (`sourceLatex`) und einen Fingerabdruck (`sourceKey`). Unveränderte Blöcke werden **zeichengenau** wieder ausgegeben → verlustfreier Round-Trip Code ⇄ Visuell.
5. **Kompilieren** (`compile.rs`): Tectonic (XeTeX) mit dem mitgelieferten TeX-Bundle, auch bei Fehlern wird ein PDF erzeugt; `prepare.rs` passt Fremddokumente zeilengenau an (biber→BibTeX, pdfx, fehlende/defekte Bilder → Platzhalter).
6. **Vorschau** (`pdf.rs`): PDF-Seiten werden in Rust (hayro) als PNG gerendert.

## Datenfluss Präsentation

`src/slides/model.ts` (Folien, Elemente in mm) → `core/slides.rs` → LaTeX-Beamer (`textpos` + TikZ, absolut positioniert) → Kompilieren wie oben.
Text in Textfeldern ist Tiptap-JSON und wird mit demselben Exporter übersetzt wie im Dokument.

## Datenfluss Skizze

`src/sketch/SketchPad.tsx` (Rastermodell) → `core/sketch.rs` → TikZ/CircuiTikZ-Code. Das Modell steht als Kommentar in der
ersten Codezeile, damit eine eingefügte Skizze wieder bearbeitet werden kann. Symbole und Anschlüsse aller Bauteile kommen aus
`resources/sketch-symbols.json` (erzeugt aus `core/sketch_catalog.rs`, siehe [Entwicklung](ENTWICKLUNG.md#werkzeuge)).

## Dateiformate

| Endung | Inhalt |
|---|---|
| `.visutex` | Projekt (JSON, Format v2): Einstellungen, Dokument-JSON, ggf. eigene Präambel, letzte Ansicht |
| `.tex` | Beliebiges LaTeX; beim Speichern bleibt die Datei LaTeX (eingebundene Teildateien werden zurückgeschrieben) |
| `.vtxslides` | Präsentation (JSON): Folien, Elemente, Design, Raster; Bilder eingebettet |
| `.bib` | Literatur (BibTeX), im Projektordner |

## Ordnerstruktur

```
.
├── src/                          Oberfläche
│   ├── main.tsx, App.tsx         Einstieg, Koordination (Zustand, Aktionen, Tastenkürzel)
│   ├── api.ts                    typisierte Rust-Befehle (+ devMock.ts für den Browser)
│   ├── components/               Ribbon, Datei-Bereich (Backstage), Dialoge, PDF-Vorschau,
│   │                             Code-Ansicht, Lineal, Suche, Systemprüfung, Add-ons
│   ├── editor/                   Tiptap: Schema (Node-testbar), Erweiterungen, NodeViews,
│   │                             Seitenumbruch/Paginierung, Suche, Tabellenbeschriftung
│   ├── latex/                    Einstellungen, Dokumentsprachen (document-languages.json),
│   │                             Mini-Renderer (KaTeX), siunitx-Anzeige
│   ├── slides/                   Folien-Editor: Modell, Darstellung, Editor, Textfelder
│   ├── sketch/                   Skizzen-Werkzeug (TikZ/CircuiTikZ)
│   ├── shortcuts/                Tastenkürzel: Befehle, Belegung, Reiter „Tastenkürzel“
│   ├── i18n/                     Oberflächensprachen: index.ts + locales/<code>.json
│   ├── state/                    Laufzeitkontext für NodeViews
│   └── monaco/                   LaTeX-Unterstützung für den Code-Editor
├── src-tauri/                    Rust
│   ├── src/lib.rs                Tauri-Befehle (dünn, async/spawn_blocking)
│   ├── src/core/                 reine Logik, ohne Tauri:
│   │   ├── export.rs / import.rs     Dokument ⇄ LaTeX
│   │   ├── preamble.rs, settings.rs  Präambel, Einstellungen (validiert)
│   │   ├── languages.rs              Dokumentsprachen (liest document-languages.json)
│   │   ├── slides.rs                 Präsentation → Beamer
│   │   ├── sketch.rs, sketch_catalog.rs  Skizze → TikZ, Bauteilkatalog
│   │   ├── bibtex.rs, analysis.rs, project.rs, includes.rs, macros.rs, markers.rs, escape.rs, addon.rs
│   │   ├── support.rs                eigene TeX-Dateien (deutscher IEEE-Stil) für Kompilieren und Export
│   ├── src/compile.rs, prepare.rs, texbundle.rs, pdf.rs, preview.rs   Kompilieren und Vorschau
│   ├── src/files.rs, images.rs, fonts.rs, system.rs, zotero.rs, addons.rs, templates.rs
│   ├── src/embedded.rs           eingebettetes TeX-Bundle/Add-ons beim Start entpacken
│   ├── src/setup.rs, updates.rs  Einrichtung (Installationsort, Verknüpfungen, Autostart, Deinstallation),
│   │                             Update-Suche und -Installation über GitHub-Releases
│   ├── src/bin/                  Entwicklerwerkzeuge (TeX-Bundle, Skizzen-Symbole)
│   ├── resources/                tex-bundle.zip, sketch-symbols.json, mitgelieferte Add-ons (in die EXE eingebettet),
│   │                             tex/visutex-ieee-de.bst (deutscher IEEE-Stil, per include_str! eingebettet)
│   └── tests/                    Round-Trip, Fremddokumente, Kompilier-Pipeline
├── tests/                        Node-Tests + Fixtures (Volltest-Dokument, Fremddokumente, Folien)
├── scripts/                      setup.mjs (Einrichtung), i18n-*.mjs (Übersetzungen), check-latex-engines.sh, portable.mjs (Testversion)
├── docs/                         diese Dokumentation
├── packaging/                    Arch-Linux-PKGBUILD
└── .github/workflows/build.yml   CI: Windows + Linux, Tests, Installation auf mehreren Distributionen
```

## Mitgeliefert (keine externe LaTeX-Installation nötig)

- **TeX-Engine**: Tectonic, statisch eingebunden (C-Bibliotheken über vcpkg).
- **TeX-Bundle** `resources/tex-bundle.zip`: alle benötigten Pakete, Klassen, Schriften und Silbentrennmuster
  (auch für alle Dokumentsprachen, Beamer, CircuiTikZ); optional lädt die App fehlende Pakete online nach.
- **Schriften** für die Anzeige: Latin Modern (Roman, Sans).
- **WebView2** (Windows) installiert der Installer bei Bedarf.

## Sprachen

- **Oberfläche**: Deutsch ist die Quellsprache (die Texte im Code sind die Schlüssel), Übersetzungen in
  `src/i18n/locales/<code>.json` (en, fr, es, it, pt, nl, pl). Fehlt ein Text, gilt Englisch, dann Deutsch.
- **Dokument**: 24 Sprachen mit babel-Option, Anführungszeichen, Dezimalzeichen, siunitx-Locale und Beschriftungen in
  `src/latex/document-languages.json` – dieselbe Datei für Rust und Oberfläche.
