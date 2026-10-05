//! Mehrteilige Projekte: `\input{…}`, `\include{…}` und `\subfile{…}` im
//! Dokumentkörper.
//!
//! - Beim Öffnen (`expand`) wird jede eigenständige Einbinde-Zeile durch einen
//!   Markerbereich mit dem Inhalt der Datei ersetzt. Das Ergebnis ist weiterhin
//!   ein vollständiges, kompilierbares Dokument (Marker sind Kommentare) und im
//!   visuellen Modus bearbeitbar (Knoten `includeBlock`).
//! - Beim Speichern (`split`) werden die Bereiche wieder zu `\input{…}` usw.,
//!   der Inhalt geht zurück in die jeweilige Datei.
//!
//! Einbindungen in der Präambel (z. B. `\input{einstellungen}`) bleiben unverändert;
//! LaTeX findet sie beim Kompilieren über den Projektordner.

use super::markers;
use super::settings::is_safe_relative_path;
use serde_json::{json, Value};
use std::path::Path;

/// Höchste Verschachtelungstiefe (Schutz vor Endlosschleifen).
const MAX_DEPTH: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct IncludedFile {
    /// Pfad relativ zum Projektordner (mit Endung)
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, Default)]
pub struct Expanded {
    pub latex: String,
    /// Eingebundene Dateien (relativ, mit Endung)
    pub files: Vec<String>,
    pub warnings: Vec<String>,
}

/// Dateipfad zu einem Einbinde-Argument (`kapitel/einleitung` → `kapitel/einleitung.tex`).
pub fn file_for(argument: &str) -> String {
    let argument = argument.trim().trim_matches('"');
    let has_extension = Path::new(argument)
        .extension()
        .is_some_and(|extension| !extension.is_empty());
    if has_extension {
        argument.to_string()
    } else {
        format!("{argument}.tex")
    }
}

/// Erkennt eine Zeile, die nur aus `\input{x}` / `\include{x}` / `\subfile{x}` besteht.
fn include_line(line: &str) -> Option<(&'static str, String)> {
    let trimmed = line.trim();
    for command in ["input", "include", "subfile"] {
        let Some(rest) = trimmed.strip_prefix(&format!("\\{command}")) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('{')?;
        let close = rest.find('}')?;
        let argument = rest[..close].trim();
        let after = rest[close + 1..].trim();
        // Rest der Zeile: nichts oder ein Kommentar
        if !(after.is_empty() || after.starts_with('%')) || argument.is_empty() {
            return None;
        }
        if argument.contains(['\\', '#', '{']) {
            return None;
        }
        return Some((
            match command {
                "include" => "include",
                "subfile" => "subfile",
                _ => "input",
            },
            argument.to_string(),
        ));
    }
    None
}

fn read_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(match String::from_utf8(bytes) {
        Ok(text) => text,
        // Ältere Dateien in Latin-1
        Err(error) => error
            .into_bytes()
            .iter()
            .map(|byte| *byte as char)
            .collect(),
    })
}

fn expand_text(text: &str, root: &Path, stack: &mut Vec<String>, result: &mut Expanded) -> String {
    let mut output = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let Some((command, argument)) = include_line(line) else {
            output.push_str(line);
            continue;
        };
        let file = file_for(&argument);
        let readable =
            is_safe_relative_path(&file) && stack.len() < MAX_DEPTH && !stack.contains(&file);
        let content = readable.then(|| read_file(&root.join(&file))).flatten();
        let Some(content) = content else {
            if stack.contains(&file) {
                result.warnings.push(format!(
                    "„{file}“ bindet sich selbst ein – nicht aufgelöst."
                ));
            }
            output.push_str(line);
            continue;
        };
        let mut data = json!({ "file": argument, "command": command });
        // \subfile: eigenständiges Dokument → nur der Körper ist bearbeitbar
        let body = match super::export::split_document(&content) {
            Some((preamble, body, postamble)) => {
                data["preamble"] = json!(preamble);
                data["postamble"] = json!(postamble);
                body
            }
            None => content,
        };
        stack.push(file.clone());
        let expanded = expand_text(&body, root, stack, result);
        stack.pop();
        if !result.files.contains(&file) {
            result.files.push(file);
        }
        output.push_str(&markers::wrap(
            "include",
            &data,
            expanded.trim_matches('\n'),
        ));
        output.push('\n');
    }
    output
}

