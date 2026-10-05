//! Skizzier-Werkzeug: Skizzenmodell (Linien, Pfade, Formen, Text, alle
//! CircuiTikZ-Bauteile und TikZ-Formen aus `sketch_catalog` auf einem Raster)
//! → TikZ- bzw. CircuiTikZ-Code.
//!
//! Die Koordinaten liegen in Rastereinheiten (0,5 cm), y wächst wie am Bildschirm
//! nach unten; im TikZ-Code wird y gespiegelt. Die erste Codezeile enthält das
//! Modell als Kommentar (`% VisuTeX-Skizze: {…}`), damit die Zeichnung später
//! wieder im Skizzier-Werkzeug bearbeitet werden kann.

use super::escape::escape_latex;
use super::sketch_catalog::{self, SymbolKind};
use serde::{Deserialize, Serialize};

pub const SKETCH_COMMENT: &str = "% VisuTeX-Skizze: ";
/// Rastereinheit in cm.
pub const UNIT_CM: f64 = 0.5;

/// Pfeilspitzen (arrows.meta), die im Werkzeug wählbar sind.
pub const ARROW_TIPS: &[&str] = &[
    "Stealth",
    "Stealth[open]",
    "Latex",
    "Latex[open]",
    "To",
    "Triangle",
    "Triangle[open]",
    "Kite",
    "Kite[open]",
    "Circle",
    "Circle[open]",
    "Square",
    "Square[open]",
    "Diamond",
    "Diamond[open]",
    "Ellipse",
    "Turned Square",
    "Bar",
    "Bracket",
    "Parenthesis",
    "Rays",
    "Hooks",
    "Straight Barb",
    "Arc Barb",
    "Tee Barb",
    "Implies",
    "Computer Modern Rightarrow",
];

pub const LINE_WIDTHS: &[&str] = &[
    "ultra thin",
    "very thin",
    "thin",
    "semithick",
    "thick",
    "very thick",
    "ultra thick",
];

pub const DASH_PATTERNS: &[&str] = &[
    "dashed",
    "densely dashed",
    "loosely dashed",
    "dotted",
    "densely dotted",
    "loosely dotted",
    "dash dot",
    "dash dot dot",
];

pub const FONT_SIZES: &[&str] = &[
    "tiny",
    "scriptsize",
    "footnotesize",
    "small",
    "normalsize",
    "large",
    "Large",
    "LARGE",
    "huge",
    "Huge",
];

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SketchElement {
    pub id: String,
    /// line | arrow | path | rect | circle | ellipse | arc | text | dot | terminal |
    /// ground | component (alt) | symbol (Katalogeintrag)
    pub kind: String,
    /// Bauteil der alten Skizzen (`component`): R, C, L, V, I, D, LED, switch, …
    pub component: String,
    /// Katalogeintrag (`symbol`): CircuiTikZ-Bauteil oder TikZ-Form
    pub symbol: String,
    /// Knotenname für Verweise (`(Q1.G)`)
    pub name: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    /// Punkte eines Pfads (Rastereinheiten)
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
    pub smooth: bool,
    /// Bogen: überstrichener Winkel in Grad (gegen den Uhrzeigersinn)
    pub sweep: f64,
    /// Drehung in Grad im Uhrzeigersinn (wie am Bildschirm)
    pub rotation: f64,
    pub mirror: bool,
    pub flip: bool,
    /// Beschriftung (Bauteil: `l=`; Text, Form: Inhalt)
    pub label: String,
    /// Bauteil: Wert/Bezeichnung auf der Gegenseite (`a=`)
    pub annotation: String,
    pub voltage: String,
    pub current: String,
    pub flow: String,
    /// Beschriftung unterhalb (`l_`)
    pub label_below: bool,
    /// Spannungspfeil: `` | `^` | `_`
    pub voltage_side: String,
    /// Strompfeil: `` | `>^` | `>_` | `<^` | `<_`
    pub current_dir: String,
    pub invert: bool,
    /// Knotenbeschriftung: above | below | left | right | inside
    pub label_position: String,
    /// Endpunkt-Verweise auf Anschlüsse (`Name.Anschluss`)
    pub ref1: String,
    pub ref2: String,
    pub color: String,
    pub fill: String,
    pub line_width: String,
    pub dash: String,
    pub arrow_start: String,
    pub arrow_end: String,
    pub font_size: String,
    pub bold: bool,
    pub italic: bool,
    /// auto | text | math
    pub text_mode: String,
    // ältere Felder
    pub dashed: bool,
    pub thick: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Sketch {
    pub version: u32,
    pub elements: Vec<SketchElement>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchCode {
    /// Inhalt der Umgebung (ohne \begin/\end)
    pub code: String,
    /// tikzpicture | circuitikz
    pub environment: String,
}

fn number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        "0".into()
    } else {
        format!("{rounded}")
    }
}

