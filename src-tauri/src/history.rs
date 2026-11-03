//! Versionsverlauf: Bei jedem Speichern legt VisuTeX eine Fassung des Dokuments (vollständiger
//! LaTeX-Code) im Datenordner ab – wie der Versionsverlauf in Word. Benannte Versionen bleiben
//! erhalten, von automatischen höchstens `MAX_AUTO` (die ältesten fallen weg). Der
//! Projektordner des Nutzers bleibt unberührt.
//!
//! Ablage: `<Datenordner>/versionen/<Prüfsumme des Dokumentpfads>/` mit `index.json` und je
//! Version `<id>.tex`.

use crate::core::diff;
use crate::files;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Höchstzahl automatischer Versionen je Dokument.
pub const MAX_AUTO: usize = 100;

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
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    document: String,
    /// älteste zuerst
    #[serde(default)]
    versions: Vec<VersionInfo>,
}

/// Ordner der Versionen eines Dokuments (Pfad unter Windows ohne Groß-/Kleinschreibung).
pub fn store_dir(data_dir: &Path, document: &str) -> PathBuf {
    let mut normalized = document.trim().replace('\\', "/");
    if cfg!(windows) {
        normalized = normalized.to_lowercase();
    }
    let hash = Sha256::digest(normalized.as_bytes());
    let key: String = hash
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    data_dir.join("versionen").join(key)
}

fn load_index(dir: &Path) -> Index {
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_index(dir: &Path, index: &Index) -> Result<(), String> {
    let text = serde_json::to_string_pretty(index).map_err(|error| error.to_string())?;
    files::write_text_atomic(&dir.join("index.json"), &text)
}

/// Kennungen sind Zeitstempel – nur Ziffern (kein Pfad außerhalb des Ordners).
fn version_file(dir: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Ungültige Versionskennung „{id}“."));
    }
    Ok(dir.join(format!("{id}.tex")))
}

/// Dokumentkörper (für die Wortzahl); ohne `\begin{document}` der ganze Text.
fn body(latex: &str) -> &str {
    let start = latex
        .find("\\begin{document}")
        .map_or(0, |index| index + "\\begin{document}".len());
    let end = latex[start..]
        .find("\\end{document}")
        .map_or(latex.len(), |index| start + index);
    &latex[start..end]
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

/// Legt eine Version an. Ohne Namen und bei unverändertem Stand passiert nichts (`None`);
/// mit Namen wird dann die letzte Version benannt statt doppelt abgelegt.
pub fn snapshot(
    dir: &Path,
    document: &str,
    latex: &str,
    name: Option<&str>,
    now_ms: u64,
) -> Result<Option<VersionInfo>, String> {
    let mut index = load_index(dir);
    index.document = document.to_string();
    let name = name.map(str::trim).unwrap_or("").to_string();
    let unchanged = index
        .versions
        .last()
        .is_some_and(|last| read(dir, &last.id).is_ok_and(|text| text == latex));
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
    let info = VersionInfo {
        id: created.to_string(),
        created,
        name,
        words: diff::word_count(body(latex)),
        size: latex.len(),
    };
    files::write_text_atomic(&version_file(dir, &info.id)?, latex)?;
    index.versions.push(info.clone());
    // älteste automatische Versionen entfernen
    let mut automatic = index.versions.iter().filter(|v| v.name.is_empty()).count();
    let mut kept = Vec::with_capacity(index.versions.len());
    for version in std::mem::take(&mut index.versions) {
        if automatic > MAX_AUTO && version.name.is_empty() {
            automatic -= 1;
            if let Ok(file) = version_file(dir, &version.id) {
                let _ = std::fs::remove_file(file);
            }
            continue;
        }
        kept.push(version);
    }
    index.versions = kept;
    save_index(dir, &index)?;
    Ok(Some(info))
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
    let file = version_file(dir, id)?;
    let mut index = load_index(dir);
    index.versions.retain(|version| version.id != id);
    save_index(dir, &index)?;
    if file.is_file() {
        std::fs::remove_file(&file)
            .map_err(|error| format!("Version konnte nicht gelöscht werden: {error}"))?;
    }
    Ok(())
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

        let first = snapshot(&dir, "Dokument", DOC, None, 1000)
            .unwrap()
            .unwrap();
        assert_eq!(first.words, 2);
        // unverändert → keine neue Version
        assert_eq!(snapshot(&dir, "Dokument", DOC, None, 2000).unwrap(), None);
        // unverändert mit Namen → letzte Version wird benannt
        let named = snapshot(&dir, "Dokument", DOC, Some(" Abgabe "), 3000)
            .unwrap()
            .unwrap();
        assert_eq!((named.id.as_str(), named.name.as_str()), ("1000", "Abgabe"));
        let changed = DOC.replace("Welt", "liebe Welt");
        let second = snapshot(&dir, "Dokument", &changed, None, 1000)
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
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn keeps_named_versions_when_pruning() {
        let data = temp("aufraeumen");
        let dir = store_dir(&data, "/home/nutzer/arbeit.tex");
        snapshot(&dir, "arbeit", "benannt", Some("Meilenstein"), 1).unwrap();
        for step in 0..MAX_AUTO + 5 {
            snapshot(
                &dir,
                "arbeit",
                &format!("Stand {step}"),
                None,
                10 + step as u64,
            )
            .unwrap();
        }
        let versions = list(&dir);
        assert_eq!(versions.len(), MAX_AUTO + 1);
        assert!(versions.iter().any(|version| version.name == "Meilenstein"));
        // die ältesten automatischen (Stand 0–4) sind weg, auch als Datei
        assert!(read(&dir, "10").is_err());
        assert!(!version_file(&dir, "10").unwrap().exists());
        assert_eq!(
            read(&dir, &versions[0].id).unwrap(),
            format!("Stand {}", MAX_AUTO + 4)
        );
        let _ = std::fs::remove_dir_all(&data);
    }
}
