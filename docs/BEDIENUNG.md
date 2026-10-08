# Bedienung

## Installieren und Updates

Die heruntergeladene Programmdatei (`VisuTeX_<Version>_windows_x64.exe` bzw. das AppImage) bringt alles mit. Beim ersten
Start erscheint **„VisuTeX einrichten“**:

- **Installationsort** – Vorschlag `%LOCALAPPDATA%\Programs\VisuTeX` (Windows) bzw. `~/.local/share/VisuTeX` (Linux),
  über *Ändern …* frei wählbar; keine Administratorrechte nötig.
- **Verknüpfung auf dem Desktop**, **Eintrag im Startmenü** (Linux: Anwendungsmenü), **Mit Windows starten** (Linux: beim
  Anmelden) und **Beim Start nach Updates suchen**.
- *Installieren und starten* kopiert das Programm dorthin und startet die installierte Fassung; die heruntergeladene Datei
  kann danach gelöscht werden. *Ohne Installation verwenden* startet direkt (portabel) und fragt nicht erneut.
- Wird später eine neuere heruntergeladene Datei gestartet, erscheint der Bildschirm wieder und ersetzt die installierte
  Version (Einstellungen und Dokumente bleiben erhalten). Über das Installationsprogramm (`…-setup.exe`, `.msi`, `.deb`,
  `.rpm`) installierte Fassungen zeigen keinen Einrichtungsbildschirm.

**Deinstallieren:** Windows *Einstellungen → Apps → VisuTeX* oder *Datei → Info → Deinstallieren …*. Entfernt werden
Programmdatei, Verknüpfungen, Autostart und entpackte Programmdaten – nur Dateien von VisuTeX, der Ordner nur, wenn er
danach leer ist. Dokumente bleiben unverändert.

**Updates:** Beim Start (abschaltbar unter *Datei → Optionen*) und über *Datei → Info → Nach Updates suchen* fragt
VisuTeX bei GitHub nach einer neueren Version (auch Vorabversionen). Der Dialog zeigt die Versionshinweise;
*Jetzt aktualisieren* lädt die neue Programmdatei, ersetzt die laufende und startet neu. *Diese Version überspringen*
unterdrückt den Hinweis bis zur nächsten Version. Ohne Internet passiert nichts. Vor dem Ersetzen wird die Prüfsumme
(SHA-256) der Datei kontrolliert. Wurde mit der Setup.exe installiert, lädt VisuTeX die neue Setup.exe und führt sie
ohne Rückfragen aus (die Version unter „Apps“ stimmt danach). Bei MSI, `.deb` und `.rpm` öffnet *Release-Seite* die
Download-Seite (Aktualisierung über das Installationsprogramm bzw. die Paketverwaltung).

## Überblick

VisuTeX hat zwei Arbeitsbereiche, die jederzeit gewechselt werden können:

| Bereich | Wofür | Datei |
|---|---|---|
| **Dokument** (wie ein Textverarbeitungsprogramm) | Berichte, Abschlussarbeiten, Artikel – visuell oder als LaTeX-Code | `.visutex` oder `.tex` |
| **Folien** (Präsentationen) | Folien frei gestalten, Elemente auf einem Raster platzieren | `.vtxslides` |

### Zwischen Dokument und Präsentation wechseln

- **Über *Datei*:** Dokument und Folien-Editor haben **denselben Datei-Bereich** (Neu mit allen Dokumentvorlagen und
  Präsentationen, Öffnen, Zuletzt verwendet, Speichern, Exportieren, Add-ons, Tastenkürzel, Optionen, Info).
  - *Datei → Neu* → eine Dokumentvorlage wählen → Dokumentansicht; *Neue/Leere Präsentation* → Folien-Editor.
  - *Datei → Neu → Zur offenen Präsentation* bzw. *Zum offenen Dokument* wechselt, ohne etwas zu schließen.
  - *Datei → Öffnen* (oder `Strg+O`, in beiden Bereichen) öffnet Dokumente **und** Präsentationen – `.vtxslides` landen im
    Folien-Editor, `.visutex`/`.tex` in der Dokumentansicht.
  - *Speichern*, *Speichern unter* und *Exportieren* wirken auf den Bereich, aus dem *Datei* geöffnet wurde.
