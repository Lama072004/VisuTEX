//! Add-on-Verwaltung: Installieren (Add-on-Datei `.vtxaddon`, ZIP oder Ordner), Erstellen,
//! Aktivieren, Entfernen und Exportieren. Ablage: `<App-Daten>/addons/<id>/addon.json`.
//! Mitgelieferte Beispiel-Add-ons liegen in den App-Ressourcen (`addons/`).
//! Anleitung für Add-on-Ersteller: `docs/ADDONS.md`.

use crate::core::addon::{
    check_svg_icon, is_svg_icon, is_valid_id, parse_manifest, validate, AddonManifest,
};
use crate::core::settings::is_safe_relative_path;
use crate::files::write_text_atomic;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const MANIFEST: &str = "addon.json";
/// Dateityp der Add-on-Pakete (ZIP mit `addon.json`). Ein Doppelklick auf die Datei startet
/// VisuTeX und bietet die Installation an – wie Add-in-Dateien bei Office.
pub const PACKAGE_EXTENSION: &str = "vtxaddon";
const STATE_FILE: &str = "addons-state.json";
const MAX_ZIP_SIZE: u64 = 200 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddon {
    pub manifest: AddonManifest,
    pub enabled: bool,
    pub builtin: bool,
    pub path: String,
    /// SVG-Symbole: Pfad im Add-on → data:-URL (fehlerhafte Dateien fehlen hier)
    pub icons: BTreeMap<String, String>,
}

/// SVG-Symboldatei prüfen und als data:-URL liefern (Anzeige in der Oberfläche).
pub fn icon_data_url(path: &Path) -> Result<String, String> {
    use base64::Engine;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Das Symbol „{name}“ konnte nicht gelesen werden: {error}"))?;
    check_svg_icon(&name, &bytes)?;
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// SVG-Symbole eines Add-ons laden (fehlerhafte werden ausgelassen).
fn load_icons(root: &Path, manifest: &AddonManifest) -> BTreeMap<String, String> {
    manifest
        .icon_files()
        .into_iter()
        .filter_map(|file| {
            let url = icon_data_url(&root.join(&file)).ok()?;
            Some((file, url))
        })
        .collect()
}

/// Ist `path` eine Add-on-Datei (`.vtxaddon`)?
pub fn is_package(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PACKAGE_EXTENSION))
}

/// Inhalt eines Add-ons vor der Installation (Sicherheitsabfrage in der Oberfläche).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonPreview {
    pub manifest: AddonManifest,
    pub icons: BTreeMap<String, String>,
    /// Anzahl der Dateien im Add-on (einschließlich `addon.json`)
    pub file_count: usize,
    /// Größe aller Dateien in Bytes (entpackt)
    pub size: u64,
    /// Bereits installierte Version mit derselben ID (wird ersetzt)
    pub installed_version: Option<String>,
    /// Die installierte Fassung ist mitgeliefert (das neue Add-on hat dann Vorrang)
    pub installed_builtin: bool,
    /// Liegt bereits im Add-on-Ordner (Entwicklung: nur prüfen, nicht kopieren)
    pub in_place: bool,
}