/// Rasterpunkt → TikZ-Koordinate (cm, y nach oben).
fn point(x: f64, y: f64) -> String {
    format!("({},{})", number(x * UNIT_CM), number(-y * UNIT_CM))
}

/// Beschriftung: Formeln (`R_1`, `\Omega`, `$…$`) im Mathematikmodus, Text sonst.
fn label_latex(label: &str) -> String {
    label_with_mode(label, "auto")
}

fn label_with_mode(label: &str, mode: &str) -> String {
    let label = label.trim();
    match mode {
        "math" => format!("${}$", label.trim_matches('$')),
        "text" => format!("{{{}}}", escape_latex(label)),
        _ => {
            if label.contains('$') {
                return label.to_string();
            }
            let mathy = label.contains(['_', '^', '\\'])
                || (label.chars().count() <= 4 && !label.contains(' '));
            if mathy {
                format!("${label}$")
            } else {
                format!("{{{}}}", escape_latex(label))
            }
        }
    }
}

/// `#1f2937` → `{rgb,255:red,31;green,41;blue,55}` (ohne Präambel-Definition).
fn color_spec(color: &str) -> Option<String> {
    let hex = color.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    let (red, green, blue) = (channel(0)?, channel(2)?, channel(4)?);
    Some(format!("{{rgb,255:red,{red};green,{green};blue,{blue}}}"))
}

fn arrow_tip(tip: &str) -> Option<&'static str> {
    ARROW_TIPS
        .iter()
        .copied()
        .find(|candidate| *candidate == tip)
}

/// Linienstil (Farbe, Stärke, Strichart, Pfeile) als TikZ-Optionen.
fn stroke_options(element: &SketchElement, with_arrows: bool) -> Vec<String> {
    let mut options = Vec::new();
    if with_arrows {
        let start = arrow_tip(&element.arrow_start);
        let end = arrow_tip(&element.arrow_end).or_else(|| {
            (element.kind == "arrow"
                && element.arrow_end.is_empty()
                && element.arrow_start.is_empty())
            .then_some("Stealth")
        });
        if start.is_some() || end.is_some() {
            options.push(format!(
                "{}-{}",
                start.map(|tip| format!("{{{tip}}}")).unwrap_or_default(),
                end.map(|tip| format!("{{{tip}}}")).unwrap_or_default()
            ));
        }
    }
    if let Some(color) = color_spec(&element.color) {
        options.push(format!("draw={color}"));
    }
    if let Some(width) = LINE_WIDTHS
        .iter()
        .find(|width| **width == element.line_width)
    {
        options.push((*width).to_string());
    } else if element.thick {
        options.push("thick".into());
    }
    if let Some(dash) = DASH_PATTERNS.iter().find(|dash| **dash == element.dash) {
        options.push((*dash).to_string());
    } else if element.dashed {
        options.push("dashed".into());
    }
    options
}

fn fill_option(element: &SketchElement) -> Option<String> {
    color_spec(&element.fill).map(|color| format!("fill={color}"))
}

fn bracket(options: &[String]) -> String {
    if options.is_empty() {
        String::new()
    } else {
        format!("[{}]", options.join(", "))
    }
}

/// Schriftoptionen für Text-Knoten.
fn font_options(element: &SketchElement) -> Vec<String> {
    let mut font = String::new();
    if let Some(size) = FONT_SIZES.iter().find(|size| **size == element.font_size) {
        font.push_str(&format!("\\{size}"));
    }
    if element.bold {
        font.push_str("\\bfseries");
    }
    if element.italic {
        font.push_str("\\itshape");
    }
    let mut options = Vec::new();
    if !font.is_empty() {
        options.push(format!("font={font}"));
    }
    if let Some(color) = color_spec(&element.color) {
        options.push(format!("text={color}"));
    }
    if element.rotation.abs() > 0.01 {
        options.push(format!("rotate={}", number(-element.rotation)));
    }
    options
}

/// Gültiger TikZ-Knotenname (Buchstabe, dann Buchstaben/Ziffern).
fn node_name(element: &SketchElement, index: usize) -> String {
    let name = element.name.trim();
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric());
    if valid {
        name.to_string()
    } else {
        format!("N{}", index + 1)
    }
}

