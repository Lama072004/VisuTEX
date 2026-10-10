//! Gespeicherte Datei teilen.
//!
//! Windows: das Teilen-Fenster des Systems (E-Mail, Teams, Nearby Sharing, OneDrive …) über
//! `IDataTransferManagerInterop` – funktioniert auch für Programme ohne App-Paket.
//! Linux: E-Mail-Programm mit Anhang über `xdg-email --attach` (freedesktop.org).
//!
//! Geteilt wird immer die Datei auf dem Datenträger, also die zuletzt gespeicherte Fassung –
//! das sagt die Oberfläche dem Nutzer vorher ausdrücklich.

use std::path::Path;

/// Prüft die Datei und liefert Name und Pfad als Text.
fn checked(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Err(format!(
            "„{}“ wurde nicht gefunden – bitte zuerst speichern.",
            path.display()
        ));
    }
    Ok(path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string()))
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;
    use std::sync::Mutex;
    use windows::core::{Interface, HSTRING};
    use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
    use windows::Foundation::TypedEventHandler;
    use windows::Storage::{IStorageItem, StorageFile};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::IDataTransferManagerInterop;
    use windows_collections::IIterable;

    /// Zuletzt angemeldeter Handler (wird beim nächsten Teilen ersetzt).
    static HANDLER: Mutex<Option<i64>> = Mutex::new(None);

    fn interop() -> windows::core::Result<IDataTransferManagerInterop> {
        windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()
    }

    /// Teilen-Fenster für `hwnd` öffnen (im Hauptthread des Fensters aufrufen).
    pub fn show(hwnd: isize, path: PathBuf, title: String) -> Result<(), String> {
        let hwnd = HWND(hwnd as *mut core::ffi::c_void);
        let failed =
            |error: windows::core::Error| format!("Teilen nicht möglich: {}", error.message());
        let interop = interop().map_err(failed)?;
        let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd) }.map_err(failed)?;
        if let Ok(mut token) = HANDLER.lock() {
            if let Some(previous) = token.take() {
                let _ = manager.RemoveDataRequested(previous);
            }
        }
        let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
            move |_, args| {
                let Some(args) = args.as_ref() else {
                    return Ok(());
                };
                let request = args.Request()?;
                let data = request.Data()?;
                data.Properties()?
                    .SetTitle(&HSTRING::from(title.as_str()))?;
                // Die Datei liegt lokal vor – das Öffnen ist schnell
                let file =
                    StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.get()?;
                let items: IIterable<IStorageItem> =
                    IIterable::from(vec![Some(file.cast::<IStorageItem>()?)]);
                data.SetStorageItemsReadOnly(&items)?;
                Ok(())
            },
        );
        let token = manager.DataRequested(&handler).map_err(failed)?;
        if let Ok(mut slot) = HANDLER.lock() {
            *slot = Some(token);
        }
        unsafe { interop.ShowShareUIForWindow(hwnd) }.map_err(failed)
    }
}

/// Teilen-Fenster (Windows) bzw. E-Mail mit Anhang (Linux) für die gespeicherte Datei öffnen.
pub fn share(window: &tauri::WebviewWindow, path: &Path) -> Result<String, String> {
    let name = checked(path)?;
    share_platform(window, path, &name)?;
    Ok(name)
}

#[cfg(windows)]
fn share_platform(window: &tauri::WebviewWindow, path: &Path, name: &str) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("Fenster nicht verfügbar: {error}"))?
        .0 as isize;
    let (sender, receiver) = std::sync::mpsc::channel();
    let path = path.to_path_buf();
    let title = name.to_string();
    window
        .run_on_main_thread(move || {
            let _ = sender.send(platform::show(hwnd, path, title));
        })
        .map_err(|error| format!("Teilen nicht möglich: {error}"))?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(20))
        .map_err(|_| "Das Teilen-Fenster hat nicht geantwortet.".to_string())?
}

#[cfg(not(windows))]
fn share_platform(_window: &tauri::WebviewWindow, path: &Path, _name: &str) -> Result<(), String> {
    let status = std::process::Command::new("xdg-email")
        .arg("--attach")
        .arg(path)
        .status()
        .map_err(|_| {
            "Teilen ist hier nicht verfügbar (xdg-email fehlt). Die Datei lässt sich über „Im Ordner anzeigen“ weitergeben.".to_string()
        })?;
    if status.success() {
        Ok(())
    } else {
        Err("Das E-Mail-Programm konnte nicht geöffnet werden.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_missing_files() {
        let missing = std::env::temp_dir().join("visutex-gibt-es-nicht.visutex");
        assert!(checked(&missing).unwrap_err().contains("zuerst speichern"));
        let dir = std::env::temp_dir().join(format!("visutex-share-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Bericht.tex");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(checked(&file).unwrap(), "Bericht.tex");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
