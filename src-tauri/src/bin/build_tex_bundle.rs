//! Erzeugt das mitgelieferte TeX-Bundle `resources/tex-bundle.zip`.
//!
//! Aufruf (Projektordner `src-tauri`):
//! ```text
//! cargo run --features dev-tools --bin build-tex-bundle -- [Ziel.zip]
//! ```
//!
//! Vorgehen: Eine Reihe von Abdeckungsdokumenten (alle VisuTeX-Funktionen,
//! alle Vorlagen, alle Literaturstile und gängige Zusatzpakete) wird mit dem
//! Tectonic-Standard-Bundle kompiliert – inklusive frischer Formaterzeugung.
//! Ein aufzeichnender Bundle-Wrapper merkt sich jede gelesene Datei. Danach
//! werden verwandte Dateien ergänzt (gleicher Paketname, TikZ-/PGF-Bibliotheken,
//! Latin-Modern-Schnitte) und alles als flaches ZIP mit `SHA256SUM` geschrieben.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tectonic_bundles::Bundle;
use tectonic_errors::Result as TexResult;
use tectonic_io_base::{digest::DigestData, InputHandle, IoProvider, OpenResult};
use tectonic_status_base::{MessageKind, NoopStatusBackend, StatusBackend};
use visutex_lib::core::export::{export_document, ExportOptions};
use visutex_lib::core::settings::DocumentSettings;

struct Recording {
    inner: Box<dyn Bundle>,
    names: Arc<Mutex<BTreeSet<String>>>,
}

impl IoProvider for Recording {
    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        let result = self.inner.input_open_name(name, status);
        if let OpenResult::Ok(_) = &result {
            self.names.lock().unwrap().insert(name.to_string());
        }
        result
    }
}

impl Bundle for Recording {
    fn get_digest(&mut self) -> TexResult<DigestData> {
        self.inner.get_digest()
    }
    fn all_files(&self) -> Vec<String> {
        self.inner.all_files()
    }
}

struct Printer;
impl StatusBackend for Printer {
    fn report(
        &mut self,
        kind: MessageKind,
        args: std::fmt::Arguments,
        error: Option<&tectonic_errors::Error>,
    ) {
        if kind != MessageKind::Note {
            match error {
                Some(error) => eprintln!("  {kind:?}: {args} ({error:#})"),
                None => eprintln!("  {kind:?}: {args}"),
            }
        }
    }
    fn dump_error_logs(&mut self, output: &[u8]) {
        let text = String::from_utf8_lossy(output);
        for line in text.lines().filter(|line| line.starts_with('!')).take(5) {
            eprintln!("  {line}");
        }
    }
}

/// Dateiliste aus dem Index im Tectonic-Cache (`<digest>.index`: „Name Offset Länge“ je Zeile).
fn index_from_cache() -> Vec<String> {
    let candidates: Vec<PathBuf> = [
        std::env::var_os("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join("TectonicProject/Tectonic/cache")),
        std::env::var_os("XDG_CACHE_HOME").map(|base| PathBuf::from(base).join("Tectonic")),
        std::env::var_os("HOME").map(|base| PathBuf::from(base).join(".cache/Tectonic")),
        std::env::var_os("HOME").map(|base| PathBuf::from(base).join("Library/Caches/Tectonic")),
    ]
    .into_iter()
    .flatten()
    .map(|base| base.join("bundles").join("data"))
    .collect();
    for dir in candidates {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut indexes: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "index")
            })
            .collect();
        indexes.sort_by_key(|path| {
            std::cmp::Reverse(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))
        });
        if let Some(index) = indexes.first() {
            if let Ok(text) = std::fs::read_to_string(index) {
                eprintln!("Dateiliste aus {}", index.display());
                return text
                    .lines()
                    .filter_map(|line| line.split_whitespace().next())
                    .filter(|name| !matches!(*name, "SVNREV" | "GITHASH"))
                    .map(str::to_string)
                    .collect();
            }
        }
    }
    Vec::new()
}

fn default_bundle() -> Box<dyn Bundle> {
    let config = tectonic::config::PersistentConfig::open(false).expect("Tectonic-Konfiguration");
    config
        .default_bundle(false)
        .expect("Tectonic-Standard-Bundle (Internet oder Cache nötig)")
}