- **Titelleiste oben rechts:** zusätzlich `Visuell` · `LaTeX` · **`Folien`**; im Folien-Editor oben rechts
  **„Zum Dokument“**. Die Präsentation bleibt dabei geöffnet (auch ungespeichert) und erscheint beim nächsten Wechsel wieder.
- Beide Bereiche sind unabhängig: Dokument und Präsentation können gleichzeitig offen sein und werden getrennt gespeichert.

## Dokument

- **Menüband:** Start (Schrift, Absatz, Formatvorlagen), Einfügen (Tabellen, Bilder, Formeln, Skizzen, Querverweise,
  Fußnoten, Umgebungen …), Layout (Seite, Ränder, Spalten, Sprache), Referenzen (Zitate, Verzeichnisse), Ansicht.
- **Code-Ansicht** (`LaTeX` in der Titelleiste): das vollständige, portable LaTeX-Dokument. Änderungen dort werden beim
  Zurückschalten übernommen; unveränderte Teile bleiben zeichengenau erhalten.
- **Kompilieren:** `F5` bzw. *Ansicht → Kompilieren*. Die eingebaute Engine braucht keine LaTeX-Installation; Fehler stehen
  mit Zeilennummer in der PDF-Vorschau, das PDF entsteht trotzdem.
- **Lange Inhalte in der Seitenansicht:** Absätze und Listen werden wie in LaTeX auf die nächste Seite umbrochen.
  Tabellen bleiben standardmäßig zusammen (Gleitumgebung); ist eine Tabelle höher als eine Seite, wird sie rot markiert –
  dann *Tabelle → „Über Seiten umbrechen“* aktivieren (`longtable`: geht auf der nächsten Seite weiter, Kopfzeile wird
  wiederholt). Bilder werden beim Kompilieren auf höchstens 90 % der Seitenhöhe begrenzt. Andere Blöcke, die höher als
  eine Seite sind (z. B. sehr lange Formeln oder Roh-LaTeX), werden ebenfalls markiert.
- **Bilder nebeneinander** (*Einfügen → Bilder nebeneinander*): Unterabbildungen (a), (b) … mit eigener Unterbeschriftung
  und eigenem Label sowie einer gemeinsamen Beschriftung (Paket `subcaption`). Block anklicken → je Bild „Bild wählen …“,
  Unterbeschriftung, Label und Breite; Pfeile ändern die Reihenfolge, „Bild hinzufügen …“ ergänzt weitere. Querverweise
  auf einzelne Teilbilder (`\ref{fig:teil-a}` → „1a“) und auf die ganze Abbildung sind möglich.
- **Größen** (*Einfügen → Größe*, siunitx): als Wert nur Zahlen (`4.7`, `1,5`, `1e-3`, `3 \pm 0.1`); Text wird abgelehnt.
- **PDF/A** (Archivformat, oft für Abschlussarbeiten verlangt): *Dokumenteinstellungen → PDF-Metadaten → PDF-Standard*
  (PDF/A-2b oder PDF/A-3b). Der LaTeX-Export beginnt dann mit `\DocumentMetadata{pdfstandard=A-2b, …}`; TeX Live,
  MiKTeX und Overleaf (ab 2022) erzeugen daraus ein PDF/A mit eingebetteten Schriften, Metadaten und Farbprofil.
  Die eingebaute Engine kennt diese Angabe nicht, überspringt sie mit Hinweis und erzeugt ein normales PDF.
- **Zoom:** Strg + Mausrad (oder Zwei-Finger-Geste auf dem Touchpad), *Ansicht → Zoom* bzw. der Regler unten rechts.
  Strg + Mausrad wirkt auch in der PDF-Vorschau, im Folien-Editor und im Skizzen-Fenster.
- **Kommentare** (wie in Word): LaTeX-Kommentare (`% …`) erscheinen als Sprechblasen rechts neben der Seite und nehmen im
  Text keinen Platz ein (im PDF sind sie unsichtbar). Klick auf den Text bearbeitet, ✕ löscht; *Einfügen → Kommentar* legt
  einen neuen an (nach dem aktuellen Absatz). Reine Trennlinien wie `% =====` werden nicht angezeigt, bleiben aber im Code.
