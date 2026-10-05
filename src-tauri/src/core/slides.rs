//! Folien-Editor: Datenmodell einer Präsentation und Export nach LaTeX-Beamer.
//!
//! Jede Folie ist eine freie Fläche (16:9 = 160 × 90 mm bzw. 4:3 = 128 × 96 mm,
//! wie `beamer` mit `aspectratio=169/43`). Elemente (Textfelder, Formen, Bilder,
//! Formeln) liegen absolut positioniert in Millimetern; der Export setzt sie mit
//! `textpos` + TikZ exakt an dieselbe Stelle. Text in Textfeldern ist Tiptap-JSON
//! und wird mit demselben Exporter wie der Dokumenteditor nach LaTeX übersetzt.
//!
//! Das erzeugte Dokument kompiliert mit pdfLaTeX, XeLaTeX, LuaLaTeX und der
//! eingebauten Engine (gleiche engine-neutrale Schrift-Einrichtung wie Dokumente).

use super::escape::escape_latex;
use super::export::{content_hash, css_color_to_hex, embedded_asset, EmbeddedAsset, Exporter};
use super::preamble::{self, Usage};
use super::settings::DocumentSettings;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FORMAT: &str = "visutex-slides";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SlideDeck {
    pub format: String,
    pub version: u32,
    pub title: String,
    pub subtitle: String,
    pub author: String,
    pub date: String,
    /// `ngerman` | `naustrian` | `english`
    pub language: String,
    /// `16:9` | `4:3`
    pub aspect: String,
    pub theme: SlideTheme,
    pub grid: SlideGrid,
    pub slides: Vec<Slide>,
}

impl Default for SlideDeck {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: 1,
            title: String::new(),
            subtitle: String::new(),
            author: String::new(),
            date: String::new(),
            language: "ngerman".into(),
            aspect: "16:9".into(),
            theme: SlideTheme::default(),
            grid: SlideGrid::default(),
            slides: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SlideTheme {
    pub background: String,
    pub text_color: String,
    pub accent: String,
    /// `sans` | `serif`
    pub font: String,
}

impl Default for SlideTheme {
    fn default() -> Self {
        Self {
            background: "#ffffff".into(),
            text_color: "#1f2937".into(),
            accent: "#2563eb".into(),
            font: "sans".into(),
        }
    }
}

/// Raster des Editors (wird nicht exportiert, aber mit der Datei gespeichert).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SlideGrid {
    /// Rasterweite in mm
    pub size: f64,
    pub snap: bool,
    pub show: bool,
    /// Hilfslinien an Folienmitte und anderen Elementen
    pub guides: bool,
}

