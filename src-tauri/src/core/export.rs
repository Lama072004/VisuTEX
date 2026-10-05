//! Export des Tiptap-Dokuments (ProseMirror-JSON) nach LaTeX.
//!
//! Ergebnis ist immer ein vollständiges, eigenständig kompilierbares Dokument
//! (Präambel + Körper). Strukturen ohne eindeutige LaTeX-Form werden mit
//! Kommentar-Markern umschlossen (siehe `markers`).

use super::escape::{escape_latex, escape_url, is_valid_key};
use super::markers;
use super::preamble::{self, bibliography_style, paragraph_spacing_declaration, Usage};
use super::settings::{is_safe_relative_path, DocumentSettings};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedAsset {
    pub path: String,
    pub mime: String,
    pub base64: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub latex: String,
    pub preamble: String,
    pub generated_preamble: String,
    pub body: String,
    pub assets: Vec<EmbeddedAsset>,
    pub warnings: Vec<String>,
    pub missing_packages: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InlineContext {
    Paragraph,
    Heading,
    Cell,
}

// ---------------------------------------------------------------- JSON-Helfer

pub fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}

pub fn children(node: &Value) -> &[Value] {
    node.get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub fn attr<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    node.get("attrs")?.get(key)
}

pub fn attr_str(node: &Value, key: &str) -> String {
    match attr(node, key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        _ => String::new(),
    }
}

pub fn attr_f64(node: &Value, key: &str) -> Option<f64> {
    match attr(node, key)? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

pub fn attr_bool(node: &Value, key: &str) -> Option<bool> {
    match attr(node, key)? {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => Some(text == "true"),
        _ => None,
    }
}

pub fn text_content(node: &Value) -> String {
    if node_type(node) == "text" {
        return node
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    children(node).iter().map(text_content).collect()
}

pub fn css_color_to_hex(color: &str) -> Option<String> {
    let color = color.trim();
    let hex = color.trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(hex.to_ascii_uppercase());
    }
    if color.starts_with('#') && hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(
            hex.chars()
                .flat_map(|c| [c, c])
                .collect::<String>()
                .to_ascii_uppercase(),
        );
    }
    let inner = color
        .strip_prefix("rgba(")
        .or_else(|| color.strip_prefix("rgb("))?;
    let channels: Vec<u8> = inner
        .trim_end_matches(')')
        .split(',')
        .take(3)
        .filter_map(|part| part.trim().parse::<f64>().ok())
        .map(|value| value.clamp(0.0, 255.0) as u8)
        .collect();
    (channels.len() == 3)
        .then(|| format!("{:02X}{:02X}{:02X}", channels[0], channels[1], channels[2]))
}

fn font_size_points(value: &str) -> Option<f64> {
    let value = value.trim();
    let (number, factor) = if let Some(points) = value.strip_suffix("pt") {
        (points, 1.0)
    } else if let Some(pixels) = value.strip_suffix("px") {
        (pixels, 0.75)
    } else {
        (value, 1.0)
    };
    let size = number.trim().parse::<f64>().ok()? * factor;
    (4.0..=200.0)
        .contains(&size)
        .then(|| (size * 10.0).round() / 10.0)
}

fn format_number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{rounded}")
}

/// Verbindet Zeilen mit `\\`. Beginnt eine Folgezeile mit `[` oder `*`, wird
/// `{}` eingefügt – sonst liest LaTeX `\\[…]` als Abstandsangabe bzw. `\\*`.
pub fn join_latex_lines(lines: &[String]) -> String {
    let mut result = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            result.push_str("\\\\");
            if line.trim_start().starts_with(['[', '*']) {
                result.push_str("{}");
            }
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

/// Fingerabdruck der erzeugten Ausgabe eines Blocks (siehe `Exporter::block`).
pub fn source_key(generated: &str) -> String {
    content_hash(generated.as_bytes())
}

/// Schlüssel der in `text` definierten Abkürzungen (`\acro`, `\acrodef`, `\newacro`).
fn defined_acronym_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for command in ["\\acro{", "\\acrodef{", "\\newacro{"] {
        for (start, _) in text.match_indices(command) {
            let rest = &text[start + command.len()..];
            if let Some(end) = rest.find('}') {
                keys.push(rest[..end].trim().to_string());
            }
        }
    }
    keys
}

/// Originalcode eines importierten Blocks, sofern der Block unverändert ist.
fn preserved_source<'n>(node: &'n Value, generated: &str) -> Option<&'n str> {
    let source = attr(node, "sourceLatex")
        .and_then(Value::as_str)
        .filter(|source| !source.trim().is_empty())?;
    let key = attr(node, "sourceKey").and_then(Value::as_str)?;
    (key == source_key(generated)).then_some(source)
}

/// Folgt `next` im Original ohne Leerzeile auf `previous` (z. B. Text nach einer
/// abgesetzten Formel)? Zwei Absätze werden nie verbunden.
fn joins_tight(previous: &Value, next: &Value) -> bool {
    attr_bool(next, "sourceTight") == Some(true)
        && !(node_type(previous) == "paragraph" && node_type(next) == "paragraph")
}

/// Setzt nach dem Import für alle Blöcke mit Originalcode den Fingerabdruck der
/// erzeugten Ausgabe – mit denselben Einstellungen und Vorlauf wie beim Export.
/// Blöcke, deren erzeugte Ausgabe ohnehin dem Original entspricht, verlieren den
/// (überflüssigen) Originalcode.
pub fn assign_source_keys(doc: &mut Value, settings: &DocumentSettings, has_chapters: bool) {
    let mut exporter = Exporter::new(settings, has_chapters);
    for node in children(doc) {
        exporter.scan(node);
    }
    fn visit(exporter: &mut Exporter, node: &mut Value) {
        if let Some(content) = node.get_mut("content").and_then(Value::as_array_mut) {
            for child in content.iter_mut() {
                visit(exporter, child);
            }
        }
        let Some(source) = attr(node, "sourceLatex")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            return;
        };
        let generated = exporter.generate_block(node);
        let Some(attrs) = node.get_mut("attrs").and_then(Value::as_object_mut) else {
            return;
        };
        if generated.trim_end() == source.trim_end() {
            attrs.remove("sourceLatex");
            attrs.remove("sourceKey");
        } else {
            attrs.insert("sourceKey".into(), Value::String(source_key(&generated)));
        }
    }
    if let Some(content) = doc.get_mut("content").and_then(Value::as_array_mut) {
        for node in content.iter_mut() {
            visit(&mut exporter, node);
        }
    }
}

/// Zitierbefehle (natbib und biblatex), die ein Zitat-Knoten tragen kann.
pub const CITE_COMMANDS: &[&str] = &[
    "cite",
    "citep",
    "citet",
    "parencite",
    "textcite",
    "autocite",
    "footcite",
    "Cite",
    "Parencite",
    "Textcite",
    "Autocite",
    "citeauthor",
    "citeyear",
    "fullcite",
];

/// siunitx-Befehle eines Größen-Knotens.
pub const QUANTITY_COMMANDS: &[&str] = &[
    "SI", "qty", "si", "unit", "num", "ang", "SIrange", "qtyrange", "numrange",
];

/// `\SI{1}{\mega\ohm}`, `\qty{…}{…}`, `\num{…}`, … eines Größen-Knotens.
/// Zusatz der Bildoptionen: Bilder höchstens 90 % der Satzspiegelhöhe (der Import erkennt ihn).
pub const IMAGE_HEIGHT_LIMIT: &str = ",height=0.9\\textheight,keepaspectratio";

/// Zahl im Sinne von siunitx: Ziffern, Vorzeichen, Dezimalzeichen (`.`, `,`, `{,}`),
/// Exponent (`e`/`d`), Unsicherheit (`(5)`, `\pm`/`±`), Produkte (`x`, `\times`),
/// komplexe Einheit (`i`/`j`). Text wie „test“ ist keine Zahl.
pub fn is_siunitx_number(value: &str) -> bool {
    let value = value
        .replace("\\pm", "±")
        .replace("\\times", "x")
        .replace("{,}", ",");
    let value = value.trim();
    if value.is_empty() || !value.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    value.chars().all(|c| {
        c.is_ascii_digit()
            || c.is_whitespace()
            || matches!(
                c,
                '.' | ',' | '+' | '-' | '±' | '(' | ')' | 'e' | 'E' | 'd' | 'D' | 'x' | 'i' | 'j'
            )
    })
}

fn quantity_latex(node: &Value) -> String {
    let command = attr_str(node, "command");
    let command = if QUANTITY_COMMANDS.contains(&command.as_str()) {
        command
    } else {
        "SI".to_string()
    };
    let argument = |key: &str| {
        let value = attr_str(node, key);
        let value = value.trim();
        if is_balanced(value) && !value.contains('%') {
            value.to_string()
        } else {
            escape_latex(value)
        }
    };
    let options = attr_str(node, "options");
    let options = options.trim();
    let options = if !options.is_empty() && is_balanced(options) {
        format!("[{options}]")
    } else {
        String::new()
    };
    let (value, value2, unit) = (argument("value"), argument("value2"), argument("unit"));
    // siunitx bricht bei Text statt Zahl das gesamte Kompilieren ab („Invalid number“):
    // dann Wert als Text und nur die Einheit über siunitx setzen.
    let numeric = |text: &str| is_siunitx_number(text);
    let invalid = match command.as_str() {
        "si" | "unit" => false,
        "ang" => !value
            .split(';')
            .all(|part| part.trim().is_empty() || numeric(part)),
        "numrange" | "SIrange" | "qtyrange" => !numeric(&value) || !numeric(&value2),
        _ => !numeric(&value),
    };
    if invalid {
        let text = |raw: &str| escape_latex(&attr_str(node, raw));
        let unit_command = if command == "SI" || command == "SIrange" {
            "si"
        } else {
            "unit"
        };
        let unit_part = if unit.is_empty() || command == "num" || command == "numrange" {
            String::new()
        } else {
            format!("~\\{unit_command}{{{unit}}}")
        };
        return match command.as_str() {
            "numrange" | "SIrange" | "qtyrange" => {
                format!("{}--{}{unit_part}", text("value"), text("value2"))
            }
            _ => format!("{}{unit_part}", text("value")),
        };
    }
    match command.as_str() {
        "si" | "unit" => format!("\\{command}{options}{{{unit}}}"),
        "num" | "ang" => format!("\\{command}{options}{{{value}}}"),
        "numrange" => format!("\\{command}{options}{{{value}}}{{{value2}}}"),
        "SIrange" | "qtyrange" => {
            format!("\\{command}{options}{{{value}}}{{{value2}}}{{{unit}}}")
        }
        _ => format!("\\{command}{options}{{{value}}}{{{unit}}}"),
    }
}

/// Schriftgrößen-Deklarationen, die in Gleitumgebungen übernommen werden.
pub const FONT_SIZE_COMMANDS: &[&str] = &[
    "tiny",
    "scriptsize",
    "footnotesize",
    "small",
    "normalsize",
    "large",
    "Large",
];

