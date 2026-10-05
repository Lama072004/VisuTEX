//! LaTeX → Tiptap-JSON.
//!
//! Konservativer, verlustfreier Parser: Alles, was sich nicht eindeutig auf
//! einen Editor-Knoten abbilden lässt, bleibt als Roh-LaTeX (Block oder
//! Inline) exakt erhalten. Strukturierte VisuTeX-Bereiche werden über
//! Kommentar-Marker erkannt und nur dann als Knoten übernommen, wenn ihr Inhalt
//! unverändert der generierten Form entspricht.

use super::escape::{text_symbol, unicode_for_math_command};
use super::export::{
    assign_source_keys, class_has_chapters, option_value, split_document, Exporter, CITE_COMMANDS,
    QUANTITY_COMMANDS,
};
use super::markers;
use super::preamble::{strip_comments, style_for_bst};
use super::settings::{Bibliography, DocumentSettings, Margins};
use serde::Serialize;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub doc: Value,
    /// Text vor `\begin{document}` (None bei Fragmenten ohne Präambel).
    pub preamble: Option<String>,
    pub postamble: String,
    pub has_chapters: bool,
    /// Aus dem Code abgeleitete Einstellungen (Literatur, Spalten, …).
    pub settings_patch: Value,
    pub warnings: Vec<String>,
    pub raw_block_count: usize,
}

const BLOCK_ENVIRONMENTS: &[&str] = &[
    "itemize",
    "enumerate",
    "description",
    "quote",
    "quotation",
    "verse",
    "center",
    "flushleft",
    "flushright",
    "figure",
    "figure*",
    "table",
    "table*",
    "equation",
    "equation*",
    "align",
    "align*",
    "gather",
    "gather*",
    "multline",
    "multline*",
    "eqnarray",
    "eqnarray*",
    "displaymath",
    "tikzpicture",
    "circuitikz",
    "verbatim",
    "lstlisting",
    "minipage",
    "tabular",
    "tabular*",
    "tabularx",
    "longtable",
    "titlepage",
    "abstract",
    "multicols",
    "landscape",
    "thebibliography",
    "acronym",
    "wrapfigure",
    "subfigure",
];

const SECTIONING: &[&str] = &[
    "part",
    "chapter",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
];

const BLOCK_COMMANDS: &[&str] = &[
    "part",
    "chapter",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
    "tableofcontents",
    "listoffigures",
    "listoftables",
    "bibliography",
    "bibliographystyle",
    "printbibliography",
    "clearpage",
    "newpage",
    "cleardoublepage",
    "pagebreak",
    "maketitle",
    "appendix",
    "phantomsection",
];

const NAMED_COLORS: &[(&str, &str)] = &[
    ("black", "#000000"),
    ("white", "#FFFFFF"),
    ("red", "#FF0000"),
    ("green", "#00FF00"),
    ("blue", "#0000FF"),
    ("cyan", "#00FFFF"),
    ("magenta", "#FF00FF"),
    ("yellow", "#FFFF00"),
    ("gray", "#808080"),
    ("darkgray", "#404040"),
    ("lightgray", "#BFBFBF"),
    ("brown", "#BF8040"),
    ("lime", "#BFFF00"),
    ("olive", "#808000"),
    ("orange", "#FF8000"),
    ("pink", "#FFBFBF"),
    ("purple", "#BF0040"),
    ("teal", "#008080"),
    ("violet", "#800080"),
];

const PLACEHOLDER_START: char = '\u{E000}';
const PLACEHOLDER_END: char = '\u{E001}';

// ------------------------------------------------------------- Scanner-Helfer

/// Nächstes Zeichen ab Byteposition (UTF-8-sicher).
fn char_at(source: &str, index: usize) -> Option<char> {
    source.get(index..)?.chars().next()
}

fn byte_at(source: &str, index: usize) -> Option<u8> {
    source.as_bytes().get(index).copied()
}

/// Liest eine Gruppe ab `start` (muss auf `open` zeigen). Liefert (Inhalt, Ende).
pub fn read_group(source: &str, start: usize, open: u8, close: u8) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&open) {
        return None;
    }
    let mut depth = 1;
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index += 1;
                if let Some(character) = char_at(source, index) {
                    index += character.len_utf8();
                }
                continue;
            }
            b'%' => {
                let line_end = source[index..].find('\n')?;
                index += line_end + 1;
                continue;
            }
            b'{' if open != b'{' => {
                let (_, end) = read_group(source, index, b'{', b'}')?;
                index = end;
                continue;
            }
            byte if byte == open => depth += 1,
            byte if byte == close => {
                depth -= 1;
                if depth == 0 {
                    return Some((&source[start + 1..index], index + 1));
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn read_command_name(source: &str, start: usize) -> Option<(&str, usize)> {
    if byte_at(source, start) != Some(b'\\') {
        return None;
    }
    let rest = &source[start + 1..];
    let letters = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic() || *b == b'@')
        .count();
    if letters > 0 {
        return Some((&rest[..letters], start + 1 + letters));
    }
    let character = rest.chars().next()?;
    Some((
        &rest[..character.len_utf8()],
        start + 1 + character.len_utf8(),
    ))
}

fn skip_spaces(source: &str, position: usize, allow_newline: bool) -> usize {
    let bytes = source.as_bytes();
    let mut index = position;
    while index < bytes.len()
        && (bytes[index] == b' '
            || bytes[index] == b'\t'
            || (allow_newline && (bytes[index] == b'\n' || bytes[index] == b'\r')))
    {
        index += 1;
    }
    index
}

pub struct Environment<'s> {
    pub name: &'s str,
    pub content: &'s str,
    pub end: usize,
    pub full: &'s str,
}

fn line_is_commented_before(source: &str, index: usize) -> bool {
    let line_start = source[..index].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let prefix = &source[line_start..index];
    let mut escaped = false;
    for character in prefix.chars() {
        if character == '%' && !escaped {
            return true;
        }
        escaped = character == '\\' && !escaped;
    }
    false
}

/// Findet `\end{name}` passend zu `\begin{name}` (verschachtelt, Kommentare beachtend).
pub fn read_environment(source: &str, start: usize) -> Option<Environment<'_>> {
    let rest = source.get(start..)?;
    let after_begin = rest.strip_prefix("\\begin")?;
    let trimmed = after_begin.trim_start_matches([' ', '\t']);
    let offset = after_begin.len() - trimmed.len();
    let (name, name_end) = read_group(trimmed, 0, b'{', b'}')?;
    let body_start = start + "\\begin".len() + offset + name_end;
    let end_tag = format!("\\end{{{name}}}");
    if matches!(
        name,
        "verbatim" | "verbatim*" | "lstlisting" | "comment" | "Verbatim" | "minted"
    ) {
        let end_index = source[body_start..].find(&end_tag)? + body_start;
        let end = end_index + end_tag.len();
        return Some(Environment {
            name,
            content: &source[body_start..end_index],
            end,
            full: &source[start..end],
        });
    }
    let begin_tag = format!("\\begin{{{name}}}");
    let mut depth = 1;
    let mut cursor = body_start;
    loop {
        let next_begin = source[cursor..].find(&begin_tag).map(|i| i + cursor);
        let next_end = source[cursor..].find(&end_tag).map(|i| i + cursor)?;
        match next_begin {
            Some(begin) if begin < next_end => {
                if !line_is_commented_before(source, begin) {
                    depth += 1;
                }
                cursor = begin + begin_tag.len();
            }
            _ => {
                if !line_is_commented_before(source, next_end) {
                    depth -= 1;
                    if depth == 0 {
                        let end = next_end + end_tag.len();
                        return Some(Environment {
                            name,
                            content: &source[body_start..next_end],
                            end,
                            full: &source[start..end],
                        });
                    }
                }
                cursor = next_end + end_tag.len();
            }
        }
    }
}

/// Liest optionale `[..]` (o) und Pflichtargumente `{..}` (m).
fn read_arguments<'s>(
    source: &'s str,
    start: usize,
    spec: &str,
) -> Option<(Vec<Option<&'s str>>, usize)> {
    let mut index = start;
    let mut args = Vec::new();
    for kind in spec.chars() {
        let position = skip_spaces(source, index, false);
        if kind == 'o' {
            if byte_at(source, position) == Some(b'[') {
                let (value, end) = read_group(source, position, b'[', b']')?;
                args.push(Some(value));
                index = end;
            } else {
                args.push(None);
            }
        } else {
            let (value, end) = read_group(source, position, b'{', b'}')?;
            args.push(Some(value));
            index = end;
        }
    }
    Some((args, index))
}

fn strip_detokenize(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix("\\detokenize") {
        let inner = inner.trim_start();
        if let Some((content, end)) = read_group(inner, 0, b'{', b'}') {
            if end == inner.len() {
                return content.trim().to_string();
            }
        }
    }
    trimmed.to_string()
}

fn find_unescaped(source: &str, needle: &str, from: usize) -> Option<usize> {
    let mut index = from;
    while let Some(found) = source.get(index..)?.find(needle) {
        let position = index + found;
        let backslashes = source[from..position]
            .bytes()
            .rev()
            .take_while(|b| *b == b'\\')
            .count();
        if backslashes % 2 == 0 || needle.starts_with('\\') {
            return Some(position);
        }
        index = position + needle.len();
    }
    None
}

fn color_value(model: Option<&str>, value: &str) -> Option<String> {
    let value = value.trim();
    match model {
        Some("HTML") if value.len() == 6 && value.chars().all(|c| c.is_ascii_hexdigit()) => {
            Some(format!("#{}", value.to_ascii_uppercase()))
        }
        None => NAMED_COLORS
            .iter()
            .find(|(name, _)| *name == value)
            .map(|(_, hex)| hex.to_string()),
        _ => None,
    }
}

fn accent_mark(command: &str) -> Option<char> {
    Some(match command {
        "\"" => '\u{0308}',
        "'" => '\u{0301}',
        "`" => '\u{0300}',
        "^" => '\u{0302}',
        "~" => '\u{0303}',
        "=" => '\u{0304}',
        "." => '\u{0307}',
        "c" => '\u{0327}',
        "v" => '\u{030c}',
        "u" => '\u{0306}',
        "H" => '\u{030b}',
        "r" => '\u{030a}',
        "k" => '\u{0328}',
        _ => return None,
    })
}

fn compose(letter: char, mark: char) -> String {
    // Häufige Kombinationen direkt (ohne Unicode-Normalisierungs-Tabellen).
    let table: &[(char, char, char)] = &[
        ('a', '\u{0308}', 'ä'),
        ('o', '\u{0308}', 'ö'),
        ('u', '\u{0308}', 'ü'),
        ('A', '\u{0308}', 'Ä'),
        ('O', '\u{0308}', 'Ö'),
        ('U', '\u{0308}', 'Ü'),
        ('e', '\u{0308}', 'ë'),
        ('i', '\u{0308}', 'ï'),
        ('ı', '\u{0308}', 'ï'),
        ('y', '\u{0308}', 'ÿ'),
        ('e', '\u{0301}', 'é'),
        ('a', '\u{0301}', 'á'),
        ('i', '\u{0301}', 'í'),
        ('ı', '\u{0301}', 'í'),
        ('o', '\u{0301}', 'ó'),
        ('u', '\u{0301}', 'ú'),
        ('E', '\u{0301}', 'É'),
        ('e', '\u{0300}', 'è'),
        ('a', '\u{0300}', 'à'),
        ('o', '\u{0300}', 'ò'),
        ('u', '\u{0300}', 'ù'),
        ('E', '\u{0300}', 'È'),
        ('e', '\u{0302}', 'ê'),
        ('a', '\u{0302}', 'â'),
        ('o', '\u{0302}', 'ô'),
        ('i', '\u{0302}', 'î'),
        ('ı', '\u{0302}', 'î'),
        ('u', '\u{0302}', 'û'),
        ('n', '\u{0303}', 'ñ'),
        ('N', '\u{0303}', 'Ñ'),
        ('a', '\u{0303}', 'ã'),
        ('o', '\u{0303}', 'õ'),
        ('c', '\u{0327}', 'ç'),
        ('C', '\u{0327}', 'Ç'),
        ('s', '\u{030c}', 'š'),
        ('S', '\u{030c}', 'Š'),
        ('c', '\u{030c}', 'č'),
        ('C', '\u{030c}', 'Č'),
        ('z', '\u{030c}', 'ž'),
        ('Z', '\u{030c}', 'Ž'),
        ('r', '\u{030c}', 'ř'),
        ('e', '\u{030c}', 'ě'),
        ('a', '\u{030a}', 'å'),
        ('A', '\u{030a}', 'Å'),
        ('o', '\u{030b}', 'ő'),
        ('u', '\u{030b}', 'ű'),
    ];
    table
        .iter()
        .find(|(base, accent, _)| *base == letter && *accent == mark)
        .map(|(_, _, composed)| composed.to_string())
        .unwrap_or_else(|| format!("{letter}{mark}"))
}

fn letter_command(command: &str) -> Option<&'static str> {
    Some(match command {
        "ss" => "ß",
        "aa" => "å",
        "AA" => "Å",
        "ae" => "æ",
        "AE" => "Æ",
        "oe" => "œ",
        "OE" => "Œ",
        "o" => "ø",
        "O" => "Ø",
        "l" => "ł",
        "L" => "Ł",
        "i" => "ı",
        _ => return None,
    })
}

fn german_shorthand(next: char) -> Option<&'static str> {
    Some(match next {
        'a' => "ä",
        'o' => "ö",
        'u' => "ü",
        'A' => "Ä",
        'O' => "Ö",
        'U' => "Ü",
        's' | 'z' => "ß",
        '`' => "„",
        '\'' => "“",
        '<' => "«",
        '>' => "»",
        '-' => "\u{00ad}",
        _ => return None,
    })
}

// ------------------------------------------------------------------ Marken

#[derive(Clone, Default, PartialEq)]
struct Marks {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    superscript: bool,
    subscript: bool,
    code: bool,
    link: Option<String>,
    color: Option<String>,
    background: Option<String>,
    font_family: Option<String>,
    font_size: Option<String>,
}

impl Marks {
    fn to_json(&self) -> Vec<Value> {
        let mut list = Vec::new();
        if let Some(href) = &self.link {
            list.push(json!({ "type": "link", "attrs": { "href": href } }));
        }
        for (flag, name) in [
            (self.bold, "bold"),
            (self.italic, "italic"),
            (self.underline, "underline"),
            (self.strike, "strike"),
            (self.superscript, "superscript"),
            (self.subscript, "subscript"),
            (self.code, "code"),
        ] {
            if flag {
                list.push(json!({ "type": name }));
            }
        }
        if self.color.is_some()
            || self.background.is_some()
            || self.font_family.is_some()
            || self.font_size.is_some()
        {
            list.push(json!({
                "type": "textStyle",
                "attrs": {
                    "color": self.color,
                    "backgroundColor": self.background,
                    "fontFamily": self.font_family,
                    "fontSize": self.font_size,
                    "userSetColor": self.color.is_some(),
                }
            }));
        }
        list
    }
}

fn text_node(text: &str, marks: &Marks) -> Value {
    let marks = marks.to_json();
    if marks.is_empty() {
        json!({ "type": "text", "text": text })
    } else {
        json!({ "type": "text", "text": text, "marks": marks })
    }
}

fn merge_text_nodes(nodes: Vec<Value>) -> Vec<Value> {
    let mut result: Vec<Value> = Vec::new();
    for node in nodes {
        if node["type"] == "text" {
            let text = node["text"].as_str().unwrap_or("");
            if text.is_empty() {
                continue;
            }
            if let Some(previous) = result.last_mut() {
                if previous["type"] == "text" && previous.get("marks") == node.get("marks") {
                    let merged = format!("{}{}", previous["text"].as_str().unwrap_or(""), text);
                    previous["text"] = Value::String(merged);
                    continue;
                }
            }
        }
        result.push(node);
    }
    for node in &mut result {
        if node["type"] == "text" {
            let text = node["text"].as_str().unwrap_or("").to_string();
            let mut collapsed = String::with_capacity(text.len());
            let mut previous_space = false;
            for character in text.chars() {
                if character == ' ' {
                    if previous_space {
                        continue;
                    }
                    previous_space = true;
                } else {
                    previous_space = false;
                }
                collapsed.push(character);
            }
            node["text"] = Value::String(collapsed);
        }
    }
    result
}

/// Entfernt führende/abschließende Leerzeichen eines Absatzes.
fn trim_inline_edges(nodes: &mut Vec<Value>) {
    while let Some(first) = nodes.first_mut() {
        if first["type"] != "text" {
            break;
        }
        let trimmed = first["text"]
            .as_str()
            .unwrap_or("")
            .trim_start()
            .to_string();
        if trimmed.is_empty() {
            nodes.remove(0);
        } else {
            first["text"] = Value::String(trimmed);
            break;
        }
    }
    while let Some(last) = nodes.last_mut() {
        if last["type"] != "text" {
            break;
        }
        let trimmed = last["text"].as_str().unwrap_or("").trim_end().to_string();
        if trimmed.is_empty() {
            nodes.pop();
        } else {
            last["text"] = Value::String(trimmed);
            break;
        }
    }
    for index in 1..nodes.len() {
        if nodes[index - 1]["type"] == "hardBreak" && nodes[index]["type"] == "text" {
            let trimmed = nodes[index]["text"]
                .as_str()
                .unwrap_or("")
                .trim_start()
                .to_string();
            nodes[index]["text"] = Value::String(trimmed);
        }
    }
    nodes.retain(|node| node["type"] != "text" || !node["text"].as_str().unwrap_or("").is_empty());
}

