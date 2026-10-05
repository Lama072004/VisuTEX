//! Auswahl des TeX-Bundles für Tectonic.
//!
//! 1. **Mitgeliefertes Bundle** (`resources/tex-bundle.zip`, Tauri-Resource):
//!    vollständig offline, keine externe LaTeX-Installation nötig.
//! 2. Optional (Einstellung „Fehlende Pakete online nachladen“): Tectonics
//!    Standard-Bundle als Ergänzung – heruntergeladene Dateien werden lokal
//!    zwischengespeichert und stehen danach ebenfalls offline zur Verfügung.
//!
//! Ist kein mitgeliefertes Bundle vorhanden (Entwicklungs-Build), wird das
//! Standard-Bundle verwendet.

use std::path::{Path, PathBuf};
use tectonic_bundles::{zip::ZipBundle, Bundle};
use tectonic_errors::Result as TexResult;
use tectonic_io_base::{digest::DigestData, InputHandle, IoProvider, OpenResult};
use tectonic_status_base::StatusBackend;

pub const BUNDLE_FILE_NAME: &str = "tex-bundle.zip";

/// Wo das Bundle liegt und ob online nachgeladen werden darf.
#[derive(Clone, Debug)]
pub struct BundleConfig {
    pub embedded_zip: Option<PathBuf>,
    pub allow_online: bool,
    /// Cache-Ordner der App (für Formatdateien des mitgelieferten Bundles).
    pub cache_dir: PathBuf,
    /// Vom Nutzer gewählter Speicherort für online nachgeladene Pakete
    /// (`None` = Tectonic-Standardordner bzw. `cache_dir/tex-pakete`).
    pub online_cache: Option<PathBuf>,
}

impl BundleConfig {
    /// Ordner, in dem nachgeladene TeX-Pakete dauerhaft gespeichert werden.
    pub fn package_cache_dir(&self) -> PathBuf {
        self.online_cache
            .clone()
            .unwrap_or_else(|| self.cache_dir.join("tex-pakete"))
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInfo {
    /// embedded | embedded+online | online
    pub kind: String,
    pub embedded_path: Option<String>,
    pub file_count: Option<usize>,
}

/// Erst das mitgelieferte Bundle, dann (falls erlaubt) das Online-Bundle.
pub struct CompositeBundle {
    primary: Box<dyn Bundle>,
    fallback: Option<Box<dyn Bundle>>,
}

impl IoProvider for CompositeBundle {
    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        match self.primary.input_open_name(name, status) {
            OpenResult::NotAvailable => match &mut self.fallback {
                Some(fallback) => fallback.input_open_name(name, status),
                None => OpenResult::NotAvailable,
            },
            other => other,
        }
    }
}

impl Bundle for CompositeBundle {
    fn get_digest(&mut self) -> TexResult<DigestData> {
        self.primary.get_digest()
    }

    fn all_files(&self) -> Vec<String> {
        let mut files = self.primary.all_files();
        if let Some(fallback) = &self.fallback {
            files.extend(fallback.all_files());
        }
        files
    }
}

/// Online-Bundle (Tectonic-Standard). Heruntergeladene Dateien landen dauerhaft in
/// `package_dir` – damit sind sie danach auch offline verfügbar.
/// Tectonic lädt den Dateiindex in `data/<hash>.index-tmp-pid<n>` und benennt ihn
/// danach um – unter Windows mit gemischten Pfadtrennern (`…\data/…`), was dort
/// mit „Datei kann nicht auf ein anderes Laufwerk verschoben werden“ scheitern kann.
/// Vollständig geladene Zwischendateien werden hier selbst umbenannt.
fn promote_index_files(package_dir: &Path) {
    let data = package_dir.join("data");
    let Ok(entries) = std::fs::read_dir(&data) else {
        return;
    };
    let mut temporary: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains(".index-tmp-pid"))
        })
        .collect();
    // Neueste zuerst
    temporary.sort_by_key(|path| {
        std::cmp::Reverse(
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok(),
        )
    });
    for path in temporary {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(base) = name.split(".index-tmp-pid").next() else {
            continue;
        };
        let target = data.join(format!("{base}.index"));
        if target.exists() {
            let _ = std::fs::remove_file(&path);
        } else {
            let _ = std::fs::rename(&path, &target);
        }
    }
}