impl Default for SlideGrid {
    fn default() -> Self {
        Self {
            size: 5.0,
            snap: true,
            show: true,
            guides: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Slide {
    pub id: String,
    /// Eigener Hintergrund (leer = Design)
    pub background: String,
    pub notes: String,
    pub hidden: bool,
    pub elements: Vec<SlideElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SlideElement {
    pub id: String,
    /// `text` | `shape` | `image` | `formula`
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Drehung im Uhrzeigersinn in Grad (wie CSS `rotate`)
    pub rotation: f64,
    /// `rect` | `roundRect` | `ellipse` | `triangle` | `diamond` | `line` | `arrow`
    pub shape: String,
    /// Linien/Pfeile: Endpunkte gespiegelt (von rechts bzw. unten)
    pub flip_h: bool,
    pub flip_v: bool,
    pub fill: String,
    pub stroke: String,
    /// Linienstärke in pt
    pub stroke_width: f64,
    /// Text (Tiptap-JSON `doc`) in Textfeldern und Formen
    pub content: Value,
    /// Grundschriftgröße in pt
    pub font_size: f64,
    pub color: String,
    /// `top` | `middle` | `bottom`
    pub vertical_align: String,
    /// Innenabstand in mm
    pub padding: f64,
    /// Bild als data:-URL (PNG, JPEG, SVG)
    pub src: String,
    /// Formel (LaTeX, Mathematikmodus)
    pub latex: String,
    /// Platzhalter-Rolle aus dem Folienlayout: `title` | `subtitle` | `body` | ``
    pub role: String,
}

impl Default for SlideElement {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: "text".into(),
            x: 10.0,
            y: 10.0,
            w: 60.0,
            h: 20.0,
            rotation: 0.0,
            shape: "rect".into(),
            flip_h: false,
            flip_v: false,
            fill: String::new(),
            stroke: String::new(),
            stroke_width: 1.0,
            content: Value::Null,
            font_size: 18.0,
            color: String::new(),
            vertical_align: "top".into(),
            padding: 2.0,
            src: String::new(),
            latex: String::new(),
            role: String::new(),
        }
    }
}

/// Foliengröße in mm (Beamer: `aspectratio=169` → 160 × 90 mm, `43` → 128 × 96 mm).
pub fn slide_size(aspect: &str) -> (f64, f64) {
    match aspect {
        "4:3" => (128.0, 96.0),
        _ => (160.0, 90.0),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlidesLatex {
    pub latex: String,
    pub assets: Vec<EmbeddedAsset>,
    pub warnings: Vec<String>,
}

/// Liest eine Folien-Datei (tolerant: fehlende Felder erhalten Standardwerte).
pub fn parse_deck(source: &str) -> Result<SlideDeck, String> {
    let value: Value = serde_json::from_str(source)
        .map_err(|error| format!("Die Foliendatei ist kein gültiges JSON: {error}"))?;
    if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err("Die Datei ist keine VisuTeX-Präsentation.".into());
    }
    let mut deck: SlideDeck = serde_json::from_value(value)
        .map_err(|error| format!("Die Foliendatei ist beschädigt: {error}"))?;
    sanitize(&mut deck);
    Ok(deck)
}

/// Begrenzt Werte auf sinnvolle Bereiche (Dateien können von Hand bearbeitet sein).
pub fn sanitize(deck: &mut SlideDeck) {
    deck.format = FORMAT.into();
    if !matches!(deck.aspect.as_str(), "16:9" | "4:3") {
        deck.aspect = "16:9".into();
    }
    if !deck.grid.size.is_finite() || deck.grid.size < 0.5 {
        deck.grid.size = 5.0;
    }
    deck.grid.size = deck.grid.size.min(20.0);
    let (width, height) = slide_size(&deck.aspect);
    for slide in &mut deck.slides {
        for element in &mut slide.elements {
            let clamp = |value: f64, min: f64, max: f64, fallback: f64| {
                if value.is_finite() {
                    value.clamp(min, max)
                } else {
                    fallback
                }
            };
            element.x = clamp(element.x, -width, 2.0 * width, 0.0);
            element.y = clamp(element.y, -height, 2.0 * height, 0.0);
            element.w = clamp(element.w, 0.0, 4.0 * width, 10.0);
            element.h = clamp(element.h, 0.0, 4.0 * height, 10.0);
            element.rotation = clamp(element.rotation, -360.0, 360.0, 0.0);
            element.font_size = clamp(element.font_size, 4.0, 200.0, 18.0);
            element.stroke_width = clamp(element.stroke_width, 0.0, 20.0, 1.0);
            element.padding = clamp(element.padding, 0.0, 20.0, 2.0);
        }
    }
}

fn number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == rounded.trunc() {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Farbe als `xcolor`-Ausdruck (`{HTML}{1F2937}`-Form für `\color[HTML]`), sonst `None`.
fn hex(color: &str) -> Option<String> {
    let color = color.trim();
    if color.is_empty() || color == "transparent" || color == "none" {
        return None;
    }
    css_color_to_hex(color).map(|value| value.trim_start_matches('#').to_uppercase())
}

struct Colors {
    names: Vec<(String, String)>,
}

impl Colors {
    /// Name einer (deduplizierten) Farbdefinition `vtxcN`.
    fn name(&mut self, color: &str) -> Option<String> {
        let value = hex(color)?;
        if let Some((name, _)) = self.names.iter().find(|(_, existing)| *existing == value) {
            return Some(name.clone());
        }
        let name = format!("vtxc{}", self.names.len() + 1);
        self.names.push((name.clone(), value));
        Some(name)
    }
}

struct SlideExport<'a> {
    exporter: Exporter<'a>,
    colors: Colors,
    assets: Vec<EmbeddedAsset>,
    warnings: Vec<String>,
}

impl SlideExport<'_> {
    fn element(&mut self, element: &SlideElement, slide_number: usize) -> String {
        let cx = element.x + element.w / 2.0;
        let cy = element.y + element.h / 2.0;
        // TikZ dreht gegen den Uhrzeigersinn, der Editor (CSS) im Uhrzeigersinn.
        let rotate = if element.rotation.abs() > 0.01 {
            format!(
                ", rotate around={{{}:({},{})}}",
                number(-element.rotation),
                number(cx),
                number(cy)
            )
        } else {
            String::new()
        };
        let node_rotate = if element.rotation.abs() > 0.01 {
            format!(", rotate={}", number(-element.rotation))
        } else {
            String::new()
        };
        let mut out = String::new();
        match element.kind.as_str() {
            "image" => {
                let Some(file) = self.image_file(&element.src) else {
                    self.warnings.push(format!(
                        "Folie {slide_number}: Bild ohne gültige Bilddaten wurde übersprungen."
                    ));
                    return String::new();
                };
                out.push_str(&format!(
                    "  \\node[anchor=center, inner sep=0pt{node_rotate}] at ({},{}) {{\\includegraphics[width={}mm,height={}mm]{{{file}}}}};\n",
                    number(cx),
                    number(cy),
                    number(element.w.max(1.0)),
                    number(element.h.max(1.0)),
                ));
            }
            "formula" => {
                let latex = element.latex.trim();
                if latex.is_empty() {
                    return String::new();
                }
                self.exporter.usage.ams = true;
                let color = self
                    .colors
                    .name(&element.color)
                    .map(|name| format!(", text={name}"))
                    .unwrap_or_default();
                let size = element.font_size;
                out.push_str(&format!(
                    "  \\node[anchor=center, inner sep=0pt{color}{node_rotate}] at ({},{}) {{\\fontsize{{{}}}{{{}}}\\selectfont$\\displaystyle {latex}$}};\n",
                    number(cx),
                    number(cy),
                    number(size),
                    number(size * 1.2),
                ));
            }
            _ => {
                out.push_str(&self.shape(element, &rotate));
                if element.kind == "text" || has_text(&element.content) {
                    out.push_str(&self.text(element, &node_rotate, cx, cy));
                }
            }
        }
        out
    }

    fn shape(&mut self, element: &SlideElement, rotate: &str) -> String {
        let fill = self.colors.name(&element.fill);
        let stroke = self.colors.name(&element.stroke);
        let is_line = matches!(element.shape.as_str(), "line" | "arrow");
        if fill.is_none() && stroke.is_none() && !is_line {
            return String::new();
        }
        let mut options: Vec<String> = Vec::new();
        if let Some(fill) = &fill {
            if !is_line {
                options.push(format!("fill={fill}"));
            }
        }
        match &stroke {
            Some(stroke) => {
                options.push(format!("draw={stroke}"));
                options.push(format!(
                    "line width={}pt",
                    number(element.stroke_width.max(0.1))
                ));
            }
            None if is_line => {
                options.push("draw".into());
                options.push(format!(
                    "line width={}pt",
                    number(element.stroke_width.max(0.1))
                ));
            }
            None => {}
        }
        if element.shape == "arrow" {
            options.push("-{Stealth[length=3mm]}".into());
        }
        let options = options.join(", ");
        let (x, y, w, h) = (element.x, element.y, element.w, element.h);
        let path = match element.shape.as_str() {
            "ellipse" => format!(
                "({},{}) ellipse [x radius={}, y radius={}]",
                number(x + w / 2.0),
                number(y + h / 2.0),
                number(w / 2.0),
                number(h / 2.0)
            ),
            "triangle" => format!(
                "({},{}) -- ({},{}) -- ({},{}) -- cycle",
                number(x + w / 2.0),
                number(y),
                number(x + w),
                number(y + h),
                number(x),
                number(y + h)
            ),
            "diamond" => format!(
                "({},{}) -- ({},{}) -- ({},{}) -- ({},{}) -- cycle",
                number(x + w / 2.0),
                number(y),
                number(x + w),
                number(y + h / 2.0),
                number(x + w / 2.0),
                number(y + h),
                number(x),
                number(y + h / 2.0)
            ),
            "line" | "arrow" => {
                let (x1, x2) = if element.flip_h {
                    (x + w, x)
                } else {
                    (x, x + w)
                };
                let (y1, y2) = if element.flip_v {
                    (y + h, y)
                } else {
                    (y, y + h)
                };
                format!(
                    "({},{}) -- ({},{})",
                    number(x1),
                    number(y1),
                    number(x2),
                    number(y2)
                )
            }
            "roundRect" => format!(
                "[rounded corners={}mm] ({},{}) rectangle ({},{})",
                number((w.min(h) * 0.15).max(0.5)),
                number(x),
                number(y),
                number(x + w),
                number(y + h)
            ),
            _ => format!(
                "({},{}) rectangle ({},{})",
                number(x),
                number(y),
                number(x + w),
                number(y + h)
            ),
        };
        format!("  \\path[{options}{rotate}] {path};\n")
    }

    fn text(&mut self, element: &SlideElement, node_rotate: &str, cx: f64, cy: f64) -> String {
        let blocks = children_of(&element.content);
        let body = self.exporter.blocks(&blocks);
        let body = body.trim();
        if body.is_empty() {
            return String::new();
        }
        let padding = element.padding;
        let width = (element.w - 2.0 * padding).max(1.0);
        let height = (element.h - 2.0 * padding).max(1.0);
        let position = match element.vertical_align.as_str() {
            "middle" => "c",
            "bottom" => "b",
            _ => "t",
        };
        let color = self
            .colors
            .name(&element.color)
            .map(|name| format!("\\color{{{name}}}"))
            .unwrap_or_default();
        let size = element.font_size;
        format!(
            "  \\node[anchor=center, inner sep=0pt{node_rotate}] at ({},{}) {{\\begin{{minipage}}[c][{}mm][{position}]{{{}mm}}\\fontsize{{{}}}{{{}}}\\selectfont{color}\\raggedright\n{body}\n\\end{{minipage}}}};\n",
            number(cx),
            number(cy),
            number(height),
            number(width),
            number(size),
            number(size * 1.2),
        )
    }

    /// Bilddatei für `\includegraphics` (eingebettete Daten werden als Datei abgelegt).
    fn image_file(&mut self, src: &str) -> Option<String> {
        let (mime, base64_data) = super::export::parse_data_url(src)?;
        let asset = if mime == "image/svg+xml" {
            use base64::Engine;
            let svg = base64::engine::general_purpose::STANDARD
                .decode(base64_data.as_bytes())
                .ok()?;
            let pdf = svg_to_pdf(&svg)?;
            EmbeddedAsset {
                path: format!("Abbildungen/eingebettet-{}.pdf", content_hash(&pdf)),
                mime: "application/pdf".into(),
                base64: base64::engine::general_purpose::STANDARD.encode(&pdf),
            }
        } else {
            embedded_asset(src)?
        };
        let path = asset.path.clone();
        if !self.assets.iter().any(|existing| existing.path == path) {
            self.assets.push(asset);
        }
        self.exporter.usage.graphics = true;
        Some(path)
    }
}

/// SVG → PDF (für LaTeX); `None`, wenn die Daten nicht lesbar sind.
fn svg_to_pdf(svg: &[u8]) -> Option<Vec<u8>> {
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = svg2pdf::usvg::Tree::from_data(svg, &options).ok()?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .ok()
}

fn children_of(content: &Value) -> Vec<Value> {
    content
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn has_text(content: &Value) -> bool {
    fn visit(node: &Value) -> bool {
        node.get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty())
            || node.get("type").and_then(Value::as_str) == Some("inlineMath")
            || node
                .get("content")
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().any(visit))
    }
    visit(content)
}

/// Präsentation → vollständiges Beamer-Dokument (+ eingebettete Bilder als Dateien).
pub fn deck_to_latex(deck: &SlideDeck) -> SlidesLatex {
    let settings = DocumentSettings {
        language: deck.language.clone(),
        ..DocumentSettings::default()
    };
    let mut export = SlideExport {
        exporter: Exporter::new(&settings, false),
        colors: Colors { names: Vec::new() },
        assets: Vec::new(),
        warnings: Vec::new(),
    };
    let (width, height) = slide_size(&deck.aspect);
    let mut frames: Vec<String> = Vec::new();
    let visible: Vec<&Slide> = deck.slides.iter().filter(|slide| !slide.hidden).collect();
    for (index, slide) in visible.iter().enumerate() {
        let number_label = index + 1;
        let mut body = String::new();
        for element in &slide.elements {
            body.push_str(&export.element(element, number_label));
        }
        let mut frame = format!("% Folie {number_label}\n");
        let background = export.colors.name(&slide.background);
        if let Some(name) = &background {
            frame.push_str(&format!(
                "{{\\setbeamercolor{{background canvas}}{{bg={name}}}\n"
            ));
        }
        frame.push_str("\\begin{frame}[plain]\n");
        if !body.is_empty() {
            frame.push_str(&format!(
                "\\begin{{textblock*}}{{{w}mm}}(0mm,0mm)\n\\begin{{tikzpicture}}[x=1mm, y=-1mm, inner sep=0pt, outer sep=0pt]\n  \\useasboundingbox (0,0) rectangle ({w},{h});\n{body}\\end{{tikzpicture}}\n\\end{{textblock*}}\n",
                w = number(width),
                h = number(height),
            ));
        } else {
            frame.push_str("\\mbox{}\n");
        }
        let notes = slide.notes.trim();
        if !notes.is_empty() {
            frame.push_str(&format!("\\note{{{}}}\n", escape_latex(notes)));
        }
        frame.push_str("\\end{frame}\n");
        if background.is_some() {
            frame.push_str("}\n");
        }
        frames.push(frame);
    }

    let usage = export.exporter.usage.clone();
    let mut lines: Vec<String> = vec![
        "%% Erzeugt mit dem VisuTeX-Folien-Editor (LaTeX-Beamer) – kompilierbar mit pdfLaTeX, XeLaTeX und LuaLaTeX.".into(),
        format!(
            "\\documentclass[aspectratio={},xcolor={{dvipsnames,table}}]{{beamer}}",
            if deck.aspect == "4:3" { "43" } else { "169" }
        ),
    ];
    // fontspec soll die Formelschriften nicht ersetzen (sonst OT1/cmr-„legacymaths“).
    lines.push("\\PassOptionsToPackage{no-math}{fontspec}".into());
    lines.extend(preamble::engine_font_lines(&settings, &usage));
    lines.push(super::languages::babel_line(&deck.language));
    lines.extend(slide_packages(&usage));
    lines.push("\\usepackage{tikz}".into());
    lines.push("\\usetikzlibrary{arrows.meta,babel}".into());
    lines.push("\\usepackage[absolute,overlay]{textpos}".into());
    lines.push("\\setlength{\\TPHorizModule}{1mm}".into());
    lines.push("\\setlength{\\TPVertModule}{1mm}".into());
    lines.push("\\setbeamertemplate{navigation symbols}{}".into());
    // Formeln in Latin Modern statt serifenloser cmss-Mathematik: Beamer unterlässt
    // dann die Schriftersetzung; die LM-Schriften sind stufenlos skalierbar und
    // überall (auch im mitgelieferten Bundle) als Type1 vorhanden.
    lines.extend(
        [
            "\\renewcommand{\\mathfamilydefault}{cmr}",
            "\\DeclareSymbolFont{operators}{OT1}{lmr}{m}{n}",
            "\\SetSymbolFont{operators}{bold}{OT1}{lmr}{bx}{n}",
            "\\DeclareSymbolFont{letters}{OML}{lmm}{m}{it}",
            "\\SetSymbolFont{letters}{bold}{OML}{lmm}{b}{it}",
            "\\DeclareSymbolFont{symbols}{OMS}{lmsy}{m}{n}",
            "\\SetSymbolFont{symbols}{bold}{OMS}{lmsy}{b}{n}",
            "\\DeclareSymbolFont{largesymbols}{OMX}{lmex}{m}{n}",
            "\\SetSymbolFont{largesymbols}{bold}{OMX}{lmex}{m}{n}",
            // Aufzählungspunkte als Kreise (wie im Editor), von PGF gezeichnet
            "\\setbeamertemplate{itemize items}[circle]",
        ]
        .map(String::from),
    );
    if deck.theme.font == "serif" {
        lines.push("\\renewcommand{\\familydefault}{\\rmdefault}".into());
    }
    let mut theme_colors: Vec<String> = Vec::new();
    for (name, color, target) in [
        (
            "vtxtext",
            &deck.theme.text_color,
            "\\setbeamercolor{normal text}{fg=vtxtext}",
        ),
        (
            "vtxaccent",
            &deck.theme.accent,
            "\\setbeamercolor{structure}{fg=vtxaccent}",
        ),
        (
            "vtxbackground",
            &deck.theme.background,
            "\\setbeamercolor{background canvas}{bg=vtxbackground}",
        ),
    ] {
        if let Some(value) = hex(color) {
            theme_colors.push(format!("\\definecolor{{{name}}}{{HTML}}{{{value}}}"));
            theme_colors.push(target.into());
        }
    }
    lines.extend(theme_colors);
    for (name, value) in &export.colors.names {
        lines.push(format!("\\definecolor{{{name}}}{{HTML}}{{{value}}}"));
    }
    for (command, value) in [
        ("title", &deck.title),
        ("subtitle", &deck.subtitle),
        ("author", &deck.author),
        ("date", &deck.date),
    ] {
        if !value.trim().is_empty() {
            lines.push(format!("\\{command}{{{}}}", escape_latex(value.trim())));
        }
    }
    let mut latex = lines.join("\n");
    latex.push_str("\n\n\\begin{document}\n\n");
    if frames.is_empty() {
        latex.push_str("\\begin{frame}[plain]\n\\mbox{}\n\\end{frame}\n");
    } else {
        latex.push_str(&frames.join("\n"));
    }
    latex.push_str("\n\\end{document}\n");
    let mut warnings = export.warnings;
    warnings.extend(export.exporter.warnings);
    SlidesLatex {
        latex,
        assets: export.assets,
        warnings,
    }
}

/// Zusatzpakete für Inhalte der Textfelder (Beamer lädt xcolor, hyperref, graphicx
/// und amsmath bereits selbst).
fn slide_packages(usage: &Usage) -> Vec<String> {
    let mut lines = vec!["\\usepackage{amssymb}".to_string()];
    if usage.underline {
        lines.push("\\usepackage[normalem]{ulem}".into());
    }
    if usage.line_spacing {
        lines.push("\\usepackage{setspace}".into());
    }
    for package in preamble::required_packages(usage) {
        if matches!(
            package,
            "amsmath"
                | "graphicx"
                | "xcolor"
                | "ulem"
                | "hyperref"
                | "setspace"
                | "float"
                | "tikz"
                | "natbib"
                | "pdflscape"
        ) {
            continue;
        }
        lines.push(format!("\\usepackage{{{package}}}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(text: &str) -> Value {
        json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] })
    }

