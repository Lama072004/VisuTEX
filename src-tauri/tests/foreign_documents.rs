//! Fremddokumente: alle `.tex`-Hauptdateien in `tests/fixtures/fremd/` (synthetische
//! Beispiele) und – falls vorhanden – in `Vorlagen_Test_TEX/` (private Vorlagen und
//! Arbeiten, nicht versioniert) werden importiert, exportiert und erneut importiert.
//!
//! - `fremddokumente_round_trip`: Import → Export → Import muss stabil sein
//!   (verlustfreier Round-Trip), Bericht in `target/visutex-fremd/bericht.md`
//!   mit allen Stellen, die visuell noch als Roh-LaTeX erscheinen.
//! - `fremddokumente_kompilieren` (`--ignored`, braucht TeX-Bundle): Original und
//!   Export werden mit der eingebauten Engine kompiliert.
//!
//! Fehlt der private Ordner (z. B. in der CI), laufen nur die synthetischen Beispiele.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use visutex_lib::core::export::{export_document, ExportOptions};
use visutex_lib::core::import::import_latex;
use visutex_lib::core::settings::DocumentSettings;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|e| e == "tex") {
            files.push(path);
        }
    }
}

/// Ordner mit Fremddokumenten: eigene synthetische Beispiele (versioniert) und
/// optional die privaten Vorlagen des Nutzers (nicht versioniert).
fn roots() -> [PathBuf; 2] {
    [
        manifest().join("../tests/fixtures/fremd"),
        manifest().join("../Vorlagen_Test_TEX"),
    ]
}

/// Nur Hauptdokumente (mit `\documentclass`); eingebundene Teildateien werden
/// über ihr Hauptdokument mitgeprüft.
fn documents() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots() {
        collect(&root, &mut files);
    }
    files.retain(|path| {
        std::fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).contains("\\documentclass"))
            .unwrap_or(false)
    });
    files
}

fn name(path: &Path) -> String {
    roots()
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Kurzbezeichnung eines Roh-LaTeX-Blocks: Umgebung bzw. erster Befehl.
fn raw_kind(raw: &str) -> String {
    let trimmed = raw.trim_start();
    if let Some(rest) = trimmed.strip_prefix("\\begin{") {
        return format!("\\begin{{{}}}", rest.split('}').next().unwrap_or(""));
    }
    if trimmed.starts_with('%') {
        return "Kommentar".into();
    }
    if let Some(rest) = trimmed.strip_prefix('\\') {
        let command: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphabetic() || *c == '@')
            .collect();
        if !command.is_empty() {
            return format!("\\{command}");
        }
    }
    let first: String = trimmed.chars().take(30).collect();
    format!("Text „{}“", first.replace('\n', " "))
}

fn walk(node: &Value, out: &mut Vec<(String, String)>) {
    match node["type"].as_str() {
        Some("rawLatexBlock") => {
            let raw = node["attrs"]["rawLatex"].as_str().unwrap_or("");
            out.push((format!("Block {}", raw_kind(raw)), raw.to_string()));
        }
        Some("rawLatexInline") => {
            let raw = node["attrs"]["latex"].as_str().unwrap_or("");
            out.push((format!("Inline {}", raw_kind(raw)), raw.to_string()));
        }
        _ => {}
    }
    if let Some(children) = node["content"].as_array() {
        for child in children {
            walk(child, out);
        }
    }
}

