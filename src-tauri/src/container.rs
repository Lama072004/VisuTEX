//! Projektdatei als Container (wie `.docx`): eine `.visutex`-Datei im ZIP-Format mit dem
//! vollständigen LaTeX-Dokument, allen Projektdateien (Bilder, Literatur, Teildateien) und
//! – auf Wunsch – dem Versionsverlauf. Ohne VisuTeX lässt sie sich entpacken (bzw. nach `.zip`
//! umbenennen) und `<Name>.tex` in jedem LaTeX-Editor öffnen oder bei Overleaf hochladen.
//!
//! ```text
//! <Name>.tex                  Hauptdokument (portables LaTeX)
//! Abbildungen/…, literatur.bib, Teildateien …   (Pfade wie im Dokument)
//! LIESMICH.txt                Hinweis für Nutzer ohne VisuTeX
//! visutex/projekt.json        VisuTeX-Projekt (Format v2: Einstellungen, Dokument, Ansicht)
//! visutex/versionen/…         Versionsverlauf (index.json, <id>.txt, dateien/<sha256>)
//! ```
//!
//! Geschrieben wird immer vollständig in eine temporäre Datei daneben, die nach `sync_all`
//! die alte ersetzt – ein Absturz beim Speichern lässt die bisherige Datei unversehrt.

use crate::files;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const PROJECT_ENTRY: &str = "visutex/projekt.json";
pub const VERSIONS_PREFIX: &str = "visutex/versionen/";
/// Grenzen beim Entpacken fremder Dateien (ZIP-Bomben, beschädigte Dateien)
const MAX_TOTAL: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 50_000;

const README: &str = "Diese Datei ist ein VisuTeX-Projekt im ZIP-Format.\r\n\
\r\n\
Ohne VisuTeX bearbeiten: Datei entpacken (oder in .zip umbenennen und öffnen) und die .tex-Datei\r\n\
in einem beliebigen LaTeX-Editor öffnen (TeXstudio, TeXworks, VS Code …) oder das ZIP bei Overleaf\r\n\
hochladen (Neues Projekt → Projekt hochladen). Bilder und Literatur liegen daneben.\r\n\
Der Ordner visutex/ enthält nur Daten für VisuTeX (Ansicht, Versionsverlauf) und kann ignoriert werden.\r\n\
\r\n\
This file is a VisuTeX project in ZIP format. To edit it without VisuTeX, unzip it (or rename it\r\n\
to .zip) and open the .tex file in any LaTeX editor, or upload the ZIP to Overleaf. The visutex/\r\n\
folder only holds VisuTeX data (view state, version history) and can be ignored.\r\n";

/// Ist die Datei ein ZIP (Container) statt einer JSON-Projektdatei?
pub fn is_container(path: &Path) -> bool {
    let mut magic = [0_u8; 4];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_ok_and(|()| magic == *b"PK\x03\x04")
}

/// Bereits komprimierte Formate werden nur gespeichert („stored“) – spart Zeit beim Speichern.
fn compressed_format(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".pdf", ".zip", ".gz", ".mp4", ".svgz",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

/// Inhalt des Containers.
pub struct Contents<'a> {
    /// Dateiname des Hauptdokuments im Container, z. B. `Bachelorarbeit.tex`
    pub main_name: &'a str,
    pub latex: &'a str,
    /// `visutex/projekt.json`
    pub project_json: &'a str,
    /// Projektordner, aus dem `files` gelesen werden
    pub root: &'a Path,
    /// relative Pfade der Projektdateien
    pub files: &'a [String],
    /// Versionsverlauf (Ordner) – `None`: nicht mitspeichern
    pub versions: Option<&'a Path>,
}

