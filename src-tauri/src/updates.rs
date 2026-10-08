//! Update-Suche über die GitHub-Releases (auch Vorabversionen wie `v0.2.0-alpha.2`).
//!
//! Gesucht wird die höchste Version (semantisch verglichen, `alpha < beta < rc < final`).
//! Installiert wird die passende Datei des Releases: Windows `…_windows_x64.exe`
//! (ersatzweise die portable ZIP), Linux die `.AppImage` – die laufende EXE wird ersetzt
//! (Windows: alte Datei beiseitelegen) und die App neu gestartet. Ohne passende Datei
//! (z. B. per Paketverwalter installiert) gibt es nur den Link zur Release-Seite.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// GitHub-Repository der Releases.
pub const REPOSITORY: &str = "Lama072004/VisuTEX";

/// Version dieses Programms; Release-Builds der CI setzen `VISUTEX_VERSION` auf den Tag
/// (z. B. `0.2.0-alpha.2`), sonst gilt die Version aus `Cargo.toml`.
pub fn current_version() -> &'static str {
    option_env!("VISUTEX_VERSION")
        .filter(|version| !version.trim().is_empty())
        .unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Prüfsumme laut GitHub (`sha256:…`), leer bei älteren Releases
    #[serde(default)]
    pub digest: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub prerelease: bool,
    pub page: String,
    /// Release-Text (gekürzt)
    pub notes: String,
    /// Passende Datei zum automatischen Aktualisieren (sonst nur Link)
    pub asset: Option<UpdateAsset>,
}

// ------------------------------------------------------------------ Versionen

#[derive(Debug, PartialEq, Eq)]
struct Version {
    numbers: Vec<u64>,
    /// leer = endgültige Version
    pre: Vec<String>,
}

fn parse_version(text: &str) -> Option<Version> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let text = text.split('+').next()?;
    let (core, pre) = match text.split_once('-') {
        Some((core, pre)) => (core, pre),
        None => (text, ""),
    };
    let numbers = core
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    if numbers.is_empty() {
        return None;
    }
    let pre = if pre.is_empty() {
        Vec::new()
    } else {
        pre.split('.').map(str::to_lowercase).collect()
    };
    Some(Version { numbers, pre })
}

fn compare_identifiers(left: &str, right: &str) -> Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => left.cmp(right),
    }
}

fn compare(left: &Version, right: &Version) -> Ordering {
    let length = left.numbers.len().max(right.numbers.len());
    for index in 0..length {
        let a = left.numbers.get(index).copied().unwrap_or(0);
        let b = right.numbers.get(index).copied().unwrap_or(0);
        if a != b {
            return a.cmp(&b);
        }
    }
    match (left.pre.is_empty(), right.pre.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            for (a, b) in left.pre.iter().zip(&right.pre) {
                let order = compare_identifiers(a, b);
                if order != Ordering::Equal {
                    return order;
                }
            }
            left.pre.len().cmp(&right.pre.len())
        }
    }
}

/// Ist `candidate` neuer als `current`?
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => compare(&candidate, &current) == Ordering::Greater,
        _ => false,
    }
}

// ------------------------------------------------------------------ GitHub

fn client(timeout: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .connect_timeout(Duration::from_secs(8))
        .user_agent(format!("VisuTeX/{}", current_version()))
        .build()
        .map_err(|error| format!("HTTP-Client konnte nicht erstellt werden: {error}"))
}

