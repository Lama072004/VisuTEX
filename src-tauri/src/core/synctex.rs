//! SyncTeX: Zuordnung PDF-Position ↔ Quelltextzeile. Ein Doppelklick in der PDF-Vorschau springt
//! an die Stelle im Dokument, „Im PDF zeigen“ in die Gegenrichtung.
//!
//! Gelesen wird die Textform der `.synctex`-Datei, die Tectonic beim Kompilieren erzeugt (das
//! Entpacken von `.synctex.gz` übernimmt der Aufrufer). Das Hauptdokument ist immer die erste
//! Eingabe (Tag 1; Tectonic nennt sie `texput`, weitere Eingaben bleiben namenlos) – zugeordnet
//! werden deshalb Zeilen des Hauptdokuments.
//!
//! Koordinaten stehen in der Datei in TeX-„scaled points“ (65536 sp = 1 pt) ab der linken oberen
//! Seitenecke, multipliziert mit `Unit`; hier werden sie in PDF-Punkte (bp, 72 je Zoll)
//! umgerechnet – dieselben Einheiten wie die Seitengrößen der PDF-Vorschau. TeX verbucht die
//! Zeilen eines Absatzes oft auf der Zeile, die ihn beendet (Leerzeile danach).

use std::collections::HashMap;

/// sp → bp
const SP_TO_BP: f64 = 72.0 / 72.27 / 65536.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Record {
    tag: u32,
    line: u32,
    /// `(` hbox, `[` vbox, `h`/`v` leere Box, `x` Punkt, `k` Kern, `g` Leim, `$` Formel
    kind: u8,
    x: f32,
    /// Grundlinie
    y: f32,
    width: f32,
    height: f32,
    depth: f32,
}

impl Record {
    /// Rechteck (links, oben, rechts, unten) in bp.
    fn rect(&self) -> (f32, f32, f32, f32) {
        match self.kind {
            // flache Boxen (Absatzeinzug, Leerzeilen) wie Punkte behandeln
            b'(' | b'[' | b'h' | b'v' if self.height + self.depth >= 1.0 => (
                self.x,
                self.y - self.height,
                self.x + self.width.max(0.5),
                self.y + self.depth,
            ),
            // Punkte: ungefähr eine Textzeile hoch
            _ => (
                self.x,
                self.y - 8.0,
                self.x + self.width.max(1.0),
                self.y + 2.0,
            ),
        }
    }
}

/// Stelle im PDF (Seite ab 0, Koordinaten in bp ab der linken oberen Ecke).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PdfPoint {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct SyncTex {
    inputs: HashMap<u32, String>,
    pages: Vec<Vec<Record>>,
}

fn numbers(text: &str) -> Vec<f64> {
    text.split([',', ':'])
        .filter_map(|part| part.trim().parse::<f64>().ok())
        .collect()
}