/// CircuiTikZ-Bauteilname der alten Skizzen (europäischer Stil).
fn component_name(component: &str) -> &'static str {
    match component {
        "C" => "C",
        "L" => "L",
        "V" => "V",
        "I" => "I",
        "D" => "D",
        "LED" => "leD",
        "switch" => "nos",
        "lamp" => "lamp",
        "battery" => "battery1",
        "voltmeter" | "ammeter" => "rmeter",
        "fuse" => "fuse",
        "Rvar" => "vR",
        "short" => "short",
        _ => "R",
    }
}

/// Bauteiloptionen (`l=`, `a=`, `v=`, `i=`, `f=`, `invert`, `mirror`).
fn bipole_options(name: &str, element: &SketchElement) -> Vec<String> {
    let mut options = vec![name.to_string()];
    // offene Klemmen sichtbar machen (wie im Skizzen-Fenster)
    if name == "open" {
        options.push("o-o".into());
    }
    match element.component.as_str() {
        "voltmeter" if element.kind == "component" => options.push("t=V".into()),
        "ammeter" if element.kind == "component" => options.push("t=A".into()),
        _ => {}
    }
    let label = element.label.trim();
    if !label.is_empty() {
        let key = if element.label_below { "l_" } else { "l" };
        options.push(format!("{key}={}", label_latex(label)));
    }
    if !element.annotation.trim().is_empty() {
        options.push(format!("a={}", label_latex(&element.annotation)));
    }
    if !element.voltage.trim().is_empty() {
        let side = match element.voltage_side.as_str() {
            "^" => "^",
            "_" => "_",
            _ => "",
        };
        options.push(format!("v{side}={}", label_latex(&element.voltage)));
    }
    if !element.current.trim().is_empty() {
        let direction = match element.current_dir.as_str() {
            ">^" => ">^",
            ">_" => ">_",
            "<^" => "<^",
            "<_" => "<_",
            _ => "",
        };
        options.push(format!("i{direction}={}", label_latex(&element.current)));
    }
    if !element.flow.trim().is_empty() {
        options.push(format!("f={}", label_latex(&element.flow)));
    }
    if element.invert {
        options.push("invert".into());
    }
    if element.mirror {
        options.push("mirror".into());
    }
    options
}

