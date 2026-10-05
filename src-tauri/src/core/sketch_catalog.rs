//! Katalog des Skizzier-Werkzeugs: CircuiTikZ-Bauteile (Zweipole für `to[…]`,
//! Mehrpole/Knoten für `node[…]`) und TikZ-Formen (`node[<Form>]`).
//!
//! Vorschaubilder und Anschlusspunkte erzeugt
//! `cargo run --features dev-tools --bin build-sketch-symbols` aus genau dieser
//! Liste (→ `resources/sketch-symbols.json`). Einträge, die die mitgelieferte
//! CircuiTikZ-Version nicht kennt, sortiert der Generator beim Kompilieren aus –
//! die App zeigt nur Einträge mit erzeugtem Symbol.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    /// Zweipol: `\draw (a) to[<id>] (b);`
    Bipole,
    /// Knoten mit Anschlüssen: `\draw (p) node[<id>] (name) {};`
    Node,
    /// TikZ-Form mit Größe: `\node[draw, <id>, minimum width=…] {};`
    Shape,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub kind: SymbolKind,
    /// CircuiTikZ-/TikZ-Name
    pub id: &'static str,
    pub category: &'static str,
    pub de: &'static str,
    pub en: &'static str,
}

/// Kategorien: (Schlüssel, Deutsch, Englisch) in Anzeigereihenfolge.
pub const CATEGORIES: &[(&str, &str, &str)] = &[
    ("resistors", "Widerstände", "Resistors"),
    ("capacitors", "Kondensatoren", "Capacitors"),
    ("inductors", "Spulen", "Inductors"),
    ("diodes", "Dioden und Thyristoren", "Diodes and thyristors"),
    ("sources", "Quellen", "Sources"),
    ("meters", "Messgeräte", "Instruments"),
    ("switches", "Schalter und Taster", "Switches and buttons"),
    (
        "protection",
        "Sicherungen, Lampen, Wandler",
        "Fuses, lamps, transducers",
    ),
    (
        "lines",
        "Leitungen und Verbindungen",
        "Wires and connections",
    ),
    (
        "supply",
        "Masse, Versorgung, Antennen",
        "Grounds, supplies, antennas",
    ),
    ("terminals", "Anschlusspunkte", "Terminals"),
    ("transistors", "Transistoren", "Transistors"),
    ("tubes", "Elektronenröhren", "Vacuum tubes"),
    ("amplifiers", "Verstärker", "Amplifiers"),
    ("logic", "Logikgatter", "Logic gates"),
    ("logic-iec", "Logikgatter (IEC)", "Logic gates (IEC)"),
    ("logic-ieee", "Logikgatter (IEEE)", "Logic gates (IEEE)"),
    (
        "digital",
        "Flipflops, Multiplexer, ICs",
        "Flip-flops, multiplexers, ICs",
    ),
    (
        "transformers",
        "Übertrager und Mehrtore",
        "Transformers and multiports",
    ),
    ("blocks", "Blockschaltbild", "Block diagram"),
    ("mechanics", "Mechanik", "Mechanics"),
    ("misc", "Sonstige Bauteile", "Other components"),
    ("shapes", "Formen", "Shapes"),
    (
        "shapes-symbols",
        "Symbole und Sprechblasen",
        "Symbols and callouts",
    ),
    ("shapes-arrows", "Blockpfeile", "Block arrows"),
];

const fn b(
    id: &'static str,
    category: &'static str,
    de: &'static str,
    en: &'static str,
) -> CatalogEntry {
    CatalogEntry {
        kind: SymbolKind::Bipole,
        id,
        category,
        de,
        en,
    }
}

const fn n(
    id: &'static str,
    category: &'static str,
    de: &'static str,
    en: &'static str,
) -> CatalogEntry {
    CatalogEntry {
        kind: SymbolKind::Node,
        id,
        category,
        de,
        en,
    }
}

const fn s(
    id: &'static str,
    category: &'static str,
    de: &'static str,
    en: &'static str,
) -> CatalogEntry {
    CatalogEntry {
        kind: SymbolKind::Shape,
        id,
        category,
        de,
        en,
    }
}

