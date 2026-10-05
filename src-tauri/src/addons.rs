//! Add-on-Verwaltung: Installieren (Ordner oder ZIP), Erstellen, Aktivieren,
//! Entfernen und Exportieren. Ablage: `<App-Daten>/addons/<id>/addon.json`.
//! Mitgelieferte Beispiel-Add-ons liegen in den App-Ressourcen (`addons/`).

use crate::core::addon::{is_valid_id, parse_manifest, validate, AddonManifest};
use crate::core::settings::is_safe_relative_path;
use crate::files::write_text_atomic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const MANIFEST: &str = "addon.json";
const STATE_FILE: &str = "addons-state.json";
const MAX_ZIP_SIZE: u64 = 200 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddon {
    pub manifest: AddonManifest,
    pub enabled: bool,
    pub builtin: bool,
    pub path: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AddonState {
    disabled: BTreeSet<String>,
    /// Mitgelieferte Add-ons sind standardmäßig aus und werden hier eingeschaltet.
    enabled: BTreeSet<String>,
}

pub struct AddonDirs {
    /// Vom Nutzer installierte Add-ons (App-Datenordner).
    pub user: PathBuf,
    /// Mitgelieferte Add-ons (Ressourcen, schreibgeschützt).
    pub builtin: Option<PathBuf>,
}

impl AddonDirs {
    fn state_path(&self) -> PathBuf {
        self.user.join(STATE_FILE)
    }

    fn state(&self) -> AddonState {
        std::fs::read_to_string(self.state_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save_state(&self, state: &AddonState) -> Result<(), String> {
        let text = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
        write_text_atomic(&self.state_path(), &text)
    }

    fn scan(dir: &Path, builtin: bool, state: &AddonState, result: &mut Vec<InstalledAddon>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut folders: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.join(MANIFEST).is_file())
            .collect();
        folders.sort();
        for folder in folders {
            let Ok(text) = std::fs::read_to_string(folder.join(MANIFEST)) else {
                continue;
            };
            let Ok(manifest) = parse_manifest(&text) else {
                continue;
            };
            if result.iter().any(|known| known.manifest.id == manifest.id) {
                continue; // Nutzer-Add-on mit gleicher ID hat Vorrang.
            }
            result.push(InstalledAddon {
                enabled: if builtin {
                    state.enabled.contains(&manifest.id)
                } else {
                    !state.disabled.contains(&manifest.id)
                },
                manifest,
                builtin,
                path: folder.display().to_string(),
            });
        }
    }

    pub fn list(&self) -> Vec<InstalledAddon> {
        let state = self.state();
        let mut result = Vec::new();
        Self::scan(&self.user, false, &state, &mut result);
        if let Some(builtin) = &self.builtin {
            Self::scan(builtin, true, &state, &mut result);
        }
        result
    }

    pub fn enabled(&self) -> Vec<InstalledAddon> {
        self.list()
            .into_iter()
            .filter(|addon| addon.enabled)
            .collect()
    }

    pub fn find(&self, id: &str) -> Result<InstalledAddon, String> {
        self.list()
            .into_iter()
            .find(|addon| addon.manifest.id == id)
            .ok_or_else(|| format!("Add-on „{id}“ ist nicht installiert."))
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        self.find(id)?;
        let mut state = self.state();
        if enabled {
            state.disabled.remove(id);
            state.enabled.insert(id.to_string());
        } else {
            state.disabled.insert(id.to_string());
            state.enabled.remove(id);
        }
        self.save_state(&state)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let addon = self.find(id)?;
        if addon.builtin {
            return Err("Mitgelieferte Add-ons können nur deaktiviert werden.".into());
        }
        let path = PathBuf::from(&addon.path);
        if !path.starts_with(&self.user) {
            return Err("Add-on liegt außerhalb des Add-on-Ordners.".into());
        }
        std::fs::remove_dir_all(&path)
            .map_err(|error| format!("Add-on konnte nicht entfernt werden: {error}"))
    }

    /// Präambelzeilen aller aktivierten Add-ons.
    pub fn preamble(&self) -> Vec<String> {
        self.enabled()
            .into_iter()
            .flat_map(|addon| addon.manifest.preamble)
            .collect()
    }

    /// Ordner mit TeX-Dateien aktivierter Add-ons (Tectonic-Suchpfade).
    pub fn search_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for addon in self.enabled() {
            let root = PathBuf::from(&addon.path);
            for file in &addon.manifest.tex_files {
                if let Some(parent) = root.join(file).parent() {
                    let parent = parent.to_path_buf();
                    if parent.is_dir() && !paths.contains(&parent) {
                        paths.push(parent);
                    }
                }
            }
        }
        paths
    }

    /// TeX-Dateien aktivierter Add-ons (für den .tex-Export: werden mitkopiert).
    pub fn tex_files(&self) -> Vec<PathBuf> {
        self.enabled()
            .into_iter()
            .flat_map(|addon| {
                let root = PathBuf::from(&addon.path);
                addon
                    .manifest
                    .tex_files
                    .into_iter()
                    .map(move |file| root.join(file))
            })
            .filter(|path| path.is_file())
            .collect()
    }

    fn target_for(&self, id: &str) -> PathBuf {
        self.user.join(id)
    }

    /// Installiert aus einem Ordner mit `addon.json`.
    pub fn install_folder(&self, source: &Path) -> Result<AddonManifest, String> {
        let manifest_text = std::fs::read_to_string(source.join(MANIFEST))
            .map_err(|_| "Im gewählten Ordner fehlt die Datei addon.json.".to_string())?;
        let manifest = parse_manifest(&manifest_text)?;
        check_files_exist(source, &manifest)?;
        let target = self.target_for(&manifest.id);
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|error| {
                format!("Vorhandene Version konnte nicht ersetzt werden: {error}")
            })?;
        }
        copy_dir(source, &target)?;
        Ok(manifest)
    }

    /// Installiert aus einer ZIP-Datei (Manifest im Wurzelordner oder in genau einem Unterordner).
    pub fn install_zip(&self, zip_path: &Path) -> Result<AddonManifest, String> {
        let file = std::fs::File::open(zip_path)
            .map_err(|error| format!("ZIP-Datei konnte nicht geöffnet werden: {error}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|error| format!("Keine gültige ZIP-Datei: {error}"))?;
        let names: Vec<String> = archive.file_names().map(str::to_string).collect();
        let manifest_name = names
            .iter()
            .filter(|name| name.rsplit('/').next() == Some(MANIFEST))
            .min_by_key(|name| name.matches('/').count())
            .cloned()
            .ok_or("Die ZIP-Datei enthält keine addon.json.")?;
        let prefix = manifest_name.trim_end_matches(MANIFEST).to_string();
        let mut manifest_text = String::new();
        archive
            .by_name(&manifest_name)
            .map_err(|error| error.to_string())?
            .read_to_string(&mut manifest_text)
            .map_err(|_| "addon.json ist keine UTF-8-Textdatei.".to_string())?;
        let manifest = parse_manifest(&manifest_text)?;
        let staging = self.user.join(format!(".installation-{}", manifest.id));
        let _ = std::fs::remove_dir_all(&staging);
        let mut total = 0_u64;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
            let Some(name) = entry.name().strip_prefix(&prefix).map(str::to_string) else {
                continue;
            };
            if name.is_empty() || entry.is_dir() {
                continue;
            }
            let relative = Path::new(&name);
            if relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(format!("Unzulässiger Pfad in der ZIP-Datei: {name}"));
            }
            total += entry.size();
            if total > MAX_ZIP_SIZE {
                return Err("Das Add-on ist zu groß (maximal 200 MB).".into());
            }
            let target = staging.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output = std::fs::File::create(&target).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        }
        let result = check_files_exist(&staging, &manifest).and_then(|_| {
            let target = self.target_for(&manifest.id);
            if target.exists() {
                std::fs::remove_dir_all(&target).map_err(|error| {
                    format!("Vorhandene Version konnte nicht ersetzt werden: {error}")
                })?;
            }
            std::fs::rename(&staging, &target)
                .map_err(|error| format!("Add-on konnte nicht installiert werden: {error}"))
        });
        let _ = std::fs::remove_dir_all(&staging);
        result.map(|_| manifest)
    }

    /// Erstellt bzw. aktualisiert ein eigenes Add-on (Add-on-Editor).
    /// `new_files`: (Quelldatei, Zielpfad relativ zum Add-on) – werden kopiert.
    pub fn save_custom(
        &self,
        manifest: AddonManifest,
        new_files: &[(PathBuf, String)],
    ) -> Result<AddonManifest, String> {
        if let Ok(existing) = self.find(&manifest.id) {
            if existing.builtin {
                return Err("Mitgelieferte Add-ons können nicht überschrieben werden – bitte eine andere ID wählen.".into());
            }
        }
        let mut manifest = manifest;
        let target = self.target_for(&manifest.id);
        std::fs::create_dir_all(&target)
            .map_err(|error| format!("Add-on-Ordner konnte nicht angelegt werden: {error}"))?;
        for (source, relative) in new_files {
            let relative = relative.replace('\\', "/");
            if !is_safe_relative_path(&relative) {
                return Err(format!("Unzulässiger Zielpfad „{relative}“."));
            }
            let destination = target.join(&relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::copy(source, &destination).map_err(|error| {
                format!(
                    "„{}“ konnte nicht kopiert werden: {error}",
                    source.display()
                )
            })?;
            if !manifest.tex_files.contains(&relative) {
                manifest.tex_files.push(relative);
            }
        }
        let manifest = validate(manifest)?;
        check_files_exist(&target, &manifest)?;
        let text = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
        write_text_atomic(&target.join(MANIFEST), &text)?;
        Ok(manifest)
    }

    /// Exportiert ein Add-on als ZIP (zum Weitergeben).
    pub fn export_zip(&self, id: &str, target: &Path) -> Result<(), String> {
        let addon = self.find(id)?;
        let root = PathBuf::from(&addon.path);
        let file = std::fs::File::create(target)
            .map_err(|error| format!("ZIP-Datei konnte nicht erstellt werden: {error}"))?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).map_err(|error| error.to_string())?;
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let relative = path
                    .strip_prefix(&root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                writer
                    .start_file(format!("{id}/{relative}"), options)
                    .map_err(|error| error.to_string())?;
                let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
                writer
                    .write_all(&bytes)
                    .map_err(|error| error.to_string())?;
            }
        }
        writer.finish().map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Projektvorlage eines Add-ons (Inhalt der `.visutex`-Datei).
    pub fn template(&self, addon_id: &str, template_id: &str) -> Result<String, String> {
        let addon = self.find(addon_id)?;
        let template = addon
            .manifest
            .templates
            .iter()
            .find(|template| template.id == template_id)
            .ok_or_else(|| format!("Vorlage „{template_id}“ nicht gefunden."))?;
        std::fs::read_to_string(PathBuf::from(&addon.path).join(&template.file))
            .map_err(|error| format!("Vorlage konnte nicht gelesen werden: {error}"))
    }
}