/// Pakete, die über die VisuTeX-Funktionen hinaus häufig in Abschlussarbeiten verwendet werden.
const EXTRA_PACKAGES: &[&str] = &[
    "booktabs",
    "longtable",
    "listings",
    "enumitem",
    "subcaption",
    "mathtools",
    "pgfplots",
    "xurl",
    "siunitx",
    "csquotes",
    "microtype",
    "lipsum",
    "blindtext",
    "wrapfig",
    "makecell",
    "threeparttable",
    "colortbl",
    "hhline",
    "rotating",
    "tcolorbox",
    "fontawesome5",
    "nicefrac",
    "cancel",
    "bm",
    "physics",
    "chemformula",
    "mhchem",
    "algorithm2e",
    "algorithmicx",
    "algpseudocode",
    "minted",
    "glossaries",
    "nomencl",
    "todonotes",
    "pdfpages",
    "standalone",
    "tabularray",
    "adjustbox",
    "titlesec",
    "tocloft",
    "appendix",
    "setspace",
    "lastpage",
    "fancyhdr",
    "scrlayer-scrpage",
    "eso-pic",
    "background",
    "datetime2",
    "upgreek",
    "esint",
    "dsfont",
    "mathrsfs",
    "stmaryrd",
    "textcomp",
    "gensymb",
    "url",
    "varioref",
    "cleveref",
    "nameref",
    "zref",
    "etoolbox",
    "xparse",
    "ifthen",
    "calc",
    "xspace",
    "footmisc",
    "footnotebackref",
    "marginnote",
    "sidecap",
    "float",
    "placeins",
    "afterpage",
    "multirow",
    "tabularx",
    "ltablex",
    "dcolumn",
    "array",
    "caption",
    "tikz-cd",
    "circuitikz",
    "pgfgantt",
    "smartdiagram",
    "forest",
    "qrcode",
    "biblatex",
    // Klassische (pdfLaTeX-)Präambeln fremder Dokumente
    "inputenc",
    "fontenc",
    "lmodern",
    "amsthm",
    "amsfonts",
    "graphicx",
    "xcolor",
    "geometry",
    "hyperref",
    "ragged2e",
    "multicol",
    "parskip",
    "eurosym",
    "acronym",
    "natbib",
    "subfig",
    "pdflscape",
    "lscape",
    "textpos",
    "framed",
    "mdframed",
    "tabto",
    "fancyvrb",
    "verbatim",
    "comment",
    "soul",
    "ulem",
    "contour",
    "pifont",
    "marvosym",
    "wasysym",
    "units",
    "nicematrix",
    "empheq",
    "cases",
    "steinmetz",
    "chngcntr",
    "titling",
    "abstract",
    "epigraph",
    "lettrine",
    "enotez",
    "endnotes",
    "imakeidx",
    "makeidx",
    "longtable",
    "supertabular",
    "tabu",
    "arydshln",
    "diagbox",
    "pgf-pie",
    "bytefield",
    "karnaugh-map",
    "tikz-timing",
    "tikz-3dplot",
    "pst-node",
    "xstring",
    "fp",
    "pgfmath",
    "ifpdf",
    "iftex",
    "ifxetex",
    "ifluatex",
    "babel",
    // Schriften
    "helvet",
    "tgheros",
    "tgtermes",
    "tgpagella",
    "times",
    "mathptmx",
    "newtxtext",
    "newtxmath",
    "mathpazo",
    "palatino",
    "charter",
    "kpfonts",
    "fourier",
    "libertine",
    "sourcesanspro",
    "roboto",
    "opensans",
    "cmbright",
    "arev",
];

/// Klassen, die fremde Dokumente häufig verwenden.
const EXTRA_CLASSES: &[&str] = &[
    "article",
    "report",
    "book",
    "scrartcl",
    "scrreprt",
    "scrbook",
    "extarticle",
    "extreport",
    "extbook",
    "memoir",
    "letter",
    "scrlttr2",
    "beamer",
    "standalone",
];