/// Ersetzt Einbindungen im Dokumentkörper durch deren Inhalt (Markerbereiche).
pub fn expand(source: &str, root: &Path) -> Expanded {
    let mut result = Expanded::default();
    let Some(begin) = source.find("\\begin{document}") else {
        result.latex = source.to_string();
        return result;
    };
    let body_start = begin + "\\begin{document}".len();
    let mut stack = Vec::new();
    let body = expand_text(&source[body_start..], root, &mut stack, &mut result);
    result.latex = format!("{}{body}", &source[..body_start]);
    result
}

/// Zerlegt ein Dokument mit Einbinde-Bereichen wieder in Hauptdatei und Teildateien.
pub fn split(latex: &str) -> (String, Vec<IncludedFile>) {
    let mut files = Vec::new();
    let main = split_into(latex, &mut files);
    (main, files)
}

fn split_into(latex: &str, files: &mut Vec<IncludedFile>) -> String {
    let regions = markers::find_regions(latex);
    if !regions.iter().any(|region| region.kind == "include") {
        return latex.to_string();
    }
    let mut output = String::with_capacity(latex.len());
    let mut cursor = 0;
    for region in regions
        .into_iter()
        .filter(|region| region.kind == "include")
    {
        let argument = region
            .data
            .get("file")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let file = file_for(&argument);
        if argument.is_empty() || !is_safe_relative_path(&file) {
            continue;
        }
        let command = match region.data.get("command").and_then(Value::as_str) {
            Some("include") => "include",
            Some("subfile") => "subfile",
            _ => "input",
        };
        output.push_str(&latex[cursor..region.start]);
        output.push_str(&format!("\\{command}{{{argument}}}\n"));
        cursor = region.end;
        let inner = split_into(&region.inner, files);
        let content = match (
            region.data.get("preamble").and_then(Value::as_str),
            region.data.get("postamble").and_then(Value::as_str),
        ) {
            (Some(preamble), postamble) => format!(
                "{preamble}\\begin{{document}}\n{}\n\\end{{document}}{}",
                inner.trim_matches('\n'),
                postamble.unwrap_or("\n")
            ),
            _ => format!("{}\n", inner.trim_matches('\n')),
        };
        files.retain(|existing| existing.path != file);
        files.push(IncludedFile {
            path: file,
            content,
        });
    }
    output.push_str(&latex[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_and_splits_included_files() {
        let root = std::env::temp_dir().join(format!("visutex-includes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("kapitel")).unwrap();
        std::fs::write(
            root.join("kapitel/einleitung.tex"),
            "\\section{Einleitung}\nText.\n\\input{kapitel/detail}\n",
        )
        .unwrap();
        std::fs::write(root.join("kapitel/detail.tex"), "Details.\n").unwrap();
        std::fs::write(root.join("schleife.tex"), "\\input{schleife}\n").unwrap();
        let main = "\\documentclass{article}\n\\input{einstellungen}\n\\begin{document}\n\\include{kapitel/einleitung}\nMitte \\input{inline} bleibt\n  \\input{schleife} % Kommentar\n\\input{fehlt}\n\\end{document}\n";
        let expanded = expand(main, &root);
        assert!(
            expanded.latex.contains("\\input{einstellungen}"),
            "Präambel bleibt"
        );
        assert!(expanded.latex.contains("%% VisuTeX-begin: include {"));
        assert!(expanded.latex.contains("\"file\":\"kapitel/einleitung\""));
        assert!(expanded.latex.contains("\\section{Einleitung}"));
        assert!(expanded.latex.contains("Details."));
        assert!(expanded.latex.contains("Mitte \\input{inline} bleibt"));
        assert!(expanded.latex.contains("\\input{fehlt}"));
        assert_eq!(
            expanded.files,
            vec![
                "kapitel/detail.tex",
                "kapitel/einleitung.tex",
                "schleife.tex"
            ]
        );
        assert_eq!(expanded.warnings.len(), 1, "Schleife erkannt");

        let (restored, files) = split(&expanded.latex);
        assert!(restored.contains("\\begin{document}\n\\include{kapitel/einleitung}\n"));
        assert!(!restored.contains("VisuTeX-begin"));
        let einleitung = files
            .iter()
            .find(|file| file.path == "kapitel/einleitung.tex")
            .unwrap();
        assert_eq!(
            einleitung.content,
            "\\section{Einleitung}\nText.\n\\input{kapitel/detail}\n"
        );
        assert!(files
            .iter()
            .any(|file| file.path == "kapitel/detail.tex" && file.content == "Details.\n"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