/// Zeilenweiser Präfixvergleich (Whitespace-tolerant); liefert den Rest oder None.
fn strip_leading_lines<'s>(source: &'s str, header: &str) -> Option<&'s str> {
    let mut rest = source;
    for line in header
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        rest = rest.trim_start();
        rest = rest.strip_prefix(line)?;
    }
    let trimmed = rest.trim_start_matches([' ', '\t']);
    Some(trimmed.strip_prefix('\n').unwrap_or(trimmed))
}

/// Bestandteile einer Gleitumgebung (figure/table) bzw. einer zentrierten Tabelle.
#[derive(Clone, Default)]
struct FloatParts {
    placement: Option<String>,
    centered: bool,
    /// (Kurzbeschriftung, Beschriftung) als LaTeX
    caption: Option<(Option<String>, String)>,
    caption_above: bool,
    label: Option<String>,
    size: Option<String>,
    stretch: Option<String>,
    /// (Umgebungsname bzw. "includegraphics", LaTeX)
    body: Option<(String, String)>,
}

/// Zerlegt den Inhalt einer Gleitumgebung. None, sobald etwas vorkommt, das der
/// Knoten nicht abbilden kann (dann bleibt die Umgebung Roh-LaTeX).
fn float_parts(content: &str, allow_placement: bool) -> Option<FloatParts> {
    let mut parts = FloatParts::default();
    let mut index = skip_spaces(content, 0, false);
    if allow_placement && byte_at(content, index) == Some(b'[') {
        let (value, end) = read_group(content, index, b'[', b']')?;
        parts.placement = Some(value.trim().to_string()).filter(|value| !value.is_empty());
        index = end;
    }
    loop {
        index = skip_spaces(content, index, true);
        if index >= content.len() {
            break;
        }
        // Kommentarzeilen (z. B. auskommentierte Varianten) überspringen – der
        // Originalcode bleibt über `sourceLatex` erhalten, solange nichts geändert wird.
        if byte_at(content, index) == Some(b'%') {
            index = content[index..]
                .find('\n')
                .map(|offset| index + offset + 1)
                .unwrap_or(content.len());
            continue;
        }
        let (name, name_end) = read_command_name(content, index)?;
        match name {
            "centering" => {
                parts.centered = true;
                index = name_end;
            }
            "caption" => {
                if parts.caption.is_some() {
                    return None;
                }
                let (args, end) = read_arguments(content, name_end, "om")?;
                let mut long = args[1]?.trim().to_string();
                // \caption{…\label{x}}
                let labels = find_labels(&long);
                if labels.len() == 1 && parts.label.is_none() {
                    let (position, value) = labels[0];
                    let label_text = format!("\\label{{{value}}}");
                    if long[position..].trim_end() == label_text {
                        parts.label = Some(value.trim().to_string());
                        long = long[..position].trim_end().to_string();
                    }
                }
                parts.caption = Some((args[0].map(|s| s.trim().to_string()), long));
                parts.caption_above = parts.body.is_none();
                index = end;
            }
            "label" => {
                if parts.label.is_some() {
                    return None;
                }
                let (args, end) = read_arguments(content, name_end, "m")?;
                parts.label = Some(args[0]?.trim().to_string());
                index = end;
            }
            "renewcommand" => {
                // \renewcommand{\arraystretch}{1.3} bzw. \renewcommand\arraystretch{1.3}
                let mut position = skip_spaces(content, name_end, false);
                let target_end = if byte_at(content, position) == Some(b'{') {
                    let (target, end) = read_group(content, position, b'{', b'}')?;
                    if target.trim() != "\\arraystretch" {
                        return None;
                    }
                    end
                } else {
                    let (target, end) = read_command_name(content, position)?;
                    if target != "arraystretch" {
                        return None;
                    }
                    end
                };
                position = skip_spaces(content, target_end, false);
                let (value, end) = read_group(content, position, b'{', b'}')?;
                value.trim().parse::<f64>().ok()?;
                parts.stretch = Some(value.trim().to_string());
                index = end;
            }
            size if super::export::FONT_SIZE_COMMANDS.contains(&size) => {
                parts.size = Some(size.to_string());
                index = name_end;
            }
            "begin" => {
                if parts.body.is_some() {
                    return None;
                }
                let env = read_environment(content, index)?;
                parts.body = Some((env.name.to_string(), env.full.to_string()));
                index = env.end;
            }
            "includegraphics" => {
                if parts.body.is_some() {
                    return None;
                }
                let (_, end) = read_arguments(content, name_end, "om")?;
                parts.body = Some(("includegraphics".into(), content[index..end].to_string()));
                index = end;
            }
            _ => return None,
        }
    }
    Some(parts)
}

/// Abstandsbefehle zwischen Unterabbildungen.
const SPACING_COMMANDS: &[&str] = &[
    "hfill",
    "quad",
    "qquad",
    "hspace",
    "par",
    "\\",
    "medskip",
    "bigskip",
    "smallskip",
    "vspace",
    "noindent",
];

/// Entfernt führende Abstandsbefehle (`\hfill`, `\quad`, `\hspace{…}`, `\\` …).
fn strip_leading_spacing(text: &str) -> &str {
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '~');
        let Some(after) = trimmed.strip_prefix('\\') else {
            return trimmed;
        };
        let name_len = if after.starts_with('\\') {
            1
        } else {
            after.chars().take_while(char::is_ascii_alphabetic).count()
        };
        let name = &after[..name_len];
        if !SPACING_COMMANDS.contains(&name) {
            return trimmed;
        }
        let mut next = after[name_len..].trim_start_matches('*');
        if matches!(name, "hspace" | "vspace") {
            let Some((_, end)) = read_group(next, skip_spaces(next, 0, false), b'{', b'}') else {
                return trimmed;
            };
            next = &next[end..];
        }
        rest = next;
    }
}

/// Nur Abstände und Kommentare?
fn only_spacing(text: &str) -> bool {
    let without_comments: String = text
        .lines()
        .map(|line| line.split('%').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    strip_leading_spacing(&without_comments).trim().is_empty()
}

/// `0.48\linewidth` / `\textwidth` / `0.3\columnwidth` → Anteil der Zeilenbreite.
fn relative_linewidth(value: &str) -> Option<f64> {
    let value = value.trim();
    ["\\linewidth", "\\textwidth", "\\columnwidth"]
        .iter()
        .find_map(|unit| {
            let factor = value.strip_suffix(unit)?.trim();
            if factor.is_empty() {
                Some(1.0)
            } else {
                factor.parse::<f64>().ok()
            }
        })
        .filter(|share| *share > 0.0 && *share <= 1.0)
}

/// Schriftgröße und Zeilenabstand einer übernommenen Tabelle.
fn apply_table_extras(table: &mut Value, parts: &FloatParts) {
    if let Some(size) = &parts.size {
        set_attr(table, "fontSize", json!(size));
    }
    if let Some(stretch) = &parts.stretch {
        set_attr(table, "arrayStretch", json!(stretch));
    }
}

/// Setzt ein Attribut (legt `attrs` bei Bedarf an).
fn set_attr(node: &mut Value, key: &str, value: Value) {
    if let Some(object) = node.as_object_mut() {
        let attrs = object.entry("attrs").or_insert_with(|| json!({}));
        if !attrs.is_object() {
            *attrs = json!({});
        }
        attrs[key] = value;
    }
}

/// Entfernt `{…}`-Gruppen (z. B. aus einer xparse-Argumentspezifikation).
fn strip_groups(text: &str) -> String {
    let mut depth = 0;
    text.chars()
        .filter(|character| {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    return false;
                }
                _ => {}
            }
            depth == 0 && *character != '{'
        })
        .collect()
}

/// Abstandsbefehl in eigener Zeile: `\vspace{1cm}`, `\vspace*{…}`, `\medskip`,
/// `\bigskip`, `\smallskip`, `\vfill`. Liefert Knoten und Ende.
fn standalone_spacing(source: &str, start: usize) -> Option<(Value, usize)> {
    let (name, name_end) = read_command_name(source, start)?;
    let (command, size, end) = match name {
        "medskip" | "bigskip" | "smallskip" | "vfill" => {
            (name.to_string(), String::new(), name_end)
        }
        "vspace" => {
            let mut index = name_end;
            let mut command = "vspace".to_string();
            if byte_at(source, index) == Some(b'*') {
                command.push('*');
                index += 1;
            }
            let (size, end) = read_group(source, skip_spaces(source, index, false), b'{', b'}')?;
            (command, size.trim().to_string(), end)
        }
        _ => return None,
    };
    // Rest der Zeile muss leer sein (sonst gehört der Befehl zum Absatz).
    let after = skip_spaces(source, end, false);
    if !(after >= source.len() || matches!(byte_at(source, after), Some(b'\n' | b'\r'))) {
        return None;
    }
    // Zeile muss mit dem Befehl beginnen.
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    if !source[line_start..start].trim().is_empty() {
        return None;
    }
    Some((
        json!({ "type": "verticalSpace", "attrs": { "command": command, "size": size } }),
        end,
    ))
}

// ----------------------------------------------------------------- Importer

pub struct Importer<'a> {
    settings: &'a DocumentSettings,
    has_chapters: bool,
    german_shorthands: bool,
    acronyms: Vec<(String, String, String)>,
    warnings: Vec<String>,
    patch: Map<String, Value>,
    placeholders: Vec<Vec<Value>>,
    raw_blocks: usize,
    exporter: Exporter<'a>,
    /// In der Präambel definierte Umgebungen (\newtcolorbox, \newenvironment,
    /// \newtheorem …): Name, Anzahl Pflichtargumente, optionales Argument.
    environments: Vec<(String, usize, bool)>,
}

/// Umgebungen, deren Inhalt normaler Text ist und die als Container-Knoten
/// (`environmentBlock`) visuell bearbeitet werden: (Name, Pflichtargumente,
/// optionales Argument).
const CONTAINER_ENVIRONMENTS: &[(&str, usize, bool)] = &[
    ("tcolorbox", 0, true),
    ("abstract", 0, false),
    ("quotation", 0, false),
    ("verse", 0, false),
    ("multicols", 1, true),
    ("multicols*", 1, true),
    ("minipage", 1, true),
    ("landscape", 0, false),
    ("small", 0, false),
    ("footnotesize", 0, false),
    ("scriptsize", 0, false),
    ("large", 0, false),
    ("Large", 0, false),
    ("onehalfspace", 0, false),
    ("doublespace", 0, false),
    ("singlespace", 0, false),
    ("spacing", 1, false),
    ("adjustwidth", 2, false),
    ("framed", 0, false),
    ("shaded", 0, false),
    ("mdframed", 0, true),
    ("proof", 0, true),
    ("center", 0, false),
    ("flushleft", 0, false),
    ("flushright", 0, false),
];

impl<'a> Importer<'a> {
    pub fn new(
        settings: &'a DocumentSettings,
        has_chapters: bool,
        german_shorthands: bool,
    ) -> Self {
        Self {
            settings,
            has_chapters,
            german_shorthands,
            acronyms: Vec::new(),
            warnings: Vec::new(),
            patch: Map::new(),
            placeholders: Vec::new(),
            raw_blocks: 0,
            exporter: Exporter::new(settings, has_chapters),
            environments: Vec::new(),
        }
    }

    /// Sammelt eigene Umgebungen aus der Präambel.
    pub fn collect_environments(&mut self, preamble: &str) {
        let text = strip_comments(preamble);
        for command in [
            "\\newtcolorbox",
            "\\renewtcolorbox",
            "\\DeclareTColorBox",
            "\\NewTColorBox",
            "\\newenvironment",
            "\\renewenvironment",
            "\\NewDocumentEnvironment",
            "\\newtheorem",
            "\\declaretheorem",
            "\\newtcbtheorem",
        ] {
            let mut offset = 0;
            while let Some(found) = text[offset..].find(command) {
                let start = offset + found + command.len();
                offset = start;
                if text[start..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                    continue;
                }
                let mut index = skip_spaces(&text, start, true);
                if byte_at(&text, index) == Some(b'*') {
                    index = skip_spaces(&text, index + 1, true);
                }
                // \newtcolorbox[init options]{name}
                if byte_at(&text, index) == Some(b'[') {
                    match read_group(&text, index, b'[', b']') {
                        Some((_, end)) => index = skip_spaces(&text, end, true),
                        None => continue,
                    }
                }
                let Some((name, name_end)) = read_group(&text, index, b'{', b'}') else {
                    continue;
                };
                let name = name.trim().to_string();
                if !super::export::is_environment_name(&name) {
                    continue;
                }
                let after = skip_spaces(&text, name_end, true);
                let (mandatory, optional) = match command {
                    "\\newtheorem" | "\\declaretheorem" => (0, true),
                    "\\newtcbtheorem" => (2, true),
                    "\\NewDocumentEnvironment" | "\\DeclareTColorBox" | "\\NewTColorBox" => {
                        // Argumentspezifikation {O{…} m}
                        let spec = read_group(&text, after, b'{', b'}')
                            .map(|(spec, _)| spec)
                            .unwrap_or("");
                        let spec = strip_groups(spec);
                        (spec.matches('m').count(), spec.contains(['o', 'O']))
                    }
                    _ => {
                        // \newenvironment{name}[n][default]{…}{…}
                        let mut count = 0;
                        let mut optional = false;
                        if let Some((value, end)) = read_group(&text, after, b'[', b']') {
                            count = value.trim().parse::<usize>().unwrap_or(0);
                            if byte_at(&text, skip_spaces(&text, end, true)) == Some(b'[') {
                                optional = true;
                                count = count.saturating_sub(1);
                            }
                        }
                        (count, optional)
                    }
                };
                self.environments
                    .retain(|(existing, _, _)| *existing != name);
                self.environments.push((name, mandatory, optional));
            }
        }
    }

    fn container_spec(&self, name: &str) -> Option<(usize, bool)> {
        self.environments
            .iter()
            .find(|(existing, _, _)| existing == name)
            .map(|(_, mandatory, optional)| (*mandatory, *optional))
            .or_else(|| {
                CONTAINER_ENVIRONMENTS
                    .iter()
                    .find(|(existing, _, _)| *existing == name)
                    .map(|(_, mandatory, optional)| (*mandatory, *optional))
            })
    }

    fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    // --------------------------------------------------------- Abkürzungen

    pub fn collect_acronyms(&mut self, source: &str) {
        for command in ["\\acro{", "\\acrodef{", "\\newacro{"] {
            let mut offset = 0;
            while let Some(found) = source[offset..].find(command) {
                let start = offset + found;
                offset = start + command.len();
                let Some((key, key_end)) =
                    read_group(source, start + command.len() - 1, b'{', b'}')
                else {
                    continue;
                };
                let mut index = skip_spaces(source, key_end, false);
                let mut short = key.trim().to_string();
                if byte_at(source, index) == Some(b'[') {
                    let Some((value, end)) = read_group(source, index, b'[', b']') else {
                        continue;
                    };
                    short = self
                        .plain_text(value)
                        .unwrap_or_else(|| value.trim().to_string());
                    index = skip_spaces(source, end, false);
                }
                let Some((long, _)) = read_group(source, index, b'{', b'}') else {
                    continue;
                };
                let key = key.trim().to_string();
                if !self
                    .acronyms
                    .iter()
                    .any(|(existing, _, _)| *existing == key)
                {
                    let long = self
                        .plain_text(long)
                        .unwrap_or_else(|| long.trim().to_string());
                    self.acronyms.push((key, short, long));
                }
            }
        }
    }

    // --------------------------------------------------------------- Marker

    fn bibliography_patch(&mut self) -> Bibliography {
        match self.patch.get("bibliography") {
            Some(value) => serde_json::from_value(value.clone())
                .unwrap_or_else(|_| self.settings.bibliography.clone()),
            None => self.settings.bibliography.clone(),
        }
    }