/// Liest die Textform einer `.synctex`-Datei. Unbekannte Zeilen werden übersprungen.
pub fn parse(text: &str) -> SyncTex {
    let mut result = SyncTex::default();
    let mut unit = 1.0_f64;
    let mut x_offset = 0.0_f64;
    let mut y_offset = 0.0_f64;
    let mut page: Option<usize> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Input:") {
            if let Some((tag, path)) = rest.split_once(':') {
                if let Ok(tag) = tag.parse() {
                    result.inputs.insert(tag, path.to_string());
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("Unit:") {
            unit = rest.trim().parse().unwrap_or(1.0);
            continue;
        }
        if let Some(rest) = line.strip_prefix("X Offset:") {
            x_offset = rest.trim().parse().unwrap_or(0.0);
            continue;
        }
        if let Some(rest) = line.strip_prefix("Y Offset:") {
            y_offset = rest.trim().parse().unwrap_or(0.0);
            continue;
        }
        let Some(&kind) = line.as_bytes().first() else {
            continue;
        };
        match kind {
            b'{' => {
                let number: usize = line[1..].trim().parse().unwrap_or(0);
                if number >= 1 {
                    if result.pages.len() < number {
                        result.pages.resize(number, Vec::new());
                    }
                    page = Some(number - 1);
                }
            }
            b'}' => page = None,
            b'(' | b'[' | b'h' | b'v' | b'x' | b'k' | b'g' | b'$' => {
                let Some(current) = page else {
                    continue;
                };
                let values = numbers(&line[1..]);
                if values.len() < 4 {
                    continue;
                }
                let scale = |value: f64, offset: f64| ((value * unit + offset) * SP_TO_BP) as f32;
                let size = |index: usize| values.get(index).map_or(0.0, |value| scale(*value, 0.0));
                result.pages[current].push(Record {
                    tag: values[0] as u32,
                    line: values[1] as u32,
                    kind,
                    x: scale(values[2], x_offset),
                    y: scale(values[3], y_offset),
                    width: size(4),
                    height: size(5),
                    depth: size(6),
                });
            }
            _ => {}
        }
    }
    result
}

impl SyncTex {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Tag des Hauptdokuments (erste Eingabe).
    fn main_tag(&self) -> u32 {
        self.inputs.keys().copied().min().unwrap_or(1)
    }

    /// Zeile im Hauptdokument zu einem Punkt auf einer Seite: der nächstgelegene (bei Gleichstand
    /// der kleinste) Eintrag mit Zeilennummer.
    pub fn line_at(&self, page: usize, x: f32, y: f32) -> Option<u32> {
        let records = self.pages.get(page)?;
        let main = self.main_tag();
        let candidates: Vec<&Record> = records
            .iter()
            .filter(|record| record.line > 0 && record.tag == main)
            .collect();
        // vboxes umfassen oft die ganze Seite und tragen nur eine grobe Zeile – nur als Notbehelf
        let precise: Vec<&Record> = candidates
            .iter()
            .copied()
            .filter(|record| record.kind != b'[' && record.kind != b'v')
            .collect();
        let candidates = if precise.is_empty() {
            candidates
        } else {
            precise
        };
        let mut best: Option<(f32, f32, u32)> = None;
        for record in candidates {
            let (left, top, right, bottom) = record.rect();
            let dx = (left - x).max(0.0).max(x - right);
            let dy = (top - y).max(0.0).max(y - bottom);
            // senkrechter Abstand wiegt schwerer: Klicks gehören zur Zeile, nicht zur Spalte daneben
            let distance = dx + 3.0 * dy;
            let area = (right - left) * (bottom - top);
            let better = match best {
                None => true,
                Some((best_distance, best_area, _)) => {
                    distance < best_distance - 0.01
                        || ((distance - best_distance).abs() <= 0.01 && area < best_area)
                }
            };
            if better {
                best = Some((distance, area, record.line));
            }
        }
        best.map(|(_, _, line)| line)
    }

    /// PDF-Stelle einer Zeile des Hauptdokuments: die erste Seite, auf der die Zeile vorkommt –
    /// sonst die nächste folgende Zeile mit Inhalt (bzw. die letzte).
    pub fn pdf_for_line(&self, line: u32) -> Option<PdfPoint> {
        let tag = self.main_tag();
        let lines = || {
            self.pages
                .iter()
                .flatten()
                .filter(move |record| record.tag == tag && record.line > 0)
                .map(|record| record.line)
        };
        let target = lines()
            .filter(|candidate| *candidate >= line)
            .min()
            .or_else(|| lines().max())?;
        for (page, records) in self.pages.iter().enumerate() {
            let hits: Vec<&Record> = records
                .iter()
                .filter(|record| record.tag == tag && record.line == target)
                .collect();
            if hits.is_empty() {
                continue;
            }
            // vboxes umfassen oft ganze Absätze/Seiten – lieber die genaueren Einträge
            let precise: Vec<&Record> = hits
                .iter()
                .copied()
                .filter(|record| record.kind != b'[')
                .collect();
            let chosen = if precise.is_empty() { hits } else { precise };
            let top = chosen
                .iter()
                .map(|record| record.rect().1)
                .fold(f32::MAX, f32::min);
            let bottom = chosen
                .iter()
                .map(|record| record.rect().3)
                .fold(f32::MIN, f32::max);
            let left = chosen
                .iter()
                .map(|record| record.x)
                .fold(f32::MAX, f32::min);
            return Some(PdfPoint {
                page,
                x: left,
                y: top,
                height: (bottom - top).max(1.0),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1 pt = 65536 sp; in bp: × 72/72.27
    fn bp(points: f64) -> f32 {
        (points * 72.0 / 72.27) as f32
    }

    const SAMPLE: &str = "SyncTeX Version:1
Input:1:texput
Input:2:
Output:pdf
Magnification:1000
Unit:1
X Offset:0
Y Offset:0
Content:
!120
{1
[1,5:4736286,7577261:30785863,40394276,0
(1,10:4736286,9830400:30785863,655360,196608
h1,9:4736286,9830400:1179648,0,0
x1,10:4736286,9830400
k1,10:6553600,9830400:65536
)
(1,12:4736286,11796480:30785863,655360,196608
)
]
}1
{2
[1,20:4736286,7577261:30785863,40394276,0
(2,3:4736286,7864320:30785863,655360,196608
)
(1,25:4736286,9830400:30785863,655360,196608
)
]
}2
Postamble:
";

    #[test]
    fn maps_pdf_points_to_source_lines() {
        let sync = parse(SAMPLE);
        assert_eq!(sync.page_count(), 2);
        // Zeile 10 liegt bei y = 150 pt (9830400 sp), x ab 72.27 pt (1 Zoll)
        assert_eq!(sync.line_at(0, bp(100.0), bp(148.0)), Some(10));
        assert_eq!(sync.line_at(0, bp(100.0), bp(178.0)), Some(12));
        // Klick neben den Text (rechts außerhalb) – trotzdem die Zeile auf dieser Höhe
        assert_eq!(sync.line_at(0, bp(560.0), bp(149.0)), Some(10));
        // Inhalt einer anderen Eingabe (Tag 2) zählt nicht – nächste Zeile des Hauptdokuments
        assert_eq!(sync.line_at(1, bp(100.0), bp(118.0)), Some(25));
        assert!(sync.line_at(5, 0.0, 0.0).is_none());
    }

    #[test]
    fn maps_source_lines_to_pdf_points() {
        let sync = parse(SAMPLE);
        let point = sync.pdf_for_line(12).unwrap();
        assert_eq!(point.page, 0);
        assert!((point.y - bp(170.0)).abs() < 0.1, "{point:?}");
        assert!((point.x - bp(72.27)).abs() < 0.1);
        // flacher Einzug (Zeile 9) wird wie eine Textzeile behandelt
        let indent = sync.pdf_for_line(9).unwrap();
        assert!(indent.height >= 9.0, "{indent:?}");
        // Zeile ohne Eintrag → nächste folgende Zeile (25 auf Seite 2)
        assert_eq!(sync.pdf_for_line(21).unwrap().page, 1);
        // hinter der letzten Zeile → letzte Zeile
        assert_eq!(sync.pdf_for_line(999).unwrap().page, 1);
    }
}
