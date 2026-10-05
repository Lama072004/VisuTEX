//! Projektformat v2 (`.visutex`) und Migration alter Projektdateien.
//!
//! ```json
//! { "format": "visutex-project", "version": 2, "settings": {…}, "document": {…Tiptap…},
//!   "customPreamble": null, "lastMode": "visual", "code": null }
//! ```
//!
//! Alte `.json`-Dateien enthielten nur das Tiptap-Dokument der Vorversion. Sie
//! werden beim Laden in das aktuelle Schema überführt (`migrate_document`).
//! Die Migration ist idempotent: aktuelle Dokumente bleiben unverändert.

use super::export::{
    attr, attr_f64, attr_str, children, css_color_to_hex, embedded_asset, node_type, EmbeddedAsset,
};
use super::settings::{is_safe_relative_path, DocumentSettings};
use serde::Serialize;
use serde_json::{json, Map, Value};

pub const FORMAT: &str = "visutex-project";
pub const VERSION: u64 = 2;

/// Breite des Satzspiegels bei A4 mit 25-mm-Rändern in CSS-Pixeln (160 mm bei 96 dpi).
const TEXT_WIDTH_PX: f64 = 605.0;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub format: String,
    pub version: u64,
    pub settings: DocumentSettings,
    pub document: Value,
    pub custom_preamble: Option<String>,
    /// visual | code
    pub last_mode: String,
    /// Zuletzt bearbeiteter Code (nur relevant, wenn `last_mode == "code"`).
    pub code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProject {
    pub project: Project,
    /// Dateien, die bei der Migration aus eingebetteten Bildern entstanden sind
    /// und in den Projektordner geschrieben werden müssen.
    pub assets: Vec<EmbeddedAsset>,
    pub warnings: Vec<String>,
    pub migrated: bool,
}

pub fn empty_document() -> Value {
    json!({ "type": "doc", "content": [{ "type": "paragraph" }] })
}

impl Project {
    pub fn new(settings: DocumentSettings, document: Value) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            settings,
            document,
            custom_preamble: None,
            last_mode: "visual".into(),
            code: None,
        }
    }
}

/// Liest eine Projektdatei (v2 oder reines Tiptap-JSON der Vorversion).
pub fn parse_project(text: &str) -> Result<LoadedProject, String> {
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Die Projektdatei ist kein gültiges JSON: {error}"))?;
    let is_v2 = value.get("format").and_then(Value::as_str) == Some(FORMAT);
    if is_v2 {
        let version = value.get("version").and_then(Value::as_u64).unwrap_or(0);
        if version > VERSION {
            return Err(format!(
                "Die Projektdatei stammt aus einer neueren VisuTeX-Version (Format {version})."
            ));
        }
    } else if node_type(&value) != "doc" {
        return Err("Die Datei ist weder ein VisuTeX-Projekt noch ein Tiptap-Dokument.".into());
    }

    let settings = DocumentSettings::from_value(value.get("settings").unwrap_or(&Value::Null));
    let raw_document = if is_v2 {
        value
            .get("document")
            .cloned()
            .unwrap_or_else(empty_document)
    } else {
        value.clone()
    };
    if node_type(&raw_document) != "doc" {
        return Err("Das Projekt enthält kein gültiges Dokument.".into());
    }
    let migration = migrate_document(&raw_document);
    let text_or_none = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string)
    };
    let last_mode = match value.get("lastMode").and_then(Value::as_str) {
        Some("code") => "code",
        _ => "visual",
    };
    let mut warnings = migration.warnings;
    if !is_v2 {
        warnings.insert(
            0,
            "Projekt aus einer früheren VisuTeX-Version wurde in das aktuelle Format überführt."
                .into(),
        );
    }
    Ok(LoadedProject {
        migrated: !is_v2 || migration.changed,
        project: Project {
            format: FORMAT.into(),
            version: VERSION,
            settings,
            document: migration.document,
            custom_preamble: text_or_none("customPreamble"),
            last_mode: last_mode.into(),
            code: text_or_none("code"),
        },
        assets: migration.assets,
        warnings,
    })
}

/// Serialisiert ein Projekt. Laufzeitattribute (Bildquellen verknüpfter Bilder,
/// Fehlerhinweise) werden nicht gespeichert.
pub fn serialize_project(project: &Project) -> Result<String, String> {
    let mut project = project.clone();
    project.format = FORMAT.into();
    project.version = VERSION;
    project.document = strip_runtime_attributes(&project.document);
    serde_json::to_string_pretty(&project)
        .map_err(|error| format!("Projekt konnte nicht serialisiert werden: {error}"))
}

