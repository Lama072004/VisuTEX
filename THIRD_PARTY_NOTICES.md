# Drittanbieter-Komponenten / Third-party notices

Der **Quellcode von VisuTeX** steht unter der [MIT-Lizenz](LICENSE).
Die **ausgelieferten Programme** (Windows-Installer, AppImage, `.deb`, `.rpm`,
Arch-Paket) enthalten zusätzlich Komponenten Dritter, die unter ihren eigenen
Lizenzen stehen. Diese Datei fasst sie zusammen; die vollständigen Lizenztexte
liegen den jeweiligen Paketen bei (Cargo-Registry bzw. `node_modules`) bzw. in
[`licenses/`](licenses/).

*English summary: the VisuTeX source code is MIT-licensed. Binary releases
statically link the Tectonic TeX engine, parts of which (XeTeX, xdvipdfmx) are
licensed under GPL-2.0-or-later. Binary releases are therefore distributed under
the terms of the GPL-2.0-or-later as a whole; the complete corresponding source
code is this repository plus the published crates listed below.*

## 1. TeX-Engine (Tectonic) – wichtig: GPL

VisuTeX bindet die TeX-Engine [Tectonic](https://tectonic-typesetting.github.io/)
(Crates `tectonic` 0.17 und `tectonic_*`) statisch ein. Tectonic selbst steht unter
der MIT-Lizenz, enthält aber C-Code aus TeX Live, der unter der
**GNU General Public License, Version 2 oder später** steht:

| Crate | Inhalt | Lizenz der C-Quellen |
|---|---|---|
| `tectonic_pdf_io` | xdvipdfmx (PDF-Ausgabe) | überwiegend GPL-2.0-or-later |
| `tectonic_engine_xetex` | XeTeX-Engine | teils GPL-2.0-or-later, sonst MIT/X11-artig |
| `tectonic_engine_xdvipdfmx`, `tectonic_engine_bibtex` u. a. | weitere Engine-Teile | MIT bzw. TeX-übliche freie Lizenzen |

**Konsequenz für Binärpakete:** Ein Programm, das diese Teile enthält, darf nur
unter den Bedingungen der GPL-2.0-or-later weitergegeben werden. Daraus folgt:

- Der vollständige Quellcode ist frei verfügbar: dieses Repository plus die
  veröffentlichten Crates auf [crates.io](https://crates.io/crates/tectonic)
  (exakte Versionen in [`src-tauri/Cargo.lock`](src-tauri/Cargo.lock)).
- Der GPL-Text liegt in [`licenses/GPL-2.0.txt`](licenses/GPL-2.0.txt) bei und
  wird mit den Installationspaketen ausgeliefert.
- Der eigene VisuTeX-Code bleibt MIT-lizenziert und darf auch ohne Tectonic
  (z. B. in anderen Projekten) unter MIT weiterverwendet werden.

## 2. Native Bibliotheken der TeX-Engine (statisch gelinkt, über vcpkg)

| Bibliothek | Lizenz |
|---|---|
| ICU | Unicode License (ICU) |
| HarfBuzz | MIT („Old MIT“) |
| FreeType | FreeType License (FTL) – *„Portions of this software are copyright © The FreeType Project (www.freetype.org). All rights reserved.“* |
| Fontconfig | MIT/HPND-artig |
| graphite2 | wahlweise LGPL-2.1+, MPL-2.0 oder GPL-2.0+ (genutzt: MPL-2.0) |
| libpng | libpng License (PNG Reference Library License v2) |
| zlib | zlib License |
| Expat | MIT |

## 3. Rust-Abhängigkeiten

Rund 780 Crates (inkl. Build-Abhängigkeiten), u. a. Tauri, serde, image, hayro,
svg2pdf, zip, reqwest. Lizenzen (Stand der Prüfung):

- MIT, Apache-2.0 bzw. „MIT OR Apache-2.0“ (große Mehrheit)
- BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, Unlicense, BSL-1.0, CC0-1.0
- Unicode-3.0 (ICU4X-Daten), CDLA-Permissive-2.0 (`webpki-root-certs`)
- MPL-2.0 (Dateiebene, unverändert genutzt: `cssparser`, `selectors`,
  `dtoa-short`, `option-ext`)

Keine dieser Lizenzen schränkt die Veröffentlichung unter MIT ein. Eine
vollständige Liste lässt sich jederzeit erzeugen, z. B. mit
`cargo install cargo-about && cargo about generate` oder `cargo metadata`.

## 4. JavaScript-Abhängigkeiten (Teil der Benutzeroberfläche)

| Paket | Lizenz |
|---|---|
| React, React DOM | MIT |
| Tiptap 3, ProseMirror | MIT |
| Monaco Editor | MIT |
| KaTeX (inkl. KaTeX-Schriften) | MIT |
| lucide-react (Symbole) | ISC |
| DOMPurify | MPL-2.0 OR Apache-2.0 (genutzt: Apache-2.0) |
| @tauri-apps/* | MIT OR Apache-2.0 |

Alle übrigen Laufzeitpakete (insgesamt 70) stehen unter MIT.

## 5. Schriften

- **Latin Modern** (`src/assets/fonts/lmroman10-*.otf`, `lmsans10-*.otf`, für die
  WYSIWYG-Darstellung): GUST Font License (LPPL-basiert), siehe
  [`src/assets/fonts/GUST-FONT-LICENSE.txt`](src/assets/fonts/GUST-FONT-LICENSE.txt).
- Schriften im TeX-Bundle: siehe Abschnitt 6.

## 6. Mitgeliefertes TeX-Bundle (`src-tauri/resources/tex-bundle.zip`)

Das Bundle enthält **unveränderte** Dateien aus der TeX-Live-Distribution
(LaTeX-Kern, Pakete, Klassen, Schriften), wie sie auch Tectonic online bezieht.
Jede Datei behält ihre eigene Lizenz, überwiegend:

- LaTeX Project Public License (LPPL 1.3c) – LaTeX-Kern und die meisten Pakete
- GPL-2.0/GPL-3.0 bzw. LGPL – einzelne Pakete
- SIL Open Font License (OFL), GUST Font License, GUST Font Nosource License
- AGPL-3.0 **mit Font-Ausnahme** (URW-Base35-Schriften): Dokumente, die diese
  Schriften einbetten, unterliegen dadurch **nicht** der AGPL
- Public Domain / Knuth-Lizenz (TeX, Computer Modern)

Die Lizenz- und Readme-Dateien der Pakete liegen – sofern vom Paket vorgesehen –
im Bundle bzw. sind in [TeX Live](https://tug.org/texlive/) und auf
[CTAN](https://ctan.org/) einsehbar. Da die Dateien unverändert weitergegeben
werden, sind die Bedingungen der LPPL erfüllt.

**Geänderte Datei:** `src-tauri/resources/tex/visutex-ieee-de.bst` ist eine eingedeutschte
Fassung von `IEEEtranN.bst` (© 2003–2015 Michael Shell, LPPL 1.3). Wie von der Lizenz verlangt,
ist sie umbenannt, im Dateikopf als geändert gekennzeichnet und nennt VisuTeX als Ansprechpartner;
alle ursprünglichen Hinweise sind erhalten. Sie wird beim LaTeX-Export neben das Dokument gelegt.

Das Bundle lässt sich mit `cargo run --features dev-tools --bin build-tex-bundle`
reproduzierbar aus dem Tectonic-Cache erzeugen.

## 7. Laufzeitumgebung

- **Microsoft Edge WebView2 Runtime** (Windows): wird vom Installer bei Bedarf
  über Microsofts weiterverteilbares Installationsprogramm eingerichtet und steht
  unter den Microsoft-Lizenzbedingungen für WebView2.
- **WebKitGTK** (Linux): Systempaket der Distribution (LGPL-2.1), wird nicht
  mitgeliefert, sondern als Paketabhängigkeit installiert.

## 8. Marken

„TeX“ ist eine Marke der American Mathematical Society. „LaTeX“ ist ein
Name des LaTeX-Projekts. „Zotero“ ist eine Marke der Corporation for Digital
Scholarship. „Windows“, „Word“ und „PowerPoint“ sind Marken der Microsoft
Corporation. VisuTeX ist mit keiner dieser Organisationen verbunden; die Namen
werden nur zur Beschreibung von Kompatibilität bzw. Dateiformaten genannt.
