//! Ende-zu-Ende: Volltest-Dokument → LaTeX → Tectonic → PDF → PNG (hayro).
//!
//! Braucht ein TeX-Bundle: entweder `resources/tex-bundle.zip` oder das
//! Tectonic-Standard-Bundle (Internet bzw. lokaler Cache). Deshalb `#[ignore]`:
//! `cargo test --test compile_pipeline -- --ignored --nocapture`

use std::path::PathBuf;
use std::time::Instant;
use visutex_lib::compile::{compile, CompileRequest};
use visutex_lib::core::export::{export_document, ExportOptions};
use visutex_lib::core::settings::DocumentSettings;
use visutex_lib::texbundle::BundleConfig;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bundle() -> BundleConfig {
    let embedded = manifest().join("resources").join("tex-bundle.zip");
    BundleConfig {
        embedded_zip: embedded.is_file().then_some(embedded),
        allow_online: false,
        cache_dir: manifest().join("target").join("visutex-cache"),
        online_cache: None,
    }
}

fn project_dir(name: &str) -> PathBuf {
    let root = manifest().join("target").join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::copy(
        manifest().join("../tests/fixtures/literatur.bib"),
        root.join("literatur.bib"),
    )
    .unwrap();
    root
}

#[test]
#[ignore]
fn full_document_compiles_and_renders() {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(manifest().join("../tests/fixtures/full-document.json")).unwrap(),
    )
    .unwrap();
    let settings = DocumentSettings::from_value(&serde_json::json!({
        "bibliography": { "file": "literatur.bib", "style": "ieee" },
        "headerFooter": { "enabled": true, "footerCenter": "Seite {seite} von {seiten}" }
    }));
    let exported = export_document(
        &fixture,
        &ExportOptions {
            settings: &settings,
            custom_preamble: None,
            addon_preamble: &["\\usepackage{siunitx}".to_string()],
        },
    );
    let root = project_dir("visutex-pipeline");
    visutex_lib::files::write_assets(&root, &exported.assets).unwrap();
    let latex = format!(
        "{}{}",
        visutex_lib::fonts::hint_prefix(&exported.latex),
        exported.latex
    );
    let started = Instant::now();
    let output = compile(
        CompileRequest {
            latex,
            project_root: Some(root),
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |note| eprintln!("  {note}"),
    )
    .unwrap_or_else(|failure| {
        panic!(
            "{}\n{:?}\n{}",
            failure.message,
            failure.messages,
            failure
                .log
                .lines()
                .rev()
                .take(40)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    eprintln!("Kompiliert in {:.1} s", started.elapsed().as_secs_f32());
    for message in &output.messages {
        eprintln!(
            "  {:?} {:?}: {}",
            message.severity, message.line, message.message
        );
    }
    assert!(output
        .messages
        .iter()
        .all(|message| message.severity != "error"));
    let stored = visutex_lib::pdf::load(output.pdf).unwrap();
    assert!(stored.pages.len() >= 8, "Seiten: {}", stored.pages.len());
    let started = Instant::now();
    let png = visutex_lib::pdf::render_page_png(&stored, 0, 1.5).unwrap();
    eprintln!(
        "Seite 1 gerendert in {:.2} s ({} KB)",
        started.elapsed().as_secs_f32(),
        png.len() / 1024
    );
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
#[ignore]
fn all_templates_compile() {
    for template in visutex_lib::templates::list() {
        let project = visutex_lib::templates::build(&template.id).unwrap();
        let exported = export_document(
            &project.document,
            &ExportOptions {
                settings: &project.settings,
                custom_preamble: None,
                addon_preamble: &[],
            },
        );
        let root = project_dir(&format!("visutex-template-{}", template.id));
        std::fs::write(root.join("literatur.bib"), "% leer\n").unwrap();
        let latex = format!(
            "{}{}",
            visutex_lib::fonts::hint_prefix(&exported.latex),
            exported.latex
        );
        let started = Instant::now();
        match compile(
            CompileRequest {
                latex,
                project_root: Some(root),
                extra_search_paths: Vec::new(),
            },
            &bundle(),
            &mut |_| {},
        ) {
            Ok(output) => eprintln!(
                "{}: ok in {:.1} s, {} Meldungen",
                template.id,
                started.elapsed().as_secs_f32(),
                output.messages.len()
            ),
            Err(failure) => panic!(
                "{}: {}\n{:?}",
                template.id, failure.message, failure.messages
            ),
        }
    }
}

/// Wie `render_tikz` in der App: Standalone-Dokument ohne fontspec (Computer Modern
/// in mehreren Größen, z. B. Indizes in Schaltplan-Beschriftungen).
#[test]
#[ignore]
fn tikz_preview_compiles_and_renders() {
    let latex = "\\documentclass[border=4pt]{standalone}\n\\usepackage{amsmath,amssymb}\n\\usepackage{tikz}\n\\usetikzlibrary{arrows.meta,positioning,calc,shapes.geometric}\n\\usepackage[european]{circuitikz}\n\\begin{document}\n\\begin{circuitikz}\n\\draw (0,0) to[V, v=$U_0$] (0,3) to[R, l=$R_1$] (3,3) to[C, l=$C_{2}$, v=$u_a$] (3,0) -- (0,0);\n\\end{circuitikz}\n\\end{document}\n";
    let output = compile(
        CompileRequest {
            latex: latex.into(),
            project_root: None,
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |_| {},
    )
    .unwrap_or_else(|failure| panic!("{}\n{}", failure.message, failure.log));
    let stored = visutex_lib::pdf::load(output.pdf).unwrap();
    let png = visutex_lib::pdf::render_page_png(&stored, 0, 2.5).unwrap();
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
#[ignore]
fn reports_errors_with_line_numbers() {
    // Wie TeXstudio/Overleaf: PDF trotz Fehler, Fehler mit Zeilennummer gemeldet.
    let latex =
        "\\documentclass{article}\n\\begin{document}\nText\n\\undefinedmacro\n\\end{document}\n";
    let output = compile(
        CompileRequest {
            latex: latex.into(),
            project_root: None,
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |_| {},
    )
    .unwrap_or_else(|failure| panic!("PDF erwartet: {}", failure.message));
    assert!(output
        .messages
        .iter()
        .any(|message| message.severity == "error" && message.line == Some(4)));
    // Fehlende Pakete werden gemeldet (die App bietet das Nachladen an).
    let missing = match compile(
        CompileRequest {
            latex: "\\documentclass{article}\n\\usepackage{gibtesnichtvisutex}\n\\begin{document}x\\end{document}\n".into(),
            project_root: None,
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |_| {},
    ) {
        Ok(output) => output.missing_files,
        Err(failure) => failure.missing_files,
    };
    assert_eq!(missing, vec!["gibtesnichtvisutex.sty".to_string()]);
}

#[test]
#[ignore]
fn slides_compile_and_render() {
    // Folien-Editor: Beamer-Export mit Text, Listen, Formen, Pfeil, Formel, PNG und SVG.
    let source = std::fs::read_to_string(manifest().join("../tests/fixtures/slides.json")).unwrap();
    let deck = visutex_lib::core::slides::parse_deck(&source).unwrap();
    let exported = visutex_lib::core::slides::deck_to_latex(&deck);
    assert!(exported.warnings.is_empty(), "{:?}", exported.warnings);
    assert_eq!(exported.assets.len(), 2);
    let root = project_dir("visutex-slides");
    visutex_lib::files::write_assets(&root, &exported.assets).unwrap();
    std::fs::write(root.join("folien.tex"), &exported.latex).unwrap();
    let started = Instant::now();
    let output = compile(
        CompileRequest {
            latex: exported.latex.clone(),
            project_root: Some(root.clone()),
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |_| {},
    )
    .unwrap_or_else(|failure| {
        panic!(
            "{}\n{:?}\n{}",
            failure.message, failure.missing_files, failure.log
        )
    });
    eprintln!(
        "Folien kompiliert in {:.1} s",
        started.elapsed().as_secs_f32()
    );
    let errors: Vec<_> = output
        .messages
        .iter()
        .filter(|message| message.severity == "error")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        output.missing_files.is_empty(),
        "{:?}",
        output.missing_files
    );
    let stored = visutex_lib::pdf::load(output.pdf).unwrap();
    assert_eq!(stored.pages.len(), 3);
    for page in 0..3 {
        let png = visutex_lib::pdf::render_page_png(&stored, page, 2.0).unwrap();
        std::fs::write(root.join(format!("folie-{}.png", page + 1)), png).unwrap();
    }
    eprintln!("Vorschau: {}", root.display());
}

#[test]
#[ignore]
fn all_document_languages_compile() {
    // Jede Dokumentsprache (babel, Silbentrennung, csquotes, siunitx) mit dem Bundle.
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(manifest().join("../tests/fixtures/full-document.json")).unwrap(),
    )
    .unwrap();
    let root = project_dir("visutex-sprachen");
    let mut failures = Vec::new();
    for language in visutex_lib::core::languages::all() {
        let settings = DocumentSettings::from_value(&serde_json::json!({
            "documentClass": "article",
            "language": language.id,
            "bibliography": { "file": "literatur.bib", "style": "ieee" }
        }));
        let exported = export_document(
            &fixture,
            &ExportOptions {
                settings: &settings,
                custom_preamble: None,
                addon_preamble: &["\\usepackage{siunitx}".to_string()],
            },
        );
        visutex_lib::files::write_assets(&root, &exported.assets).unwrap();
        let result = compile(
            CompileRequest {
                latex: exported.latex,
                project_root: Some(root.clone()),
                extra_search_paths: Vec::new(),
            },
            &bundle(),
            &mut |_| {},
        );
        let problem = match result {
            Ok(output) => output
                .messages
                .iter()
                .find(|message| message.severity == "error")
                .map(|message| message.message.clone())
                .or_else(|| {
                    (!output.missing_files.is_empty())
                        .then(|| format!("fehlt: {:?}", output.missing_files))
                }),
            Err(failure) => Some(failure.message),
        };
        eprintln!("{}: {}", language.id, problem.as_deref().unwrap_or("ok"));
        if let Some(problem) = problem {
            failures.push(format!("{}: {problem}", language.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// PDF/A-Dokumente (`\DocumentMetadata`, LaTeX ab 2022) kompilieren auch mit der
/// eingebauten Engine: die Zeile wird zeilengenau ausgeblendet (mit Hinweis).
#[test]
#[ignore]
fn pdfa_document_metadata_compiles() {
    let latex =
        std::fs::read_to_string(manifest().join("../tests/fixtures/pdfa.tex")).unwrap();
    let mut notes = Vec::new();
    let output = compile(
        CompileRequest {
            latex,
            project_root: Some(project_dir("visutex-pdfa")),
            extra_search_paths: Vec::new(),
        },
        &bundle(),
        &mut |note| notes.push(note.to_string()),
    )
    .unwrap_or_else(|failure| panic!("{}\n{:?}", failure.message, failure.messages));
    let errors: Vec<_> = output
        .messages
        .iter()
        .filter(|message| message.severity == "error")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(output.pdf.starts_with(b"%PDF"));
}