pub fn to_latex(sketch: &Sketch) -> SketchCode {
    let circuit = sketch
        .elements
        .iter()
        .any(|element| match element.kind.as_str() {
            "component" | "ground" | "terminal" => true,
            "symbol" => sketch_catalog::entry(&element.symbol)
                .is_some_and(|entry| entry.kind != SymbolKind::Shape),
            _ => false,
        });
    // Knotennamen (für Verweise von Leitungen auf Anschlüsse)
    let names: Vec<(String, String)> = sketch
        .elements
        .iter()
        .enumerate()
        .filter(|(_, element)| element.kind == "symbol")
        .map(|(index, element)| (element.id.clone(), node_name(element, index)))
        .collect();
    let endpoint = |reference: &str, x: f64, y: f64| -> String {
        if let Some((id, anchor)) = reference.split_once('.') {
            if let Some((_, name)) = names.iter().find(|(element, _)| element == id) {
                let anchor_ok = !anchor.is_empty()
                    && anchor
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '+' || c == '-');
                if anchor_ok {
                    return format!("({name}.{anchor})");
                }
            }
        }
        point(x, y)
    };
    let mut lines = Vec::new();
    let model = serde_json::to_string(&Sketch {
        version: 2,
        elements: sketch.elements.clone(),
    })
    .unwrap_or_else(|_| "{}".into());
    lines.push(format!("{SKETCH_COMMENT}{model}"));
    for (index, element) in sketch.elements.iter().enumerate() {
        let from = endpoint(&element.ref1, element.x1, element.y1);
        let to = endpoint(&element.ref2, element.x2, element.y2);
        let label = element.label.trim();
        let line = match element.kind.as_str() {
            "line" | "arrow" => {
                let options = stroke_options(element, true);
                let node = if label.is_empty() {
                    String::new()
                } else {
                    format!(" node[midway, above, sloped] {{{}}}", label_latex(label))
                };
                format!("\\draw{} {from} --{node} {to};", bracket(&options))
            }
            "path" => {
                if element.points.len() < 2 {
                    continue;
                }
                let mut options = stroke_options(element, !element.closed);
                if element.closed {
                    options.extend(fill_option(element));
                }
                let coordinates: Vec<String> =
                    element.points.iter().map(|[x, y]| point(*x, *y)).collect();
                if element.smooth {
                    let cycle = if element.closed { " cycle" } else { "" };
                    format!(
                        "\\draw{} plot[smooth{cycle}] coordinates {{{}}};",
                        bracket(&options),
                        coordinates.join(" ")
                    )
                } else {
                    let cycle = if element.closed { " -- cycle" } else { "" };
                    format!(
                        "\\draw{} {}{cycle};",
                        bracket(&options),
                        coordinates.join(" -- ")
                    )
                }
            }
            "rect" => {
                let mut options = stroke_options(element, false);
                options.extend(fill_option(element));
                let node = if label.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\\node at {} {{{}}};",
                        point(
                            (element.x1 + element.x2) / 2.0,
                            (element.y1 + element.y2) / 2.0
                        ),
                        label_latex(label)
                    )
                };
                format!("\\draw{} {from} rectangle {to};{node}", bracket(&options))
            }
            "circle" => {
                let mut options = stroke_options(element, false);
                options.extend(fill_option(element));
                let radius =
                    ((element.x2 - element.x1).powi(2) + (element.y2 - element.y1).powi(2)).sqrt()
                        * UNIT_CM;
                let node = if label.is_empty() {
                    String::new()
                } else {
                    format!(" node {{{}}}", label_latex(label))
                };
                format!(
                    "\\draw{} {from} circle ({}){node};",
                    bracket(&options),
                    number(radius.max(0.05))
                )
            }
            "ellipse" => {
                let mut options = stroke_options(element, false);
                options.extend(fill_option(element));
                format!(
                    "\\draw{} {} ellipse [x radius={}, y radius={}];",
                    bracket(&options),
                    point(
                        (element.x1 + element.x2) / 2.0,
                        (element.y1 + element.y2) / 2.0
                    ),
                    number(((element.x2 - element.x1).abs() / 2.0 * UNIT_CM).max(0.05)),
                    number(((element.y2 - element.y1).abs() / 2.0 * UNIT_CM).max(0.05))
                )
            }
            "arc" => {
                let options = stroke_options(element, true);
                let dx = element.x2 - element.x1;
                let dy = element.y2 - element.y1;
                let radius = (dx * dx + dy * dy).sqrt() * UNIT_CM;
                // Winkel in TikZ-Richtung (y nach oben)
                let start = (-dy).atan2(dx).to_degrees();
                let sweep = if element.sweep.abs() < 0.5 {
                    90.0
                } else {
                    element.sweep
                };
                format!(
                    "\\draw{} {} arc[start angle={}, end angle={}, radius={}];",
                    bracket(&options),
                    point(element.x2, element.y2),
                    number(start),
                    number(start + sweep),
                    number(radius.max(0.05))
                )
            }
            "text" => {
                let options = font_options(element);
                format!(
                    "\\node{} at {from} {{{}}};",
                    bracket(&options),
                    if label.is_empty() {
                        "Text".to_string()
                    } else {
                        label_with_mode(label, &element.text_mode)
                    }
                )
            }
            "dot" => {
                if circuit {
                    format!("\\draw {from} node[circ] {{}};")
                } else {
                    format!("\\fill {from} circle (2pt);")
                }
            }
            "terminal" => {
                let node = if label.is_empty() {
                    String::new()
                } else {
                    format!(" node[left] {{{}}}", label_latex(label))
                };
                format!("\\draw {from} node[ocirc] {{}}{node};")
            }
            "ground" => format!("\\draw {from} node[ground] {{}};"),
            "component" => {
                let options = bipole_options(component_name(&element.component), element);
                format!(
                    "\\draw{} {from} to[{}] {to};",
                    bracket(&stroke_options(element, false)),
                    options.join(", ")
                )
            }
            "symbol" => {
                let Some(entry) = sketch_catalog::entry(&element.symbol) else {
                    lines.push("% unbekanntes Symbol ausgelassen".to_string());
                    continue;
                };
                match entry.kind {
                    SymbolKind::Bipole => {
                        let options = bipole_options(entry.id, element);
                        format!(
                            "\\draw{} {from} to[{}] {to};",
                            bracket(&stroke_options(element, false)),
                            options.join(", ")
                        )
                    }
                    SymbolKind::Node => {
                        let name = node_name(element, index);
                        let mut options = vec![entry.id.to_string()];
                        if element.rotation.abs() > 0.01 {
                            options.push(format!("rotate={}", number(-element.rotation)));
                        }
                        if element.mirror {
                            options.push("xscale=-1".into());
                        }
                        if element.flip {
                            options.push("yscale=-1".into());
                        }
                        if let Some(color) = color_spec(&element.color) {
                            options.push(format!("color={color}"));
                        }
                        let inside = element.label_position == "inside";
                        if !label.is_empty() && !inside {
                            let position = match element.label_position.as_str() {
                                "below" | "left" | "right" => element.label_position.as_str(),
                                _ => "above",
                            };
                            options.push(format!(
                                "label={{[absolute]{position}:{}}}",
                                label_latex(label)
                            ));
                        }
                        let text = if inside && !label.is_empty() {
                            label_latex(label)
                        } else {
                            String::new()
                        };
                        format!(
                            "\\draw {from} node[{}] ({name}) {{{text}}};",
                            options.join(", ")
                        )
                    }
                    SymbolKind::Shape => {
                        let mut options = vec![format!(
                            "draw{}",
                            color_spec(&element.color)
                                .map(|color| format!("={color}"))
                                .unwrap_or_default()
                        )];
                        options.extend(fill_option(element));
                        options.push(entry.id.to_string());
                        if let Some(width) = LINE_WIDTHS
                            .iter()
                            .find(|width| **width == element.line_width)
                        {
                            options.push((*width).to_string());
                        }
                        if let Some(dash) = DASH_PATTERNS.iter().find(|dash| **dash == element.dash)
                        {
                            options.push((*dash).to_string());
                        }
                        let width = (element.x2 - element.x1).abs() * UNIT_CM;
                        let height = (element.y2 - element.y1).abs() * UNIT_CM;
                        options.push(format!("minimum width={}cm", number(width.max(0.2))));
                        options.push(format!("minimum height={}cm", number(height.max(0.2))));
                        let mut text_options = font_options(element);
                        // Die Füllfarbe darf die Schriftfarbe nicht ersetzen: Text in `text=`.
                        options.append(&mut text_options);
                        let name = node_name(element, index);
                        format!(
                            "\\node[{}] ({name}) at {} {{{}}};",
                            options.join(", "),
                            point(
                                (element.x1 + element.x2) / 2.0,
                                (element.y1 + element.y2) / 2.0
                            ),
                            if label.is_empty() {
                                String::new()
                            } else if element.text_mode.is_empty() {
                                label_with_mode(label, "text")
                            } else {
                                label_with_mode(label, &element.text_mode)
                            }
                        )
                    }
                }
            }
            _ => continue,
        };
        lines.push(line);
    }
    SketchCode {
        code: lines.join("\n"),
        environment: if circuit {
            "circuitikz".into()
        } else {
            "tikzpicture".into()
        },
    }
}