fn strip_runtime_attributes(node: &Value) -> Value {
    let Value::Object(object) = node else {
        return node.clone();
    };
    let kind = node_type(node);
    let mut result = Map::new();
    for (key, value) in object {
        match key.as_str() {
            "attrs" => {
                let mut attrs = value.as_object().cloned().unwrap_or_default();
                attrs.remove("missingResource");
                attrs.remove("missingResources");
                if kind == "image" && !attr_str(node, "latexPath").is_empty() {
                    attrs.insert("src".into(), Value::Null);
                }
                if kind == "titlePage" {
                    attrs.remove("logoSrc");
                }
                result.insert(key.clone(), Value::Object(attrs));
            }
            "content" => {
                let items = value
                    .as_array()
                    .map(|items| items.iter().map(strip_runtime_attributes).collect())
                    .unwrap_or_default();
                result.insert(key.clone(), Value::Array(items));
            }
            _ => {
                result.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(result)
}

/// Alle Projektdateien, auf die das Dokument verweist (Bilder, Logo, Literatur).
pub fn referenced_files(document: &Value, settings: &DocumentSettings) -> Vec<String> {
    fn walk(node: &Value, files: &mut Vec<String>) {
        let mut push = |path: String| {
            let path = path.trim().replace('\\', "/");
            if !path.is_empty() && is_safe_relative_path(&path) && !files.contains(&path) {
                files.push(path);
            }
        };
        match node_type(node) {
            "image" => push(attr_str(node, "latexPath")),
            "titlePage" => push(attr_str(node, "logoPath")),
            "subfigures" => {
                for item in attr(node, "items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    push(
                        item.get("latexPath")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    );
                }
            }
            _ => {}
        }
        for child in children(node) {
            walk(child, files);
        }
    }
    let mut files = Vec::new();
    walk(document, &mut files);
    let bib = settings.bibliography.file.trim();
    if !bib.is_empty() && is_safe_relative_path(bib) && !files.iter().any(|file| file == bib) {
        files.push(bib.to_string());
    }
    files
}

// ---------------------------------------------------------------- Migration

pub struct Migration {
    pub document: Value,
    pub assets: Vec<EmbeddedAsset>,
    pub warnings: Vec<String>,
    pub changed: bool,
}

struct Migrator {
    assets: Vec<EmbeddedAsset>,
    warnings: Vec<String>,
}

pub fn migrate_document(document: &Value) -> Migration {
    let mut migrator = Migrator {
        assets: Vec::new(),
        warnings: Vec::new(),
    };
    let migrated = migrator.block(document);
    let migrated = migrated.into_iter().next().unwrap_or_else(empty_document);
    Migration {
        changed: &migrated != document,
        document: migrated,
        assets: migrator.assets,
        warnings: migrator.warnings,
    }
}

fn attrs_of(node: &Value) -> Map<String, Value> {
    node.get("attrs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn with_attrs(node: &Value, attrs: Map<String, Value>) -> Value {
    let mut node = node.clone();
    if let Some(object) = node.as_object_mut() {
        object.insert("attrs".into(), Value::Object(attrs));
    }
    node
}

fn with_content(node: &Value, content: Vec<Value>) -> Value {
    let mut node = node.clone();
    if let Some(object) = node.as_object_mut() {
        if content.is_empty() {
            object.remove("content");
        } else {
            object.insert("content".into(), Value::Array(content));
        }
    }
    node
}

fn paragraph(text: &str) -> Value {
    if text.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] })
    }
}

/// Absätze aus Klartext (Leerzeilen trennen Absätze).
fn paragraphs_from_text(text: &str) -> Vec<Value> {
    let normalized = text.replace("\r\n", "\n");
    let mut paragraphs: Vec<Value> = normalized
        .split("\n\n")
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| paragraph(&part.replace('\n', " ")))
        .collect();
    if paragraphs.is_empty() {
        paragraphs.push(paragraph(""));
    }
    paragraphs
}

