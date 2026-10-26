# Add-ons für VisuTeX entwickeln

Diese Anleitung richtet sich an alle, die VisuTeX erweitern möchten – mit eigenen LaTeX-Paketen, Bausteinen,
Menüband-Schaltflächen und Dokumentvorlagen, etwa für ein Fachgebiet, eine Hochschule oder ein Unternehmen.

Add-ons funktionieren wie Add-ins bei Office: Ein Add-on ist **eine Datei** mit eigenem Dateityp **`.vtxaddon`**.
Ein Doppelklick darauf startet VisuTeX, zeigt, was das Add-on mitbringt, und installiert es nach einer Bestätigung.
Danach erscheinen seine Schaltflächen im Menüband, seine Vorlagen unter *Datei → Neu*, und seine LaTeX-Pakete stehen
beim Kompilieren bereit.

Ein vollständiges, automatisch getestetes Beispiel liegt in [`docs/addon-beispiel/hinweisboxen`](addon-beispiel/hinweisboxen).
Am schnellsten kopiert man diesen Ordner und passt ihn an.

---

## Inhalt

1. [Überblick](#überblick)
2. [Der Dateityp `.vtxaddon`](#der-dateityp-vtxaddon)
3. [Schritt für Schritt: das erste eigene Add-on](#schritt-für-schritt-das-erste-eigene-add-on)
4. [Ohne Programmieren: Add-on in der App erstellen](#ohne-programmieren-add-on-in-der-app-erstellen)
5. [Aufbau eines Add-ons](#aufbau-eines-add-ons)
6. [`addon.json` – Referenz](#addonjson--referenz)
7. [Bausteine (Snippets)](#bausteine-snippets)
8. [Menüband-Einträge](#menüband-einträge)
9. [Symbole als SVG](#symbole-als-svg)
10. [TeX-Dateien](#tex-dateien)
11. [Präambelzeilen](#präambelzeilen)
12. [Vorlagen](#vorlagen)
13. [Installieren – so erleben es die Nutzer](#installieren--so-erleben-es-die-nutzer)
14. [So verarbeitet VisuTeX Add-ons](#so-verarbeitet-visutex-add-ons)
15. [Sicherheit](#sicherheit)
16. [Weitergeben, veröffentlichen, aktualisieren](#weitergeben-veröffentlichen-aktualisieren)
17. [Fehlermeldungen](#fehlermeldungen)
18. [Checkliste vor dem Veröffentlichen](#checkliste-vor-dem-veröffentlichen)
19. [Für Entwickler von VisuTeX: wo was im Code steht](#für-entwickler-von-visutex-wo-was-im-code-steht)

---

## Überblick

Add-ons sind **deklarativ**: Sie bestehen aus einer Beschreibung (`addon.json`) und Dateien. Sie enthalten **keinen
ausführbaren Code** – kein JavaScript, keine Programme, keine Skripte. Ein Add-on kann die App daher nicht verändern,
nichts aus dem Internet laden und nicht auf Dateien außerhalb seines Ordners zugreifen.

| Ein Add-on kann …                                                          | Feld in `addon.json`    |
|----------------------------------------------------------------------------|-------------------------|
| Präambelzeilen in jedes Dokument einfügen (z. B. `\usepackage{…}`)         | `preamble`              |
| eigene TeX-Dateien mitbringen (Pakete, Klassen, Stile, Bilder, Schriften)  | `texFiles`              |
| Bausteine anbieten, die per Klick ins Dokument eingefügt werden            | `snippets`              |
| Schaltflächen im Menüband-Reiter **Add-ons** anlegen                       | `ribbon`                |
| Dokumentvorlagen unter *Datei → Neu* bereitstellen                         | `templates`             |
| eigene Symbole (SVG) für das Add-on und seine Schaltflächen mitbringen     | `icon`, `ribbon[].icon` |

Nicht möglich (bewusst): eigene Dialoge oder Programmlogik, Menüs außerhalb des Reiters **Add-ons**, Änderungen an der
Oberfläche, Shell-Befehle aus TeX heraus (`\write18`), Netzwerkzugriffe.

Wo Nutzer Add-ons finden:

| Ort | Was |
|-----|-----|
| Menüband → Reiter **Add-ons** | die Schaltflächen aller aktiven Add-ons (erscheint, sobald es mindestens eine gibt) |
| Menüband → **Einfügen → Meine Add-ons** | dieselben Bausteine als Liste und *Add-ons verwalten …* |
| *Datei → Add-ons* | Verwaltung: installieren, ein-/ausschalten, bearbeiten, exportieren, entfernen |
| *Datei → Neu* | Vorlagen aktiver Add-ons („Vorlage aus dem Add-on „…““) |
| Explorer bzw. Dateimanager | Doppelklick auf eine `.vtxaddon`-Datei installiert sie |

---

## Der Dateityp `.vtxaddon`

Eine **Add-on-Datei** (`*.vtxaddon`) ist ein ZIP-Archiv mit fester Endung – so wie `.docx` oder `.xlam` bei Office.

| Eigenschaft | Wert |
|-------------|------|
| Endung | `.vtxaddon` |
| Format | ZIP (Deflate oder unkomprimiert) |
| Inhalt | `addon.json` im Wurzelverzeichnis **oder** in genau einem Unterordner (z. B. `hinweisboxen/addon.json`) und alle Dateien des Add-ons |
| Dateiname (Empfehlung) | `<id>-<version>.vtxaddon`, z. B. `hinweisboxen-1.0.0.vtxaddon` |
| MIME-Typ (Linux) | `application/x-visutex-addon` (Unterart von `application/zip`) |
| Windows-Dateityp | „VisuTeX-Add-on“ (ProgID `VisuTeX.Addon` bei der Setup.exe, `VisuTeXApp.Addon` bei der Einrichtung aus der App) |
| Größe | entpackt höchstens 200 MB |

**Doppelklick:** Die Setup.exe (Windows), das MSI, die deb/rpm-Pakete (Linux) und die Einrichtung aus der App heraus
melden `.vtxaddon` – ebenso `.visutex` (Projekte) und `.vtxslides` (Präsentationen) – beim System an. Ein Doppelklick
startet VisuTeX mit der Datei; VisuTeX zeigt die [Sicherheitsabfrage](#installieren--so-erleben-es-die-nutzer).
Läuft VisuTeX ohne Installation (portable Fassung), funktioniert *Datei → Öffnen* bzw. *Datei → Add-ons → Aus Datei
installieren …* genauso.

### Add-on-Datei erzeugen

**In VisuTeX (empfohlen):** *Datei → Add-ons* → beim Add-on auf **Als Add-on-Datei exportieren …** klicken. Das ergibt
`<id>-<version>.vtxaddon` mit dem Ordner `<id>/` darin; versteckte Dateien (`.git`, `.DS_Store` …) werden ausgelassen.

**Von Hand** (z. B. in einem Build-Skript) – aus dem Ordner, der `addon.json` enthält:

```powershell
# Windows (PowerShell): erst als .zip packen, dann umbenennen
Compress-Archive -Path .\hinweisboxen\* -DestinationPath hinweisboxen-1.0.0.zip -Force
Rename-Item hinweisboxen-1.0.0.zip hinweisboxen-1.0.0.vtxaddon
```

```bash
# Linux/macOS
cd hinweisboxen && zip -r -X ../hinweisboxen-1.0.0.vtxaddon . -x '.*' && cd ..
```

Eine gewöhnliche `.zip`-Datei mit demselben Inhalt nimmt VisuTeX ebenfalls an (*Aus Datei installieren …*); nur der
Doppelklick braucht die Endung `.vtxaddon`.

---

## Schritt für Schritt: das erste eigene Add-on

Dieses Beispiel baut das Add-on „Hinweisboxen“ aus [`docs/addon-beispiel/hinweisboxen`](addon-beispiel/hinweisboxen)
nach. Gearbeitet wird direkt im Add-on-Ordner von VisuTeX – Änderungen sind dann mit einem Klick auf **Neu laden**
sichtbar, ohne jedes Mal neu zu installieren.

**1. Add-on-Ordner öffnen.** *Datei → Add-ons → Add-on-Ordner öffnen*. Dort einen Ordner `hinweisboxen` anlegen
(Name beliebig; üblich ist die ID).

**2. Das LaTeX-Paket schreiben** – `hinweisboxen/tex/hinweisboxen.sty`:

```latex
\NeedsTeXFormat{LaTeX2e}
\ProvidesPackage{hinweisboxen}[2026/10/22 v1.0 Hinweis- und Warnboxen]
\RequirePackage{xcolor}
\definecolor{hinweisrand}{RGB}{37,99,235}
\definecolor{hinweisgrund}{RGB}{235,242,255}
\newcommand{\hinweisbox}[4]{\par\medskip\noindent
  \fcolorbox{#1}{#2}{\parbox{\dimexpr\linewidth-2\fboxsep-2\fboxrule\relax}{\textbf{\color{#1}#3}\quad #4}}\par\medskip}
\newcommand{\hinweis}[1]{\hinweisbox{hinweisrand}{hinweisgrund}{Hinweis:}{#1}}
```

**3. Ein Symbol zeichnen** – `hinweisboxen/icons/hinweis.svg` (siehe [Symbole als SVG](#symbole-als-svg)):

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5v.01"/>
</svg>
```

**4. Das Add-on beschreiben** – `hinweisboxen/addon.json`:

```json
{
  "id": "hinweisboxen",
  "name": "Hinweisboxen",
  "version": "1.0.0",
  "author": "Ihr Name",
  "description": "Farbige Hinweisboxen.",
  "icon": "icons/hinweis.svg",
  "texFiles": ["tex/hinweisboxen.sty"],
  "preamble": ["\\usepackage{hinweisboxen}"],
  "snippets": [
    { "id": "hinweis", "name": "Hinweis", "kind": "block", "latex": "\\hinweis{Text des Hinweises.}" }
  ],
  "ribbon": [
    { "group": "Hinweisboxen", "label": "Hinweis", "icon": "icons/hinweis.svg", "snippet": "hinweis" }
  ]
}
```

**5. Neu laden.** In *Datei → Add-ons* auf **Neu laden** klicken. Das Add-on erscheint in der Liste und ist aktiv.
Enthält es einen Fehler, steht es stattdessen rot unter **„Wird nicht geladen:“** – mit der genauen Ursache (z. B.
ein fehlender Backslash in JSON oder eine falsch geschriebene Datei).

**6. Ausprobieren.** Zurück zum Dokument: Im Menüband gibt es jetzt den Reiter **Add-ons** mit der Schaltfläche
*Hinweis*. Klicken, kompilieren (F5), Ergebnis prüfen. Auch in der LaTeX-Ansicht nachsehen: Die Präambel enthält
`\usepackage{hinweisboxen}`.

**7. Ändern und wiederholen.** Dateien im Ordner bearbeiten → **Neu laden** → testen. Die Schaltflächen im Menüband
aktualisieren sich auch, wenn man nur zwischen Fenstern wechselt.

**8. Exportieren.** **Als Add-on-Datei exportieren …** erzeugt `hinweisboxen-1.0.0.vtxaddon`. Diese Datei weitergeben;
ein Doppelklick installiert sie beim Empfänger.

**9. Gegenprobe.** Das Add-on auf einem zweiten Rechner (oder unter Linux) per Doppelklick installieren und dort einmal
kompilieren sowie als `.tex` exportieren.

---

## Ohne Programmieren: Add-on in der App erstellen

Für einfache Add-ons braucht man keinen Texteditor:

1. *Datei → Add-ons → Eigenes Add-on erstellen*.
2. Name, Version, Autor/in und Beschreibung eintragen; die ID wird aus dem Namen gebildet.
3. Optional **Symbol des Add-ons (SVG)** wählen (*SVG wählen …*).
4. **Präambelzeilen** eintragen (eine je Zeile, z. B. `\usepackage{siunitx}`).
5. Unter **TeX-Dateien → Hinzufügen …** eigene `.sty`-, `.cls`-, Bild- oder Schriftdateien wählen.
6. Unter **Bausteine → + Baustein** LaTeX-Code eintragen und die Art wählen. Mit **Im Menüband** erhält der Baustein
   eine Schaltfläche mit kurzem Text (z. B. „Ω“) oder **SVG-Symbol**.
7. **Add-on speichern** – es ist sofort aktiv.
8. **Als Add-on-Datei exportieren …** zum Weitergeben.

Vorlagen werden im Editor (noch) nicht angeboten: Add-on exportieren, die Datei mit einem ZIP-Programm öffnen oder im
Add-on-Ordner arbeiten, die Vorlage ergänzen (siehe [Vorlagen](#vorlagen)) und **Neu laden**.

---

## Aufbau eines Add-ons

Ein Add-on ist ein Ordner (bzw. eine `.vtxaddon`-Datei mit diesem Ordner). Pflicht ist nur `addon.json`; alle übrigen
Namen und Unterordner sind frei wählbar und werden in `addon.json` mit relativen Pfaden angegeben.

```text
hinweisboxen/
├── addon.json                 Pflicht: Beschreibung des Add-ons
├── tex/
│   └── hinweisboxen.sty       TeX-Dateien (Pakete, Klassen, Bilder, Schriften …)
├── icons/
│   ├── addon.svg              Symbol des Add-ons (Add-on-Verwaltung, Installationsabfrage)
│   ├── hinweis.svg            Symbole der Menüband-Schaltflächen
│   └── warnung.svg
├── vorlagen/
│   └── merkblatt.visutex      Dokumentvorlage
└── LIESMICH.md                optional: weitere Dateien (Lizenz, Hinweise) werden mitgeliefert, aber nicht ausgewertet
```

Pfade:

- immer **relativ** zum Add-on-Ordner, mit `/` (ein `\` wird automatisch umgewandelt);
- kein `..`, kein Laufwerksbuchstabe, kein führender `/`;
- keine der Zeichen `{ } % # $ & ~ ^` und keine Zeilenumbrüche (sie stören in LaTeX-Pfaden);
- versteckte Dateien und Ordner (Name beginnt mit `.`) werden beim Installieren und Exportieren ausgelassen.

---

## `addon.json` – Referenz

Die Datei ist UTF-8-kodiertes JSON (ein BOM wird toleriert). **Backslashes müssen in JSON verdoppelt werden:**
`\usepackage{siunitx}` wird zu `"\\usepackage{siunitx}"`.

Vollständiges Beispiel (aus `docs/addon-beispiel/hinweisboxen`):

```json
{
  "id": "hinweisboxen",
  "name": "Hinweisboxen",
  "version": "1.0.0",
  "author": "Beispiel",
  "description": "Farbige Hinweis- und Warnboxen mit eigenem LaTeX-Paket, SVG-Symbolen und einer Vorlage.",
  "icon": "icons/addon.svg",
  "texFiles": ["tex/hinweisboxen.sty"],
  "preamble": ["\\usepackage{hinweisboxen}"],
  "snippets": [
    { "id": "hinweis", "name": "Hinweis", "group": "Boxen", "kind": "block", "latex": "\\hinweis{Text des Hinweises.}" },
    { "id": "warnung", "name": "Warnung", "group": "Boxen", "kind": "block", "latex": "\\warnung{Text der Warnung.}" },
    { "id": "hinweis-wort", "name": "Hinweis-Wort", "group": "Text", "kind": "inline", "latex": "\\textbf{\\color{hinweisrand}Hinweis}" }
  ],
  "ribbon": [
    { "group": "Hinweisboxen", "label": "Hinweis", "icon": "icons/hinweis.svg", "snippet": "hinweis" },
    { "group": "Hinweisboxen", "label": "Warnung", "icon": "icons/warnung.svg", "snippet": "warnung" },
    { "group": "Hinweisboxen", "label": "Wort", "icon": "H", "snippet": "hinweis-wort" }
  ],
  "templates": [
    { "id": "merkblatt", "name": "Merkblatt mit Hinweisboxen", "file": "vorlagen/merkblatt.visutex" }
  ]
}
```

### Felder

| Feld          | Typ    | Pflicht | Regeln |
|---------------|--------|---------|--------|
| `id`          | Text   | ja      | 1–64 Zeichen, nur `a–z`, `0–9`, `-`, `_`. Eindeutig; bestimmt den Ordnernamen bei der Installation. **Nicht mehr ändern**, sonst gilt das Add-on als neues Add-on. |
| `name`        | Text   | –       | Anzeigename, höchstens 80 Zeichen (leer → `id`). |
| `version`     | Text   | –       | Frei, höchstens 20 Zeichen (leer → `1.0.0`). Empfohlen: `MAJOR.MINOR.PATCH`. |
| `author`      | Text   | –       | Höchstens 80 Zeichen. |
| `description` | Text   | –       | Höchstens 500 Zeichen; erscheint in der Verwaltung und in der Installationsabfrage. |
| `icon`        | Pfad   | –       | SVG-Datei im Add-on (siehe [Symbole](#symbole-als-svg)). Ohne Angabe: Standardsymbol. |
| `texFiles`    | Liste  | –       | Pfade zu TeX-Dateien (siehe [TeX-Dateien](#tex-dateien)). |
| `preamble`    | Liste  | –       | Präambelzeilen, höchstens 100 (siehe [Präambelzeilen](#präambelzeilen)). |
| `snippets`    | Liste  | –       | Bausteine (siehe [Bausteine](#bausteine-snippets)). |
| `ribbon`      | Liste  | –       | Menüband-Schaltflächen (siehe [Menüband-Einträge](#menüband-einträge)). |
| `templates`   | Liste  | –       | Vorlagen (siehe [Vorlagen](#vorlagen)). |

Zeilenumbrüche in Text-Feldern werden durch Leerzeichen ersetzt, zu lange Texte gekürzt, unbekannte Felder ignoriert.
Fehler in Pfaden, IDs, Präambel oder Verweisen führen dagegen **zur Ablehnung** des ganzen Add-ons, damit sie beim
Entwickeln auffallen.

---

## Bausteine (Snippets)

Ein Baustein ist ein Stück LaTeX, das VisuTeX an der Cursorposition einfügt – über eine Menüband-Schaltfläche oder
*Einfügen → Meine Add-ons*.

| Feld    | Regeln |
|---------|--------|
| `id`    | wie die Add-on-ID (`a–z`, `0–9`, `-`, `_`, 1–64 Zeichen), innerhalb des Add-ons eindeutig |
| `name`  | Anzeigename, höchstens 80 Zeichen (leer → `id`) |
| `group` | Gruppierung, höchstens 60 Zeichen (derzeit nur informativ) |
| `kind`  | Art des Bausteins, siehe Tabelle; Unbekanntes wird zu `inline` |
| `latex` | der LaTeX-Code, höchstens 20 000 Zeichen |

| `kind`       | Was im Dokument entsteht | Hinweise zum `latex`-Feld |
|--------------|--------------------------|---------------------------|
| `inline`     | LaTeX im Fließtext (bleibt als Code erhalten) | z. B. `\\SI{230}{\\volt}` |
| `block`      | eigener LaTeX-Block mit **kompilierter Vorschau** im visuellen Editor | ganze Umgebungen oder Befehle wie `\\hinweis{…}` |
| `math`       | abgesetzte Formel (unnummeriert), danach im grafischen Formeleditor bearbeitbar | **ohne** `\\[ … \\]` oder `$$` |
| `inlineMath` | Formel im Text | **ohne** `$ … $` |
| `tikz`       | TikZ-Zeichnung mit Vorschau | nur der Inhalt, **ohne** `\\begin{tikzpicture}`; enthält der Code `to[`, wird `circuitikz` verwendet |

Alle Bausteine sind ganz normales LaTeX im Dokument: Der Code bleibt beim Speichern, beim Wechsel in die LaTeX-Ansicht
und beim Export unverändert erhalten.

---

## Menüband-Einträge

Sobald ein aktives Add-on mindestens einen Eintrag in `ribbon` hat, erscheint im Menüband der Reiter **Add-ons**.
Schaltflächen mit gleicher `group` stehen in einer gemeinsamen Gruppe; ganz rechts folgt immer *Add-ons verwalten*.
Dieselben Einträge listet *Einfügen → Meine Add-ons*.

| Feld      | Regeln |
|-----------|--------|
| `snippet` | ID eines Bausteins **dieses** Add-ons (Pflicht; unbekannte ID → Fehler) |
| `label`   | Beschriftung, höchstens 40 Zeichen – kurz halten (ein bis zwei Wörter) |
| `group`   | Gruppenname, höchstens 40 Zeichen (leer → Name des Add-ons) |
| `icon`    | Pfad zu einer **SVG-Datei** im Add-on **oder** kurzer Text mit höchstens 4 Zeichen (z. B. `"Ω"`); leer → Standardsymbol |

Der Tooltip einer Schaltfläche zeigt den LaTeX-Code des Bausteins. Die Schaltflächen sind in der visuellen Ansicht
aktiv (in der LaTeX-Ansicht tippt man den Code direkt).

---

## Symbole als SVG

VisuTeX verwendet für Symbole **Vektorgrafiken (SVG)** – sie bleiben in jeder Vergrößerung und auf hochauflösenden
Bildschirmen scharf. Bitmaps (PNG, JPG, ICO) werden als Symbol nicht angenommen.

### Empfehlungen

- **Zeichenfläche 24 × 24:** `viewBox="0 0 24 24"`. Angezeigt wird das Symbol mit 22 × 22 Pixeln (Menüband, Liste),
  16 × 16 (Menüs) bzw. 28 × 28 (Installationsabfrage).
- **Linienstil wie die eingebauten Symbole:** `fill="none" stroke="currentColor" stroke-width="2"
  stroke-linecap="round" stroke-linejoin="round"`, etwa 2 Einheiten Abstand zum Rand.
- **`currentColor` verwenden:** Enthält die Datei `currentColor`, färbt VisuTeX das Symbol in der Textfarbe der
  Oberfläche ein – es passt automatisch zum hellen und dunklen Design und wird bei deaktivierten Schaltflächen
  abgeblendet. (Technisch dient die Form als Maske; die Farben in der Datei spielen dann keine Rolle.)
- **Mehrfarbige Symbole** (ohne `currentColor`) werden so angezeigt, wie sie gezeichnet sind – auf ausreichenden
  Kontrast in beiden Designs achten.
- Texte in Symbolen in Pfade umwandeln (Schriften sind nicht garantiert vorhanden).

### Regeln (geprüft beim Installieren und schon beim Auswählen im Add-on-Editor)

| Regel | Grund |
|-------|-------|
| Pfad endet auf `.svg`, liegt im Add-on, höchstens 200 Zeichen | keine fremden Dateien |
| höchstens **64 KB**, UTF-8-Text, enthält ein `<svg>`-Element | Symbole sind klein |
| kein `<script>`, `<foreignObject>`, `<iframe>`, `<embed>`, `<object>`, kein `javascript:` | keine aktiven Inhalte |
| keine Ereignis-Attribute (`onload=`, `onclick=` …) | keine aktiven Inhalte |
| `href`/`src` nur auf interne Verweise (`#id`) oder eingebettete Bilder (`data:image/…`) | keine Zugriffe nach außen |
| kein `url(http…)`, kein `@import` | keine Zugriffe nach außen |

Grafikprogramme (z. B. Inkscape) speichern zusätzliche Metadaten; „Optimiertes SVG“ bzw. „Einfaches SVG“ ergibt kleinere
Dateien.

---

## TeX-Dateien

Unter `texFiles` stehen Dateien, die der eingebauten TeX-Engine zur Verfügung stehen sollen. Erlaubte Endungen:

`sty` `cls` `tex` `bst` `bib` `def` `cfg` `clo` `fd` `ldf` · `png` `jpg` `jpeg` `pdf` · `otf` `ttf` `tfm` `pfb` `enc` `map` `vf`

So werden sie verwendet:

- **Suchpfad:** Jeder Ordner, in dem eine aufgeführte Datei liegt, wird beim Kompilieren als zusätzlicher Suchpfad
  übergeben. `\usepackage{hinweisboxen}` findet `tex/hinweisboxen.sty` ohne Pfadangabe; ebenso `\includegraphics{logo}`
  ein mitgeliefertes `logo.pdf`.
- **Export als `.tex`:** Die Dateien werden neben das exportierte Dokument kopiert (sofern dort noch keine Datei gleichen
  Namens liegt). Das Dokument kompiliert dadurch auch ohne VisuTeX – mit TeX Live, MiKTeX oder Overleaf.
- Beim Export wird **flach nach Dateinamen** kopiert: Dateinamen deshalb eindeutig wählen (z. B. mit Präfix).

Tipps:

- Viele Pakete sind bereits im TeX-Bundle von VisuTeX enthalten (u. a. `siunitx`, `circuitikz`, `tikz`, `xcolor`,
  `amsmath`, `booktabs`, KOMA-Script). Eigene Dateien nur für eigene oder fehlende Pakete mitliefern; fehlende
  Standardpakete lädt VisuTeX auf Wunsch aus dem Internet nach (*Datei → Optionen*).
- Eigene Pakete möglichst **engine-neutral** schreiben (pdfLaTeX, XeLaTeX und LuaLaTeX), damit der `.tex`-Export überall
  kompiliert.
- Mitgelieferte Schriften (`otf`/`ttf`) sind über `fontspec` mit Dateinamen nutzbar, z. B.
  `\setmainfont{MeineSchrift-Regular.otf}`. Lizenz der Schrift beachten!

---

## Präambelzeilen

`preamble` enthält Zeilen, die VisuTeX in die Präambel **jedes** Dokuments schreibt, solange das Add-on aktiv ist.

- höchstens 100 Zeilen, jede höchstens 1000 Zeichen;
- verboten: `\write18`, `\openout`, `\documentclass`, `\begin{document}`;
- Position: **nach** allem, was VisuTeX selbst erzeugt (Pakete, Seitenlayout, Kopf- und Fußzeilen, Sprache, Literatur),
  und **vor** `hyperref`. Add-on-Zeilen können daher Einstellungen von VisuTeX überschreiben (z. B. `\sisetup{…}`);
- mehrere aktive Add-ons werden in der Reihenfolge der Add-on-Liste eingefügt.

Hinweise:

- `\usepackage{…}` ist unproblematisch, wenn VisuTeX dasselbe Paket bereits lädt. Lädt VisuTeX es aber **mit anderen
  Optionen**, meldet LaTeX „Option clash“ – dann die Einstellungen über die Konfigurationsbefehle des Pakets setzen
  (z. B. `\sisetup`, `\captionsetup`, `\KOMAoptions`).
- Pakete, die laut ihrer Dokumentation **nach** `hyperref` geladen werden müssen, gehören nicht in die Add-on-Präambel.
- Präambelzeilen landen sichtbar im Dokument (LaTeX-Ansicht, `.tex`-Export) – das Dokument bleibt so portabel.
- Die Installationsabfrage zeigt Nutzern alle Präambelzeilen an, bevor sie ein Add-on installieren.

---

## Vorlagen

Vorlagen sind VisuTeX-Projektdateien (`.visutex`). Aktive Add-ons zeigen sie unter *Datei → Neu* als
„Vorlage aus dem Add-on „…““.

| Feld   | Regeln |
|--------|--------|
| `id`   | wie die Add-on-ID, eindeutig im Add-on |
| `name` | Anzeigename, höchstens 80 Zeichen |
| `file` | Pfad zur `.visutex`-Datei im Add-on |

So entsteht eine Vorlage:

1. Das Dokument in VisuTeX gestalten (Dokumenteinstellungen, Titelseite, Kopfzeilen, Beispielinhalt …).
2. *Datei → Speichern unter* → Dateityp **VisuTeX-Projekt (.visutex)**.
3. Die Datei in den Add-on-Ordner kopieren (z. B. `vorlagen/`) und in `templates` eintragen.

Befehle aus den eigenen TeX-Dateien des Add-ons funktionieren in der Vorlage, solange das Add-on aktiv ist.

---

## Installieren – so erleben es die Nutzer

Es gibt drei gleichwertige Wege; alle führen zur selben **Sicherheitsabfrage**:

1. **Doppelklick** auf die `.vtxaddon`-Datei im Explorer bzw. Dateimanager (VisuTeX startet mit der Datei).
2. *Datei → Öffnen* und die `.vtxaddon`-Datei wählen.
3. *Datei → Add-ons → Aus Datei installieren …* (`.vtxaddon` oder `.zip`) bzw. *Aus Ordner installieren …*.

Die Sicherheitsabfrage prüft das Add-on vollständig, **bevor** etwas installiert wird, und zeigt:

- Symbol, Name, Version, Autor/in und Beschreibung,
- die Schaltflächen im Menüband (mit Symbolen), Anzahl der Bausteine, die Vorlagen,
- die TeX-Dateien und **alle Präambelzeilen** (aufklappbar),
- Anzahl und Größe der Dateien,
- ob eine vorhandene Version ersetzt wird bzw. ob ein mitgeliefertes Add-on gleicher ID überdeckt wird,
- den Hinweis, Add-ons nur aus vertrauenswürdigen Quellen zu installieren.

Ist die Datei fehlerhaft, nennt die Abfrage die Ursache und bietet keine Installation an. Nach **Installieren**
(bzw. **Ersetzen**) ist das Add-on sofort aktiv. Ist VisuTeX schon in einem anderen Fenster geöffnet, übernimmt es die
neuen Schaltflächen, sobald man in dieses Fenster wechselt.

---

## So verarbeitet VisuTeX Add-ons

### Speicherorte

| Was | Wo |
|-----|----|
| installierte und selbst erstellte Add-ons | Windows: `%APPDATA%\com.mathias-lampert.visutex\addons\<id>\` · Linux: `~/.local/share/com.mathias-lampert.visutex/addons/<id>/` (*Datei → Add-ons → Add-on-Ordner öffnen*) |
| Ein/Aus-Zustand | `addons-state.json` im selben Ordner |
| mitgelieferte Add-ons | im Programm eingebettet, beim Start in den Cache-Ordner entpackt (nur lesbar) |
| Dateitypen | Windows: `HKCU\Software\Classes\.vtxaddon` (bzw. vom Installer), Linux: `/usr/share/mime/packages/visutex.xml` bzw. `~/.local/share/mime/packages/visutex.xml` |

### Installieren

- **Add-on-Datei/ZIP:** wird in einen Zwischenordner entpackt; Pfade, die aus dem Add-on herausführen, und mehr als
  200 MB werden abgelehnt. Erst wenn Manifest, alle genannten Dateien und alle SVG-Symbole geprüft sind, ersetzt der
  Zwischenordner eine vorhandene Version. **Bei einem Fehler wird nichts installiert.**
- **Ordner:** wird geprüft und in den Add-on-Ordner kopiert (ohne versteckte Dateien). Liegt der Ordner schon im
  Add-on-Ordner (Entwicklung), wird er nur geprüft und neu geladen – nie überschrieben.

### Aktivieren, Vorrang, Bearbeiten

- Installierte Add-ons sind sofort **aktiv**; mitgelieferte Add-ons (z. B. *Elektrotechnik*) sind anfangs **aus** und
  werden per Schalter eingeschaltet.
- Ein installiertes Add-on mit **derselben ID** wie ein mitgeliefertes hat Vorrang. So lässt sich ein mitgeliefertes
  Add-on anpassen: exportieren, ändern, installieren.
- Mitgelieferte Add-ons lassen sich nicht bearbeiten oder entfernen, nur exportieren.
- Ordner im Add-on-Ordner, die nicht geladen werden können (fehlerhaftes `addon.json`, fehlende Datei, unzulässiges
  Symbol, keine `addon.json`), zeigt die Verwaltung unter **„Wird nicht geladen:“** mit der Ursache.

### Was ein aktives Add-on bewirkt

| Wann | Was |
|------|-----|
| Menüband | Reiter **Add-ons** und *Einfügen → Meine Add-ons* mit den Bausteinen aus `ribbon` |
| *Datei → Neu* | Vorlagen aus `templates` |
| Kompilieren (PDF, Vorschauen von LaTeX-Blöcken und TikZ) | Präambelzeilen aller aktiven Add-ons + Ordner der TeX-Dateien als Suchpfade |
| Export als `.tex` | Präambelzeilen stehen im Dokument, TeX-Dateien werden daneben kopiert |
| Speichern als `.visutex` | Bausteine sind normaler Dokumentinhalt; das Add-on selbst wird **nicht** in die Datei kopiert |

Wird ein Dokument mit Befehlen aus einem Add-on auf einem anderen Rechner geöffnet, muss das Add-on dort ebenfalls
installiert sein – oder man gibt den `.tex`-Export weiter, der die TeX-Dateien enthält.

---

## Sicherheit

- Add-ons enthalten keinen Programmcode; VisuTeX führt aus einem Add-on nichts aus.
- Vor jeder Installation steht die Sicherheitsabfrage mit dem vollständigen Inhalt (inkl. Präambelzeilen).
- Pfade werden auf den Add-on-Ordner beschränkt, Paket- und Symbolgröße begrenzt, SVG-Symbole auf aktive Inhalte und
  externe Verweise geprüft und nur als Bild angezeigt.
- Die TeX-Engine läuft ohne Shell-Escape; `\write18` und `\openout` sind in Präambelzeilen zusätzlich verboten.
- TeX-Dateien sind trotzdem Programme für die TeX-Engine: Add-ons nur aus vertrauenswürdigen Quellen installieren –
  und als Ersteller nur Dateien mitliefern, die man selbst geprüft hat.

---

## Weitergeben, veröffentlichen, aktualisieren

- **Weitergeben:** *Als Add-on-Datei exportieren …* → `<id>-<version>.vtxaddon` per E-Mail, Netzlaufwerk, Website oder
  als Anhang eines GitHub-Releases verteilen. Empfänger installieren per Doppelklick.
- **Veröffentlichen:** Den Add-on-Ordner (mit `addon.json`, ohne `.vtxaddon`) in ein eigenes Repository legen, eine
  `LIESMICH.md` und eine Lizenz dazulegen und die `.vtxaddon`-Datei je Version als Release-Datei anhängen.
- **Aktualisieren:** `version` erhöhen und die neue Datei mit **derselben `id`** verteilen. Die Installationsabfrage
  zeigt „Ersetzt die installierte Version …“; der Ein/Aus-Zustand bleibt erhalten.
- **Plattformen:** Add-ons funktionieren unverändert unter Windows und Linux. Dateinamen nur in einer Schreibweise
  verwenden (Linux unterscheidet Groß- und Kleinschreibung).
- **Lizenzen:** Für mitgelieferte Pakete, Schriften und Logos die Weitergabe-Erlaubnis klären und die Lizenz ins
  Add-on legen.

---

## Fehlermeldungen

| Meldung | Ursache | Abhilfe |
|---------|---------|---------|
| *Keine gültige Add-on-Datei (.vtxaddon bzw. ZIP erwartet).* | Datei ist kein ZIP-Archiv | Mit *Als Add-on-Datei exportieren …* oder einem ZIP-Programm neu packen |
| *Die Add-on-Datei enthält keine addon.json.* | Manifest fehlt im Archiv | `addon.json` ins Wurzelverzeichnis bzw. einen Unterordner legen |
| *addon.json muss im Wurzelordner der Add-on-Datei oder in genau einem Unterordner liegen.* | zu tief verschachtelt | eine Ordnerebene weniger packen |
| *Unzulässiger Pfad in der Add-on-Datei: …* | Pfad mit `..` oder absolut | Archiv ohne solche Pfade erzeugen |
| *addon.json ist ungültig: …* | JSON-Syntaxfehler | Kommas, Anführungszeichen und verdoppelte Backslashes prüfen |
| *Die Add-on-ID darf nur Kleinbuchstaben, Ziffern, „-“ und „_“ enthalten …* | ungültige `id` | z. B. `mein-addon` statt `Mein Add-on` |
| *Unzulässige TeX-Datei im Add-on: „…“* | Endung nicht erlaubt oder unsicherer Pfad | Endung und Pfad prüfen (siehe [TeX-Dateien](#tex-dateien)) |
| *Im Add-on fehlt die Datei „…“* | Datei aus `texFiles`/`templates` nicht vorhanden | Pfad und Schreibweise prüfen |
| *Im Add-on fehlt das Symbol „…“* | SVG aus `icon` nicht vorhanden | Pfad prüfen |
| *Das Symbol „…“ enthält Unzulässiges (…)* / *… Ereignis-Attribute …* | aktive Inhalte im SVG | SVG ohne Skripte und `on…`-Attribute speichern |
| *Das Symbol „…“ verweist auf eine externe Datei.* | externer `href`/`src` oder `url(http…)` | Grafiken einbetten oder in Pfade umwandeln |
| *Das Symbol „…“ ist größer als 64 KB.* | zu große SVG-Datei | vereinfachen bzw. optimiert speichern |
| *Das Add-on-Symbol muss eine SVG-Datei im Add-on sein* | `icon` ist z. B. eine PNG-Datei | SVG verwenden |
| *Unzulässiger Symbolpfad „…“* | Symbolpfad führt aus dem Add-on heraus | relativen Pfad im Add-on verwenden |
| *Unzulässige Präambelzeile …* / *… weder \documentclass noch \begin{document} …* | verbotener Befehl oder Zeile zu lang | Zeile entfernen oder kürzen |
| *Ein Add-on darf höchstens 100 Präambelzeilen enthalten.* | zu viele Zeilen | Einstellungen in ein eigenes `.sty` auslagern |
| *Ungültige oder doppelte Snippet-ID „…“* | ID ungültig oder doppelt | eindeutige IDs vergeben |
| *Ribbon-Eintrag „…“ verweist auf ein unbekanntes Snippet „…“* | Tippfehler in `snippet` | ID des Bausteins eintragen |
| *Ungültige Vorlage „…“ im Add-on.* | ungültige `id` oder unsicherer Pfad | ID und Pfad prüfen |
| *Mitgelieferte Add-ons können nicht überschrieben werden …* | Add-on-Editor mit ID eines mitgelieferten Add-ons | andere ID wählen oder als Datei installieren |
| *Das Add-on ist zu groß (maximal 200 MB).* | entpackt zu groß | große Dateien entfernen |
| *Der Ordner enthält keine addon.json.* | Ordner im Add-on-Ordner ohne Manifest | `addon.json` anlegen oder Ordner entfernen |
| LaTeX: *File `….sty' not found* | Paket weder im Bundle noch im Add-on | Datei in `texFiles` aufnehmen |
| LaTeX: *Option clash for package …* | Paket mit anderen Optionen als von VisuTeX geladen | Einstellungen per Konfigurationsbefehl setzen |

---

## Checkliste vor dem Veröffentlichen

- [ ] `id` endgültig gewählt (Kleinbuchstaben, Ziffern, `-`, `_`)
- [ ] `version` erhöht, wenn eine neue Fassung verteilt wird
- [ ] alle Pfade relativ, mit `/`, Dateinamen eindeutig und in einer Schreibweise
- [ ] Backslashes in `addon.json` verdoppelt
- [ ] eigene Pakete kompilieren mit pdfLaTeX, XeLaTeX und LuaLaTeX
- [ ] Symbole als SVG, `viewBox="0 0 24 24"`, `currentColor`, unter 64 KB, ohne Skripte und externe Verweise
- [ ] im Add-on-Ordner getestet: **Neu laden** ohne Eintrag unter „Wird nicht geladen“, jede Schaltfläche ausprobiert,
      Dokument kompiliert, Vorlage geöffnet
- [ ] `.tex`-Export getestet (kompiliert auch außerhalb von VisuTeX)
- [ ] Lizenzen mitgelieferter Dateien (Pakete, Schriften, Logos) geklärt und beigelegt
- [ ] als `.vtxaddon` exportiert und auf einem zweiten Rechner (oder unter Linux) per Doppelklick installiert

---

## Für Entwickler von VisuTeX: wo was im Code steht

Wer das Add-on-System selbst erweitert (neue Felder, neue Bausteinarten), findet hier die Stellen:

| Datei | Aufgabe |
|-------|---------|
| `src-tauri/src/core/addon.rs` | Manifest (`AddonManifest`), alle Grenzen und Prüfungen, SVG-Prüfung (`check_svg_icon`) |
| `src-tauri/src/addons.rs` | Installieren (Datei/ZIP/Ordner), Vorschau ohne Installation (`inspect`), Ein/Aus, Export als `.vtxaddon`, Problemliste, Präambel/Suchpfade für Kompilieren und Export |
| `src-tauri/src/launch.rs` | per Doppelklick übergebene Dateien (`startup_files`) |
| `src-tauri/src/setup.rs`, `src-tauri/tauri.conf.json` (`fileAssociations`), `packaging/linux/visutex-mime.xml` | Dateitypen `.vtxaddon`, `.visutex`, `.vtxslides` (Windows-Registrierung, MIME-Typen, Pakete) |
| `src-tauri/src/core/preamble.rs` | Position der Add-on-Präambel (vor `hyperref`) |
| `src/components/AddonInstallDialog.tsx` | Sicherheitsabfrage |
| `src/components/AddonManager.tsx` | Verwaltung und Add-on-Editor |
| `src/components/AddonIcon.tsx`, `src/components/Ribbon.tsx` | Symbole und Menüband (Reiter *Add-ons*, *Einfügen → Meine Add-ons*) |
| `src/App.tsx` (`insertSnippet`) | was jede Bausteinart ins Dokument einfügt |

Tests: `cargo test --lib addon` (Manifest, SVG, Installation, Vorschau, mitgelieferte Add-ons und das Beispiel),
`cargo test --lib setup` (Dateitypen stimmen mit `tauri.conf.json` und der MIME-Datei überein),
`cargo test --test compile_pipeline -- --ignored example_addon` (Beispiel-Add-on kompiliert mit der eingebauten
Engine). Neue Felder immer in `core/addon.rs` prüfen, in dieser Anleitung beschreiben und das Beispiel ergänzen.
