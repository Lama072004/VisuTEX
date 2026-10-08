//! In die EXE eingebettete Dateien (TeX-Bundle, mitgelieferte Add-ons).
//!
//! `main.rs` bettet die Dateien per `include_bytes!` ein und übergibt sie an [`crate::run_with`] –
//! so bleibt die Bibliothek (und jeder Test) klein. Beim ersten Bedarf werden sie einmalig in den
//! Cache-Ordner der App geschrieben; danach arbeitet alles wie mit normalen Dateien. Damit genügt
//! die EXE allein: kein Ordner `resources` daneben nötig.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Eingebettete Dateien; leer, wenn die App ohne sie gestartet wurde (Entwicklung, Tests).
#[derive(Clone, Copy, Default)]
pub struct EmbeddedAssets {
    /// `resources/tex-bundle.zip`
    pub tex_bundle: &'static [u8],
    /// Dateien unter `resources/addons` als (relativer Pfad mit `/`, Inhalt)
    pub addons: &'static [(&'static str, &'static [u8])],
}

static ASSETS: OnceLock<EmbeddedAssets> = OnceLock::new();
static BUNDLE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static ADDONS_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

pub fn install(assets: EmbeddedAssets) {
    let _ = ASSETS.set(assets);
}

fn assets() -> EmbeddedAssets {
    ASSETS.get().copied().unwrap_or_default()
}

/// Unterordner im App-Cache, je Programmversion (alte Versionen werden aufgeräumt).
fn target_dir(cache_dir: &Path, name: &str) -> PathBuf {
    cache_dir
        .join("mitgeliefert")
        .join(format!("{name}-{}", env!("CARGO_PKG_VERSION")))
}

/// Schreibt `bytes` nach `path`, falls die Datei fehlt oder eine andere Größe hat (atomar über eine Zwischendatei).
fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() == bytes.len() as u64) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

/// Entfernt Ordner älterer Programmversionen (`name-<alte Version>`).
fn remove_old_versions(cache_dir: &Path, name: &str) {
    let base = cache_dir.join("mitgeliefert");
    let current = format!("{name}-{}", env!("CARGO_PKG_VERSION"));
    let Ok(entries) = std::fs::read_dir(&base) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with(&format!("{name}-")) && file_name != current {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Pfad des eingebetteten TeX-Bundles (beim ersten Aufruf entpackt); `None` ohne Einbettung.
pub fn tex_bundle(cache_dir: &Path) -> Option<PathBuf> {
    BUNDLE_PATH
        .get_or_init(|| {
            let bytes = assets().tex_bundle;
            if bytes.is_empty() {
                return None;
            }
            remove_old_versions(cache_dir, "tex");
            let path = target_dir(cache_dir, "tex").join(crate::texbundle::BUNDLE_FILE_NAME);
            write_if_changed(&path, bytes).ok().map(|()| path)
        })
        .clone()
}

/// Ordner der eingebetteten Add-ons (beim ersten Aufruf entpackt); `None` ohne Einbettung.
pub fn addons(cache_dir: &Path) -> Option<PathBuf> {
    ADDONS_PATH
        .get_or_init(|| {
            let files = assets().addons;
            if files.is_empty() {
                return None;
            }
            remove_old_versions(cache_dir, "addons");
            let dir = target_dir(cache_dir, "addons");
            for (relative, bytes) in files {
                // nur sichere relative Pfade
                if relative
                    .split('/')
                    .any(|part| part.is_empty() || part == "..")
                {
                    continue;
                }
                if write_if_changed(&dir.join(relative), bytes).is_err() {
                    return None;
                }
            }
            Some(dir)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_files_once_and_skips_unsafe_paths() {
        let dir = std::env::temp_dir().join(format!("visutex-embedded-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("a").join("b.txt");
        write_if_changed(&path, b"hallo").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hallo");
        // gleiche Größe → nicht neu geschrieben, andere Größe → ersetzt
        write_if_changed(&path, b"welt!").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hallo");
        write_if_changed(&path, b"neuer Inhalt").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"neuer Inhalt");
        // alte Versionen aufräumen
        std::fs::create_dir_all(dir.join("mitgeliefert").join("tex-0.0.1")).unwrap();
        remove_old_versions(&dir, "tex");
        assert!(!dir.join("mitgeliefert").join("tex-0.0.1").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