pub const ENTRIES: &[CatalogEntry] = &[
    // ------------------------------------------------------------ Widerstände
    b("R", "resistors", "Widerstand", "Resistor"),
    b(
        "american resistor",
        "resistors",
        "Widerstand (US-Symbol)",
        "Resistor (US symbol)",
    ),
    b(
        "vR",
        "resistors",
        "Einstellbarer Widerstand",
        "Variable resistor",
    ),
    b("pR", "resistors", "Potentiometer", "Potentiometer"),
    b("sR", "resistors", "Widerstandssensor", "Resistive sensor"),
    b("thermistor", "resistors", "Thermistor", "Thermistor"),
    b(
        "thermistor ptc",
        "resistors",
        "Kaltleiter (PTC)",
        "PTC thermistor",
    ),
    b(
        "thermistor ntc",
        "resistors",
        "Heißleiter (NTC)",
        "NTC thermistor",
    ),
    b(
        "photoresistor",
        "resistors",
        "Fotowiderstand (LDR)",
        "Photoresistor (LDR)",
    ),
    b("varistor", "resistors", "Varistor", "Varistor"),
    b("memristor", "resistors", "Memristor", "Memristor"),
    b(
        "generic",
        "resistors",
        "Allgemeine Impedanz",
        "Generic impedance",
    ),
    b(
        "ageneric",
        "resistors",
        "Allgemeine Impedanz (einstellbar)",
        "Generic impedance (adjustable)",
    ),
    b(
        "tgeneric",
        "resistors",
        "Allgemeine Impedanz (Abgriff)",
        "Generic impedance (tunable)",
    ),
    b(
        "xgeneric",
        "resistors",
        "Allgemeine Impedanz (gekreuzt)",
        "Generic impedance (crossed)",
    ),
    b(
        "fullgeneric",
        "resistors",
        "Allgemeine Impedanz (gefüllt)",
        "Generic impedance (filled)",
    ),
    b(
        "tfullgeneric",
        "resistors",
        "Allgemeine Impedanz (gefüllt, Abgriff)",
        "Generic impedance (filled, tunable)",
    ),
    // ------------------------------------------------------------ Kondensatoren
    b("C", "capacitors", "Kondensator", "Capacitor"),
    b(
        "eC",
        "capacitors",
        "Elektrolytkondensator",
        "Electrolytic capacitor",
    ),
    b(
        "pC",
        "capacitors",
        "Gepolter Kondensator",
        "Polarized capacitor",
    ),
    b(
        "cC",
        "capacitors",
        "Kondensator (gebogen)",
        "Curved capacitor",
    ),
    b("vC", "capacitors", "Drehkondensator", "Variable capacitor"),
    b(
        "sC",
        "capacitors",
        "Kapazitiver Sensor",
        "Capacitive sensor",
    ),
    b(
        "piezoelectric",
        "capacitors",
        "Piezoelement",
        "Piezoelectric element",
    ),
    b(
        "ferrocap",
        "capacitors",
        "Ferroelektrischer Kondensator",
        "Ferroelectric capacitor",
    ),
    b(
        "cpe",
        "capacitors",
        "Konstantphasenelement (CPE)",
        "Constant phase element (CPE)",
    ),
    // ------------------------------------------------------------ Spulen
    b("L", "inductors", "Spule", "Inductor"),
    b(
        "american inductor",
        "inductors",
        "Spule (US-Symbol)",
        "Inductor (US symbol)",
    ),
    b(
        "cute inductor",
        "inductors",
        "Spule (Windungen)",
        "Inductor (coils)",
    ),
    b("vL", "inductors", "Einstellbare Spule", "Variable inductor"),
    b("sL", "inductors", "Induktiver Sensor", "Inductive sensor"),
    b("cute choke", "inductors", "Drossel", "Choke"),
    // ------------------------------------------------------------ Dioden
    b("D", "diodes", "Diode", "Diode"),
    b("D*", "diodes", "Diode (gefüllt)", "Diode (filled)"),
    b("sD", "diodes", "Schottky-Diode", "Schottky diode"),
    b("zD", "diodes", "Z-Diode", "Zener diode"),
    b("zzD", "diodes", "Z-Diode (Doppel-Z)", "Zener diode (ZZ)"),
    b("tD", "diodes", "Tunneldiode", "Tunnel diode"),
    b("pD", "diodes", "Fotodiode", "Photodiode"),
    b(
        "leD",
        "diodes",
        "Leuchtdiode (LED)",
        "Light-emitting diode (LED)",
    ),
    b("lasD", "diodes", "Laserdiode", "Laser diode"),
    b("VC", "diodes", "Kapazitätsdiode", "Varactor diode"),
    b(
        "biD",
        "diodes",
        "Bidirektionale Diode (Diac)",
        "Bidirectional diode (diac)",
    ),
    b(
        "GTOb",
        "diodes",
        "GTO-Thyristor (Balken)",
        "GTO thyristor (bar)",
    ),
    b("Ty", "diodes", "Thyristor", "Thyristor"),
    b("Tr", "diodes", "Triac", "Triac"),
    b(
        "PUT",
        "diodes",
        "Programmierbarer Unijunction-Transistor",
        "Programmable unijunction transistor",
    ),
    b("GTO", "diodes", "GTO-Thyristor", "Gate turn-off thyristor"),
    b(
        "agtobar",
        "diodes",
        "GTO-Thyristor (Anodensteuerung)",
        "GTO thyristor (anode gate)",
    ),
    // ------------------------------------------------------------ Quellen
    b("V", "sources", "Spannungsquelle", "Voltage source"),
    b(
        "american voltage source",
        "sources",
        "Spannungsquelle (US-Symbol)",
        "Voltage source (US symbol)",
    ),
    b(
        "cute european voltage source",
        "sources",
        "Spannungsquelle (Kreis)",
        "Voltage source (circle)",
    ),
    b("I", "sources", "Stromquelle", "Current source"),
    b(
        "american current source",
        "sources",
        "Stromquelle (US-Symbol)",
        "Current source (US symbol)",
    ),
    b(
        "cute european current source",
        "sources",
        "Stromquelle (Kreis)",
        "Current source (circle)",
    ),
    b(
        "cV",
        "sources",
        "Gesteuerte Spannungsquelle",
        "Controlled voltage source",
    ),
    b(
        "american controlled voltage source",
        "sources",
        "Gesteuerte Spannungsquelle (US-Symbol)",
        "Controlled voltage source (US symbol)",
    ),
    b(
        "cI",
        "sources",
        "Gesteuerte Stromquelle",
        "Controlled current source",
    ),
    b(
        "american controlled current source",
        "sources",
        "Gesteuerte Stromquelle (US-Symbol)",
        "Controlled current source (US symbol)",
    ),
    b(
        "empty controlled source",
        "sources",
        "Gesteuerte Quelle (leer)",
        "Controlled source (empty)",
    ),
    b(
        "sV",
        "sources",
        "Sinusspannungsquelle",
        "Sinusoidal voltage source",
    ),
    b(
        "sI",
        "sources",
        "Sinusstromquelle",
        "Sinusoidal current source",
    ),
    b(
        "controlled sinusoidal voltage source",
        "sources",
        "Gesteuerte Sinusspannungsquelle",
        "Controlled sinusoidal voltage source",
    ),
    b(
        "controlled sinusoidal current source",
        "sources",
        "Gesteuerte Sinusstromquelle",
        "Controlled sinusoidal current source",
    ),
    b(
        "dcvsource",
        "sources",
        "Gleichspannungsquelle",
        "DC voltage source",
    ),
    b(
        "dcisource",
        "sources",
        "Gleichstromquelle",
        "DC current source",
    ),
    b(
        "square voltage source",
        "sources",
        "Rechteckspannungsquelle",
        "Square-wave voltage source",
    ),
    b(
        "triangle voltage source",
        "sources",
        "Dreieckspannungsquelle",
        "Triangle-wave voltage source",
    ),
    b(
        "noise voltage source",
        "sources",
        "Rauschspannungsquelle",
        "Noise voltage source",
    ),
    b(
        "noise current source",
        "sources",
        "Rauschstromquelle",
        "Noise current source",
    ),
    b("battery", "sources", "Batterie", "Battery"),
    b(
        "battery1",
        "sources",
        "Batterie (eine Zelle)",
        "Battery (single cell)",
    ),
    b(
        "battery2",
        "sources",
        "Batterie (zwei Zellen)",
        "Battery (two cells)",
    ),
    b("pvsource", "sources", "Solarzelle", "Photovoltaic cell"),
    b("pvmodule", "sources", "Solarmodul", "Photovoltaic module"),
    b("thermocouple", "sources", "Thermoelement", "Thermocouple"),
    b(
        "esource",
        "sources",
        "Quelle (leerer Kreis)",
        "Source (empty circle)",
    ),
    b(
        "ioosource",
        "sources",
        "Stromquelle (Doppelkreis)",
        "Current source (double circle)",
    ),
    b(
        "voosource",
        "sources",
        "Spannungsquelle (Doppelkreis)",
        "Voltage source (double circle)",
    ),
    b(
        "ooosource",
        "sources",
        "Quelle (Doppelkreis)",
        "Source (double circle)",
    ),
    b(
        "oosourcetrans",
        "sources",
        "Quelle (Doppelkreis, Übertrager)",
        "Source (double circle, transformer)",
    ),
    b("iloop", "sources", "Stromschleife", "Current loop"),
    b(
        "iloop2",
        "sources",
        "Stromschleife (Variante)",
        "Current loop (variant)",
    ),
    // ------------------------------------------------------------ Messgeräte
    b("ammeter", "meters", "Strommesser", "Ammeter"),
    b("voltmeter", "meters", "Spannungsmesser", "Voltmeter"),
    b("ohmmeter", "meters", "Widerstandsmesser", "Ohmmeter"),
    b("rmeter", "meters", "Messgerät (rund)", "Meter (round)"),
    b(
        "rmeterwa",
        "meters",
        "Messgerät (rund, Pfeil)",
        "Meter (round, arrow)",
    ),
    b("smeter", "meters", "Messgerät (eckig)", "Meter (square)"),
    b("qvprobe", "meters", "Spannungsmessung", "Voltage probe"),
    b("qiprobe", "meters", "Strommessung", "Current probe"),
    b("qpprobe", "meters", "Leistungsmessung", "Power probe"),
    b("oscope", "meters", "Oszilloskop", "Oscilloscope"),
    // ------------------------------------------------------------ Schalter
    b("nos", "switches", "Schließer", "Normally open switch"),
    b("ncs", "switches", "Öffner", "Normally closed switch"),
    b(
        "closing switch",
        "switches",
        "Schließender Schalter",
        "Closing switch",
    ),
    b(
        "opening switch",
        "switches",
        "Öffnender Schalter",
        "Opening switch",
    ),
    b(
        "push button",
        "switches",
        "Taster (Schließer)",
        "Push button (normally open)",
    ),
    b(
        "ncpb",
        "switches",
        "Taster (Öffner)",
        "Push button (normally closed)",
    ),
    b(
        "nopbc",
        "switches",
        "Taster (Schließer, Variante)",
        "Push button (normally open, variant)",
    ),
    b(
        "ncpbo",
        "switches",
        "Taster (Öffner, Variante)",
        "Push button (normally closed, variant)",
    ),
    b("toggle switch", "switches", "Kippschalter", "Toggle switch"),
    b("reed", "switches", "Reedkontakt", "Reed switch"),
    b(
        "cute open switch",
        "switches",
        "Schalter offen (Kontakte)",
        "Open switch (contacts)",
    ),
    b(
        "cute closed switch",
        "switches",
        "Schalter geschlossen (Kontakte)",
        "Closed switch (contacts)",
    ),
    b(
        "cute opening switch",
        "switches",
        "Öffnender Schalter (Kontakte)",
        "Opening switch (contacts)",
    ),
    b(
        "cute closing switch",
        "switches",
        "Schließender Schalter (Kontakte)",
        "Closing switch (contacts)",
    ),
    n("spdt", "switches", "Umschalter", "Changeover switch (SPDT)"),
    n(
        "cute spdt up",
        "switches",
        "Umschalter oben (Kontakte)",
        "Changeover switch up (contacts)",
    ),
    n(
        "cute spdt mid",
        "switches",
        "Umschalter Mitte (Kontakte)",
        "Changeover switch middle (contacts)",
    ),
    n(
        "cute spdt down",
        "switches",
        "Umschalter unten (Kontakte)",
        "Changeover switch down (contacts)",
    ),
    n(
        "cute spdt up arrow",
        "switches",
        "Umschalter oben mit Pfeil",
        "Changeover switch up with arrow",
    ),
    n(
        "cute spdt mid arrow",
        "switches",
        "Umschalter Mitte mit Pfeil",
        "Changeover switch middle with arrow",
    ),
    n(
        "cute spdt down arrow",
        "switches",
        "Umschalter unten mit Pfeil",
        "Changeover switch down with arrow",
    ),
    // ------------------------------------------------------------ Sicherungen, Lampen, Wandler
    b("fuse", "protection", "Sicherung", "Fuse"),
    b(
        "asymmetric fuse",
        "protection",
        "Sicherung (asymmetrisch)",
        "Asymmetric fuse",
    ),
    b(
        "european gas filled surge arrester",
        "protection",
        "Überspannungsableiter",
        "Gas-filled surge arrester",
    ),
    b(
        "american gas filled surge arrester",
        "protection",
        "Überspannungsableiter (US-Symbol)",
        "Gas-filled surge arrester (US symbol)",
    ),
    b("lamp", "protection", "Lampe", "Lamp"),
    b("bulb", "protection", "Glühlampe", "Light bulb"),
    b("loudspeaker", "protection", "Lautsprecher", "Loudspeaker"),
    b("mic", "protection", "Mikrofon", "Microphone"),
    b("squid", "protection", "SQUID", "SQUID"),
    b("barrier", "protection", "Barriere", "Barrier"),
    b(
        "openbarrier",
        "protection",
        "Offene Barriere",
        "Open barrier",
    ),
    n(
        "elmech",
        "protection",
        "Elektromotor (Wandler)",
        "Electromechanical device (motor)",
    ),
    // ------------------------------------------------------------ Leitungen
    b("short", "lines", "Leitung", "Wire"),
    b("open", "lines", "Offene Klemmen", "Open circuit"),
    b(
        "crossing",
        "lines",
        "Leitungskreuzung (Sprung)",
        "Wire crossing (jump)",
    ),
    b("tline", "lines", "Übertragungsleitung", "Transmission line"),
    b(
        "mstline",
        "lines",
        "Mikrostreifenleitung",
        "Microstrip line",
    ),
    b("multiwire", "lines", "Mehrfachleitung", "Multiwire"),
    b(
        "bmultiwire",
        "lines",
        "Mehrfachleitung (Bus)",
        "Multiwire (bus)",
    ),
    b(
        "tmultiwire",
        "lines",
        "Mehrfachleitung (Strich)",
        "Multiwire (tick)",
    ),
    n(
        "jump crossing",
        "lines",
        "Kreuzung mit Sprung",
        "Jump crossing",
    ),
    n(
        "plain crossing",
        "lines",
        "Kreuzung ohne Verbindung",
        "Plain crossing",
    ),
    // ------------------------------------------------------------ Masse und Versorgung
    n("ground", "supply", "Masse", "Ground"),
    n(
        "tlground",
        "supply",
        "Masse (dicke Linie)",
        "Ground (thick line)",
    ),
    n("rground", "supply", "Bezugsmasse", "Reference ground"),
    n("sground", "supply", "Signalmasse", "Signal ground"),
    n("nground", "supply", "Rauscharme Masse", "Noiseless ground"),
    n("pground", "supply", "Schutzerde", "Protective ground"),
    n("cground", "supply", "Gehäusemasse", "Chassis ground"),
    n("eground", "supply", "Erde", "Earth ground"),
    n(
        "eground2",
        "supply",
        "Erde (Variante)",
        "Earth ground (variant)",
    ),
    n("vcc", "supply", "Versorgung (+)", "Supply (Vcc)"),
    n("vee", "supply", "Versorgung (−)", "Supply (Vee)"),
    n("antenna", "supply", "Antenne", "Antenna"),
    n(
        "rxantenna",
        "supply",
        "Empfangsantenne",
        "Receiving antenna",
    ),
    n(
        "txantenna",
        "supply",
        "Sendeantenne",
        "Transmitting antenna",
    ),
    n(
        "bareantenna",
        "supply",
        "Antenne (einfach)",
        "Antenna (bare)",
    ),
    n(
        "bareRXantenna",
        "supply",
        "Empfangsantenne (einfach)",
        "Receiving antenna (bare)",
    ),
    n(
        "bareTXantenna",
        "supply",
        "Sendeantenne (einfach)",
        "Transmitting antenna (bare)",
    ),
    // ------------------------------------------------------------ Anschlusspunkte
    n("circ", "terminals", "Verbindungspunkt", "Connection dot"),
    n("ocirc", "terminals", "Anschluss (offen)", "Open terminal"),
    n(
        "diamondpole",
        "terminals",
        "Anschluss (Raute)",
        "Diamond terminal",
    ),
    n(
        "squarepole",
        "terminals",
        "Anschluss (Quadrat)",
        "Square terminal",
    ),
    // ------------------------------------------------------------ Transistoren
    n("npn", "transistors", "NPN-Transistor", "NPN transistor"),
    n("pnp", "transistors", "PNP-Transistor", "PNP transistor"),
    n("nmos", "transistors", "N-MOSFET", "N-channel MOSFET"),
    n("pmos", "transistors", "P-MOSFET", "P-channel MOSFET"),
    n(
        "nmosd",
        "transistors",
        "N-MOSFET (Verarmung)",
        "N-channel MOSFET (depletion)",
    ),
    n(
        "pmosd",
        "transistors",
        "P-MOSFET (Verarmung)",
        "P-channel MOSFET (depletion)",
    ),
    n("nfet", "transistors", "N-FET", "N-channel FET"),
    n("pfet", "transistors", "P-FET", "P-channel FET"),
    n(
        "nfetd",
        "transistors",
        "N-FET (Verarmung)",
        "N-channel FET (depletion)",
    ),
    n(
        "pfetd",
        "transistors",
        "P-FET (Verarmung)",
        "P-channel FET (depletion)",
    ),
    n(
        "nigfete",
        "transistors",
        "N-IGFET (Anreicherung)",
        "N-channel IGFET (enhancement)",
    ),
    n(
        "nigfetd",
        "transistors",
        "N-IGFET (Verarmung)",
        "N-channel IGFET (depletion)",
    ),
    n(
        "pigfete",
        "transistors",
        "P-IGFET (Anreicherung)",
        "P-channel IGFET (enhancement)",
    ),
    n(
        "pigfetd",
        "transistors",
        "P-IGFET (Verarmung)",
        "P-channel IGFET (depletion)",
    ),
    n(
        "nigfetebulk",
        "transistors",
        "N-IGFET mit Bulk",
        "N-channel IGFET with bulk",
    ),
    n(
        "pigfetebulk",
        "transistors",
        "P-IGFET mit Bulk",
        "P-channel IGFET with bulk",
    ),
    n("nigbt", "transistors", "N-IGBT", "N-channel IGBT"),
    n("pigbt", "transistors", "P-IGBT", "P-channel IGBT"),
    n(
        "Lnigbt",
        "transistors",
        "N-IGBT (L-Form)",
        "N-channel IGBT (L shape)",
    ),
    n(
        "Lpigbt",
        "transistors",
        "P-IGBT (L-Form)",
        "P-channel IGBT (L shape)",
    ),
    n("hemt", "transistors", "HEMT", "HEMT"),
    n(
        "njfet",
        "transistors",
        "N-Sperrschicht-FET",
        "N-channel JFET",
    ),
    n(
        "pjfet",
        "transistors",
        "P-Sperrschicht-FET",
        "P-channel JFET",
    ),
    n(
        "nujt",
        "transistors",
        "Unijunction-Transistor (N)",
        "Unijunction transistor (N)",
    ),
    n(
        "pujt",
        "transistors",
        "Unijunction-Transistor (P)",
        "Unijunction transistor (P)",
    ),
    n("isfet", "transistors", "ISFET", "ISFET"),
    // ------------------------------------------------------------ Röhren
    n("diodetube", "tubes", "Röhrendiode", "Vacuum diode"),
    n("triode", "tubes", "Triode", "Triode"),
    n("tetrode", "tubes", "Tetrode", "Tetrode"),
    n("pentode", "tubes", "Pentode", "Pentode"),
    // ------------------------------------------------------------ Verstärker
    n(
        "op amp",
        "amplifiers",
        "Operationsverstärker",
        "Operational amplifier",
    ),
    n(
        "fd op amp",
        "amplifiers",
        "Volldifferenzieller OPV",
        "Fully differential op amp",
    ),
    n(
        "en amp",
        "amplifiers",
        "Verstärker (IEC)",
        "Amplifier (IEC)",
    ),
    n(
        "gm amp",
        "amplifiers",
        "Transkonduktanzverstärker",
        "Transconductance amplifier",
    ),
    n(
        "inst amp",
        "amplifiers",
        "Instrumentenverstärker",
        "Instrumentation amplifier",
    ),
    n(
        "fd inst amp",
        "amplifiers",
        "Volldifferenzieller Instrumentenverstärker",
        "Fully differential instrumentation amplifier",
    ),
    n(
        "plain amp",
        "amplifiers",
        "Verstärker (Dreieck)",
        "Amplifier (triangle)",
    ),
    n(
        "plain mono amp",
        "amplifiers",
        "Verstärker (ein Eingang)",
        "Amplifier (single input)",
    ),
    n("buffer", "amplifiers", "Puffer", "Buffer"),
    // ------------------------------------------------------------ Logik (US)
    n("and port", "logic", "UND-Gatter", "AND gate"),
    n("or port", "logic", "ODER-Gatter", "OR gate"),
    n("not port", "logic", "NICHT-Gatter", "NOT gate"),
    n("nand port", "logic", "NAND-Gatter", "NAND gate"),
    n("nor port", "logic", "NOR-Gatter", "NOR gate"),
    n("xor port", "logic", "XOR-Gatter", "XOR gate"),
    n("xnor port", "logic", "XNOR-Gatter", "XNOR gate"),
    n("buffer port", "logic", "Treiber", "Buffer gate"),
    // ------------------------------------------------------------ Logik (IEC)
    n(
        "european and port",
        "logic-iec",
        "UND-Gatter (IEC)",
        "AND gate (IEC)",
    ),
    n(
        "european or port",
        "logic-iec",
        "ODER-Gatter (IEC)",
        "OR gate (IEC)",
    ),
    n(
        "european not port",
        "logic-iec",
        "NICHT-Gatter (IEC)",
        "NOT gate (IEC)",
    ),
    n(
        "european nand port",
        "logic-iec",
        "NAND-Gatter (IEC)",
        "NAND gate (IEC)",
    ),
    n(
        "european nor port",
        "logic-iec",
        "NOR-Gatter (IEC)",
        "NOR gate (IEC)",
    ),
    n(
        "european xor port",
        "logic-iec",
        "XOR-Gatter (IEC)",
        "XOR gate (IEC)",
    ),
    n(
        "european xnor port",
        "logic-iec",
        "XNOR-Gatter (IEC)",
        "XNOR gate (IEC)",
    ),
    n(
        "european buffer port",
        "logic-iec",
        "Treiber (IEC)",
        "Buffer gate (IEC)",
    ),
    // ------------------------------------------------------------ Logik (IEEE)
    n(
        "ieeestd and port",
        "logic-ieee",
        "UND-Gatter (IEEE)",
        "AND gate (IEEE)",
    ),
    n(
        "ieeestd or port",
        "logic-ieee",
        "ODER-Gatter (IEEE)",
        "OR gate (IEEE)",
    ),
    n(
        "ieeestd not port",
        "logic-ieee",
        "NICHT-Gatter (IEEE)",
        "NOT gate (IEEE)",
    ),
    n(
        "ieeestd nand port",
        "logic-ieee",
        "NAND-Gatter (IEEE)",
        "NAND gate (IEEE)",
    ),
    n(
        "ieeestd nor port",
        "logic-ieee",
        "NOR-Gatter (IEEE)",
        "NOR gate (IEEE)",
    ),
    n(
        "ieeestd xor port",
        "logic-ieee",
        "XOR-Gatter (IEEE)",
        "XOR gate (IEEE)",
    ),
    n(
        "ieeestd xnor port",
        "logic-ieee",
        "XNOR-Gatter (IEEE)",
        "XNOR gate (IEEE)",
    ),
    n(
        "ieeestd buffer port",
        "logic-ieee",
        "Treiber (IEEE)",
        "Buffer gate (IEEE)",
    ),
    n(
        "ieeestd schmitt port",
        "logic-ieee",
        "Schmitt-Trigger",
        "Schmitt trigger",
    ),
    n(
        "ieeestd invschmitt port",
        "logic-ieee",
        "Schmitt-Trigger (invertierend)",
        "Inverting Schmitt trigger",
    ),
    n(
        "tgate",
        "logic-ieee",
        "Transmissionsgatter",
        "Transmission gate",
    ),
    n(
        "double tgate",
        "logic-ieee",
        "Transmissionsgatter (doppelt)",
        "Double transmission gate",
    ),
    // ------------------------------------------------------------ Digital
    n("flipflop D", "digital", "D-Flipflop", "D flip-flop"),
    n("flipflop T", "digital", "T-Flipflop", "T flip-flop"),
    n("flipflop JK", "digital", "JK-Flipflop", "JK flip-flop"),
    n("flipflop SR", "digital", "RS-Flipflop", "SR flip-flop"),
    n(
        "muxdemux",
        "digital",
        "Multiplexer/Demultiplexer",
        "Multiplexer/demultiplexer",
    ),
    n("dipchip", "digital", "IC (DIP-Gehäuse)", "IC (DIP package)"),
    n("qfpchip", "digital", "IC (QFP-Gehäuse)", "IC (QFP package)"),
    // ------------------------------------------------------------ Übertrager, Mehrtore
    n(
        "transformer",
        "transformers",
        "Transformator",
        "Transformer",
    ),
    n(
        "transformer core",
        "transformers",
        "Transformator mit Kern",
        "Transformer with core",
    ),
    n("gyrator", "transformers", "Gyrator", "Gyrator"),
    n("fourport", "transformers", "Viertor", "Four-port"),
    n("coupler", "transformers", "Koppler", "Coupler"),
    n(
        "coupler2",
        "transformers",
        "Koppler (Variante)",
        "Coupler (variant)",
    ),
    // ------------------------------------------------------------ Blockschaltbild
    b("amp", "blocks", "Verstärker (Block)", "Amplifier (block)"),
    b(
        "vamp",
        "blocks",
        "Einstellbarer Verstärker",
        "Variable amplifier",
    ),
    b("twoport", "blocks", "Zweitor", "Two-port"),
    b(
        "twoportsplit",
        "blocks",
        "Zweitor (geteilt)",
        "Two-port (split)",
    ),
    b("phaseshifter", "blocks", "Phasenschieber", "Phase shifter"),
    b(
        "vphaseshifter",
        "blocks",
        "Einstellbarer Phasenschieber",
        "Variable phase shifter",
    ),
    b("detector", "blocks", "Detektor", "Detector"),
    b(
        "vco",
        "blocks",
        "VCO (spannungsgesteuerter Oszillator)",
        "Voltage-controlled oscillator",
    ),
    b("lowpass", "blocks", "Tiefpass", "Low-pass filter"),
    b(
        "lowpass2",
        "blocks",
        "Tiefpass (Variante)",
        "Low-pass filter (variant)",
    ),
    b("highpass", "blocks", "Hochpass", "High-pass filter"),
    b(
        "highpass2",
        "blocks",
        "Hochpass (Variante)",
        "High-pass filter (variant)",
    ),
    b("bandpass", "blocks", "Bandpass", "Band-pass filter"),
    b("bandstop", "blocks", "Bandsperre", "Band-stop filter"),
    b("allpass", "blocks", "Allpass", "All-pass filter"),
    b(
        "adc",
        "blocks",
        "A/D-Wandler",
        "Analog-to-digital converter",
    ),
    b(
        "dac",
        "blocks",
        "D/A-Wandler",
        "Digital-to-analog converter",
    ),
    b(
        "dsp",
        "blocks",
        "Signalprozessor (DSP)",
        "Digital signal processor",
    ),
    b("fft", "blocks", "FFT", "FFT"),
    b(
        "sdcdc",
        "blocks",
        "Gleichspannungswandler",
        "DC/DC converter",
    ),
    b("sdcac", "blocks", "Wechselrichter", "DC/AC converter"),
    b("sacdc", "blocks", "Gleichrichter", "AC/DC converter"),
    b(
        "tdcac",
        "blocks",
        "Wechselrichter (Variante)",
        "DC/AC converter (variant)",
    ),
    b(
        "tacdc",
        "blocks",
        "Gleichrichter (Variante)",
        "AC/DC converter (variant)",
    ),
    b(
        "piattenuator",
        "blocks",
        "Pi-Dämpfungsglied",
        "Pi attenuator",
    ),
    b(
        "vpiattenuator",
        "blocks",
        "Einstellbares Pi-Dämpfungsglied",
        "Variable pi attenuator",
    ),
    b("tattenuator", "blocks", "T-Dämpfungsglied", "T attenuator"),
    b(
        "vtattenuator",
        "blocks",
        "Einstellbares T-Dämpfungsglied",
        "Variable T attenuator",
    ),
    n("mixer", "blocks", "Mischer", "Mixer"),
    n("adder", "blocks", "Addierer", "Adder"),
    n("oscillator", "blocks", "Oszillator", "Oscillator"),
    n("circulator", "blocks", "Zirkulator", "Circulator"),
    n(
        "wilkinson",
        "blocks",
        "Wilkinson-Teiler",
        "Wilkinson divider",
    ),
    // ------------------------------------------------------------ Mechanik
    b("mass", "mechanics", "Masse (Mechanik)", "Mass"),
    b("spring", "mechanics", "Feder", "Spring"),
    b("damper", "mechanics", "Dämpfer", "Damper"),
    b("inerter", "mechanics", "Inerter", "Inerter"),
    b(
        "viscoe",
        "mechanics",
        "Viskoelastisches Element",
        "Viscoelastic element",
    ),
    // ------------------------------------------------------------ TikZ-Formen
    s("rectangle", "shapes", "Rechteck", "Rectangle"),
    s(
        "rounded rectangle",
        "shapes",
        "Abgerundetes Rechteck",
        "Rounded rectangle",
    ),
    s(
        "chamfered rectangle",
        "shapes",
        "Abgeschrägtes Rechteck",
        "Chamfered rectangle",
    ),
    s("circle", "shapes", "Kreis", "Circle"),
    s("ellipse", "shapes", "Ellipse", "Ellipse"),
    s("diamond", "shapes", "Raute", "Diamond"),
    s("trapezium", "shapes", "Trapez", "Trapezium"),
    s("semicircle", "shapes", "Halbkreis", "Semicircle"),
    s(
        "isosceles triangle",
        "shapes",
        "Gleichschenkliges Dreieck",
        "Isosceles triangle",
    ),
    s(
        "regular polygon",
        "shapes",
        "Regelmäßiges Vieleck",
        "Regular polygon",
    ),
    s("star", "shapes", "Stern", "Star"),
    s("kite", "shapes", "Drachenviereck", "Kite"),
    s("dart", "shapes", "Pfeilspitze", "Dart"),
    s(
        "circular sector",
        "shapes",
        "Kreissektor",
        "Circular sector",
    ),
    s("cylinder", "shapes", "Zylinder", "Cylinder"),
    s("cross out", "shapes", "Durchgekreuzt", "Cross out"),
    s("strike out", "shapes", "Durchgestrichen", "Strike out"),
    s("cloud", "shapes-symbols", "Wolke", "Cloud"),
    s("starburst", "shapes-symbols", "Explosion", "Starburst"),
    s("signal", "shapes-symbols", "Signal", "Signal"),
    s("tape", "shapes-symbols", "Band", "Tape"),
    s(
        "magnetic tape",
        "shapes-symbols",
        "Magnetband",
        "Magnetic tape",
    ),
    s(
        "rectangle callout",
        "shapes-symbols",
        "Sprechblase (eckig)",
        "Rectangle callout",
    ),
    s(
        "ellipse callout",
        "shapes-symbols",
        "Sprechblase (rund)",
        "Ellipse callout",
    ),
    s(
        "cloud callout",
        "shapes-symbols",
        "Denkblase",
        "Cloud callout",
    ),
    s(
        "single arrow",
        "shapes-arrows",
        "Blockpfeil",
        "Single arrow",
    ),
    s(
        "double arrow",
        "shapes-arrows",
        "Doppelpfeil",
        "Double arrow",
    ),
    s("arrow box", "shapes-arrows", "Pfeilkasten", "Arrow box"),
];