/// Einzug-Stufe (1,25 cm je Stufe) aus CSS-Längen wie `2em`, `1.25cm`, `24px`.
fn indent_level(value: &str) -> u64 {
    let value = value.trim();
    let centimeters = [
        ("cm", 1.0),
        ("mm", 0.1),
        ("em", 0.42),
        ("pt", 0.0353),
        ("px", 0.0265),
    ]
    .iter()
    .find_map(|(unit, factor)| {
        value
            .strip_suffix(unit)
            .and_then(|number| number.trim().parse::<f64>().ok())
            .map(|number| number * factor)
    })
    .unwrap_or(0.0);
    if centimeters <= 0.0 {
        0
    } else {
        ((centimeters / 1.25).round() as u64).clamp(1, 8)
    }
}

impl Migrator {
    /// Migriert einen Knoten; kann ihn durch mehrere Knoten ersetzen.
    fn block(&mut self, node: &Value) -> Vec<Value> {
        match node_type(node) {
            "frontmatterBlock"
                if attr(node, "body").is_some() || attr_str(node, "kind") == "title" =>
            {
                self.legacy_frontmatter(node)
            }
            "paragraph" | "heading" => vec![self.textblock(node)],
            "image" => vec![self.image(node)],
            "citation" | "crossReference" => vec![self.inline_atom(node)],
            _ => {
                let content: Vec<Value> = children(node)
                    .iter()
                    .flat_map(|child| self.block(child))
                    .collect();
                if children(node).is_empty() {
                    vec![node.clone()]
                } else {
                    vec![with_content(node, content)]
                }
            }
        }
    }

    fn legacy_frontmatter(&mut self, node: &Value) -> Vec<Value> {
        let kind = attr_str(node, "kind");
        let title = attr_str(node, "title");
        let body = attr_str(node, "body");
        if kind == "title" {
            let logo = attr_str(node, "logoPath").replace('\\', "/");
            let mut result = vec![json!({
                "type": "titlePage",
                "attrs": {
                    "title": title,
                    "subtitle": attr_str(node, "subtitle"),
                    "logoPath": if is_safe_relative_path(&logo) { logo } else { String::new() },
                }
            })];
            if !body.trim().is_empty() {
                self.warnings.push(
                    "Der Freitext der alten Titelseite wurde als Absatz nach der Titelseite eingefügt – bitte in die Felder der Titelseite übernehmen."
                        .into(),
                );
                result.extend(paragraphs_from_text(&body));
            }
            return result;
        }
        let kind = match kind.as_str() {
            "confidentiality" | "dedication" | "summary" | "abstract" | "preface" => kind,
            _ => "abstract".into(),
        };
        vec![json!({
            "type": "frontmatterBlock",
            "attrs": { "kind": kind, "title": title, "inToc": false },
            "content": paragraphs_from_text(&body),
        })]
    }

    fn image(&mut self, node: &Value) -> Value {
        let mut attrs = attrs_of(node);
        if attrs.get("widthPercent").is_none() {
            if let Some(width) = attr_f64(node, "width").filter(|width| *width > 0.0) {
                let percent = (width / TEXT_WIDTH_PX * 100.0).round().clamp(5.0, 100.0);
                attrs.insert("widthPercent".into(), json!(percent));
                attrs.insert("width".into(), Value::Null);
            }
        }
        let src = attr_str(node, "src");
        if attr_str(node, "latexPath").is_empty() && src.starts_with("data:") {
            if let Some(asset) = embedded_asset(&src) {
                attrs.insert("latexPath".into(), json!(asset.path));
                attrs.insert("src".into(), Value::Null);
                if !self.assets.iter().any(|known| known.path == asset.path) {
                    self.assets.push(asset);
                }
            }
        }
        with_attrs(node, attrs)
    }

    fn inline_atom(&mut self, node: &Value) -> Value {
        let mut attrs = attrs_of(node);
        match node_type(node) {
            "citation" => {
                if let Some(key) = attrs.remove("key") {
                    let keys = attrs.get("keys").and_then(Value::as_str).unwrap_or("");
                    if keys.is_empty() {
                        attrs.insert("keys".into(), key);
                    }
                }
            }
            "crossReference" => {
                if let Some(page) = attrs.remove("page") {
                    if attrs.get("kind").is_none() {
                        let kind = if page == Value::Bool(true) {
                            "pageref"
                        } else {
                            "ref"
                        };
                        attrs.insert("kind".into(), json!(kind));
                    }
                }
            }
            _ => {}
        }
        with_attrs(node, attrs)
    }

