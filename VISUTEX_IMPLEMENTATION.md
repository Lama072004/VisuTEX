# VisuTeX – Implementierungsstand

Stand: 5. Oktober 2026 · Aufbau: [docs/ARCHITEKTUR.md](docs/ARCHITEKTUR.md) · Befehle/Tests: [docs/ENTWICKLUNG.md](docs/ENTWICKLUNG.md)

Legende: ✅ funktioniert (automatisch bzw. im echten Fenster getestet) · ☑️ umgesetzt, nur per Test/Typprüfung geprüft,
noch nicht im echten Fenster durchgeklickt · ⏳ offen

## Funktionen

| Bereich | Status | Anmerkung |
|---|---|---|
| Visueller Editor (Formatierung, Listen, Tabellen, Bilder, Formeln, Querverweise, Fußnoten, Abkürzungen, Größen, Umgebungen, Titelseite, Verzeichnisse) | ✅ | Knopf-Durchlauf im Fenster: 103 Bedienelemente ohne Fehler |
| Code-Ansicht, verlustfreier Round-Trip | ✅ | Volltest-Dokument + 31 echte und synthetische Fremddokumente: Export unverändert = Original |
| Fremde LaTeX-Projekte (`\input`/`\include`, eigene Präambel, Bilder in Unterordnern) | ✅ | |
| Kompilieren mit eingebauter Engine, Fehler mit Zeilennummer, PDF trotz Fehlern | ✅ | defekte PNG/JPEG werden durch Platzhalter ersetzt (sonst Absturz in libpng) |
| Literatur (BibTeX/natbib), Zotero | ✅ | |
| Add-ons | ✅ | |
| Skizzen-Werkzeug (TikZ/CircuiTikZ) – Palette mit allen 303 Bauteilen und Formen, Pins, Bauteilbeschriftungen, Text, Pfade, Pfeilspitzen | ✅ | Symbole mit aktuellem Bundle erzeugt (keine abgelehnten Einträge); Zeichnen, Verschieben, Tasten, Rückgängig im Browser getestet |
| Folien-Editor (Datei-Reiter, Raster, Hilfslinien, Textfelder, Formen, Bilder, Formeln, Präsentation, Export PDF/Beamer) | ☑️ | Beamer-Export kompiliert (3 Folien, visuell geprüft); Ziehen repariert, noch nicht erneut im Fenster durchgeklickt |
| Wechsel Dokument ⇄ Präsentation (Titelleiste „Folien“, „Zum Dokument“, Datei → Neu/Öffnen mit `.vtxslides`) | ☑️ | |
| Tastenkürzel frei belegbar (Datei → Tastenkürzel) | ☑️ | Logik getestet (Node), Oberfläche typgeprüft |
| Lineal mit verschiebbaren Rändern | ☑️ | |
| Lange Tabellen über mehrere Seiten (longtable), Überlauf-Markierung in der Seitenansicht | ☑️ | Round-Trip grün (auch im Volltest-Dokument); Kompilieren scheitert noch am fehlenden `ltcaption.sty` im Bundle → Bundle neu bauen |
| Oberflächensprachen de, en, fr, es, it, pt, nl, pl | ☑️ | alle 759 Texte übersetzt (`npm run i18n`: 0 fehlend) |
| 24 Dokumentsprachen (babel, Silbentrennung, Anführungszeichen, Dezimalzeichen, Beschriftungen) | ✅ | alle Sprachen kompiliert; Tschechisch/Slowakisch mit `shorthands=off` (sonst bricht `\cline{1-2}`) |
| Systemprüfung, Einrichtungsassistent, Installer Windows/Linux, CI | ✅ | CI-Lauf auf GitHub erst nach Veröffentlichung |
| Lizenzen (MIT + Drittanbieter-Hinweise, GPL-Text im Installer), eigenes Icon | ✅ | |

## Zuletzt gemacht (Runde 7)

- **Gemeinsamer Datei-Bereich** (`components/Backstage.tsx`) für Dokument und Folien: Neu mit Dokumentvorlagen und
  Präsentationen, Öffnen, Zuletzt verwendet, Add-ons, Tastenkürzel, Optionen, Info; Speichern/Exportieren wirken auf
  den aktiven Bereich (Folien über `SlideFileActions`), *Eigenschaften* nur bei Folien; Wechsel Dokument ⇄ Folien direkt
  über *Datei → Neu/Öffnen*; `Esc` schließt.
- **Folien-Editor:** Verschieben/Skalieren/Drehen repariert (Zeiger-Listener gingen bei jedem Neuzeichnen verloren);
  Tastatur wirkt nach Klick auf die Fläche. Im Browser-Test geprüft: Ziehen, Pfeiltasten, Entf, Datei-Bereich hin und zurück.
- **Skizzen:** Verschiebe-Werkzeug mit Hand-Symbol und Greif-Cursor; Tasten Entf/Rücktaste, Pfeile, Strg+C/X/V/D,
  Strg+Z/Y, Zoom, R, Enter, Esc, V/L/T. Rückgängig per Tastatur repariert (Verlauf in Refs); Ziehen hält den Zeiger fest
  (kein Hängenbleiben, wenn die Maus das Fenster verlässt). Im Browser-Test geprüft.
