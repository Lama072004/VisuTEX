//! Dateioperationen als eigene Rust-Commands (statt `@tauri-apps/plugin-fs`,
//! dessen Scope Speichern/Kopieren außerhalb des Dialogpfads blockierte).

use crate::core::export::EmbeddedAsset;
use crate::core::settings::is_safe_relative_path;
use base64::Engine;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileStamp {
    pub modified_ms: u64,
    pub size: u64,
}

pub fn stamp(path: &Path) -> Option<FileStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis() as u64;
    Some(FileStamp {
        modified_ms,
        size: metadata.len(),
    })
}

pub fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("„{}“ konnte nicht gelesen werden: {error}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text.trim_start_matches('\u{feff}').to_string()),
        // Ältere LaTeX-Dateien sind oft Latin-1 kodiert.
        Err(error) => Ok(error
            .into_bytes()
            .iter()
            .map(|&byte| byte as char)
            .collect()),
    }
}

/// Schreibt atomar (temporäre Datei + Umbenennen), damit ein Absturz beim
/// Speichern keine halbe Datei hinterlässt.
pub fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    write_bytes_atomic(path, content.as_bytes())
}

pub fn write_bytes_atomic(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Ordner „{}“ konnte nicht angelegt werden: {error}",
                parent.display()
            )
        })?;
    }
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "datei".into());
    let temporary = path.with_file_name(format!(".{file_name}.visutex-tmp"));
    std::fs::write(&temporary, content).map_err(|error| {
        format!(
            "„{}“ konnte nicht gespeichert werden: {error}",
            path.display()
        )
    })?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!(
            "„{}“ konnte nicht gespeichert werden: {error}",
            path.display()
        )
    })
}