    /// Literaturdatei (`\addbibresource`), Titel und Autor aus einer fremden Präambel.
    pub fn read_preamble_metadata(&mut self, preamble: &str) {
        let text = strip_comments(preamble);
        let find_argument = |command: &str| -> Option<String> {
            let mut offset = 0;
            while let Some(found) = text[offset..].find(command) {
                let start = offset + found + command.len();
                offset = start;
                if text[start..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                    continue;
                }
                let (args, _) = read_arguments(&text, start, "om")?;
                return args[1].map(|value| value.trim().to_string());
            }
            None
        };
        if let Some(file) = find_argument("\\addbibresource").filter(|file| !file.is_empty()) {
            let mut bibliography = self.bibliography_patch();
            bibliography.file = file;
            self.patch.insert(
                "bibliography".into(),
                serde_json::to_value(bibliography).unwrap_or(Value::Null),
            );
        }
        let mut metadata = Map::new();
        for (command, key) in [("\\title", "title"), ("\\author", "author")] {
            if let Some(value) = find_argument(command) {
                let plain = latex_to_plain_text(&value.replace("\\\\", " "));
                if !plain.trim().is_empty() {
                    metadata.insert(key.into(), json!(plain.trim()));
                }
            }
        }
        if !metadata.is_empty() {
            self.patch
                .insert("metadata".into(), Value::Object(metadata));
        }
    }

    fn read_bibliography_commands(&mut self, source: &str) {
        let mut bibliography = self.bibliography_patch();
        if let Some(index) = source.find("\\bibliographystyle") {
            if let Some((style, _)) = read_group(
                source,
                skip_spaces(source, index + "\\bibliographystyle".len(), true),
                b'{',
                b'}',
            ) {
                match style_for_bst(style.trim()) {
                    Some(mapped) => bibliography.style = mapped.into(),
                    None => self.warn(format!(
                        "Literaturstil „{}“ wird nicht direkt unterstützt; verwendet wird „{}“.",
                        style.trim(),
                        bibliography.style
                    )),
                }
            }
        }
        if let Some(index) = source
            .find("\\bibliography{")
            .or_else(|| source.find("\\bibliography {"))
        {
            let position = skip_spaces(source, index + "\\bibliography".len(), true);
            if let Some((files, _)) = read_group(source, position, b'{', b'}') {
                if let Some(first) = files
                    .split(',')
                    .next()
                    .map(str::trim)
                    .filter(|file| !file.is_empty())
                {
                    bibliography.file = if first.to_ascii_lowercase().ends_with(".bib") {
                        first.to_string()
                    } else {
                        format!("{first}.bib")
                    };
                }
            }
        }
        self.patch.insert(
            "bibliography".into(),
            serde_json::to_value(bibliography).unwrap_or(Value::Null),
        );
    }