    /// Absatz/Überschrift: Absatzformat aus Inline-Marken übernehmen,
    /// Abkürzungs-Marken in Abkürzungs-Knoten umwandeln.
    fn textblock(&mut self, node: &Value) -> Value {
        let mut attrs = attrs_of(node);
        // Rahmen/Schattierung der Vorversion waren CSS-Werte.
        if let Some(border) = attrs.get("border").and_then(Value::as_str) {
            if border != "single" {
                let border = if border.trim().is_empty() || border.contains("none") {
                    Value::Null
                } else {
                    json!("single")
                };
                attrs.insert("border".into(), border);
            }
        }
        if let Some(shading) = attrs.get("paragraphShading").and_then(Value::as_str) {
            if !(shading.len() == 7 && shading.starts_with('#')) {
                let value = css_color_to_hex(shading)
                    .map(|hex| json!(format!("#{}", hex.to_ascii_lowercase())))
                    .unwrap_or(Value::Null);
                attrs.insert("paragraphShading".into(), value);
            }
        }

        let mut content: Vec<Value> = Vec::new();
        let mut line_height: Option<String> = None;
        let mut indent: u64 = 0;
        for child in children(node) {
            let child = self.inline_atom_if_needed(child);
            if node_type(&child) != "text" {
                content.push(child);
                continue;
            }
            let mut child = child;
            let mut acronym: Option<Map<String, Value>> = None;
            if let Some(marks) = child.get_mut("marks").and_then(Value::as_array_mut) {
                marks.retain_mut(|mark| match mark.get("type").and_then(Value::as_str) {
                    Some("acronym") => {
                        acronym = Some(attrs_of(mark));
                        false
                    }
                    Some("textStyle") => {
                        if let Some(mark_attrs) =
                            mark.get_mut("attrs").and_then(Value::as_object_mut)
                        {
                            if !mark_attrs.contains_key("lineHeight")
                                && !mark_attrs.contains_key("textIndent")
                            {
                                return true;
                            }
                            if let Some(value) = mark_attrs.remove("lineHeight") {
                                if let Some(text) = value.as_str() {
                                    if text.trim().parse::<f64>().is_ok() {
                                        line_height.get_or_insert_with(|| text.trim().to_string());
                                    }
                                }
                            }
                            if let Some(value) = mark_attrs.remove("textIndent") {
                                indent = indent.max(indent_level(value.as_str().unwrap_or("")));
                            }
                            mark_attrs
                                .values()
                                .any(|value| !value.is_null() && value != &Value::Bool(false))
                        } else {
                            true
                        }
                    }
                    _ => true,
                });
                if marks.is_empty() {
                    if let Some(object) = child.as_object_mut() {
                        object.remove("marks");
                    }
                }
            }
            match acronym {
                Some(mark) => {
                    let key = mark
                        .get("key")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let text = child
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    // Aufeinanderfolgende Textstücke derselben Abkürzung zusammenfassen.
                    let previous_same = content.last().is_some_and(|last| {
                        node_type(last) == "acronym" && attr_str(last, "key") == key
                    });
                    if !previous_same {
                        let short = mark
                            .get("short")
                            .and_then(Value::as_str)
                            .filter(|short| !short.is_empty())
                            .map(str::to_string)
                            .unwrap_or(text);
                        content.push(json!({
                            "type": "acronym",
                            "attrs": {
                                "key": key,
                                "short": short,
                                "long": mark.get("long").cloned().unwrap_or(json!("")),
                                "command": "ac",
                            }
                        }));
                    }
                }
                None => content.push(child),
            }
        }
        if let Some(line_height) = line_height {
            if attrs.get("lineHeight").is_none_or(Value::is_null) {
                attrs.insert("lineHeight".into(), json!(line_height));
            }
        }
        if indent > 0 && attr_f64(node, "indent").unwrap_or(0.0) == 0.0 {
            attrs.insert("indent".into(), json!(indent));
        }
        let node = if attrs.is_empty() && node.get("attrs").is_none() {
            node.clone()
        } else {
            with_attrs(node, attrs)
        };
        if children(&node).is_empty() {
            node
        } else {
            with_content(&node, content)
        }
    }

