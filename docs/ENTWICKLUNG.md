# Entwicklung

## Voraussetzungen

| Werkzeug | Zweck |
|---|---|
| Node.js ≥ 20 (empfohlen 22/24) | Oberfläche, Tests, Skripte |
| Rust (stable, über rustup) | Backend |
| `cargo-vcpkg` | baut die statischen C-Bibliotheken der TeX-Engine (ICU, HarfBuzz, FreeType, Fontconfig, Graphite2) |
| Windows: Visual Studio Build Tools (C++), WebView2 | Kompilieren, Laufzeit |
| Linux: `libwebkit2gtk-4.1-dev libsoup-3.0-dev librsvg2-dev patchelf` + autotools, gperf, bison | Tauri und vcpkg |

**Am einfachsten:** `npm run setup` prüft alles, bietet Fehlendes zur Installation an und fragt nach dem Speicherort.
`npm run doctor` prüft nur. Rechnerspezifisches landet in `src-tauri/.cargo/config.toml` (nicht versioniert).

Einmalig (dauert beim ersten Mal 20–40 Minuten):

```bash
npm install
cargo install cargo-vcpkg
cd src-tauri && cargo vcpkg build
```

## Starten

```bash
npm run tauri dev        # aus der Projektwurzel: Oberfläche mit Hot-Reload + Rust-Backend
```

