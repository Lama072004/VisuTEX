//! Beim Start übergebene Dateien – Doppelklick im Explorer bzw. Dateimanager oder „Öffnen mit“.
//! Add-on-Dateien (`.vtxaddon`) bietet die Oberfläche zur Installation an, Dokumente und
//! Präsentationen öffnet sie.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

/// Dateiendungen, die VisuTeX beim Start annimmt.
const EXTENSIONS: &[&str] = &["vtxaddon", "visutex", "vtxslides", "tex"];

static PENDING: Mutex<Option<Vec<PathBuf>>> = Mutex::new(None);

/// Vorhandene Dateien mit bekannter Endung aus der Befehlszeile (ohne Programmpfad und Schalter
/// wie `--uninstall`), als absolute Pfade.
pub fn file_arguments(args: impl IntoIterator<Item = OsString>) -> Vec<PathBuf> {
    args.into_iter()
        .skip(1)
        .map(PathBuf::from)
        .filter(|path| !path.to_string_lossy().starts_with('-'))
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                })
        })
        .filter(|path| path.is_file())
        .map(|path| std::path::absolute(&path).unwrap_or(path))
        .collect()
}

/// Beim Programmstart aufrufen.
pub fn remember(args: impl IntoIterator<Item = OsString>) {
    if let Ok(mut pending) = PENDING.lock() {
        *pending = Some(file_arguments(args));
    }
}

/// Übergebene Dateien – nur einmal (ein Neuladen der Oberfläche öffnet sie nicht erneut).
pub fn take() -> Vec<PathBuf> {
    PENDING
        .lock()
        .ok()
        .and_then(|mut pending| pending.take())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_existing_files_with_known_extensions() {
        let dir = std::env::temp_dir().join(format!("visutex-start-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let addon = dir.join("Mein Add-on.VTXADDON");
        let document = dir.join("bericht.visutex");
        let other = dir.join("bild.png");
        for file in [&addon, &document, &other] {
            std::fs::write(file, "x").unwrap();
        }
        let args = [
            OsString::from("visutex.exe"),
            OsString::from("--uninstall"),
            addon.clone().into_os_string(),
            document.clone().into_os_string(),
            other.into_os_string(),
            dir.join("fehlt.vtxaddon").into_os_string(),
        ];
        let files = file_arguments(args.clone());
        assert_eq!(files, vec![addon, document]);
        assert!(files.iter().all(|file| file.is_absolute()));

        remember(args);
        assert_eq!(take().len(), 2);
        assert!(take().is_empty(), "nur einmal");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