/// TikZ-Bibliotheken, die die Formen benötigen (Präambel und Vorschau).
pub const TIKZ_LIBRARIES: &str =
    "arrows.meta,positioning,calc,shapes.geometric,shapes.symbols,shapes.arrows,shapes.misc,shapes.callouts";

/// Mögliche Anschlussnamen von Knoten (der Generator prüft, welche es gibt).
pub const ANCHOR_CANDIDATES: &[&str] = &[
    "G",
    "D",
    "S",
    "B",
    "C",
    "E",
    "bulk",
    "nobulk",
    "gate",
    "drain",
    "source",
    "base",
    "collector",
    "emitter",
    "anode",
    "cathode",
    "control",
    "grid",
    "screen",
    "suppressor",
    "filament 1",
    "filament 2",
    "+",
    "-",
    "out",
    "out +",
    "out -",
    "up",
    "down",
    "refv up",
    "refv down",
    "in",
    "in 1",
    "in 2",
    "in 3",
    "in 4",
    "in 5",
    "in 6",
    "in 7",
    "in 8",
    "bin 1",
    "bin 2",
    "out 1",
    "out 2",
    "out 3",
    "A1",
    "A2",
    "B1",
    "B2",
    "1",
    "2",
    "3",
    "4",
    "pin 1",
    "pin 2",
    "pin 3",
    "pin 4",
    "pin 5",
    "pin 6",
    "pin 7",
    "pin 8",
    "pin 9",
    "pin 10",
    "pin 11",
    "pin 12",
    "pin 13",
    "pin 14",
    "pin 15",
    "pin 16",
    "bpin 1",
    "bpin 2",
    "lpin 1",
    "lpin 2",
    "lpin 3",
    "lpin 4",
    "rpin 1",
    "rpin 2",
    "rpin 3",
    "rpin 4",
    "tpin 1",
    "bpin 3",
    "left",
    "right",
    "wiper",
    "mid",
    "center",
];