/// Schreibt den Container atomar nach `path`. Gibt nicht gefundene Projektdateien zurück.
pub fn write(path: &Path, contents: &Contents) -> Result<Vec<String>, String> {
    let name = path
        .file_name()
        .ok_or("Ungültiger Dateiname.")?
        .to_string_lossy()
        .to_string();
    let temporary = path.with_file_name(format!(".~{name}.tmp"));
    let result = write_zip(&temporary, contents);
    let missing = match result {
        Ok(missing) => missing,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
    };
    files::replace_with(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!(
            "„{}“ konnte nicht gespeichert werden: {error}",
            path.display()
        )
    })?;
    Ok(missing)
}

fn write_zip(target: &Path, contents: &Contents) -> Result<Vec<String>, String> {
    let file = std::fs::File::create(target).map_err(|error| {
        format!(
            "„{}“ konnte nicht angelegt werden: {error}",
            target.display()
        )
    })?;
    let mut writer = zip::ZipWriter::new(file);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    let stored = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    let mut add = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let options = if compressed_format(name) {
            stored
        } else {
            deflated
        };
        writer
            .start_file(name, options)
            .and_then(|()| writer.write_all(bytes).map_err(Into::into))
            .map_err(|error| format!("Projektdatei konnte nicht geschrieben werden: {error}"))
    };
    // Hauptdokument zuerst – Overleaf und Entpackprogramme zeigen es dann oben
    add(contents.main_name, contents.latex.as_bytes())?;
    add("LIESMICH.txt", README.as_bytes())?;
    add(PROJECT_ENTRY, contents.project_json.as_bytes())?;
    let mut missing = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for relative in contents.files {
        let relative = relative.replace('\\', "/");
        if !crate::core::settings::is_safe_relative_path(&relative)
            || relative.eq_ignore_ascii_case(contents.main_name)
            || relative.starts_with("visutex/")
            || !seen.insert(relative.clone())
        {
            continue;
        }
        match std::fs::read(contents.root.join(&relative)) {
            Ok(bytes) => add(&relative, &bytes)?,
            Err(_) => missing.push(relative),
        }
    }
    if let Some(dir) = contents.versions {
        for path in crate::history::walk(dir) {
            let relative = path
                .strip_prefix(dir)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if relative.ends_with(".tmp") {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
            add(&format!("{VERSIONS_PREFIX}{relative}"), &bytes)?;
        }
    }
    let file = writer
        .finish()
        .map_err(|error| format!("Projektdatei konnte nicht abgeschlossen werden: {error}"))?;
    // erst auf die Platte, dann umbenennen – sonst kann nach einem Absturz eine leere Datei bleiben
    file.sync_all()
        .map_err(|error| format!("Projektdatei konnte nicht gesichert werden: {error}"))?;
    Ok(missing)
}

/// Entpackt den Container: Projektdateien nach `root` (vorher geleert), den Versionsverlauf nach
/// `versions` (nur wenn der Container einen enthält). Gibt `visutex/projekt.json` zurück.
pub fn extract(path: &Path, root: &Path, versions: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("„{}“ konnte nicht geöffnet werden: {error}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| "Die Projektdatei ist beschädigt (kein gültiges ZIP).".to_string())?;
    if archive.len() > MAX_ENTRIES {
        return Err("Die Projektdatei enthält zu viele Einträge.".into());
    }
    let mut project = String::new();
    archive
        .by_name(PROJECT_ENTRY)
        .map_err(|_| "Keine VisuTeX-Projektdatei (visutex/projekt.json fehlt).".to_string())?
        .read_to_string(&mut project)
        .map_err(|_| "visutex/projekt.json ist beschädigt.".to_string())?;
    let has_versions = archive
        .file_names()
        .any(|name| name.starts_with(VERSIONS_PREFIX));
    reset_dir(root)?;
    if has_versions {
        reset_dir(versions)?;
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let (base, relative) = if let Some(rest) = name.strip_prefix(VERSIONS_PREFIX) {
            (versions, rest.to_string())
        } else if name.starts_with("visutex/") {
            continue;
        } else {
            (root, name.clone())
        };
        if relative.is_empty()
            || Path::new(&relative)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("Unzulässiger Pfad in der Projektdatei: {name}"));
        }
        total += entry.size();
        if total > MAX_TOTAL {
            return Err("Die Projektdatei ist zu groß zum Entpacken.".into());
        }
        let target = base.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = std::fs::File::create(&target)
            .map_err(|error| format!("„{relative}“ konnte nicht entpackt werden: {error}"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("„{relative}“ konnte nicht entpackt werden: {error}"))?;
    }
    Ok(project)
}