const RELATED_PREFIXES: &[&str] = &[
    "tikzlibrary",
    "pgflibrary",
    "pgfplots",
    "lmroman",
    "lmsans",
    "lmmono",
    "latinmodern-math",
    "circuitikz",
    "siunitx",
    "babel-german",
    "ngerman",
    "naustrian",
    "german",
    "english",
    "hyph-de",
    "hyph-en",
    "loadhyph-de",
    "loadhyph-en",
    "scr",
    "koma",
    "tocbasic",
    "typearea",
    "IEEEtran",
    "plainnat",
    "abbrvnat",
    "unsrtnat",
    "alpha",
    "natbib",
    "fontawesome5",
    "FontAwesome5",
    "pgfplotslibrary",
    "tcb",
    "tcolorbox",
    "listings",
    "lst",
    // extsizes (extarticle 8pt, 14pt …) und Größenoptionen der Standardklassen
    "ext",
    "size",
    "bk",
    // Latin Modern als Type1 (T1/TS1/OT1, Mathematik) – Ersatz für EC-Schriften
    "lmodern",
    "ec-lm",
    "ts1-lm",
    "rm-lm",
    "lm-",
    "lmmi",
    "lmsy",
    "lmex",
    "lmbsy",
    "t1lm",
    "ot1lm",
    "ts1lm",
    "omllm",
    "omslm",
    "omxlm",
    "ly1lm",
    // Babel-Sprachen
    "babel.",
    "austrian",
    "ngermanb",
    "germanb",
    "english.ldf",
    "british",
    "american",
    "UKenglish",
    "USenglish",
    // inputenc/fontenc-Kodierungen
    "utf8",
    "t1enc",
    "ts1enc",
    "ot1enc",
    "omlenc",
    "omsenc",
    "t1cmr",
    "ts1cmr",
    // Schriften der Schriftpakete (URW/TeX Gyre)
    "uhv",
    "qhv",
    "utm",
    "qtm",
    "upl",
    "qpl",
    "t1uhv",
    "t1qhv",
    "ts1uhv",
    "ts1qhv",
    "t1ptm",
    "t1ppl",
    "ot1uhv",
    "ot1ptm",
    "texgyre",
    "tgheros",
    "tgtermes",
    "tgpagella",
    "nimbus",
];

/// Type1-Schnitte von Latin Modern (`lmr10.pfb`, `lmbx12.pfb`, …).
fn is_latin_modern_type1(file: &str) -> bool {
    file.starts_with("lm") && file.ends_with(".pfb")
}

/// Computer-Modern- und AMS-Schriften (Metriken und Type1, alle Größen und
/// Schnitte, auch fett): werden von Formeln, CircuiTikZ-Symbolen und Beamer
/// je nach Größe benötigt.
fn is_cm_or_ams_font(file: &str) -> bool {
    let Some((stem, extension)) = file.rsplit_once('.') else {
        return false;
    };
    if !matches!(extension, "tfm" | "pfb") {
        return false;
    }
    let family: String = stem
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let size = &stem[family.len()..];
    !size.is_empty()
        && size.chars().all(|c| c.is_ascii_digit())
        && matches!(
            family.as_str(),
            "cmr"
                | "cmbx"
                | "cmb"
                | "cmti"
                | "cmbxti"
                | "cmsl"
                | "cmbxsl"
                | "cmtt"
                | "cmitt"
                | "cmsltt"
                | "cmss"
                | "cmssi"
                | "cmssbx"
                | "cmssdc"
                | "cmssq"
                | "cmssqi"
                | "cmcsc"
                | "cmmi"
                | "cmmib"
                | "cmsy"
                | "cmbsy"
                | "cmex"
                | "cmu"
                | "cmfib"
                | "cmdunh"
                | "cmvtt"
                | "msam"
                | "msbm"
                | "eufm"
                | "eufb"
                | "eurm"
                | "eurb"
                | "eusm"
                | "eusb"
                | "rsfs"
        )
}

/// Alle `.tex`-Dateien unterhalb eines Ordners (für die Testdokumente des Nutzers).
fn tex_files(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            tex_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "tex") {
            files.push(path);
        }
    }
}

