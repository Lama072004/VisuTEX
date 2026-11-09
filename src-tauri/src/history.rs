//! Versionsverlauf: Bei jedem Speichern legt VisuTeX eine Fassung des Dokuments ab – den
//! vollständigen LaTeX-Code und die Projektdateien (Bilder, Literatur …), wie der
//! Versionsverlauf in Word. Dateien werden inhaltsadressiert (SHA-256) und damit nur einmal
//! gespeichert, auch wenn sie in vielen Versionen vorkommen.
//!
//! Aufbewahrung: benannte Versionen immer; automatische alle der letzten 24 Stunden, davor die
//! letzte je Tag – viele Strg+S löschen so nie den Stand von gestern.
//!
//! Ablage (ein Ordner je Dokument, Ort wählbar – siehe `lib.rs`): `index.json`, je Version
//! `<id>.txt` (LaTeX; `.txt`, damit Overleaf beim Hochladen eines Projekt-ZIPs keine alte
//! Version für das Hauptdokument hält) und `dateien/<sha256>`.

use crate::core::diff;
use crate::files;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Automatische Versionen der letzten 24 Stunden bleiben alle (höchstens so viele).
pub const MAX_RECENT: usize = 200;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    /// Zeitpunkt in Millisekunden seit 1970 (UTC)
    pub created: u64,
    /// leer: automatisch beim Speichern angelegt
    #[serde(default)]
    pub name: String,
    /// Wörter im Dokumentkörper
    #[serde(default)]
    pub words: usize,
    /// Größe des LaTeX-Codes in Bytes
    #[serde(default)]
    pub size: usize,
    /// Projektdateien dieser Version: relativer Pfad → SHA-256
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    document: String,
    /// älteste zuerst
    #[serde(default)]
    versions: Vec<VersionInfo>,
}

/// Ordnername aus einem Dokumentpfad (Prüfsumme; unter Windows ohne Groß-/Kleinschreibung).
pub fn path_key(document: &str) -> String {
    let mut normalized = document.trim().replace('\\', "/");
    if cfg!(windows) {
        normalized = normalized.to_lowercase();
    }
    Sha256::digest(normalized.as_bytes())
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Ordner der Versionen eines Dokuments im App-Datenordner.
pub fn store_dir(data_dir: &Path, document: &str) -> PathBuf {
    data_dir.join("versionen").join(path_key(document))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Kennungen sind Zeitstempel – nur Ziffern (kein Pfad außerhalb des Ordners).
fn version_file(dir: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Ungültige Versionskennung „{id}“."));
    }
    Ok(dir.join(format!("{id}.txt")))
}

fn blob_file(dir: &Path, hash: &str) -> Result<PathBuf, String> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Ungültige Prüfsumme im Versionsverlauf.".into());
    }
    Ok(dir.join("dateien").join(hash))
}

/// Index lesen; fehlt er oder ist er beschädigt, aus den Versionsdateien neu aufbauen.
fn load_index(dir: &Path) -> Index {
    if let Some(index) = std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Index>(&text).ok())
    {
        return index;
    }
    let mut versions: Vec<VersionInfo> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let id = name.strip_suffix(".txt")?.to_string();
            let created = id.parse::<u64>().ok()?;
            let latex = files::read_text(&entry.path()).ok()?;
            Some(VersionInfo {
                id,
                created,
                name: String::new(),
                words: diff::word_count(&body(&latex)),
                size: latex.len(),
                files: BTreeMap::new(),
            })
        })
        .collect();
    versions.sort_by_key(|version| version.created);
    Index {
        document: String::new(),
        versions,
    }
}

fn save_index(dir: &Path, index: &Index) -> Result<(), String> {
    let text = serde_json::to_string_pretty(index).map_err(|error| error.to_string())?;
    files::write_text_atomic(&dir.join("index.json"), &text)
}

/// Text für die Wortzahl: Dokumentkörper (ohne `\begin{document}` der ganze Text) ohne Kommentare
/// (z. B. Modellzeilen von Skizzen/Diagrammen) und ohne Zeichnungscode (TikZ-Koordinaten).
fn body(latex: &str) -> String {
    let start = latex
        .find("\\begin{document}")
        .map_or(0, |index| index + "\\begin{document}".len());
    let end = latex[start..]
        .find("\\end{document}")
        .map_or(latex.len(), |index| start + index);
    let mut text = String::new();
    let mut drawing = 0_usize;
    for line in latex[start..end].lines() {
        // Kommentar ab unmaskiertem %
        let mut cut = line.len();
        let mut escaped = false;
        for (index, c) in line.char_indices() {
            if c == '%' && !escaped {
                cut = index;
                break;
            }
            escaped = c == '\\' && !escaped;
        }
        let line = &line[..cut];
        for environment in ["tikzpicture", "circuitikz"] {
            drawing += line.matches(&format!("\\begin{{{environment}}}")).count();
        }
        if drawing == 0 {
            text.push_str(line);
            text.push('\n');
        }
        for environment in ["tikzpicture", "circuitikz"] {
            drawing =
                drawing.saturating_sub(line.matches(&format!("\\end{{{environment}}}")).count());
        }
    }
    text
}