    fn inline_atom_if_needed(&mut self, node: &Value) -> Value {
        match node_type(node) {
            "citation" | "crossReference" => self.inline_atom(node),
            "image" => self.image(node),
            _ => node.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_document() {
        let legacy = json!({
            "type": "doc",
            "content": [
                { "type": "frontmatterBlock", "attrs": { "kind": "title", "title": "Titel", "subtitle": "Untertitel", "body": "", "logoPath": "Abbildungen/logo.png" } },
                { "type": "frontmatterBlock", "attrs": { "kind": "abstract", "title": "Abstract", "body": "Erster Absatz.\n\nZweiter Absatz." } },
                { "type": "paragraph", "attrs": { "border": "1px solid #000000", "paragraphShading": "rgb(221, 235, 247)" }, "content": [
                    { "type": "text", "text": "Die " },
                    { "type": "text", "text": "API", "marks": [{ "type": "acronym", "attrs": { "key": "API", "short": "API", "long": "Programmierschnittstelle" } }] },
                    { "type": "text", "text": " wird", "marks": [{ "type": "textStyle", "attrs": { "lineHeight": "1.5", "textIndent": "2.5cm", "color": null } }] },
                    { "type": "citation", "attrs": { "key": "knuth1984", "label": "Knuth" } },
                    { "type": "crossReference", "attrs": { "label": "fig:a", "page": true } }
                ]},
                { "type": "image", "attrs": { "src": "data:image/png;base64,iVBORw0KGgo=", "width": 302, "latexPath": "" } }
            ]
        });
        let migration = migrate_document(&legacy);
        assert!(migration.changed);
        let content = migration.document["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "titlePage");
        assert_eq!(content[0]["attrs"]["logoPath"], "Abbildungen/logo.png");
        assert_eq!(content[1]["type"], "frontmatterBlock");
        assert_eq!(content[1]["content"].as_array().unwrap().len(), 2);
        let paragraph = &content[2];
        assert_eq!(paragraph["attrs"]["border"], "single");
        assert_eq!(paragraph["attrs"]["paragraphShading"], "#ddebf7");
        assert_eq!(paragraph["attrs"]["lineHeight"], "1.5");
        assert_eq!(paragraph["attrs"]["indent"], 2);
        let inline = paragraph["content"].as_array().unwrap();
        assert_eq!(inline[1]["type"], "acronym");
        assert_eq!(inline[1]["attrs"]["long"], "Programmierschnittstelle");
        assert!(
            inline[2].get("marks").is_none(),
            "leere textStyle-Marke entfernt"
        );
        assert_eq!(inline[3]["attrs"]["keys"], "knuth1984");
        assert_eq!(inline[4]["attrs"]["kind"], "pageref");
        let image = &content[3];
        assert_eq!(image["attrs"]["widthPercent"], 50.0);
        assert!(image["attrs"]["latexPath"]
            .as_str()
            .unwrap()
            .starts_with("Abbildungen/eingebettet-"));
        assert_eq!(migration.assets.len(), 1);

        // Idempotent
        let again = migrate_document(&migration.document);
        assert!(!again.changed);
    }

    #[test]
    fn current_fixture_is_unchanged_by_migration() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/full-document.json");
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let document: Value = serde_json::from_str(&text).unwrap();
        let migration = migrate_document(&document);
        // Einziger erwarteter Unterschied: das eingebettete Bild wird zur Datei.
        let images: Vec<&Value> = migration.document["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["type"] == "image")
            .collect();
        assert_eq!(images.len(), 1);
        assert_eq!(migration.assets.len(), 1);
        let mut expected = document.clone();
        for node in expected["content"].as_array_mut().unwrap() {
            if node["type"] == "image" {
                node["attrs"]["latexPath"] = images[0]["attrs"]["latexPath"].clone();
                node["attrs"]["src"] = Value::Null;
            }
        }
        assert_eq!(migration.document, expected);
    }

    #[test]
    fn project_round_trip_and_runtime_attributes() {
        let mut project = Project::new(
            DocumentSettings::default(),
            json!({ "type": "doc", "content": [
                { "type": "image", "attrs": { "src": "asset://x", "latexPath": "Abbildungen/a.png", "missingResource": "x" } }
            ]}),
        );
        project.custom_preamble = Some("\\documentclass{article}".into());
        let text = serialize_project(&project).unwrap();
        assert!(!text.contains("asset://x"));
        assert!(!text.contains("missingResource"));
        let loaded = parse_project(&text).unwrap();
        assert!(!loaded.migrated);
        assert_eq!(
            loaded.project.custom_preamble.as_deref(),
            Some("\\documentclass{article}")
        );
        assert_eq!(
            referenced_files(&loaded.project.document, &loaded.project.settings),
            vec!["Abbildungen/a.png".to_string()]
        );
        assert!(parse_project("{\"format\":\"visutex-project\",\"version\":99}").is_err());
        assert!(parse_project("[]").is_err());
    }
}
