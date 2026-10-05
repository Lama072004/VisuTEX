//! Erzeugt die LaTeX-Präambel aus den Dokumenteinstellungen.
//!
//! Die Präambel ist engine-neutral (iftex): Dasselbe Dokument kompiliert mit
//! der eingebauten Tectonic-Engine (XeTeX), XeLaTeX, LuaLaTeX und pdfLaTeX in
//! TeX Live, MiKTeX oder Overleaf.

use super::escape::escape_latex;
use super::settings::{DocumentSettings, DEFAULT_FONT};
use std::collections::BTreeSet;

/// Welche Pakete/Funktionen der Dokumentkörper benötigt (vom Export gesammelt).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub tikz: bool,
    pub circuitikz: bool,
    pub multirow: bool,
    pub multicol: bool,
    pub landscape_sections: bool,
    pub float_here: bool,
    pub acronyms: bool,
    pub citations: bool,
    pub bibliography: bool,
    pub font_helper: bool,
    pub list_of_figures: bool,
    pub list_of_tables: bool,
    /// \citep/\citet (natbib)
    pub natbib: bool,
    /// Eigenes Literaturverzeichnis im Roh-LaTeX (\bibliography, thebibliography, biblatex)
    pub manual_bibliography: bool,
    pub booktabs: bool,
    pub siunitx: bool,
    pub enumitem: bool,
    pub listings: bool,
    pub tcolorbox: bool,
    pub maketitle: bool,
    pub tabularx: bool,
    /// Tabellen über mehrere Seiten
    pub longtable: bool,
    /// Unterabbildungen (subfigure)
    pub subcaption: bool,
    pub graphics: bool,
    pub colors: bool,
    pub underline: bool,
    pub links: bool,
    pub line_spacing: bool,
    /// align/gather/multline (amsmath)
    pub ams: bool,
}

/// Merkt sich pro Schrift einmalig, ob sie existiert (`\IfFontExistsTF` durchsucht
/// sonst bei jedem Aufruf alle Systemschriften).
pub const FONT_CHECK_MACRO: &str = "\\providecommand{\\visutexfontcheck}[1]{\\ifcsname visutex@font@#1\\endcsname\\else\\IfFontExistsTF{#1}{\\expandafter\\gdef\\csname visutex@font@#1\\endcsname##1##2{##1}}{\\expandafter\\gdef\\csname visutex@font@#1\\endcsname##1##2{##2}}\\fi}";

/// Schriftnamen, die ein Dokument über `\visutexfontcheck`/`\visutexfont` abfragt.
pub fn checked_fonts(latex: &str) -> BTreeSet<String> {
    let mut fonts = BTreeSet::new();
    for command in ["\\visutexfontcheck{", "\\visutexfont{"] {
        let mut rest = latex;
        while let Some(index) = rest.find(command) {
            let after = &rest[index + command.len()..];
            let Some(end) = after.find('}') else {
                break;
            };
            let name = after[..end].trim();
            if !name.is_empty()
                && !name.starts_with('#')
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
            {
                fonts.insert(name.to_string());
            }
            rest = &after[end..];
        }
    }
    fonts
}

/// Vorab-Ergebnisse der Schriftprüfung für das interne Kompilieren (Tectonic).
/// Wird ohne Zeilenumbruch vor die erste Zeile gesetzt, damit Zeilennummern in
/// Fehlermeldungen unverändert bleiben. Unbekannte Schriften bleiben ungeprüft
/// (dann entscheidet `\IfFontExistsTF` einmalig zur Laufzeit).
pub fn font_hint_prefix<'a>(available: impl IntoIterator<Item = &'a str>) -> String {
    available
        .into_iter()
        .map(|font| format!("\\expandafter\\gdef\\csname visutex@font@{font}\\endcsname#1#2{{#1}}"))
        .collect()
}