fn coverage_documents() -> Vec<(String, String)> {
    let mut documents = Vec::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(manifest.join("../tests/fixtures/full-document.json"))
            .expect("Volltest-Dokument"),
    )
    .expect("gültiges JSON");

    // Volltest-Dokument in allen Varianten der Einstellungen.
    let variants = [
        serde_json::json!({ "bibliography": { "file": "literatur.bib", "style": "ieee" }, "headerFooter": { "enabled": true, "footerCenter": "Seite {seite} von {seiten}" } }),
        serde_json::json!({ "documentClass": "article", "language": "english", "bibliography": { "file": "literatur.bib", "style": "plainnat" } }),
        serde_json::json!({ "documentClass": "article", "language": "naustrian", "defaultFontSize": 11, "bibliography": { "file": "literatur.bib", "style": "abbrvnat" } }),
        serde_json::json!({ "paperFormat": "a5", "defaultFontSize": 9, "bibliography": { "file": "literatur.bib", "style": "plainnat-authoryear" }, "lineSpacing": 1.5, "paragraphStyle": "skip" }),
        serde_json::json!({ "paperFormat": "letter", "twoside": true, "bindingOffset": 8, "bibliography": { "file": "literatur.bib", "style": "alpha" }, "pagePdfColor": "#202020" }),
    ];
    let addon = vec!["\\usepackage{siunitx}".to_string()];
    for (index, variant) in variants.iter().enumerate() {
        let settings = DocumentSettings::from_value(variant);
        let exported = export_document(
            &fixture,
            &ExportOptions {
                settings: &settings,
                custom_preamble: None,
                addon_preamble: &addon,
            },
        );
        documents.push((format!("volltest-{index}"), exported.latex));
    }
    for template in visutex_lib::templates::list() {
        let project = visutex_lib::templates::build(&template.id).expect("Vorlage");
        let exported = export_document(
            &project.document,
            &ExportOptions {
                settings: &project.settings,
                custom_preamble: None,
                addon_preamble: &[],
            },
        );
        documents.push((format!("vorlage-{}", template.id), exported.latex));
    }
    // Computer Modern in allen Größen (Dokumente ohne fontspec, Formeln mit Indizes)
    documents.push((
        "cm-groessen".into(),
        "\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n\\begin{document}\n{\\tiny a $x_1^{2_3}$}{\\scriptsize a $x_1^2$}{\\footnotesize a $x_1^2$}{\\small a $x_1^2$} a $x_1^{2_3} \\mathbb{R} \\mathfrak{g} \\sum_i \\int_0^1$ \\textbf{b \\textit{bi}} \\textit{i} \\texttt{t} \\textsf{s} \\textsc{c} \\textsl{sl}{\\large a $x_1^2$}{\\Large a $x_1$}{\\LARGE a $x$}{\\huge a $x$}{\\Huge a $x$}\n\\end{document}\n".into(),
    ));
    // TikZ-Vorschau (standalone)
    documents.push((
        "tikz".into(),
        "\\documentclass[border=4pt]{standalone}\n\\usepackage{amsmath,amssymb}\n\\usepackage{tikz}\n\\usetikzlibrary{arrows.meta,positioning,calc,shapes.geometric}\n\\usepackage[european]{circuitikz}\n\\begin{document}\n\\begin{circuitikz}\\draw (0,0) to[R=$R$] (2,0);\\end{circuitikz}\n\\end{document}\n".into(),
    ));
    // Alle Symbole des Skizzier-Werkzeugs (CircuiTikZ/TikZ) – in Gruppen, damit ein
    // Fehler nicht die übrigen Symbole verdeckt.
    let entries: Vec<_> = visutex_lib::core::sketch_catalog::ENTRIES.iter().collect();
    for (index, chunk) in entries.chunks(20).enumerate() {
        let (latex, _) = visutex_lib::core::sketch_catalog::symbols_document(chunk);
        documents.push((format!("skizze-{index}"), latex));
    }
    // Folien-Editor (Beamer)
    let deck = visutex_lib::core::slides::parse_deck(
        &std::fs::read_to_string(manifest.join("../tests/fixtures/slides.json"))
            .expect("Beispiel-Präsentation"),
    )
    .expect("gültige Präsentation");
    documents.push((
        "folien".into(),
        visutex_lib::core::slides::deck_to_latex(&deck).latex,
    ));
    // Alle Dokumentsprachen (babel, Silbentrennung, csquotes, siunitx)
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
                addon_preamble: &addon,
            },
        );
        documents.push((format!("sprache-{}", language.id), exported.latex));
    }
    // Fette Mathematik und AMS-Symbole in allen Größen (fontspec und klassisch)
    let math = "\\boldmath $x_1^{2} \\alpha \\sum_i \\int_0^1 \\mathbb{R} \\mathfrak{g} \\blacktriangleright \\checkmark \\lozenge \\square$\\unboldmath\\ $\\mathbf{v} \\mathbb{N} \\mathcal{L} \\mathscr{A}$ \\textbf{\\boldmath$\\Omega$}";
    let sizes = [
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
    let all_sizes: String = sizes
        .iter()
        .map(|size| format!("{{\\{size} {math}}}\\par\n"))
        .collect();
    for (name, fonts) in [
        ("fontspec", "\\usepackage{fontspec}"),
        (
            "klassisch",
            "\\usepackage[utf8]{inputenc}\n\\usepackage[T1]{fontenc}",
        ),
        (
            "lmodern",
            "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
        ),
    ] {
        let latex = format!("\\documentclass{{article}}\n{fonts}\n\\usepackage{{amsmath,amssymb,mathrsfs,bm}}\n\\begin{{document}}\n{all_sizes}\\end{{document}}\n");
        documents.push((
            format!("mathematik-{name}"),
            visutex_lib::prepare::prepare_for_engine(&latex, None).latex,
        ));
    }
    // Zusatzpakete einzeln (ein fehlendes Paket soll die anderen nicht verhindern).
    for package in EXTRA_PACKAGES {
        let body = match *package {
            "pgfplots" => {
                "\\begin{tikzpicture}\\begin{axis}\\addplot {x^2};\\end{axis}\\end{tikzpicture}"
            }
            "lipsum" => "\\lipsum[1]",
            "blindtext" => "\\blindtext",
            "fontawesome5" => "\\faBook",
            "listings" => "\\begin{lstlisting}[language=Python]\nprint(1)\n\\end{lstlisting}",
            "biblatex" => "Test",
            _ => "Test",
        };
        let preamble = match *package {
            "pgfplots" => "\\usepackage{pgfplots}\\pgfplotsset{compat=1.18}".to_string(),
            "minted" => continue, // braucht Shell-Escape
            other => format!("\\usepackage{{{other}}}"),
        };
        documents.push((
            format!("paket-{package}"),
            format!("\\documentclass{{scrreprt}}\n\\usepackage{{fontspec}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"),
        ));
        // Klassische pdfLaTeX-Präambel (inputenc, T1 → Latin Modern, babel)
        let body = match *package {
            "tcolorbox" => "\\begin{tcolorbox}[title=Titel]Test $x^2$\\end{tcolorbox}",
            "siunitx" => "\\SI{1}{\\mega\\ohm} \\qty{2}{\\kilo\\volt} \\num{1e3}",
            "acronym" => "\\begin{acronym}\\acro{A}{Abk}\\end{acronym}\\ac{A}",
            _ => body,
        };
        let preamble = if *package == "tcolorbox" {
            "\\usepackage[most]{tcolorbox}".to_string()
        } else {
            preamble
        };
        let classic = format!(
            "\\documentclass[11pt,a4paper]{{article}}\n\\usepackage[utf8]{{inputenc}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[ngerman]{{babel}}\n\\usepackage{{amsmath,amssymb}}\n{preamble}\n\\begin{{document}}\nÄÖÜ äöü ß \\textbf{{fett}} \\textit{{kursiv}} {body}\n\\end{{document}}\n"
        );
        documents.push((
            format!("klassisch-{package}"),
            visutex_lib::prepare::prepare_for_engine(&classic, None).latex,
        ));
    }
    // Klassen in mehreren Grundgrößen mit allen Schriftgrößen und -schnitten
    let sample = "\\section{Abschnitt}Text ÄÖÜß \\textbf{fett \\textit{fk}} \\textit{kursiv} \\textsl{schräg} \\textsc{Kapitälchen} \\texttt{mono \\textbf{mb}} \\textsf{serifenlos \\textbf{sb} \\textit{si}} $x_1^2 \\alpha \\int \\sum$ {\\tiny t}{\\scriptsize s}{\\footnotesize f}{\\small k}{\\large g}{\\Large G}{\\LARGE GG}{\\huge h}{\\Huge H}";
    for class in EXTRA_CLASSES {
        let sizes: &[&str] = match *class {
            "extarticle" | "extreport" | "extbook" => {
                &["8pt", "9pt", "10pt", "14pt", "17pt", "20pt"]
            }
            "beamer" | "standalone" | "letter" | "scrlttr2" => &["11pt"],
            _ => &["10pt", "11pt", "12pt"],
        };
        for size in sizes {
            let body = match *class {
                "beamer" => "\\begin{frame}{Folie}Text $x^2$\\end{frame}".to_string(),
                "letter" | "scrlttr2" | "standalone" => "Text ÄÖÜ $x^2$".to_string(),
                _ => sample.to_string(),
            };
            for (variant, fonts) in [
                (
                    "klassisch",
                    "\\usepackage[utf8]{inputenc}\n\\usepackage[T1]{fontenc}",
                ),
                (
                    "lmodern",
                    "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
                ),
                ("ot1", ""),
            ] {
                let latex = format!(
                    "\\documentclass[{size},a4paper]{{{class}}}\n{fonts}\n\\usepackage[ngerman]{{babel}}\n\\usepackage{{amsmath,amssymb}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
                );
                documents.push((
                    format!("klasse-{class}-{size}-{variant}"),
                    visutex_lib::prepare::prepare_for_engine(&latex, None).latex,
                ));
            }
        }
    }
    // Schriftpakete mit T1 und Sans-Standard (z. B. Helvetica-Ersatz als Grundschrift)
    for font in [
        "helvet",
        "tgheros",
        "times",
        "mathptmx",
        "newtxtext,newtxmath",
        "mathpazo",
        "tgpagella",
        "tgtermes",
        "charter",
        "kpfonts",
        "fourier",
    ] {
        documents.push((
            format!("schrift-{font}"),
            format!("\\documentclass[12pt]{{article}}\n\\usepackage[utf8]{{inputenc}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{{font}}}\n\\renewcommand{{\\familydefault}}{{\\sfdefault}}\n\\begin{{document}}\n{sample}\n\\normalfont\\rmfamily {sample}\n\\end{{document}}\n"),
        ));
    }
    documents
}