/// Passende Datei für dieses System.
fn pick_asset(assets: &[serde_json::Value], kind: &str) -> Option<UpdateAsset> {
    let list: Vec<UpdateAsset> = assets
        .iter()
        .filter_map(|asset| {
            Some(UpdateAsset {
                name: asset.get("name")?.as_str()?.to_string(),
                url: asset.get("browser_download_url")?.as_str()?.to_string(),
                size: asset
                    .get("size")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                digest: asset
                    .get("digest")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect();
    let find = |predicate: &dyn Fn(&str) -> bool| {
        list.iter()
            .find(|asset| predicate(&asset.name.to_lowercase()))
            .cloned()
    };
    match kind {
        // Über Setup.exe installiert: neue Setup.exe still ausführen (Eintrag unter „Apps“ stimmt)
        "nsis" => find(&|name| name.ends_with("-setup.exe")),
        "appimage" => find(&|name| name.ends_with(".appimage")),
        // MSI (Administratorrechte) und deb/rpm (Paketverwaltung): nur Hinweis + Release-Seite
        "msi" | "package" => None,
        _ if cfg!(windows) => find(&|name| name.ends_with("_windows_x64.exe"))
            .or_else(|| find(&|name| name.ends_with("-portable.zip"))),
        _ => None,
    }
}

/// Testmodus (nur Debug-Builds): `VISUTEX_UPDATE_FEED=<URL>` ersetzt die GitHub-Abfrage durch
/// eine lokale Releases-Liste (gleiches JSON-Format), Downloads dürfen dann von `127.0.0.1` kommen.
fn test_feed() -> Option<String> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("VISUTEX_UPDATE_FEED")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn allowed_download(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.starts_with("https://github.com/")
        || lower.starts_with("https://objects.githubusercontent.com/")
        || (test_feed().is_some() && lower.starts_with("http://127.0.0.1:"))
}

/// Fragt die Releases ab und liefert die neueste Version.
pub fn check() -> Result<UpdateInfo, String> {
    let feed = test_feed().unwrap_or_else(|| {
        format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=30")
    });
    let response = client(20)?
        .get(feed)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|_| "GitHub ist nicht erreichbar – keine Internetverbindung?".to_string())?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("Antwort von GitHub unlesbar: {error}"))?;
    if !status.is_success() {
        return Err(format!("GitHub antwortet mit Fehler {status}."));
    }
    let releases: Vec<serde_json::Value> = serde_json::from_str(&text)
        .map_err(|error| format!("Antwort von GitHub unlesbar: {error}"))?;
    let newest = releases
        .iter()
        .filter(|release| {
            !release
                .get("draft")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|release| {
            let tag = release.get("tag_name")?.as_str()?;
            Some((parse_version(tag)?, release))
        })
        .max_by(|(left, _), (right, _)| compare(left, right))
        .map(|(_, release)| release);
    let current = current_version().to_string();
    let Some(release) = newest else {
        return Ok(UpdateInfo {
            current: current.clone(),
            latest: current,
            newer: false,
            prerelease: false,
            page: format!("https://github.com/{REPOSITORY}/releases"),
            notes: String::new(),
            asset: None,
        });
    };
    let tag = release
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let latest = tag.trim_start_matches(['v', 'V']).to_string();
    let notes: String = release
        .get("body")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .chars()
        .take(2000)
        .collect();
    Ok(UpdateInfo {
        newer: is_newer(&latest, &current),
        current,
        latest,
        prerelease: release
            .get("prerelease")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        page: release
            .get("html_url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        notes,
        asset: release
            .get("assets")
            .and_then(serde_json::Value::as_array)
            .and_then(|assets| pick_asset(assets, crate::setup::install_kind())),
    })
}

/// Lädt die neue Version herunter, ersetzt die laufende Programmdatei und liefert ihren Pfad
/// (zum Neustart). Bei einer Setup.exe (über das Installationsprogramm installiert) läuft
/// diese nach dem Beenden still und startet VisuTeX neu – dann `None`.
/// `progress(geladen, gesamt)` wird während des Downloads aufgerufen.
pub fn install(
    asset: &UpdateAsset,
    work_dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Option<PathBuf>, String> {
    if !allowed_download(&asset.url) {
        return Err("Unerwartete Download-Adresse.".into());
    }
    let exe = crate::setup::current_exe().ok_or("Programmdatei nicht gefunden.")?;
    std::fs::create_dir_all(work_dir).map_err(|error| error.to_string())?;
    let download = work_dir.join("update.download");
    let mut response = client(1800)?
        .get(&asset.url)
        .send()
        .map_err(|error| format!("Download fehlgeschlagen: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Download fehlgeschlagen ({}).", response.status()));
    }
    let total = response.content_length().unwrap_or(asset.size);
    let mut file = std::fs::File::create(&download).map_err(|error| error.to_string())?;
    let mut buffer = vec![0u8; 1 << 16];
    let mut loaded = 0u64;
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("Download abgebrochen: {error}"))?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buffer[..read]).map_err(|error| error.to_string())?;
        sha2::Digest::update(&mut hasher, &buffer[..read]);
        loaded += read as u64;
        progress(loaded, total);
    }
    drop(file);
    if asset.size > 0 && loaded != asset.size {
        let _ = std::fs::remove_file(&download);
        return Err("Download unvollständig – bitte erneut versuchen.".into());
    }
    let actual = hex(&sha2::Digest::finalize(hasher));
    if !digest_matches(&asset.digest, &actual) {
        let _ = std::fs::remove_file(&download);
        return Err(
            "Prüfsumme der heruntergeladenen Datei stimmt nicht – Update abgebrochen.".into(),
        );
    }
    if asset.name.to_lowercase().ends_with("-setup.exe") {
        let setup = work_dir.join("VisuTeX-Setup.exe");
        let _ = std::fs::remove_file(&setup);
        std::fs::rename(&download, &setup).map_err(|error| error.to_string())?;
        run_setup_after_exit(&setup, &exe)?;
        return Ok(None);
    }
    // ZIP (portable Version): Programmdatei herausholen
    let new_file = if asset.name.to_lowercase().ends_with(".zip") {
        let extracted = work_dir.join("update.exe");
        extract_exe(&download, &extracted)?;
        let _ = std::fs::remove_file(&download);
        extracted
    } else {
        download
    };
    let staged = exe.with_extension("neu");
    std::fs::copy(&new_file, &staged)
        .map_err(|error| format!("Update konnte nicht abgelegt werden: {error}"))?;
    let _ = std::fs::remove_file(&new_file);
    crate::setup::replace_file(&staged, &exe)?;
    Ok(Some(exe))
}

/// Setup.exe nach dem Beenden still ausführen (`/S`; `/UPDATE`: bestehende Installation
/// aktualisieren) und VisuTeX danach neu starten, sofern das Installationsprogramm es nicht
/// schon selbst gestartet hat. Nicht `/P` (passiv): dabei zeigt der Installer trotzdem die
/// Sprachauswahl (`displayLanguageSelector`) und das Update bliebe dort stehen – im echten
/// Durchlauf so beobachtet. Still behält er die bei der Installation gewählte Sprache.
#[cfg(windows)]
fn run_setup_after_exit(setup: &Path, exe: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let quote = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "''"));
    let process = exe
        .file_stem()
        .map(|stem| stem.to_string_lossy().replace('\'', "''"))
        .unwrap_or_else(|| "visutex".into());
    let script = format!(
        "Start-Sleep -Seconds 2; Start-Process -FilePath {} -ArgumentList '/S','/UPDATE' -Wait; \
         Start-Sleep -Seconds 1; \
         if (-not (Get-Process -Name '{process}' -ErrorAction SilentlyContinue)) {{ Start-Process -FilePath {} }}",
        quote(setup),
        quote(exe)
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Installationsprogramm konnte nicht gestartet werden: {error}"))
}