/// Literaturstil → (bst-Datei, natbib-Optionen)
pub fn bibliography_style(style: &str) -> (&'static str, &'static str) {
    match style {
        "plainnat" => ("plainnat", "numbers,sort&compress"),
        "abbrvnat" => ("abbrvnat", "numbers,sort&compress"),
        "plainnat-authoryear" => ("plainnat", "round,authoryear"),
        "alpha" => ("alpha", "numbers"),
        _ => ("IEEEtranN", "numbers,sort&compress"),
    }
}

pub fn style_for_bst(bst: &str) -> Option<&'static str> {
    match bst {
        "IEEEtranN" | "IEEEtran" => Some("ieee"),
        "plainnat" => Some("plainnat"),
        "abbrvnat" => Some("abbrvnat"),
        "alpha" => Some("alpha"),
        _ => None,
    }
}

fn pdf_font_substitute(font: &str) -> Option<&'static str> {
    match font {
        "Arial" | "Liberation Sans" | "Verdana" | "Segoe UI" | "Calibri" => {
            Some("\\usepackage[scaled=0.92]{helvet}\\renewcommand{\\familydefault}{\\sfdefault}")
        }
        "Times New Roman" | "Liberation Serif" | "Cambria" | "Georgia" => {
            Some("\\usepackage{newtxtext}")
        }
        "Palatino Linotype" => Some("\\usepackage{newpxtext}"),
        "Courier New" | "Consolas" => {
            Some("\\usepackage{courier}\\renewcommand{\\familydefault}{\\ttdefault}")
        }
        _ => None,
    }
}

/// babel-Option der Dokumentsprache (siehe `core::languages`).
pub fn babel_language(language: &str) -> &'static str {
    &super::languages::get(language).babel
}

fn mm(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{rounded}mm")
}

fn paper_option(settings: &DocumentSettings) -> String {
    match settings.paper_format.as_str() {
        "a5" => "a5paper".into(),
        "letter" => "letterpaper".into(),
        "legal" => "legalpaper".into(),
        "custom" => format!(
            "paperwidth={},paperheight={}",
            mm(settings.custom_width),
            mm(settings.custom_height)
        ),
        _ => "a4paper".into(),
    }
}

/// Ersetzt Platzhalter in Kopf-/Fußzeilentexten; liefert (LaTeX, braucht lastpage).
pub fn header_footer_latex(text: &str, settings: &DocumentSettings) -> (String, bool) {
    let mut result = String::new();
    let mut last_page = false;
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let Some(length) = rest[start..].find('}') else {
            break;
        };
        let token = &rest[start..start + length + 1];
        let replacement = match token {
            "{seite}" => Some("\\thepage{}".to_string()),
            "{seiten}" => {
                last_page = true;
                Some("\\pageref*{LastPage}".to_string())
            }
            "{kapitel}" => Some("\\leftmark{}".to_string()),
            "{titel}" => Some(escape_latex(&settings.metadata.title)),
            "{autor}" => Some(escape_latex(&settings.metadata.author)),
            "{datum}" => Some("\\today{}".to_string()),
            _ => None,
        };
        match replacement {
            Some(value) => {
                result.push_str(&escape_latex(&rest[..start]));
                result.push_str(&value);
                rest = &rest[start + length + 1..];
            }
            None => {
                result.push_str(&escape_latex(&rest[..start + 1]));
                rest = &rest[start + 1..];
            }
        }
    }
    result.push_str(&escape_latex(rest));
    (result, last_page)
}

fn spacing_command(value: f64) -> Option<String> {
    if (value - 1.0).abs() < 0.001 {
        None
    } else if (value - 1.5).abs() < 0.001 {
        Some("\\onehalfspacing".into())
    } else if (value - 2.0).abs() < 0.001 {
        Some("\\doublespacing".into())
    } else {
        Some(format!(
            "\\setstretch{{{}}}",
            (value * 100.0).round() / 100.0
        ))
    }
}

/// Zeilenabstand als Absatz-Deklaration.
pub fn paragraph_spacing_declaration(value: f64) -> String {
    spacing_command(value).unwrap_or_else(|| "\\singlespacing".into())
}

