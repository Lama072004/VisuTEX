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
ohne Rückfragen aus (die Version unter „Apps“ stimmt danach). Wurde mit dem **MSI** installiert, lädt VisuTeX das neue
MSI; nach dem Beenden fragt Windows einmal nach Administratorrechten, das Installationsprogramm ersetzt die Installation
im bisherigen Ordner (mit Fortschrittsbalken) und VisuTeX startet neu. Bei `.deb` und `.rpm` öffnet *Release-Seite*
die Download-Seite (Aktualisierung über die Paketverwaltung).

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

- **Menüband:** Start (Schrift, Absatz, Formatvorlagen), Einfügen (Tabellen, Bilder, Formeln, Skizzen, Diagramme,
  Querverweise, Fußnoten, Umgebungen …), Layout (Seite, Ränder, Spalten, Sprache), Referenzen (Zitate, Verzeichnisse),
  Überprüfen (Abgabe-Check, Versionsverlauf, Vergleichen), Ansicht.
- **Schnellzugriff** (Titelleiste links, wie in Office): Speichern, Rückgängig, Wiederholen und beliebige weitere Befehle.
  Das kleine **▾** daneben blendet häufige Befehle ein/aus; *Weitere Befehle …* bzw. *Datei → Optionen →
  Symbolleiste für den Schnellzugriff* bietet alle Befehle zum Hinzufügen, Entfernen und Umsortieren (höchstens 16).
- **Teilen:** Rechtsklick auf die Titelleiste → *Gespeicherte Datei teilen …* (auch als Befehl „Teilen …“ für den
  Schnellzugriff oder ein Tastenkürzel). Der Dialog nennt ausdrücklich, **welche Datei** geteilt wird (Name, Pfad,
  Speicherzeitpunkt): immer die **zuletzt gespeicherte Fassung**. Gibt es ungespeicherte Änderungen, weist er darauf hin
  und bietet *Speichern und teilen* an. Windows öffnet sein Teilen-Fenster (E-Mail, Teams, Nearby Sharing, OneDrive …),
  Linux das E-Mail-Programm mit der Datei als Anhang. Außerdem: *Im Ordner anzeigen* und *Dateipfad kopieren*.
- **Formeln (wie in Word/OneNote):** *Einfügen → Formel* (abgesetzt) bzw. *Inline-Formel* öffnet sofort ein leeres
  Formelfeld. Darin wird **grafisch** getippt: `/` erzeugt einen Bruch, `^` und `_` Hoch-/Tiefstellung, Tab bzw.
  Pfeiltasten springen zwischen den Feldern (Zähler, Nenner, Grenzen …), `lpha` + Leertaste ergibt α. Sobald eine
  Formel ausgewählt ist, erscheint die Registerkarte **Formel**:
  - *Tools:* neue Formel, Formel im Text, **Häufige Formeln** (Pythagoras, Lösungsformel, binomischer Lehrsatz,
    Fourier-/Taylor-Reihe, Ohmsches Gesetz …).
  - *Bearbeitung:* zwischen **Grafisch** und **LaTeX**-Code umschalten, normalen Text einfügen.
  - *Symbole:* Grundlegende Mathematik, griechische Buchstaben (klein/groß), buchstabenähnliche Symbole (ℕ, ℝ …),
    Operatoren, Relationen, negierte Relationen, Pfeile, Mengen und Logik, Geometrie.
  - *Strukturen:* Bruch, Skript (hoch/tief), Wurzel, Integral, großer Operator (Summe, Produkt …), Klammer (inkl.
    Betrag, Fallunterscheidung), Funktion, Akzent, Grenzwert und Log, Operator, Matrix – jeweils als Galerie mit
    Vorschau; Platzhalter füllt man direkt aus.

  Mehrzeilige Formeln (`align`, `gather` …) öffnen im LaTeX-Modus (mit Live-Vorschau). Eine Formel wird nur
  umgeschrieben, wenn man sie tatsächlich ändert; eine geleerte Formel wird entfernt. Esc kehrt in den Text zurück.
