//! Dokumenteinstellungen – gemeinsame Quelle für Vorschau (CSS im Frontend),
//! LaTeX-Export und Projektdatei.
//!
//! Eingaben aus dem Frontend, aus alten Projektdateien oder aus dem
//! localStorage früherer Versionen werden tolerant gelesen und auf gültige
//! Werte begrenzt (`DocumentSettings::from_value`). Dadurch kann keine
//! ungültige oder schädliche Präambel entstehen.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_FONT: &str = "Latin Modern Roman";

pub const AVAILABLE_FONTS: &[&str] = &[
    "Latin Modern Roman",
    "Arial",
    "Calibri",
    "Cambria",
    "Consolas",
    "Courier New",
    "Georgia",
    "Liberation Sans",
    "Liberation Serif",
    "Palatino Linotype",
    "Segoe UI",
    "Times New Roman",
    "Verdana",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooter {
    pub enabled: bool,
    pub header_left: String,
    pub header_center: String,
    pub header_right: String,
    pub footer_left: String,
    pub footer_center: String,
    pub footer_right: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bibliography {
    /// Pfad der .bib-Datei relativ zum Projektordner.
    pub file: String,
    /// ieee | plainnat | abbrvnat | plainnat-authoryear | alpha
    pub style: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSettings {
    /// a4 | a5 | letter | legal | custom
    pub paper_format: String,
    /// portrait | landscape
    pub orientation: String,
    pub margins: Margins,
    pub columns: u8,
    pub custom_width: f64,
    pub custom_height: f64,
    /// scrreprt | article
    pub document_class: String,
    pub twoside: bool,
    pub binding_offset: f64,
    pub default_font_family: String,
    pub default_font_size: u8,
    pub line_spacing: f64,
    /// indent | skip
    pub paragraph_style: String,
    /// ngerman | naustrian | english
    pub language: String,
    pub numbering_depth: u8,
    pub toc_depth: u8,
    pub page_display_color: String,
    pub page_pdf_color: String,
    pub header_footer: HeaderFooter,
    pub metadata: Metadata,
    pub bibliography: Bibliography,
}

impl Default for HeaderFooter {
    fn default() -> Self {
        Self {
            enabled: false,
            header_left: String::new(),
            header_center: String::new(),
            header_right: "{kapitel}".into(),
            footer_left: String::new(),
            footer_center: "{seite}".into(),
            footer_right: String::new(),
        }
    }
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            paper_format: "a4".into(),
            orientation: "portrait".into(),
            margins: Margins {
                top: 25.0,
                right: 25.0,
                bottom: 25.0,
                left: 25.0,
            },
            columns: 1,
            custom_width: 210.0,
            custom_height: 297.0,
            document_class: "scrreprt".into(),
            twoside: false,
            binding_offset: 0.0,
            default_font_family: DEFAULT_FONT.into(),
            default_font_size: 12,
            line_spacing: 1.0,
            paragraph_style: "indent".into(),
            language: "ngerman".into(),
            numbering_depth: 3,
            toc_depth: 3,
            page_display_color: String::new(),
            page_pdf_color: String::new(),
            header_footer: HeaderFooter::default(),
            metadata: Metadata::default(),
            bibliography: Bibliography {
                file: String::new(),
                style: "ieee".into(),
            },
        }
    }
}

fn number(value: Option<&Value>, min: f64, max: f64, fallback: f64) -> f64 {
    let parsed = match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    };
    match parsed {
        Some(value) if value.is_finite() => value.clamp(min, max),
        _ => fallback,
    }
}

fn pick(value: Option<&Value>, allowed: &[&str], fallback: &str) -> String {
    match value.and_then(Value::as_str) {
        Some(text) if allowed.contains(&text) => text.to_string(),
        _ => fallback.to_string(),
    }
}

fn color(value: Option<&Value>) -> String {
    match value.and_then(Value::as_str) {
        Some(text)
            if text.len() == 7
                && text.starts_with('#')
                && text[1..].chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            text.to_ascii_lowercase()
        }
        _ => String::new(),
    }
}