    /// Ersetzt Markerbereiche durch Platzhalter und berechnet die Knoten.
    fn replace_marker_regions(&mut self, source: &str) -> String {
        let regions = markers::find_regions(source);
        if regions.is_empty() {
            return source.to_string();
        }
        let mut columns: u8 = 1;
        let mut landscape = false;
        let mut output = String::with_capacity(source.len());
        let mut cursor = 0;
        for region in regions {
            output.push_str(&source[cursor..region.start]);
            cursor = region.end;
            let data = &region.data;
            let mut nodes: Option<Vec<Value>> = None;
            match region.kind.as_str() {
                "titlepage" => {
                    let expected = self.exporter.title_page_latex(data);
                    if markers::same_latex(&expected, &region.inner) {
                        let mut attrs = Map::new();
                        if let Some(object) = data.as_object() {
                            for (key, value) in object {
                                if value.is_string() {
                                    attrs.insert(key.clone(), value.clone());
                                }
                            }
                        }
                        nodes = Some(vec![json!({ "type": "titlePage", "attrs": attrs })]);
                    } else {
                        self.warn(
                            "Die Titelseite wurde im Code verändert und wird als LaTeX übernommen.",
                        );
                    }
                }
                "frontmatter" => {
                    let kind = data
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("abstract")
                        .to_string();
                    let title = data
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let in_toc = data.get("inToc").and_then(Value::as_bool).unwrap_or(false);
                    let header = self.exporter.frontmatter_header(&kind, &title, in_toc);
                    match strip_leading_lines(&region.inner, &header) {
                        Some(rest) => {
                            let rest = if kind == "confidentiality" {
                                rest.trim_end().strip_suffix("\\clearpage").unwrap_or(rest).to_string()
                            } else {
                                rest.to_string()
                            };
                            let mut content = self.parse_blocks(&rest);
                            if content.is_empty() {
                                content.push(json!({ "type": "paragraph" }));
                            }
                            nodes = Some(vec![json!({
                                "type": "frontmatterBlock",
                                "attrs": { "kind": kind, "title": title, "inToc": in_toc },
                                "content": content,
                            })]);
                        }
                        None => self.warn("Ein Vorspann-Bereich wurde im Code verändert und als normaler Inhalt übernommen."),
                    }
                }
                "directory" => {
                    let kind = data.get("kind").and_then(Value::as_str).unwrap_or("");
                    let in_toc = data.get("inToc").and_then(Value::as_bool) == Some(true);
                    let mut inner = Some(region.inner.trim().to_string());
                    if in_toc {
                        let prefix = self.exporter.directory_toc_prefix(data);
                        inner = inner.and_then(|inner| {
                            strip_leading_lines(&inner, &prefix).map(|rest| rest.trim().to_string())
                        });
                    }
                    let inner = inner.unwrap_or_default();
                    let printbibliography =
                        data.get("command").and_then(Value::as_str) == Some("printbibliography");
                    let simple = match kind {
                        "contents" => Some("\\tableofcontents"),
                        "figures" => Some("\\listoffigures"),
                        "tables" => Some("\\listoftables"),
                        _ => None,
                    };
                    let matches = match simple {
                        Some(command) => markers::same_latex(&inner, command),
                        None if kind == "acronyms" => inner.contains("\\begin{acronym}"),
                        None if kind == "bibliography" && printbibliography => {
                            inner.starts_with("\\printbibliography")
                        }
                        None if kind == "bibliography" => {
                            self.read_bibliography_commands(&inner);
                            true
                        }
                        None => false,
                    };
                    if matches {
                        let mut node =
                            json!({ "type": "directoryBlock", "attrs": { "kind": kind } });
                        for key in ["inToc", "tocTitle", "tocLevel", "command", "options"] {
                            if let Some(value) = data.get(key) {
                                set_attr(&mut node, key, value.clone());
                            }
                        }
                        nodes = Some(vec![node]);
                    }
                }
                "sectionbreak" => {
                    let to_columns = data
                        .get("columns")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0)
                        .clamp(1.0, 3.0) as u8;
                    let orientation = data
                        .get("orientation")
                        .and_then(Value::as_str)
                        .unwrap_or("keep")
                        .to_string();
                    let mut to_landscape = landscape;
                    if orientation == "landscape" && self.settings.orientation == "portrait" {
                        to_landscape = true;
                    }
                    if orientation == "portrait" {
                        to_landscape = false;
                    }
                    let expected =
                        Exporter::section_transition(columns, landscape, to_columns, to_landscape);
                    if markers::same_latex(&expected, &region.inner) {
                        nodes = Some(vec![json!({
                            "type": "pageBreak",
                            "attrs": { "breakType": "section", "columns": to_columns, "orientation": orientation }
                        })]);
                        columns = to_columns;
                        landscape = to_landscape;
                    } else {
                        self.warn("Ein Abschnittsumbruch wurde im Code verändert und wird als LaTeX übernommen.");
                    }
                }
                "layout-start" => {
                    let start_columns = data
                        .get("columns")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0)
                        .clamp(1.0, 3.0) as u8;
                    let expected = if start_columns > 1 {
                        format!("\\begin{{multicols}}{{{start_columns}}}")
                    } else {
                        String::new()
                    };
                    if markers::same_latex(&expected, &region.inner) {
                        columns = start_columns;
                        self.patch.insert("columns".into(), json!(start_columns));
                        nodes = Some(Vec::new());
                    }
                }
                "layout-end" => {
                    let mut closing = Vec::new();
                    if columns > 1 {
                        closing.push("\\end{multicols}");
                    }
                    if landscape {
                        closing.push("\\end{landscape}");
                    }
                    if markers::same_latex(&closing.join("\n"), &region.inner) {
                        nodes = Some(Vec::new());
                    }
                }
                "acronym-definitions" => nodes = Some(Vec::new()),
                "include" => {
                    // Eingebundene Datei (\input/\include) eines mehrteiligen Projekts.
                    let file = data
                        .get("file")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let command = match data.get("command").and_then(Value::as_str) {
                        Some("include") => "include",
                        Some("subfile") => "subfile",
                        _ => "input",
                    };
                    let mut content = self.parse_blocks(&region.inner);
                    if content.is_empty() {
                        content.push(json!({ "type": "paragraph" }));
                    }
                    let mut node = json!({
                        "type": "includeBlock",
                        "attrs": { "file": file, "command": command },
                        "content": content,
                    });
                    // \subfile: Präambel der Teildatei bleibt erhalten
                    for key in ["preamble", "postamble"] {
                        if let Some(value) = data.get(key).filter(|value| value.is_string()) {
                            set_attr(&mut node, key, value.clone());
                        }
                    }
                    nodes = Some(vec![node]);
                }
                "auto-bibliography" => {
                    self.read_bibliography_commands(&region.inner);
                    nodes = Some(Vec::new());
                }
                _ => {}
            }
            let nodes = match nodes {
                Some(nodes) => nodes,
                None => self.parse_blocks(&region.inner),
            };
            self.placeholders.push(nodes);
            output.push_str(&format!(
                "\n\n{PLACEHOLDER_START}{}{PLACEHOLDER_END}\n\n",
                self.placeholders.len() - 1
            ));
        }
        output.push_str(&source[cursor..]);
        output
    }

    // --------------------------------------------------------------- Blöcke

    fn raw_block(&mut self, nodes: &mut Vec<Value>, raw: &str, counts: bool) {
        let trimmed = raw.trim_start_matches('\n').trim_end();
        if trimmed.is_empty() {
            return;
        }
        nodes.push(json!({ "type": "rawLatexBlock", "attrs": { "rawLatex": trimmed } }));
        if counts {
            self.raw_blocks += 1;
        }
    }

    /// Merkt sich den Originalcode eines eben erzeugten Blocks (`sourceLatex`)
    /// und ob er im Original ohne Leerzeile auf den vorigen folgt (`sourceTight`).
    fn attach_source(
        &self,
        nodes: &mut [Value],
        first: usize,
        source: &str,
        span: (usize, usize),
        tight: bool,
    ) {
        if nodes.len() <= first {
            return;
        }
        if tight && first > 0 {
            set_attr(&mut nodes[first], "sourceTight", json!(true));
        }
        if nodes.len() != first + 1 || nodes[first]["type"] == "rawLatexBlock" {
            return;
        }
        let (start, end) = span;
        let Some(slice) = source.get(start..end.min(source.len())) else {
            return;
        };
        let slice = slice.trim_end();
        if slice.is_empty() || slice.contains([PLACEHOLDER_START, PLACEHOLDER_END]) {
            return;
        }
        set_attr(&mut nodes[first], "sourceLatex", json!(slice));
    }

    pub fn parse_blocks(&mut self, input: &str) -> Vec<Value> {
        let source = self.replace_marker_regions(input);
        let source = source.as_str();
        let mut nodes: Vec<Value> = Vec::new();
        let mut position = 0;
        let length = source.len();
        // Zuletzt begonnener Block: (Start, Anzahl Knoten davor, ohne Leerzeile angeschlossen)
        let mut pending: Option<(usize, usize, bool)> = None;
        let mut previous_start = 0;
        while position < length {
            if let Some((start, first, tight)) = pending.take() {
                self.attach_source(&mut nodes, first, source, (start, position), tight);
                previous_start = start;
            }
            position = skip_spaces(source, position, true);
            if position >= length {
                break;
            }
            // Leerraum zwischen dem vorigen Inhalt und diesem Block (Kommentarblöcke
            // enden nach ihrem Zeilenumbruch – deshalb vom Textende aus zählen).
            let content_end = source[..position].trim_end().len().max(previous_start);
            let tight =
                source[content_end..position].matches('\n').count() < 2 && !nodes.is_empty();
            pending = Some((position, nodes.len(), tight));
            let rest = &source[position..];

            // Abstand in eigener Zeile (\vspace{…}, \medskip, …); \vspace{\baselineskip}
            // ist ein leerer Absatz (so exportiert VisuTeX leere Absätze).
            if let Some((node, end)) = standalone_spacing(source, position)
                .filter(|_| !rest.starts_with("\\vspace{\\baselineskip}"))
            {
                nodes.push(node);
                position = end;
                continue;
            }
            // \noindent vor einer Tabelle gehört zur Tabelle
            if let Some(after) = rest.strip_prefix("\\noindent") {
                let next = skip_spaces(after, 0, true);
                if after[next..].starts_with("\\begin{tabular") {
                    if let Some(env) =
                        read_environment(source, position + "\\noindent".len() + next)
                    {
                        let (full, end) = (env.full.to_string(), env.end);
                        if let Some(table) = self.table(&full) {
                            nodes.push(table);
                            position = end;
                            continue;
                        }
                    }
                }
            }

            if rest.starts_with(PLACEHOLDER_START) {
                let end = rest.find(PLACEHOLDER_END).unwrap_or(rest.len());
                let index: usize = rest[PLACEHOLDER_START.len_utf8()..end]
                    .parse()
                    .unwrap_or(usize::MAX);
                if let Some(placeholder) = self.placeholders.get(index) {
                    nodes.extend(placeholder.clone());
                }
                position += end + PLACEHOLDER_END.len_utf8();
                continue;
            }

            // Kommentare (auch alte Marker der Vorversion)
            if rest.starts_with('%') {
                let mut end = position;
                let mut kept: Vec<&str> = Vec::new();
                while end < length && byte_at(source, skip_spaces(source, end, false)) == Some(b'%')
                {
                    let line_end = source[end..].find('\n').map(|i| end + i).unwrap_or(length);
                    let line = &source[end..line_end];
                    let trimmed = line.trim();
                    if let Some(spec) = trimmed.strip_prefix("%VISUTEX_SECTION_BREAK:") {
                        let mut fields = spec.split(':');
                        let columns = fields
                            .next()
                            .and_then(|v| v.parse::<u8>().ok())
                            .unwrap_or(1)
                            .clamp(1, 3);
                        let orientation = fields
                            .next()
                            .filter(|o| ["keep", "portrait", "landscape"].contains(o))
                            .unwrap_or("keep");
                        if !kept.is_empty() {
                            let raw = kept.join("\n");
                            self.raw_block(&mut nodes, &raw, false);
                            kept.clear();
                        }
                        nodes.push(json!({ "type": "pageBreak", "attrs": { "breakType": "section", "columns": columns, "orientation": orientation } }));
                    } else if ["%VISUTEX_FRONTMATTER", "%VISUTEX_ACRONYMS", "%VISUTEX_BODY"]
                        .contains(&trimmed)
                        || trimmed.starts_with(markers::PREFIX_BEGIN.trim_end())
                        || trimmed.starts_with(markers::PREFIX_END.trim_end())
                    {
                        // alte bzw. verwaiste Marker verwerfen
                    } else {
                        kept.push(line);
                    }
                    end = if line_end < length {
                        line_end + 1
                    } else {
                        length
                    };
                }
                if !kept.is_empty() {
                    let raw = kept.join("\n");
                    self.raw_block(&mut nodes, &raw, false);
                }
                position = end;
                continue;
            }

            // Display-Mathematik
            if rest.starts_with("\\[") {
                match find_unescaped(source, "\\]", position + 2) {
                    Some(end) => {
                        let latex = source[position + 2..end].trim();
                        nodes.push(json!({ "type": "mathBlock", "attrs": { "latex": latex, "numbered": false, "label": "", "environment": "equation" } }));
                        position = end + 2;
                    }
                    None => {
                        self.warn("Nicht geschlossene Display-Formel als LaTeX übernommen.");
                        self.raw_block(&mut nodes, rest, true);
                        break;
                    }
                }
                continue;
            }
            if rest.starts_with("$$") {
                match source[position + 2..].find("$$") {
                    Some(offset) => {
                        let end = position + 2 + offset;
                        let latex = source[position + 2..end].trim();
                        nodes.push(json!({ "type": "mathBlock", "attrs": { "latex": latex, "numbered": false, "label": "", "environment": "equation" } }));
                        position = end + 2;
                    }
                    None => {
                        self.raw_block(&mut nodes, rest, true);
                        break;
                    }
                }
                continue;
            }

            // Horizontale Linie
            for rule in [
                "\\noindent\\rule{\\linewidth}{0.4pt}",
                "\\noindent\\rule{\\textwidth}{0.4pt}",
            ] {
                if rest.starts_with(rule) {
                    let after = skip_spaces(source, position + rule.len(), false);
                    if after >= length
                        || matches!(byte_at(source, after), Some(b'\n') | Some(b'\r'))
                    {
                        nodes.push(json!({ "type": "horizontalRule" }));
                        position += rule.len();
                        break;
                    }
                }
            }
            if position != skip_spaces(source, position, true)
                || source[position..].len() != rest.len()
            {
                continue;
            }

            // Umgebungen
            if rest.starts_with("\\begin") && rest["\\begin".len()..].trim_start().starts_with('{')
            {
                match read_environment(source, position) {
                    Some(env) => {
                        let (name, content, full, end) = (
                            env.name.to_string(),
                            env.content.to_string(),
                            env.full.to_string(),
                            env.end,
                        );
                        match self.environment(&name, &content, &full) {
                            Some(parsed) => nodes.extend(parsed),
                            None => self.raw_block(&mut nodes, &full, true),
                        }
                        position = end;
                    }
                    None => {
                        self.warn("Nicht geschlossene Umgebung: der restliche Code bleibt als LaTeX-Block erhalten.");
                        self.raw_block(&mut nodes, rest, true);
                        break;
                    }
                }
                continue;
            }

            // Blockbefehle
            if let Some((name, name_end)) = read_command_name(source, position) {
                if BLOCK_COMMANDS.contains(&name) {
                    let name = name.to_string();
                    if let Some((parsed, end)) =
                        self.block_command(source, position, &name, name_end)
                    {
                        nodes.extend(parsed);
                        position = end;
                        continue;
                    }
                }
            }

            // Formatgruppe {\centering … \par} bzw. Rahmen/Schattierung
            if let Some((node, end)) = self.formatted_paragraph(source, position) {
                nodes.push(node);
                position = end;
                continue;
            }

            // Absatz
            let end = self.paragraph_end(source, position);
            let end = if end <= position {
                (position + 1..=length)
                    .find(|i| source.is_char_boundary(*i))
                    .unwrap_or(length)
            } else {
                end
            };
            let text = &source[position..end];
            match self.paragraph(text) {
                Some(paragraph) => nodes.push(paragraph),
                None => {
                    let text = text.to_string();
                    self.raw_block(&mut nodes, &text, true);
                }
            }
            position = end;
        }
        if let Some((start, first, tight)) = pending.take() {
            self.attach_source(&mut nodes, first, source, (start, position), tight);
        }
        nodes
    }

    /// Absatzende: Leerzeile oder Beginn eines Blockelements (Klammertiefe 0).
    fn paragraph_end(&self, source: &str, start: usize) -> usize {
        let bytes = source.as_bytes();
        let mut depth: i32 = 0;
        let mut index = start;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => {
                    if depth == 0 && index > start {
                        if let Some((name, _)) = read_command_name(source, index) {
                            if name == "begin" {
                                let after = source[index + 6..].trim_start();
                                if let Some((env, _)) = read_group(after, 0, b'{', b'}') {
                                    if BLOCK_ENVIRONMENTS.contains(&env) {
                                        return index;
                                    }
                                }
                            } else if BLOCK_COMMANDS.contains(&name) || name == "[" {
                                return index;
                            }
                        }
                    }
                    if source[index..].starts_with("\\verb")
                        && !source[index + 5..].starts_with(|c: char| c.is_ascii_alphabetic())
                    {
                        if let Some(delimiter) = char_at(source, index + 5) {
                            let search_from = index + 5 + delimiter.len_utf8();
                            index = source[search_from..]
                                .find(delimiter)
                                .map(|i| search_from + i + delimiter.len_utf8())
                                .unwrap_or(bytes.len());
                            continue;
                        }
                    }
                    index += 1;
                    if let Some(character) = char_at(source, index) {
                        index += character.len_utf8();
                    }
                    continue;
                }
                b'%' => match source[index..].find('\n') {
                    Some(offset) => {
                        index += offset + 1;
                        continue;
                    }
                    None => return bytes.len(),
                },
                b'{' => depth += 1,
                b'}' => depth = (depth - 1).max(0),
                b'$' if depth == 0 && index > start && bytes.get(index + 1) == Some(&b'$') => {
                    return index
                }
                b'\n' if depth == 0 => {
                    let next = skip_spaces(source, index + 1, false);
                    if matches!(bytes.get(next), Some(b'\n') | Some(b'\r')) {
                        return index;
                    }
                    // Abstand in eigener Zeile beendet den Absatz (eigener Block).
                    if bytes.get(next) == Some(&b'\\') && standalone_spacing(source, next).is_some()
                    {
                        return index;
                    }
                }
                _ => {
                    if source[index..].starts_with(PLACEHOLDER_START) && depth == 0 {
                        return index;
                    }
                }
            }
            index += 1;
            while index < bytes.len() && !source.is_char_boundary(index) {
                index += 1;
            }
        }
        bytes.len()
    }

    fn paragraph(&mut self, text: &str) -> Option<Value> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed == "\\vspace{\\baselineskip}" {
            return Some(json!({ "type": "paragraph" }));
        }
        // Trennlinie in eigener Zeile
        let rule = trimmed
            .strip_prefix("\\noindent")
            .unwrap_or(trimmed)
            .trim_start();
        if trimmed == "\\hrule"
            || ["\\rule{\\linewidth}{", "\\rule{\\textwidth}{"]
                .iter()
                .any(|prefix| {
                    rule.strip_prefix(prefix).is_some_and(|thickness| {
                        thickness.ends_with('}')
                            && !thickness[..thickness.len() - 1].contains(['{', '}', '\\'])
                    })
                })
        {
            return Some(json!({ "type": "horizontalRule" }));
        }
        let mut content = self.parse_inline(trimmed, &Marks::default());
        trim_inline_edges(&mut content);
        if content.is_empty() {
            return None;
        }
        Some(json!({ "type": "paragraph", "content": content }))
    }

    fn formatted_paragraph(&mut self, source: &str, start: usize) -> Option<(Value, usize)> {
        let rest = &source[start..];
        for (prefix, kind) in [
            ("\\noindent\\fcolorbox", "fcolorbox"),
            ("\\noindent\\colorbox", "colorbox"),
            ("\\noindent\\fbox", "fbox"),
        ] {
            if !rest.starts_with(prefix) {
                continue;
            }
            let mut index = start + prefix.len();
            let mut shading: Option<String> = None;
            if kind != "fbox" {
                let (model, end) = read_group(source, index, b'[', b']')?;
                if model != "HTML" {
                    return None;
                }
                index = end;
                if kind == "fcolorbox" {
                    let (frame, end) = read_group(source, index, b'{', b'}')?;
                    if frame != "808080" {
                        return None;
                    }
                    index = end;
                }
                let (color, end) = read_group(source, index, b'{', b'}')?;
                if color.len() != 6 || !color.chars().all(|c| c.is_ascii_hexdigit()) {
                    return None;
                }
                shading = Some(format!("#{}", color.to_ascii_lowercase()));
                index = end;
            }
            let (boxed, box_end) = read_group(source, index, b'{', b'}')?;
            let parbox_prefix = [
                "\\parbox{\\dimexpr\\linewidth-2\\fboxsep-2\\fboxrule\\relax}",
                "\\parbox{\\dimexpr\\linewidth-2\\fboxsep\\relax}",
            ]
            .into_iter()
            .find(|prefix| boxed.starts_with(prefix))?;
            let (inner, inner_end) = read_group(boxed, parbox_prefix.len(), b'{', b'}')?;
            if inner_end != boxed.len() {
                return None;
            }
            let mut paragraph = match self.formatted_paragraph(inner, 0) {
                Some((node, end)) if end == inner.len() => node,
                _ => self.paragraph(inner)?,
            };
            let attrs = paragraph
                .as_object_mut()?
                .entry("attrs")
                .or_insert_with(|| json!({}));
            attrs["border"] = if kind == "colorbox" {
                Value::Null
            } else {
                json!("single")
            };
            attrs["paragraphShading"] = shading.map(Value::String).unwrap_or(Value::Null);
            return Some((paragraph, box_end));
        }
        if !rest.starts_with('{') {
            return None;
        }
        let (group, end) = read_group(source, start, b'{', b'}')?;
        let group_trimmed = group.trim_end();
        let body_with_par = group_trimmed.strip_suffix("\\par")?;
        let mut body = body_with_par;
        let mut attrs = Map::new();
        let mut matched = false;
        loop {
            body = body.trim_start();
            let mut consumed = 0;
            for (command, key, value) in [
                ("\\centering", "textAlign", "center"),
                ("\\raggedleft", "textAlign", "right"),
                ("\\raggedright", "textAlign", "left"),
                ("\\singlespacing", "lineHeight", "1"),
                ("\\onehalfspacing", "lineHeight", "1.5"),
                ("\\doublespacing", "lineHeight", "2"),
            ] {
                if body.starts_with(command)
                    && !body[command.len()..].starts_with(|c: char| c.is_ascii_alphabetic())
                {
                    attrs.insert(key.into(), json!(value));
                    consumed = command.len();
                    break;
                }
            }
            if consumed == 0 {
                if let Some(after) = body.strip_prefix("\\setstretch") {
                    if let Some((value, end)) = read_group(after, 0, b'{', b'}') {
                        if value.parse::<f64>().is_ok() {
                            attrs.insert("lineHeight".into(), json!(value));
                            consumed = "\\setstretch".len() + end;
                        }
                    }
                }
            }
            if consumed == 0 {
                for prefix in ["\\addtolength{\\leftskip}", "\\setlength{\\leftskip}"] {
                    if let Some(after) = body.strip_prefix(prefix) {
                        if let Some((value, end)) = read_group(after, 0, b'{', b'}') {
                            if let Some(centimeters) =
                                value.strip_suffix("cm").and_then(|v| v.parse::<f64>().ok())
                            {
                                attrs.insert(
                                    "indent".into(),
                                    json!(((centimeters / 1.25).round() as i64).max(1)),
                                );
                                consumed = prefix.len() + end;
                            }
                        }
                    }
                }
            }
            if consumed == 0 {
                break;
            }
            matched = true;
            body = &body[consumed..];
        }
        if !matched {
            return None;
        }
        let mut paragraph = self.paragraph(body)?;
        let object = paragraph.as_object_mut()?;
        let paragraph_attrs = object.entry("attrs").or_insert_with(|| json!({}));
        for (key, value) in attrs {
            paragraph_attrs[key] = value;
        }
        Some((paragraph, end))
    }

    fn block_command(
        &mut self,
        source: &str,
        start: usize,
        name: &str,
        name_end: usize,
    ) -> Option<(Vec<Value>, usize)> {
        if SECTIONING.contains(&name) {
            if name == "part" || (!self.has_chapters && name == "chapter") {
                return None;
            }
            let mut index = name_end;
            let mut numbered = true;
            if byte_at(source, index) == Some(b'*') {
                numbered = false;
                index += 1;
            }
            index = skip_spaces(source, index, true);
            if byte_at(source, index) == Some(b'[') {
                return None;
            }
            let (title, title_end) = read_group(source, index, b'{', b'}')?;
            let mut end = title_end;
            let mut label = String::new();
            let after = skip_spaces(source, title_end, true);
            if source[after..].starts_with("\\label") {
                let position = skip_spaces(source, after + "\\label".len(), false);
                if let Some((value, label_end)) = read_group(source, position, b'{', b'}') {
                    label = value.trim().to_string();
                    end = label_end;
                }
            }
            let level = if self.has_chapters {
                match name {
                    "chapter" => 1,
                    "section" => 2,
                    "subsection" => 3,
                    "subsubsection" => 4,
                    "paragraph" => 5,
                    _ => 6,
                }
            } else {
                match name {
                    "section" => 1,
                    "subsection" => 2,
                    "subsubsection" => 3,
                    "paragraph" => 4,
                    _ => 5,
                }
            };
            let mut content = self.parse_inline(title.trim(), &Marks::default());
            trim_inline_edges(&mut content);
            let mut heading = json!({ "type": "heading", "attrs": { "level": level, "numbered": numbered, "label": label } });
            if !content.is_empty() {
                heading["content"] = Value::Array(content);
            }
            // \section*{…} \addcontentsline{toc}{section}{…}: trotzdem im Inhaltsverzeichnis
            if !numbered {
                let after = skip_spaces(source, end, true);
                if let Some(rest) = source[after..].strip_prefix("\\addcontentsline") {
                    if let Some((args, args_end)) = read_arguments(rest, 0, "mmm") {
                        if args[0].map(str::trim) == Some("toc")
                            && args[1].map(str::trim) == Some(name)
                        {
                            let entry = args[2].unwrap_or("").trim();
                            set_attr(&mut heading, "inToc", json!(true));
                            if entry != title.trim() {
                                set_attr(&mut heading, "tocTitle", json!(entry));
                            }
                            end = after + "\\addcontentsline".len() + args_end;
                        }
                    }
                }
            }
            return Some((vec![heading], end));
        }
        match name {
            "tableofcontents" | "listoffigures" | "listoftables" => {
                let kind = match name {
                    "tableofcontents" => "contents",
                    "listoffigures" => "figures",
                    _ => "tables",
                };
                Some((
                    vec![json!({ "type": "directoryBlock", "attrs": { "kind": kind } })],
                    name_end,
                ))
            }
            "bibliographystyle" | "bibliography" => {
                let mut end = name_end;
                let position = skip_spaces(source, end, true);
                let (_, group_end) = read_group(source, position, b'{', b'}')?;
                end = group_end;
                if name == "bibliographystyle" {
                    let after = skip_spaces(source, end, true);
                    if !source[after..].starts_with("\\bibliography")
                        || source[after..].starts_with("\\bibliographystyle")
                    {
                        return None;
                    }
                    let position = skip_spaces(source, after + "\\bibliography".len(), true);
                    let (_, group_end) = read_group(source, position, b'{', b'}')?;
                    end = group_end;
                }
                let commands = source[start..end].to_string();
                self.read_bibliography_commands(&commands);
                Some((
                    vec![json!({ "type": "directoryBlock", "attrs": { "kind": "bibliography" } })],
                    end,
                ))
            }
            "clearpage" | "newpage" | "cleardoublepage" | "pagebreak" => {
                if name == "pagebreak" && byte_at(source, name_end) == Some(b'[') {
                    return None;
                }
                Some((
                    vec![
                        json!({ "type": "pageBreak", "attrs": { "breakType": "page", "columns": 1, "orientation": "keep" } }),
                    ],
                    name_end,
                ))
            }
            "maketitle" => Some((vec![json!({ "type": "maketitle" })], name_end)),
            "appendix" => Some((vec![json!({ "type": "appendixMarker" })], name_end)),
            "printbibliography" => {
                let mut end = name_end;
                let mut options = String::new();
                let spaced = skip_spaces(source, end, false);
                if byte_at(source, spaced) == Some(b'[') {
                    let (value, group_end) = read_group(source, spaced, b'[', b']')?;
                    options = value.trim().to_string();
                    end = group_end;
                }
                Some((
                    vec![json!({ "type": "directoryBlock", "attrs": {
                        "kind": "bibliography", "command": "printbibliography", "options": options,
                    } })],
                    end,
                ))
            }
            "phantomsection" => {
                // \phantomsection \addcontentsline{toc}{Ebene}{Titel} \listoffigures …
                let after = skip_spaces(source, name_end, true);
                let rest = source[after..].strip_prefix("\\addcontentsline")?;
                let (args, args_end) = read_arguments(rest, 0, "mmm")?;
                if args[0]?.trim() != "toc" {
                    return None;
                }
                let level = args[1]?.trim().to_string();
                let title = args[2]?.trim().to_string();
                let next = skip_spaces(source, after + "\\addcontentsline".len() + args_end, true);
                let (directory, directory_end) = read_command_name(source, next)?;
                if !matches!(
                    directory,
                    "tableofcontents"
                        | "listoffigures"
                        | "listoftables"
                        | "printbibliography"
                        | "bibliographystyle"
                        | "bibliography"
                ) {
                    return None;
                }
                let directory = directory.to_string();
                let (mut nodes, end) =
                    self.block_command(source, next, &directory, directory_end)?;
                if nodes.len() != 1 || nodes[0]["type"] != "directoryBlock" {
                    return None;
                }
                set_attr(&mut nodes[0], "inToc", json!(true));
                set_attr(&mut nodes[0], "tocTitle", json!(title));
                set_attr(&mut nodes[0], "tocLevel", json!(level));
                Some((nodes, end))
            }
            _ => None,
        }
    }

    // ------------------------------------------------------- Umgebungen

    fn environment(&mut self, name: &str, content: &str, full: &str) -> Option<Vec<Value>> {
        match name {
            "equation" | "equation*" | "displaymath" => {
                let numbered = name == "equation";
                let labels: Vec<(usize, &str)> = find_labels(content);
                if labels.len() > 1 || (!numbered && !labels.is_empty()) {
                    return None;
                }
                let mut latex = content.to_string();
                let mut label = String::new();
                if let Some((index, value)) = labels.first() {
                    label = value.trim().to_string();
                    let label_text = format!("\\label{{{value}}}");
                    latex.replace_range(*index..*index + label_text.len(), "\n");
                }
                let latex = latex.trim().to_string();
                Some(vec![
                    json!({ "type": "mathBlock", "attrs": { "latex": latex, "numbered": numbered, "label": label, "environment": "equation" } }),
                ])
            }
            "align" | "align*" | "gather" | "gather*" | "multline" | "multline*" | "eqnarray"
            | "eqnarray*" => {
                let base = name.trim_end_matches('*');
                let numbered = !name.ends_with('*');
                let mut latex = content.trim().to_string();
                let mut label = String::new();
                let labels = find_labels(&latex);
                if numbered && labels.len() == 1 {
                    if let Some(index) = latex.rfind("\n\\label{") {
                        let tail = &latex[index + 1..];
                        if tail.trim_end().ends_with('}') && !tail.contains('\n') {
                            label = labels[0].1.trim().to_string();
                            latex = latex[..index].trim_end().to_string();
                        }
                    }
                }
                Some(vec![
                    json!({ "type": "mathBlock", "attrs": { "latex": latex, "numbered": numbered, "label": label, "environment": base } }),
                ])
            }
            "itemize" | "enumerate" | "description" => self.list(name, content),
            "quote" => {
                let mut blocks = self.parse_blocks(content);
                if blocks.is_empty() {
                    blocks.push(json!({ "type": "paragraph" }));
                }
                Some(vec![json!({ "type": "blockquote", "content": blocks })])
            }
            "center" | "flushleft" | "flushright" => {
                if name == "center" {
                    if let Some(table) = self.centered_table(content) {
                        return Some(vec![table]);
                    }
                }
                self.aligned_environment(name, content)
                    .or_else(|| self.container(name, content))
            }
            "figure" => self.figure(content),
            "table" => self.table_float(content),
            "lstlisting" | "minted" | "Verbatim" => self.listing(name, content),
            "tabular" | "tabularx" | "tabular*" => self.table(full).map(|table| vec![table]),
            "longtable" => self.long_table(content).map(|table| vec![table]),
            "tikzpicture" | "circuitikz" => {
                let trimmed = content.trim_start_matches([' ', '\t']);
                let (options, code) = if trimmed.starts_with('[') {
                    match read_group(trimmed, 0, b'[', b']') {
                        Some((options, end)) => (options.trim().to_string(), &trimmed[end..]),
                        None => (String::new(), trimmed),
                    }
                } else {
                    (String::new(), content)
                };
                let code = code
                    .strip_prefix('\n')
                    .unwrap_or(code)
                    .trim_end()
                    .to_string();
                let code = code.trim_start_matches(['\n', '\r']).to_string();
                Some(vec![
                    json!({ "type": "tikzBlock", "attrs": { "code": code, "environment": name, "options": options, "caption": "", "label": "" } }),
                ])
            }
            "verbatim" => {
                let text = content.strip_prefix('\n').unwrap_or(content);
                let text = text.strip_suffix('\n').unwrap_or(text);
                let mut block = json!({ "type": "codeBlock", "attrs": { "language": null } });
                if !text.is_empty() {
                    block["content"] = json!([{ "type": "text", "text": text }]);
                }
                Some(vec![block])
            }
            _ => self.container(name, content),
        }
    }

    /// Umgebung mit Textinhalt als Container (`environmentBlock`): tcolorbox,
    /// multicols, abstract, minipage, eigene Boxen aus der Präambel …
    fn container(&mut self, name: &str, content: &str) -> Option<Vec<Value>> {
        let (mandatory, optional) = self.container_spec(name)?;
        // Argumente direkt nach \begin{name} (nur in derselben Zeile)
        let mut index = 0;
        let mut read = 0;
        loop {
            let spaced = skip_spaces(content, index, false);
            match byte_at(content, spaced) {
                Some(b'[') if optional => {
                    let (_, end) = read_group(content, spaced, b'[', b']')?;
                    index = end;
                }
                Some(b'{') if read < mandatory => {
                    let (_, end) = read_group(content, spaced, b'{', b'}')?;
                    index = end;
                    read += 1;
                }
                _ => break,
            }
        }
        if read < mandatory {
            return None;
        }
        let args = content[..index].to_string();
        let mut blocks = self.parse_blocks(&content[index..]);
        if blocks.is_empty() {
            blocks.push(json!({ "type": "paragraph" }));
        }
        Some(vec![json!({
            "type": "environmentBlock",
            "attrs": { "name": name, "args": args },
            "content": blocks,
        })])
    }

    /// lstlisting / minted / Verbatim → Codeblock mit Sprache und Optionen.
    fn listing(&mut self, name: &str, content: &str) -> Option<Vec<Value>> {
        let mut rest = content;
        let mut options = String::new();
        let spaced = skip_spaces(rest, 0, false);
        if byte_at(rest, spaced) == Some(b'[') {
            let (value, end) = read_group(rest, spaced, b'[', b']')?;
            options = value.trim().to_string();
            rest = &rest[end..];
        }
        let mut language = option_value(&options, "language").unwrap_or_default();
        if name == "minted" {
            let spaced = skip_spaces(rest, 0, false);
            let (value, end) = read_group(rest, spaced, b'{', b'}')?;
            language = value.trim().to_string();
            rest = &rest[end..];
        }
        // Rest der Kopfzeile muss leer sein
        let line_end = rest.find('\n').unwrap_or(rest.len());
        if !rest[..line_end].trim().is_empty() {
            return None;
        }
        let text = &rest[(line_end + 1).min(rest.len())..];
        let text = text.strip_suffix('\n').unwrap_or(text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let language = language.trim().to_ascii_lowercase();
        let mut block = json!({ "type": "codeBlock", "attrs": {
            "language": if language.is_empty() { Value::Null } else { json!(language) },
            "environment": name,
            "listingOptions": options,
        } });
        if !text.is_empty() {
            block["content"] = json!([{ "type": "text", "text": text }]);
        }
        Some(vec![block])
    }

    /// Beschriftung: Klartext oder (mit Formeln, Einheiten …) LaTeX-Quelltext.
    fn caption_attrs(&mut self, short: Option<&str>, long: &str, node: &mut Value) {
        let long = long.trim();
        match self.plain_text(long) {
            Some(text) if short.is_none_or(|s| self.plain_text(s).is_some()) => {
                set_attr(node, "caption", json!(text));
                if let Some(short) = short.and_then(|s| self.plain_text(s)) {
                    set_attr(node, "shortCaption", json!(short));
                }
            }
            _ => {
                set_attr(node, "caption", json!(long));
                set_attr(node, "captionLatex", json!(true));
                if let Some(short) = short {
                    set_attr(node, "shortCaption", json!(short.trim()));
                }
            }
        }
    }

    /// Überträgt Platzierung, Zentrierung und Beschriftung einer Gleitumgebung.
    fn apply_float_parts(&mut self, node: &mut Value, parts: &FloatParts) {
        if parts.caption.is_none() && parts.label.is_none() {
            // Ohne Beschriftung wäre der Knoten sonst keine Gleitumgebung.
            set_attr(node, "wrapper", json!("float"));
        }
        set_attr(
            node,
            "placement",
            json!(parts.placement.clone().unwrap_or_else(|| "htbp".into())),
        );
        if !parts.centered {
            set_attr(node, "centered", json!(false));
        }
        set_attr(node, "caption", json!(""));
        set_attr(
            node,
            "label",
            json!(parts.label.clone().unwrap_or_default()),
        );
        if let Some((short, long)) = &parts.caption {
            self.caption_attrs(short.as_deref(), long, node);
            if parts.caption_above {
                set_attr(node, "captionAbove", json!(true));
            }
        }
    }

    /// `\begin{center}` mit genau einer Tabelle (ggf. `\small`, `\arraystretch`).
    fn centered_table(&mut self, content: &str) -> Option<Value> {
        let parts = float_parts(content, false)?;
        if parts.caption.is_some() || parts.label.is_some() {
            return None;
        }
        let (kind, latex) = parts.body.as_ref()?;
        if !matches!(kind.as_str(), "tabular" | "tabularx" | "tabular*") {
            return None;
        }
        let mut table = self.table(latex)?;
        set_attr(&mut table, "wrapper", json!("center"));
        apply_table_extras(&mut table, &parts);
        Some(table)
    }

    fn list(&mut self, name: &str, content: &str) -> Option<Vec<Value>> {
        let mut body = content;
        // enumitem-Optionen direkt nach \begin{…}: [leftmargin=1.5em], [label=\alph*)]
        let mut options = String::new();
        let spaced = skip_spaces(body, 0, false);
        if byte_at(body, spaced) == Some(b'[') {
            let (value, end) = read_group(body, spaced, b'[', b']')?;
            options = value.trim().to_string();
            body = &body[end..];
        }
        let mut start: Option<i64> = None;
        let trimmed = body.trim_start();
        if name == "enumerate" {
            for counter in ["enumi", "enumii", "enumiii", "enumiv"] {
                let prefix = format!("\\setcounter{{{counter}}}");
                if let Some(after) = trimmed.strip_prefix(&prefix) {
                    if let Some((value, end)) = read_group(after, 0, b'{', b'}') {
                        if let Ok(number) = value.trim().parse::<i64>() {
                            start = Some(number + 1);
                            body = &after[end..];
                        }
                    }
                    break;
                }
            }
        }
        let items = split_items(body)?;
        let mut list_items = Vec::new();
        for (label, text) in items {
            let mut text = text.as_str();
            let leading = text.trim_start().starts_with("\\mbox{}");
            if leading {
                text = text.trim_start().strip_prefix("\\mbox{}").unwrap_or(text);
            }
            let mut blocks = self.parse_blocks(text);
            if leading || blocks.is_empty() || blocks[0]["type"] != "paragraph" {
                blocks.insert(0, json!({ "type": "paragraph" }));
            }
            let mut item = json!({ "type": "listItem", "content": blocks });
            if let Some(label) = label {
                set_attr(&mut item, "itemLabel", json!(label.trim()));
            }
            list_items.push(item);
        }
        if list_items.is_empty() {
            return None;
        }
        let mut list = json!({ "type": if name == "enumerate" { "orderedList" } else { "bulletList" }, "content": list_items });
        if name == "enumerate" {
            list["attrs"] = json!({ "start": start.unwrap_or(1) });
        }
        if name == "description" {
            set_attr(&mut list, "listEnvironment", json!("description"));
        }
        if !options.is_empty() {
            set_attr(&mut list, "listOptions", json!(options));
        }
        Some(vec![list])
    }

    fn aligned_environment(&mut self, name: &str, content: &str) -> Option<Vec<Value>> {
        let inner = content.trim();
        if name == "center" {
            if let Some((options, path)) = single_includegraphics(inner) {
                return self.image_node(&options, &path).map(|image| vec![image]);
            }
            if inner.starts_with("\\begin{tikzpicture}") || inner.starts_with("\\begin{circuitikz}")
            {
                if let Some(env) = read_environment(inner, 0) {
                    if env.end == inner.len() {
                        let (env_name, env_content, env_full) = (
                            env.name.to_string(),
                            env.content.to_string(),
                            env.full.to_string(),
                        );
                        return self.environment(&env_name, &env_content, &env_full);
                    }
                }
            }
        }
        let blocks = self.parse_blocks(content);
        if blocks.is_empty() || blocks.iter().any(|block| block["type"] != "paragraph") {
            return None;
        }
        let align = match name {
            "center" => "center",
            "flushleft" => "left",
            _ => "right",
        };
        Some(
            blocks
                .into_iter()
                .map(|mut block| {
                    let attrs = block
                        .as_object_mut()
                        .map(|object| object.entry("attrs").or_insert_with(|| json!({})).clone());
                    let mut attrs = attrs.unwrap_or_else(|| json!({}));
                    attrs["textAlign"] = json!(align);
                    block["attrs"] = attrs;
                    block
                })
                .collect(),
        )
    }

    fn image_node(&self, options: &str, raw_path: &str) -> Option<Value> {
        let path = strip_detokenize(raw_path).trim_matches('"').to_string();
        if path.is_empty() {
            return None;
        }
        let mut attrs = json!({
            "src": "", "alt": path, "title": path, "latexPath": path, "caption": "", "label": "",
            "widthPercent": 100, "graphicsOptions": "", "placement": "htbp",
        });
        let options = options.trim();
        // Eigener Zusatz des Exports (Höhenbegrenzung) – gehört zur Breitenangabe.
        let options = options
            .strip_suffix(super::export::IMAGE_HEIGHT_LIMIT)
            .unwrap_or(options);
        if !options.is_empty() {
            let relative = options
                .strip_prefix("width=")
                .map(str::trim)
                .and_then(|value| {
                    ["\\linewidth", "\\textwidth", "\\columnwidth"]
                        .iter()
                        .find_map(|unit| {
                            let factor = value.strip_suffix(unit)?.trim();
                            if factor.is_empty() {
                                Some(1.0)
                            } else {
                                factor.parse::<f64>().ok()
                            }
                        })
                });
            match relative {
                Some(factor) => attrs["widthPercent"] = json!((factor * 100.0).round() as i64),
                None => attrs["graphicsOptions"] = json!(options),
            }
        }
        Some(json!({ "type": "image", "attrs": attrs }))
    }

    fn figure(&mut self, content: &str) -> Option<Vec<Value>> {
        if content.contains("\\begin{subfigure}") {
            return self.subfigures(content).map(|node| vec![node]);
        }
        let parts = float_parts(content, true)?;
        if parts.size.is_some() || parts.stretch.is_some() {
            return None;
        }
        let (kind, latex) = parts.body.clone()?;
        let mut node = match kind.as_str() {
            "includegraphics" => {
                let (options, path) = single_includegraphics(&latex)?;
                self.image_node(&options, &path)?
            }
            "tikzpicture" | "circuitikz" => {
                let env = read_environment(&latex, 0)?;
                let (name, env_content, full) = (
                    env.name.to_string(),
                    env.content.to_string(),
                    env.full.to_string(),
                );
                self.environment(&name, &env_content, &full)?
                    .into_iter()
                    .next()?
            }
            _ => return None,
        };
        self.apply_float_parts(&mut node, &parts);
        Some(vec![node])
    }

    /// `figure` mit `subfigure`-Umgebungen (subcaption) → Knoten `subfigures`.
    /// Zwischen den Unterabbildungen sind nur Abstände erlaubt (`\hfill`, `\quad`,
    /// `\hspace{…}`, `\\`, Leerzeilen); sonst bleibt die Umgebung Roh-LaTeX.
    fn subfigures(&mut self, content: &str) -> Option<Value> {
        let mut items = Vec::new();
        let mut outside = String::new();
        let mut index = 0;
        let mut previous_end: Option<usize> = None;
        while let Some(offset) = content[index..].find("\\begin{subfigure}") {
            let start = index + offset;
            let between = &content[index..start];
            if previous_end.is_some() {
                if !only_spacing(between) {
                    return None;
                }
            } else {
                outside.push_str(between);
            }
            let env = read_environment(content, start)?;
            if env.name != "subfigure" {
                return None;
            }
            items.push(self.subfigure_item(env.content)?);
            index = env.end;
            previous_end = Some(env.end);
        }
        let after = &content[index..];
        // Abstand direkt nach der letzten Unterabbildung (z. B. `\hfill` vor `\caption`) ignorieren
        let after_trimmed = strip_leading_spacing(after);
        outside.push('\n');
        outside.push_str(after_trimmed);
        let parts = float_parts(&outside, true)?;
        if parts.body.is_some()
            || parts.size.is_some()
            || parts.stretch.is_some()
            || items.is_empty()
        {
            return None;
        }
        let mut node = json!({ "type": "subfigures", "attrs": { "items": items } });
        self.apply_float_parts(&mut node, &parts);
        Some(node)
    }

    fn subfigure_item(&mut self, content: &str) -> Option<Value> {
        let mut index = skip_spaces(content, 0, true);
        let mut position = "t".to_string();
        if byte_at(content, index) == Some(b'[') {
            let (value, end) = read_group(content, index, b'[', b']')?;
            position = value.trim().to_string();
            index = end;
        }
        let index = skip_spaces(content, index, true);
        let (width, end) = read_group(content, index, b'{', b'}')?;
        let share = relative_linewidth(width)?;
        let parts = float_parts(&content[end..], false)?;
        if parts.size.is_some() || parts.stretch.is_some() {
            return None;
        }
        let (kind, latex) = parts.body.clone()?;
        if kind != "includegraphics" {
            return None;
        }
        let (options, path) = single_includegraphics(&latex)?;
        let mut node = self.image_node(&options, &path)?;
        if let Some(attrs) = node.get_mut("attrs").and_then(Value::as_object_mut) {
            attrs.remove("placement");
        }
        set_attr(
            &mut node,
            "boxPercent",
            json!((share * 100.0).round() as i64),
        );
        if position != "t" {
            if !matches!(position.as_str(), "c" | "b") {
                return None;
            }
            set_attr(&mut node, "position", json!(position));
        }
        if !parts.centered {
            set_attr(&mut node, "centered", json!(false));
        }
        set_attr(
            &mut node,
            "label",
            json!(parts.label.clone().unwrap_or_default()),
        );
        if let Some((short, long)) = &parts.caption {
            if short.is_some() || parts.caption_above {
                return None;
            }
            self.caption_attrs(None, long, &mut node);
        }
        node.get("attrs").cloned()
    }

    fn table_float(&mut self, content: &str) -> Option<Vec<Value>> {
        let parts = float_parts(content, true)?;
        let (kind, latex) = parts.body.clone()?;
        if !matches!(kind.as_str(), "tabular" | "tabularx" | "tabular*") {
            return None;
        }
        let mut table = self.table(&latex)?;
        self.apply_float_parts(&mut table, &parts);
        apply_table_extras(&mut table, &parts);
        Some(vec![table])
    }

    /// `longtable` (Tabelle über mehrere Seiten) → Tabellenknoten mit
    /// `breakAcrossPages`. Beschriftung/Label aus der ersten Zeile, der wiederholte
    /// Kopf (`\endfirsthead … \endhead`) wird nur einmal übernommen. Fuß-Bereiche
    /// (`\endfoot`) bleiben Roh-LaTeX.
    fn long_table(&mut self, content: &str) -> Option<Value> {
        let mut index = skip_spaces(content, 0, true);
        if byte_at(content, index) == Some(b'[') {
            let (_, end) = read_group(content, index, b'[', b']')?;
            index = skip_spaces(content, end, true);
        }
        let (spec, spec_end) = read_group(content, index, b'{', b'}')?;
        let mut rest = content[spec_end..].trim_start();
        let mut caption: Option<String> = None;
        let mut label: Option<String> = None;
        if let Some(after) = rest.strip_prefix("\\caption") {
            let (text, end) = read_group(after, 0, b'{', b'}')?;
            caption = Some(text.trim().to_string());
            rest = after[end..].trim_start();
            if let Some(after) = rest.strip_prefix("\\label") {
                let (text, end) = read_group(after, 0, b'{', b'}')?;
                label = Some(text.trim().to_string());
                rest = after[end..].trim_start();
            }
            rest = rest.strip_prefix("\\\\")?.trim_start();
        }
        if rest.contains("\\endfoot") || rest.contains("\\endlastfoot") {
            return None;
        }
        let rows = match (rest.find("\\endfirsthead"), rest.find("\\endhead")) {
            (Some(first), Some(head)) if first < head => {
                format!("{}\n{}", &rest[..first], &rest[head + "\\endhead".len()..])
            }
            (None, Some(head)) => {
                format!("{}\n{}", &rest[..head], &rest[head + "\\endhead".len()..])
            }
            (None, None) => rest.to_string(),
            _ => return None,
        };
        let mut table = self.table(&format!(
            "\\begin{{tabular}}{{{spec}}}\n{rows}\n\\end{{tabular}}"
        ))?;
        if let Some(attrs) = table.get_mut("attrs").and_then(Value::as_object_mut) {
            attrs.remove("columnSpec");
            attrs.remove("tableEnvironment");
            // Linien, die genau dem Stil entsprechen, nicht als eigene Angabe speichern
            let style = attrs
                .get("tableStyle")
                .and_then(Value::as_str)
                .unwrap_or("grid")
                .to_string();
            let standard = |rules: &[Value]| {
                let count = rules.len().saturating_sub(1);
                rules.iter().enumerate().all(|(index, rule)| {
                    let rule = rule.as_str().unwrap_or("");
                    match style.as_str() {
                        "booktabs" => {
                            rule == match index {
                                0 => "\\toprule",
                                1 if count > 1 => "\\midrule",
                                _ if index == count => "\\bottomrule",
                                _ => "",
                            }
                        }
                        "plain" => rule.is_empty(),
                        _ => rule == "\\hline",
                    }
                })
            };
            if attrs
                .get("rowRules")
                .and_then(Value::as_array)
                .is_some_and(|rules| standard(rules))
            {
                attrs.remove("rowRules");
            }
        }
        set_attr(&mut table, "breakAcrossPages", json!(true));
        if let Some(caption) = caption {
            set_attr(&mut table, "caption", json!(caption));
        }
        if let Some(label) = label {
            set_attr(&mut table, "label", json!(label));
        }
        Some(table)
    }

    /// Parst tabular/tabularx/tabular* in einen Tiptap-Tabellenknoten.
    fn table(&mut self, full: &str) -> Option<Value> {
        let env = read_environment(full, 0)?;
        let mut body = env.content;
        let spec: &str;
        let mut width = "";
        if env.name == "tabularx" || env.name == "tabular*" {
            let (value, width_end) = read_group(body, skip_spaces(body, 0, true), b'{', b'}')?;
            width = value;
            let (value, spec_end) =
                read_group(body, skip_spaces(body, width_end, true), b'{', b'}')?;
            spec = value;
            body = &body[spec_end..];
        } else {
            let mut index = skip_spaces(body, 0, true);
            if byte_at(body, index) == Some(b'[') {
                let (_, end) = read_group(body, index, b'[', b']')?;
                index = skip_spaces(body, end, true);
            }
            let (value, spec_end) = read_group(body, index, b'{', b'}')?;
            spec = value;
            body = &body[spec_end..];
        }
        let columns = parse_column_spec(spec)?;
        if columns.is_empty() {
            return None;
        }
        let (rows, rules) = split_table_rows_with_rules(body)?;
        let row_count = rows.len();
        let mut occupied: Vec<usize> = Vec::new();
        let mut table_rows: Vec<Value> = Vec::new();
        for row_source in rows {
            let cells = split_cells(&row_source)?;
            let covered: Vec<bool> = occupied.iter().map(|count| *count > 0).collect();
            let mut row_cells: Vec<Value> = Vec::new();
            let mut column = 0;
            for raw_cell in cells {
                let mut text = raw_cell.trim().to_string();
                let mut colspan = 1;
                let mut rowspan = 1;
                let mut align: Option<String> = columns.get(column).map(|c| c.align.to_string());
                let mut explicit_align = false;
                if let Some(after) = text.strip_prefix("\\multicolumn") {
                    let (args, end) = read_arguments(after, 0, "mmm")?;
                    if end != after.len() {
                        return None;
                    }
                    colspan = args[0]?.trim().parse::<usize>().ok()?.max(1);
                    let letter = args[1]?.chars().find(|c| matches!(c, 'l' | 'c' | 'r'));
                    align = Some(
                        match letter {
                            Some('c') => "center",
                            Some('r') => "right",
                            _ => "left",
                        }
                        .to_string(),
                    );
                    explicit_align = true;
                    text = args[2]?.trim().to_string();
                }
                if covered.get(column).copied().unwrap_or(false) {
                    if !text.is_empty() {
                        return None;
                    }
                    column += colspan;
                    continue;
                }
                if let Some(after) = text.strip_prefix("\\multirow") {
                    let (args, end) = read_arguments(after, 0, "mmm")?;
                    if end != after.len() {
                        return None;
                    }
                    rowspan = args[0]?.trim().parse::<usize>().ok()?.max(1);
                    text = args[2]?.trim().to_string();
                }
                for (prefix, value) in [
                    ("\\centering\\arraybackslash", "center"),
                    ("\\raggedleft\\arraybackslash", "right"),
                    ("\\raggedright\\arraybackslash", "left"),
                ] {
                    if let Some(after) = text.strip_prefix(prefix) {
                        align = Some(value.to_string());
                        explicit_align = true;
                        text = after.trim_start().to_string();
                        break;
                    }
                }
                let mut header = false;
                if let Some(after) = text.strip_prefix("\\bfseries") {
                    if !after.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        header = true;
                        text = after.trim_start().to_string();
                    }
                }
                let mut content: Vec<Value> = Vec::new();
                for part in split_par(&text) {
                    let part = part.trim();
                    if part.is_empty() {
                        content.push(json!({ "type": "paragraph" }));
                        continue;
                    }
                    let blocks = self.parse_blocks(part);
                    if blocks.len() == 1 {
                        content.extend(blocks);
                    } else {
                        let mut inline = self.parse_inline(part, &Marks::default());
                        trim_inline_edges(&mut inline);
                        content.push(json!({ "type": "paragraph", "content": inline }));
                    }
                }
                if content.is_empty() {
                    content.push(json!({ "type": "paragraph" }));
                }
                let mut attrs = json!({ "colspan": colspan, "rowspan": rowspan, "colwidth": null });
                if let Some(align) = &align {
                    if align != "left" || explicit_align {
                        attrs["align"] = json!(align);
                    }
                }
                let widths: Vec<Option<f64>> = columns
                    .iter()
                    .skip(column)
                    .take(colspan)
                    .map(|c| c.width)
                    .collect();
                if widths.len() == colspan
                    && widths.iter().all(Option::is_some)
                    && columns
                        .iter()
                        .any(|c| c.width.is_some_and(|w| (w - 1.0).abs() > 1e-9))
                {
                    attrs["colwidth"] = json!(widths
                        .iter()
                        .map(|w| (w.unwrap_or(1.0) * 160.0).round() as i64)
                        .collect::<Vec<_>>());
                }
                row_cells.push(json!({ "type": if header { "tableHeader" } else { "tableCell" }, "attrs": attrs, "content": content }));
                if rowspan > 1 {
                    for c in column..column + colspan {
                        if occupied.len() <= c {
                            occupied.resize(c + 1, 0);
                        }
                        occupied[c] = rowspan;
                    }
                }
                column += colspan;
            }
            for count in occupied.iter_mut() {
                *count = count.saturating_sub(1);
            }
            if !row_cells.is_empty() {
                table_rows.push(json!({ "type": "tableRow", "content": row_cells }));
            }
        }
        if table_rows.is_empty() {
            return None;
        }
        // Vorversion: Kopfzeile als \textbf{…} in jeder Zelle der ersten Zeile.
        let first = table_rows[0]["content"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let all_bold = first.iter().all(|cell| {
            cell["type"] == "tableHeader"
                || cell["content"].as_array().is_some_and(|paragraphs| {
                    paragraphs.len() == 1
                        && paragraphs[0]["content"].as_array().is_some_and(|inline| {
                            !inline.is_empty()
                                && inline.iter().all(|node| {
                                    node["marks"].as_array().is_some_and(|marks| {
                                        marks.iter().any(|mark| mark["type"] == "bold")
                                    })
                                })
                        })
                })
        });
        if all_bold && first.iter().any(|cell| cell["type"] != "tableHeader") {
            let converted: Vec<Value> = first
                .into_iter()
                .map(|mut cell| {
                    cell["type"] = json!("tableHeader");
                    if let Some(paragraphs) = cell["content"].as_array_mut() {
                        for paragraph in paragraphs {
                            if let Some(inline) = paragraph["content"].as_array_mut() {
                                for node in inline {
                                    if let Some(marks) = node["marks"].as_array_mut() {
                                        marks.retain(|mark| mark["type"] != "bold");
                                        if marks.is_empty() {
                                            node.as_object_mut()
                                                .map(|object| object.remove("marks"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    cell
                })
                .collect();
            table_rows[0]["content"] = Value::Array(converted);
        }
        let mut table = json!({ "type": "table", "attrs": { "caption": "", "label": "" }, "content": table_rows });
        // Von VisuTeX erzeugte Gittertabellen (|>{…\arraybackslash}X|…) brauchen
        // keine Stilangaben; fremde Tabellen behalten Spalten, Umgebung und Linien.
        let generated = env.name == "tabularx"
            && width.trim() == "\\linewidth"
            && spec.contains("\\arraybackslash}X");
        let style = if rules.iter().any(|rule| rule.contains("rule")) {
            "booktabs"
        } else if rules.iter().all(String::is_empty) && !spec.contains('|') {
            "plain"
        } else {
            "grid"
        };
        if generated {
            if style != "grid" {
                set_attr(&mut table, "tableStyle", json!(style));
            }
        } else {
            set_attr(&mut table, "columnSpec", json!(spec.trim()));
            set_attr(&mut table, "tableEnvironment", json!(env.name));
            if !width.trim().is_empty() {
                set_attr(&mut table, "tableWidth", json!(width.trim()));
            }
            set_attr(&mut table, "tableStyle", json!(style));
            if table["content"].as_array().map_or(0, Vec::len) == row_count {
                set_attr(&mut table, "rowRules", json!(rules));
            }
        }
        Some(table)
    }

    // ---------------------------------------------------------------- Inline

    /// Klartext aus einfachem LaTeX (nur Maskierungen/Textsymbole), sonst None.
    pub fn plain_text(&mut self, source: &str) -> Option<String> {
        let nodes = self.parse_inline(source, &Marks::default());
        let mut text = String::new();
        for node in nodes {
            if node["type"] != "text" || node.get("marks").is_some() {
                return None;
            }
            text.push_str(node["text"].as_str().unwrap_or(""));
        }
        Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    fn parse_inline(&mut self, source: &str, marks: &Marks) -> Vec<Value> {
        let mut result: Vec<Value> = Vec::new();
        let mut text = String::new();
        let bytes = source.as_bytes();
        let mut index = 0;

        macro_rules! flush {
            () => {
                if !text.is_empty() {
                    result.push(text_node(&text, marks));
                    text.clear();
                }
            };
        }
        macro_rules! push_node {
            ($node:expr) => {{
                flush!();
                result.push($node);
            }};
        }
        macro_rules! push_raw {
            ($latex:expr) => {{
                let latex: String = $latex.to_string();
                if !latex.is_empty() {
                    push_node!(json!({ "type": "rawLatexInline", "attrs": { "latex": latex } }));
                }
            }};
        }

        while index < bytes.len() {
            let character = char_at(source, index).unwrap_or(' ');
            if character == PLACEHOLDER_START {
                index = source[index..]
                    .find(PLACEHOLDER_END)
                    .map(|i| index + i + PLACEHOLDER_END.len_utf8())
                    .unwrap_or(bytes.len());
                continue;
            }
            if character.is_whitespace() {
                while let Some(next) = char_at(source, index) {
                    if !next.is_whitespace() {
                        break;
                    }
                    index += next.len_utf8();
                }
                if !text.ends_with(' ') {
                    text.push(' ');
                }
                continue;
            }
            match character {
                '%' => {
                    let end = source[index..]
                        .find('\n')
                        .map(|i| index + i + 1)
                        .unwrap_or(bytes.len());
                    push_raw!(&source[index..end]);
                    index = end;
                    continue;
                }
                '~' => {
                    text.push('\u{00a0}');
                    index += 1;
                    continue;
                }
                '$' => {
                    if bytes.get(index + 1) == Some(&b'$') {
                        match source[index + 2..].find("$$") {
                            Some(offset) => {
                                let end = index + 2 + offset + 2;
                                push_raw!(&source[index..end]);
                                index = end;
                            }
                            None => {
                                push_raw!(&source[index..]);
                                index = bytes.len();
                            }
                        }
                        continue;
                    }
                    match find_unescaped(source, "$", index + 1) {
                        Some(end) => {
                            let latex = &source[index + 1..end];
                            match unicode_for_math_command(latex) {
                                Some(symbol) => text.push(symbol),
                                None => push_node!(
                                    json!({ "type": "inlineMath", "attrs": { "latex": latex.trim() } })
                                ),
                            }
                            index = end + 1;
                        }
                        None => {
                            push_raw!(&source[index..]);
                            index = bytes.len();
                        }
                    }
                    continue;
                }
                '"' if self.german_shorthands => {
                    if let Some(next) = char_at(source, index + 1) {
                        if let Some(replacement) = german_shorthand(next) {
                            text.push_str(replacement);
                            index += 1 + next.len_utf8();
                            continue;
                        }
                        if matches!(next, '"' | '=' | '~' | '|') {
                            push_raw!(&source[index..index + 2]);
                            index += 2;
                            continue;
                        }
                    }
                }
                '&' | '#' | '^' | '_' => {
                    push_raw!(character);
                    index += 1;
                    continue;
                }
                '{' => {
                    match read_group(source, index, b'{', b'}') {
                        Some((group, end)) => {
                            if let Some(declared) = self.declaration_group(group, marks) {
                                flush!();
                                result.extend(declared);
                            } else if group.trim_start().starts_with('\\')
                                && !starts_with_known_inline(group.trim_start())
                            {
                                // z. B. {\"O} oder {\ss}: nur übernehmen, wenn reiner Text entsteht.
                                let nested = self.parse_inline(group, marks);
                                if !nested.is_empty()
                                    && nested.iter().all(|node| node["type"] == "text")
                                {
                                    flush!();
                                    result.extend(nested);
                                } else {
                                    push_raw!(&source[index..end]);
                                }
                            } else if !group.is_empty() {
                                flush!();
                                let nested = self.parse_inline(group, marks);
                                result.extend(nested);
                            }
                            index = end;
                        }
                        None => {
                            push_raw!(&source[index..]);
                            index = bytes.len();
                        }
                    }
                    continue;
                }
                '\\' => {}
                _ => {
                    text.push(character);
                    index += character.len_utf8();
                    continue;
                }
            }

            // ------------------------------------------------------ Befehle
            let Some((name, after)) = read_command_name(source, index) else {
                text.push('\\');
                index += 1;
                continue;
            };
            let name = name.to_string();
            let mut after = after;

            if name.len() == 1 && "%$#&_{}".contains(name.as_str()) {
                text.push_str(&name);
                index = after;
                continue;
            }
            if name == "\\" {
                let mut end = after;
                if byte_at(source, end) == Some(b'*') {
                    end += 1;
                }
                let spaced = skip_spaces(source, end, true);
                if byte_at(source, spaced) == Some(b'[') {
                    let stop = read_group(source, spaced, b'[', b']')
                        .map(|(_, e)| e)
                        .unwrap_or(end);
                    push_raw!(&source[index..stop]);
                    index = stop;
                    continue;
                }
                if source[end..].starts_with("{}") {
                    end += 2;
                }
                push_node!(json!({ "type": "hardBreak" }));
                index = skip_spaces(source, end, true);
                continue;
            }
            match name.as_str() {
                " " | "\n" => {
                    text.push(' ');
                    index = after;
                    continue;
                }
                "," => {
                    text.push('\u{202f}');
                    index = after;
                    continue;
                }
                "-" => {
                    text.push('\u{00ad}');
                    index = after;
                    continue;
                }
                "(" => {
                    match source[after..].find("\\)") {
                        Some(offset) => {
                            let latex = source[after..after + offset].trim();
                            push_node!(
                                json!({ "type": "inlineMath", "attrs": { "latex": latex } })
                            );
                            index = after + offset + 2;
                        }
                        None => {
                            push_raw!(&source[index..]);
                            index = bytes.len();
                        }
                    }
                    continue;
                }
                "protect" => {
                    index = after;
                    continue;
                }
                _ => {}
            }
            if let Some(mark) = accent_mark(&name) {
                let mut position = if name.chars().all(|c| c.is_ascii_alphabetic()) {
                    skip_spaces(source, after, false)
                } else {
                    after
                };
                let mut letter: Option<char> = None;
                if byte_at(source, position) == Some(b'{') {
                    if let Some((group, end)) = read_group(source, position, b'{', b'}') {
                        let group = if group == "\\i" { "ı" } else { group };
                        if group.chars().count() == 1 {
                            letter = group.chars().next();
                            position = end;
                        }
                    }
                } else if let Some(next) =
                    char_at(source, position).filter(|c| c.is_ascii_alphabetic())
                {
                    letter = Some(next);
                    position += 1;
                }
                if let Some(letter) = letter {
                    text.push_str(&compose(letter, mark));
                    index = position;
                    continue;
                }
            }
            if let Some(letter) = letter_command(&name) {
                text.push_str(letter);
                index = if source[after..].starts_with("{}") {
                    after + 2
                } else if byte_at(source, after) == Some(b' ') {
                    after + 1
                } else {
                    after
                };
                continue;
            }
            if let Some(symbol) = text_symbol(&name) {
                text.push_str(symbol);
                if source[after..].starts_with("{}") {
                    after += 2;
                } else if byte_at(source, after) == Some(b' ') {
                    after += 1;
                }
                index = after;
                continue;
            }
            if name == "ensuremath" {
                if let Some((group, end)) = read_group(source, after, b'{', b'}') {
                    match unicode_for_math_command(group) {
                        Some(symbol) => text.push(symbol),
                        None => push_node!(
                            json!({ "type": "inlineMath", "attrs": { "latex": group.trim() } })
                        ),
                    }
                    index = end;
                    continue;
                }
            }
            if name == "newline" || (name == "linebreak" && byte_at(source, after) != Some(b'[')) {
                push_node!(json!({ "type": "hardBreak" }));
                index = skip_spaces(source, after, true);
                continue;
            }
            let simple = match name.as_str() {
                "textbf" => Some("bold"),
                "textit" | "emph" | "textsl" => Some("italic"),
                "uline" | "underline" => Some("underline"),
                "sout" => Some("strike"),
                "textsuperscript" => Some("superscript"),
                "textsubscript" => Some("subscript"),
                "texttt" => Some("code"),
                _ => None,
            };
            if let Some(kind) = simple {
                if let Some((args, end)) = read_arguments(source, after, "m") {
                    let mut next = marks.clone();
                    match kind {
                        "bold" => next.bold = true,
                        "italic" => next.italic = true,
                        "underline" => next.underline = true,
                        "strike" => next.strike = true,
                        "superscript" => next.superscript = true,
                        "subscript" => next.subscript = true,
                        _ => next.code = true,
                    }
                    flush!();
                    let nested = self.parse_inline(args[0].unwrap_or(""), &next);
                    result.extend(nested);
                    index = end;
                    continue;
                }
            }
            if name == "textcolor" || name == "colorbox" {
                if let Some((args, end)) = read_arguments(source, after, "omm") {
                    if let Some(color) = color_value(args[0], args[1].unwrap_or("")) {
                        let mut next = marks.clone();
                        if name == "textcolor" {
                            next.color = Some(color);
                        } else {
                            next.background = Some(color);
                        }
                        flush!();
                        let nested = self.parse_inline(args[2].unwrap_or(""), &next);
                        result.extend(nested);
                        index = end;
                        continue;
                    }
                }
            }
            if name == "visutexfont" {
                if let Some((args, end)) = read_arguments(source, after, "mm") {
                    let family = args[0].unwrap_or("").trim();
                    if !family.is_empty()
                        && family
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
                    {
                        let mut next = marks.clone();
                        next.font_family = Some(family.to_string());
                        flush!();
                        let nested = self.parse_inline(args[1].unwrap_or(""), &next);
                        result.extend(nested);
                        index = end;
                        continue;
                    }
                }
            }
            if name == "href" {
                if let Some((args, end)) = read_arguments(source, after, "mm") {
                    let url = super::escape::unescape_url(&strip_detokenize(args[0].unwrap_or("")));
                    let lower = url.to_ascii_lowercase();
                    if lower.starts_with("http:")
                        || lower.starts_with("https:")
                        || lower.starts_with("mailto:")
                    {
                        let mut next = marks.clone();
                        next.link = Some(url);
                        flush!();
                        let nested = self.parse_inline(args[1].unwrap_or(""), &next);
                        result.extend(nested);
                        index = end;
                        continue;
                    }
                }
            }
            if name == "url" {
                if let Some((args, end)) = read_arguments(source, after, "m") {
                    let url = super::escape::unescape_url(args[0].unwrap_or("").trim());
                    let lower = url.to_ascii_lowercase();
                    if lower.starts_with("http:")
                        || lower.starts_with("https:")
                        || lower.starts_with("mailto:")
                    {
                        let mut next = marks.clone();
                        next.link = Some(url.clone());
                        push_node!(text_node(&url, &next));
                        index = end;
                        continue;
                    }
                }
            }
            // siunitx: \SI{1}{\mega\ohm}, \qty{…}{…}, \si{…}, \num{…}, Bereiche
            if QUANTITY_COMMANDS.contains(&name.as_str()) {
                let spec = match name.as_str() {
                    "si" | "unit" | "num" | "ang" => "om",
                    "numrange" => "omm",
                    "SIrange" | "qtyrange" => "ommm",
                    _ => "omm",
                };
                if let Some((args, end)) = read_arguments(source, after, spec) {
                    let arg = |i: usize| args.get(i).copied().flatten().unwrap_or("").trim();
                    let (value, value2, unit) = match name.as_str() {
                        "si" | "unit" => ("", "", arg(1)),
                        "num" | "ang" => (arg(1), "", ""),
                        "numrange" => (arg(1), arg(2), ""),
                        "SIrange" | "qtyrange" => (arg(1), arg(2), arg(3)),
                        _ => (arg(1), "", arg(2)),
                    };
                    push_node!(json!({ "type": "quantity", "attrs": {
                        "command": name, "value": value, "value2": value2, "unit": unit,
                        "options": arg(0),
                    } }));
                    index = end;
                    continue;
                }
            }
            if CITE_COMMANDS.contains(&name.as_str()) && byte_at(source, after) != Some(b'*') {
                if let Some((args, end)) = read_arguments(source, after, "oom") {
                    let keys: Vec<String> = args[2]
                        .unwrap_or("")
                        .split(',')
                        .map(|key| key.trim().to_string())
                        .filter(|key| !key.is_empty())
                        .collect();
                    let (pre, post) = match (args[0], args[1]) {
                        (Some(first), Some(second)) => {
                            (self.plain_text(first), self.plain_text(second))
                        }
                        (Some(first), None) => (Some(String::new()), self.plain_text(first)),
                        _ => (Some(String::new()), Some(String::new())),
                    };
                    if let (false, Some(pre), Some(post)) = (keys.is_empty(), pre, post) {
                        push_node!(json!({ "type": "citation", "attrs": {
                            "keys": keys.join(","), "label": keys.join(", "), "prenote": pre, "postnote": post, "command": name,
                        } }));
                        index = end;
                        continue;
                    }
                }
            }
            if name == "footnote" && byte_at(source, after) != Some(b'[') {
                if let Some((args, end)) = read_arguments(source, after, "m") {
                    if let Some(plain) = self.plain_text(args[0].unwrap_or("")) {
                        push_node!(json!({ "type": "footnote", "attrs": { "text": plain } }));
                        index = end;
                        continue;
                    }
                }
            }
            if matches!(name.as_str(), "ref" | "pageref" | "eqref" | "autoref")
                && byte_at(source, after) != Some(b'*')
            {
                if let Some((args, end)) = read_arguments(source, after, "m") {
                    push_node!(
                        json!({ "type": "crossReference", "attrs": { "label": args[0].unwrap_or("").trim(), "kind": name } })
                    );
                    index = end;
                    continue;
                }
            }
            if matches!(
                name.as_str(),
                "ac" | "acs" | "acl" | "acf" | "acp" | "acsp" | "aclp" | "acfp"
            ) && byte_at(source, after) != Some(b'*')
            {
                if let Some((args, end)) = read_arguments(source, after, "m") {
                    let key = args[0].unwrap_or("").trim().to_string();
                    let definition = self
                        .acronyms
                        .iter()
                        .find(|(existing, _, _)| *existing == key)
                        .cloned();
                    if definition.is_none() {
                        self.warn(format!(
                            "Abkürzung „{key}“ wird verwendet, ist aber nicht definiert."
                        ));
                    }
                    let (short, long) = definition
                        .map(|(_, short, long)| (short, long))
                        .unwrap_or_else(|| (key.clone(), String::new()));
                    push_node!(
                        json!({ "type": "acronym", "attrs": { "key": key, "short": short, "long": long, "command": name } })
                    );
                    index = end;
                    continue;
                }
            }
            if name == "verb" {
                if let Some(delimiter) = char_at(source, after) {
                    let search = after + delimiter.len_utf8();
                    let stop = source[search..]
                        .find(delimiter)
                        .map(|i| search + i + delimiter.len_utf8())
                        .unwrap_or(bytes.len());
                    push_raw!(&source[index..stop]);
                    index = stop;
                    continue;
                }
            }

            // Unbekannter Befehl: Name + direkt folgende Argumente als Roh-LaTeX
            let mut end = after;
            if byte_at(source, end) == Some(b'*') {
                end += 1;
            }
            loop {
                match byte_at(source, end) {
                    Some(b'[') => match read_group(source, end, b'[', b']') {
                        Some((_, group_end)) => end = group_end,
                        None => break,
                    },
                    Some(b'{') => match read_group(source, end, b'{', b'}') {
                        Some((_, group_end)) => end = group_end,
                        None => break,
                    },
                    _ => break,
                }
            }
            // TeX überspringt Leerraum (auch einen Zeilenumbruch) nach einem Befehlswort –
            // er gehört zum Roh-LaTeX, damit der Export die Stelle unverändert wiedergibt.
            if end == after && name.chars().all(|c| c.is_ascii_alphabetic()) {
                while matches!(byte_at(source, end), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                    end += 1;
                }
            }
            push_raw!(&source[index..end]);
            index = end;
        }
        flush!();
        merge_text_nodes(result)
    }

    /// {\bfseries …}, {\color[HTML]{X} …}, {\fontsize{a}{b}\selectfont …}, {\fontspec{F} …}
    fn declaration_group(&mut self, value: &str, marks: &Marks) -> Option<Vec<Value>> {
        let mut rest = value;
        let mut next = marks.clone();
        let mut matched = false;
        loop {
            rest = rest.trim_start();
            let mut consumed = 0;
            for (command, kind) in [
                ("\\bfseries", "bold"),
                ("\\itshape", "italic"),
                ("\\em", "italic"),
                ("\\slshape", "italic"),
                ("\\ttfamily", "code"),
            ] {
                if rest.starts_with(command)
                    && !rest[command.len()..].starts_with(|c: char| c.is_ascii_alphabetic())
                {
                    match kind {
                        "bold" => next.bold = true,
                        "italic" => next.italic = true,
                        _ => next.code = true,
                    }
                    consumed = command.len();
                    break;
                }
            }
            if consumed == 0 {
                if let Some(after) = rest.strip_prefix("\\fontsize") {
                    if let Some((args, end)) = read_arguments(after, 0, "mm") {
                        let size = args[0].unwrap_or("").trim().trim_end_matches("pt").trim();
                        let tail = after[end..].trim_start();
                        if let (Ok(points), Some(remaining)) =
                            (size.parse::<f64>(), tail.strip_prefix("\\selectfont"))
                        {
                            if !remaining.starts_with(|c: char| c.is_ascii_alphabetic()) {
                                let formatted = format!("{}", (points * 10.0).round() / 10.0);
                                next.font_size = Some(format!("{formatted}pt"));
                                consumed = rest.len() - remaining.len();
                            }
                        }
                    }
                }
            }
            if consumed == 0 {
                if let Some(after) = rest.strip_prefix("\\color") {
                    if !after.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        let (args, end) = read_arguments(after, 0, "om")?;
                        let color = color_value(args[0], args[1].unwrap_or(""))?;
                        next.color = Some(color);
                        consumed = "\\color".len() + end;
                    }
                }
            }
            if consumed == 0 {
                if let Some(after) = rest.strip_prefix("\\fontspec") {
                    let (args, end) = read_arguments(after, 0, "m")?;
                    let family = args[0].unwrap_or("").trim();
                    if family.is_empty()
                        || !family
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
                    {
                        return None;
                    }
                    next.font_family = Some(family.to_string());
                    consumed = "\\fontspec".len() + end;
                }
            }
            if consumed == 0 {
                break;
            }
            matched = true;
            rest = &rest[consumed..];
        }
        if !matched {
            return None;
        }
        let trimmed = rest.trim_start();
        if ["\\centering", "\\raggedright", "\\raggedleft", "\\par"]
            .iter()
            .any(|command| trimmed.starts_with(command))
        {
            return None;
        }
        Some(self.parse_inline(rest, &next))
    }
}

fn starts_with_known_inline(value: &str) -> bool {
    [
        "\\text",
        "\\emph",
        "\\uline",
        "\\underline",
        "\\sout",
        "\\href",
        "\\url",
        "\\cite",
        "\\footnote",
        "\\ref",
        "\\pageref",
        "\\eqref",
        "\\autoref",
        "\\ac",
        "\\visutexfont",
        "\\colorbox",
        "\\ensuremath",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
}

fn find_labels(source: &str) -> Vec<(usize, &str)> {
    let mut labels = Vec::new();
    let mut offset = 0;
    while let Some(found) = source[offset..].find("\\label") {
        let start = offset + found;
        offset = start + "\\label".len();
        let position = skip_spaces(source, offset, false);
        if let Some((value, end)) = read_group(source, position, b'{', b'}') {
            if position == offset {
                labels.push((start, value));
            }
            offset = end;
        }
    }
    labels
}

fn single_includegraphics(source: &str) -> Option<(String, String)> {
    let after = source.strip_prefix("\\includegraphics")?;
    let (args, end) = read_arguments(after, 0, "om")?;
    if end != after.len() {
        return None;
    }
    Some((args[0].unwrap_or("").to_string(), args[1]?.to_string()))
}

fn split_par(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut index = 0;
    let bytes = source.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'\\'
            && depth == 0
            && source[index..].starts_with("\\par")
            && !source[index + 4..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            parts.push(std::mem::take(&mut current));
            index += 4;
            continue;
        }
        let character = char_at(source, index).unwrap_or(' ');
        if character == '\\' {
            current.push('\\');
            index += 1;
            if let Some(next) = char_at(source, index) {
                current.push(next);
                index += next.len_utf8();
            }
            continue;
        }
        if character == '{' {
            depth += 1;
        }
        if character == '}' {
            depth -= 1;
        }
        current.push(character);
        index += character.len_utf8();
    }
    parts.push(current);
    parts
}

#[derive(Clone, Copy)]
struct ColumnInfo {
    align: &'static str,
    width: Option<f64>,
}

fn parse_column_spec(spec: &str) -> Option<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    let mut pending_align: Option<&'static str> = None;
    let mut pending_width: Option<f64> = None;
    let bytes = spec.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'|' => index += 1,
            b'>' | b'<' | b'@' | b'!' => {
                let (group, end) =
                    read_group(spec, skip_spaces(spec, index + 1, true), b'{', b'}')?;
                if bytes[index] == b'>' {
                    if group.contains("\\centering") {
                        pending_align = Some("center");
                    } else if group.contains("\\raggedleft") {
                        pending_align = Some("right");
                    } else if group.contains("\\raggedright") {
                        pending_align = Some("left");
                    }
                    if let Some(position) = group.find("\\hsize=") {
                        let number: String = group[position + 7..]
                            .chars()
                            .take_while(|c| c.is_ascii_digit() || *c == '.')
                            .collect();
                        pending_width = number.parse().ok();
                    }
                }
                index = end;
            }
            b'*' => {
                let (count, count_end) = read_group(spec, index + 1, b'{', b'}')?;
                let (repeated, end) = read_group(spec, count_end, b'{', b'}')?;
                let inner = parse_column_spec(repeated)?;
                for _ in 0..count.trim().parse::<usize>().ok()?.min(64) {
                    columns.extend(inner.iter().copied());
                }
                index = end;
            }
            b'l' | b'c' | b'r' => {
                columns.push(ColumnInfo {
                    align: match bytes[index] {
                        b'c' => "center",
                        b'r' => "right",
                        _ => "left",
                    },
                    width: None,
                });
                pending_align = None;
                index += 1;
            }
            b'p' | b'm' | b'b' | b'X' => {
                let mut end = index + 1;
                if bytes[index] != b'X' {
                    let (_, group_end) = read_group(spec, index + 1, b'{', b'}')?;
                    end = group_end;
                }
                columns.push(ColumnInfo {
                    align: pending_align.unwrap_or("left"),
                    width: if bytes[index] == b'X' {
                        Some(pending_width.unwrap_or(1.0))
                    } else {
                        None
                    },
                });
                pending_align = None;
                pending_width = None;
                index = end;
            }
            _ => return None,
        }
    }
    Some(columns)
}

/// Teilt den Tabellenkörper in Zeilen und sammelt die Linienbefehle zwischen den
/// Zeilen (`rules[i]` steht vor Zeile i, `rules[n]` nach der letzten Zeile).
fn split_table_rows_with_rules(body: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut rows = Vec::new();
    let mut rules = Vec::new();
    let mut pending_rule = String::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let bytes = body.as_bytes();
    let mut index = 0;
    // Zeile abschließen; leere Zeilen (nur Linien) gehen in die nächste Grenze ein.
    let finish_row = |current: &mut String,
                      rows: &mut Vec<String>,
                      rules: &mut Vec<String>,
                      pending_rule: &mut String| {
        if current.trim().is_empty() {
            current.clear();
            return;
        }
        rules.push(std::mem::take(pending_rule));
        rows.push(std::mem::take(current));
    };
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                index = body[index..]
                    .find('\n')
                    .map(|i| index + i)
                    .unwrap_or(bytes.len());
                continue;
            }
            b'\\' => {
                if depth == 0 && bytes.get(index + 1) == Some(&b'\\') {
                    finish_row(&mut current, &mut rows, &mut rules, &mut pending_rule);
                    index += 2;
                    if bytes.get(index) == Some(&b'*') {
                        index += 1;
                    }
                    let spaced = skip_spaces(body, index, true);
                    if bytes.get(spaced) == Some(&b'[') {
                        if let Some((_, end)) = read_group(body, spaced, b'[', b']') {
                            index = end;
                        }
                    }
                    continue;
                }
                let (name, name_end) = read_command_name(body, index)?;
                if depth == 0
                    && matches!(
                        name,
                        "hline" | "toprule" | "midrule" | "bottomrule" | "cline" | "cmidrule"
                    )
                {
                    let mut end = name_end;
                    if bytes.get(end) == Some(&b'(') {
                        end = body[end..].find(')').map(|i| end + i + 1).unwrap_or(end);
                    }
                    let spaced = skip_spaces(body, end, false);
                    if bytes.get(spaced) == Some(&b'{') && matches!(name, "cline" | "cmidrule") {
                        if let Some((_, group_end)) = read_group(body, spaced, b'{', b'}') {
                            end = group_end;
                        }
                    }
                    if !pending_rule.is_empty() {
                        pending_rule.push(' ');
                    }
                    pending_rule.push_str(body[index..end].trim());
                    index = end;
                    continue;
                }
                if depth == 0 && name == "tabularnewline" {
                    finish_row(&mut current, &mut rows, &mut rules, &mut pending_rule);
                    index = name_end;
                    continue;
                }
                current.push_str(&body[index..name_end]);
                index = name_end;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return None;
        }
        let character = char_at(body, index)?;
        current.push(character);
        index += character.len_utf8();
    }
    finish_row(&mut current, &mut rows, &mut rules, &mut pending_rule);
    rules.push(pending_rule);
    Some((rows, rules))
}

fn split_cells(row: &str) -> Option<Vec<String>> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut index = 0;
    while index < row.len() {
        let character = char_at(row, index)?;
        if character == '\\' {
            current.push('\\');
            index += 1;
            if let Some(next) = char_at(row, index) {
                current.push(next);
                index += next.len_utf8();
            }
            continue;
        }
        if character == '{' {
            depth += 1;
        }
        if character == '}' {
            depth -= 1;
        }
        if character == '&' && depth == 0 {
            cells.push(std::mem::take(&mut current));
            index += 1;
            continue;
        }
        current.push(character);
        index += character.len_utf8();
    }
    cells.push(current);
    (depth == 0).then_some(cells)
}

/// Teilt den Inhalt einer Listenumgebung an `\item` (Tiefe 0).
fn split_items(content: &str) -> Option<Vec<(Option<String>, String)>> {
    let mut items: Vec<(Option<String>, String)> = Vec::new();
    let mut preamble = String::new();
    let mut depth = 0i32;
    let mut env_depth = 0i32;
    let mut index = 0;
    let bytes = content.as_bytes();
    while index < bytes.len() {
        let character = char_at(content, index)?;
        let target = match items.last_mut() {
            Some((_, text)) => text,
            None => &mut preamble,
        };
        if character == '%' {
            let end = content[index..]
                .find('\n')
                .map(|i| index + i + 1)
                .unwrap_or(bytes.len());
            target.push_str(&content[index..end]);
            index = end;
            continue;
        }
        if character == '\\' {
            let rest = &content[index..];
            if rest.starts_with("\\begin") {
                env_depth += 1;
            } else if rest.starts_with("\\end") {
                env_depth -= 1;
            }
            if depth == 0
                && env_depth == 0
                && rest.starts_with("\\item")
                && !rest[5..].starts_with(|c: char| c.is_ascii_alphabetic())
            {
                let mut position = index + 5;
                let mut label = None;
                let spaced = skip_spaces(content, position, false);
                if byte_at(content, spaced) == Some(b'[') {
                    let (value, end) = read_group(content, spaced, b'[', b']')?;
                    label = Some(value.to_string());
                    position = end;
                }
                items.push((label, String::new()));
                index = position;
                continue;
            }
            target.push('\\');
            index += 1;
            if let Some(next) = char_at(content, index) {
                target.push(next);
                index += next.len_utf8();
            }
            continue;
        }
        if character == '{' {
            depth += 1;
        }
        if character == '}' {
            depth -= 1;
        }
        target.push(character);
        index += character.len_utf8();
    }
    if !preamble.trim().is_empty() {
        return None;
    }
    Some(items)
}

/// Leitet sichtbare Einstellungen aus einer fremden Präambel ab (für die Vorschau).
pub fn infer_settings_from_preamble(preamble: &str) -> Value {
    let text = strip_comments(preamble);
    let mut patch = Map::new();
    if let Some(index) = text.find("\\documentclass") {
        let rest = &text[index + "\\documentclass".len()..];
        let (options, _) = match rest.trim_start().strip_prefix('[') {
            Some(after) => after
                .split_once(']')
                .map(|(options, after)| (options, after))
                .unwrap_or(("", after)),
            None => ("", rest),
        };
        patch.insert(
            "documentClass".into(),
            json!(if class_has_chapters(&text) {
                "scrreprt"
            } else {
                "article"
            }),
        );
        for option in options.split(',').map(str::trim) {
            let size = option.strip_prefix("fontsize=").unwrap_or(option);
            if let Some(points) = size
                .strip_suffix("pt")
                .and_then(|value| value.parse::<f64>().ok())
            {
                patch.insert("defaultFontSize".into(), json!(points.round() as i64));
            }
            match option {
                "twoside" => {
                    patch.insert("twoside".into(), json!(true));
                }
                "a4paper" | "paper=a4" => {
                    patch.insert("paperFormat".into(), json!("a4"));
                }
                "a5paper" | "paper=a5" => {
                    patch.insert("paperFormat".into(), json!("a5"));
                }
                "letterpaper" | "paper=letter" => {
                    patch.insert("paperFormat".into(), json!("letter"));
                }
                "landscape" => {
                    patch.insert("orientation".into(), json!("landscape"));
                }
                _ => {}
            }
        }
    }
    let geometry = text.find("{geometry}").and_then(|index| {
        let before = &text[..index];
        let open = before.rfind("\\usepackage")?;
        let segment = &before[open..];
        let start = segment.find('[')?;
        let end = segment.rfind(']')?;
        Some(segment[start + 1..end].to_string())
    });
    if let Some(geometry) = geometry {
        let to_mm = |value: &str| -> Option<f64> {
            let value = value.trim();
            for (unit, factor) in [("mm", 1.0), ("cm", 10.0), ("in", 25.4), ("pt", 0.3515)] {
                if let Some(number) = value.strip_suffix(unit) {
                    return number
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .map(|n| (n * factor * 10.0).round() / 10.0);
                }
            }
            None
        };
        let mut margins = Margins {
            top: 25.0,
            right: 25.0,
            bottom: 25.0,
            left: 25.0,
        };
        let mut has_margins = false;
        for option in geometry.split(',') {
            let (key, value) = option
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .unwrap_or((option.trim(), ""));
            match key {
                "a4paper" => {
                    patch.insert("paperFormat".into(), json!("a4"));
                }
                "a5paper" => {
                    patch.insert("paperFormat".into(), json!("a5"));
                }
                "letterpaper" => {
                    patch.insert("paperFormat".into(), json!("letter"));
                }
                "legalpaper" => {
                    patch.insert("paperFormat".into(), json!("legal"));
                }
                "landscape" => {
                    patch.insert("orientation".into(), json!("landscape"));
                }
                "margin" => {
                    if let Some(mm) = to_mm(value) {
                        margins = Margins {
                            top: mm,
                            right: mm,
                            bottom: mm,
                            left: mm,
                        };
                        has_margins = true;
                    }
                }
                "top" | "bottom" | "left" | "right" | "inner" | "outer" => {
                    if let Some(mm) = to_mm(value) {
                        match key {
                            "top" => margins.top = mm,
                            "bottom" => margins.bottom = mm,
                            "left" | "inner" => margins.left = mm,
                            _ => margins.right = mm,
                        }
                        has_margins = true;
                    }
                }
                "bindingoffset" => {
                    if let Some(mm) = to_mm(value) {
                        patch.insert("bindingOffset".into(), json!(mm));
                    }
                }
                _ => {}
            }
        }
        if has_margins {
            patch.insert(
                "margins".into(),
                serde_json::to_value(margins).unwrap_or(Value::Null),
            );
        }
    }
    if let Some(index) = text.find("{babel}") {
        let before = &text[..index];
        if let (Some(start), Some(end)) = (before.rfind('['), before.rfind(']')) {
            if start < end {
                let main = before[start + 1..end]
                    .split(',')
                    .map(str::trim)
                    .last()
                    .unwrap_or("");
                let language = super::languages::from_babel(main);
                if let Some(language) = language {
                    patch.insert("language".into(), json!(language));
                }
            }
        }
    }
    // Grundschrift für die Anzeige im Editor (serifenlos, Times, Palatino)
    let loads = |name: &str| {
        super::preamble::loaded_packages(&text)
            .iter()
            .any(|package| package == name)
    };
    let sans_default = text.contains("\\familydefault}{\\sfdefault}")
        || text.contains("\\familydefault{\\sfdefault}")
        || text.contains("\\familydefault\\sfdefault");
    let font = if sans_default || ["arial", "uarial"].iter().any(|name| loads(name)) {
        Some("Arial")
    } else if ["mathptmx", "times", "newtxtext", "tgtermes", "txfonts"]
        .iter()
        .any(|name| loads(name))
    {
        Some("Times New Roman")
    } else if ["mathpazo", "palatino", "tgpagella", "newpxtext"]
        .iter()
        .any(|name| loads(name))
    {
        Some("Palatino Linotype")
    } else {
        None
    };
    if let Some(font) = font {
        patch.insert("defaultFontFamily".into(), json!(font));
    }
    Value::Object(patch)
}

pub fn import_latex(source: &str, settings: &DocumentSettings) -> ImportResult {
    let split = split_document(source);
    let (preamble, body, postamble) = match &split {
        Some((preamble, body, postamble)) => {
            (Some(preamble.clone()), body.clone(), postamble.clone())
        }
        None => (None, source.to_string(), String::new()),
    };
    let has_chapters = match &preamble {
        Some(preamble) => class_has_chapters(preamble),
        None => settings.has_chapters(),
    };
    let german_shorthands = match &preamble {
        Some(preamble) => {
            let text = strip_comments(preamble);
            text.find("{babel}").is_some_and(|index| {
                let before = &text[..index];
                let options = before
                    .rfind('[')
                    .map(|start| &before[start..])
                    .unwrap_or("");
                options
                    .trim_start_matches('[')
                    .split([',', ']'])
                    .filter_map(super::languages::from_babel)
                    .any(|id| super::languages::get(id).german_shorthands)
            }) || text.contains("{ngerman}")
        }
        None => super::languages::get(&settings.language).german_shorthands,
    };
    let mut importer = Importer::new(settings, has_chapters, german_shorthands);
    importer.collect_acronyms(source);
    if let Some(preamble) = &preamble {
        importer.collect_environments(preamble);
        importer.read_preamble_metadata(preamble);
    }
    // Vorversion: führendes \color[HTML]{…} entfernen.
    let mut cleaned = body.as_str();
    let trimmed = cleaned.trim_start();
    if let Some(after) = trimmed.strip_prefix("\\color[HTML]") {
        if let Some((value, end)) = read_group(after, 0, b'{', b'}') {
            if value.len() == 6 {
                cleaned = &after[end..];
            }
        }
    }
    let mut content = importer.parse_blocks(cleaned);
    if content.is_empty() {
        content.push(json!({ "type": "paragraph" }));
    }
    if importer.raw_blocks > 0 {
        importer.warn(format!(
            "{} Abschnitt(e) konnten nicht visuell dargestellt werden und bleiben als Roh-LaTeX erhalten.",
            importer.raw_blocks
        ));
    }
    let mut doc = json!({ "type": "doc", "content": content });
    // Sichtbare Einstellungen (Papier, Ränder, Schrift, Sprache) aus der fremden
    // Präambel – für eine Seitenansicht wie im PDF.
    if let Some(preamble) = &preamble {
        if let Value::Object(inferred) = infer_settings_from_preamble(preamble) {
            for (key, value) in inferred {
                importer.patch.entry(key).or_insert(value);
            }
        }
    }
    // Fingerabdrücke mit den Einstellungen, die auch beim Export gelten.
    let patched = settings.with_patch(&Value::Object(importer.patch.clone()));
    assign_source_keys(&mut doc, &patched, has_chapters);
    ImportResult {
        doc,
        preamble,
        postamble,
        has_chapters,
        settings_patch: Value::Object(importer.patch),
        warnings: importer.warnings,
        raw_block_count: importer.raw_blocks,
    }
}

/// Wandelt einfachen LaTeX-Text (z. B. BibTeX-Felder) in Klartext um.
/// Unbekannte Befehle werden entfernt, Gruppenklammern aufgelöst.
pub fn latex_to_plain_text(source: &str) -> String {
    let settings = DocumentSettings::default();
    let mut importer = Importer::new(&settings, true, false);
    if let Some(text) = importer.plain_text(source) {
        return text;
    }
    let nodes = importer.parse_inline(source, &Marks::default());
    let mut text = String::new();
    for node in nodes {
        match node["type"].as_str() {
            Some("text") => text.push_str(node["text"].as_str().unwrap_or("")),
            Some("inlineMath") => text.push_str(node["attrs"]["latex"].as_str().unwrap_or("")),
            Some("rawLatexInline") => text.push_str(&strip_commands(
                node["attrs"]["latex"].as_str().unwrap_or(""),
            )),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Entfernt Befehlsnamen und Klammern aus Roh-LaTeX (\TeX/\LaTeX bleiben als Wort).
fn strip_commands(raw: &str) -> String {
    let mut result = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '{' | '}' => {}
            '\\' => {
                let mut name = String::new();
                while let Some(next) = chars.peek().copied().filter(|c| c.is_ascii_alphabetic()) {
                    name.push(next);
                    chars.next();
                }
                if matches!(name.as_str(), "TeX" | "LaTeX" | "LaTeXe" | "BibTeX") {
                    result.push_str(&name);
                } else if name.is_empty() {
                    if let Some(next) = chars.next() {
                        result.push(next);
                    }
                }
            }
            other => result.push(other),
        }
    }
    result
}