/// Skizzenmodell aus dem Kommentar in der ersten Codezeile (falls vorhanden).
pub fn from_code(code: &str) -> Option<Sketch> {
    let first = code.lines().next()?.trim();
    let json = first.strip_prefix(SKETCH_COMMENT.trim_end())?.trim();
    serde_json::from_str(json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(
        kind: &str,
        component: &str,
        coords: (f64, f64, f64, f64),
        label: &str,
    ) -> SketchElement {
        SketchElement {
            id: format!("{kind}-{component}"),
            kind: kind.into(),
            component: component.into(),
            x1: coords.0,
            y1: coords.1,
            x2: coords.2,
            y2: coords.3,
            label: label.into(),
            ..Default::default()
        }
    }

    #[test]
    fn generates_circuitikz_and_round_trips_model() {
        let mut source = element("component", "V", (0.0, 6.0, 0.0, 0.0), "U_0");
        source.voltage = "U_0".into();
        let sketch = Sketch {
            version: 2,
            elements: vec![
                source,
                element("component", "R", (0.0, 0.0, 6.0, 0.0), "R_1"),
                element("component", "C", (6.0, 0.0, 6.0, 6.0), "C"),
                element("line", "", (6.0, 6.0, 0.0, 6.0), ""),
                element("ground", "", (0.0, 6.0, 0.0, 6.0), ""),
                element("text", "", (3.0, 3.0, 3.0, 3.0), "Tiefpass erster Ordnung"),
            ],
        };
        let code = to_latex(&sketch);
        assert_eq!(code.environment, "circuitikz");
        assert!(code
            .code
            .contains("\\draw (0,-3) to[V, l=$U_0$, v=$U_0$] (0,0);"));
        assert!(code.code.contains("\\draw (0,0) to[R, l=$R_1$] (3,0);"));
        assert!(code.code.contains("\\draw (3,-3) -- (0,-3);"));
        assert!(code.code.contains("\\draw (0,-3) node[ground] {};"));
        assert!(code
            .code
            .contains("\\node at (1.5,-1.5) {{Tiefpass erster Ordnung}};"));
        assert_eq!(from_code(&code.code), Some(sketch));
    }

    #[test]
    fn plain_drawing_uses_tikzpicture() {
        let mut arrow = element("arrow", "", (0.0, 0.0, 4.0, -2.0), "F");
        arrow.thick = true;
        let code = to_latex(&Sketch {
            version: 2,
            elements: vec![arrow, element("circle", "", (2.0, 2.0, 4.0, 2.0), "")],
        });
        assert_eq!(code.environment, "tikzpicture");
        assert!(code.code.contains(
            "\\draw[-{Stealth}, thick] (0,0) -- node[midway, above, sloped] {$F$} (2,1);"
        ));
        assert!(code.code.contains("\\draw (1,-1) circle (1);"));
    }

    #[test]
    fn open_circuit_shows_terminals() {
        let mut open = element("symbol", "", (0.0, 0.0, 4.0, 0.0), "");
        open.symbol = "open".into();
        open.voltage = "U".into();
        let code = to_latex(&Sketch {
            version: 2,
            elements: vec![open],
        })
        .code;
        assert!(code.contains("to[open, o-o, v=$U$]"), "{code}");
    }

    #[test]
    fn catalog_symbols_styles_and_pin_references() {
        let mut transistor = element("symbol", "", (4.0, 4.0, 4.0, 4.0), "Q_1");
        transistor.id = "q".into();
        transistor.symbol = "nmos".into();
        transistor.name = "Q1".into();
        transistor.rotation = 90.0;
        transistor.label_position = "right".into();
        let mut wire = element("line", "", (0.0, 4.0, 3.0, 4.0), "");
        wire.ref2 = "q.G".into();
        let mut resistor = element("symbol", "", (0.0, 0.0, 6.0, 0.0), "R_1");
        resistor.symbol = "R".into();
        resistor.annotation = "10 kΩ".into();
        resistor.current = "I".into();
        resistor.current_dir = "<_".into();
        resistor.label_below = true;
        let mut cloud = element("symbol", "", (0.0, 8.0, 4.0, 10.0), "Idee");
        cloud.symbol = "cloud".into();
        cloud.fill = "#dbeafe".into();
        cloud.font_size = "small".into();
        cloud.bold = true;
        let mut path = element("path", "", (0.0, 0.0, 0.0, 0.0), "");
        path.points = vec![[0.0, 0.0], [2.0, -2.0], [4.0, 0.0]];
        path.smooth = true;
        path.arrow_start = "Latex".into();
        path.arrow_end = "Circle[open]".into();
        path.color = "#c00000".into();
        path.dash = "dotted".into();
        let mut unknown = element("symbol", "", (0.0, 0.0, 1.0, 1.0), "");
        unknown.symbol = "gibtsnicht}\\evil".into();
        let code = to_latex(&Sketch {
            version: 2,
            elements: vec![transistor, wire, resistor, cloud, path, unknown],
        })
        .code;
        assert!(code.contains(
            "\\draw (2,-2) node[nmos, rotate=-90, label={[absolute]right:$Q_1$}] (Q1) {};"
        ));
        assert!(code.contains("\\draw (0,-2) -- (Q1.G);"));
        assert!(code.contains("to[R, l_=$R_1$, a={10 k\\ensuremath{\\Omega}}, i<_=$I$] (3,0);"));
        assert!(code.contains("\\node[draw, fill={rgb,255:red,219;green,234;blue,254}, cloud, minimum width=2cm, minimum height=1cm, font=\\small\\bfseries] (N4) at (1,-4.5) {{Idee}};"));
        assert!(code.contains("\\draw[{Latex}-{Circle[open]}, draw={rgb,255:red,192;green,0;blue,0}, dotted] plot[smooth] coordinates {(0,0) (1,1) (2,0)};"));
        assert!(code.contains("% unbekanntes Symbol"));
        // Der Name steht nur im Modell-Kommentar (erste Zeile), nie im ausgeführten Code.
        assert!(code
            .lines()
            .filter(|line| !line.starts_with('%'))
            .all(|line| !line.contains("\\evil")));
    }
}
