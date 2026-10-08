//! Einrichtung der heruntergeladenen EXE (wie ein Installer, aber in der App):
//! Installationsort wählen, EXE dorthin kopieren, Verknüpfungen (Desktop, Startmenü),
//! Autostart und – unter Windows – Eintrag in „Apps“ zum Deinstallieren.
//!
//! Windows: Verknüpfungen über die Shell (PowerShell/WScript.Shell), Autostart und
//! Deinstallationseintrag in `HKCU` (ohne Administratorrechte). Linux: `.desktop`-Dateien
//! nach XDG (Anwendungsmenü, Desktop, `~/.config/autostart`).
//!
//! Von Paketverwaltern bzw. dem MSI/NSIS-Installer installierte Programme und
//! Entwicklungs-Builds zeigen die Einrichtung nicht.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_NAME: &str = "VisuTeX";
const STATE_FILE: &str = "einrichtung.json";
/// Symbol für Linux-Verknüpfungen
#[cfg(not(windows))]
const ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

/// Gespeicherte Entscheidung (im Datenordner der App).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    /// installed | portable
    pub mode: String,
    pub install_dir: Option<String>,
    pub desktop_shortcut: bool,
    pub start_menu: bool,
    pub autostart: bool,
    /// Version, mit der der Eintrag unter „Apps“ zuletzt geschrieben wurde
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub version: String,
    /// windows | linux | macos
    pub platform: String,
    pub current_exe: String,
    pub default_dir: String,
    pub state: Option<SetupState>,
    /// Läuft die installierte Kopie?
    pub running_installed: bool,
    /// Vom Paketverwalter bzw. MSI/NSIS installiert → keine eigene Einrichtung
    pub packaged: bool,
    /// Einrichtungsbildschirm zeigen
    pub show_setup: bool,
    /// Mit `--uninstall` gestartet (Windows „Apps“ → Deinstallieren)
    pub uninstall_requested: bool,
    /// Testmodus aktiv (nur Debug-Builds, siehe [`sandbox`])
    pub sandbox: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub dir: String,
    pub desktop_shortcut: bool,
    pub start_menu: bool,
    pub autostart: bool,
}

fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Die eigentliche Programmdatei (unter Linux bei AppImage die `.AppImage`-Datei).
pub fn current_exe() -> Option<PathBuf> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Some(PathBuf::from(appimage));
    }
    std::env::current_exe().ok()
}

fn exe_name() -> String {
    if cfg!(windows) {
        format!("{APP_NAME}.exe")
    } else if std::env::var_os("APPIMAGE").is_some() {
        format!("{APP_NAME}.AppImage")
    } else {
        APP_NAME.to_string()
    }
}

/// Vorschlag für den Installationsort (ohne Administratorrechte).
pub fn default_dir() -> PathBuf {
    if let Some(root) = sandbox() {
        return root.join("Programme").join(APP_NAME);
    }
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Programs")
            .join(APP_NAME)
    } else {
        home().join(".local").join("share").join(APP_NAME)
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Von Paketverwalter oder Installer eingerichtet (dann keine eigene Einrichtung anbieten).
fn is_packaged(exe: &Path) -> bool {
    let text = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    if cfg!(windows) {
        let dir = exe.parent();
        text.contains("/program files")
            || dir.is_some_and(|dir| dir.join("uninstall.exe").is_file())
    } else {
        std::env::var_os("APPIMAGE").is_none()
            && (text.starts_with("/usr/") || text.starts_with("/opt/"))
    }
}

/// Wie wurde dieses Programm installiert? `nsis` (Setup.exe), `msi`, `package` (deb/rpm),
/// `appimage` oder `exe` (einzelne Programmdatei, ggf. über die eigene Einrichtung).
pub fn install_kind() -> &'static str {
    let Some(exe) = current_exe() else {
        return "exe";
    };
    if std::env::var_os("APPIMAGE").is_some() {
        return "appimage";
    }
    if cfg!(windows) {
        if exe
            .parent()
            .is_some_and(|dir| dir.join("uninstall.exe").is_file())
        {
            return "nsis";
        }
        if is_packaged(&exe) {
            return "msi";
        }
    } else if is_packaged(&exe) {
        return "package";
    }
    "exe"
}