/// Luminanz nach WCAG – dunkle PDF-Seitenfarbe ⇒ weiße Standardschrift.
pub fn is_dark_color(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let luminance: f64 = [0.2126, 0.7152, 0.0722]
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            let channel = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap_or(255)
                as f64
                / 255.0;
            let linear = if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            };
            weight * linear
        })
        .sum();
    luminance <= 0.179
}

/// PDF/A: `\DocumentMetadata{pdfstandard=A-2b, lang=de-DE}` (vor `\documentclass`;
/// LaTeX-PDF-Verwaltung, schreibt XMP-Metadaten und Farbprofil).
pub fn document_metadata_line(settings: &DocumentSettings) -> Option<String> {
    let standard = settings.metadata.pdf_standard.as_str();
    if !super::settings::PDF_STANDARDS.contains(&standard) {
        return None;
    }
    let level = standard.trim_start_matches("a-");
    Some(format!(
        "\\DocumentMetadata{{pdfstandard=A-{level}, lang={}}}",
        super::languages::get(&settings.language).bcp47
    ))
}

pub fn generate(settings: &DocumentSettings, usage: &Usage, addon_preamble: &[String]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let report = settings.has_chapters();
    let font_size = settings.default_font_size;

    lines.push("%% Erzeugt mit VisuTeX – kompilierbar mit pdfLaTeX, XeLaTeX und LuaLaTeX.".into());
    if let Some(line) = document_metadata_line(settings) {
        lines.push(line);
    }
    let mut options: Vec<String> = Vec::new();
    if report {
        options.push(format!("fontsize={font_size}pt"));
        if settings.paper_format != "custom" {
            options.push(format!("paper={}", settings.paper_format));
        }
        options.push(
            if settings.twoside {
                "twoside"
            } else {
                "oneside"
            }
            .into(),
        );
        if settings.paragraph_style == "skip" {
            options.push("parskip=half".into());
        }
        options.push("listof=totoc".into());
        options.push("bibliography=totoc".into());
        lines.push(format!(
            "\\documentclass[{}]{{scrreprt}}",
            options.join(",")
        ));
    } else {
        let standard_size = matches!(font_size, 10..=12);
        if standard_size {
            options.push(format!("{font_size}pt"));
        }
        if settings.paper_format != "custom" {
            options.push(paper_option(settings));
        }
        options.push(
            if settings.twoside {
                "twoside"
            } else {
                "oneside"
            }
            .into(),
        );
        lines.push(format!("\\documentclass[{}]{{article}}", options.join(",")));
        if !standard_size {
            lines.push(format!("\\usepackage[fontsize={font_size}pt]{{scrextend}}"));
        }
    }

    lines.extend(engine_font_lines(settings, usage));

    lines.push(super::languages::babel_line(&settings.language));
    lines.push("\\usepackage{csquotes}".into());
    generate_rest(settings, usage, addon_preamble, lines)
}

/// Engine-neutrale Schrift-Einrichtung (iftex): pdfLaTeX mit T1/utf8/lmodern,
/// XeLaTeX/LuaLaTeX/Tectonic mit fontspec; eigene Schriften nur, wenn vorhanden.
pub fn engine_font_lines(settings: &DocumentSettings, usage: &Usage) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let font = settings.default_font_family.as_str();
    let custom_font = font != DEFAULT_FONT;
    lines.push("\\usepackage{iftex}".into());
    lines.push("\\ifPDFTeX".into());
    lines.push("  \\usepackage[T1]{fontenc}".into());
    lines.push("  \\usepackage[utf8]{inputenc}".into());
    lines.push("  \\usepackage{lmodern}".into());
    if custom_font {
        if let Some(substitute) = pdf_font_substitute(font) {
            lines.push(format!("  {substitute}"));
        }
    }
    if usage.font_helper {
        lines.push("  \\providecommand{\\visutexfont}[2]{#2}".into());
    }
    lines.push("\\else".into());
    lines.push("  \\usepackage{fontspec}".into());
    if custom_font || usage.font_helper {
        // Jede Schrift wird nur einmal gesucht (\IfFontExistsTF ist teuer); das
        // Ergebnis wird global als \visutex@font@<Name> = \firstoftwo/\secondoftwo gemerkt.
        lines.push(format!("  {FONT_CHECK_MACRO}"));
    }
    if custom_font {
        lines.push(format!(
            "  \\visutexfontcheck{{{font}}}\\csname visutex@font@{font}\\endcsname{{\\setmainfont{{{font}}}}}{{}}"
        ));
    }
    if usage.font_helper {
        lines.push(
            "  \\providecommand{\\visutexfont}[2]{\\visutexfontcheck{#1}\\csname visutex@font@#1\\endcsname{{\\fontspec{#1}#2}}{#2}}"
                .into(),
        );
    }
    lines.push("\\fi".into());
    lines
}

