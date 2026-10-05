//! Vorbereitung beliebiger LaTeX-Dokumente für die eingebaute Engine (Tectonic/XeTeX).
//!
//! Die Datei des Nutzers bleibt unverändert – angepasst wird nur der Text, der an
//! die Engine geht. Alle Ersetzungen bleiben in derselben Zeile, damit
//! Zeilennummern in Fehlermeldungen stimmen.
//!
//! - `biblatex` mit Biber (externes Programm, nicht enthalten) → `backend=bibtex`
//!   (BibTeX ist in Tectonic eingebaut; Zitate und Literaturverzeichnis funktionieren).
//! - `pdfx` (PDF/A) scheitert unter XeTeX → wird übersprungen, `hyperref` bleibt verfügbar.
//! - Fehlende Bilder (`\includegraphics`) → beschrifteter Platzhalter statt Abbruch.

use std::path::Path;

/// Platzhalter für fehlende Bilder (vor `\documentclass`, ohne Zeilenumbruch).
const MISSING_GRAPHIC_MACRO: &str = "\\providecommand\\visutexmissinggraphic[2]{\\fbox{\\parbox[c][0.25\\linewidth][c]{\\dimexpr#1-2\\fboxsep-2\\fboxrule\\relax}{\\centering\\footnotesize\\textbf{Bild fehlt:}\\\\\\detokenize{#2}}}}";

#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub latex: String,
    pub notes: Vec<String>,
}

/// Teil einer Zeile vor einem unmaskierten `%`.
fn code_part(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'%' => return &line[..index],
            _ => index += 1,
        }
    }
    line
}

/// Findet `\usepackage[opts]{name}` bzw. `\RequirePackage…` in `code`; liefert
/// (Start, Ende, Optionen-Bereich) für das Paket `name`.
fn find_package(code: &str, name: &str) -> Option<(usize, usize, Option<(usize, usize)>)> {
    for command in ["\\usepackage", "\\RequirePackage"] {
        let mut offset = 0;
        while let Some(found) = code[offset..].find(command) {
            let start = offset + found;
            let mut position = start + command.len();
            offset = position;
            let rest = &code[position..];
            position += rest.len() - rest.trim_start().len();
            let mut options = None;
            if code[position..].starts_with('[') {
                let close = code[position..].find(']')? + position;
                options = Some((position + 1, close));
                position = close + 1;
                let rest = &code[position..];
                position += rest.len() - rest.trim_start().len();
            }
            if !code[position..].starts_with('{') {
                continue;
            }
            let close = code[position..].find('}')? + position;
            let packages = &code[position + 1..close];
            if packages.split(',').any(|package| package.trim() == name) {
                let mut end = close + 1;
                // optionales Datum: \usepackage{pdfx}[2018/12/22]
                if code[end..].starts_with('[') {
                    if let Some(date_end) = code[end..].find(']') {
                        end += date_end + 1;
                    }
                }
                return Some((start, end, options));
            }
        }
    }
    None
}

fn preamble_end(latex: &str) -> usize {
    latex.find("\\begin{document}").unwrap_or(latex.len())
}

fn loads_package(preamble: &str, name: &str) -> bool {
    preamble
        .lines()
        .any(|line| find_package(code_part(line), name).is_some())
}

