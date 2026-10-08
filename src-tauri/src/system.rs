//! Systemprüfung: Was ist mitgeliefert, was wurde auf dem System gefunden, was
//! fehlt – mit Lösungsvorschlag (z. B. Speicherort wählen). Gilt für Windows,
//! Linux und macOS gleichermaßen.

use crate::texbundle::BundleConfig;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tectonic_bundles::{zip::ZipBundle, Bundle};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub id: &'static str,
    pub name: String,
    /// ok | warning | error | info
    pub status: &'static str,
    pub detail: String,
    /// choose-bundle | choose-package-dir | enable-online
    pub action: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCheck {
    pub items: Vec<CheckItem>,
    pub package_cache_dir: String,
    pub bundle_path: Option<String>,
    pub os: String,
}

pub struct CheckInput {
    pub bundle: BundleConfig,
    pub data_dir: Option<PathBuf>,
    pub webview_version: Result<String, String>,
    pub check_zotero: bool,
}

/// Pfad für Menschen lesbar (ohne Windows-Präfix \\?\).
pub fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix(r"\\?\")
        .map(str::to_string)
        .unwrap_or(text)
}

/// Prüft, ob in einem Ordner Dateien angelegt werden können (legt ihn bei Bedarf an).
pub fn writable(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let probe = dir.join(".visutex-schreibtest");
    std::fs::write(&probe, b"ok").map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

/// Hinweis, wo das mitgelieferte Bundle erwartet wird (häufigster Fehler: nur die EXE kopiert).
fn resources_hint() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("resources")))
        .map(|dir| {
            format!(
                " Erwartet wird der Ordner „resources“ neben dem Programm ({}). Bei der portablen Version immer den ganzen Ordner kopieren bzw. eine Verknüpfung zur EXE anlegen – nicht nur VisuTeX.exe.",
                display_path(&dir)
            )
        })
        .unwrap_or_default()
}

fn item(
    id: &'static str,
    name: &str,
    status: &'static str,
    detail: String,
    action: Option<&'static str>,
) -> CheckItem {
    CheckItem {
        id,
        name: name.into(),
        status,
        detail,
        action,
    }
}

pub fn run(input: CheckInput) -> SystemCheck {
    let mut items = Vec::new();
    let package_dir = input.bundle.package_cache_dir();

    items.push(item(
        "engine",
        "TeX-Engine",
        "ok",
        "Tectonic 0.17 (XeTeX, BibTeX, xdvipdfmx) ist in VisuTeX eingebaut – keine LaTeX-Installation nötig.".into(),
        None,
    ));

    match input.bundle.embedded_zip.as_deref().filter(|path| path.is_file()) {
        Some(path) => match ZipBundle::open(path) {
            Ok(mut bundle) => {
                let files = bundle.all_files().len();
                match bundle.get_digest() {
                    Ok(_) => items.push(item(
                        "bundle",
                        "TeX-Pakete",
                        "ok",
                        format!("Mitgeliefert: {files} Dateien ({})", display_path(&path)),
                        None,
                    )),
                    Err(error) => items.push(item(
                        "bundle",
                        "TeX-Pakete",
                        "error",
                        format!("Das TeX-Bundle ist beschädigt ({error}). Bitte eine gültige Bundle-Datei wählen."),
                        Some("choose-bundle"),
                    )),
                }
            }
            Err(error) => items.push(item(
                "bundle",
                "TeX-Pakete",
                "error",
                format!("Das TeX-Bundle „{}“ kann nicht gelesen werden: {error}", display_path(&path)),
                Some("choose-bundle"),
            )),
        },
        None if input.bundle.allow_online => items.push(item(
            "bundle",
            "TeX-Pakete",
            "warning",
            format!(
                "Kein mitgeliefertes TeX-Bundle gefunden. Pakete werden bei Bedarf online geladen und in {} gespeichert.{}",
                display_path(&package_dir),
                resources_hint()
            ),
            Some("choose-bundle"),
        )),
        None => items.push(item(
            "bundle",
            "TeX-Pakete",
            "error",
            format!(
                "Kein TeX-Bundle gefunden. Bitte eine Bundle-Datei wählen oder das Nachladen aus dem Internet erlauben (mit wählbarem Speicherort).{}",
                resources_hint()
            ),
            Some("choose-bundle"),
        )),
    }

    if input.bundle.allow_online {
        match writable(&package_dir) {
            Ok(()) => items.push(item(
                "packages",
                "Nachladen fehlender Pakete",
                "ok",
                format!("Erlaubt – Speicherort: {}", display_path(&package_dir)),
                Some("choose-package-dir"),
            )),
            Err(error) => items.push(item(
                "packages",
                "Nachladen fehlender Pakete",
                "error",
                format!(
                    "Speicherort {} ist nicht beschreibbar: {error}",
                    display_path(&package_dir)
                ),
                Some("choose-package-dir"),
            )),
        }
    } else {
        items.push(item(
            "packages",
            "Nachladen fehlender Pakete",
            "info",
            "Aus – VisuTeX arbeitet vollständig offline. Fehlt ein Paket, wird nachgefragt (inklusive Speicherort).".into(),
            Some("enable-online"),
        ));
    }

    match writable(&input.bundle.cache_dir) {
        Ok(()) => items.push(item(
            "cache",
            "Zwischenspeicher",
            "ok",
            display_path(&input.bundle.cache_dir),
            None,
        )),
        Err(error) => items.push(item(
            "cache",
            "Zwischenspeicher",
            "error",
            format!(
                "{} ist nicht beschreibbar: {error}",
                display_path(&input.bundle.cache_dir)
            ),
            None,
        )),
    }

    if let Some(data_dir) = &input.data_dir {
        match writable(data_dir) {
            Ok(()) => items.push(item(
                "data",
                "App-Daten (Add-ons)",
                "ok",
                display_path(&data_dir),
                None,
            )),
            Err(error) => items.push(item(
                "data",
                "App-Daten (Add-ons)",
                "error",
                format!(
                    "{} ist nicht beschreibbar: {error}",
                    display_path(&data_dir)
                ),
                None,
            )),
        }
    }

    let families = crate::fonts::installed_families().len();
    items.push(item(
        "fonts",
        "Schriften",
        "ok",
        format!("Latin Modern ist mitgeliefert; {families} Systemschriften gefunden."),
        None,
    ));

    match &input.webview_version {
        Ok(version) => items.push(item(
            "webview",
            "WebView-Laufzeit",
            "ok",
            version.clone(),
            None,
        )),
        Err(error) => items.push(item(
            "webview",
            "WebView-Laufzeit",
            "error",
            error.clone(),
            None,
        )),
    }

    if input.check_zotero {
        let status = crate::zotero::status();
        items.push(item(
            "zotero",
            "Zotero (optional)",
            if status.better_bibtex || status.local_api {
                "ok"
            } else {
                "info"
            },
            status.message,
            None,
        ));
    }

    SystemCheck {
        items,
        package_cache_dir: display_path(&package_dir),
        bundle_path: input
            .bundle
            .embedded_zip
            .as_ref()
            .map(|path| display_path(&path)),
        os: std::env::consts::OS.into(),
    }
}