/// Seitengröße der Symbolseiten (cm), Ursprung in der Mitte.
pub const PAGE_CM: f64 = 8.0;

/// Dokument mit einer Seite je Eintrag; liefert zusätzlich die Startzeile jedes Eintrags.
pub fn symbols_document(entries: &[&CatalogEntry]) -> (String, Vec<usize>) {
    let mut lines: Vec<String> = vec![
        "\\documentclass{article}".into(),
        format!("\\usepackage[paperwidth={PAGE_CM}cm,paperheight={PAGE_CM}cm,margin=0pt]{{geometry}}"),
        "\\usepackage{tikz}".into(),
        format!("\\usetikzlibrary{{{TIKZ_LIBRARIES}}}"),
        "\\usepackage[european]{circuitikz}".into(),
        "\\pagestyle{empty}".into(),
        "\\setlength{\\parindent}{0pt}\\setlength{\\topskip}{0pt}".into(),
        "\\makeatletter".into(),
        "\\def\\vtxanchor#1#2#3{\\expandafter\\ifx\\csname pgf@anchor@\\csname pgf@sh@ns@#2\\endcsname @#3\\endcsname\\relax\\else\\pgfpointanchor{#2}{#3}\\pgfgetlastxy\\vtx@x\\vtx@y\\typeout{VA|#1|#3|\\vtx@x|\\vtx@y}\\fi}".into(),
        "\\makeatother".into(),
        "\\begin{document}".into(),
    ];
    let mut starts = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        starts.push(lines.len() + 1);
        lines.push(format!("% {}", entry.id));
        lines.push(format!(
            "\\begin{{tikzpicture}}\\useasboundingbox (-{h},-{h}) rectangle ({h},{h});",
            h = PAGE_CM / 2.0
        ));
        match entry.kind {
            // „open“ zeichnet selbst nichts – für das Vorschaubild die offenen Klemmen zeigen.
            SymbolKind::Bipole if entry.id == "open" => {
                lines.push("\\draw (-1,0) to[open, o-o] (1,0);".into())
            }
            SymbolKind::Bipole => lines.push(format!("\\draw (-1,0) to[{}] (1,0);", entry.id)),
            SymbolKind::Node => {
                lines.push(format!("\\draw (0,0) node[{}] (vtx) {{}};", entry.id));
                for anchor in ANCHOR_CANDIDATES {
                    lines.push(format!("\\vtxanchor{{{index}}}{{vtx}}{{{anchor}}}"));
                }
            }
            SymbolKind::Shape => lines.push(format!(
                "\\node[draw, thick, {}, minimum width=2cm, minimum height=1.2cm] at (0,0) {{}};",
                entry.id
            )),
        }
        lines.push("\\end{tikzpicture}\\newpage".into());
    }
    lines.push("\\end{document}".into());
    (lines.join("\n"), starts)
}