/// Ende einer Gruppe `open … close` ab `start` (zeigt auf `open`), Zeilen-lokal.
fn group_end(code: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = code.as_bytes();
    let mut depth = 0;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            byte if byte == open => depth += 1,
            byte if byte == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Breite aus `width=…` der \includegraphics-Optionen (sonst halbe Zeilenbreite).
fn graphic_width(options: &str) -> String {
    crate::core::export::option_value(options, "width")
        .filter(|width| crate::core::export::is_balanced(width) && !width.is_empty())
        .unwrap_or_else(|| "0.5\\linewidth".into())
}

/// Ersetzt `\includegraphics` mit fehlender Datei durch einen Platzhalter
/// (in derselben Zeile, Zeilennummern bleiben gleich).
fn replace_missing_graphics(latex: &str, root: &Path, notes: &mut Vec<String>) -> String {
    let search = crate::files::graphics_paths(latex);
    let mut output = String::with_capacity(latex.len() + 256);
    let mut used = false;
    for (index, line) in latex.split_inclusive('\n').enumerate() {
        let code = code_part(line);
        let mut cursor = 0;
        let mut from = 0;
        while let Some(found) = code[from..].find("\\includegraphics") {
            let start = from + found;
            let mut position = start + "\\includegraphics".len();
            from = position;
            if code[position..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                continue;
            }
            if code[position..].starts_with('*') {
                position += 1;
            }
            position += code[position..].len() - code[position..].trim_start().len();
            let mut options = "";
            if code[position..].starts_with('[') {
                let Some(close) = group_end(code, position, b'[', b']') else {
                    break;
                };
                options = &code[position + 1..close];
                position = close + 1;
                position += code[position..].len() - code[position..].trim_start().len();
            }
            if !code[position..].starts_with('{') {
                continue;
            }
            let Some(close) = group_end(code, position, b'{', b'}') else {
                break;
            };
            let path = code[position + 1..close].trim();
            from = close + 1;
            if path.is_empty() || path.contains(['\\', '#']) {
                continue;
            }
            let problem = match crate::files::resolve_graphic(root, path, &search) {
                Some(file) if graphic_is_readable(&file) => continue,
                Some(_) => "ist beschädigt bzw. nicht lesbar",
                None => "wurde im Projektordner nicht gefunden",
            };
            output.push_str(&line[cursor..start]);
            output.push_str(&format!(
                "\\visutexmissinggraphic{{{}}}{{{path}}}",
                graphic_width(options)
            ));
            cursor = close + 1;
            used = true;
            notes.push(format!(
                "Zeile {}: Bild „{path}“ {problem} – Platzhalter eingesetzt.",
                index + 1
            ));
        }
        output.push_str(&line[cursor..]);
    }
    if used {
        format!("{MISSING_GRAPHIC_MACRO}{output}")
    } else {
        output
    }
}

/// PNG/JPEG vollständig dekodierbar? Beschädigte Dateien brächten die PDF-Ausgabe
/// (libpng in xdvipdfmx) zum Absturz. Ergebnis je Datei (Pfad, Größe, Änderungszeit)
/// zwischengespeichert, damit große Dokumente nicht bei jedem Lauf neu prüfen.
pub fn graphic_is_readable(file: &Path) -> bool {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Key = (std::path::PathBuf, u64, Option<std::time::SystemTime>);
    static CACHE: OnceLock<Mutex<HashMap<Key, bool>>> = OnceLock::new();
    let extension = file
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        return true;
    }
    let Ok(metadata) = std::fs::metadata(file) else {
        return false;
    };
    let key = (file.to_path_buf(), metadata.len(), metadata.modified().ok());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(result) = cache.lock().ok().and_then(|cache| cache.get(&key).copied()) {
        return result;
    }
    let readable = image::ImageReader::open(file)
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .is_some_and(|reader| reader.decode().is_ok());
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, readable);
    }
    readable
}

pub fn prepare_for_engine(latex: &str, project_root: Option<&Path>) -> Prepared {
    let mut prepared = prepare_packages(latex);
    if let Some(root) = project_root {
        prepared.latex = replace_missing_graphics(&prepared.latex, root, &mut prepared.notes);
    }
    prepared
}

/// Pakete, die eine eigene Textschrift festlegen (dann bleibt T1 unangetastet).
const FONT_PACKAGES: &[&str] = &[
    "lmodern",
    "fontspec",
    "times",
    "mathptmx",
    "newtxtext",
    "newpxtext",
    "helvet",
    "tgheros",
    "tgtermes",
    "tgpagella",
    "tgbonum",
    "tgschola",
    "palatino",
    "mathpazo",
    "libertine",
    "libertinus",
    "charter",
    "XCharter",
    "bookman",
    "kpfonts",
    "fourier",
    "ebgaramond",
    "sourcesanspro",
    "sourceserifpro",
    "roboto",
    "opensans",
    "lato",
    "cmbright",
    "arev",
    "fouriernc",
    "concrete",
    "ae",
    "cm-super",
    "mlmodern",
];