/// Spaltenarten einer tabular-Spaltendefinition (`l`, `c`, `r`, `p`, `X`, …),
/// None bei unbekannter Syntax.
pub fn spec_column_kinds(spec: &str) -> Option<Vec<char>> {
    let chars: Vec<char> = spec.chars().collect();
    let mut kinds = Vec::new();
    let mut index = 0;
    // Länge einer {…}-Gruppe ab `start` (muss mit `{` beginnen).
    let group_end = |start: usize| -> Option<usize> {
        let mut depth = 0;
        for (offset, character) in chars.iter().enumerate().skip(start) {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(offset + 1);
                    }
                }
                _ if depth == 0 => return None,
                _ => {}
            }
        }
        None
    };
    while index < chars.len() {
        match chars[index] {
            ' ' | '\t' | '\n' | '|' => index += 1,
            '>' | '<' | '@' | '!' => {
                let mut start = index + 1;
                while chars.get(start) == Some(&' ') {
                    start += 1;
                }
                index = group_end(start)?;
            }
            '*' => {
                let count_end = group_end(index + 1)?;
                let count: String = chars[index + 2..count_end - 1].iter().collect();
                let repeated_end = group_end(count_end)?;
                let repeated: String = chars[count_end + 1..repeated_end - 1].iter().collect();
                let inner = spec_column_kinds(&repeated)?;
                for _ in 0..count.trim().parse::<usize>().ok()?.min(64) {
                    kinds.extend(inner.iter().copied());
                }
                index = repeated_end;
            }
            'p' | 'm' | 'b' => {
                index = group_end(index + 1)?;
                kinds.push('p');
            }
            'S' => {
                // siunitx-Spalte mit optionalen Einstellungen
                index += 1;
                if chars.get(index) == Some(&'[') {
                    let close = chars[index..].iter().position(|c| *c == ']')?;
                    index += close + 1;
                }
                kinds.push('S');
            }
            letter if letter.is_ascii_alphabetic() => {
                kinds.push(letter);
                index += 1;
            }
            _ => return None,
        }
    }
    Some(kinds)
}

/// Beschriftung als LaTeX: Klartext wird maskiert, übernommene LaTeX-Beschriftungen
/// (mit Formeln, Einheiten …) bleiben unverändert.
/// Standardbreite je Unterabbildung (Anteil der Zeilenbreite, Platz für `\hfill`).
pub fn subfigure_default_share(count: usize) -> f64 {
    ((0.96 / count.max(1) as f64) * 100.0).floor() / 100.0
}

fn caption_latex(node: &Value, key: &str) -> String {
    let text = attr_str(node, key);
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else if attr_bool(node, "captionLatex") == Some(true) && is_balanced(text) {
        text.to_string()
    } else {
        escape_latex(text)
    }
}

/// Teilt eine Optionsliste `a=1, b={x,y}` an Kommas der obersten Ebene.
pub fn split_options(options: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for character in options.chars() {
        match character {
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Wert einer Option `key=value` (Klammern um den Wert entfernt).
pub fn option_value(options: &str, key: &str) -> Option<String> {
    split_options(options).into_iter().find_map(|part| {
        let (name, value) = part.split_once('=')?;
        (name.trim() == key).then(|| {
            let value = value.trim();
            value
                .strip_prefix('{')
                .and_then(|inner| inner.strip_suffix('}'))
                .unwrap_or(value)
                .to_string()
        })
    })
}

/// Setzt bzw. entfernt (`value` leer) eine Option in einer Optionsliste.
fn set_option(options: &str, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = split_options(options)
        .into_iter()
        .filter(|part| part.split('=').next().map(str::trim) != Some(key))
        .collect();
    if !value.is_empty() {
        let value = if value.contains([',', '=']) {
            format!("{{{value}}}")
        } else {
            value.to_string()
        };
        parts.insert(0, format!("{key}={value}"));
    }
    parts.join(", ")
}

/// `\vspace{…}`, `\medskip`, … eines Abstandsblocks.
fn vertical_space_latex(node: &Value) -> String {
    let command = attr_str(node, "command");
    match command.as_str() {
        "medskip" | "bigskip" | "smallskip" | "vfill" => format!("\\{command}"),
        _ => {
            let size = attr_str(node, "size");
            let size = size.trim();
            let size = if size.is_empty() || !is_balanced(size) || size.contains("\\par") {
                "1em"
            } else {
                size
            };
            let star = if command == "vspace*" { "*" } else { "" };
            format!("\\vspace{star}{{{size}}}")
        }
    }
}

/// Klammern `{}`/`[]` ausgeglichen und keine Leerzeile (für übernommene Optionen).
pub fn is_balanced(text: &str) -> bool {
    let mut braces = 0i32;
    let mut brackets = 0i32;
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '{' => braces += 1,
            '}' => braces -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            _ => {}
        }
        if braces < 0 || brackets < 0 {
            return false;
        }
    }
    braces == 0 && brackets == 0 && !text.contains("\n\n")
}

/// Gültiger Umgebungsname (Buchstaben, optional `*`).
pub fn is_environment_name(name: &str) -> bool {
    let base = name.strip_suffix('*').unwrap_or(name);
    !base.is_empty()
        && base.chars().all(|c| c.is_ascii_alphabetic())
        && !matches!(base, "document" | "verbatim" | "lstlisting" | "comment")
}

/// Endet `latex` mit einem Befehlswort wie `\noindent` (nicht `\\` oder `\%`)?
fn ends_with_control_word(latex: &str) -> bool {
    let letters = latex
        .chars()
        .rev()
        .take_while(char::is_ascii_alphabetic)
        .count();
    if letters == 0 {
        return false;
    }
    let before = &latex[..latex.len() - letters];
    let backslashes = before.chars().rev().take_while(|c| *c == '\\').count();
    backslashes % 2 == 1
}

pub fn content_hash(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn parse_data_url(src: &str) -> Option<(String, String)> {
    let rest = src.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    if !meta.ends_with(";base64") {
        return None;
    }
    let mime = meta.split(';').next()?.to_ascii_lowercase();
    Some((mime, data.chars().filter(|c| !c.is_whitespace()).collect()))
}

/// Pfad, unter dem ein eingebettetes Bild (data-URL) als Datei abgelegt wird.
pub fn embedded_asset(src: &str) -> Option<EmbeddedAsset> {
    let (mime, base64) = parse_data_url(src)?;
    let extension = match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "application/pdf" => "pdf",
        _ => return None,
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64.as_bytes())
        .ok()?;
    Some(EmbeddedAsset {
        path: format!(
            "Abbildungen/eingebettet-{}.{extension}",
            content_hash(&bytes)
        ),
        mime,
        base64,
    })
}

const HEADINGS_REPORT: [&str; 6] = [
    "chapter",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
];
const HEADINGS_ARTICLE: [&str; 6] = [
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
    "subparagraph",
];

/// Gliederungsebenen für `\addcontentsline{toc}{…}`.
pub const SECTION_LEVELS: &[&str] = &[
    "part",
    "chapter",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
];

pub fn heading_command(level: usize, has_chapters: bool) -> &'static str {
    let commands = if has_chapters {
        HEADINGS_REPORT
    } else {
        HEADINGS_ARTICLE
    };
    commands[level.clamp(1, 6) - 1]
}

pub fn class_has_chapters(preamble: &str) -> bool {
    let text = preamble::strip_comments(preamble);
    let Some(index) = text.find("\\documentclass") else {
        return false;
    };
    let rest = &text[index + "\\documentclass".len()..];
    let rest = match rest.trim_start().strip_prefix('[') {
        Some(options) => options
            .split_once(']')
            .map(|(_, after)| after)
            .unwrap_or(""),
        None => rest,
    };
    let name = rest
        .trim_start()
        .strip_prefix('{')
        .and_then(|inner| inner.split_once('}'))
        .map(|(name, _)| name.trim());
    matches!(
        name,
        Some("scrreprt" | "scrbook" | "report" | "book" | "memoir")
    )
}

struct PseudoMark {
    key: String,
    open: String,
    close: &'static str,
}

pub struct Exporter<'a> {
    settings: &'a DocumentSettings,
    has_chapters: bool,
    pub usage: Usage,
    pub warnings: Vec<String>,
    pub assets: Vec<EmbeddedAsset>,
    acronyms: Vec<(String, String, String)>,
    /// Abkürzungen, die bereits im übernommenen Originalcode/der Präambel definiert sind.
    pub predefined_acronyms: Vec<String>,
    has_acronym_list: bool,
    has_bibliography_block: bool,
    columns: u8,
    landscape: bool,
    layout_started: bool,
    enumerate_depth: usize,
    /// Die (eigene) Präambel lädt biblatex → `\printbibliography` statt BibTeX-Befehlen.
    pub biblatex: bool,
}

impl<'a> Exporter<'a> {
    pub fn new(settings: &'a DocumentSettings, has_chapters: bool) -> Self {
        Self {
            settings,
            has_chapters,
            usage: Usage::default(),
            warnings: Vec::new(),
            assets: Vec::new(),
            acronyms: Vec::new(),
            predefined_acronyms: Vec::new(),
            has_acronym_list: false,
            has_bibliography_block: false,
            columns: 1,
            landscape: false,
            layout_started: false,
            enumerate_depth: 0,
            biblatex: false,
        }
    }