- **Markieren und löschen (wie in Word):** Was mit der Maus (Ziehen, Umschalt+Klick) oder der Tastatur (Umschalt+Pfeile,
  Strg+A) markiert ist, löschen `Entf`/`Rücktaste` vollständig – Text, Tabellen, Bilder, Zeichnungen (TikZ), Formeln,
  LaTeX-Blöcke, Verzeichnisse, Umbrüche, Kommentare usw. Alles Markierte erscheint – wie Text – blau hinterlegt, auch
  Skizzen, Bilder, Formeln und ganze Tabellen bzw. Boxen. Innerhalb einer Markierung öffnet sich kein Bearbeitungsfeld.
  Ein **einzelner Block** wird per Klick ausgewählt; `Entf`/`Rücktaste` löschen ihn, ebenso der Knopf *Löschen* im
  Bearbeitungsfeld (Zeichnung, Formel). Bearbeiten: Doppelklick, `Eingabetaste`/`F2` oder einfach lostippen (bei
  Zeichnungen landet der Text im Code); `Esc` kehrt zum ausgewählten Block zurück. Formeln öffnen per Klick sofort den
  Formel-Editor; per Tastatur erreicht (Pfeiltasten, nach dem Löschen eines Nachbarblocks) sind sie nur markiert.
- **Code-Ansicht** (`LaTeX` in der Titelleiste): das vollständige, portable LaTeX-Dokument. Änderungen dort werden beim
  Zurückschalten übernommen; unveränderte Teile bleiben zeichengenau erhalten.
- **Kompilieren:** `F5` bzw. *Ansicht → Kompilieren*. Die eingebaute Engine braucht keine LaTeX-Installation; Fehler stehen
  mit Zeilennummer in der PDF-Vorschau, das PDF entsteht trotzdem.
- **PDF ↔ Dokument (SyncTeX):** **Doppelklick ins PDF** springt an die passende Stelle im Dokument (visuell: der Absatz
  blinkt kurz auf; Code-Ansicht: die Zeile). Umgekehrt zeigt *Ansicht → Im PDF zeigen* bzw. `Strg+Alt+J` (auch der
  Fadenkreuz-Knopf in der PDF-Vorschau) die Cursorstelle im PDF mit einer Markierung. Nach Änderungen gilt die Zuordnung
  des letzten Kompilierens – bei Abweichungen neu kompilieren (`F5`).
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
- **Literatur per DOI/ISBN:** *Referenzen → Per DOI/ISBN …* (bzw. Reiter *DOI / ISBN* im Zitat-Dialog) – DOI
  (`10.1038/nature14539` oder als Link `https://doi.org/…`) bzw. ISBN eines Buchs eingeben, *Suchen*. VisuTeX holt die
  Literaturangabe (DOI über doi.org, ISBN über OpenLibrary), zeigt Titel, Autoren, Jahr und das BibTeX und vergibt einen
  lesbaren Schlüssel (`lecun2015`, bei Doppelungen `lecun2015a`). *Zur Literaturdatei hinzufügen* schreibt den Eintrag
  in die `.bib`-Datei und wählt ihn zum Zitieren aus. Braucht Internet – nur diese eine Anfrage, nur auf Knopfdruck.
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

## Diagramme aus Daten

*Einfügen → Diagramm* (wie „Diagramm einfügen“ in Word) – das Diagramm entsteht als Vektorgrafik mit `pgfplots`
(mitgeliefert) und kompiliert auch in TeX Live, MiKTeX und Overleaf.

- **Diagrammart:** Säulen, Balken, Linie, Fläche, Punkte (XY), Kreis.
- **Daten:** Tabelle im Dialog – erste Zeile = Namen der Datenreihen, erste Spalte = Kategorien bzw. x-Werte. Einen
  Bereich in **Excel/LibreOffice kopieren** und mit `Strg+V` in eine Zelle einfügen (in die erste Zelle: ersetzt die
  ganze Tabelle) oder *CSV laden …* (Trennzeichen `;`, `,` oder Tabulator). Zahlen mit Dezimalkomma (`12,5`),
  Tausenderpunkt (`1.234,5`) und `%` werden erkannt. `Eingabetaste` springt eine Zeile tiefer, Knöpfe fügen
  Zeilen/Reihen an, *Tauschen* vertauscht Zeilen und Spalten.