/// Versionen, neueste zuerst (ohne Einträge, deren Datei fehlt).
pub fn list(dir: &Path) -> Vec<VersionInfo> {
    let mut versions: Vec<VersionInfo> = load_index(dir)
        .versions
        .into_iter()
        .filter(|version| version_file(dir, &version.id).is_ok_and(|file| file.is_file()))
        .collect();
    versions.reverse();
    versions
}

pub fn read(dir: &Path, id: &str) -> Result<String, String> {
    let file = version_file(dir, id)?;
    if !file.is_file() {
        return Err("Diese Version gibt es nicht mehr.".into());
    }
    files::read_text(&file)
}

/// Legt eine Version an (`files`: relativer Pfad → Datei auf der Platte). Ohne Namen und bei
/// unverändertem Stand passiert nichts (`None`); mit Namen wird dann die letzte Version benannt.
pub fn snapshot(
    dir: &Path,
    document: &str,
    latex: &str,
    files: &[(String, PathBuf)],
    name: Option<&str>,
    now_ms: u64,
) -> Result<Option<VersionInfo>, String> {
    let mut index = load_index(dir);
    index.document = document.to_string();
    let name = name.map(str::trim).unwrap_or("").to_string();
    // Dateien lesen und als Prüfsummen erfassen (fehlende Dateien werden übergangen)
    let mut contents: BTreeMap<String, (String, Vec<u8>)> = BTreeMap::new();
    for (relative, path) in files {
        if let Ok(bytes) = std::fs::read(path) {
            contents.insert(relative.clone(), (sha256_hex(&bytes), bytes));
        }
    }
    let hashes: BTreeMap<String, String> = contents
        .iter()
        .map(|(path, (hash, _))| (path.clone(), hash.clone()))
        .collect();
    let unchanged = index.versions.last().is_some_and(|last| {
        last.files == hashes && read(dir, &last.id).is_ok_and(|text| text == latex)
    });
    if unchanged {
        if name.is_empty() {
            return Ok(None);
        }
        let Some(last) = index.versions.last_mut() else {
            return Ok(None);
        };
        last.name = name;
        let info = last.clone();
        save_index(dir, &index)?;
        return Ok(Some(info));
    }
    let mut created = now_ms;
    while index
        .versions
        .iter()
        .any(|version| version.id == created.to_string())
    {
        created += 1;
    }
    // erst die Inhalte, dann der Index: ein Absturz hinterlässt höchstens verwaiste Dateien
    for (hash, bytes) in contents.values() {
        let blob = blob_file(dir, hash)?;
        if !blob.is_file() {
            files::write_bytes_atomic(&blob, bytes)?;
        }
    }
    let info = VersionInfo {
        id: created.to_string(),
        created,
        name,
        words: diff::word_count(&body(latex)),
        size: latex.len(),
        files: hashes,
    };
    files::write_text_atomic(&version_file(dir, &info.id)?, latex)?;
    index.versions.push(info.clone());
    prune(&mut index, now_ms);
    save_index(dir, &index)?;
    collect_garbage(dir, &index);
    Ok(Some(info))
}

/// Aufbewahrung: benannte immer; automatische der letzten 24 h (höchstens `MAX_RECENT`), davor die
/// letzte je Tag.
fn prune(index: &mut Index, now_ms: u64) {
    let recent_limit = now_ms.saturating_sub(DAY_MS);
    let mut recent_kept = 0;
    let mut days = BTreeSet::new();
    // neueste zuerst durchgehen
    let mut keep: Vec<bool> = vec![true; index.versions.len()];
    for (position, version) in index.versions.iter().enumerate().rev() {
        if !version.name.is_empty() {
            continue;
        }
        if version.created >= recent_limit && recent_kept < MAX_RECENT {
            recent_kept += 1;
        } else if !days.insert(version.created / DAY_MS) {
            keep[position] = false;
        }
    }
    let mut position = 0;
    index.versions.retain(|_| {
        position += 1;
        keep[position - 1]
    });
}