/// Schreibt eingebettete Bilder des Exports in den Projektordner (falls nicht vorhanden).
pub fn write_assets(root: &Path, assets: &[EmbeddedAsset]) -> Result<(), String> {
    for asset in assets {
        if !is_safe_relative_path(&asset.path) {
            continue;
        }
        let target = root.join(&asset.path);
        if target.is_file() {
            continue;
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(asset.base64.as_bytes())
            .map_err(|_| format!("Eingebettetes Bild „{}“ ist beschädigt.", asset.path))?;
        write_bytes_atomic(&target, &bytes)?;
    }
    Ok(())
}

pub fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// Kopiert Projektdateien (relative Pfade) von `source` nach `target`.
/// Gleicher Inhalt wird übersprungen; abweichender Inhalt am Ziel ist ein Fehler.
/// Liefert die Dateien, die in der Quelle fehlen.
pub fn copy_project_files(
    source: &Path,
    target: &Path,
    files: &[String],
) -> Result<Vec<String>, String> {
    let mut missing = Vec::new();
    if same_directory(source, target) {
        return Ok(missing);
    }
    for file in files {
        if !is_safe_relative_path(file) {
            continue;
        }
        let Some(from) = find_resource(source, file, &[]) else {
            missing.push(file.clone());
            continue;
        };
        let relative = from
            .strip_prefix(source)
            .unwrap_or(Path::new(file))
            .to_path_buf();
        let to = target.join(&relative);
        let bytes = std::fs::read(&from).map_err(|error| {
            format!("„{}“ konnte nicht gelesen werden: {error}", from.display())
        })?;
        match std::fs::read(&to) {
            Ok(existing) if existing == bytes => continue,
            Ok(_) => {
                return Err(format!(
                    "Am Speicherort existiert bereits eine andere Datei „{}“. Bitte einen anderen Ordner wählen oder die Datei umbenennen.",
                    relative.display()
                ))
            }
            Err(_) => write_bytes_atomic(&to, &bytes)?,
        }
    }
    Ok(missing)
}

// ---------------------------------------------------------------- Ressourcen im LaTeX-Code

fn latex_group(source: &str, start: usize) -> Option<(&str, usize)> {
    if source.as_bytes().get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 1_u32;
    let mut escaped = false;
    for (offset, character) in source[start + 1..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + 1 + offset;
                    return Some((&source[start + 1..end], end + 1));
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub command: String,
    pub path: String,
}

/// Lokale Dateien, auf die ein LaTeX-Dokument verweist (Bilder, Literatur,
/// eingebundene Dateien, eigene Pakete/Klassen im Projektordner).
pub fn referenced_resources(latex: &str) -> Vec<Resource> {
    let text = crate::core::preamble::strip_comments(latex);
    let mut resources: Vec<Resource> = Vec::new();
    for command in [
        "includegraphics",
        "bibliography",
        "addbibresource",
        "input",
        "include",
        "usepackage",
        "RequirePackage",
        "documentclass",
        "bibliographystyle",
    ] {
        let marker = format!("\\{command}");
        let mut offset = 0;
        while let Some(index) = text[offset..].find(&marker) {
            let start = offset + index + marker.len();
            offset = start;
            // Nur ganze Befehle (\input, nicht \inputencoding).
            if text[start..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            {
                continue;
            }
            let mut position = start;
            let rest = &text[position..];
            let trimmed = rest.trim_start();
            position += rest.len() - trimmed.len();
            if trimmed.starts_with('*') {
                position += 1;
            }
            if text[position..].starts_with('[') {
                match text[position..].find(']') {
                    Some(end) => position += end + 1,
                    None => continue,
                }
            }
            let rest = &text[position..];
            position += rest.len() - rest.trim_start().len();
            let Some((argument, end)) = latex_group(&text, position) else {
                continue;
            };
            offset = end;
            let argument = argument
                .strip_prefix("\\detokenize{")
                .and_then(|value| value.strip_suffix('}'))
                .unwrap_or(argument);
            for name in argument.split(',') {
                let name = name.trim();
                if name.is_empty() || name.contains('\\') || name.contains('#') {
                    continue;
                }
                let resource = Resource {
                    command: command.into(),
                    path: name.to_string(),
                };
                if !resources.contains(&resource) {
                    resources.push(resource);
                }
            }
        }
    }
    resources
}

fn extensions_for(command: &str) -> &'static [&'static str] {
    match command {
        "includegraphics" => &[
            "pdf", "png", "jpg", "jpeg", "eps", "PDF", "PNG", "JPG", "JPEG", "EPS",
        ],
        "bibliography" => &["bib"],
        "input" | "include" => &["tex"],
        "usepackage" | "RequirePackage" => &["sty"],
        "documentclass" => &["cls"],
        "bibliographystyle" => &["bst"],
        _ => &[],
    }
}

/// Ordner aus `\graphicspath{{bilder/}{../abb/}}` (ohne Kommentare).
pub fn graphics_paths(latex: &str) -> Vec<String> {
    let text = crate::core::preamble::strip_comments(latex);
    let mut paths = Vec::new();
    let mut offset = 0;
    while let Some(found) = text[offset..].find("\\graphicspath") {
        let start = offset + found + "\\graphicspath".len();
        offset = start;
        let rest = text[start..].trim_start();
        let position = start + (text[start..].len() - rest.len());
        let Some((inner, end)) = latex_group(&text, position) else {
            continue;
        };
        offset = end;
        let mut cursor = 0;
        while let Some(open) = inner[cursor..].find('{') {
            let begin = cursor + open;
            let Some((path, path_end)) = latex_group(inner, begin) else {
                break;
            };
            let path = path.trim();
            if !path.is_empty() && !paths.iter().any(|known: &String| known == path) {
                paths.push(path.to_string());
            }
            cursor = path_end;
        }
    }
    paths
}

/// Bilddatei wie LaTeX/graphicx: direkt, mit Standardendungen und in den
/// Ordnern aus `\graphicspath` (relativ zum Projektordner).
pub fn resolve_graphic(root: &Path, path: &str, search: &[String]) -> Option<PathBuf> {
    let path = path.trim().trim_matches('"');
    if path.is_empty() {
        return None;
    }
    let extensions = extensions_for("includegraphics");
    if let Some(found) = find_resource(root, path, extensions) {
        return Some(found);
    }
    if Path::new(path).is_absolute() {
        return None;
    }
    search.iter().find_map(|folder| {
        let folder = folder.replace('\\', "/");
        let folder = folder.trim_start_matches("./");
        find_resource(&root.join(folder), path, extensions)
    })
}

/// Sucht eine Projektdatei (mit typischen Endungen, falls keine angegeben).
pub fn find_resource(root: &Path, path: &str, extensions: &[&str]) -> Option<PathBuf> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    let direct = root.join(path);
    if direct.is_file() {
        return Some(direct);
    }
    extensions
        .iter()
        .map(|extension| root.join(format!("{path}.{extension}")))
        .find(|candidate| candidate.is_file())
}

/// Für das Kopieren beim Export: alle lokal vorhandenen Ressourcen als relative Pfade.
pub fn local_resource_files(latex: &str, root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let search = graphics_paths(latex);
    for resource in referenced_resources(latex) {
        if Path::new(&resource.path).is_absolute() || !is_safe_relative_path(&resource.path) {
            continue;
        }
        let found = if resource.command == "includegraphics" {
            resolve_graphic(root, &resource.path, &search)
        } else {
            find_resource(root, &resource.path, extensions_for(&resource.command))
        };
        if let Some(found) = found {
            if let Ok(relative) = found.strip_prefix(root) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                if !files.contains(&relative) {
                    files.push(relative);
                }
            }
        }
    }
    files
}