#[cfg(not(windows))]
fn run_setup_after_exit(_setup: &Path, _exe: &Path) -> Result<(), String> {
    Err("Setup.exe gibt es nur unter Windows.".into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// GitHub-Prüfsumme (`sha256:<hex>`) vergleichen; ohne Angabe (ältere Releases) nur die Größe.
fn digest_matches(expected: &str, actual_sha256: &str) -> bool {
    match expected.trim().split_once(':') {
        Some((algorithm, value)) if algorithm.eq_ignore_ascii_case("sha256") => {
            value.trim().eq_ignore_ascii_case(actual_sha256)
        }
        Some(_) => true, // anderes Verfahren – nicht prüfbar
        None => expected.trim().is_empty() || expected.trim().eq_ignore_ascii_case(actual_sha256),
    }
}

fn extract_exe(zip_path: &Path, target: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|error| error.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Update-Archiv beschädigt: {error}"))?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().replace('\\', "/").to_lowercase();
        if name.ends_with("/visutex.exe") || name == "visutex.exe" {
            let mut output = std::fs::File::create(target).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
            return Ok(());
        }
    }
    Err("Im Update-Archiv wurde keine VisuTeX.exe gefunden.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_including_prereleases() {
        assert!(is_newer("v0.2.0-alpha.2", "0.2.0-alpha.1"));
        assert!(is_newer("0.2.0-beta.1", "0.2.0-alpha.9"));
        assert!(is_newer("0.2.0-rc.1", "0.2.0-beta.3"));
        assert!(is_newer("0.2.0", "0.2.0-rc.1"));
        assert!(is_newer("0.3.0-alpha.1", "0.2.0"));
        assert!(is_newer("0.2.10", "0.2.9"));
        assert!(!is_newer("0.2.0-alpha.1", "0.2.0"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("kein-tag", "0.2.0"));
    }

    #[test]
    fn picks_matching_asset() {
        let assets: Vec<serde_json::Value> = serde_json::from_str(
            r#"[
                {"name": "VisuTeX_0.2.0_x64-setup.exe", "browser_download_url": "https://github.com/a", "size": 1},
                {"name": "VisuTeX_0.2.0_windows_x64.exe", "browser_download_url": "https://github.com/b", "size": 2},
                {"name": "VisuTeX_0.2.0_x64-portable.zip", "browser_download_url": "https://github.com/c", "size": 3},
                {"name": "VisuTeX_0.2.0_amd64.AppImage", "browser_download_url": "https://github.com/d", "size": 4}
            ]"#,
        )
        .unwrap();
        let asset = pick_asset(&assets, "exe");
        if cfg!(windows) {
            assert_eq!(asset.unwrap().name, "VisuTeX_0.2.0_windows_x64.exe");
        }
        assert_eq!(
            pick_asset(&assets, "nsis").unwrap().name,
            "VisuTeX_0.2.0_x64-setup.exe"
        );
        assert_eq!(
            pick_asset(&assets, "appimage").unwrap().name,
            "VisuTeX_0.2.0_amd64.AppImage"
        );
        assert!(pick_asset(&assets, "msi").is_none());
        assert!(pick_asset(&assets, "package").is_none());
    }

    #[test]
    fn checks_github_digest() {
        let sha = hex(&<sha2::Sha256 as sha2::Digest>::digest(b"abc"));
        assert_eq!(
            sha,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(digest_matches(&format!("sha256:{sha}"), &sha));
        assert!(digest_matches(
            &format!("SHA256:{}", sha.to_uppercase()),
            &sha
        ));
        assert!(!digest_matches("sha256:00", &sha));
        assert!(digest_matches("", &sha));
    }

    #[test]
    fn extracts_exe_from_portable_zip() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("visutex-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("portable.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("VisuTeX-portable\\licenses\\LICENSE.txt", options)
            .unwrap();
        writer.write_all(b"Lizenz").unwrap();
        // Windows PowerShell 5.1 (Compress-Archive) speichert Pfade mit „\“
        writer
            .start_file("VisuTeX-portable\\VisuTeX.exe", options)
            .unwrap();
        writer.write_all(b"programm").unwrap();
        writer.finish().unwrap();
        let target = dir.join("VisuTeX.neu");
        extract_exe(&archive, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"programm");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