fn check_files_exist(root: &Path, manifest: &AddonManifest) -> Result<(), String> {
    if !is_valid_id(&manifest.id) {
        return Err("Ungültige Add-on-ID.".into());
    }
    for file in manifest
        .tex_files
        .iter()
        .chain(manifest.templates.iter().map(|template| &template.file))
    {
        if !root.join(file).is_file() {
            return Err(format!("Im Add-on fehlt die Datei „{file}“."));
        }
    }
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &destination)?;
        } else {
            std::fs::copy(&path, &destination).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_lists_disables_exports_and_removes() {
        let base = std::env::temp_dir().join(format!("visutex-addons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let source = base.join("quelle");
        std::fs::create_dir_all(source.join("tex")).unwrap();
        std::fs::write(source.join("tex/mein.sty"), "\\ProvidesPackage{mein}").unwrap();
        std::fs::write(
            source.join(MANIFEST),
            r#"{ "id": "test", "name": "Test", "texFiles": ["tex/mein.sty"], "preamble": ["\\usepackage{mein}"] }"#,
        )
        .unwrap();
        let dirs = AddonDirs {
            user: base.join("addons"),
            builtin: None,
        };
        dirs.install_folder(&source).unwrap();
        assert_eq!(dirs.list().len(), 1);
        assert_eq!(dirs.preamble(), vec!["\\usepackage{mein}".to_string()]);
        assert_eq!(
            dirs.search_paths(),
            vec![dirs.user.join("test").join("tex")]
        );
        dirs.set_enabled("test", false).unwrap();
        assert!(dirs.preamble().is_empty());
        let zip_path = base.join("test.zip");
        dirs.export_zip("test", &zip_path).unwrap();
        dirs.remove("test").unwrap();
        assert!(dirs.list().is_empty());
        dirs.install_zip(&zip_path).unwrap();
        assert_eq!(
            dirs.list()[0].manifest.tex_files,
            vec!["tex/mein.sty".to_string()]
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