- **Unsichtbare Layout-Befehle** (`\setcounter`, `\setlength`, `\begingroup`/`\endgroup`, `\setstretch`, `\pagestyle` …),
  die allein in einer Zeile stehen, erzeugen im PDF nichts Sichtbares. Sie erscheinen deshalb als kleine graue Markierung
  am **linken** Seitenrand (rechts stehen die Kommentare) und nehmen im Text keinen Platz ein. Überfahren zeigt den ganzen
  Befehl, Klick bearbeitet, ✕ löscht. Mit eingeschalteten Formatierungszeichen (¶) stehen sie wieder als eigene Zeile im Text.
- **Titelseiten aus LaTeX** (`\begin{titlepage}` in fremden Dokumenten) erscheinen als ganze Seite genau wie im PDF.
- **Formatierungszeichen** (wie „Alle anzeigen“ in Word): *Start → Absatz →* ¶ bzw. `Strg+Umschalt+*` zeigt Absatzmarken (¶),
  Leerzeichen (·), geschützte Leerzeichen (°) und Zeilenumbrüche (↵). Nur Anzeige – Dokument, Seitenumbrüche und
  LaTeX bleiben unverändert; die Einstellung bleibt gespeichert.
- **Lineal:** *Ansicht → Lineal*; die Dreiecke verschieben den linken/rechten Seitenrand (Alt: feine Schritte).
- **Literatur:** *Referenzen → Zitat einfügen* – Einträge aus Zotero (Zotero muss laufen) oder als BibTeX; die Datei
  `literatur.bib` liegt im Projektordner. Zitierstil unter *Referenzen* bzw. *Dokumenteinstellungen → Literatur*:
  **IEEE (deutsch)** schreibt „und“, „u. a.“, „Hrsg.“, „2. Aufl.“, „Bd.“, „Nr.“, „S.“, deutsche Monatsnamen und
  „Anführungszeichen“; deutsche Vorlagen verwenden ihn automatisch, und beim Wechsel der Dokumentsprache schaltet VisuTeX
  zwischen IEEE und IEEE (deutsch) um. Die Stildatei `visutex-ieee-de.bst` wird beim LaTeX-Export neben das Dokument
  gelegt (kompiliert damit auch in TeX Live, MiKTeX und Overleaf).
- **Fremde LaTeX-Projekte** (`.tex`, auch mit `\input`/`\include`, Bildern in Unterordnern, eigener Präambel) lassen sich
  öffnen, visuell bearbeiten und wieder als LaTeX speichern.

## Skizzen (TikZ und CircuiTikZ)

*Einfügen → Skizze* öffnet das Skizzen-Fenster (verschiebbar, in der Größe änderbar).

- **Zeichnen:** Linie, Pfeil, Linienzug, Vieleck, Kurve, Freihand, Rechteck, Kreis, Ellipse, Bogen, Text, Verbindungspunkt.
- **Palette links:** alle CircuiTikZ-Bauteile (Widerstände, Quellen, Dioden, Transistoren, Operationsverstärker,
  Logikgatter, Messgeräte, Schalter, Übertrager, Blockschaltbild …) und TikZ-Formen – mit Suche (deutsche/englische Namen
  oder CircuiTikZ-Name).
  - **Zweipole** (Widerstand, Quelle …): vom Anfangs- zum Endpunkt ziehen.
  - **Mehrpole** (Transistor, OPV, Gatter …): klicken. Blaue Punkte zeigen die **Anschlüsse**; Leitungen docken dort an
    und folgen beim Verschieben.
- **Beschreibung:** im Eigenschaftenfeld rechts – Beschriftung (z. B. `R_1`), Wert (z. B. `10 kΩ`), Spannungs-, Strom- und
  Flusspfeil mit Seite/Richtung, Invertieren, Spiegeln; bei Mehrpolen Name, Beschriftung und deren Position, Drehung.