/// Entfernt flüchtige Attribute für den Vergleich zweier Importe.
fn normalize(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(normalize).collect()),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "src" | "missingResources" | "preview" | "previewCode"
                    )
                })
                .map(|(key, item)| (key.clone(), normalize(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn export(doc: &Value, preamble: Option<&str>, settings: &DocumentSettings) -> String {
    export_document(
        doc,
        &ExportOptions {
            settings,
            custom_preamble: preamble,
            addon_preamble: &[],
        },
    )
    .latex
}

#[test]
fn fremddokumente_round_trip() {
    let files = documents();
    if files.is_empty() {
        eprintln!("Vorlagen_Test_TEX fehlt – übersprungen");
        return;
    }
    let mut report = String::from("# Fremddokumente – Import-Bericht\n\n");
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut examples: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut failures = Vec::new();
    for path in &files {
        let source = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                failures.push(format!("{}: nicht lesbar ({error})", name(path)));
                continue;
            }
        };
        let settings = DocumentSettings::default();
        let first = import_latex(&source, &settings);
        let latex = export(&first.doc, first.preamble.as_deref(), &settings);
        let second = import_latex(&latex, &settings);
        let again = export(&second.doc, second.preamble.as_deref(), &settings);

        let mut raws = Vec::new();
        walk(&first.doc, &mut raws);
        let blocks = first.doc["content"].as_array().map_or(0, Vec::len);
        report.push_str(&format!(
            "## {}\n\n{} Blöcke, {} Roh-LaTeX-Stellen\n\n",
            name(path),
            blocks,
            raws.len()
        ));
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for (kind, raw) in &raws {
            *kinds.entry(kind.clone()).or_default() += 1;
            *totals.entry(kind.clone()).or_default() += 1;
            let list = examples.entry(kind.clone()).or_default();
            if list.len() < 6 {
                let snippet: String = raw.chars().take(700).collect();
                list.push(format!("{}:\n```latex\n{snippet}\n```", name(path)));
            }
        }
        for (kind, count) in &kinds {
            report.push_str(&format!("- {kind}: {count}\n"));
        }
        for warning in &first.warnings {
            report.push_str(&format!("- Warnung: {warning}\n"));
        }
        report.push('\n');

        if normalize(&first.doc) != normalize(&second.doc) {
            let left = first.doc["content"].as_array().cloned().unwrap_or_default();
            let right = second.doc["content"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let index = (0..left.len().max(right.len()))
                .find(|i| left.get(*i).map(normalize) != right.get(*i).map(normalize))
                .unwrap_or(0);
            failures.push(format!(
                "{}: zweiter Import weicht ab (Block {index})\n  vorher:  {}\n  nachher: {}",
                name(path),
                left.get(index).map(Value::to_string).unwrap_or_default(),
                right.get(index).map(Value::to_string).unwrap_or_default()
            ));
        } else if latex != again {
            failures.push(format!("{}: Export nicht stabil", name(path)));
        }
        if first.preamble.is_some() && !latex.contains("\\begin{document}") {
            failures.push(format!("{}: Export ohne \\begin{{document}}", name(path)));
        }
        // Unverändert exportiert = Original (bis auf Leerraum zwischen Blöcken).
        let collapse = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        let original = collapse(&source);
        let exported = collapse(&latex);
        if original != exported {
            let position = original
                .chars()
                .zip(exported.chars())
                .take_while(|(a, b)| a == b)
                .count();
            let context = |text: &str| -> String {
                text.chars()
                    .skip(position.saturating_sub(60))
                    .take(160)
                    .collect()
            };
            failures.push(format!(
                "{}: Export weicht vom Original ab (Zeichen {position})\n  Original: {}\n  Export:   {}",
                name(path),
                context(&original),
                context(&exported)
            ));
        }
    }
    report.push_str("## Summe aller Roh-LaTeX-Stellen\n\n");
    let mut sorted: Vec<_> = totals.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for (kind, count) in sorted {
        report.push_str(&format!("- {kind}: {count}\n"));
    }
    let out = manifest().join("target/visutex-fremd");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("bericht.md"), &report).unwrap();
    let mut listing = String::from("# Roh-LaTeX-Beispiele\n\n");
    for (kind, list) in &examples {
        listing.push_str(&format!("## {kind}\n\n{}\n\n", list.join("\n\n")));
    }
    std::fs::write(out.join("beispiele.md"), &listing).unwrap();
    eprintln!("Bericht: {}", out.join("bericht.md").display());
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
#[ignore]
fn fremddokumente_kompilieren() {
    use visutex_lib::compile::{compile, CompileRequest};
    use visutex_lib::texbundle::BundleConfig;
    let files = documents();
    if files.is_empty() {
        return;
    }
    let embedded = manifest().join("resources").join("tex-bundle.zip");
    let bundle = BundleConfig {
        embedded_zip: embedded.is_file().then_some(embedded),
        allow_online: false,
        cache_dir: manifest().join("target").join("visutex-cache"),
        online_cache: None,
    };
    let mut report = String::from("# Fremddokumente – Kompilier-Bericht\n\n");
    let mut failed = 0;
    for path in &files {
        let source = String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned();
        let settings = DocumentSettings::default();
        let imported = import_latex(&source, &settings);
        let exported = export(&imported.doc, imported.preamble.as_deref(), &settings);
        let root = path.parent().map(Path::to_path_buf);
        for (label, latex) in [("Original", &source), ("Export", &exported)] {
            let latex = format!("{}{}", visutex_lib::fonts::hint_prefix(latex), latex);
            let started = std::time::Instant::now();
            let result = compile(
                CompileRequest {
                    latex,
                    project_root: root.clone(),
                    extra_search_paths: Vec::new(),
                },
                &bundle,
                &mut |_| {},
            );
            let line = match result {
                Ok(output) => format!(
                    "ok ({:.1} s, {} Meldungen)",
                    started.elapsed().as_secs_f32(),
                    output.messages.len()
                ),
                Err(failure) => {
                    failed += 1;
                    format!(
                        "FEHLER: {} {:?}",
                        failure.message.replace('\n', " "),
                        failure.missing_files
                    )
                }
            };
            eprintln!("{} [{label}]: {line}", name(path));
            report.push_str(&format!("- {} [{label}]: {line}\n", name(path)));
        }
    }
    let out = manifest().join("target/visutex-fremd");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("kompilieren.md"), &report).unwrap();
    assert_eq!(failed, 0, "siehe target/visutex-fremd/kompilieren.md");
}