/// Versionstexte und Dateien löschen, die zu keiner Version im Index mehr gehören.
fn collect_garbage(dir: &Path, index: &Index) {
    let ids: BTreeSet<String> = index.versions.iter().map(|v| v.id.clone()).collect();
    let hashes: BTreeSet<&String> = index
        .versions
        .iter()
        .flat_map(|v| v.files.values())
        .collect();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(id) = name.strip_suffix(".txt") {
            if !ids.contains(id) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    for entry in std::fs::read_dir(dir.join("dateien"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !hashes.contains(&name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Projektdateien einer Version in `root` zurückschreiben, soweit sie fehlen oder abweichen.
/// Gibt die geschriebenen Pfade zurück.
pub fn restore_files(dir: &Path, id: &str, root: &Path) -> Result<Vec<String>, String> {
    let index = load_index(dir);
    let version = index
        .versions
        .iter()
        .find(|version| version.id == id)
        .ok_or("Diese Version gibt es nicht mehr.")?;
    let mut written = Vec::new();
    for (relative, hash) in &version.files {
        if !crate::core::settings::is_safe_relative_path(relative) {
            continue;
        }
        let target = root.join(relative);
        let current = std::fs::read(&target).ok().map(|bytes| sha256_hex(&bytes));
        if current.as_deref() == Some(hash.as_str()) {
            continue;
        }
        let Ok(bytes) = std::fs::read(blob_file(dir, hash)?) else {
            continue;
        };
        files::write_bytes_atomic(&target, &bytes)?;
        written.push(relative.clone());
    }
    Ok(written)
}

/// Benennt eine Version (leerer Name: wieder „automatisch“).
pub fn rename(dir: &Path, id: &str, name: &str) -> Result<VersionInfo, String> {
    let mut index = load_index(dir);
    let version = index
        .versions
        .iter_mut()
        .find(|version| version.id == id)
        .ok_or("Diese Version gibt es nicht mehr.")?;
    version.name = name.trim().to_string();
    let info = version.clone();
    save_index(dir, &index)?;
    Ok(info)
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    version_file(dir, id)?;
    let mut index = load_index(dir);
    index.versions.retain(|version| version.id != id);
    save_index(dir, &index)?;
    collect_garbage(dir, &index);
    Ok(())
}

/// Ganzen Verlauf kopieren (*Speichern unter* nimmt den Verlauf mit).
pub fn copy_store(from: &Path, to: &Path) -> Result<(), String> {
    if !from.is_dir() || from == to {
        return Ok(());
    }
    for entry in walk(from) {
        let relative = entry
            .strip_prefix(from)
            .map_err(|error| error.to_string())?;
        let target = to.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::copy(&entry, &target)
            .map_err(|error| format!("Versionsverlauf konnte nicht kopiert werden: {error}"))?;
    }
    Ok(())
}

/// Alle Dateien unterhalb von `dir` (rekursiv).
pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                result.push(path);
            }
        }
    }
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("visutex-versionen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    const DOC: &str = "\\documentclass{article}\n\\begin{document}\nHallo Welt\n\\end{document}\n";

    #[test]
    fn stores_lists_and_reads_versions() {
        let data = temp("liste");
        let dir = store_dir(&data, "C:\\Arbeit\\Dokument.visutex");
        if cfg!(windows) {
            assert_eq!(dir, store_dir(&data, "c:/arbeit/dokument.visutex"));
        }
        assert_ne!(dir, store_dir(&data, "C:/Arbeit/Anderes.visutex"));
        assert!(list(&dir).is_empty());

        let first = snapshot(&dir, "Dokument", DOC, &[], None, 1000)
            .unwrap()
            .unwrap();
        assert_eq!(first.words, 2);
        assert!(dir.join("1000.txt").is_file(), "Versionstext als .txt");
        // unverändert → keine neue Version
        assert_eq!(
            snapshot(&dir, "Dokument", DOC, &[], None, 2000).unwrap(),
            None
        );
        // unverändert mit Namen → letzte Version wird benannt
        let named = snapshot(&dir, "Dokument", DOC, &[], Some(" Abgabe "), 3000)
            .unwrap()
            .unwrap();
        assert_eq!((named.id.as_str(), named.name.as_str()), ("1000", "Abgabe"));
        let changed = DOC.replace("Welt", "liebe Welt");
        let second = snapshot(&dir, "Dokument", &changed, &[], None, 1000)
            .unwrap()
            .unwrap();
        assert_eq!(second.id, "1001", "eindeutige Kennung");
        let versions = list(&dir);
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id, "1001", "neueste zuerst");
        assert_eq!(read(&dir, "1001").unwrap(), changed);

        assert_eq!(rename(&dir, "1001", "Entwurf").unwrap().name, "Entwurf");
        delete(&dir, "1000").unwrap();
        assert_eq!(list(&dir).len(), 1);
        assert!(read(&dir, "1000").is_err());
        assert!(read(&dir, "../index").is_err(), "keine Pfade außerhalb");
        // beschädigter Index → aus den Versionsdateien neu aufgebaut
        std::fs::write(dir.join("index.json"), "{ kaputt").unwrap();
        assert_eq!(list(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn word_count_ignores_comments_and_drawings() {
        let latex = "\\begin{document}\nZwei Wörter % Kommentar zählt nicht\n\\begin{tikzpicture}\n% VisuTeX-Diagramm: {\"a\": 1}\n\\draw (0,0) -- (2,3);\n\\end{tikzpicture}\n100 \\% sicher\n\\end{document}";
        assert_eq!(diff::word_count(&body(latex)), 4);
    }

    #[test]
    fn keeps_recent_named_and_one_per_day() {
        let data = temp("aufraeumen");
        let dir = store_dir(&data, "/home/nutzer/arbeit.tex");
        let now = 100 * DAY_MS;
        snapshot(
            &dir,
            "arbeit",
            "benannt",
            &[],
            Some("Meilenstein"),
            10 * DAY_MS,
        )
        .unwrap();
        // vor 3 Tagen viermal gespeichert, vor 2 Tagen zweimal, heute dreimal
        let mut step = 0;
        for time in [
            now - 3 * DAY_MS,
            now - 3 * DAY_MS + 1000,
            now - 3 * DAY_MS + 2000,
            now - 3 * DAY_MS + 3000,
            now - 2 * DAY_MS,
            now - 2 * DAY_MS + 5000,
            now - 3000,
            now - 2000,
            now - 1000,
        ] {
            step += 1;
            snapshot(&dir, "arbeit", &format!("Stand {step}"), &[], None, time).unwrap();
        }
        let versions = list(&dir);
        // Meilenstein + je Tag die letzte (2) + heute alle (3)
        assert_eq!(versions.len(), 6, "{versions:?}");
        assert!(versions.iter().any(|version| version.name == "Meilenstein"));
        assert_eq!(read(&dir, &versions[0].id).unwrap(), "Stand 9");
        assert!(versions
            .iter()
            .any(|version| read(&dir, &version.id).unwrap() == "Stand 4"));
        assert!(!versions
            .iter()
            .any(|version| read(&dir, &version.id).unwrap() == "Stand 1"));
        assert!(
            !dir.join(format!("{}.txt", now - 3 * DAY_MS)).exists(),
            "Datei entfernt"
        );
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn versions_keep_project_files_deduplicated() {
        let data = temp("dateien");
        let project = data.join("projekt");
        std::fs::create_dir_all(project.join("Abbildungen")).unwrap();
        let image = project.join("Abbildungen/bild.png");
        std::fs::write(&image, b"Bild 1").unwrap();
        let dir = data.join("verlauf");
        let files = vec![("Abbildungen/bild.png".to_string(), image.clone())];
        let first = snapshot(&dir, "doc", DOC, &files, None, 1000)
            .unwrap()
            .unwrap();
        // gleiche Datei in zweiter Version → nur einmal gespeichert
        snapshot(
            &dir,
            "doc",
            &DOC.replace("Welt", "Leute"),
            &files,
            None,
            2000,
        )
        .unwrap();
        assert_eq!(walk(&dir.join("dateien")).len(), 1);
        // nur das Bild geändert → neue Version
        std::fs::write(&image, b"Bild 2").unwrap();
        let third = snapshot(
            &dir,
            "doc",
            &DOC.replace("Welt", "Leute"),
            &files,
            None,
            3000,
        )
        .unwrap()
        .expect("geändertes Bild ergibt eine Version");
        assert_ne!(third.files, first.files);
        assert_eq!(walk(&dir.join("dateien")).len(), 2);
        // erste Version wiederherstellen → altes Bild zurück
        assert_eq!(
            restore_files(&dir, &first.id, &project).unwrap(),
            vec!["Abbildungen/bild.png".to_string()]
        );
        assert_eq!(std::fs::read(&image).unwrap(), b"Bild 1");
        assert!(restore_files(&dir, &first.id, &project).unwrap().is_empty());
        // Verlauf kopieren (Speichern unter)
        let copy = data.join("kopie");
        copy_store(&dir, &copy).unwrap();
        assert_eq!(list(&copy).len(), 3);
        let _ = std::fs::remove_dir_all(&data);
    }
}