- **Optionen:** Titel, Achsenbeschriftungen, Legende (Standard: unter dem Diagramm), Breite, Gitternetz, gestapelt
  (Säulen/Balken), Datenpunkte markieren und Linien glätten (Linie/Fläche), Dezimalkomma (richtet sich nach der
  Dokumentsprache). Jahreszahlen als x-Werte erscheinen ohne Tausendertrennung.
- **Beschriftung und Label:** werden zur Abbildungsunterschrift bzw. zum Ziel für Querverweise (`fig:…`).
- Die Vorschau im Dialog erzeugt die eingebaute Engine. **Doppelklick** auf ein eingefügtes Diagramm (oder
  *Diagramm bearbeiten* im Bearbeitungsfeld) öffnet den Dialog wieder mit allen Daten. Das Modell steht als erste
  Kommentarzeile (`% VisuTeX-Diagramm: …`) im Code; von Hand geänderter Diagrammcode bleibt erhalten, lässt sich danach
  aber nur noch als Code bearbeiten.

## Überprüfen

### Abgabe-Check

*Überprüfen → Abgabe-Check* prüft das Dokument vor der Abgabe und listet die Funde links, gruppiert nach Bereich
(Fehler zuerst); ein Klick springt zur Stelle:

- **Querverweise:** Verweis auf unbekanntes Label, doppelt vergebene oder nie verwendete Labels.
- **Literatur:** zitierte Quelle fehlt in der `.bib`, Zitate ohne Literaturdatei, Literaturdatei nicht gefunden.
- **Abbildungen und Tabellen:** ohne Beschriftung, Bilddatei fehlt.
- **Gliederung:** leere Abschnitte, übersprungene Ebenen (z. B. Kapitel → Unterabschnitt).
- **Inhalt:** Platzhalter und Notizen (`TODO`, `FIXME`, `XXX`, „Lorem ipsum“, Vorlagenfelder wie `[Titel der Arbeit]`),
  leere Titelseitenfelder, Kommentare.
- **Kompilieren und Seitenlayout:** Fehler des letzten Kompilierens, über den Rand ragender Text (Overfull hbox) und zu
  volle Seiten – dafür vorher kompilieren (`F5`).

Nach Änderungen *Erneut prüfen* (↻).

### Versionsverlauf

Jedes **Speichern** legt automatisch eine Version an – mit Text, Bildern und Literatur (unveränderter Stand: keine
neue). *Überprüfen → Versionsverlauf* zeigt sie nach Tagen mit Uhrzeit, Name und Wortzahl (inkl. Zu-/Abnahme) und oben,
wo sie liegen:

- **Version speichern …** sichert den aktuellen Stand unter einem Namen (z. B. „Abgabe“). Benannte Versionen bleiben
  immer erhalten; von den automatischen alle der letzten 24 Stunden, davor die letzte je Tag.
- **Klick** auf eine Version vergleicht sie mit dem aktuellen Stand; über die Knöpfe daneben: vergleichen,
  **wiederherstellen** (der aktuelle Stand wird vorher als Version gesichert; Bilder und Literatur der Version kommen
  zurück, wenn sie abweichen; übernommen ist die alte Fassung erst nach dem Speichern), umbenennen, löschen.
- **Projektdateien (`.visutex`)** tragen ihren Verlauf in sich (siehe unten) – er reist beim Verschieben, Umbenennen
  und Kopieren mit. Abschalten: *Datei → Optionen → „Versionsverlauf in der Projektdatei speichern“* (dann nur auf
  diesem Computer). *Speichern unter* nimmt den Verlauf mit.
- **Einzelne `.tex`-Dateien:** Ort wählbar unter *Datei → Optionen → „Versionsverlauf einzelner .tex-Dateien“* –
  auf diesem Computer (App-Datenordner, Standard), neben dem Dokument (Ordner `.visutex-versionen`) oder ein eigener
  Ordner (z. B. in der Cloud).

### Projektdatei (`.visutex`) – eine Datei wie `.docx`