/// Add-on im Add-on-Ordner, das nicht geladen werden kann (Hinweis für Ersteller).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonProblem {
    pub folder: String,
    pub error: String,
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
            // Nur vollständige Add-ons laden; fehlerhafte nennt `problems()` mit Ursache
            let Ok(manifest) = read_manifest(&folder) else {
                continue;
            };
            if check_files_exist(&folder, &manifest).is_err() {
                continue;
            }
            if result.iter().any(|known| known.manifest.id == manifest.id) {
                continue; // Nutzer-Add-on mit gleicher ID hat Vorrang.
            }
            result.push(InstalledAddon {
                enabled: if builtin {
                    state.enabled.contains(&manifest.id)
                } else {
                    !state.disabled.contains(&manifest.id)
                },
                icons: load_icons(&folder, &manifest),
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

    /// Liegt `folder` im Add-on-Ordner des Nutzers (dort entwickelte Add-ons)?
    fn is_in_user_dir(&self, folder: &Path) -> bool {
        let canonical = |path: &Path| std::fs::canonicalize(path).unwrap_or(path.to_path_buf());
        canonical(folder).starts_with(canonical(&self.user))
    }

    /// Ordner, Datei `addon.json` oder Add-on-Datei/ZIP installieren.
    pub fn install(&self, path: &Path) -> Result<AddonManifest, String> {
        match addon_folder(path) {
            Some(folder) => self.install_folder(&folder),
            None => self.install_zip(path),
        }
    }

    /// Prüft ein Add-on (Datei, ZIP oder Ordner) vollständig, ohne es zu installieren.
    pub fn inspect(&self, path: &Path) -> Result<AddonPreview, String> {
        let (manifest, root, temporary) = match addon_folder(path) {
            Some(folder) => (read_manifest(&folder)?, folder, None),
            None => {
                let staging = std::env::temp_dir()
                    .join(format!("visutex-addon-vorschau-{}", std::process::id()));
                let manifest = unpack_zip(path, &staging)?;
                (manifest, staging.clone(), Some(staging))
            }
        };
        let result = check_files_exist(&root, &manifest).map(|()| {
            let (file_count, size) = folder_stats(&root);
            let installed = self.find(&manifest.id).ok();
            AddonPreview {
                icons: load_icons(&root, &manifest),
                file_count,
                size,
                installed_version: installed
                    .as_ref()
                    .map(|addon| addon.manifest.version.clone()),
                installed_builtin: installed.as_ref().is_some_and(|addon| addon.builtin),
                in_place: temporary.is_none() && self.is_in_user_dir(&root),
                manifest,
            }
        });
        if let Some(staging) = temporary {
            let _ = std::fs::remove_dir_all(staging);
        }
        result
    }

    /// Ordner im Add-on-Ordner, die nicht geladen werden können – mit Ursache.
    pub fn problems(&self) -> Vec<AddonProblem> {
        let Ok(entries) = std::fs::read_dir(&self.user) else {
            return Vec::new();
        };
        let mut folders: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && !is_hidden(path))
            .collect();
        folders.sort();
        folders
            .into_iter()
            .filter_map(|folder| {
                let error = if folder.join(MANIFEST).is_file() {
                    read_manifest(&folder)
                        .and_then(|manifest| check_files_exist(&folder, &manifest))
                        .err()?
                } else {
                    "Der Ordner enthält keine addon.json.".to_string()
                };
                Some(AddonProblem {
                    folder: folder.display().to_string(),
                    error,
                })
            })
            .collect()
    }

    /// Installiert aus einem Ordner mit `addon.json`.
    pub fn install_folder(&self, source: &Path) -> Result<AddonManifest, String> {
        let manifest = read_manifest(source)?;
        check_files_exist(source, &manifest)?;
        // Schon im Add-on-Ordner (dort entwickelt): nur prüfen – nie sich selbst überschreiben
        if self.is_in_user_dir(source) {
            return Ok(manifest);
        }
        let target = self.target_for(&manifest.id);
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|error| {
                format!("Vorhandene Version konnte nicht ersetzt werden: {error}")
            })?;
        }
        copy_dir(source, &target)?;
        Ok(manifest)
    }

    /// Installiert aus einer Add-on-Datei (`.vtxaddon`) bzw. ZIP-Datei (Manifest im Wurzelordner
    /// oder in genau einem Unterordner). Entpackt wird in einen Zwischenordner; erst wenn alles
    /// geprüft ist, ersetzt er eine vorhandene Version.
    pub fn install_zip(&self, zip_path: &Path) -> Result<AddonManifest, String> {
        std::fs::create_dir_all(&self.user)
            .map_err(|error| format!("Add-on-Ordner konnte nicht angelegt werden: {error}"))?;
        let staging = self
            .user
            .join(format!(".installation-{}", std::process::id()));
        let manifest = unpack_zip(zip_path, &staging)?;
        let result = check_files_exist(&staging, &manifest).and_then(|_| {
            let target = self.target_for(&manifest.id);
            if target.exists() {
                std::fs::remove_dir_all(&target).map_err(|error| {
                    format!("Vorhandene Version konnte nicht ersetzt werden: {error}")
                })?;
            }
            // Umbenennen; wo das Dateisystem es ablehnt, kopieren (Zwischenordner wird danach entfernt)
            std::fs::rename(&staging, &target)
                .or_else(|_| copy_dir(&staging, &target))
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
        // Erst alles prüfen, dann schreiben – ein abgelehntes Add-on hinterlässt keinen Ordner
        validate(manifest.clone())?;
        for (source, relative) in new_files {
            let relative = relative.replace('\\', "/");
            if !is_safe_relative_path(&relative) {
                return Err(format!("Unzulässiger Zielpfad „{relative}“."));
            }
            if is_svg_icon(&relative) {
                icon_data_url(source)?;
            }
        }
        let target = self.target_for(&manifest.id);
        let existed = target.exists();
        let result = self.write_custom(manifest, new_files, &target);
        if result.is_err() && !existed {
            let _ = std::fs::remove_dir_all(&target);
        }
        result
    }

    fn write_custom(
        &self,
        manifest: AddonManifest,
        new_files: &[(PathBuf, String)],
        target: &Path,
    ) -> Result<AddonManifest, String> {
        let mut manifest = manifest;
        std::fs::create_dir_all(target)
            .map_err(|error| format!("Add-on-Ordner konnte nicht angelegt werden: {error}"))?;
        for (source, relative) in new_files {
            let relative = relative.replace('\\', "/");
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
            // SVG-Symbole werden über `icon` angesprochen, nicht als TeX-Datei
            if !is_svg_icon(&relative) && !manifest.tex_files.contains(&relative) {
                manifest.tex_files.push(relative);
            }
        }
        let manifest = validate(manifest)?;
        check_files_exist(target, &manifest)?;
        let text = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
        write_text_atomic(&target.join(MANIFEST), &text)?;
        Ok(manifest)
    }

    /// Exportiert ein Add-on als Add-on-Datei (`.vtxaddon`, ZIP-Format) zum Weitergeben.
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
                if is_hidden(&path) {
                    continue;
                }
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

/// Ordner eines Add-ons, falls `path` ein Ordner oder dessen `addon.json` ist (sonst Datei/ZIP).
fn addon_folder(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path.to_path_buf())
    } else if path.file_name().is_some_and(|name| name == MANIFEST) {
        path.parent().map(Path::to_path_buf)
    } else {
        None
    }
}