pub fn entry(id: &str) -> Option<&'static CatalogEntry> {
    ENTRIES.iter().find(|entry| entry.id == id)
}

pub fn category_name(key: &str, english: bool) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(id, _, _)| *id == key)
        .map(|(_, de, en)| if english { *en } else { *de })
        .unwrap_or("")
}

// ---------------------------------------------------------------- Katalog für die Oberfläche

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCategory {
    pub key: &'static str,
    pub de: &'static str,
    pub en: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSymbol {
    pub id: &'static str,
    pub kind: SymbolKind,
    pub category: &'static str,
    pub de: &'static str,
    pub en: &'static str,
    /// Vorschaubild als data:-URL (schwarz mit Transparenz)
    pub image: String,
    /// Ursprung (Bauteilmitte bzw. Knotenposition) im Bild in Pixeln
    pub ox: f64,
    pub oy: f64,
    pub w: f64,
    pub h: f64,
    /// Anschlüsse: Name, x, y in cm relativ zum Ursprung (y nach oben)
    pub anchors: Vec<(String, f64, f64)>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub px_per_cm: f64,
    pub categories: Vec<CatalogCategory>,
    pub symbols: Vec<CatalogSymbol>,
    pub arrow_tips: &'static [&'static str],
    pub line_widths: &'static [&'static str],
    pub dash_patterns: &'static [&'static str],
    pub font_sizes: &'static [&'static str],
}