Eine `.visutex`-Datei ist ein **ZIP** mit allem, was zum Dokument gehört:

```text
Bachelorarbeit.visutex
├── Bachelorarbeit.tex      vollständiges LaTeX-Dokument
├── Abbildungen/…, literatur.bib, Teildateien
├── LIESMICH.txt            Hinweis für alle ohne VisuTeX
└── visutex/                nur für VisuTeX: Ansicht, Einstellungen, Versionsverlauf
```

- **Ohne VisuTeX weiterarbeiten:** Datei entpacken (oder in `.zip` umbenennen) und `Bachelorarbeit.tex` in einem
  beliebigen LaTeX-Editor öffnen (TeXstudio, TeXworks, VS Code …) – oder das ZIP bei Overleaf hochladen. Der Ordner
  `visutex/` kann dabei ignoriert werden.
- **Teilen** schickt standardmäßig eine Kopie **ohne** Versionsverlauf (Empfänger sehen keine früheren Fassungen);
  „Versionsverlauf mitschicken“ im Teilen-Dialog nimmt ihn mit.
- **Datei → Exportieren → LaTeX-Projekt (ZIP):** `.tex`, Bilder und Literatur ohne VisuTeX-Daten – zum Hochladen bei
  Overleaf oder für jeden anderen Editor.
- Gespeichert wird immer erst in eine temporäre Datei, die dann die alte ersetzt: Ein Absturz beim Speichern lässt die
  bisherige Datei unversehrt. Ältere `.visutex`-Dateien (JSON) öffnen weiterhin und werden beim nächsten Speichern
  zur Projektdatei. Beim Öffnen entpackt VisuTeX in einen Arbeitsordner im App-Datenordner; neu eingefügte Bilder
  landen beim Speichern in der Datei.

### Vergleichen

Der Vergleich zeigt geänderte Stellen mit etwas Kontext: gelöschte Zeilen rot, neue grün, **geänderte Wörter
hervorgehoben**; oben die Bilanz (Änderungen, Wörter mehr/weniger) und Pfeile zur vorherigen/nächsten Änderung.
*Überprüfen → Mit Datei vergleichen …* vergleicht den aktuellen Stand mit einer beliebigen `.tex`-Datei (z. B. der
Fassung mit Anmerkungen der Betreuerin oder des Betreuers).

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

Add-ons funktionieren wie Add-ins bei Office: Ein Add-on ist eine Datei mit dem Dateityp **`.vtxaddon`**. Sie bringen
Präambelzeilen, TeX-Dateien, Bausteine im Menüband und Vorlagen mit – deklarativ, ohne ausführbaren Code.

- **Installieren:** Doppelklick auf die `.vtxaddon`-Datei, *Datei → Öffnen* oder *Datei → Add-ons → Aus Datei
  installieren …* (auch `.zip`) bzw. *Aus Ordner installieren …*. Vorher zeigt eine **Sicherheitsabfrage**, was das
  Add-on mitbringt (Schaltflächen, Vorlagen, TeX-Dateien, alle Präambelzeilen) und ob eine vorhandene Version ersetzt
  wird.
- **Verwenden:** Reiter **Add-ons** im Menüband oder *Einfügen → Meine Add-ons*; Vorlagen unter *Datei → Neu*.
- **Verwalten** (*Datei → Add-ons*): ein-/ausschalten, bearbeiten, **als Add-on-Datei exportieren**, entfernen.
  *Add-on-Ordner öffnen* und *Neu laden* helfen beim Entwickeln; fehlerhafte Add-ons stehen mit Ursache unter
  „Wird nicht geladen“.
- **Eigene Add-ons erstellen:** *Eigenes Add-on erstellen* (ohne Programmieren); Add-on und Menüband-Schaltflächen
  erhalten auf Wunsch ein eigenes **SVG-Symbol** (*SVG wählen …*), das sich dem hellen bzw. dunklen Design anpasst.

Die vollständige Anleitung für Add-on-Entwickler (Dateityp, `addon.json`, Symbole, TeX-Dateien, Beispiel,
Fehlermeldungen) steht in [ADDONS.md](ADDONS.md).