    /// Pakete, die Roh-LaTeX bzw. unverändert übernommener Originalcode benötigt.
    fn note_raw_usage(&mut self, raw: &str) {
        let has = |needles: &[&str]| needles.iter().any(|needle| raw.contains(needle));
        if has(&["\\begin{tikzpicture}"]) {
            self.usage.tikz = true;
        }
        if has(&["\\begin{circuitikz}"]) {
            self.usage.circuitikz = true;
        }
        if has(&["\\multirow"]) {
            self.usage.multirow = true;
        }
        if has(&[
            "\\ac{",
            "\\acs{",
            "\\acl{",
            "\\acf{",
            "\\acp{",
            "\\acro{",
            "\\acrodef{",
        ]) {
            self.usage.acronyms = true;
        }
        if has(&[
            "\\cite{", "\\citep{", "\\citet{", "\\cite[", "\\citep[", "\\citet[",
        ]) {
            self.usage.citations = true;
        }
        if has(&["\\citep", "\\citet"]) {
            self.usage.natbib = true;
        }
        if has(&[
            "\\bibliography{",
            "\\begin{thebibliography}",
            "\\printbibliography",
            "\\bibitem",
        ]) {
            self.usage.manual_bibliography = true;
        }
        if has(&["\\toprule", "\\midrule", "\\bottomrule", "\\cmidrule"]) {
            self.usage.booktabs = true;
        }
        if has(&["\\begin{subfigure}", "\\subcaptionbox", "\\subref{"]) {
            self.usage.subcaption = true;
        }
        if has(&[
            "\\SI{", "\\SI[", "\\si{", "\\si[", "\\qty{", "\\qty[", "\\num{", "\\unit{", "\\ang{",
        ]) {
            self.usage.siunitx = true;
        }
        if has(&["\\begin{tcolorbox}"]) {
            self.usage.tcolorbox = true;
        }
        if has(&["\\begin{lstlisting}", "\\lstinline"]) {
            self.usage.listings = true;
        }
        if has(&[
            "\\begin{itemize}[",
            "\\begin{enumerate}[",
            "\\begin{description}[",
        ]) {
            self.usage.enumitem = true;
        }
        if has(&["\\begin{multicols"]) {
            self.usage.multicol = true;
        }
        if has(&["\\includegraphics"]) {
            self.usage.graphics = true;
        }
        if has(&["\\textcolor", "\\color{", "\\color[", "\\colorbox"]) {
            self.usage.colors = true;
        }
        if has(&["\\href", "\\url{", "\\autoref"]) {
            self.usage.links = true;
        }
        if has(&["\\maketitle"]) {
            self.usage.maketitle = true;
        }
        if raw.contains("[H]") {
            self.usage.float_here = true;
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    // ------------------------------------------------------------- Vorlauf

    fn scan(&mut self, node: &Value) {
        if let Some(source) = attr(node, "sourceLatex").and_then(Value::as_str) {
            self.predefined_acronyms
                .extend(defined_acronym_keys(source));
        }
        match node_type(node) {
            "acronym" => {
                let key = attr_str(node, "key");
                if is_valid_key(&key) {
                    self.usage.acronyms = true;
                    let short = Some(attr_str(node, "short"))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| key.clone());
                    let long = Some(attr_str(node, "long"))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| key.clone());
                    match self
                        .acronyms
                        .iter()
                        .find(|(existing, _, _)| *existing == key)
                    {
                        None => self.acronyms.push((key, short, long)),
                        Some((_, existing_short, existing_long))
                            if *existing_short != short || *existing_long != long =>
                        {
                            self.warn(format!(
                                "Abkürzung „{key}“ ist mehrfach unterschiedlich definiert – die erste Definition wird verwendet."
                            ));
                        }
                        _ => {}
                    }
                }
            }
            "citation" => {
                if attr_str(node, "keys")
                    .split(',')
                    .any(|key| is_valid_key(key.trim()))
                {
                    self.usage.citations = true;
                    if matches!(attr_str(node, "command").as_str(), "citep" | "citet") {
                        self.usage.natbib = true;
                    }
                }
            }
            "directoryBlock" => match attr_str(node, "kind").as_str() {
                "acronyms" => {
                    self.has_acronym_list = true;
                    self.usage.acronyms = true;
                }
                "bibliography" => {
                    self.has_bibliography_block = true;
                    self.usage.bibliography = true;
                }
                "figures" => self.usage.list_of_figures = true,
                "tables" => self.usage.list_of_tables = true,
                _ => {}
            },
            "tikzBlock" => {
                if attr_str(node, "environment") == "circuitikz" {
                    self.usage.circuitikz = true;
                } else {
                    self.usage.tikz = true;
                }
            }
            "rawLatexBlock" | "rawLatexInline" => {
                let raw = if node_type(node) == "rawLatexBlock" {
                    attr_str(node, "rawLatex")
                } else {
                    attr_str(node, "latex")
                };
                self.note_raw_usage(&raw);
                self.predefined_acronyms.extend(defined_acronym_keys(&raw));
            }
            "pageBreak" if attr_str(node, "breakType") == "section" => {
                if attr_f64(node, "columns").unwrap_or(1.0) > 1.0 {
                    self.usage.multicol = true;
                }
                if attr_str(node, "orientation") == "landscape"
                    && self.settings.orientation == "portrait"
                {
                    self.usage.landscape_sections = true;
                }
            }
            _ => {}
        }
        if let Some(marks) = node.get("marks").and_then(Value::as_array) {
            for mark in marks {
                if mark.get("type").and_then(Value::as_str) == Some("textStyle")
                    && mark
                        .pointer("/attrs/fontFamily")
                        .and_then(Value::as_str)
                        .is_some_and(|f| !f.is_empty())
                {
                    self.usage.font_helper = true;
                }
            }
        }
        for child in children(node) {
            self.scan(child);
        }
    }

    // -------------------------------------------------------------- Inline

    fn pseudo_marks(&mut self, node: &Value) -> Vec<PseudoMark> {
        if node_type(node) == "hardBreak" {
            return Vec::new();
        }
        let mut marks: Vec<(u8, PseudoMark)> = Vec::new();
        for mark in node
            .get("marks")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let attrs = mark.get("attrs").cloned().unwrap_or(Value::Null);
            let get = |key: &str| {
                attrs
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            match mark.get("type").and_then(Value::as_str).unwrap_or("") {
                "link" => {
                    let href = get("href").trim().to_string();
                    let lower = href.to_ascii_lowercase();
                    if lower.starts_with("http:")
                        || lower.starts_with("https:")
                        || lower.starts_with("mailto:")
                    {
                        marks.push((
                            0,
                            PseudoMark {
                                key: format!("link:{href}"),
                                open: format!("\\href{{{}}}{{", escape_url(&href)),
                                close: "}",
                            },
                        ));
                    }
                }
                "textStyle" => {
                    let family = get("fontFamily");
                    let family = family
                        .trim_matches(|c| c == '\'' || c == '"')
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !family.is_empty()
                        && family
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
                    {
                        self.usage.font_helper = true;
                        marks.push((
                            1,
                            PseudoMark {
                                key: format!("font:{family}"),
                                open: format!("\\visutexfont{{{family}}}{{"),
                                close: "}",
                            },
                        ));
                    }
                    if let Some(size) = font_size_points(&get("fontSize")) {
                        let baseline = (size * 12.0).round() / 10.0;
                        marks.push((
                            2,
                            PseudoMark {
                                key: format!("size:{size}"),
                                open: format!(
                                    "{{\\fontsize{{{}pt}}{{{}pt}}\\selectfont ",
                                    format_number(size),
                                    format_number(baseline)
                                ),
                                close: "}",
                            },
                        ));
                    }
                    if let Some(color) = css_color_to_hex(&get("color")) {
                        marks.push((
                            3,
                            PseudoMark {
                                key: format!("color:{color}"),
                                open: format!("\\textcolor[HTML]{{{color}}}{{"),
                                close: "}",
                            },
                        ));
                    }
                    if let Some(background) = css_color_to_hex(&get("backgroundColor")) {
                        marks.push((
                            4,
                            PseudoMark {
                                key: format!("bg:{background}"),
                                open: format!("\\colorbox[HTML]{{{background}}}{{"),
                                close: "}",
                            },
                        ));
                    }
                }
                "highlight" => {
                    if let Some(background) = css_color_to_hex(&get("color")) {
                        marks.push((
                            4,
                            PseudoMark {
                                key: format!("bg:{background}"),
                                open: format!("\\colorbox[HTML]{{{background}}}{{"),
                                close: "}",
                            },
                        ));
                    }
                }
                "bold" => marks.push((
                    5,
                    PseudoMark {
                        key: "bold".into(),
                        open: "\\textbf{".into(),
                        close: "}",
                    },
                )),
                "italic" => marks.push((
                    6,
                    PseudoMark {
                        key: "italic".into(),
                        open: "\\textit{".into(),
                        close: "}",
                    },
                )),
                "underline" => marks.push((
                    7,
                    PseudoMark {
                        key: "underline".into(),
                        open: "\\uline{".into(),
                        close: "}",
                    },
                )),
                "strike" => marks.push((
                    8,
                    PseudoMark {
                        key: "strike".into(),
                        open: "\\sout{".into(),
                        close: "}",
                    },
                )),
                "superscript" => marks.push((
                    9,
                    PseudoMark {
                        key: "sup".into(),
                        open: "\\textsuperscript{".into(),
                        close: "}",
                    },
                )),
                "subscript" => marks.push((
                    10,
                    PseudoMark {
                        key: "sub".into(),
                        open: "\\textsubscript{".into(),
                        close: "}",
                    },
                )),
                "code" => marks.push((
                    11,
                    PseudoMark {
                        key: "code".into(),
                        open: "\\texttt{".into(),
                        close: "}",
                    },
                )),
                _ => {}
            }
        }
        marks.sort_by_key(|(order, _)| *order);
        for (_, mark) in &marks {
            match mark.key.split(':').next().unwrap_or("") {
                "link" => self.usage.links = true,
                "color" | "bg" => self.usage.colors = true,
                "underline" | "strike" => self.usage.underline = true,
                _ => {}
            }
        }
        marks.into_iter().map(|(_, mark)| mark).collect()
    }

