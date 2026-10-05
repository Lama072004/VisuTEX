//! Round-Trip-Test: Tiptap-JSON → LaTeX → Tiptap-JSON mit dem vollständigen
//! Test-Dokument (alle Knotentypen). Schreibt das erzeugte LaTeX zusätzlich nach
//! `target/visutex-roundtrip/doc.tex`, damit es mit pdfLaTeX/XeLaTeX/LuaLaTeX
//! geprüft werden kann (`scripts/check-latex-engines.sh`).

use serde_json::{Map, Value};
use std::path::PathBuf;
use visutex_lib::core::export::{export_document, ExportOptions};
use visutex_lib::core::import::import_latex;
use visutex_lib::core::settings::DocumentSettings;

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/full-document.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture lesbar"))
        .expect("gültiges JSON")
}

/// Entfernt Attribute, die Standardwerte tragen oder bewusst nicht übertragen werden.
fn normalize(value: &Value, parent_type: &str) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| normalize(item, parent_type))
                .collect(),
        ),
        Value::Object(object) => {
            let node_type = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or(parent_type)
                .to_string();
            let mut result = Map::new();
            for (key, item) in object {
                if key == "attrs" {
                    let mut attrs = Map::new();
                    for (name, attr) in item.as_object().cloned().unwrap_or_default() {
                        let ignored = matches!(
                            (node_type.as_str(), name.as_str()),
                            ("image", "src" | "alt" | "title" | "latexPath")
                                | ("citation", "label")
                                | (
                                    _,
                                    "missingResources"
                                        | "userSetColor"
                                        | "preview"
                                        | "previewCode"
                                        | "logoSrc"
                                )
                        );
                        let default = matches!(attr, Value::Null)
                            || attr == Value::String(String::new())
                            || attr == Value::Bool(false)
                            || (name == "indent" && attr == Value::from(0))
                            || (name == "numbered"
                                && node_type == "heading"
                                && attr == Value::Bool(true))
                            || ((name == "colspan" || name == "rowspan" || name == "start")
                                && attr == Value::from(1))
                            || (name == "placement" && attr == "htbp")
                            || (name == "environment"
                                && (attr == "equation" || attr == "tikzpicture"))
                            || (name == "orientation" && attr == "keep")
                            || (name == "breakType" && attr == "page")
                            || (name == "columns"
                                && attr == Value::from(1)
                                && node_type == "pageBreak")
                            || (name == "command" && (attr == "cite" || attr == "ac"))
                            || (name == "kind" && node_type == "crossReference" && attr == "ref");
                        if ignored || default {
                            continue;
                        }
                        let attr = match (name.as_str(), &attr) {
                            ("color" | "backgroundColor", Value::String(color)) => {
                                Value::String(color.to_uppercase())
                            }
                            // Unterabbildungen: Bildquelle wird beim Export zur Datei
                            ("items", Value::Array(items)) => Value::Array(
                                items
                                    .iter()
                                    .map(|item| {
                                        let mut item =
                                            item.as_object().cloned().unwrap_or_default();
                                        item.retain(|key, value| {
                                            !matches!(
                                                key.as_str(),
                                                "src" | "alt" | "title" | "latexPath"
                                            ) && !value.is_null()
                                                && *value != Value::String(String::new())
                                        });
                                        Value::Object(item)
                                    })
                                    .collect(),
                            ),
                            ("lineHeight", Value::String(text)) => Value::String(text.clone()),
                            _ => attr.clone(),
                        };
                        attrs.insert(name, attr);
                    }
                    if !attrs.is_empty() {
                        result.insert(key.clone(), Value::Object(attrs));
                    }
                } else {
                    result.insert(key.clone(), normalize(item, &node_type));
                }
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

#[test]
fn full_document_round_trips_through_latex() {
    let doc = fixture();
    let settings = DocumentSettings::from_value(&serde_json::json!({
        "bibliography": { "file": "literatur.bib", "style": "ieee" },
        "metadata": { "title": "Test & Titel", "author": "M. Muster" },
        "headerFooter": { "enabled": true, "headerLeft": "{titel}", "footerCenter": "Seite {seite} von {seiten}" }
    }));
    let addon = vec!["\\usepackage{siunitx}".to_string()];
    let exported = export_document(
        &doc,
        &ExportOptions {
            settings: &settings,
            custom_preamble: None,
            addon_preamble: &addon,
        },
    );
    assert!(
        exported.warnings.is_empty(),
        "Warnungen: {:?}",
        exported.warnings
    );
    assert_eq!(exported.assets.len(), 1, "eingebettetes Bild als Datei");

    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/visutex-roundtrip");
    std::fs::create_dir_all(out_dir.join("Abbildungen")).unwrap();
    std::fs::write(out_dir.join("doc.tex"), &exported.latex).unwrap();
    for asset in &exported.assets {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&asset.base64)
            .unwrap();
        std::fs::write(out_dir.join(&asset.path), bytes).unwrap();
    }
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/literatur.bib"),
        out_dir.join("literatur.bib"),
    )
    .unwrap();

    let imported = import_latex(&exported.latex, &settings);
    // Einzige erwartete Meldung: der absichtliche Roh-LaTeX-Block (minipage).
    assert!(
        imported
            .warnings
            .iter()
            .all(|warning| warning.starts_with("1 Abschnitt(e)")),
        "Import-Warnungen: {:?}",
        imported.warnings
    );
    assert_eq!(
        imported.settings_patch["bibliography"]["file"],
        "literatur.bib"
    );

    let original = normalize(&doc, "doc");
    let round_tripped = normalize(&imported.doc, "doc");
    let left = original["content"].as_array().unwrap();
    let right = round_tripped["content"].as_array().unwrap();
    let mut differences = Vec::new();
    for index in 0..left.len().max(right.len()) {
        if left.get(index) != right.get(index) {
            differences.push(format!(
                "Block {index}:\n  vorher:  {}\n  nachher: {}",
                left.get(index).map(Value::to_string).unwrap_or_default(),
                right.get(index).map(Value::to_string).unwrap_or_default()
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));

    // Zweiter Durchlauf muss identisches LaTeX liefern (stabiler Export).
    let again = export_document(
        &imported.doc,
        &ExportOptions {
            settings: &settings,
            custom_preamble: None,
            addon_preamble: &addon,
        },
    );
    assert_eq!(
        again.body.replace("eingebettet-", ""),
        exported.body.replace("eingebettet-", "")
    );
}

#[test]
fn foreign_thesis_template_imports_without_losing_content() {
    // Private Hochschulvorlage des Nutzers (nicht versioniert) – fehlt sie, wird übersprungen.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let Some(source) = [
        "Vorlagen_Test_TEX/Sonstiges/vorlage_fhv_v7.tex",
        "vorlage_fhv_v7.tex",
    ]
    .iter()
    .find_map(|path| std::fs::read_to_string(root.join(path)).ok()) else {
        return;
    };
    let settings = DocumentSettings::default();
    let imported = import_latex(&source, &settings);
    let types: Vec<&str> = imported.doc["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["type"].as_str())
        .collect();
    // Tabellen/Abbildungen mit Kurzbeschriftung (\\caption[kurz]{lang}) bleiben bewusst Roh-LaTeX.
    for expected in [
        "heading",
        "paragraph",
        "orderedList",
        "mathBlock",
        "directoryBlock",
        "blockquote",
        "rawLatexBlock",
    ] {
        assert!(types.contains(&expected), "{expected} fehlt: {types:?}");
    }
    assert!(imported
        .preamble
        .as_deref()
        .unwrap_or("")
        .contains("\\documentclass[a4paper,12pt,twoside]{scrreprt}"));
    // Mit eigener (Fremd-)Präambel exportiert ergibt sich wieder ein vollständiges Dokument.
    let exported = export_document(
        &imported.doc,
        &ExportOptions {
            settings: &settings,
            custom_preamble: imported.preamble.as_deref(),
            addon_preamble: &[],
        },
    );
    assert!(exported
        .latex
        .starts_with("\\documentclass[a4paper,12pt,twoside]{scrreprt}"));
    for fragment in [
        "\\EUR{12345,68}",
        "\\cite[vgl.][Kapitel 2]{bathe_finite-elemente-methoden_1990}",
        "\\begin{eqnarray*}",
        "\\printbibliography",
    ] {
        assert!(
            exported.latex.contains(fragment),
            "{fragment} fehlt im Export"
        );
    }
}

#[test]
fn long_table_breaks_across_pages_and_round_trips() {
    let cell = |kind: &str, text: &str| serde_json::json!({ "type": kind, "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] });
    let mut rows = vec![
        serde_json::json!({ "type": "tableRow", "content": [cell("tableHeader", "Nr"), cell("tableHeader", "Wert")] }),
    ];
    for index in 1..=40 {
        rows.push(serde_json::json!({ "type": "tableRow", "content": [cell("tableCell", &index.to_string()), cell("tableCell", "Messung")] }));
    }
    let doc = serde_json::json!({ "type": "doc", "content": [{
        "type": "table",
        "attrs": { "breakAcrossPages": true, "caption": "Lange Messreihe", "label": "tab:lang", "tableStyle": "booktabs" },
        "content": rows
    }] });
    let settings = DocumentSettings::default();
    let options = ExportOptions {
        settings: &settings,
        custom_preamble: None,
        addon_preamble: &[],
    };
    let first = export_document(&doc, &options);
    for fragment in [
        r"\usepackage{longtable}",
        r"\begin{longtable}",
        r"\caption{Lange Messreihe}\label{tab:lang}\\",
        r"\endfirsthead",
        r"\endhead",
        r"\end{longtable}",
    ] {
        assert!(
            first.latex.contains(fragment),
            "{fragment} fehlt:\n{}",
            first.body
        );
    }
    // keine Gleitumgebung (longtable darf nicht in table stehen)
    assert!(!first.body.contains(r"\begin{table}"));
    let imported = import_latex(&first.latex, &settings);
    let table = &imported.doc["content"][0];
    assert_eq!(table["type"], "table", "{}", imported.doc);
    assert_eq!(table["attrs"]["breakAcrossPages"], true);
    assert_eq!(table["attrs"]["caption"], "Lange Messreihe");
    assert_eq!(table["content"].as_array().map(Vec::len), Some(41));
    let again = export_document(
        &imported.doc,
        &ExportOptions {
            settings: &settings,
            custom_preamble: imported.preamble.as_deref(),
            addon_preamble: &[],
        },
    );
    assert_eq!(again.body, first.body);
}

#[test]
fn subfigures_import_from_foreign_code_and_export_stably() {
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/subfigures.tex"),
    )
    .expect("Fixture lesbar");
    let settings = DocumentSettings::default();
    let imported = import_latex(&source, &settings);
    let node = imported.doc["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["type"] == "subfigures")
        .unwrap_or_else(|| panic!("keine Unterabbildungen: {}", imported.doc));
    let items = node["attrs"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["caption"], "Vorher");
    assert_eq!(items[0]["label"], "fig:vorher");
    assert_eq!(items[0]["position"], "b");
    assert_eq!(items[0]["boxPercent"], 45);
    assert_eq!(items[1]["latexPath"], "bilder/nachher.png");
    assert_eq!(node["attrs"]["caption"], "Vergleich");
    assert_eq!(node["attrs"]["label"], "fig:vergleich");

    // Unverändert: Originalcode bleibt erhalten
    let options = ExportOptions {
        settings: &settings,
        custom_preamble: imported.preamble.as_deref(),
        addon_preamble: &[],
    };
    let unchanged = export_document(&imported.doc, &options);
    assert!(unchanged
        .body
        .contains(r"\begin{subfigure}[b]{0.45\textwidth}"));

    // Bearbeitet: neu erzeugt, und der erzeugte Code ist selbst stabil
    let mut doc = imported.doc.clone();
    for block in doc["content"].as_array_mut().unwrap() {
        if block["type"] == "subfigures" {
            block["attrs"]["items"][0]["caption"] = Value::from("Vorher (neu)");
            if let Some(attrs) = block["attrs"].as_object_mut() {
                attrs.remove("sourceLatex");
            }
        }
    }
    let plain = DocumentSettings::default();
    let generated = export_document(
        &doc,
        &ExportOptions {
            settings: &plain,
            custom_preamble: None,
            addon_preamble: &[],
        },
    );
    for fragment in [
        r"\usepackage{subcaption}",
        r"\begin{subfigure}[b]{0.45\linewidth}",
        r"\caption{Vorher (neu)}",
        r"\label{fig:nachher}",
        r"\hfill",
        r"\caption{Vergleich}",
    ] {
        assert!(
            generated.latex.contains(fragment),
            "{fragment} fehlt:\n{}",
            generated.body
        );
    }
    let again = import_latex(&generated.latex, &plain);
    let reexported = export_document(
        &again.doc,
        &ExportOptions {
            settings: &plain,
            custom_preamble: again.preamble.as_deref(),
            addon_preamble: &[],
        },
    );
    assert_eq!(reexported.body, generated.body);
    let analysis = visutex_lib::core::analysis::analyze(&again.doc);
    let labels: Vec<&str> = analysis.labels.iter().map(|l| l.label.as_str()).collect();
    for label in ["fig:vorher", "fig:nachher", "fig:vergleich"] {
        assert!(labels.contains(&label), "{label} fehlt in {labels:?}");
    }
}

#[test]
fn pdfa_setting_writes_document_metadata_and_imports_back() {
    let doc = serde_json::json!({ "type": "doc", "content": [
        { "type": "paragraph", "content": [{ "type": "text", "text": "Archiv" }] }
    ] });
    let settings = DocumentSettings::from_value(&serde_json::json!({
        "language": "english",
        "metadata": { "title": "Arbeit", "pdfStandard": "a-2b" }
    }));
    let exported = export_document(
        &doc,
        &ExportOptions {
            settings: &settings,
            custom_preamble: None,
            addon_preamble: &[],
        },
    );
    let metadata_line = exported
        .latex
        .lines()
        .position(|line| line == r"\DocumentMetadata{pdfstandard=A-2b, lang=en-GB}")
        .unwrap_or_else(|| panic!("keine DocumentMetadata-Zeile:\n{}", exported.latex));
    let class_line = exported
        .latex
        .lines()
        .position(|line| line.starts_with(r"\documentclass"))
        .unwrap();
    assert!(metadata_line < class_line);
    let imported = import_latex(&exported.latex, &DocumentSettings::default());
    assert_eq!(imported.settings_patch["metadata"]["pdfStandard"], "a-2b");
    // ungültige Angaben werden verworfen
    let invalid =
        DocumentSettings::from_value(&serde_json::json!({ "metadata": { "pdfStandard": "x-9" } }));
    assert_eq!(invalid.metadata.pdf_standard, "");
}