fn detect_default(location: &str, package_dir: &Path) -> Result<Box<dyn Bundle>, String> {
    let mut bundle = tectonic_bundles::detect_bundle(
        location.to_string(),
        false,
        Some(package_dir.to_path_buf()),
    )
    .map_err(|error| {
        format!("Das TeX-Paketbundle konnte nicht geladen werden (keine Internetverbindung?): {error:#}")
    })?
    .ok_or("Unbekannte Bundle-Adresse in der Tectonic-Konfiguration.")?;
    // Index sofort laden (sonst passiert das erst mitten im Kompilieren).
    let probe = bundle.input_open_name(
        "tectonic-format-latex.tex",
        &mut tectonic_status_base::NoopStatusBackend {},
    );
    match probe {
        OpenResult::Err(error) => Err(format!("{error:#}")),
        _ => Ok(bundle),
    }
}

fn default_bundle(package_dir: &Path) -> Result<(Box<dyn Bundle>, PathBuf), String> {
    let config = tectonic::config::PersistentConfig::open(false).map_err(|error| {
        format!("Tectonic-Konfiguration konnte nicht geladen werden: {error:#}")
    })?;
    std::fs::create_dir_all(package_dir).map_err(|error| {
        format!(
            "Speicherort für TeX-Pakete „{}“ ist nicht verfügbar: {error}",
            package_dir.display()
        )
    })?;
    let location = config.default_bundle_loc().to_string();
    promote_index_files(package_dir);
    let bundle = match detect_default(&location, package_dir) {
        Ok(bundle) => bundle,
        Err(_) => {
            // Zwischendatei des Index selbst umbenennen und erneut versuchen
            promote_index_files(package_dir);
            detect_default(&location, package_dir)?
        }
    };
    let formats = package_dir.join("formate");
    std::fs::create_dir_all(&formats)
        .map_err(|error| format!("Formatcache konnte nicht angelegt werden: {error}"))?;
    Ok((bundle, formats))
}

fn zip_bundle(path: &Path) -> Result<Box<dyn Bundle>, String> {
    ZipBundle::open(path)
        .map(|bundle| Box::new(bundle) as Box<dyn Bundle>)
        .map_err(|error| {
            format!(
                "Das mitgelieferte TeX-Bundle „{}“ ist beschädigt: {error:#}",
                path.display()
            )
        })
}

/// Öffnet das Bundle gemäß Konfiguration; liefert zusätzlich den Formatcache-Ordner.
pub fn open_bundle(config: &BundleConfig) -> Result<(Box<dyn Bundle>, PathBuf), String> {
    match config.embedded_zip.as_deref().filter(|path| path.is_file()) {
        Some(path) => {
            let primary = zip_bundle(path)?;
            let fallback = if config.allow_online {
                default_bundle(&config.package_cache_dir())
                    .ok()
                    .map(|(bundle, _)| bundle)
            } else {
                None
            };
            let cache = config.cache_dir.join("tex-formats");
            std::fs::create_dir_all(&cache)
                .map_err(|error| format!("Formatcache konnte nicht angelegt werden: {error}"))?;
            Ok((Box::new(CompositeBundle { primary, fallback }), cache))
        }
        None if config.allow_online => default_bundle(&config.package_cache_dir()),
        None => Err("Kein TeX-Bundle gefunden und das Nachladen aus dem Internet ist aus. Datei → Info → Systemprüfung zeigt die Lösung (Bundle-Datei wählen oder Nachladen mit Speicherort erlauben).".into()),
    }
}

pub fn bundle_info(config: &BundleConfig) -> BundleInfo {
    match config.embedded_zip.as_deref().filter(|path| path.is_file()) {
        Some(path) => BundleInfo {
            kind: if config.allow_online {
                "embedded+online".into()
            } else {
                "embedded".into()
            },
            embedded_path: Some(path.display().to_string()),
            file_count: zip_bundle(path).ok().map(|bundle| bundle.all_files().len()),
        },
        None => BundleInfo {
            kind: "online".into(),
            embedded_path: None,
            file_count: None,
        },
    }
}