Nur die Oberfläche im Browser (`npm run dev`, http://localhost:1420) läuft mit Platzhaltern aus `src/devMock.ts` –
Kompilieren, Dateien und Zotero gibt es nur in der Desktop-App.

## Testen

| Befehl | Was |
|---|---|
| `npm run typecheck` | TypeScript-Prüfung |
| `npm test` | Node-Tests: Editor-Schema, Suche, Tastenkürzel, Folienmodell |
| `npm run i18n` | Übersetzungen: fehlende/ungenutzte Texte je Sprache |
| `cd src-tauri && cargo fmt --check` | Formatierung |
| `cargo test --lib` | Unit-Tests des Rust-Kerns |
| `cargo test --test core_roundtrip` | Volltest-Dokument: Export → Import → Export |
| `cargo test --test foreign_documents` | Fremddokumente (`tests/fixtures/fremd`, optional private `Vorlagen_Test_TEX/`): Round-Trip, Export = Original |
| `cargo test --test compile_pipeline -- --ignored` | echtes Kompilieren mit dem TeX-Bundle: Volltest-Dokument, TikZ, Fehlermeldungen, Folien, **alle 24 Dokumentsprachen** |
| `cargo test --test foreign_documents -- --ignored` | alle Fremddokumente kompilieren |
| `scripts/check-latex-engines.sh` | erzeugtes LaTeX mit pdfLaTeX, XeLaTeX, LuaLaTeX (braucht TeX Live) |

Läuft die App gerade, sperrt sie `target/debug/visutex.exe` – vor `cargo test` beenden.

## VS Code

1. Ordner öffnen, die empfohlenen Erweiterungen installieren (Hinweis unten rechts; `.vscode/extensions.json`:
   rust-analyzer, Tauri, CodeLLDB, C/C++).
2. **Starten:** `Strg+Umschalt+B` → „VisuTeX starten (Entwicklung)“ (`npm run tauri dev`).
3. **Tests:** *Terminal → Aufgabe ausführen …* → „Tests: alle (schnell)“, „Tests: Frontend“, „Tests: Rust-Kern“
   oder „Tests: Kompilieren mit TeX-Bundle“.
4. **Debuggen (Rust):** *Ausführen und Debuggen* → „VisuTeX debuggen (Windows)“ bzw. „(Linux, LLDB)“ → `F5`.
   Vorher baut VS Code das Backend und startet den Vite-Server. Haltepunkte in `src-tauri/src/**` funktionieren.
5. **Debuggen (Oberfläche):** in der laufenden App `Strg+Umschalt+I` öffnet die WebView-Entwicklertools
   (nur im Debug-Build).
6. Weitere Aufgaben: „Übersetzungen prüfen“, „Installer bauen (Release)“.

## Automatisierte Oberflächentests (echtes Fenster)

Die Debug-Build stellt im Fenster `window.__visutex` bereit (Editor, Zustand, Aktionen). Mit
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223` gestartet, lässt sich das Fenster über das
Chrome-DevTools-Protokoll steuern (Skripte ausführen, Screenshots) – so werden Knöpfe, Skizze und Folien Ende-zu-Ende geprüft.

## Werkzeuge

| Befehl (in `src-tauri`) | Zweck |
|---|---|
| `cargo run --features dev-tools --bin build-tex-bundle` | erzeugt `resources/tex-bundle.zip` neu: kompiliert Abdeckungsdokumente (alle Funktionen, Vorlagen, Sprachen, Skizzen-Symbole, Folien) mit dem Tectonic-Standard-Bundle und packt jede gelesene Datei ein. Braucht Internet bzw. den Tectonic-Cache. |
| `cargo run --features dev-tools --bin build-sketch-symbols` | erzeugt `resources/sketch-symbols.json`: kompiliert jeden Eintrag aus `core/sketch_catalog.rs` mit dem Bundle, sortiert Unbekanntes aus, rendert die Symbole und liest die Anschlüsse aus. **Nach jedem Bundle-Neubau ausführen.** |

Übersetzungen:

```bash
npm run i18n                                            # Bericht
node scripts/i18n-keys.mjs export texte.txt             # alle Texte nummeriert (Nr|"Text")
node scripts/i18n-keys.mjs import fr fr.txt texte.txt   # Nr|"Übersetzung" → src/i18n/locales/fr.json
```

Neue Oberflächensprache: `src/i18n/locales/<code>.json` anlegen und in `UI_LANGUAGES` (`src/i18n/index.ts`) eintragen.
Neue Dokumentsprache: Eintrag in `src/latex/document-languages.json`, danach Bundle neu bauen (babel-Dateien).

## EXE und Installer erstellen

Voraussetzung: die Einrichtung oben (`npm run setup` zeigt alles grün). Alle Befehle aus der Projektwurzel; der erste
Release-Build dauert wegen der Optimierung (LTO) 15–30 Minuten, danach deutlich weniger.

| Befehl | Ergebnis | Zum Testen |
|---|---|---|
| `npm run exe` | `src-tauri/target/release/visutex.exe` (Release, ohne Installer) | direkt starten |
| `npm run portable` | `src-tauri/target/release/VisuTeX-portable/` – EXE + `resources/` + `licenses/` in einem Ordner | Ordner kopieren und auf einem anderen Rechner starten |
| `npm run installer` | `src-tauri/target/release/bundle/nsis/VisuTeX_<Version>_x64-setup.exe` | wie ein Nutzer installieren |
| `npm run installer:alle` | Windows: NSIS + MSI · Linux: `.deb`, `.rpm`, `.AppImage` | |

- Die EXE findet das TeX-Bundle im Ordner `resources` neben sich (der Build legt ihn an); die portable Version enthält ihn.
- Die portable Version braucht WebView2 – unter Windows 10/11 ist es vorinstalliert. Der Installer bringt den
  WebView2-Offline-Installer mit; dafür lädt `tauri build` beim ersten Mal das Installationsprogramm von WebView2 und die
  NSIS-/WiX-Werkzeuge herunter (nur beim Bauen, nicht zur Laufzeit).
- Version anpassen: `version` in `package.json`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json` gleich halten.
- Prüfen, dass alles statisch gelinkt ist (keine fehlenden DLLs auf fremden Rechnern):
  `dumpbin /dependents src-tauri\target\release\visutex.exe` darf keine ICU-/Fontconfig-/FreeType-/VC-Runtime-DLLs zeigen.

**Was testen?** Neues Dokument aus einer Vorlage, Bild/Bilder nebeneinander einfügen, lange Tabelle, Formeln, Skizze,
Kompilieren (F5) und PDF-Vorschau, Code-Ansicht hin und zurück, eigene `.tex`-Datei öffnen, Präsentation (Datei → Neu →
Präsentation) inkl. Export als PDF/Beamer, Speichern/Öffnen, Sprache der Oberfläche, Datei → Info → Systemprüfung.

`dumpbin /dependents target\release\visutex.exe` darf unter Windows keine ICU-/Fontconfig-/FreeType-/VC-Runtime-DLLs zeigen
(alles statisch gelinkt). Die CI (`.github/workflows/build.yml`) baut Windows und Linux, führt alle Tests aus und installiert
die Linux-Pakete testweise auf Ubuntu, Debian, Mint, Fedora, Arch und openSUSE.

## Konventionen

- Oberflächentexte, Fehlermeldungen und Doku auf Deutsch; Übersetzungen über `src/i18n/locales`.
- Rust: `cargo fmt`, keine `unwrap()` auf Nutzereingaben, Tauri-Befehle `async` bzw. `spawn_blocking`.
- Jede Änderung an Export/Import mit Tests; neue Knotentypen in `tests/fixtures/full-document.json` aufnehmen.
- Keine `window.prompt/confirm/alert` – eigene Dialoge (`src/components/Dialogs.tsx`).