    fn inline_internal(&mut self, nodes: &[Value], context: InlineContext) -> String {
        let mut output = String::new();
        let mut stack: Vec<PseudoMark> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let marks = self.pseudo_marks(node);
            let mut common = 0;
            while common < stack.len()
                && common < marks.len()
                && stack[common].key == marks[common].key
            {
                common += 1;
            }
            while stack.len() > common {
                output.push_str(stack.pop().map(|mark| mark.close).unwrap_or(""));
            }
            for mark in marks.into_iter().skip(common) {
                output.push_str(&mark.open);
                stack.push(mark);
            }
            let next = nodes.get(index + 1);
            let rendered = self.inline_node(node, context, &output, next);
            output.push_str(&rendered);
        }
        while let Some(mark) = stack.pop() {
            output.push_str(mark.close);
        }
        output
    }

    fn inline_node(
        &mut self,
        node: &Value,
        context: InlineContext,
        before: &str,
        next: Option<&Value>,
    ) -> String {
        let fragile = if matches!(context, InlineContext::Heading) {
            "\\protect"
        } else {
            ""
        };
        match node_type(node) {
            "text" => {
                let text = escape_latex(node.get("text").and_then(Value::as_str).unwrap_or(""));
                // Nach einem Befehlswort (z. B. Roh-LaTeX `\noindent`) würde TeX ein
                // Leerzeichen verschlucken bzw. Buchstaben zum Befehlsnamen ziehen.
                if ends_with_control_word(before) {
                    if let Some(rest) = text.strip_prefix(' ') {
                        return format!("\\ {rest}");
                    }
                    if text.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        return format!("{{}}{text}");
                    }
                }
                text
            }
            "hardBreak" => match context {
                InlineContext::Heading => " ".into(),
                InlineContext::Cell => "\\newline ".into(),
                InlineContext::Paragraph => {
                    let next_text = next
                        .filter(|n| node_type(n) == "text")
                        .map(text_content)
                        .unwrap_or_default();
                    let guard = if next_text.trim_start().starts_with(['[', '*']) {
                        "{}"
                    } else {
                        ""
                    };
                    if before.trim().is_empty() {
                        format!("\\mbox{{}}\\\\{guard}\n")
                    } else {
                        format!("\\\\{guard}\n")
                    }
                }
            },
            "inlineMath" => {
                let latex = attr_str(node, "latex");
                let latex = latex.trim();
                if latex.is_empty() {
                    String::new()
                } else {
                    format!("${latex}$")
                }
            }
            "citation" => {
                let keys: Vec<String> = attr_str(node, "keys")
                    .split(',')
                    .map(|key| key.trim().to_string())
                    .filter(|key| is_valid_key(key))
                    .collect();
                if keys.is_empty() {
                    return String::new();
                }
                let command = attr_str(node, "command");
                let command = if CITE_COMMANDS.contains(&command.as_str()) {
                    command
                } else {
                    "cite".to_string()
                };
                let prenote = attr_str(node, "prenote");
                let postnote = attr_str(node, "postnote");
                let options = if !prenote.trim().is_empty() {
                    format!(
                        "[{}][{}]",
                        escape_latex(prenote.trim()),
                        escape_latex(postnote.trim())
                    )
                } else if !postnote.trim().is_empty() {
                    format!("[{}]", escape_latex(postnote.trim()))
                } else {
                    String::new()
                };
                format!("\\{command}{options}{{{}}}", keys.join(","))
            }
            "footnote" => format!(
                "{fragile}\\footnote{{{}}}",
                escape_latex(&attr_str(node, "text"))
            ),
            "acronym" => {
                let key = attr_str(node, "key");
                if !is_valid_key(&key) {
                    return escape_latex(&attr_str(node, "short"));
                }
                let command = attr_str(node, "command");
                let command = if ["ac", "acs", "acl", "acf", "acp", "acsp", "aclp", "acfp"]
                    .contains(&command.as_str())
                {
                    command
                } else {
                    "ac".into()
                };
                format!("\\{command}{{{key}}}")
            }
            "crossReference" => {
                let label = attr_str(node, "label");
                if !is_valid_key(&label) {
                    return String::new();
                }
                let kind = attr_str(node, "kind");
                let kind = if ["ref", "pageref", "eqref", "autoref"].contains(&kind.as_str()) {
                    kind
                } else {
                    "ref".into()
                };
                format!("\\{kind}{{{label}}}")
            }
            "rawLatexInline" => attr_str(node, "latex"),
            "quantity" => {
                self.usage.siunitx = true;
                quantity_latex(node)
            }
            _ => {
                let content = children(node).to_vec();
                if content.is_empty() {
                    String::new()
                } else {
                    self.inline_internal(&content, context)
                }
            }
        }
    }

    // -------------------------------------------------------------- Blöcke

    pub fn blocks(&mut self, nodes: &[Value]) -> String {
        let mut output = String::new();
        let mut previous: Option<&Value> = None;
        for node in nodes {
            let latex = self.block(node);
            if latex.trim().is_empty() {
                continue;
            }
            if let Some(previous) = previous {
                output.push_str(if joins_tight(previous, node) {
                    "\n"
                } else {
                    "\n\n"
                });
            }
            output.push_str(&latex);
            previous = Some(node);
        }
        output
    }

    fn paragraph(&mut self, node: &Value) -> String {
        let content = self.inline_internal(children(node), InlineContext::Paragraph);
        let content = content.trim_end().to_string();
        if content.trim().is_empty() {
            return "\\vspace{\\baselineskip}".into();
        }
        let mut declarations: Vec<String> = Vec::new();
        match attr_str(node, "textAlign").as_str() {
            "center" => declarations.push("\\centering".into()),
            "right" => declarations.push("\\raggedleft".into()),
            "left" => declarations.push("\\raggedright".into()),
            _ => {}
        }
        if let Some(line_height) = attr_f64(node, "lineHeight").filter(|value| *value > 0.0) {
            self.usage.line_spacing = true;
            declarations.push(paragraph_spacing_declaration(line_height));
        }
        if let Some(indent) = attr_f64(node, "indent").filter(|value| *value > 0.0) {
            // \addtolength behält die Dehnbarkeit von \centering/\raggedleft bei.
            declarations.push(format!(
                "\\addtolength{{\\leftskip}}{{{}cm}}",
                format_number(indent * 1.25)
            ));
        }
        let mut latex = if declarations.is_empty() {
            content
        } else {
            format!("{{{} {content}\\par}}", declarations.join(""))
        };
        let shading = attr(node, "paragraphShading")
            .and_then(Value::as_str)
            .and_then(css_color_to_hex);
        let border = attr(node, "border")
            .is_some_and(|value| !value.is_null() && value != &Value::Bool(false) && value != "");
        if shading.is_some() {
            self.usage.colors = true;
        }
        if border || shading.is_some() {
            let width = if border {
                "\\linewidth-2\\fboxsep-2\\fboxrule"
            } else {
                "\\linewidth-2\\fboxsep"
            };
            let parbox = format!("\\parbox{{\\dimexpr{width}\\relax}}{{{latex}}}");
            latex = match (border, shading) {
                (true, Some(color)) => {
                    format!("\\noindent\\fcolorbox[HTML]{{808080}}{{{color}}}{{{parbox}}}")
                }
                (true, None) => format!("\\noindent\\fbox{{{parbox}}}"),
                (false, Some(color)) => {
                    format!("\\noindent\\colorbox[HTML]{{{color}}}{{{parbox}}}")
                }
                (false, None) => latex,
            };
        }
        latex
    }

    fn heading(&mut self, node: &Value) -> String {
        let level = attr_f64(node, "level").unwrap_or(1.0) as usize;
        let command = heading_command(level, self.has_chapters);
        let star = if attr_bool(node, "numbered") == Some(false) {
            "*"
        } else {
            ""
        };
        let title = self.inline_internal(children(node), InlineContext::Heading);
        let label = attr_str(node, "label");
        let label = if !label.is_empty() && is_valid_key(&label) {
            format!("\\label{{{label}}}")
        } else {
            String::new()
        };
        let mut latex = format!("\\{command}{star}{{{}}}{label}", title.trim());
        // Nicht nummerierte Überschrift trotzdem im Inhaltsverzeichnis.
        if !star.is_empty() && attr_bool(node, "inToc") == Some(true) {
            let toc_title = attr_str(node, "tocTitle");
            let toc_title = toc_title.trim();
            let entry = if !toc_title.is_empty() && is_balanced(toc_title) {
                toc_title.to_string()
            } else {
                title.trim().to_string()
            };
            latex.push_str(&format!(
                "\n\\addcontentsline{{toc}}{{{command}}}{{{entry}}}"
            ));
        }
        latex
    }

    fn list(&mut self, node: &Value) -> String {
        let ordered = node_type(node) == "orderedList";
        let environment = if attr_str(node, "listEnvironment") == "description" {
            "description"
        } else if ordered {
            "enumerate"
        } else {
            "itemize"
        };
        if ordered {
            self.enumerate_depth += 1;
        }
        let counter =
            ["enumi", "enumii", "enumiii", "enumiv"][self.enumerate_depth.clamp(1, 4) - 1];
        let start = attr_f64(node, "start").unwrap_or(1.0) as i64;
        let mut items = Vec::new();
        for item in children(node) {
            let content = children(item);
            let head = match content.first() {
                Some(first) if node_type(first) == "paragraph" => self
                    .inline_internal(children(first), InlineContext::Paragraph)
                    .trim()
                    .to_string(),
                Some(first) => format!("\\mbox{{}}\n{}", self.block(first)),
                None => String::new(),
            };
            let rest = self.blocks(content.get(1..).unwrap_or(&[]));
            // Eigene Marke: \item[a)]
            let label = attr_str(item, "itemLabel");
            let item_command = if label.is_empty() {
                "\\item".to_string()
            } else if label.contains(']') || !is_balanced(&label) {
                format!("\\item[{{{}}}]", escape_latex(&label))
            } else {
                format!("\\item[{label}]")
            };
            let separator = if head.is_empty() { "" } else { " " };
            items.push(if rest.is_empty() {
                format!("{item_command}{separator}{head}")
            } else {
                format!("{item_command}{separator}{head}\n\n{rest}")
            });
        }
        if ordered {
            self.enumerate_depth -= 1;
        }
        if items.is_empty() {
            return String::new();
        }
        let start_line = if ordered && start > 1 {
            format!("\\setcounter{{{counter}}}{{{}}}\n", start - 1)
        } else {
            String::new()
        };
        // enumitem-Optionen wie [leftmargin=1.5em] oder [label=\alph*)]
        let options = attr_str(node, "listOptions");
        let options = options.trim();
        let options = if !options.is_empty() && is_balanced(options) {
            self.usage.enumitem = true;
            format!("[{options}]")
        } else {
            String::new()
        };
        format!(
            "\\begin{{{environment}}}{options}\n{start_line}{}\n\\end{{{environment}}}",
            items.join("\n")
        )
    }

    fn in_multicols(&self) -> bool {
        self.layout_started && self.columns > 1
    }

    fn float_placement(&mut self, requested: &str) -> String {
        if self.in_multicols() || requested == "H" {
            self.usage.float_here = true;
            return "H".into();
        }
        if !requested.is_empty() && requested.chars().all(|c| "htbp!".contains(c)) {
            requested.to_string()
        } else {
            "htbp".into()
        }
    }

    /// Gleitumgebung (figure/table) mit Beschriftung. Beachtet die aus fremden
    /// Dokumenten übernommenen Angaben: Platzierung, `\centering`, Beschriftung
    /// oben/unten, Kurzbeschriftung, LaTeX-Beschriftung und Zusatzzeilen (`\small`).
    fn float_environment(
        &mut self,
        environment: &str,
        body: &str,
        node: &Value,
        extra: &[String],
    ) -> String {
        let requested = attr_str(node, "placement");
        let placement = self.float_placement(if requested.is_empty() {
            "htbp"
        } else {
            &requested
        });
        let mut lines = vec![format!("\\begin{{{environment}}}[{placement}]")];
        if attr_bool(node, "centered") != Some(false) {
            lines.push("\\centering".to_string());
        }
        lines.extend(extra.iter().cloned());
        let mut caption_lines = Vec::new();
        let caption = caption_latex(node, "caption");
        if !caption.is_empty() {
            let short = caption_latex(node, "shortCaption");
            let short = if short.is_empty() {
                String::new()
            } else {
                format!("[{short}]")
            };
            caption_lines.push(format!("\\caption{short}{{{caption}}}"));
        }
        let label = attr_str(node, "label");
        let label = label.trim();
        if !label.is_empty() && is_valid_key(label) {
            caption_lines.push(format!("\\label{{{label}}}"));
        }
        if attr_bool(node, "captionAbove") == Some(true) {
            lines.extend(caption_lines);
            lines.push(body.to_string());
        } else {
            lines.push(body.to_string());
            lines.extend(caption_lines);
        }
        lines.push(format!("\\end{{{environment}}}"));
        lines.join("\n")
    }

    fn image(&mut self, node: &Value) -> String {
        let Some(graphic) = self.graphic(node, 80.0) else {
            return String::new();
        };
        let caption = attr_str(node, "caption").trim().to_string();
        let label = attr_str(node, "label").trim().to_string();
        if caption.is_empty() && label.is_empty() && attr_str(node, "wrapper") != "float" {
            return format!("\\begin{{center}}\n{graphic}\n\\end{{center}}");
        }
        self.float_environment("figure", &graphic, node, &[])
    }

    /// Mehrere Bilder nebeneinander (Paket subcaption): je Bild eine `subfigure` mit
    /// eigener Unterbeschriftung und Label, darunter die gemeinsame Beschriftung.
    fn subfigures(&mut self, node: &Value) -> String {
        let items: Vec<Value> = attr(node, "items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let share = subfigure_default_share(items.len());
        let mut parts = Vec::new();
        for item in &items {
            let item = serde_json::json!({ "type": "image", "attrs": item });
            let Some(graphic) = self.graphic(&item, 100.0) else {
                continue;
            };
            let width = attr_f64(&item, "boxPercent")
                .map(|percent| percent.clamp(5.0, 100.0) / 100.0)
                .unwrap_or(share);
            let position = match attr_str(&item, "position").as_str() {
                "c" => "c",
                "b" => "b",
                _ => "t",
            };
            let mut lines = vec![
                format!(
                    "\\begin{{subfigure}}[{position}]{{{}\\linewidth}}",
                    format_number(width)
                ),
                "\\centering".to_string(),
                graphic,
            ];
            let caption = caption_latex(&item, "caption");
            if !caption.is_empty() {
                lines.push(format!("\\caption{{{caption}}}"));
            }
            let label = attr_str(&item, "label");
            let label = label.trim();
            if !label.is_empty() && is_valid_key(label) {
                lines.push(format!("\\label{{{label}}}"));
            }
            lines.push("\\end{subfigure}".to_string());
            parts.push(lines.join("\n"));
        }
        if parts.is_empty() {
            return String::new();
        }
        self.usage.subcaption = true;
        self.float_environment("figure", &parts.join("\n\\hfill\n"), node, &[])
    }

    /// `\includegraphics[…]{pfad}` eines Bildknotens (None: kein darstellbares Bild).
    fn graphic(&mut self, node: &Value, default_percent: f64) -> Option<String> {
        let mut path = attr_str(node, "latexPath").replace('\\', "/");
        if !path.is_empty() && !is_safe_relative_path(&path) {
            self.warn(format!(
                "Bildpfad „{path}“ ist kein sicherer relativer Projektpfad und wurde übersprungen."
            ));
            path.clear();
        }
        if path.is_empty() {
            let src = attr_str(node, "src");
            match embedded_asset(&src) {
                Some(asset) => {
                    path = asset.path.clone();
                    if !self
                        .assets
                        .iter()
                        .any(|existing| existing.path == asset.path)
                    {
                        self.assets.push(asset);
                    }
                }
                None if !src.is_empty() => {
                    self.warn("Ein eingebettetes Bild hat ein von LaTeX nicht unterstütztes Format (nur PNG, JPG, PDF) und wurde übersprungen.");
                    return None;
                }
                None => return None,
            }
        }
        let percent = attr_f64(node, "widthPercent")
            .unwrap_or(default_percent)
            .clamp(5.0, 100.0);
        let raw_options = attr_str(node, "graphicsOptions");
        let options = if !raw_options.trim().is_empty()
            && !raw_options.contains("\\input")
            && !raw_options.contains("\\write")
        {
            raw_options.trim().to_string()
        } else {
            // Höhe begrenzen: sehr hohe Bilder werden (seitenverhältnistreu) verkleinert
            // statt über den Seitenrand hinauszulaufen.
            format!(
                "width={}\\linewidth{IMAGE_HEIGHT_LIMIT}",
                format_number(percent / 100.0)
            )
        };
        self.usage.graphics = true;
        Some(format!("\\includegraphics[{options}]{{{path}}}"))
    }

    fn tikz(&mut self, node: &Value) -> String {
        let environment = if attr_str(node, "environment") == "circuitikz" {
            "circuitikz"
        } else {
            "tikzpicture"
        };
        if environment == "circuitikz" {
            self.usage.circuitikz = true;
        } else {
            self.usage.tikz = true;
        }
        let options = attr_str(node, "options");
        let options = options.trim();
        let code = attr_str(node, "code");
        let picture = format!(
            "\\begin{{{environment}}}{}\n{}\n\\end{{{environment}}}",
            if options.is_empty() {
                String::new()
            } else {
                format!("[{options}]")
            },
            code.trim_end()
        );
        let caption = attr_str(node, "caption").trim().to_string();
        let label = attr_str(node, "label").trim().to_string();
        if caption.is_empty() && label.is_empty() && attr_str(node, "wrapper") != "float" {
            return format!("\\begin{{center}}\n{picture}\n\\end{{center}}");
        }
        self.float_environment("figure", &picture, node, &[])
    }

    fn math(&mut self, node: &Value) -> String {
        let latex = attr_str(node, "latex");
        let latex = if latex.trim().is_empty() {
            "\\square".to_string()
        } else {
            latex.trim().to_string()
        };
        let numbered = attr_bool(node, "numbered") == Some(true);
        let label = attr_str(node, "label");
        let label = label.trim();
        let label_latex =
            if numbered && !label.is_empty() && is_valid_key(label) && !latex.contains("\\label") {
                format!("\n\\label{{{label}}}")
            } else {
                String::new()
            };
        let environment = attr_str(node, "environment");
        let environment =
            if ["align", "gather", "multline", "eqnarray"].contains(&environment.as_str()) {
                environment
            } else {
                "equation".into()
            };
        if environment != "equation" && environment != "eqnarray" {
            self.usage.ams = true;
        }
        if environment == "equation" {
            return if numbered {
                format!("\\begin{{equation}}\n{latex}{label_latex}\n\\end{{equation}}")
            } else {
                format!("\\[\n{latex}\n\\]")
            };
        }
        let name = if numbered {
            environment
        } else {
            format!("{environment}*")
        };
        format!("\\begin{{{name}}}\n{latex}{label_latex}\n\\end{{{name}}}")
    }

    fn code_block(&mut self, node: &Value) -> String {
        let text = text_content(node);
        let environment = attr_str(node, "environment");
        let options = attr_str(node, "listingOptions");
        let options = if is_balanced(&options) {
            options.trim().to_string()
        } else {
            String::new()
        };
        let language = attr_str(node, "language");
        let language = language.trim();
        let language = if language
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-_#".contains(c))
        {
            language
        } else {
            ""
        };
        match environment.as_str() {
            "lstlisting" if !text.contains("\\end{lstlisting}") => {
                self.usage.listings = true;
                let options = match option_value(&options, "language") {
                    // Sprache in der Oberfläche geändert → Option anpassen
                    Some(current)
                        if !language.is_empty() && !current.eq_ignore_ascii_case(language) =>
                    {
                        set_option(&options, "language", language)
                    }
                    None if !language.is_empty() => set_option(&options, "language", language),
                    _ => options,
                };
                let options = if options.is_empty() {
                    String::new()
                } else {
                    format!("[{options}]")
                };
                return format!("\\begin{{lstlisting}}{options}\n{text}\n\\end{{lstlisting}}");
            }
            "minted" if !text.contains("\\end{minted}") && !language.is_empty() => {
                let options = if options.is_empty() {
                    String::new()
                } else {
                    format!("[{options}]")
                };
                return format!(
                    "\\begin{{minted}}{options}{{{language}}}\n{text}\n\\end{{minted}}"
                );
            }
            "Verbatim" if !text.contains("\\end{Verbatim}") => {
                let options = if options.is_empty() {
                    String::new()
                } else {
                    format!("[{options}]")
                };
                return format!("\\begin{{Verbatim}}{options}\n{text}\n\\end{{Verbatim}}");
            }
            _ => {}
        }
        if text.contains("\\end{verbatim}") {
            return text
                .lines()
                .map(|line| {
                    format!(
                        "\\noindent\\texttt{{{}}}\\\\",
                        if line.is_empty() {
                            "~".to_string()
                        } else {
                            escape_latex(line)
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        format!("\\begin{{verbatim}}\n{text}\n\\end{{verbatim}}")
    }

    fn table(&mut self, node: &Value) -> String {
        #[derive(Clone)]
        struct Cell {
            row: usize,
            col: usize,
            rowspan: usize,
            colspan: usize,
            header: bool,
            index: (usize, usize),
        }
        let rows: Vec<&Value> = children(node)
            .iter()
            .filter(|row| node_type(row) == "tableRow")
            .collect();
        if rows.is_empty() {
            return String::new();
        }
        let mut grid: Vec<Vec<Option<usize>>> = vec![Vec::new(); rows.len()];
        let mut cells: Vec<Cell> = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let mut col = 0;
            for (cell_index, cell_node) in children(row).iter().enumerate() {
                while grid[row_index].get(col).is_some_and(Option::is_some) {
                    col += 1;
                }
                let colspan = attr_f64(cell_node, "colspan").unwrap_or(1.0).max(1.0) as usize;
                let rowspan = (attr_f64(cell_node, "rowspan").unwrap_or(1.0).max(1.0) as usize)
                    .min(rows.len() - row_index);
                let id = cells.len();
                cells.push(Cell {
                    row: row_index,
                    col,
                    rowspan,
                    colspan,
                    header: node_type(cell_node) == "tableHeader",
                    index: (row_index, cell_index),
                });
                for r in row_index..row_index + rowspan {
                    for c in col..col + colspan {
                        if grid[r].len() <= c {
                            grid[r].resize(c + 1, None);
                        }
                        grid[r][c] = Some(id);
                    }
                }
                col += colspan;
            }
        }
        let column_count = grid.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let cell_node = |cell: &Cell| -> &Value { &children(rows[cell.index.0])[cell.index.1] };
        let align_of = |cell: &Cell| -> String {
            let align = attr_str(cell_node(cell), "align");
            if align.is_empty() {
                "left".into()
            } else {
                align
            }
        };

        let mut column_align = Vec::new();
        let mut column_widths: Vec<Option<f64>> = Vec::new();
        for c in 0..column_count {
            match grid[0].get(c).copied().flatten() {
                Some(id) => {
                    let cell = &cells[id];
                    column_align.push(align_of(cell));
                    let width = attr(cell_node(cell), "colwidth")
                        .and_then(Value::as_array)
                        .and_then(|widths| widths.get(c - cell.col))
                        .and_then(Value::as_f64)
                        .filter(|width| *width > 0.0);
                    column_widths.push(width);
                }
                None => {
                    column_align.push("left".into());
                    column_widths.push(None);
                }
            }
        }
        let all_widths = column_widths.iter().all(Option::is_some);
        let mean = if all_widths {
            column_widths.iter().flatten().sum::<f64>() / column_count as f64
        } else {
            0.0
        };
        let declaration = |align: &str| match align {
            "center" => "\\centering",
            "right" => "\\raggedleft",
            _ => "\\raggedright",
        };
        // Stil: grid (Standard, alle Linien) | booktabs | plain (ohne Linien)
        let style = attr_str(node, "tableStyle");
        let grid_style = !matches!(style.as_str(), "booktabs" | "plain");
        // Aus einem fremden Dokument übernommene Spaltendefinition (falls die
        // Spaltenzahl noch stimmt) samt Umgebung (tabular/tabularx/tabular*).
        let original_spec = attr_str(node, "columnSpec");
        let original_kinds = (!original_spec.trim().is_empty() && is_balanced(&original_spec))
            .then(|| spec_column_kinds(&original_spec))
            .flatten()
            .filter(|kinds| kinds.len() == column_count);
        let generated_spec: Vec<String> = column_align
            .iter()
            .enumerate()
            .map(|(index, align)| {
                let factor = if all_widths && mean > 0.0 {
                    (column_widths[index].unwrap_or(mean) / mean * 1000.0).round() / 1000.0
                } else {
                    1.0
                };
                let size = if (factor - 1.0).abs() > f64::EPSILON {
                    format!("\\hsize={factor}\\hsize\\linewidth=\\hsize")
                } else {
                    String::new()
                };
                format!(">{{{size}{}\\arraybackslash}}X", declaration(align))
            })
            .collect();
        let bars = grid_style && original_kinds.is_none();
        let column_kind = |c: usize| -> char {
            original_kinds
                .as_ref()
                .and_then(|kinds| kinds.get(c).copied())
                .unwrap_or('X')
        };

        let mut lines = Vec::new();
        let mut row_lines: Vec<String> = Vec::new();
        let mut boundaries: Vec<Option<String>> = vec![None; rows.len() + 1];
        for r in 0..rows.len() {
            let mut row_cells = Vec::new();
            let mut c = 0;
            while c < column_count {
                let Some(id) = grid[r].get(c).copied().flatten() else {
                    row_cells.push(String::new());
                    c += 1;
                    continue;
                };
                let cell = cells[id].clone();
                let is_origin = cell.row == r && cell.col == c;
                let mut content = String::new();
                if is_origin {
                    let mut parts = Vec::new();
                    for child in children(cell_node(&cell)) {
                        let part = if node_type(child) == "paragraph" {
                            self.inline_internal(children(child), InlineContext::Cell)
                                .trim()
                                .to_string()
                        } else {
                            self.block(child)
                        };
                        if !part.is_empty() {
                            parts.push(part);
                        }
                    }
                    content = parts.join("\\par ");
                    if cell.header && !content.is_empty() {
                        content = format!("\\bfseries {content}");
                    }
                    if cell.rowspan > 1 {
                        self.usage.multirow = true;
                        content = format!("\\multirow{{{}}}{{=}}{{{content}}}", cell.rowspan);
                    }
                }
                let align = align_of(&cell);
                let letter = match align.as_str() {
                    "center" => "c",
                    "right" => "r",
                    _ => "l",
                };
                let (left_rule, right_rule) = if bars {
                    (if c == 0 { "|" } else { "" }, "|")
                } else {
                    ("", "")
                };
                if cell.colspan > 1 {
                    content = format!(
                        "\\multicolumn{{{}}}{{{left_rule}{letter}{right_rule}}}{{{content}}}",
                        cell.colspan
                    );
                } else if is_origin && align != column_align[c] {
                    if matches!(column_kind(c), 'l' | 'c' | 'r') {
                        content = format!("\\multicolumn{{1}}{{{letter}}}{{{content}}}");
                    } else {
                        // X-/p-Spalten sind in \multicolumn nicht erlaubt → Ausrichtung in der Zelle.
                        content = format!("{}\\arraybackslash {content}", declaration(&align));
                    }
                }
                row_cells.push(content);
                c += cell.colspan.max(1);
            }
            row_lines.push(format!("{} \\\\", row_cells.join(" & ")));
            lines.push(format!("{} \\\\", row_cells.join(" & ")));
            let continuing: Vec<bool> = if r + 1 < rows.len() {
                (0..column_count)
                    .map(|c| {
                        let here = grid[r].get(c).copied().flatten();
                        here.is_some() && grid[r + 1].get(c).copied().flatten() == here
                    })
                    .collect()
            } else {
                Vec::new()
            };
            if continuing.iter().any(|value| *value) {
                let mut segments = Vec::new();
                let mut start: Option<usize> = None;
                for c in 0..=column_count {
                    let open = c < column_count && !continuing[c];
                    if open && start.is_none() {
                        start = Some(c);
                    }
                    if !open {
                        if let Some(begin) = start.take() {
                            segments.push(format!("\\cline{{{}-{c}}}", begin + 1));
                        }
                    }
                }
                boundaries[r + 1] = Some(segments.join(""));
                lines.push(segments.join(""));
            } else {
                lines.push("\\hline".into());
            }
        }

        // Linien zwischen den Zeilen: übernommene (rowRules) bzw. nach Stil.
        let stored: Vec<String> = attr(node, "rowRules")
            .and_then(Value::as_array)
            .map(|rules| {
                rules
                    .iter()
                    .map(|rule| rule.as_str().unwrap_or("").trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let stored_valid = stored.len() >= 2 && stored.iter().all(|rule| is_balanced(rule));
        let row_count = rows.len();
        let rules: Option<Vec<String>> = if stored_valid {
            Some(
                (0..=row_count)
                    .map(|index| {
                        if stored.len() == row_count + 1 {
                            stored[index].clone()
                        } else if index == 0 {
                            stored[0].clone()
                        } else if index == row_count {
                            stored[stored.len() - 1].clone()
                        } else if stored.len() <= 2 {
                            String::new()
                        } else {
                            stored[index.min(stored.len() - 2)].clone()
                        }
                    })
                    .collect(),
            )
        } else {
            match style.as_str() {
                "booktabs" => Some(
                    (0..=row_count)
                        .map(|index| match index {
                            0 => "\\toprule".to_string(),
                            1 if row_count > 1 => "\\midrule".to_string(),
                            _ if index == row_count => "\\bottomrule".to_string(),
                            _ => String::new(),
                        })
                        .collect(),
                ),
                "plain" => Some(vec![String::new(); row_count + 1]),
                _ => None,
            }
        };
        let body = match rules {
            // Standard-Gitter (wie bisher)
            None => format!("\\hline\n{}", lines.join("\n")),
            Some(rules) => {
                self.note_raw_usage(&rules.join(" "));
                let mut output = Vec::new();
                for (index, rule) in rules.iter().enumerate() {
                    // Über mehrere Zeilen verbundene Zellen: \hline → \cline-Segmente
                    let rule = match &boundaries[index] {
                        Some(segments) if rule == "\\hline" => segments.clone(),
                        _ => rule.clone(),
                    };
                    if !rule.is_empty() {
                        output.push(rule);
                    }
                    if let Some(row) = row_lines.get(index) {
                        output.push(row.clone());
                    }
                }
                output.join("\n")
            }
        };
        if attr_bool(node, "breakAcrossPages") == Some(true) {
            let header_row =
                !grid[0].is_empty() && grid[0].iter().flatten().all(|id| cells[*id].header);
            return self.long_table(node, &body, &column_align, &column_widths, bars, header_row);
        }
        let environment = attr_str(node, "tableEnvironment");
        let width = attr_str(node, "tableWidth");
        let width = if width.trim().is_empty() || !is_balanced(&width) {
            "\\linewidth".to_string()
        } else {
            width.trim().to_string()
        };
        let begin = match (&original_kinds, environment.as_str()) {
            (Some(_), "tabular") => format!("\\begin{{tabular}}{{{original_spec}}}"),
            (Some(_), "tabular*") => format!("\\begin{{tabular*}}{{{width}}}{{{original_spec}}}"),
            (Some(_), _) => {
                self.usage.tabularx = true;
                format!("\\begin{{tabularx}}{{{width}}}{{{original_spec}}}")
            }
            (None, _) => {
                self.usage.tabularx = true;
                if bars {
                    format!(
                        "\\begin{{tabularx}}{{\\linewidth}}{{|{}|}}",
                        generated_spec.join("|")
                    )
                } else {
                    format!(
                        "\\begin{{tabularx}}{{\\linewidth}}{{{}}}",
                        generated_spec.join("")
                    )
                }
            }
        };
        let end = match (&original_kinds, environment.as_str()) {
            (Some(_), "tabular") => "\\end{tabular}",
            (Some(_), "tabular*") => "\\end{tabular*}",
            _ => "\\end{tabularx}",
        };
        let tabular = format!("{begin}\n{body}\n{end}");

        // Zusatzzeilen aus dem Original: Schriftgröße, Zeilenabstand
        let mut extra = Vec::new();
        let size = attr_str(node, "fontSize");
        if FONT_SIZE_COMMANDS.contains(&size.as_str()) {
            extra.push(format!("\\{size}"));
        }
        let stretch = attr_str(node, "arrayStretch");
        if !stretch.is_empty() && stretch.parse::<f64>().is_ok() {
            extra.push(format!("\\renewcommand{{\\arraystretch}}{{{stretch}}}"));
        }
        let caption = attr_str(node, "caption").trim().to_string();
        let label = attr_str(node, "label").trim().to_string();
        let wrapper = attr_str(node, "wrapper");
        if !caption.is_empty() || !label.is_empty() || wrapper == "float" {
            return self.float_environment("table", &tabular, node, &extra);
        }
        if wrapper == "center" {
            let mut lines = vec!["\\begin{center}".to_string()];
            lines.extend(extra);
            lines.push(tabular);
            lines.push("\\end{center}".into());
            return lines.join("\n");
        }
        if extra.is_empty() {
            format!("\\noindent\n{tabular}")
        } else {
            format!(
                "\\begingroup\n{}\n\\noindent\n{tabular}\n\\endgroup",
                extra.join("\n")
            )
        }
    }

    /// Tabelle über mehrere Seiten (`longtable`): Spalten als `p{…}` mit den Anteilen
    /// der Spaltenbreiten, Beschriftung nur auf der ersten Seite, Kopfzeile auf jeder Seite.
    fn long_table(
        &mut self,
        node: &Value,
        body: &str,
        column_align: &[String],
        column_widths: &[Option<f64>],
        bars: bool,
        header_row: bool,
    ) -> String {
        self.usage.longtable = true;
        let count = column_align.len().max(1);
        let total: f64 = if column_widths.iter().all(Option::is_some) {
            column_widths.iter().flatten().sum()
        } else {
            0.0
        };
        let columns: Vec<String> = column_align
            .iter()
            .enumerate()
            .map(|(index, align)| {
                let share = match column_widths.get(index).copied().flatten() {
                    Some(width) if total > 0.0 => width / total,
                    _ => 1.0 / count as f64,
                };
                let declaration = match align.as_str() {
                    "center" => "\\centering",
                    "right" => "\\raggedleft",
                    _ => "\\raggedright",
                };
                format!(
                    ">{{{declaration}\\arraybackslash}}p{{\\dimexpr {}\\linewidth-2\\tabcolsep\\relax}}",
                    (share * 1000.0).round() / 1000.0
                )
            })
            .collect();
        let spec = if bars {
            format!("|{}|", columns.join("|"))
        } else {
            columns.join("")
        };
        let lines: Vec<&str> = body.lines().collect();
        let is_row = |line: &str| line.trim_end().ends_with("\\\\");
        let caption = attr_str(node, "caption").trim().to_string();
        let label = attr_str(node, "label").trim().to_string();
        let mut output = vec![format!("\\begin{{longtable}}{{{spec}}}")];
        if !caption.is_empty() || (!label.is_empty() && is_valid_key(&label)) {
            let mut line = format!("\\caption{{{}}}", caption_latex(node, "caption"));
            if !label.is_empty() && is_valid_key(&label) {
                line.push_str(&format!("\\label{{{label}}}"));
            }
            line.push_str("\\\\");
            output.push(line);
        }
        let first_row = lines.iter().position(|line| is_row(line));
        match first_row.filter(|_| header_row) {
            Some(row) => {
                // Kopf = Linien davor, Kopfzeile, Linien danach
                let mut end = row + 1;
                while end < lines.len()
                    && !is_row(lines[end])
                    && lines[end + 1..].iter().any(|line| is_row(line))
                {
                    end += 1;
                }
                let head = lines[..end].join("\n");
                output.push(head.clone());
                output.push("\\endfirsthead".into());
                output.push(head);
                output.push("\\endhead".into());
                output.extend(lines[end..].iter().map(|line| line.to_string()));
            }
            None => output.extend(lines.iter().map(|line| line.to_string())),
        }
        output.push("\\end{longtable}".into());
        let mut extra = Vec::new();
        let size = attr_str(node, "fontSize");
        if FONT_SIZE_COMMANDS.contains(&size.as_str()) {
            extra.push(format!("\\{size}"));
        }
        let stretch = attr_str(node, "arrayStretch");
        if !stretch.is_empty() && stretch.parse::<f64>().is_ok() {
            extra.push(format!("\\renewcommand{{\\arraystretch}}{{{stretch}}}"));
        }
        let table = output.join("\n");
        if extra.is_empty() {
            table
        } else {
            format!("\\begingroup\n{}\n{table}\n\\endgroup", extra.join("\n"))
        }
    }

    pub fn bibliography_commands(&self) -> String {
        let (bst, _) = bibliography_style(&self.settings.bibliography.style);
        let file = self
            .settings
            .bibliography
            .file
            .trim_end_matches(".bib")
            .trim_end_matches(".BIB");
        format!("\\bibliographystyle{{{bst}}}\n\\bibliography{{{file}}}")
    }

    fn acronym_list(&self) -> String {
        let mut entries = self.acronyms.clone();
        entries.sort_by_key(|(_, short, _)| short.to_lowercase());
        if entries.is_empty() {
            return "\\begin{acronym}\n\\end{acronym}".into();
        }
        let longest = entries
            .iter()
            .map(|(_, short, _)| short.as_str())
            .max_by_key(|short| short.chars().count())
            .unwrap_or("");
        let lines: Vec<String> = entries
            .iter()
            .map(|(key, short, long)| {
                format!(
                    "\\acro{{{key}}}[{}]{{{}}}",
                    escape_latex(short),
                    escape_latex(long)
                )
            })
            .collect();
        format!(
            "\\begin{{acronym}}[{}]\n{}\n\\end{{acronym}}",
            escape_latex(longest),
            lines.join("\n")
        )
    }

    pub fn acronym_heading(&self) -> String {
        let heading = if self.has_chapters {
            "chapter"
        } else {
            "section"
        };
        let title = &super::languages::get(&self.settings.language)
            .labels
            .acronyms;
        format!("\\{heading}*{{{title}}}\n\\addcontentsline{{toc}}{{{heading}}}{{{title}}}")
    }

    /// Standardtitel eines Verzeichnisses in der Dokumentsprache (für den Eintrag
    /// im Inhaltsverzeichnis).
    fn directory_title(&self, kind: &str) -> &'static str {
        let labels = &super::languages::get(&self.settings.language).labels;
        match kind {
            "contents" => &labels.contents,
            "figures" => &labels.figures,
            "tables" => &labels.tables,
            "bibliography" => &labels.bibliography,
            "acronyms" => &labels.acronyms,
            _ => &labels.list,
        }
    }

    /// `\phantomsection\addcontentsline{…}` vor einem Verzeichnis (Eintrag im
    /// Inhaltsverzeichnis); Titel und Ebene stehen in den Markerdaten.
    pub fn directory_toc_prefix(&self, data: &Value) -> String {
        let level = data
            .get("tocLevel")
            .and_then(Value::as_str)
            .filter(|level| SECTION_LEVELS.contains(level))
            .unwrap_or(if self.has_chapters {
                "chapter"
            } else {
                "section"
            });
        let kind = data.get("kind").and_then(Value::as_str).unwrap_or("");
        let title = data
            .get("tocTitle")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty() && is_balanced(title))
            .map(str::to_string)
            .unwrap_or_else(|| self.directory_title(kind).to_string());
        format!("\\phantomsection\n\\addcontentsline{{toc}}{{{level}}}{{{title}}}")
    }

    fn directory(&mut self, node: &Value) -> String {
        let kind = attr_str(node, "kind");
        let mut data = json!({ "kind": kind });
        let mut prefix = String::new();
        if attr_bool(node, "inToc") == Some(true) && kind != "acronyms" {
            data["inToc"] = json!(true);
            for key in ["tocTitle", "tocLevel"] {
                let value = attr_str(node, key);
                if !value.trim().is_empty() {
                    data[key] = json!(value.trim());
                }
            }
            prefix = format!("{}\n", self.directory_toc_prefix(&data));
        }
        let wrap = |data: &Value, content: &str| {
            markers::wrap("directory", data, &format!("{prefix}{content}"))
        };
        match kind.as_str() {
            "contents" => wrap(&data, "\\tableofcontents"),
            "figures" => wrap(&data, "\\listoffigures"),
            "tables" => wrap(&data, "\\listoftables"),
            "acronyms" => {
                let content = format!("{}\n{}", self.acronym_heading(), self.acronym_list());
                markers::wrap("directory", &data, &content)
            }
            "bibliography" if attr_str(node, "command") == "printbibliography" || self.biblatex => {
                // biblatex: \printbibliography[Optionen]
                let options = attr_str(node, "options");
                let options = options.trim();
                let options = if !options.is_empty() && is_balanced(options) {
                    data["options"] = json!(options);
                    format!("[{options}]")
                } else {
                    String::new()
                };
                data["command"] = json!("printbibliography");
                wrap(&data, &format!("\\printbibliography{options}"))
            }
            "bibliography" => {
                if self.settings.bibliography.file.is_empty() {
                    self.warn("Literaturverzeichnis eingefügt, aber keine .bib-Datei gewählt (Referenzen → Literaturdatei).");
                    wrap(&data, "% Literaturverzeichnis: keine .bib-Datei ausgewählt")
                } else if !self.usage.citations {
                    // Ohne \cite erzeugt BibTeX eine leere thebibliography-Umgebung
                    // („Something's wrong--perhaps a missing \item“).
                    wrap(
                        &data,
                        "% Literaturverzeichnis: erscheint, sobald das Dokument Zitate enthält",
                    )
                } else {
                    let commands = self.bibliography_commands();
                    wrap(&data, &commands)
                }
            }
            _ => String::new(),
        }
    }

    /// Umgebung mit visuell bearbeitbarem Inhalt (tcolorbox, multicols, abstract,
    /// minipage, eigene Boxen aus der Präambel …).
    fn environment_block(&mut self, node: &Value) -> String {
        let name = attr_str(node, "name");
        let body = self.blocks(children(node));
        if !is_environment_name(&name) {
            return body;
        }
        let args = attr_str(node, "args");
        let args = if is_balanced(&args) {
            args.trim_end().to_string()
        } else {
            String::new()
        };
        match name.trim_end_matches('*') {
            "tcolorbox" => self.usage.tcolorbox = true,
            "multicols" => self.usage.multicol = true,
            "landscape" => self.usage.landscape_sections = true,
            _ => {}
        }
        format!("\\begin{{{name}}}{args}\n{body}\n\\end{{{name}}}")
    }

    /// Inhalt einer eingebundenen Datei (`\input`/`\include`). Im Gesamtdokument
    /// steht er als Markerbereich; beim Speichern wird er wieder in die eigene
    /// Datei geschrieben (`core::includes::split`).
    fn include_block(&mut self, node: &Value) -> String {
        let file = attr_str(node, "file");
        let command = match attr_str(node, "command").as_str() {
            "include" => "include",
            "subfile" => "subfile",
            _ => "input",
        };
        let body = self.blocks(children(node));
        let mut data = json!({ "file": file, "command": command });
        for key in ["preamble", "postamble"] {
            if let Some(value) = attr(node, key).filter(|value| value.is_string()) {
                data[key] = value.clone();
            }
        }
        markers::wrap("include", &data, &body)
    }

    pub fn frontmatter_header(&self, kind: &str, title: &str, in_toc: bool) -> String {
        let heading = if self.has_chapters {
            "chapter"
        } else {
            "section"
        };
        let escaped = escape_latex(title);
        let mut lines = Vec::new();
        if kind == "confidentiality" {
            lines.push("\\thispagestyle{empty}".to_string());
            lines.push(format!("\\section*{{{escaped}}}"));
        } else {
            if !self.has_chapters {
                lines.push("\\clearpage".to_string());
            }
            lines.push(format!("\\{heading}*{{{escaped}}}"));
        }
        if in_toc {
            lines.push(format!(
                "\\addcontentsline{{toc}}{{{heading}}}{{{escaped}}}"
            ));
        }
        lines.join("\n")
    }

    fn frontmatter(&mut self, node: &Value) -> String {
        let kind = Some(attr_str(node, "kind"))
            .filter(|kind| !kind.is_empty())
            .unwrap_or_else(|| "abstract".into());
        let title = attr_str(node, "title");
        let in_toc = attr_bool(node, "inToc") == Some(true);
        let header = self.frontmatter_header(&kind, &title, in_toc);
        let body = self.blocks(children(node));
        let closing = if kind == "confidentiality" {
            "\n\\clearpage"
        } else {
            ""
        };
        markers::wrap(
            "frontmatter",
            &json!({ "kind": kind, "title": title, "inToc": in_toc }),
            &format!("{header}\n{body}{closing}"),
        )
    }

    pub fn title_page_latex(&self, data: &Value) -> String {
        let field = |name: &str| {
            data.get(name)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let lines_of = |name: &str| -> Vec<String> {
            field(name)
                .lines()
                .map(|line| escape_latex(line.trim()))
                .filter(|line| !line.is_empty())
                .collect()
        };
        let group = |names: &[&str]| -> Vec<String> {
            names.iter().flat_map(|name| lines_of(name)).collect()
        };
        let mut lines: Vec<String> = vec!["\\begin{titlepage}".into()];
        let logo = field("logoPath").replace('\\', "/");
        let logo = logo.trim();
        if !logo.is_empty() && is_safe_relative_path(logo) {
            lines.push("\\begin{flushright}".into());
            lines.push(format!("\\includegraphics[width=0.4\\linewidth]{{{logo}}}"));
            lines.push("\\end{flushright}".into());
        }
        lines.push("\\begin{flushleft}".into());
        let title = join_latex_lines(&lines_of("title"));
        if !title.is_empty() {
            lines.push(format!("{{\\Large\\bfseries {title}\\par}}"));
        }
        let subtitle = join_latex_lines(&lines_of("subtitle"));
        if !subtitle.is_empty() {
            lines.push("\\vspace{0.5em}".into());
            lines.push(format!("{{\\large {subtitle}\\par}}"));
        }
        for (names, space) in [
            (&["thesisType", "degreeIntro"][..], "\\vspace{1cm}"),
            (&["institution", "program"][..], "\\vspace{1cm}"),
            (
                &["authorLabel", "author", "placeDate"][..],
                "\\vspace{2.5cm}",
            ),
            (&["supervisorLabel", "supervisor"][..], "\\vspace{3cm}"),
        ] {
            let content = group(names);
            if !content.is_empty() {
                lines.push(space.into());
                lines.push(join_latex_lines(&content));
                lines.push(String::new());
            }
            if names[0] == "thesisType" {
                let degree = lines_of("degree").join(" ");
                if !degree.is_empty() {
                    lines.push("\\vspace{0.5cm}".into());
                    lines.push(format!("\\textbf{{{degree}}}"));
                    lines.push(String::new());
                }
            }
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines.push("\\end{flushleft}".into());
        lines.push("\\end{titlepage}".into());
        lines.join("\n")
    }

    fn title_page(&mut self, node: &Value) -> String {
        let mut data = serde_json::Map::new();
        if let Some(attrs) = node.get("attrs").and_then(Value::as_object) {
            for (key, value) in attrs {
                if key != "logoSrc" && value.as_str().is_some_and(|text| !text.is_empty()) {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        let data = Value::Object(data);
        let latex = self.title_page_latex(&data);
        markers::wrap("titlepage", &data, &latex)
    }

    /// Übergang zwischen Abschnittslayouts (Spalten/Querformat).
    pub fn section_transition(
        from_columns: u8,
        from_landscape: bool,
        to_columns: u8,
        to_landscape: bool,
    ) -> String {
        let mut lines = Vec::new();
        if from_columns > 1 {
            lines.push("\\end{multicols}".to_string());
        }
        lines.push("\\clearpage".to_string());
        if from_landscape && !to_landscape {
            lines.push("\\end{landscape}".to_string());
        }
        if !from_landscape && to_landscape {
            lines.push("\\begin{landscape}".to_string());
        }
        if to_columns > 1 {
            lines.push(format!("\\begin{{multicols}}{{{to_columns}}}"));
        }
        lines.join("\n")
    }

    fn section_break(&mut self, node: &Value) -> String {
        let columns = attr_f64(node, "columns").unwrap_or(1.0).clamp(1.0, 3.0) as u8;
        let mut orientation = attr_str(node, "orientation");
        if !["keep", "portrait", "landscape"].contains(&orientation.as_str()) {
            orientation = "keep".into();
        }
        let mut landscape = self.landscape;
        if orientation == "landscape" {
            if self.settings.orientation == "portrait" {
                landscape = true;
            } else {
                self.warn("Querformat-Abschnitte sind in einem bereits querformatigen Dokument wirkungslos.");
            }
        }
        if orientation == "portrait" {
            landscape = false;
        }
        self.layout_started = true;
        let transition = Self::section_transition(self.columns, self.landscape, columns, landscape);
        if columns > 1 {
            self.usage.multicol = true;
        }
        if landscape {
            self.usage.landscape_sections = true;
        }
        self.columns = columns;
        self.landscape = landscape;
        markers::wrap(
            "sectionbreak",
            &json!({ "columns": columns, "orientation": orientation }),
            &transition,
        )
    }

    /// LaTeX eines Blocks. Importierte Blöcke tragen ihren Originalcode
    /// (`sourceLatex`) und den Fingerabdruck der daraus erzeugten Ausgabe
    /// (`sourceKey`): Solange der Block unverändert ist, wird der Originalcode
    /// exakt zurückgeschrieben (Formatierung, Zeilenumbrüche, Tabellenstil …).
    pub fn block(&mut self, node: &Value) -> String {
        let generated = self.generate_block(node);
        match preserved_source(node, &generated) {
            Some(source) => {
                self.note_raw_usage(source);
                source.to_string()
            }
            None => generated,
        }
    }

    fn generate_block(&mut self, node: &Value) -> String {
        match node_type(node) {
            "paragraph" => self.paragraph(node),
            "heading" => self.heading(node),
            "blockquote" => format!(
                "\\begin{{quote}}\n{}\n\\end{{quote}}",
                self.blocks(children(node))
            ),
            "bulletList" | "orderedList" => self.list(node),
            "codeBlock" => self.code_block(node),
            "horizontalRule" => "\\noindent\\rule{\\linewidth}{0.4pt}".into(),
            "image" => self.image(node),
            "subfigures" => self.subfigures(node),
            "table" => self.table(node),
            "mathBlock" => self.math(node),
            "tikzBlock" => self.tikz(node),
            "rawLatexBlock" => attr_str(node, "rawLatex").trim_end().to_string(),
            "directoryBlock" => self.directory(node),
            "frontmatterBlock" => self.frontmatter(node),
            "titlePage" => self.title_page(node),
            "pageBreak" => {
                if attr_str(node, "breakType") == "section" {
                    self.section_break(node)
                } else {
                    "\\clearpage".into()
                }
            }
            "verticalSpace" => vertical_space_latex(node),
            "maketitle" => {
                self.usage.maketitle = true;
                "\\maketitle".into()
            }
            "appendixMarker" => "\\appendix".into(),
            "environmentBlock" => self.environment_block(node),
            "includeBlock" => self.include_block(node),
            _ => {
                let content = children(node).to_vec();
                self.blocks(&content)
            }
        }
    }

    /// Dokumentkörper inkl. Layoutbereichen und automatisch erzeugten Teilen.
    pub fn body(&mut self, doc: &Value) -> String {
        for node in children(doc) {
            self.scan(node);
        }
        let mut parts: Vec<String> = Vec::new();
        let undefined: Vec<&(String, String, String)> = self
            .acronyms
            .iter()
            .filter(|(key, _, _)| !self.predefined_acronyms.contains(key))
            .collect();
        if !undefined.is_empty() && !self.has_acronym_list {
            let definitions: Vec<String> = undefined
                .iter()
                .map(|(key, short, long)| {
                    format!(
                        "\\acrodef{{{key}}}[{}]{{{}}}",
                        escape_latex(short),
                        escape_latex(long)
                    )
                })
                .collect();
            parts.push(markers::wrap(
                "acronym-definitions",
                &json!({}),
                &definitions.join("\n"),
            ));
        }
        let leading = [
            "titlePage",
            "frontmatterBlock",
            "directoryBlock",
            "pageBreak",
        ];
        let mut previous: Option<&Value> = None;
        // Leere Absätze am Dokumentende (der Editor hängt nach Blöcken wie Tabellen
        // oder Roh-LaTeX automatisch einen an) erzeugen keinen Code.
        let all = children(doc);
        let last_content = all
            .iter()
            .rposition(|node| !(node_type(node) == "paragraph" && children(node).is_empty()))
            .map_or(0, |index| index + 1);
        for node in &all[..last_content] {
            let kind = node_type(node);
            let is_section_break = kind == "pageBreak" && attr_str(node, "breakType") == "section";
            if !self.layout_started && !is_section_break && !leading.contains(&kind) {
                self.layout_started = true;
                if self.settings.columns > 1 {
                    self.usage.multicol = true;
                    self.columns = self.settings.columns;
                    parts.push(markers::wrap(
                        "layout-start",
                        &json!({ "columns": self.settings.columns }),
                        &format!("\\begin{{multicols}}{{{}}}", self.settings.columns),
                    ));
                    previous = None;
                }
            }
            let latex = self.block(node);
            if latex.trim().is_empty() {
                continue;
            }
            match (previous, parts.last_mut()) {
                (Some(previous), Some(last)) if joins_tight(previous, node) => {
                    last.push('\n');
                    last.push_str(&latex);
                }
                _ => parts.push(latex),
            }
            previous = Some(node);
        }
        // Eigene Literaturverzeichnisse (biblatex, thebibliography, \bibliography im
        // Roh-LaTeX) nicht durch ein automatisches ergänzen.
        if self.usage.citations
            && !self.has_bibliography_block
            && !self.usage.manual_bibliography
            && !self.biblatex
        {
            if !self.settings.bibliography.file.is_empty() {
                self.usage.bibliography = true;
                let commands = self.bibliography_commands();
                parts.push(markers::wrap("auto-bibliography", &json!({}), &commands));
            } else {
                self.warn("Das Dokument enthält Zitate, aber keine .bib-Datei – im PDF erscheinen Fragezeichen.");
            }
        }
        let mut closing = Vec::new();
        if self.layout_started && self.columns > 1 {
            closing.push("\\end{multicols}");
        }
        if self.landscape {
            closing.push("\\end{landscape}");
        }
        if !closing.is_empty() {
            parts.push(markers::wrap("layout-end", &json!({}), &closing.join("\n")));
        }
        parts.join("\n\n")
    }
}

pub struct ExportOptions<'a> {
    pub settings: &'a DocumentSettings,
    /// Vom Nutzer bearbeitete Präambel (alles vor \begin{document}).
    pub custom_preamble: Option<&'a str>,
    /// Zusätzliche Präambelzeilen aktiver Add-ons.
    pub addon_preamble: &'a [String],
}

pub fn export_document(doc: &Value, options: &ExportOptions) -> ExportResult {
    let custom = options
        .custom_preamble
        .filter(|preamble| !preamble.trim().is_empty());
    let has_chapters = match custom {
        Some(preamble) => class_has_chapters(preamble),
        None => options.settings.has_chapters(),
    };
    let mut exporter = Exporter::new(options.settings, has_chapters);
    exporter.biblatex =
        custom.is_some_and(|preamble| preamble::loaded_packages(preamble).contains("biblatex"));
    if let Some(preamble) = custom {
        exporter.predefined_acronyms = defined_acronym_keys(preamble);
    }
    let body = exporter.body(doc);
    let generated_preamble =
        preamble::generate(options.settings, &exporter.usage, options.addon_preamble);
    let used_preamble = match custom {
        Some(preamble) => format!("{}\n", preamble.trim_end()),
        None => generated_preamble.clone(),
    };
    let latex = format!("{used_preamble}\\begin{{document}}\n\n{body}\n\n\\end{{document}}\n");
    let missing = custom
        .map(|preamble| preamble::missing_packages(preamble, &exporter.usage))
        .unwrap_or_default();
    let mut warnings = exporter.warnings;
    if !missing.is_empty() {
        warnings.push(format!(
            "Die eigene Präambel lädt nicht alle benötigten Pakete: {}.",
            missing.join(", ")
        ));
    }
    ExportResult {
        latex,
        preamble: used_preamble,
        generated_preamble,
        body,
        assets: exporter.assets,
        warnings,
        missing_packages: missing,
    }
}

/// Zerlegt ein vollständiges Dokument in (Präambel, Körper, Rest).
pub fn split_document(source: &str) -> Option<(String, String, String)> {
    let begin = find_begin_document(source)?;
    let after_begin = source[begin..].find('}').map(|index| begin + index + 1)?;
    let end = source.rfind("\\end{document}")?;
    if end < after_begin {
        return None;
    }
    Some((
        source[..begin].to_string(),
        source[after_begin..end].to_string(),
        source[end + "\\end{document}".len()..].to_string(),
    ))
}

fn find_begin_document(source: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(index) = source[offset..].find("\\begin") {
        let start = offset + index;
        let rest = source[start + "\\begin".len()..].trim_start();
        if rest.starts_with("{document}") {
            // Auskommentiert?
            let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if !preamble::strip_comments(&source[line_start..start])
                .trim_end()
                .ends_with('%')
                && !source[line_start..start].contains('%')
            {
                return Some(start);
            }
        }
        offset = start + "\\begin".len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_with_text_instead_of_numbers_stay_compilable() {
        for valid in [
            "4.7",
            "1,5",
            "1{,}5",
            "-3e-4",
            "1.23(4)",
            "10 \\pm 2",
            "3 x 4",
            "2+3i",
            "12 345",
        ] {
            assert!(is_siunitx_number(valid), "{valid}");
        }
        for invalid in ["test", "", "abc1", "~", "x"] {
            assert!(!is_siunitx_number(invalid), "{invalid}");
        }
        let quantity = |command: &str, value: &str| serde_json::json!({ "type": "quantity", "attrs": { "command": command, "value": value, "value2": "5", "unit": "\\volt" } });
        assert_eq!(
            quantity_latex(&quantity("qty", "4.7")),
            "\\qty{4.7}{\\volt}"
        );
        assert_eq!(
            quantity_latex(&quantity("qty", "test")),
            "test~\\unit{\\volt}"
        );
        assert_eq!(quantity_latex(&quantity("SI", "test")), "test~\\si{\\volt}");
        assert_eq!(quantity_latex(&quantity("num", "viel")), "viel");
        assert_eq!(
            quantity_latex(&quantity("qtyrange", "a")),
            "a--5~\\unit{\\volt}"
        );
    }

    #[test]
    fn line_joins_guard_brackets_and_stars() {
        let lines = vec![
            "Titel".to_string(),
            "[Studiengang]".to_string(),
            "*Stern".to_string(),
            "normal".to_string(),
        ];
        assert_eq!(
            join_latex_lines(&lines),
            "Titel\\\\{}\n[Studiengang]\\\\{}\n*Stern\\\\\nnormal"
        );
        assert_eq!(join_latex_lines(&[]), "");
    }

    #[test]
    fn title_page_with_bracket_placeholders_is_safe() {
        let settings = DocumentSettings::default();
        let doc = json!({ "type": "doc", "content": [{ "type": "titlePage", "attrs": {
            "title": "[Titel]", "institution": "Hochschule Beispiel", "program": "[Studiengang]"
        }}]});
        let exported = export_document(
            &doc,
            &ExportOptions {
                settings: &settings,
                custom_preamble: None,
                addon_preamble: &[],
            },
        );
        assert!(exported
            .latex
            .contains("Hochschule Beispiel\\\\{}\n[Studiengang]"));
        assert!(!exported.latex.contains("\\\\\n["));
    }
}