fn text(value: Option<&Value>, fallback: &str) -> String {
    match value {
        Some(Value::String(text)) => text.replace(['\r', '\n'], " ").chars().take(300).collect(),
        Some(_) => String::new(),
        None => fallback.to_string(),
    }
}

/// Relative Projektpfade: keine absoluten Pfade, kein `..`, keine LaTeX-Sonderzeichen.
/// Führt `patch` rekursiv in `target` zusammen (Objekte werden gemischt).
pub fn merge_json(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                match target.get_mut(key) {
                    Some(existing) if existing.is_object() && value.is_object() => {
                        merge_json(existing, value)
                    }
                    _ => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

pub fn is_safe_relative_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    !normalized.is_empty()
        && !normalized.starts_with('/')
        && !(normalized.len() > 1 && normalized.as_bytes()[1] == b':')
        && !normalized.split('/').any(|segment| segment == "..")
        && !normalized.chars().any(|c| {
            matches!(
                c,
                '{' | '}' | '\\' | '%' | '#' | '$' | '&' | '~' | '^' | '\r' | '\n'
            )
        })
}

impl DocumentSettings {
    /// Liest Einstellungen tolerant aus beliebigem JSON (auch aus Vorversionen).
    pub fn from_value(value: &Value) -> Self {
        let defaults = DocumentSettings::default();
        let empty = serde_json::Map::new();
        let raw = value.as_object().unwrap_or(&empty);
        let get = |key: &str| raw.get(key);
        let object = |key: &str| {
            raw.get(key)
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default()
        };
        let margins = object("margins");
        let header_footer = object("headerFooter");
        let metadata = object("metadata");
        let bibliography = object("bibliography");
        // Frühere Versionen speicherten nur den Dateinamen unter `bibliographyFile`.
        let bib_file = bibliography
            .get("file")
            .and_then(Value::as_str)
            .or_else(|| get("bibliographyFile").and_then(Value::as_str))
            .unwrap_or("")
            .replace('\\', "/");
        let font = get("defaultFontFamily")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|font| {
                !font.is_empty()
                    && font
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
            })
            .unwrap_or(DEFAULT_FONT)
            .to_string();
        let header_flag = header_footer
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || get("headerFooterEnabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let hf_default = HeaderFooter::default();

        DocumentSettings {
            paper_format: pick(
                get("paperFormat"),
                &["a4", "a5", "letter", "legal", "custom"],
                &defaults.paper_format,
            ),
            orientation: pick(
                get("orientation"),
                &["portrait", "landscape"],
                &defaults.orientation,
            ),
            margins: Margins {
                top: number(margins.get("top"), 0.0, 100.0, defaults.margins.top),
                right: number(margins.get("right"), 0.0, 100.0, defaults.margins.right),
                bottom: number(margins.get("bottom"), 0.0, 100.0, defaults.margins.bottom),
                left: number(margins.get("left"), 0.0, 100.0, defaults.margins.left),
            },
            columns: number(get("columns"), 1.0, 3.0, 1.0).round() as u8,
            custom_width: number(get("customWidth"), 50.0, 1000.0, defaults.custom_width),
            custom_height: number(get("customHeight"), 50.0, 1000.0, defaults.custom_height),
            document_class: pick(
                get("documentClass"),
                &["scrreprt", "article"],
                &defaults.document_class,
            ),
            twoside: get("twoside").and_then(Value::as_bool).unwrap_or(false),
            binding_offset: number(get("bindingOffset"), 0.0, 50.0, defaults.binding_offset),
            default_font_family: font,
            default_font_size: number(get("defaultFontSize"), 8.0, 20.0, 12.0).round() as u8,
            line_spacing: number(get("lineSpacing"), 1.0, 3.0, 1.0),
            paragraph_style: pick(get("paragraphStyle"), &["indent", "skip"], "indent"),
            language: pick(
                get("language"),
                &super::languages::ids(),
                super::languages::DEFAULT_LANGUAGE,
            ),
            numbering_depth: number(get("numberingDepth"), 0.0, 5.0, 3.0).round() as u8,
            toc_depth: number(get("tocDepth"), 0.0, 5.0, 3.0).round() as u8,
            page_display_color: color(get("pageDisplayColor")),
            page_pdf_color: color(get("pagePdfColor")),
            header_footer: HeaderFooter {
                enabled: header_flag,
                header_left: text(header_footer.get("headerLeft"), &hf_default.header_left),
                header_center: text(header_footer.get("headerCenter"), &hf_default.header_center),
                header_right: text(header_footer.get("headerRight"), &hf_default.header_right),
                footer_left: text(header_footer.get("footerLeft"), &hf_default.footer_left),
                footer_center: text(header_footer.get("footerCenter"), &hf_default.footer_center),
                footer_right: text(header_footer.get("footerRight"), &hf_default.footer_right),
            },
            metadata: Metadata {
                title: text(metadata.get("title"), ""),
                author: text(metadata.get("author"), ""),
                subject: text(metadata.get("subject"), ""),
                keywords: text(metadata.get("keywords"), ""),
            },
            bibliography: Bibliography {
                file: if !bib_file.is_empty()
                    && is_safe_relative_path(&bib_file)
                    && bib_file.to_ascii_lowercase().ends_with(".bib")
                {
                    bib_file
                } else {
                    String::new()
                },
                style: pick(
                    bibliography.get("style"),
                    &[
                        "ieee",
                        "plainnat",
                        "abbrvnat",
                        "plainnat-authoryear",
                        "alpha",
                    ],
                    "ieee",
                ),
            },
        }
    }

    pub fn has_chapters(&self) -> bool {
        self.document_class == "scrreprt"
    }

    /// Einstellungen mit eingearbeitetem Teil-Patch (z. B. aus dem Import).
    pub fn with_patch(&self, patch: &Value) -> Self {
        let mut merged = serde_json::to_value(self).unwrap_or(Value::Null);
        merge_json(&mut merged, patch);
        Self::from_value(&merged)
    }

    /// Papiermaße (mm) unter Berücksichtigung der Ausrichtung.
    pub fn page_dimensions(&self) -> (f64, f64) {
        let (width, height) = match self.paper_format.as_str() {
            "a5" => (148.0, 210.0),
            "letter" => (215.9, 279.4),
            "legal" => (215.9, 355.6),
            "custom" => (self.custom_width, self.custom_height),
            _ => (210.0, 297.0),
        };
        if self.orientation == "landscape" {
            (height, width)
        } else {
            (width, height)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_invalid_and_legacy_values() {
        let settings = DocumentSettings::from_value(&json!({
            "paperFormat": "a3",
            "margins": { "top": -4, "right": "12", "bottom": 400 },
            "columns": 7,
            "defaultFontFamily": "Evil}\\font",
            "defaultFontSize": 96,
            "latexEngine": "pdflatex",
            "bibliographyFile": "quellen.bib",
            "headerFooterEnabled": true,
            "pagePdfColor": "#ABCDEF",
            "pageDisplayColor": "red"
        }));
        assert_eq!(settings.paper_format, "a4");
        assert_eq!(
            settings.margins,
            Margins {
                top: 0.0,
                right: 12.0,
                bottom: 100.0,
                left: 25.0
            }
        );
        assert_eq!(settings.columns, 3);
        assert_eq!(settings.default_font_family, DEFAULT_FONT);
        assert_eq!(settings.default_font_size, 20);
        assert_eq!(settings.bibliography.file, "quellen.bib");
        assert!(settings.header_footer.enabled);
        assert_eq!(settings.page_pdf_color, "#abcdef");
        assert_eq!(settings.page_display_color, "");
    }

    #[test]
    fn rejects_unsafe_paths() {
        assert!(is_safe_relative_path("Abbildungen/bild.png"));
        assert!(!is_safe_relative_path("../geheim.bib"));
        assert!(!is_safe_relative_path("C:/x.bib"));
        assert!(!is_safe_relative_path("/etc/passwd"));
        assert!(!is_safe_relative_path("a}b.bib"));
    }
}