/// Kopie zum Weitergeben: ohne Versionsverlauf (`keep_versions = false`) bzw. als reines
/// LaTeX-Projekt ohne VisuTeX-Daten (`keep_meta = false`, z. B. für Overleaf).
pub fn copy_filtered(
    source: &Path,
    target: &Path,
    keep_versions: bool,
    keep_meta: bool,
) -> Result<(), String> {
    let file = std::fs::File::open(source).map_err(|error| {
        format!(
            "„{}“ konnte nicht geöffnet werden: {error}",
            source.display()
        )
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| "Die Projektdatei ist beschädigt (kein gültiges ZIP).".to_string())?;
    let output = std::fs::File::create(target).map_err(|error| {
        format!(
            "„{}“ konnte nicht angelegt werden: {error}",
            target.display()
        )
    })?;
    let mut writer = zip::ZipWriter::new(output);
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| error.to_string())?;
        let name = entry.name().to_string();
        let skip = (!keep_versions && name.starts_with(VERSIONS_PREFIX))
            || (!keep_meta && (name.starts_with("visutex/") || name == "LIESMICH.txt"));
        if skip {
            continue;
        }
        // unverändert übernehmen (ohne neu zu komprimieren)
        writer
            .raw_copy_file(entry)
            .map_err(|error| format!("Kopie konnte nicht geschrieben werden: {error}"))?;
    }
    let file = writer
        .finish()
        .map_err(|error| format!("Kopie konnte nicht abgeschlossen werden: {error}"))?;
    file.sync_all().map_err(|error| error.to_string())?;
    Ok(())
}

/// Ordner leeren bzw. anlegen (nur App-eigene Arbeitsordner).
fn reset_dir(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(|error| format!("Arbeitsordner konnte nicht geleert werden: {error}"))?;
    }
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("Arbeitsordner konnte nicht angelegt werden: {error}"))
}

/// Dateiname des Hauptdokuments im Container: Name der Projektdatei mit `.tex`.
pub fn main_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or_else(|| "dokument".into());
    // nur für TeX unkritische Zeichen (Leerzeichen sind in TeX Live/Overleaf erlaubt)
    let safe: String = stem
        .chars()
        .map(|c| {
            if matches!(c, '{' | '}' | '%' | '#' | '$' | '&' | '~' | '^' | '\\') {
                '_'
            } else {
                c
            }
        })
        .collect();
    format!("{safe}.tex")
}