fn prepare_packages(latex: &str) -> Prepared {
    let split = preamble_end(latex);
    let (preamble, body) = latex.split_at(split);
    let mut notes = Vec::new();
    let mut output = String::with_capacity(latex.len() + 64);
    let hyperref_elsewhere = loads_package(preamble, "hyperref");
    // T1-Kodierung mit Computer Modern (EC-Schriften): stattdessen das optisch
    // gleichwertige Latin Modern, das im mitgelieferten Bundle enthalten ist.
    let use_latin_modern = !FONT_PACKAGES
        .iter()
        .any(|package| loads_package(preamble, package));

    for (index, line) in preamble.split_inclusive('\n').enumerate() {
        let code = code_part(line);
        let mut current = line.to_string();
        if use_latin_modern {
            if let Some((_, end, Some((start, options_end)))) = find_package(code, "fontenc") {
                if code[start..options_end]
                    .split(',')
                    .any(|option| option.trim() == "T1")
                {
                    current = format!("{}\\usepackage{{lmodern}}{}", &line[..end], &line[end..]);
                    notes.push(format!(
                        "Zeile {}: T1-Schriften (Computer Modern) werden als Latin Modern gesetzt.",
                        index + 1
                    ));
                }
            }
        }
        let base = current.clone();
        let code = code_part(&base);
        if let Some((_, _, options)) = find_package(code, "biblatex") {
            match options {
                Some((start, end)) => {
                    let text = &code[start..end];
                    if text.contains("backend=biber") || text.contains("backend = biber") {
                        let replaced = text
                            .replace("backend=biber", "backend=bibtex")
                            .replace("backend = biber", "backend=bibtex");
                        current = format!("{}{}{}", &base[..start], replaced, &base[end..]);
                    } else if !text.contains("backend") {
                        current = format!("{}backend=bibtex,{}", &base[..start], &base[start..]);
                    }
                }
                None => {
                    let (start, _, _) = find_package(code, "biblatex").unwrap_or((0, 0, None));
                    let command_end = start
                        + if code[start..].starts_with("\\usepackage") {
                            "\\usepackage".len()
                        } else {
                            "\\RequirePackage".len()
                        };
                    current = format!(
                        "{}[backend=bibtex]{}",
                        &base[..command_end],
                        &base[command_end..]
                    );
                }
            }
            if current != base {
                notes.push(format!(
                    "Zeile {}: biblatex wird mit dem eingebauten BibTeX statt Biber verarbeitet (Datei bleibt unverändert).",
                    index + 1
                ));
            }
        }
        let code = code_part(&current).to_string();
        if let Some((start, end, _)) = find_package(&code, "pdfx") {
            let replacement = if hyperref_elsewhere {
                "\\relax"
            } else {
                "\\usepackage{hyperref}"
            };
            current = format!("{}{}{}", &current[..start], replacement, &current[end..]);
            notes.push(format!(
                "Zeile {}: pdfx (PDF/A) wird von der eingebauten Engine nicht unterstützt und beim Kompilieren übersprungen.",
                index + 1
            ));
        }
        output.push_str(&current);
    }
    output.push_str(body);
    Prepared {
        latex: output,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_biber_and_pdfx_without_changing_line_numbers() {
        let source = "\\documentclass{scrreprt}\n\\usepackage[linktocpage=true]{hyperref}\n\\usepackage[a-2b,mathxmp]{pdfx}[2018/12/22]\n\\usepackage[style=ieee,citestyle=ieee,backend=biber]{biblatex}   % Literatur\n%\\usepackage[backend=biber]{biblatex}\n\\begin{document}\n\\usepackage{pdfx} im Text bleibt\n\\end{document}\n";
        let prepared = prepare_for_engine(source, None);
        assert_eq!(prepared.latex.lines().count(), source.lines().count());
        assert!(prepared.latex.contains("\\relax"));
        assert!(!prepared.latex.lines().nth(2).unwrap().contains("pdfx"));
        assert!(prepared
            .latex
            .contains("[style=ieee,citestyle=ieee,backend=bibtex]{biblatex}   % Literatur"));
        assert!(
            prepared
                .latex
                .contains("%\\usepackage[backend=biber]{biblatex}"),
            "Kommentare bleiben"
        );
        assert!(
            prepared.latex.contains("\\usepackage{pdfx} im Text bleibt"),
            "Körper bleibt"
        );
        assert_eq!(prepared.notes.len(), 2);
    }

    #[test]
    fn adds_backend_when_missing() {
        let prepared = prepare_for_engine("\\documentclass{article}\n\\usepackage{biblatex}\n\\usepackage[style=apa]{biblatex}\n\\begin{document}\\end{document}", None);
        assert!(prepared
            .latex
            .contains("\\usepackage[backend=bibtex]{biblatex}"));
        assert!(prepared
            .latex
            .contains("\\usepackage[backend=bibtex,style=apa]{biblatex}"));
        let untouched = prepare_for_engine("\\documentclass{article}\n\\usepackage[backend=bibtex8]{biblatex}\n\\begin{document}\\end{document}", None);
        assert!(untouched.notes.is_empty());
        let pdfx_only = prepare_for_engine(
            "\\documentclass{article}\n\\usepackage{pdfx}\n\\begin{document}\\end{document}",
            None,
        );
        assert!(pdfx_only.latex.contains("\\usepackage{hyperref}"));
    }

    #[test]
    fn t1_computer_modern_becomes_latin_modern() {
        let source = "\\documentclass{article}\n\\usepackage[utf8]{inputenc}\n\\usepackage[T1]{fontenc} % Schrift\n\\begin{document}x\\end{document}";
        let prepared = prepare_for_engine(source, None);
        assert!(prepared
            .latex
            .contains("\\usepackage[T1]{fontenc}\\usepackage{lmodern} % Schrift"));
        assert_eq!(prepared.latex.lines().count(), source.lines().count());
        let own_font = prepare_for_engine(
            "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{helvet}\n\\begin{document}x\\end{document}",
            None,
        );
        assert!(!own_font.latex.contains("lmodern"));
    }

    #[test]
    fn missing_graphics_become_placeholders() {
        let root = std::env::temp_dir().join(format!("visutex-prepare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Bilder mit Leerzeichen")).unwrap();
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAGklEQVR4nGNQTX79dR43hGSAs1STXzPglAEAf0IYiZt0O6wAAAAASUVORK5CYII=")
            .unwrap();
        std::fs::write(root.join("Bilder mit Leerzeichen/da.png"), &png).unwrap();
        // Beschädigt (falsche Prüfsumme) – brächte libpng in xdvipdfmx zum Absturz.
        let mut broken = png.clone();
        let data = broken.len() - 20;
        broken[data] ^= 0xff;
        std::fs::write(root.join("kaputt.png"), &broken).unwrap();
        let source = "\\documentclass{article}\n\\graphicspath{{Bilder mit Leerzeichen/}}\n\\begin{document}\n\\includegraphics[width=0.8\\textwidth]{da}\n\\includegraphics[width=0.8\\textwidth]{fehlt/bild.png} Text\n% \\includegraphics{auskommentiert}\n\\includegraphics{kaputt}\n\\end{document}\n";
        let prepared = prepare_for_engine(source, Some(&root));
        assert_eq!(prepared.latex.lines().count(), source.lines().count());
        assert!(prepared
            .latex
            .contains("\\includegraphics[width=0.8\\textwidth]{da}"));
        assert!(prepared
            .latex
            .contains("\\visutexmissinggraphic{0.8\\textwidth}{fehlt/bild.png} Text"));
        assert!(prepared
            .latex
            .contains("% \\includegraphics{auskommentiert}"));
        assert!(prepared
            .latex
            .starts_with("\\providecommand\\visutexmissinggraphic"));
        assert!(prepared
            .latex
            .contains("\\visutexmissinggraphic{0.5\\linewidth}{kaputt}"));
        assert_eq!(prepared.notes.len(), 2, "{:?}", prepared.notes);
        assert!(prepared.notes[1].contains("beschädigt"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