/// Testmodus (nur Debug-Builds): `VISUTEX_SETUP_SANDBOX=<Ordner>` leitet Verknüpfungen,
/// Autostart, den Eintrag unter „Apps“, den Installationsvorschlag und den gespeicherten
/// Einrichtungszustand in diesen Ordner um – Einrichtung, Update und Deinstallation lassen
/// sich so vollständig durchspielen, ohne das System zu verändern.
pub fn sandbox() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var_os("VISUTEX_SETUP_SANDBOX")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn same_path(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    normalize(left) == normalize(right)
}

pub fn load_state(data_dir: &Path) -> Option<SetupState> {
    let text = std::fs::read_to_string(data_dir.join(STATE_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_state(data_dir: &Path, state: &SetupState) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    std::fs::write(data_dir.join(STATE_FILE), text)
        .map_err(|error| format!("Einrichtung konnte nicht gespeichert werden: {error}"))
}

pub fn status(data_dir: &Path) -> SetupStatus {
    let exe = current_exe().unwrap_or_default();
    let state = load_state(data_dir);
    let installed_exe = state
        .as_ref()
        .filter(|state| state.mode == "installed")
        .and_then(|state| state.install_dir.as_ref())
        .map(|dir| PathBuf::from(dir).join(exe_name()));
    let running_installed = installed_exe
        .as_ref()
        .is_some_and(|installed| same_path(installed, &exe));
    let packaged = is_packaged(&exe);
    let uninstall_requested = std::env::args().any(|arg| arg == "--uninstall");
    // Entwicklung: nur mit VISUTEX_SETUP=1 (zum Testen der Oberfläche)
    let development = cfg!(debug_assertions)
        && std::env::var_os("VISUTEX_SETUP").is_none()
        && sandbox().is_none();
    let show_setup = !development
        && !packaged
        && !uninstall_requested
        && match &state {
            None => true,
            Some(state) if state.mode == "portable" => false,
            // installiert, aber eine andere (z. B. neu heruntergeladene) EXE gestartet
            Some(_) => !running_installed,
        };
    SetupStatus {
        version: crate::updates::current_version().to_string(),
        platform: platform().into(),
        current_exe: exe.display().to_string(),
        default_dir: default_dir().display().to_string(),
        state,
        running_installed,
        packaged,
        show_setup,
        uninstall_requested,
        sandbox: sandbox().is_some(),
    }
}

/// Ohne Installation weiterarbeiten (Entscheidung merken).
pub fn use_portable(data_dir: &Path) -> Result<(), String> {
    save_state(
        data_dir,
        &SetupState {
            mode: "portable".into(),
            ..SetupState::default()
        },
    )
}

/// Installiert die laufende EXE; liefert den Pfad der installierten EXE (zum Neustart).
pub fn install(options: &InstallOptions, data_dir: &Path) -> Result<PathBuf, String> {
    let source = current_exe().ok_or("Programmdatei nicht gefunden.")?;
    let dir = PathBuf::from(options.dir.trim());
    if options.dir.trim().is_empty() || !dir.is_absolute() {
        return Err("Bitte einen vollständigen Installationsordner angeben.".into());
    }
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "Ordner „{}“ konnte nicht angelegt werden: {error}",
            dir.display()
        )
    })?;
    let target = dir.join(exe_name());
    if !same_path(&source, &target) {
        copy_replacing(&source, &target)?;
    }
    // Alte Einträge (anderer Ort) zuerst entfernen
    if let Some(previous) = load_state(data_dir) {
        remove_integration(&previous);
    }
    let state = SetupState {
        mode: "installed".into(),
        install_dir: Some(dir.display().to_string()),
        desktop_shortcut: options.desktop_shortcut,
        start_menu: options.start_menu,
        autostart: options.autostart,
        version: Some(crate::updates::current_version().to_string()),
    };
    apply_integration(&state, &target)?;
    save_state(data_dir, &state)?;
    Ok(target)
}