fn generate_rest(
    settings: &DocumentSettings,
    usage: &Usage,
    addon_preamble: &[String],
    mut lines: Vec<String>,
) -> String {
    let report = settings.has_chapters();
    let needs_bibliography = usage.bibliography || usage.citations;

    let mut geometry = vec![paper_option(settings)];
    if settings.orientation == "landscape" {
        geometry.push("landscape".into());
    }
    geometry.push(format!("top={}", mm(settings.margins.top)));
    geometry.push(format!("bottom={}", mm(settings.margins.bottom)));
    geometry.push(format!("left={}", mm(settings.margins.left)));
    geometry.push(format!("right={}", mm(settings.margins.right)));
    if settings.binding_offset > 0.0 {
        geometry.push(format!("bindingoffset={}", mm(settings.binding_offset)));
    }
    if settings.header_footer.enabled {
        geometry.push("headheight=15pt".into());
    }
    lines.push(format!("\\usepackage[{}]{{geometry}}", geometry.join(",")));
    lines.push("\\usepackage{amsmath,amssymb}".into());
    lines.push("\\usepackage{graphicx}".into());
    lines.push("\\usepackage[dvipsnames,table]{xcolor}".into());
    lines.push("\\usepackage{setspace}".into());
    lines.push("\\usepackage[normalem]{ulem}".into());
    lines.push("\\usepackage{array,tabularx}".into());
    if usage.longtable {
        lines.push("\\usepackage{longtable}".into());
    }
    if usage.multirow {
        lines.push("\\usepackage{multirow}".into());
    }
    if usage.multicol || settings.columns > 1 {
        lines.push("\\usepackage{multicol}".into());
    }
    if usage.landscape_sections {
        lines.push("\\usepackage{pdflscape}".into());
    }
    if usage.float_here {
        if report {
            // KOMA-Script: float nutzt eine veraltete Schnittstelle → scrhack vermeidet die Warnung
            lines.push("\\usepackage{scrhack}".into());
        }
        lines.push("\\usepackage{float}".into());
    }
    lines.push("\\usepackage{caption}".into());
    if usage.subcaption {
        lines.push("\\usepackage{subcaption}".into());
    }
    lines.push("\\usepackage{eurosym}".into());
    if usage.booktabs {
        lines.push("\\usepackage{booktabs}".into());
    }
    if usage.siunitx {
        lines.push("\\usepackage{siunitx}".into());
        lines.push(super::languages::siunitx_setup(&settings.language));
    }
    if usage.enumitem {
        lines.push("\\usepackage{enumitem}".into());
    }
    if usage.listings {
        lines.push("\\usepackage{listings}".into());
        lines.push("\\lstset{basicstyle=\\ttfamily\\small,breaklines=true,frame=single,columns=fullflexible,keepspaces=true}".into());
    }
    if usage.tcolorbox {
        lines.push("\\usepackage[most]{tcolorbox}".into());
    }
    if usage.acronyms {
        lines.push("\\usepackage{acronym}".into());
    }
    if usage.tikz || usage.circuitikz {
        lines.push("\\usepackage{tikz}".into());
        // `babel`: aktive Zeichen mancher Sprachen (z. B. „:“ im Französischen) in TikZ entschärfen
        lines.push(format!(
            "\\usetikzlibrary{{{},babel}}",
            super::sketch_catalog::TIKZ_LIBRARIES
        ));
    }
    if usage.circuitikz {
        lines.push("\\usepackage[european]{circuitikz}".into());
    }
    if needs_bibliography {
        let (_, natbib) = bibliography_style(&settings.bibliography.style);
        lines.push(format!("\\usepackage[{natbib}]{{natbib}}"));
    }
    if !report && (needs_bibliography || usage.list_of_figures || usage.list_of_tables) {
        lines.push("\\usepackage[nottoc]{tocbibind}".into());
    }

    if settings.header_footer.enabled {
        let hf = &settings.header_footer;
        let fields = [
            ("head", "L", &hf.header_left),
            ("head", "C", &hf.header_center),
            ("head", "R", &hf.header_right),
            ("foot", "L", &hf.footer_left),
            ("foot", "C", &hf.footer_center),
            ("foot", "R", &hf.footer_right),
        ];
        let converted: Vec<(&str, &str, String, bool)> = fields
            .iter()
            .map(|(kind, position, text)| {
                let (latex, last_page) = header_footer_latex(text, settings);
                (*kind, *position, latex, last_page)
            })
            .collect();
        if converted.iter().any(|field| field.3) {
            lines.push("\\usepackage{lastpage}".into());
        }
        let has_header = converted
            .iter()
            .any(|field| field.0 == "head" && !field.2.is_empty());
        if report {
            // KOMA-Script: scrlayer-scrpage statt fancyhdr (fancyhdr erzeugt eine Klassenwarnung).
            lines.push(if has_header {
                "\\usepackage[headsepline]{scrlayer-scrpage}".into()
            } else {
                "\\usepackage{scrlayer-scrpage}".into()
            });
            lines.push("\\clearpairofpagestyles".into());
            lines.push("\\automark[section]{chapter}".into());
            for (kind, position, latex, _) in &converted {
                if latex.is_empty() {
                    continue;
                }
                let position = position.to_ascii_lowercase();
                // Fußzeilen auch auf Kapitelanfangsseiten (Seitenstil plain).
                let plain = if *kind == "foot" {
                    format!("[{latex}]")
                } else {
                    String::new()
                };
                lines.push(format!(
                    "\\{position}o{kind}{plain}{{{latex}}}\\{position}e{kind}{plain}{{{latex}}}"
                ));
            }
            lines.push("\\pagestyle{scrheadings}".into());
        }
    }
    if settings.header_footer.enabled && !report {
        let hf = &settings.header_footer;
        let converted: Vec<(&str, &str, String)> = [
            ("head", "L", &hf.header_left),
            ("head", "C", &hf.header_center),
            ("head", "R", &hf.header_right),
            ("foot", "L", &hf.footer_left),
            ("foot", "C", &hf.footer_center),
            ("foot", "R", &hf.footer_right),
        ]
        .iter()
        .map(|(kind, position, text)| (*kind, *position, header_footer_latex(text, settings).0))
        .collect();
        lines.push("\\usepackage{fancyhdr}".into());
        lines.push("\\pagestyle{fancy}".into());
        lines.push("\\fancyhf{}".into());
        lines.push("\\renewcommand{\\sectionmark}[1]{\\markboth{#1}{}}".into());
        for (kind, position, latex) in &converted {
            if !latex.is_empty() {
                lines.push(format!("\\fancy{kind}[{position}]{{{latex}}}"));
            }
        }
        if !converted
            .iter()
            .any(|field| field.0 == "head" && !field.2.is_empty())
        {
            lines.push("\\renewcommand{\\headrulewidth}{0pt}".into());
        }
        let plain_foot: String = converted
            .iter()
            .filter(|field| field.0 == "foot" && !field.2.is_empty())
            .map(|field| format!("\\fancyfoot[{}]{{{}}}", field.1, field.2))
            .collect();
        lines.push(format!("\\fancypagestyle{{plain}}{{\\fancyhf{{}}{plain_foot}\\renewcommand{{\\headrulewidth}}{{0pt}}}}"));
    }

    for line in addon_preamble {
        let trimmed = line.trim_end();
        if !trimmed.trim().is_empty() {
            lines.push(trimmed.to_string());
        }
    }

    lines.push("\\usepackage[hidelinks,linktocpage=true]{hyperref}".into());
    let metadata = &settings.metadata;
    let mut pdf_info: Vec<String> = Vec::new();
    for (key, value) in [
        ("pdftitle", &metadata.title),
        ("pdfauthor", &metadata.author),
        ("pdfsubject", &metadata.subject),
        ("pdfkeywords", &metadata.keywords),
    ] {
        if !value.is_empty() {
            pdf_info.push(format!("{key}={{{}}}", escape_latex(value)));
        }
    }
    if !pdf_info.is_empty() {
        lines.push(format!("\\hypersetup{{{}}}", pdf_info.join(",")));
    }
    if usage.maketitle {
        // \maketitle im Dokument: Titel/Autor aus den Dokumenteigenschaften
        lines.push(format!("\\title{{{}}}", escape_latex(&metadata.title)));
        lines.push(format!("\\author{{{}}}", escape_latex(&metadata.author)));
        lines.push("\\date{\\today}".into());
    }

    let offset = if report { 1 } else { 0 };
    lines.push(format!(
        "\\setcounter{{secnumdepth}}{{{}}}",
        settings.numbering_depth as i32 - offset
    ));
    lines.push(format!(
        "\\setcounter{{tocdepth}}{{{}}}",
        settings.toc_depth as i32 - offset
    ));
    if !report && settings.paragraph_style == "skip" {
        lines.push("\\usepackage{parskip}".into());
    }
    if let Some(spacing) = spacing_command(settings.line_spacing) {
        lines.push(spacing);
    }
    if !settings.page_pdf_color.is_empty() {
        lines.push(format!(
            "\\pagecolor[HTML]{{{}}}",
            settings
                .page_pdf_color
                .trim_start_matches('#')
                .to_ascii_uppercase()
        ));
        if is_dark_color(&settings.page_pdf_color) {
            lines.push("\\AtBeginDocument{\\color{white}}".into());
        }
    }
    let mut result = lines.join("\n");
    result.push('\n');
    result
}