fn read_manifest(folder: &Path) -> Result<AddonManifest, String> {
    let text = std::fs::read_to_string(folder.join(MANIFEST))
        .map_err(|_| "Im gewählten Ordner fehlt die Datei addon.json.".to_string())?;
    parse_manifest(&text)
}

/// Versteckte Dateien und Ordner (`.git`, `.DS_Store` …) gehören nicht ins Add-on.
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
}

/// Anzahl und Gesamtgröße der Dateien eines Ordners (ohne versteckte).
fn folder_stats(root: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut size = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            if is_hidden(&path) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                count += 1;
                size += std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            }
        }
    }
    (count, size)
}

/// Add-on-Datei bzw. ZIP nach `staging` entpacken (vorher geleert) und das Manifest prüfen.
/// Bei einem Fehler bleibt kein Zwischenordner zurück.
fn unpack_zip(zip_path: &Path, staging: &Path) -> Result<AddonManifest, String> {
    let _ = std::fs::remove_dir_all(staging);
    let result = unpack_zip_into(zip_path, staging);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(staging);
    }
    result
}

fn unpack_zip_into(zip_path: &Path, staging: &Path) -> Result<AddonManifest, String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|error| format!("Add-on-Datei konnte nicht geöffnet werden: {error}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| "Keine gültige Add-on-Datei (.vtxaddon bzw. ZIP erwartet).".to_string())?;
    let names: Vec<String> = archive.file_names().map(str::to_string).collect();
    let manifest_name = names
        .iter()
        .filter(|name| name.rsplit('/').next() == Some(MANIFEST))
        .min_by_key(|name| name.matches('/').count())
        .cloned()
        .ok_or("Die Add-on-Datei enthält keine addon.json.")?;
    if manifest_name.matches('/').count() > 1 {
        return Err(
            "addon.json muss im Wurzelordner der Add-on-Datei oder in genau einem Unterordner liegen."
                .into(),
        );
    }
    let prefix = manifest_name.trim_end_matches(MANIFEST).to_string();
    let mut manifest_text = String::new();
    archive
        .by_name(&manifest_name)
        .map_err(|error| error.to_string())?
        .read_to_string(&mut manifest_text)
        .map_err(|_| "addon.json ist keine UTF-8-Textdatei.".to_string())?;
    let manifest = parse_manifest(&manifest_text)?;
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
            return Err(format!("Unzulässiger Pfad in der Add-on-Datei: {name}"));
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
    Ok(manifest)
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
    for file in manifest.icon_files() {
        let bytes = std::fs::read(root.join(&file))
            .map_err(|_| format!("Im Add-on fehlt das Symbol „{file}“."))?;
        check_svg_icon(&file, &bytes)?;
    }
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if is_hidden(&path) {
            continue;
        }
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
        std::fs::create_dir_all(source.join("icons")).unwrap();
        std::fs::write(
            source.join("icons/test.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 12h20" stroke="currentColor"/></svg>"#,
        )
        .unwrap();
        std::fs::write(
            source.join(MANIFEST),
            r#"{ "id": "test", "name": "Test", "icon": "icons/test.svg", "texFiles": ["tex/mein.sty"], "preamble": ["\\usepackage{mein}"] }"#,
        )
        .unwrap();
        let dirs = AddonDirs {
            user: base.join("addons"),
            builtin: None,
        };
        dirs.install_folder(&source).unwrap();
        assert_eq!(dirs.list().len(), 1);
        assert!(dirs.list()[0].icons["icons/test.svg"].starts_with("data:image/svg+xml;base64,"));
        assert_eq!(dirs.preamble(), vec!["\\usepackage{mein}".to_string()]);
        assert_eq!(
            dirs.search_paths(),
            vec![dirs.user.join("test").join("tex")]
        );
        dirs.set_enabled("test", false).unwrap();
        assert!(dirs.preamble().is_empty());
        // Add-on-Datei: Export → Vorschau (ohne Installation) → Installation
        std::fs::create_dir_all(source.join(".git")).unwrap();
        std::fs::write(dirs.user.join("test").join(".notiz"), "versteckt").unwrap();
        let package = base.join("test-1.0.0.vtxaddon");
        assert!(is_package(&package));
        dirs.export_zip("test", &package).unwrap();
        let preview = dirs.inspect(&package).unwrap();
        assert_eq!(preview.manifest.id, "test");
        assert_eq!(preview.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(
            preview.file_count, 3,
            "addon.json, .sty, .svg – ohne versteckte Datei"
        );
        assert!(!preview.in_place);
        assert_eq!(preview.icons.len(), 1);
        dirs.remove("test").unwrap();
        assert!(dirs.list().is_empty());
        assert_eq!(dirs.inspect(&package).unwrap().installed_version, None);
        dirs.install(&package).unwrap();
        assert_eq!(
            dirs.list()[0].manifest.tex_files,
            vec!["tex/mein.sty".to_string()]
        );
        // im Add-on-Ordner entwickelt: nur prüfen, nie sich selbst überschreiben
        let installed = dirs.user.join("test");
        assert!(dirs.inspect(&installed).unwrap().in_place);
        dirs.install(&installed.join(MANIFEST)).unwrap();
        assert!(installed.join("tex/mein.sty").is_file());
        assert!(dirs.problems().is_empty());
        // fehlerhafte Ordner meldet die Verwaltung mit Ursache
        std::fs::create_dir_all(dirs.user.join("kaputt")).unwrap();
        std::fs::write(dirs.user.join("kaputt").join(MANIFEST), "{ kein json").unwrap();
        let problems = dirs.problems();
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].error.contains("ungültig"),
            "{}",
            problems[0].error
        );
        // fehlt eine Datei, wird das Add-on nicht geladen (sonst bräche das Kompilieren)
        std::fs::write(
            dirs.user.join("kaputt").join(MANIFEST),
            r#"{ "id": "kaputt", "texFiles": ["fehlt.sty"] }"#,
        )
        .unwrap();
        assert!(dirs.problems()[0].error.contains("fehlt"));
        assert!(dirs.find("kaputt").is_err());
        // keine Add-on-Datei
        std::fs::write(base.join("falsch.vtxaddon"), "kein zip").unwrap();
        assert!(dirs
            .inspect(&base.join("falsch.vtxaddon"))
            .unwrap_err()
            .contains("Keine gültige Add-on-Datei"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn refuses_addons_with_unsafe_icons() {
        let base = std::env::temp_dir().join(format!("visutex-addon-icon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let source = base.join("quelle");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("böse.svg"),
            "<svg><script>alert(1)</script></svg>",
        )
        .unwrap();
        std::fs::write(
            source.join(MANIFEST),
            r#"{ "id": "boese", "icon": "böse.svg" }"#,
        )
        .unwrap();
        let dirs = AddonDirs {
            user: base.join("addons"),
            builtin: None,
        };
        assert!(dirs
            .install_folder(&source)
            .unwrap_err()
            .contains("Unzulässiges"));
        std::fs::write(
            source.join(MANIFEST),
            r#"{ "id": "boese", "icon": "fehlt.svg" }"#,
        )
        .unwrap();
        assert!(dirs.install_folder(&source).unwrap_err().contains("fehlt"));

        // Add-on-Editor: unsicheres Symbol wird abgelehnt, ohne einen Ordner anzulegen
        let manifest = parse_manifest(r#"{ "id": "boese", "icon": "icons/boese.svg" }"#).unwrap();
        let files = [(source.join("böse.svg"), "icons/boese.svg".to_string())];
        assert!(dirs.save_custom(manifest, &files).is_err());
        assert!(!dirs.user.join("boese").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Mitgelieferte Add-ons und das Beispiel aus `docs/ADDONS.md` sind gültig – samt Symbolen.
    #[test]
    fn bundled_and_example_addons_install() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let base =
            std::env::temp_dir().join(format!("visutex-addon-beispiel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dirs = AddonDirs {
            user: base.join("addons"),
            builtin: None,
        };
        let example = manifest_dir.join("../docs/addon-beispiel/hinweisboxen");
        let mut folders = vec![example];
        for entry in std::fs::read_dir(manifest_dir.join("resources/addons")).unwrap() {
            folders.push(entry.unwrap().path());
        }
        for folder in &folders {
            let manifest = dirs
                .install_folder(folder)
                .unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
            let installed = dirs.find(&manifest.id).unwrap();
            assert_eq!(installed.icons.len(), manifest.icon_files().len());
            for template in &manifest.templates {
                let text = dirs.template(&manifest.id, &template.id).unwrap();
                crate::core::project::parse_project(&text).unwrap();
            }
        }
        let example = dirs.find("hinweisboxen").unwrap();
        assert_eq!(example.icons.len(), 3);
        assert!(dirs
            .preamble()
            .contains(&"\\usepackage{hinweisboxen}".to_string()));
        assert!(dirs.search_paths().iter().any(|path| path.ends_with("tex")));
        let _ = std::fs::remove_dir_all(&base);
    }
}