    fn deck() -> SlideDeck {
        SlideDeck {
            title: "Vortrag & Test".into(),
            slides: vec![
                Slide {
                    id: "s1".into(),
                    notes: "Begrüßung".into(),
                    elements: vec![
                        SlideElement {
                            id: "e1".into(),
                            kind: "text".into(),
                            content: text("Titel mit 50 % Anteil"),
                            font_size: 36.0,
                            ..SlideElement::default()
                        },
                        SlideElement {
                            id: "e2".into(),
                            kind: "shape".into(),
                            shape: "ellipse".into(),
                            fill: "#ff0000".into(),
                            rotation: 30.0,
                            ..SlideElement::default()
                        },
                        SlideElement {
                            id: "e3".into(),
                            kind: "formula".into(),
                            latex: "\\int_0^1 x\\,dx".into(),
                            ..SlideElement::default()
                        },
                    ],
                    ..Slide::default()
                },
                Slide {
                    id: "s2".into(),
                    background: "#000000".into(),
                    elements: vec![SlideElement {
                        id: "e4".into(),
                        kind: "shape".into(),
                        shape: "arrow".into(),
                        stroke: "#ffffff".into(),
                        flip_h: true,
                        ..SlideElement::default()
                    }],
                    ..Slide::default()
                },
                Slide {
                    id: "s3".into(),
                    hidden: true,
                    ..Slide::default()
                },
            ],
            ..SlideDeck::default()
        }
    }