/// Ohne Kommentare.
pub fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut escaped = false;
            for (index, character) in line.char_indices() {
                if character == '%' && !escaped {
                    return &line[..index];
                }
                escaped = character == '\\' && !escaped;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Paketnamen (und `class:<name>`) aus einer Präambel.
pub fn loaded_packages(preamble: &str) -> BTreeSet<String> {
    let text = strip_comments(preamble);
    let mut packages = BTreeSet::new();
    for command in ["\\usepackage", "\\RequirePackage", "\\documentclass"] {
        let mut rest = text.as_str();
        while let Some(index) = rest.find(command) {
            rest = &rest[index + command.len()..];
            let mut cursor = rest.trim_start();
            if cursor.starts_with('[') {
                match cursor.find(']') {
                    Some(end) => cursor = cursor[end + 1..].trim_start(),
                    None => break,
                }
            }
            if let Some(inner) = cursor.strip_prefix('{') {
                if let Some(end) = inner.find('}') {
                    for name in inner[..end].split(',') {
                        let name = name.trim();
                        if !name.is_empty() {
                            if command == "\\documentclass" {
                                packages.insert(format!("class:{name}"));
                            } else {
                                packages.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    packages
}

/// Pakete, die der Dokumentkörper tatsächlich benötigt (für die Prüfung einer
/// eigenen Präambel; die erzeugte Präambel lädt die Grundpakete immer).
pub fn required_packages(usage: &Usage) -> Vec<&'static str> {
    let mut required = Vec::new();
    for (used, package) in [
        (usage.ams, "amsmath"),
        (usage.graphics, "graphicx"),
        (usage.colors, "xcolor"),
        (usage.underline, "ulem"),
        (usage.tabularx, "tabularx"),
        (usage.longtable, "longtable"),
        (usage.subcaption, "subcaption"),
        (usage.links, "hyperref"),
        (usage.line_spacing, "setspace"),
        (usage.booktabs, "booktabs"),
        (usage.siunitx, "siunitx"),
        (usage.enumitem, "enumitem"),
        (usage.listings, "listings"),
        (usage.tcolorbox, "tcolorbox"),
    ] {
        if used {
            required.push(package);
        }
    }
    if usage.multirow {
        required.push("multirow");
    }
    if usage.multicol {
        required.push("multicol");
    }
    if usage.landscape_sections {
        required.push("pdflscape");
    }
    if usage.float_here {
        required.push("float");
    }
    if usage.acronyms {
        required.push("acronym");
    }
    if usage.tikz {
        required.push("tikz");
    }
    if usage.circuitikz {
        required.push("circuitikz");
    }
    if usage.natbib {
        required.push("natbib");
    }
    required
}

/// Pakete, die eine benutzerdefinierte Präambel für den Dokumentkörper nachladen müsste.
pub fn missing_packages(preamble: &str, usage: &Usage) -> Vec<String> {
    let loaded = loaded_packages(preamble);
    let implied: &[(&str, &[&str])] = &[
        ("tikz", &["circuitikz", "pgfplots"]),
        ("xcolor", &["tikz", "circuitikz", "pgfplots"]),
        ("graphicx", &["tikz"]),
        ("amsmath", &["mathtools"]),
    ];
    required_packages(usage)
        .into_iter()
        .filter(|name| {
            !loaded.contains(*name)
                && !implied
                    .iter()
                    .find(|(package, _)| package == name)
                    .is_some_and(|(_, others)| others.iter().any(|other| loaded.contains(*other)))
        })
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_placeholders_are_converted_and_text_is_escaped() {
        let mut settings = DocumentSettings::default();
        settings.metadata.title = "A & B".into();
        let (latex, last_page) = header_footer_latex(
            "Seite {seite} von {seiten} – {titel} {unbekannt}",
            &settings,
        );
        assert_eq!(
            latex,
            "Seite \\thepage{} von \\pageref*{LastPage} – A \\& B \\{unbekannt\\}"
        );
        assert!(last_page);
    }

    #[test]
    fn detects_missing_packages_in_custom_preamble() {
        let usage = Usage {
            tikz: true,
            acronyms: true,
            underline: true,
            tabularx: true,
            graphics: true,
            colors: true,
            ..Usage::default()
        };
        let missing = missing_packages(
            "\\documentclass{article}\n\\usepackage{amsmath,graphicx}\n\\usepackage[table]{xcolor}\n% \\usepackage{tikz}\n\\usepackage{circuitikz}",
            &usage,
        );
        assert_eq!(missing, vec!["ulem", "tabularx", "acronym"]);
        // Nur tatsächlich Benutztes wird verlangt (fremde Dokumente ohne Warnflut).
        assert!(missing_packages("\\documentclass{article}", &Usage::default()).is_empty());
    }

    #[test]
    fn report_preamble_uses_koma_options() {
        let mut settings = DocumentSettings::default();
        settings.twoside = true;
        settings.binding_offset = 8.0;
        let preamble = generate(&settings, &Usage::default(), &[]);
        assert!(preamble.contains("\\documentclass[fontsize=12pt,paper=a4,twoside,listof=totoc,bibliography=totoc]{scrreprt}"));
        assert!(preamble.contains("bindingoffset=8mm"));
        assert!(preamble.contains("\\usepackage[normalem]{ulem}"));
        assert!(preamble.trim_end().ends_with("\\setcounter{tocdepth}{2}"));
    }
}