- **LaTeX-Code von Tabellen/Roh-Blöcken** wird vollständig angezeigt (Höhe wächst mit, ab 70 % Fensterhöhe Scrollbalken).
- **Seitenüberlauf:** Tabellen-Option „Über Seiten umbrechen“ → `longtable` (Export, Import, Round-Trip-Test,
  Seitenansicht umbricht zeilenweise); Blöcke höher als eine Seite werden rot markiert; Bilder im Export auf
  `height=0.9\textheight,keepaspectratio` begrenzt (Import entfernt den Zusatz wieder).
- **Kompilierfehler `\qty{test}{\volt}`:** Größen-Dialog akzeptiert nur siunitx-Zahlen; der Export setzt ungültige Werte
  als Text + `\unit{…}` (kein Abbruch mehr). KOMA-Warnung `\float@addtolists` behoben (`scrhack`).
  Die Meldungen „shellesc: Shell escape disabled“ und „hyperref: Rerun to get /PageLabels“ sind harmlos.
- Offene Klemmen (`open`) werden als `to[open, o-o]` gezeichnet (Vorschau und Export).
- Re-Import langer Tabellen speichert keine überflüssigen `rowRules` mehr.
- Skizzen-Symbole neu erzeugt: 303 Einträge (vorher 288; neu u. a. Operationsverstärker, Instrumentenverstärker,
  Ohmmeter, Trapez, Drachen, Kreissektor).
- Kompilier-Tests mit dem Bundle: Folien, Vorlagen, TikZ-Vorschau, Fehlerzeilen ✅; alle Fremddokumente kompilieren ✅.
- Übersetzungen ergänzt (`npm run i18n`: 0 fehlend).

## Runde 6

- Skizzen-Katalog in Rust (`core/sketch_catalog.rs`, rund 300 Einträge) + Generator `build-sketch-symbols`
  (kompiliert jeden Eintrag mit dem Bundle, rendert Symbole, liest Anschlüsse aus) + Befehl `sketch_catalog`.
- Skizzenmodell erweitert (`core/sketch.rs`): Katalogsymbole, Pfade, Ellipsen, Bögen, Farben, Linienarten, Pfeilspitzen,
  Textformatierung, Bauteil-Beschriftung/Wert/Spannung/Strom/Fluss, Pin-Verweise `(Q1.G)`; alte Skizzen bleiben kompatibel.
- Skizzen-Fenster neu (`src/sketch/SketchPad.tsx`).
- Dokumentsprachen als gemeinsame Tabelle (`src/latex/document-languages.json`, `core/languages.rs`).
- Oberflächensprachen als JSON (`src/i18n/locales`), Prüf- und Übersetzungswerkzeuge (`npm run i18n`, `scripts/i18n-keys.mjs`).
- TeX-Bundle neu gebaut (5172 Dateien, 66 MB): fette und AMS-Mathematik-Schriften, alle Sprachen, alle Skizzen-Symbole, Beamer.
- Code nach Funktionsbereichen geordnet (`src/sketch`, `src/shortcuts`, `src/i18n`), Doku neu (`README.md`, `docs/`,
  `CONTRIBUTING.md`), VS-Code-Aufgaben und Debug-Konfigurationen (`.vscode/`), überflüssige Dateien entfernt.

## Offen – damit alles ohne Probleme läuft

1. **TeX-Bundle neu bauen** (`cargo run --features dev-tools --bin build-tex-bundle`, braucht Internet bzw.
   Tectonic-Cache, dauert länger): Das Volltest-Dokument enthält jetzt eine lange Tabelle; dadurch nimmt der Neubau
   `ltcaption.sty` auf (caption lädt es zusammen mit longtable). Danach
   `cargo test --test compile_pipeline -- --ignored` – derzeit scheitern dort genau deshalb `full_document_compiles_and_renders`
   und `all_document_languages_compile`. Bis dahin bricht ein Dokument mit „Über Seiten umbrechen“ in der eingebauten
   Engine mit „ltcaption.sty not found“ ab (TeX Live/MiKTeX/Overleaf haben die Datei).
2. **Im echten Fenster durchklicken** (im Browser bereits geprüft): Folien-Editor (Drehen, Text, Präsentation, Export),
   Tastenkürzel, Lineal, Sprachwechsel.
3. **Git/GitHub** macht der Nutzer selbst (GitHub Desktop). Ein leeres Repository (`git init`, Zweig `main`, noch ohne
   Commit) ist angelegt; `.gitignore` schließt private Dokumente, Build-Ausgaben und `.claude/` aus.
   `tex-bundle.zip` ist 66 MB (GitHub-Grenze 100 MB pro Datei; ggf. Git LFS).
4. Später: PDF/A, Unterabbildungen (subfigure) visuell, CI-Job mit TeX Live für `scripts/check-latex-engines.sh`,
   macOS. EPS-Vorschau bleibt ohne Ghostscript (AGPL) bewusst offen.
