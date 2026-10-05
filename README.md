<p align="center"><img src="docs/logo.png" alt="VisuTeX" width="128"></p>

<h1 align="center">VisuTeX</h1>

<p align="center"><b>LaTeX bearbeiten wie in einer Textverarbeitung – mit eingebauter TeX-Engine.</b><br>
Visuell oder als Code, verlustfrei hin und her. Keine LaTeX-Installation nötig.</p>

<p align="center"><a href="#english">English</a> · <a href="docs/BEDIENUNG.md">Bedienung</a> ·
<a href="docs/ARCHITEKTUR.md">Architektur</a> · <a href="docs/ENTWICKLUNG.md">Entwicklung</a></p>

---

## Funktionen

- **Visueller Editor** mit Seitenansicht, Menüband, Formatvorlagen, Tabellen, Bildern, Formeln (KaTeX-Vorschau),
  Querverweisen, Fußnoten, Abkürzungen, Größen mit Einheiten (siunitx), Verzeichnissen und Titelseite.
- **Code-Ansicht**: das vollständige, portable LaTeX-Dokument (pdfLaTeX, XeLaTeX, LuaLaTeX; TeX Live, MiKTeX, Overleaf).
  Wechsel ohne Verluste – unveränderte Teile bleiben zeichengenau erhalten.
- **Fremde LaTeX-Projekte öffnen** und weiterbearbeiten, auch mit `\input`/`\include`, eigener Präambel und Bildern in Unterordnern.
- **Eingebaute TeX-Engine** (Tectonic) mit mitgeliefertem Paket-Bundle – funktioniert offline; fehlende Pakete optional online.
- **Literatur** über BibTeX/natbib, **Zotero-Anbindung**.
- **Folien-Editor**: Präsentationen frei gestalten (Textfelder, Formen, Bilder, Formeln auf einem Raster, Hilfslinien,
  Bildschirmpräsentation), Export als PDF oder LaTeX-Beamer.
- **Skizzen-Werkzeug**: TikZ-Zeichnungen und **alle CircuiTikZ-Bauteile** mit Beschriftungen, Werten, Spannungs- und
  Strompfeilen; Leitungen docken an Bauteilanschlüssen an.
- **Frei belegbare Tastenkürzel**, Lineal, Suchen & Ersetzen, Format übertragen, Wiederherstellung nach Absturz.
- **Sprachen**: Oberfläche auf Deutsch, Englisch, Französisch, Spanisch, Italienisch, Portugiesisch, Niederländisch,
  Polnisch; 24 Dokumentsprachen (babel, Silbentrennung, Anführungszeichen, Zahlenformat).
- **Add-ons** (deklarativ): Präambel, TeX-Dateien, Bausteine im Menüband, Vorlagen.

## Installation

Installationspakete stehen unter *Releases* bereit:

| System | Paket |
|---|---|
| Windows 10/11 | `VisuTeX_x.y.z_x64-setup.exe` (installiert WebView2 bei Bedarf) |
| Ubuntu, Debian, Mint | `.deb` |
| Fedora, openSUSE | `.rpm` |
| andere Distributionen | `.AppImage` (Arch: `packaging/arch/PKGBUILD`) |

Alles Nötige (TeX-Engine, Pakete, Schriften) ist enthalten. Die *Systemprüfung* (Datei → Info) zeigt, was gefunden wurde.

## Aus dem Quellcode bauen

```bash
npm install
npm run setup            # prüft Rust, vcpkg, Systempakete und bietet Fehlendes an
npm run tauri dev        # starten
npm run exe              # Programm bauen (src-tauri/target/release/visutex.exe)
npm run portable         # daraus eine portable Testversion (Ordner mit EXE + TeX-Bundle)
npm run installer        # Windows-Installer (Setup.exe)
```

Details, Tests und VS-Code-Einrichtung: [docs/ENTWICKLUNG.md](docs/ENTWICKLUNG.md).

## Lizenz

Der Quellcode steht unter der [MIT-Lizenz](LICENSE). Die fertigen Programme enthalten die TeX-Engine Tectonic, deren
XeTeX-/xdvipdfmx-Teile unter der GPL-2.0-or-later stehen; Binärpakete werden daher insgesamt unter den Bedingungen der
GPL-2.0-or-later weitergegeben. Einzelheiten: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

---

## English

**VisuTeX** is a WYSIWYG LaTeX editor with a built-in TeX engine. Edit documents visually like in a word processor or as
LaTeX code – switching is lossless. It opens existing LaTeX projects, compiles offline with the bundled Tectonic engine,
manages references with BibTeX and Zotero, and includes a slide editor (exports to PDF or LaTeX Beamer) and a sketch tool
covering all CircuiTikZ components. The user interface is available in German, English, French, Spanish, Italian,
Portuguese, Dutch and Polish.

Build: `npm install && npm run setup && npm run tauri dev`. Source code: MIT; binaries include GPL-2.0-or-later parts of
Tectonic (see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)). Contributions welcome – see [CONTRIBUTING.md](CONTRIBUTING.md).