/// Fehlende lokale Ressourcen (Pakete/Klassen nur, wenn sie wie Dateien aussehen).
pub fn missing_resources(latex: &str, root: &Path) -> Vec<Resource> {
    let search = graphics_paths(latex);
    referenced_resources(latex)
        .into_iter()
        .filter(|resource| {
            let looks_local = resource.path.contains('/') || resource.path.contains('.');
            let always_local = matches!(
                resource.command.as_str(),
                "includegraphics" | "bibliography" | "addbibresource" | "input" | "include"
            );
            let found = if resource.command == "includegraphics" {
                resolve_graphic(root, &resource.path, &search).is_some()
            } else {
                find_resource(root, &resource.path, extensions_for(&resource.command)).is_some()
            };
            (always_local || looks_local) && !found
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_resources_and_missing_files() {
        let latex = "\\documentclass{scrreprt}\n\\usepackage[utf8]{inputenc}\n\\usepackage{eigenes/paket}\n% \\includegraphics{auskommentiert.png}\n\\begin{document}\\includegraphics[width=\\linewidth]{Abbildungen/logo}\\input{kapitel/einleitung}\\inputencoding{x}\\bibliography{literatur,mehr}\\end{document}";
        let resources: Vec<String> = referenced_resources(latex)
            .into_iter()
            .map(|resource| format!("{}:{}", resource.command, resource.path))
            .collect();
        assert_eq!(
            resources,
            vec![
                "includegraphics:Abbildungen/logo",
                "bibliography:literatur",
                "bibliography:mehr",
                "input:kapitel/einleitung",
                "usepackage:inputenc",
                "usepackage:eigenes/paket",
                "documentclass:scrreprt",
            ]
        );
        let root = std::env::temp_dir().join(format!("visutex-res-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Abbildungen")).unwrap();
        std::fs::write(root.join("Abbildungen/logo.png"), b"png").unwrap();
        std::fs::write(root.join("literatur.bib"), b"@misc{a,}").unwrap();
        let missing: Vec<String> = missing_resources(latex, &root)
            .into_iter()
            .map(|resource| resource.path)
            .collect();
        assert_eq!(missing, vec!["mehr", "kapitel/einleitung", "eigenes/paket"]);
        assert_eq!(
            local_resource_files(latex, &root),
            vec!["Abbildungen/logo.png", "literatur.bib"]
        );
        let target = root.join("kopie");
        let missing = copy_project_files(
            &root,
            &target,
            &["Abbildungen/logo.png".into(), "fehlt.png".into()],
        )
        .unwrap();
        assert_eq!(missing, vec!["fehlt.png"]);
        assert!(target.join("Abbildungen/logo.png").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