    #[test]
    fn exports_beamer_document() {
        let result = deck_to_latex(&deck());
        let latex = &result.latex;
        assert!(
            latex.contains("\\documentclass[aspectratio=169,xcolor={dvipsnames,table}]{beamer}")
        );
        assert!(latex.contains("\\usepackage[absolute,overlay]{textpos}"));
        assert!(latex.contains("\\title{Vortrag \\& Test}"));
        assert!(latex.contains("Titel mit 50 \\% Anteil"));
        assert!(latex.contains("\\fontsize{36}{43.2}\\selectfont"));
        assert!(latex.contains("ellipse [x radius=30, y radius=10]"));
        assert!(latex.contains("rotate around={-30:(40,20)}"));
        assert!(latex.contains("$\\displaystyle \\int_0^1 x\\,dx$"));
        assert!(latex.contains("\\note{Begrüßung}"));
        // Pfeil von rechts oben nach links unten, Folienhintergrund schwarz
        assert!(latex.contains("-{Stealth[length=3mm]}] (70,10) -- (10,30);"));
        assert!(latex.contains("{\\setbeamercolor{background canvas}{bg=vtxc"));
        // ausgeblendete Folie fehlt
        assert_eq!(latex.matches("\\begin{frame}").count(), 2);
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        assert_eq!(latex.matches('{').count(), latex.matches('}').count());
    }