/// Testdokumente des Nutzers (`Vorlagen_Test_TEX`), vorbereitet wie in der App
/// (biblatex → BibTeX, fehlende Bilder → Platzhalter, T1 → Latin Modern).
fn user_documents() -> Vec<(String, String, PathBuf)> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    tex_files(&manifest.join("../Vorlagen_Test_TEX"), &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let root = path.parent()?.to_path_buf();
            let latex = visutex_lib::prepare::prepare_for_engine(&text, Some(&root)).latex;
            let name = path.file_stem()?.to_string_lossy().replace(' ', "-");
            Some((format!("nutzer-{name}"), latex, root))
        })
        .collect()
}

fn compile(
    name: &str,
    latex: &str,
    bundle: Box<dyn Bundle>,
    format_cache: &PathBuf,
    root: &PathBuf,
) -> bool {
    let mut status = Printer;
    let mut builder = tectonic::driver::ProcessingSessionBuilder::default();
    builder
        .bundle(bundle)
        .primary_input_buffer(latex.as_bytes())
        .tex_input_name(&format!("{name}.tex"))
        .format_name("latex")
        .format_cache_path(format_cache)
        .filesystem_root(root)
        .keep_logs(false)
        .keep_intermediates(false)
        .print_stdout(false)
        .output_format(tectonic::driver::OutputFormat::Pdf)
        .do_not_write_output_files();
    match builder.create(&mut status) {
        Ok(mut session) => session.run(&mut status).is_ok(),
        Err(error) => {
            eprintln!("  Start fehlgeschlagen: {error:#}");
            false
        }
    }
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("resources").join("tex-bundle.zip"));
    let work = std::env::temp_dir().join("visutex-bundle-build");
    let _ = std::fs::remove_dir_all(&work);
    let format_cache = work.join("formate");
    let root = work.join("projekt");
    std::fs::create_dir_all(&format_cache).unwrap();
    std::fs::create_dir_all(root.join("Abbildungen")).unwrap();
    std::fs::copy(
        manifest.join("../tests/fixtures/literatur.bib"),
        root.join("literatur.bib"),
    )
    .unwrap();
    // Eingebettetes Testbild des Volltest-Dokuments
    let fixture_text =
        std::fs::read_to_string(manifest.join("../tests/fixtures/full-document.json")).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(&fixture_text).unwrap();
    let exported = export_document(
        &fixture,
        &ExportOptions {
            settings: &DocumentSettings::default(),
            custom_preamble: None,
            addon_preamble: &[],
        },
    );
    visutex_lib::files::write_assets(&root, &exported.assets).unwrap();

    let names = Arc::new(Mutex::new(BTreeSet::new()));
    let mut documents: Vec<(String, String, PathBuf)> = coverage_documents()
        .into_iter()
        .map(|(name, latex)| (name, latex, root.clone()))
        .collect();
    documents.extend(user_documents());
    let mut failed = Vec::new();
    for (index, (name, latex, document_root)) in documents.iter().enumerate() {
        eprintln!("[{}/{}] {name}", index + 1, documents.len());
        let bundle = Box::new(Recording {
            inner: default_bundle(),
            names: names.clone(),
        });
        if !compile(name, latex, bundle, &format_cache, document_root) {
            failed.push(name.clone());
        }
    }

    // Verwandte Dateien ergänzen.
    let mut source = default_bundle();
    // Der Cache lädt den Dateiindex erst beim ersten Zugriff.
    let _ = source.input_open_name("tectonic-format-latex.tex", &mut NoopStatusBackend {});
    let _ = source.get_digest();
    let mut all_files = source.all_files();
    if all_files.is_empty() {
        all_files = index_from_cache();
    }
    let mut selected: BTreeSet<String> = names.lock().unwrap().clone();
    let stems: BTreeSet<String> = selected
        .iter()
        .filter(|name| name.ends_with(".sty") || name.ends_with(".cls"))
        .map(|name| {
            name.rsplit_once('.')
                .map(|(stem, _)| stem.to_string())
                .unwrap_or_default()
        })
        .filter(|stem| stem.len() >= 4)
        .collect();
    eprintln!(
        "Standard-Bundle: {} Dateien, Beispiele: {:?}",
        all_files.len(),
        all_files.iter().take(5).collect::<Vec<_>>()
    );
    let recorded = selected.len();
    for full in &all_files {
        // Manche Bundles liefern Pfade – im ZIP stehen nur Basisnamen.
        let file = full.rsplit(['/', '\\']).next().unwrap_or(full);
        let related = RELATED_PREFIXES
            .iter()
            .any(|prefix| file.starts_with(prefix))
            || is_latin_modern_type1(file)
            || is_cm_or_ams_font(file)
            || stems.iter().any(|stem| {
                file.starts_with(&format!("{stem}-")) || file.starts_with(&format!("{stem}."))
            });
        let unwanted = file.ends_with(".pdf") || file.ends_with(".html") || file.ends_with(".dtx");
        if related && !unwanted {
            selected.insert(file.to_string());
        }
    }
    eprintln!(
        "Aufgezeichnet: {recorded}, nach Ergänzung: {}",
        selected.len()
    );
    selected.insert("tectonic-format-latex.tex".into());
    selected.remove("SHA256SUM");

    // ZIP schreiben.
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let temporary = target.with_extension("zip.tmp");
    let file = std::fs::File::create(&temporary).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(false);
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut missing = 0;
    for name in &selected {
        let mut data = Vec::new();
        match source.input_open_name(name, &mut NoopStatusBackend {}) {
            OpenResult::Ok(mut handle) => {
                handle.read_to_end(&mut data).unwrap();
            }
            _ => {
                missing += 1;
                continue;
            }
        }
        hasher.update(name.as_bytes());
        hasher.update([0]);
        hasher.update(&data);
        total += data.len() as u64;
        zip.start_file(name, options).unwrap();
        zip.write_all(&data).unwrap();
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    zip.start_file("SHA256SUM", options).unwrap();
    zip.write_all(digest.as_bytes()).unwrap();
    zip.finish().unwrap();
    std::fs::rename(&temporary, &target).unwrap();
    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "\nTeX-Bundle geschrieben: {} ({} Dateien, {:.1} MB unkomprimiert, {:.1} MB ZIP, {} nicht lesbar)",
        target.display(),
        selected.len() - missing,
        total as f64 / 1_048_576.0,
        size as f64 / 1_048_576.0,
        missing
    );
    if !failed.is_empty() {
        eprintln!(
            "Nicht kompiliert (Pakete evtl. nicht im Standard-Bundle): {}",
            failed.join(", ")
        );
    }
    let _ = std::fs::remove_dir_all(&work);
}