/// Katalog aus den erzeugten Symbolen (`resources/sketch-symbols.json`); nur
/// Einträge mit Symbolbild, in Katalogreihenfolge.
pub fn build_catalog(symbols_json: &str) -> Result<Catalog, String> {
    let value: serde_json::Value = serde_json::from_str(symbols_json)
        .map_err(|error| format!("Symboldatei ist beschädigt: {error}"))?;
    let px_per_cm = value["pxPerCm"].as_f64().unwrap_or(85.0);
    let images = value["symbols"]
        .as_object()
        .ok_or("Symboldatei enthält keine Symbole.")?;
    let symbols = ENTRIES
        .iter()
        .filter_map(|entry| {
            let image = images.get(entry.id)?;
            Some(CatalogSymbol {
                id: entry.id,
                kind: entry.kind,
                category: entry.category,
                de: entry.de,
                en: entry.en,
                image: format!("data:image/png;base64,{}", image["png"].as_str()?),
                ox: image["ox"].as_f64()?,
                oy: image["oy"].as_f64()?,
                w: image["w"].as_f64()?,
                h: image["h"].as_f64()?,
                anchors: image["anchors"]
                    .as_array()
                    .map(|anchors| {
                        anchors
                            .iter()
                            .filter_map(|anchor| {
                                Some((
                                    anchor[0].as_str()?.to_string(),
                                    anchor[1].as_f64()?,
                                    anchor[2].as_f64()?,
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect();
    Ok(Catalog {
        px_per_cm,
        categories: CATEGORIES
            .iter()
            .map(|(key, de, en)| CatalogCategory { key, de, en })
            .collect(),
        symbols,
        arrow_tips: super::sketch::ARROW_TIPS,
        line_widths: super::sketch::LINE_WIDTHS,
        dash_patterns: super::sketch::DASH_PATTERNS,
        font_sizes: super::sketch::FONT_SIZES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_is_consistent() {
        let mut ids = BTreeSet::new();
        for entry in ENTRIES {
            assert!(ids.insert(entry.id), "doppelt: {}", entry.id);
            assert!(
                CATEGORIES.iter().any(|(key, _, _)| *key == entry.category),
                "unbekannte Kategorie: {}",
                entry.category
            );
            assert!(!entry.de.is_empty() && !entry.en.is_empty());
            // Namen landen unverändert in TikZ-Optionen.
            assert!(
                !entry.id.contains([',', '=', '{', '}', '[', ']']),
                "{}",
                entry.id
            );
        }
        assert!(ENTRIES.len() > 250);
    }
}