/// Arbeitsordner eines Containers im App-Datenordner (je Speicherort der Datei).
pub fn work_dir(local_data: &Path, path: &Path) -> PathBuf {
    local_data
        .join("projekte")
        .join(crate::history::path_key(&path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("visutex-container-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const LATEX: &str = "\\documentclass{article}\n\\begin{document}\n\\includegraphics{Abbildungen/bild.png}\n\\end{document}\n";

    #[test]
    fn writes_portable_zip_and_reads_it_back() {
        let dir = temp("rundweg");
        let project = dir.join("projekt");
        std::fs::create_dir_all(project.join("Abbildungen")).unwrap();
        std::fs::write(project.join("Abbildungen/bild.png"), b"PNG-Daten").unwrap();
        std::fs::write(project.join("literatur.bib"), "@book{a, title={T}}").unwrap();
        let versions = dir.join("verlauf");
        crate::history::snapshot(&versions, "doc", LATEX, &[], None, 1000).unwrap();

        let path = dir.join("Bachelorarbeit.visutex");
        let missing = write(
            &path,
            &Contents {
                main_name: &main_name(&path),
                latex: LATEX,
                project_json: "{\"format\":\"visutex-project\"}",
                root: &project,
                files: &[
                    "Abbildungen/bild.png".into(),
                    "literatur.bib".into(),
                    "fehlt.png".into(),
                    "../aussen.txt".into(),
                ],
                versions: Some(&versions),
            },
        )
        .unwrap();
        assert_eq!(missing, vec!["fehlt.png".to_string()]);
        assert!(is_container(&path));
        assert!(
            !dir.join(".~Bachelorarbeit.visutex.tmp").exists(),
            "keine Zwischendatei"
        );

        // wie ein normales ZIP lesbar: Hauptdokument zuerst, Bilder unter ihrem Pfad
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let names: Vec<String> = archive.file_names().map(str::to_string).collect();
        assert_eq!(names[0], "Bachelorarbeit.tex");
        assert!(names.contains(&"Abbildungen/bild.png".to_string()));
        assert!(names.contains(&"visutex/versionen/1000.txt".to_string()));
        assert!(!names.iter().any(|name| name.contains("aussen")));
        let mut tex = String::new();
        archive
            .by_name("Bachelorarbeit.tex")
            .unwrap()
            .read_to_string(&mut tex)
            .unwrap();
        assert_eq!(tex, LATEX);

        // entpacken
        let root = dir.join("arbeit");
        let restored = dir.join("arbeit-verlauf");
        let json = extract(&path, &root, &restored).unwrap();
        assert!(json.contains("visutex-project"));
        assert_eq!(
            std::fs::read(root.join("Abbildungen/bild.png")).unwrap(),
            b"PNG-Daten"
        );
        assert!(root.join("Bachelorarbeit.tex").is_file());
        assert!(
            !root.join("visutex").exists(),
            "VisuTeX-Daten nicht im Projektordner"
        );
        assert_eq!(crate::history::list(&restored).len(), 1);

        // Kopien zum Weitergeben
        let without = dir.join("ohne-verlauf.visutex");
        copy_filtered(&path, &without, false, true).unwrap();
        let archive = zip::ZipArchive::new(std::fs::File::open(&without).unwrap()).unwrap();
        assert!(!archive
            .file_names()
            .any(|name| name.starts_with(VERSIONS_PREFIX)));
        assert!(archive.file_names().any(|name| name == PROJECT_ENTRY));
        let plain = dir.join("overleaf.zip");
        copy_filtered(&path, &plain, false, false).unwrap();
        let archive = zip::ZipArchive::new(std::fs::File::open(&plain).unwrap()).unwrap();
        assert!(!archive
            .file_names()
            .any(|name| name.starts_with("visutex/") || name == "LIESMICH.txt"));
        assert!(archive
            .file_names()
            .any(|name| name == "Bachelorarbeit.tex"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_unsafe_and_foreign_zips() {
        let dir = temp("fremd");
        let evil = dir.join("boese.visutex");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file(PROJECT_ENTRY, options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.start_file("../ausserhalb.txt", options).unwrap();
        writer.write_all(b"x").unwrap();
        writer.finish().unwrap();
        assert!(extract(&evil, &dir.join("a"), &dir.join("b")).is_err());
        assert!(!dir.join("ausserhalb.txt").exists());

        let foreign = dir.join("fremd.visutex");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&foreign).unwrap());
        writer.start_file("irgendwas.txt", options).unwrap();
        writer.finish().unwrap();
        assert!(extract(&foreign, &dir.join("c"), &dir.join("d"))
            .unwrap_err()
            .contains("projekt.json"));
        assert!(!is_container(&dir.join("gibt-es-nicht")));
        assert_eq!(
            main_name(Path::new("C:/x/Mein Bericht.visutex")),
            "Mein Bericht.tex"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