    #[test]
    fn parses_and_sanitizes_deck_files() {
        let mut value = serde_json::to_value(deck()).unwrap();
        value["slides"][0]["elements"][0]["fontSize"] = json!(1000);
        value["aspect"] = json!("21:9");
        let parsed = parse_deck(&value.to_string()).unwrap();
        assert_eq!(parsed.aspect, "16:9");
        assert_eq!(parsed.slides[0].elements[0].font_size, 200.0);
        assert!(parse_deck("{\"format\":\"visutex-project\"}").is_err());
        assert!(parse_deck("kein json").is_err());
        // Minimaldatei: alles andere mit Standardwerten
        let minimal = parse_deck("{\"format\":\"visutex-slides\"}").unwrap();
        assert_eq!(minimal.grid.size, 5.0);
        assert!(minimal.slides.is_empty());
    }

    #[test]
    fn embeds_images_as_files() {
        let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let deck = SlideDeck {
            slides: vec![Slide {
                elements: vec![SlideElement {
                    kind: "image".into(),
                    src: png.into(),
                    ..SlideElement::default()
                }],
                ..Slide::default()
            }],
            ..SlideDeck::default()
        };
        let result = deck_to_latex(&deck);
        assert_eq!(result.assets.len(), 1);
        assert!(result.latex.contains(&format!(
            "\\includegraphics[width=60mm,height=20mm]{{{}}}",
            result.assets[0].path
        )));
    }
}