/// Kopiert die EXE; eine laufende Zieldatei wird beiseitegelegt (`.old`, beim nächsten Start gelöscht).
pub fn copy_replacing(source: &Path, target: &Path) -> Result<(), String> {
    let temporary = target.with_extension("neu");
    std::fs::copy(source, &temporary)
        .map_err(|error| format!("Programm konnte nicht kopiert werden: {error}"))?;
    replace_file(&temporary, target)
}

/// Ersetzt `target` durch `new_file` – auch wenn `target` gerade läuft (Windows: umbenennen).
pub fn replace_file(new_file: &Path, target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(new_file, std::fs::Permissions::from_mode(0o755));
    }
    if target.exists() {
        let old = target.with_extension("old");
        let _ = std::fs::remove_file(&old);
        if std::fs::rename(target, &old).is_err() {
            std::fs::remove_file(target).map_err(|error| {
                format!(
                    "„{}“ kann nicht ersetzt werden (läuft das Programm noch?): {error}",
                    target.display()
                )
            })?;
        }
    }
    std::fs::rename(new_file, target)
        .map_err(|error| format!("Programm konnte nicht abgelegt werden: {error}"))
}

/// Reste eines früheren Updates/einer Installation aufräumen (`*.old`, `*.neu`). Direkt nach
/// einem Update läuft die alte Version noch kurz (Datei gesperrt) → im Hintergrund bis zu
/// 20 s lang erneut versuchen.
pub fn cleanup_leftovers() {
    let Some(exe) = current_exe() else {
        return;
    };
    let files: Vec<PathBuf> = ["old", "neu"]
        .iter()
        .map(|extension| exe.with_extension(extension))
        .filter(|file| file.exists())
        .collect();
    if files.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..20 {
            let remaining = files
                .iter()
                .filter(|file| file.exists() && std::fs::remove_file(file).is_err())
                .count();
            if remaining == 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

/// Nach einem Update (neue Version läuft als installierte Kopie): Eintrag unter „Apps“
/// mit der neuen Versionsnummer schreiben. Läuft beim Start, kostet sonst nichts.
pub fn refresh_after_update(data_dir: &Path) {
    let Some(mut state) = load_state(data_dir) else {
        return;
    };
    let current = crate::updates::current_version();
    if state.mode != "installed" || state.version.as_deref() == Some(current) {
        return;
    }
    let Some(exe) = current_exe() else {
        return;
    };
    let installed = state
        .install_dir
        .as_ref()
        .is_some_and(|dir| same_path(&PathBuf::from(dir).join(exe_name()), &exe));
    if installed && register_uninstall(&exe).is_ok() {
        state.version = Some(current.to_string());
        let _ = save_state(data_dir, &state);
    }
}

/// Autostart nachträglich ändern (Optionen).
pub fn set_autostart(data_dir: &Path, enabled: bool) -> Result<(), String> {
    let mut state = load_state(data_dir).unwrap_or_default();
    let exe = current_exe().ok_or("Programmdatei nicht gefunden.")?;
    autostart(enabled, &exe)?;
    state.autostart = enabled;
    if state.mode.is_empty() {
        state.mode = "portable".into();
    }
    save_state(data_dir, &state)
}

/// Deinstallieren: Verknüpfungen, Autostart, Eintrag in „Apps“ und eingebettete Dateien im
/// Cache entfernen. Nach dem Beenden werden nur die eigenen Programmdateien gelöscht und der
/// Ordner nur, wenn er danach leer ist – nie fremde Dateien (z. B. bei Installation nach `D:\`).
pub fn uninstall(data_dir: &Path, cache_dir: &Path) -> Result<(), String> {
    let state = load_state(data_dir).unwrap_or_default();
    remove_integration(&state);
    if sandbox().is_none() {
        let _ = std::fs::remove_dir_all(cache_dir.join("mitgeliefert"));
    }
    let _ = std::fs::remove_file(data_dir.join(STATE_FILE));
    if let Some(dir) = state.install_dir.filter(|_| state.mode == "installed") {
        let dir = PathBuf::from(dir);
        delete_after_exit(&program_files(&dir), &dir);
    }
    Ok(())
}

/// Dateien, die VisuTeX im Installationsordner anlegt.
fn program_files(dir: &Path) -> Vec<PathBuf> {
    let exe = dir.join(exe_name());
    vec![
        exe.with_extension("old"),
        exe.with_extension("neu"),
        dir.join("visutex.png"),
        exe,
    ]
}

// ------------------------------------------------------------------ Integration ins System

fn apply_integration(state: &SetupState, exe: &Path) -> Result<(), String> {
    if state.desktop_shortcut {
        shortcut(ShortcutPlace::Desktop, exe)?;
    }
    if state.start_menu {
        shortcut(ShortcutPlace::StartMenu, exe)?;
    }
    autostart(state.autostart, exe)?;
    register_uninstall(exe)?;
    Ok(())
}

fn remove_integration(state: &SetupState) {
    for place in [ShortcutPlace::Desktop, ShortcutPlace::StartMenu] {
        if let Some(path) = shortcut_path(place) {
            let _ = std::fs::remove_file(path);
        }
    }
    if let Some(exe) = current_exe() {
        let _ = autostart(false, &exe);
    }
    let _ = unregister_uninstall();
    let _ = state;
}

#[derive(Clone, Copy)]
enum ShortcutPlace {
    Desktop,
    StartMenu,
}

// Hüllen: im Testmodus Dateien im Testordner statt Desktop, Startmenü und Registrierung.

fn shortcut_path(place: ShortcutPlace) -> Option<PathBuf> {
    let Some(root) = sandbox() else {
        return system_shortcut_path(place);
    };
    let folder = root.join(match place {
        ShortcutPlace::Desktop => "Desktop",
        ShortcutPlace::StartMenu => "Startmenü",
    });
    let _ = std::fs::create_dir_all(&folder);
    Some(folder.join(if cfg!(windows) {
        "VisuTeX.lnk"
    } else {
        "visutex.desktop"
    }))
}

fn sandbox_marker(name: &str, content: Option<String>) -> Result<(), String> {
    let Some(root) = sandbox() else {
        return Ok(());
    };
    let path = root.join(name);
    match content {
        Some(text) => {
            std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
            std::fs::write(path, text).map_err(|error| error.to_string())
        }
        None => {
            let _ = std::fs::remove_file(path);
            Ok(())
        }
    }
}

fn autostart(enabled: bool, exe: &Path) -> Result<(), String> {
    if sandbox().is_some() {
        return sandbox_marker("autostart.txt", enabled.then(|| exe.display().to_string()));
    }
    system_autostart(enabled, exe)
}

fn register_uninstall(exe: &Path) -> Result<(), String> {
    if sandbox().is_some() {
        return sandbox_marker(
            "apps-eintrag.txt",
            Some(format!(
                "DisplayVersion={}\nUninstallString=\"{}\" --uninstall\n",
                crate::updates::current_version(),
                exe.display()
            )),
        );
    }
    system_register_uninstall(exe)
}

fn unregister_uninstall() -> Result<(), String> {
    if sandbox().is_some() {
        return sandbox_marker("apps-eintrag.txt", None);
    }
    system_unregister_uninstall()
}

#[cfg(windows)]
fn hidden(command: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(windows)]
fn powershell(script: &str) -> Result<String, String> {
    let output = hidden(Command::new("powershell").args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]))
    .output()
    .map_err(|error| format!("PowerShell nicht verfügbar: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(windows)]
fn ps_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

#[cfg(windows)]
fn special_folder(name: &str) -> Option<PathBuf> {
    powershell(&format!("[Environment]::GetFolderPath('{name}')"))
        .ok()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

#[cfg(windows)]
fn system_shortcut_path(place: ShortcutPlace) -> Option<PathBuf> {
    let folder = match place {
        ShortcutPlace::Desktop => special_folder("Desktop")?,
        ShortcutPlace::StartMenu => special_folder("Programs")?,
    };
    Some(folder.join(format!("{APP_NAME}.lnk")))
}

#[cfg(windows)]
fn shortcut(place: ShortcutPlace, exe: &Path) -> Result<(), String> {
    let path = shortcut_path(place).ok_or("Ordner für Verknüpfungen nicht gefunden.")?;
    let exe_text = exe.display().to_string();
    let dir_text = exe
        .parent()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    powershell(&format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut({}); $s.TargetPath = {}; $s.WorkingDirectory = {}; $s.IconLocation = {}; $s.Description = 'VisuTeX – LaTeX-Editor'; $s.Save()",
        ps_quote(&path.display().to_string()),
        ps_quote(&exe_text),
        ps_quote(&dir_text),
        ps_quote(&format!("{exe_text},0")),
    ))
    .map(|_| ())
    .map_err(|error| format!("Verknüpfung konnte nicht angelegt werden: {error}"))
}

#[cfg(windows)]
fn reg(args: &[&str]) -> Result<(), String> {
    let output = hidden(Command::new("reg").args(args))
        .output()
        .map_err(|error| format!("Registrierung nicht erreichbar: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
/// Eigener Schlüsselname – nicht `…\Uninstall\VisuTeX`: den verwendet die Setup.exe (NSIS);
/// bei gleichem Namen überschrieben bzw. löschten sich die beiden Installationsarten gegenseitig.
#[cfg(windows)]
const UNINSTALL_KEY: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\com.mathias-lampert.visutex";

#[cfg(windows)]
fn system_autostart(enabled: bool, exe: &Path) -> Result<(), String> {
    if enabled {
        let value = format!("\"{}\"", exe.display());
        reg(&[
            "add", RUN_KEY, "/v", APP_NAME, "/t", "REG_SZ", "/d", &value, "/f",
        ])
        .map_err(|error| format!("Autostart konnte nicht eingerichtet werden: {error}"))
    } else {
        let _ = reg(&["delete", RUN_KEY, "/v", APP_NAME, "/f"]);
        Ok(())
    }
}

#[cfg(windows)]
fn system_register_uninstall(exe: &Path) -> Result<(), String> {
    let exe_text = exe.display().to_string();
    let dir = exe
        .parent()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    let size_kb = std::fs::metadata(exe)
        .map(|metadata| metadata.len() / 1024)
        .unwrap_or_default()
        .to_string();
    let uninstall = format!("\"{exe_text}\" --uninstall");
    let version = crate::updates::current_version().to_string();
    for (name, kind, value) in [
        ("DisplayName", "REG_SZ", APP_NAME),
        ("DisplayVersion", "REG_SZ", version.as_str()),
        ("Publisher", "REG_SZ", "Mathias Lampert"),
        ("DisplayIcon", "REG_SZ", exe_text.as_str()),
        ("InstallLocation", "REG_SZ", dir.as_str()),
        ("UninstallString", "REG_SZ", uninstall.as_str()),
        ("EstimatedSize", "REG_DWORD", size_kb.as_str()),
        ("NoModify", "REG_DWORD", "1"),
        ("NoRepair", "REG_DWORD", "1"),
    ] {
        reg(&[
            "add",
            UNINSTALL_KEY,
            "/v",
            name,
            "/t",
            kind,
            "/d",
            value,
            "/f",
        ])
        .map_err(|error| format!("Eintrag unter „Apps“ fehlgeschlagen: {error}"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn system_unregister_uninstall() -> Result<(), String> {
    reg(&["delete", UNINSTALL_KEY, "/f"])
}

#[cfg(windows)]
fn delete_after_exit(files: &[PathBuf], dir: &Path) {
    // kurz warten, bis die App beendet ist; dann eigene Dateien und den Ordner, falls leer
    let list = files
        .iter()
        .map(|file| ps_quote(&file.display().to_string()))
        .collect::<Vec<_>>()
        .join(",");
    let dir = ps_quote(&dir.display().to_string());
    let script = format!(
        "Start-Sleep -Seconds 3; Remove-Item -LiteralPath {list} -Force -ErrorAction SilentlyContinue; \
         if (-not (Get-ChildItem -LiteralPath {dir} -Force -ErrorAction SilentlyContinue)) {{ \
         Remove-Item -LiteralPath {dir} -Force -ErrorAction SilentlyContinue }}"
    );
    let _ = hidden(Command::new("powershell").args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ]))
    .spawn();
}

// ---- Linux (XDG)

#[cfg(not(windows))]
fn desktop_dir() -> PathBuf {
    Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("Desktop"))
}

#[cfg(not(windows))]
fn system_shortcut_path(place: ShortcutPlace) -> Option<PathBuf> {
    Some(match place {
        ShortcutPlace::Desktop => desktop_dir().join("visutex.desktop"),
        ShortcutPlace::StartMenu => home()
            .join(".local")
            .join("share")
            .join("applications")
            .join("visutex.desktop"),
    })
}

#[cfg(not(windows))]
fn desktop_entry(exe: &Path) -> String {
    let icon = exe
        .parent()
        .map(|dir| dir.join("visutex.png"))
        .unwrap_or_default();
    if let Some(parent) = icon.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&icon, ICON_PNG);
    format!(
        "[Desktop Entry]\nType=Application\nName=VisuTeX\nComment=LaTeX-Editor\nExec=\"{}\" %f\nIcon={}\nTerminal=false\nCategories=Office;Publishing;\nMimeType=text/x-tex;\n",
        exe.display(),
        icon.display()
    )
}

#[cfg(not(windows))]
fn write_desktop_file(path: &Path, exe: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, desktop_entry(exe))
        .map_err(|error| format!("Verknüpfung konnte nicht angelegt werden: {error}"))?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    Ok(())
}

#[cfg(not(windows))]
fn shortcut(place: ShortcutPlace, exe: &Path) -> Result<(), String> {
    let path = shortcut_path(place).ok_or("Ordner für Verknüpfungen nicht gefunden.")?;
    write_desktop_file(&path, exe)
}

#[cfg(not(windows))]
fn system_autostart(enabled: bool, exe: &Path) -> Result<(), String> {
    let path = home()
        .join(".config")
        .join("autostart")
        .join("visutex.desktop");
    if enabled {
        write_desktop_file(&path, exe)
    } else {
        let _ = std::fs::remove_file(path);
        Ok(())
    }
}

#[cfg(not(windows))]
fn system_register_uninstall(_exe: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
fn system_unregister_uninstall() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
fn delete_after_exit(files: &[PathBuf], dir: &Path) {
    let quote = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
    let list = files
        .iter()
        .map(|file| quote(file))
        .collect::<Vec<_>>()
        .join(" ");
    // rmdir entfernt den Ordner nur, wenn er leer ist
    let _ = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "sleep 3; rm -f {list}; rmdir {} 2>/dev/null",
            quote(dir)
        ))
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trip_and_portable_choice() {
        let dir = std::env::temp_dir().join(format!("visutex-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_state(&dir).is_none());
        use_portable(&dir).unwrap();
        assert_eq!(load_state(&dir).unwrap().mode, "portable");
        let state = SetupState {
            mode: "installed".into(),
            install_dir: Some("C:/x".into()),
            desktop_shortcut: true,
            start_menu: true,
            autostart: false,
            version: Some("0.2.0".into()),
        };
        save_state(&dir, &state).unwrap();
        assert_eq!(load_state(&dir), Some(state));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_files_even_if_target_exists() {
        let dir = std::env::temp_dir().join(format!("visutex-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("app.bin");
        std::fs::write(&target, b"alt").unwrap();
        let source = dir.join("neu.bin");
        std::fs::write(&source, b"neu").unwrap();
        copy_replacing(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"neu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn uninstall_removes_only_own_files() {
        let dir = Path::new("D:\\");
        for file in program_files(dir) {
            assert_eq!(file.parent(), Some(dir));
            assert!(file
                .file_name()
                .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("visutex")));
        }
    }

    #[test]
    fn default_dir_is_per_user() {
        let dir = default_dir().display().to_string();
        assert!(dir.ends_with("VisuTeX"), "{dir}");
    }
}