- **Text:** frei platzierbar, Größe, fett/kursiv, Farbe, Drehung, Satz als Text oder Formel (`$…$`, `_`, `\Omega` werden
  automatisch als Formel gesetzt).
- **Stil:** Farbe, Füllung, Linienstärke, Linienart, Pfeilspitzen an beiden Enden.
- **Verschieben:** Werkzeug *Auswählen/Verschieben* (Hand, Taste `V`) – Element greifen und ziehen.
- Tasten: `Entf`/`Rücktaste` löschen · Pfeiltasten verschieben (Umschalt ×5, Alt ½ Raster) · `R` drehen ·
  `Strg+C`/`X`/`V` kopieren/ausschneiden/einfügen · `Strg+D` duplizieren · `Strg+Z`/`Strg+Y` (bzw. `Strg+Umschalt+Z`) ·
  `Strg++`/`Strg+-`/`Strg+0` Zoom · `Enter` beendet Linienzug/Vieleck · `Esc` bricht ab bzw. hebt die Auswahl auf ·
  `L`/`T` Linie/Text. Alt beim Ziehen = halbe Rasterschritte.
- „An Cursorposition einfügen“ setzt den Code ins Dokument; „In Skizze bearbeiten“ an einer eingefügten Skizze öffnet sie wieder.

## Folien

- **Datei** (links neben den Registern): derselbe Datei-Bereich wie im Dokument, zusätzlich *Eigenschaften* der
  Präsentation (Titel, Untertitel, Autor, Datum, Sprache); Exportieren bietet PDF und LaTeX-Beamer.
- **Folien** links (Ziehen ordnet um), **Arbeitsfläche** in der Mitte, **Format** rechts (Position, Größe, Drehung,
  Füllung, Linie, Text, Formel), **Notizen** unten.
- **Einfügen:** Textfeld, Formen, Bild, Formel; Textfelder und Formen werden auf der Fläche aufgezogen.
- **Bearbeiten:** Doppelklick auf Text; Ziehen verschiebt mit Raster und intelligenten Hilfslinien (Alt: frei), Anfasser
  skalieren (Umschalt: Seitenverhältnis) und drehen.
- **Tasten:** `Entf`/`Rücktaste` löschen, Pfeiltasten verschieben, `Strg+C/X/V/D`, `Strg+Z/Y`, `Esc`.
- **Anordnen:** Vordergrund/Hintergrund, Ausrichten, Verteilen, Raster und Rasterweite.
- **Entwurf:** 16:9 oder 4:3, Hintergrund, Text- und Akzentfarbe, Schrift.
- **Präsentieren:** *Präsentieren* bzw. `F11`; Pfeiltasten/Leertaste/Klick weiter, `Esc` beendet.
- **Speichern** (`Strg+S`) als `.vtxslides`; **Exportieren** als PDF oder als LaTeX-Beamer (`.tex`).

## Tastenkürzel

*Datei → Tastenkürzel* (oder *Ansicht → Tastenkürzel*): alle Befehle mit Belegung, „+“ klicken und die gewünschte
Kombination drücken. Konflikte werden angezeigt, jeder Befehl lässt sich auf den Standard zurücksetzen.

## Sprachen

- **Oberfläche:** *Datei → Optionen → Sprache der Oberfläche* – Deutsch, English, Français, Español, Italiano, Português,
  Nederlands, Polski. Beim ersten Start gilt die Systemsprache.
- **Dokumentsprache:** *Layout → Sprache* oder *Dokumenteinstellungen* – 24 Sprachen. Sie bestimmt Silbentrennung,
  Anführungszeichen, Dezimalzeichen bei Größen, Datumsformat, die Namen der Verzeichnisse und die Rechtschreibprüfung.
  Neue leere Dokumente übernehmen die Sprache der Oberfläche.

## Add-ons

*Datei → Add-ons*: Add-ons (ZIP oder Ordner) bringen Präambelzeilen, TeX-Dateien, Bausteine im Menüband und Vorlagen mit –
deklarativ, ohne ausführbaren Code. Eigene Add-ons lassen sich direkt in der App erstellen und exportieren.
