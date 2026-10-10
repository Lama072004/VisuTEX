//! VisuTeX – Tauri-Setup und dünne Command-Schicht.
//!
//! Die Logik liegt in `core/*` (reine, getestete LaTeX-Logik) und den Modulen
//! für Kompilieren, PDF, Dateien, Bilder, Zotero und Add-ons. Alle Commands, die
//! Dateien oder Tectonic benutzen, laufen in `spawn_blocking`, damit die
//! Oberfläche nie einfriert.

pub mod addons;
pub mod compile;
pub mod core;
pub mod embedded;
pub mod files;
pub mod fonts;
pub mod images;
pub mod pdf;
pub mod prepare;
pub mod preview;
pub mod setup;
pub mod share;
pub mod system;
pub mod templates;
pub mod texbundle;
pub mod updates;
pub mod zotero;

use crate::core::analysis::{self, Analysis};
use crate::core::bibtex::{self, BibEntry};
use crate::core::export::{
    export_document as export_core, EmbeddedAsset, ExportOptions, ExportResult,
};
use crate::core::import::{import_latex as import_core, ImportResult};
use crate::core::markers::same_latex;
use crate::core::project::{self, LoadedProject, Project};
use crate::core::settings::{is_safe_relative_path, merge_json, DocumentSettings};
use addons::{AddonDirs, InstalledAddon};
use compile::{CompileFailure, CompileRequest, TexMessage};
use files::FileStamp;
use pdf::{PageSize, PdfStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use texbundle::{BundleConfig, BundleInfo};

#[derive(Default)]
struct AppState {
    pdfs: PdfStore,
    tex: std::sync::Mutex<TexOptions>,
}

/// Einstellungen der TeX-Engine aus den App-Optionen (vom Frontend gesetzt).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TexOptions {
    allow_online: bool,
    /// Vom Nutzer gewählter Speicherort für nachgeladene Pakete.
    package_cache_dir: Option<String>,
    /// Eigene Bundle-Datei statt der mitgelieferten.
    bundle_path: Option<String>,
}

// ---------------------------------------------------------------- Hilfsfunktionen

async fn blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("Interner Fehler: {error}"))?
}

fn resource_path(app: &AppHandle, relative: &str) -> Option<PathBuf> {
    app.path()
        .resolve(relative, tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|path| path.exists())
}

/// Cache-Ordner der App (hier werden eingebettete Dateien beim ersten Bedarf entpackt).
fn cache_dir_root(app: &AppHandle) -> PathBuf {
    app.path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("visutex"))
}

fn tex_options(app: &AppHandle) -> TexOptions {
    app.state::<AppState>()
        .tex
        .lock()
        .map(|options| options.clone())
        .unwrap_or_default()
}

/// Bundle-Konfiguration: eigenes Bundle (falls gewählt) vor dem mitgelieferten;
/// nachgeladene Pakete im gewählten Ordner bzw. im App-Datenordner (`tex-pakete`).
fn bundle_config(app: &AppHandle, allow_online: bool) -> Result<BundleConfig, String> {
    let options = tex_options(app);
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Cache-Ordner nicht verfügbar: {error}"))?;
    let custom_bundle = options
        .bundle_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let online_cache = match options
        .package_cache_dir
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        Some(path) => Some(PathBuf::from(path)),
        None => app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.join("tex-pakete")),
    };
    Ok(BundleConfig {
        // eigenes Bundle → in die EXE eingebettetes → Ordner `resources` (Entwicklung)
        embedded_zip: custom_bundle
            .or_else(|| embedded::tex_bundle(&cache_dir_root(app)))
            .or_else(|| resource_path(app, &format!("resources/{}", texbundle::BUNDLE_FILE_NAME))),
        allow_online: allow_online || options.allow_online,
        cache_dir,
        online_cache,
    })
}

fn addon_dirs(app: &AppHandle) -> Result<AddonDirs, String> {
    let user = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Datenordner nicht verfügbar: {error}"))?
        .join("addons");
    std::fs::create_dir_all(&user)
        .map_err(|error| format!("Add-on-Ordner konnte nicht angelegt werden: {error}"))?;
    Ok(AddonDirs {
        user,
        builtin: embedded::addons(&cache_dir_root(app))
            .or_else(|| resource_path(app, "resources/addons")),
    })
}

fn ensure_project_root(path: &Option<String>) -> Result<Option<PathBuf>, String> {
    match path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        None => Ok(None),
        Some(path) => {
            let path = PathBuf::from(path);
            std::fs::create_dir_all(&path).map_err(|error| {
                format!(
                    "Projektordner „{}“ nicht verfügbar: {error}",
                    path.display()
                )
            })?;
            Ok(Some(path))
        }
    }
}

fn parent_dir(path: &Path) -> Result<PathBuf, String> {
    path.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Ungültiger Dateipfad.".to_string())
}

fn allow_asset_directory(app: &AppHandle, root: &Path) {
    let _ = app.asset_protocol_scope().allow_directory(root, true);
}

fn new_temp_root(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Cache-Ordner nicht verfügbar: {error}"))?
        .join("ungespeichert");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    // Alte ungespeicherte Projekte (älter als 14 Tage) entfernen.
    if let Ok(entries) = std::fs::read_dir(&base) {
        let limit = std::time::Duration::from_secs(14 * 24 * 3600);
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let old = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age > limit);
            if old && path.is_dir() && entry.file_name().to_string_lossy().starts_with("projekt-") {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
    let root = base.join(format!("projekt-{stamp}"));
    std::fs::create_dir_all(&root).map_err(|error| {
        format!("Temporärer Projektordner konnte nicht angelegt werden: {error}")
    })?;
    allow_asset_directory(app, &root);
    Ok(root)
}

/// Legt eine fehlende Literaturdatei als leere .bib an.
fn ensure_bib_file(root: &Path, settings: &DocumentSettings) {
    let file = settings.bibliography.file.trim();
    if !file.is_empty() && is_safe_relative_path(file) {
        let path = root.join(file);
        if !path.exists() {
            let _ = files::write_text_atomic(
                &path,
                "% Literaturdatenbank (BibTeX) – Einträge über „Referenzen → Zitat einfügen“ oder Zotero hinzufügen.\n",
            );
        }
    }
}

// ---------------------------------------------------------------- Allgemein

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    bundle: BundleInfo,
    available_fonts: Vec<String>,
}

#[tauri::command]
async fn app_info(app: AppHandle, allow_online: bool) -> Result<AppInfo, String> {
    let config = bundle_config(&app, allow_online)?;
    let version = updates::current_version().to_string();
    blocking(move || {
        let available_fonts = core::settings::AVAILABLE_FONTS
            .iter()
            .filter(|font| **font == core::settings::DEFAULT_FONT || fonts::is_installed(font))
            .map(|font| font.to_string())
            .collect();
        Ok(AppInfo {
            version,
            bundle: texbundle::bundle_info(&config),
            available_fonts,
        })
    })
    .await
}

#[tauri::command]
fn set_tex_options(state: State<'_, AppState>, options: TexOptions) {
    if let Ok(mut current) = state.tex.lock() {
        *current = options;
    }
}

/// Systemprüfung: mitgelieferte und gefundene Komponenten, mit Lösungsvorschlag.
#[tauri::command]
async fn system_check(app: AppHandle) -> Result<system::SystemCheck, String> {
    let bundle = bundle_config(&app, false)?;
    let data_dir = app.path().app_data_dir().ok();
    let webview_version = tauri::webview_version().map_err(|error| {
        format!("Die WebView-Laufzeit wurde nicht gefunden ({error}). Unter Windows bitte die Microsoft-Edge-WebView2-Laufzeit installieren; unter Linux das Paket webkit2gtk-4.1.")
    });
    blocking(move || {
        Ok(system::run(system::CheckInput {
            bundle,
            data_dir,
            webview_version,
            check_zotero: true,
        }))
    })
    .await
}

// ---------------------------------------------------------------- LaTeX-Kern

#[tauri::command]
async fn export_document(
    app: AppHandle,
    document: Value,
    settings: Value,
    custom_preamble: Option<String>,
) -> Result<ExportResult, String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || {
        let settings = DocumentSettings::from_value(&settings);
        let addon_preamble = dirs.preamble();
        Ok(export_core(
            &document,
            &ExportOptions {
                settings: &settings,
                custom_preamble: custom_preamble.as_deref(),
                addon_preamble: &addon_preamble,
            },
        ))
    })
    .await
}

#[tauri::command]
async fn import_latex(source: String, settings: Value) -> Result<ImportResult, String> {
    blocking(move || {
        Ok(import_core(
            &source,
            &DocumentSettings::from_value(&settings),
        ))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncResult {
    document: Value,
    settings: DocumentSettings,
    custom_preamble: Option<String>,
    warnings: Vec<String>,
    raw_block_count: usize,
    missing_resources: Vec<String>,
}

/// Code-Ansicht → visuelle Ansicht. Entspricht die Präambel der beim Wechsel
/// erzeugten, bleibt sie „verwaltet“; sonst wird sie zur eigenen Präambel.
#[tauri::command]
async fn code_to_document(
    code: String,
    settings: Value,
    generated_preamble: Option<String>,
    custom_preamble: Option<String>,
    project_root: Option<String>,
) -> Result<SyncResult, String> {
    let root = project_root_opt(&project_root);
    blocking(move || {
        let mut settings = DocumentSettings::from_value(&settings);
        let imported = import_core(&code, &settings);
        let mut custom = custom_preamble.filter(|text| !text.trim().is_empty());
        if let Some(preamble) = &imported.preamble {
            let generated = generated_preamble.as_deref().unwrap_or("");
            if same_latex(preamble, generated) {
                custom = None;
            } else if custom
                .as_deref()
                .is_none_or(|known| !same_latex(known, preamble))
            {
                let mut merged =
                    serde_json::to_value(&settings).map_err(|error| error.to_string())?;
                merge_json(&mut merged, &imported.settings_patch);
                settings = DocumentSettings::from_value(&merged);
                custom = Some(preamble.trim_end().to_string());
            }
        }
        let missing_resources = match &root {
            Some(root) => files::missing_resources(&code, root)
                .into_iter()
                .map(|resource| resource.path)
                .collect(),
            None => Vec::new(),
        };
        Ok(SyncResult {
            document: imported.doc,
            settings,
            custom_preamble: custom,
            warnings: imported.warnings,
            raw_block_count: imported.raw_block_count,
            missing_resources,
        })
    })
    .await
}

fn project_root_opt(path: &Option<String>) -> Option<PathBuf> {
    path.as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

/// Skizze (Raster-Modell) → TikZ/CircuiTikZ-Code.
#[tauri::command]
fn sketch_to_latex(sketch: core::sketch::Sketch) -> core::sketch::SketchCode {
    core::sketch::to_latex(&sketch)
}

/// Skizzenmodell aus dem Code eines TikZ-Blocks (zum erneuten Bearbeiten).
#[tauri::command]
fn sketch_from_code(code: String) -> Option<core::sketch::Sketch> {
    core::sketch::from_code(&code)
}

// ------------------------------------------------------------------ Einrichtung und Updates

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(root) = setup::sandbox() {
        return Ok(root.join("daten"));
    }
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Datenordner nicht verfügbar: {error}"))
}

/// Startet `exe` neu und beendet diese Instanz (nach kurzer Pause, damit die Antwort ankommt).
fn restart_into(app: &AppHandle, exe: PathBuf) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if std::process::Command::new(&exe).spawn().is_ok() {
            app.exit(0);
        }
    });
}

#[tauri::command]
async fn setup_status(app: AppHandle) -> Result<setup::SetupStatus, String> {
    let dir = data_dir(&app)?;
    blocking(move || Ok(setup::status(&dir))).await
}

/// Installieren und die installierte Kopie starten.
#[tauri::command]
async fn setup_install(app: AppHandle, options: setup::InstallOptions) -> Result<String, String> {
    let dir = data_dir(&app)?;
    let exe = blocking(move || setup::install(&options, &dir)).await?;
    let text = exe.display().to_string();
    restart_into(&app, exe);
    Ok(text)
}

#[tauri::command]
async fn setup_use_portable(app: AppHandle) -> Result<(), String> {
    let dir = data_dir(&app)?;
    blocking(move || setup::use_portable(&dir)).await
}

#[tauri::command]
async fn setup_set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let dir = data_dir(&app)?;
    blocking(move || setup::set_autostart(&dir, enabled)).await
}

/// Deinstallieren und beenden.
#[tauri::command]
async fn setup_uninstall(app: AppHandle) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let cache = cache_dir_root(&app);
    blocking(move || setup::uninstall(&dir, &cache)).await?;
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        handle.exit(0);
    });
    Ok(())
}

/// Gespeicherte Datei teilen (Windows: Teilen-Fenster, Linux: E-Mail mit Anhang).
#[tauri::command]
async fn share_file(window: tauri::WebviewWindow, path: String) -> Result<String, String> {
    blocking(move || share::share(&window, std::path::Path::new(&path))).await
}

#[tauri::command]
async fn update_check() -> Result<updates::UpdateInfo, String> {
    blocking(updates::check).await
}

/// Neue Version laden, Programmdatei ersetzen und neu starten. Fortschritt als Ereignis
/// `update-progress` ({ loaded, total }).
#[tauri::command]
async fn update_install(app: AppHandle, asset: updates::UpdateAsset) -> Result<(), String> {
    let work = cache_dir_root(&app).join("update");
    let emitter = app.clone();
    let restart = blocking(move || {
        let mut last = 0u64;
        updates::install(&asset, &work, &mut |loaded, total| {
            // höchstens etwa alle 1 MB melden
            if loaded - last >= 1 << 20 || loaded == total {
                last = loaded;
                let _ = emitter.emit(
                    "update-progress",
                    serde_json::json!({ "loaded": loaded, "total": total }),
                );
            }
        })
    })
    .await?;
    match restart {
        Some(exe) => restart_into(&app, exe),
        // Setup.exe übernimmt (still) und startet VisuTeX danach selbst neu
        None => {
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(400));
                handle.exit(0);
            });
        }
    }
    Ok(())
}

/// Symbole des Skizzier-Werkzeugs, beim Bauen eingebettet (≈ 0,9 MB) – Rückfall, falls der
/// Ordner `resources` neben dem Programm fehlt.
const EMBEDDED_SKETCH_SYMBOLS: &str = include_str!("../resources/sketch-symbols.json");

/// Alle Symbole des Skizzier-Werkzeugs (CircuiTikZ-Bauteile, TikZ-Formen) mit
/// Vorschaubild und Anschlüssen (aus `resources/sketch-symbols.json`).
#[tauri::command]
async fn sketch_catalog(app: AppHandle) -> Result<core::sketch_catalog::Catalog, String> {
    static CATALOG: std::sync::OnceLock<core::sketch_catalog::Catalog> = std::sync::OnceLock::new();
    if let Some(catalog) = CATALOG.get() {
        return Ok(catalog.clone());
    }
    // Datei im Ordner `resources` (nach `build-sketch-symbols` aktuell); fehlt sie – etwa wenn nur
    // die EXE kopiert wurde –, die beim Bauen eingebettete Fassung verwenden.
    let path = resource_path(&app, "resources/sketch-symbols.json").filter(|path| path.is_file());
    let catalog = blocking(move || {
        let text = match path {
            Some(path) => std::fs::read_to_string(&path)
                .map_err(|error| format!("Symboldatei nicht lesbar: {error}"))?,
            None => EMBEDDED_SKETCH_SYMBOLS.to_string(),
        };
        core::sketch_catalog::build_catalog(&text)
    })
    .await?;
    Ok(CATALOG.get_or_init(|| catalog).clone())
}

/// Eigene Makros einer Präambel (für die Anzeige im Editor).
#[tauri::command]
async fn preamble_macros(preamble: String) -> Result<Vec<core::macros::MacroDef>, String> {
    blocking(move || Ok(core::macros::collect(&preamble))).await
}

#[tauri::command]
async fn analyze_document(document: Value) -> Result<Analysis, String> {
    blocking(move || Ok(analysis::analyze(&document))).await
}

#[tauri::command]
async fn check_resources(
    latex: String,
    project_root: Option<String>,
) -> Result<Vec<files::Resource>, String> {
    let root = project_root_opt(&project_root);
    blocking(move || {
        Ok(match root {
            Some(root) => files::missing_resources(&latex, &root),
            None => Vec::new(),
        })
    })
    .await
}

// ---------------------------------------------------------------- Kompilieren und PDF

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompileResult {
    pdf_id: u64,
    pages: Vec<PageSize>,
    messages: Vec<TexMessage>,
    duration_ms: u64,
    /// Fehlende TeX-Dateien (PDF trotzdem erzeugt)
    missing_files: Vec<String>,
}

#[tauri::command]
async fn compile_document(
    app: AppHandle,
    state: State<'_, AppState>,
    latex: String,
    project_root: Option<String>,
    assets: Vec<EmbeddedAsset>,
    allow_online: bool,
) -> Result<CompileResult, CompileFailure> {
    let fail = |message: String| CompileFailure {
        message,
        messages: Vec::new(),
        log: String::new(),
        missing_files: Vec::new(),
    };
    let config = bundle_config(&app, allow_online).map_err(fail)?;
    let root = ensure_project_root(&project_root).map_err(fail)?;
    let search_paths = addon_dirs(&app).map_err(fail)?.search_paths();
    let handle = app.clone();
    let started = std::time::Instant::now();
    let output = tauri::async_runtime::spawn_blocking(move || {
        if let Some(root) = &root {
            files::write_assets(root, &assets).map_err(fail)?;
            // Fehlende Bilder werden beim Kompilieren durch Platzhalter ersetzt
            // (prepare.rs), fehlende .bib erzeugen nur BibTeX-Warnungen – abbrechen
            // nur bei fehlenden eingebundenen Teildateien.
            let missing: Vec<String> = files::missing_resources(&latex, root)
                .into_iter()
                .filter(|resource| matches!(resource.command.as_str(), "input" | "include"))
                .map(|resource| resource.path)
                .collect();
            if !missing.is_empty() {
                return Err(fail(format!(
                    "Fehlende Dateien im Projektordner „{}“: {}",
                    root.display(),
                    missing.join(", ")
                )));
            }
        }
        let latex = format!("{}{latex}", fonts::hint_prefix(&latex));
        let mut progress = |text: &str| {
            let _ = handle.emit("compile-progress", text.to_string());
        };
        let output = compile::compile(
            CompileRequest {
                latex,
                project_root: root,
                extra_search_paths: search_paths,
            },
            &config,
            &mut progress,
        )?;
        let stored = pdf::load(output.pdf).map_err(fail)?;
        Ok::<_, CompileFailure>((stored, output.messages, output.missing_files))
    })
    .await
    .map_err(|error| fail(format!("Interner Fehler: {error}")))??;
    let (stored, messages, missing_files) = output;
    let (pdf_id, pages) = state.pdfs.insert(stored);
    Ok(CompileResult {
        pdf_id,
        pages,
        messages,
        duration_ms: started.elapsed().as_millis() as u64,
        missing_files,
    })
}

#[tauri::command]
async fn render_pdf_page(
    state: State<'_, AppState>,
    pdf_id: u64,
    page: usize,
    scale: f32,
) -> Result<tauri::ipc::Response, String> {
    let stored = state.pdfs.get(pdf_id)?;
    let png = blocking(move || pdf::render_page_png(&stored, page, scale)).await?;
    Ok(tauri::ipc::Response::new(png))
}

#[tauri::command]
async fn save_pdf(state: State<'_, AppState>, pdf_id: u64, path: String) -> Result<(), String> {
    let stored = state.pdfs.get(pdf_id)?;
    blocking(move || files::write_bytes_atomic(Path::new(&path), &stored.bytes)).await
}

/// TikZ-/CircuiTikZ-Vorschau als PNG-data-URL.
#[tauri::command]
async fn render_tikz(
    app: AppHandle,
    code: String,
    environment: String,
    options: String,
    allow_online: bool,
) -> Result<String, String> {
    let trimmed = code.trim().to_string();
    if trimmed.is_empty() {
        return Err("Der TikZ-Code ist leer.".into());
    }
    if trimmed.len() > 100_000 {
        return Err("TikZ-Zeichnungen dürfen höchstens 100.000 Zeichen enthalten.".into());
    }
    let environment = if environment == "circuitikz" {
        "circuitikz"
    } else {
        "tikzpicture"
    };
    let options = options.trim().replace(['\r', '\n'], " ");
    let config = bundle_config(&app, allow_online)?;
    let dirs = addon_dirs(&app)?;
    blocking(move || {
        let addon_preamble = dirs.preamble().join("\n");
        let circuit = if environment == "circuitikz" {
            "\\usepackage[european]{circuitikz}\n"
        } else {
            ""
        };
        let option_text = if options.is_empty() { String::new() } else { format!("[{options}]") };
        let document = format!(
            "\\documentclass[border=4pt]{{standalone}}\n\\usepackage{{amsmath,amssymb}}\n\\usepackage{{tikz}}\n\\usetikzlibrary{{arrows.meta,positioning,calc,shapes.geometric}}\n{circuit}{addon_preamble}\n\\begin{{document}}\n\\begin{{{environment}}}{option_text}\n{trimmed}\n\\end{{{environment}}}\n\\end{{document}}\n"
        );
        let output = compile::compile(
            CompileRequest {
                latex: document,
                project_root: None,
                extra_search_paths: dirs.search_paths(),
            },
            &config,
            &mut |_| {},
        )
        .map_err(|failure| failure.message)?;
        let stored = pdf::load(output.pdf)?;
        let png = pdf::render_page_png(&stored, 0, 2.5)?;
        use base64::Engine;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        ))
    })
    .await
}

/// Vorschau-Bilder für Roh-LaTeX-Blöcke (Tabellen, Titelseiten, eigene Umgebungen …)
/// mit der Präambel des Dokuments.
#[tauri::command]
async fn render_latex_previews(
    app: AppHandle,
    preamble: String,
    blocks: Vec<String>,
    project_root: Option<String>,
    allow_online: bool,
) -> Result<Vec<preview::BlockPreview>, String> {
    let config = bundle_config(&app, allow_online)?;
    let search_paths = addon_dirs(&app)?.search_paths();
    let root = project_root_opt(&project_root);
    blocking(move || {
        Ok(preview::render_blocks(
            &preamble,
            &blocks,
            root.as_deref(),
            &search_paths,
            &config,
        ))
    })
    .await
}

/// Bilddatei wie LaTeX auflösen (Unterordner, \graphicspath, ohne Endung, PDF).
#[tauri::command]
async fn resolve_image(
    app: AppHandle,
    project_root: String,
    path: String,
    preamble: Option<String>,
) -> Result<preview::ResolvedImage, String> {
    let handle = app.clone();
    blocking(move || {
        let root = PathBuf::from(project_root);
        let resolved = preview::resolve_image(&root, &path, preamble.as_deref().unwrap_or(""));
        // Bilder in Unterordnern bzw. per \graphicspath auch für das Asset-Protokoll freigeben
        if let Some(file) = &resolved.file {
            if let Some(parent) = Path::new(file).parent() {
                if parent.starts_with(&root) {
                    allow_asset_directory(&handle, parent);
                }
            }
        }
        Ok(resolved)
    })
    .await
}

// ---------------------------------------------------------------- Projekte und Dateien

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenedProject {
    project: Project,
    /// Projektordner (Bilder, .bib)
    root: String,
    /// Gespeicherte Datei (leer bei Vorlagen/neuen Dokumenten)
    path: Option<String>,
    /// visutex | tex
    kind: String,
    warnings: Vec<String>,
    missing_files: Vec<String>,
    stamp: Option<FileStamp>,
    temporary: bool,
}

fn finish_opened(
    app: &AppHandle,
    loaded: LoadedProject,
    root: PathBuf,
    path: Option<PathBuf>,
    kind: &str,
    temporary: bool,
) -> Result<OpenedProject, String> {
    files::write_assets(&root, &loaded.assets)?;
    allow_asset_directory(app, &root);
    let missing_files =
        project::referenced_files(&loaded.project.document, &loaded.project.settings)
            .into_iter()
            .filter(|file| !root.join(file).is_file())
            .collect();
    Ok(OpenedProject {
        stamp: path.as_deref().and_then(files::stamp),
        project: loaded.project,
        root: root.display().to_string(),
        path: path.map(|path| path.display().to_string()),
        kind: kind.into(),
        warnings: loaded.warnings,
        missing_files,
        temporary,
    })
}

#[tauri::command]
async fn new_project(
    app: AppHandle,
    template: Option<String>,
    addon: Option<String>,
    language: Option<String>,
) -> Result<OpenedProject, String> {
    let root = new_temp_root(&app)?;
    let dirs = addon_dirs(&app)?;
    let handle = app.clone();
    blocking(move || {
        // Leere Dokumente in der Sprache der Oberfläche (Vorlagen behalten ihre Sprache)
        let neutral = matches!(
            template.as_deref(),
            None | Some("leer-bericht" | "leer-artikel")
        ) && addon.is_none();
        let mut loaded = match (template.as_deref(), addon.as_deref()) {
            (Some(template), Some(addon)) => {
                project::parse_project(&dirs.template(addon, template)?)?
            }
            (Some(template), None) => LoadedProject {
                project: templates::build(template)?,
                assets: Vec::new(),
                warnings: Vec::new(),
                migrated: false,
            },
            _ => LoadedProject {
                project: Project::new(DocumentSettings::default(), project::empty_document()),
                assets: Vec::new(),
                warnings: Vec::new(),
                migrated: false,
            },
        };
        if let Some(language) = language.filter(|_| neutral) {
            if core::languages::all()
                .iter()
                .any(|entry| entry.id == language)
            {
                loaded.project.settings.language = language;
            }
        }
        ensure_bib_file(&root, &loaded.project.settings);
        finish_opened(&handle, loaded, root, None, "visutex", true)
    })
    .await
}

#[tauri::command]
async fn load_project(app: AppHandle, path: String) -> Result<OpenedProject, String> {
    let handle = app.clone();
    blocking(move || {
        let path = PathBuf::from(path);
        let root = parent_dir(&path)?;
        let text = files::read_text(&path)?;
        let is_tex = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"));
        if is_tex {
            // Mehrteilige Projekte: \input/\include im Körper einlesen (beim
            // Speichern werden die Teildateien wieder getrennt geschrieben).
            let expanded = core::includes::expand(&text, &root);
            let text = expanded.latex;
            let mut settings = DocumentSettings::default();
            let imported = import_core(&text, &settings);
            let mut merged = serde_json::to_value(&settings).map_err(|error| error.to_string())?;
            merge_json(&mut merged, &imported.settings_patch);
            settings = DocumentSettings::from_value(&merged);
            let mut project = Project::new(settings, imported.doc);
            project.custom_preamble = imported.preamble.map(|text| text.trim_end().to_string());
            project.code = Some(text.clone());
            let mut warnings = imported.warnings;
            warnings.extend(expanded.warnings);
            if !expanded.files.is_empty() {
                warnings.push(format!(
                    "Teildateien eingebunden (werden beim Speichern zurückgeschrieben): {}",
                    expanded.files.join(", ")
                ));
            }
            let missing: Vec<String> = files::missing_resources(&text, &root)
                .into_iter()
                .map(|resource| resource.path)
                .collect();
            if !missing.is_empty() {
                warnings.push(format!(
                    "Fehlende Dateien im Projektordner: {}",
                    missing.join(", ")
                ));
            }
            let loaded = LoadedProject {
                project,
                assets: Vec::new(),
                warnings,
                migrated: false,
            };
            return finish_opened(&handle, loaded, root, Some(path), "tex", false);
        }
        let loaded = project::parse_project(&text)?;
        finish_opened(&handle, loaded, root, Some(path), "visutex", false)
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedFile {
    root: String,
    path: String,
    stamp: Option<FileStamp>,
    missing_files: Vec<String>,
}

#[tauri::command]
async fn save_project(
    app: AppHandle,
    path: String,
    project: Value,
    source_root: Option<String>,
) -> Result<SavedFile, String> {
    let handle = app.clone();
    blocking(move || {
        let path = PathBuf::from(path);
        let root = parent_dir(&path)?;
        let settings =
            DocumentSettings::from_value(project.get("settings").unwrap_or(&Value::Null));
        let document = project
            .get("document")
            .cloned()
            .unwrap_or_else(project::empty_document);
        let text_field = |key: &str| {
            project
                .get(key)
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
        };
        let mut model = Project::new(settings, document);
        model.custom_preamble = text_field("customPreamble");
        model.code = text_field("code");
        if project.get("lastMode").and_then(Value::as_str) == Some("code") {
            model.last_mode = "code".into();
        }
        let mut missing_files = Vec::new();
        if let Some(source) = project_root_opt(&source_root) {
            missing_files = files::copy_project_files(
                &source,
                &root,
                &project::referenced_files(&model.document, &model.settings),
            )?;
        }
        files::write_text_atomic(&path, &project::serialize_project(&model)?)?;
        allow_asset_directory(&handle, &root);
        Ok(SavedFile {
            root: root.display().to_string(),
            stamp: files::stamp(&path),
            path: path.display().to_string(),
            missing_files,
        })
    })
    .await
}

/// Speichert ein vollständiges LaTeX-Dokument und kopiert alle lokalen
/// Ressourcen (Bilder, .bib, eigene Pakete, Add-on-TeX-Dateien) mit.
#[tauri::command]
async fn export_tex(
    app: AppHandle,
    path: String,
    latex: String,
    assets: Vec<EmbeddedAsset>,
    source_root: Option<String>,
) -> Result<SavedFile, String> {
    let dirs = addon_dirs(&app)?;
    let handle = app.clone();
    blocking(move || {
        let path = PathBuf::from(path);
        let root = parent_dir(&path)?;
        files::write_assets(&root, &assets)?;
        let mut missing_files = Vec::new();
        if let Some(source) = project_root_opt(&source_root) {
            files::write_assets(&source, &assets)?;
            let resources = files::local_resource_files(&latex, &source);
            missing_files = files::copy_project_files(&source, &root, &resources)?;
        }
        // Eigene Dateien von VisuTeX (z. B. deutscher IEEE-Stil) mitliefern
        core::support::write_files(&latex, &root, false)?;
        for file in dirs.tex_files() {
            if let Some(name) = file.file_name() {
                let target = root.join(name);
                if !target.exists() {
                    std::fs::copy(&file, &target).map_err(|error| {
                        format!("Add-on-Datei konnte nicht kopiert werden: {error}")
                    })?;
                }
            }
        }
        missing_files.extend(
            files::missing_resources(&latex, &root)
                .into_iter()
                .filter(|resource| {
                    resource.command != "usepackage" && resource.command != "documentclass"
                })
                .map(|resource| resource.path),
        );
        missing_files.dedup();
        // Eingebundene Teildateien (\input/\include) wieder getrennt schreiben –
        // nur, wenn sich ihr Inhalt geändert hat.
        let (main, parts) = core::includes::split(&latex);
        for part in parts {
            let target = root.join(&part.path);
            let unchanged = std::fs::read(&target)
                .map(|existing| existing == part.content.as_bytes())
                .unwrap_or(false);
            if !unchanged {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        format!("Ordner für „{}“ nicht anlegbar: {error}", part.path)
                    })?;
                }
                files::write_text_atomic(&target, &part.content)?;
            }
        }
        files::write_text_atomic(&path, &main)?;
        allow_asset_directory(&handle, &root);
        Ok(SavedFile {
            root: root.display().to_string(),
            stamp: files::stamp(&path),
            path: path.display().to_string(),
            missing_files,
        })
    })
    .await
}

// ---------------------------------------------------------------- Folien-Editor

/// Präsentation → vollständiges Beamer-Dokument (+ eingebettete Bilder).
#[tauri::command]
async fn slides_to_latex(
    deck: core::slides::SlideDeck,
) -> Result<core::slides::SlidesLatex, String> {
    blocking(move || Ok(core::slides::deck_to_latex(&deck))).await
}

#[tauri::command]
async fn slides_load(path: String) -> Result<core::slides::SlideDeck, String> {
    blocking(move || {
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("„{path}“ konnte nicht gelesen werden: {error}"))?;
        core::slides::parse_deck(&String::from_utf8_lossy(&bytes))
    })
    .await
}

#[tauri::command]
async fn slides_save(
    path: String,
    mut deck: core::slides::SlideDeck,
) -> Result<Option<FileStamp>, String> {
    blocking(move || {
        core::slides::sanitize(&mut deck);
        let json = serde_json::to_string_pretty(&deck)
            .map_err(|error| format!("Präsentation nicht speicherbar: {error}"))?;
        let path = PathBuf::from(path);
        files::write_text_atomic(&path, &json)?;
        Ok(files::stamp(&path))
    })
    .await
}

/// Präsentation kompilieren (Beamer). Eingebettete Bilder landen in einem
/// temporären Arbeitsordner, nicht neben der Foliendatei.
#[tauri::command]
async fn slides_compile(
    app: AppHandle,
    state: State<'_, AppState>,
    deck: core::slides::SlideDeck,
    allow_online: bool,
) -> Result<CompileResult, CompileFailure> {
    let exported = blocking(move || Ok(core::slides::deck_to_latex(&deck)))
        .await
        .map_err(|message| CompileFailure {
            message,
            messages: Vec::new(),
            log: String::new(),
            missing_files: Vec::new(),
        })?;
    let root = std::env::temp_dir().join("visutex-folien");
    compile_document(
        app,
        state,
        exported.latex,
        Some(root.display().to_string()),
        exported.assets,
        allow_online,
    )
    .await
}

/// Bilddatei für eine Folie als data:-URL (konvertiert nicht webtaugliche Formate).
#[tauri::command]
async fn slides_read_image(path: String) -> Result<images::SlideImage, String> {
    blocking(move || images::slide_image(Path::new(&path))).await
}

#[tauri::command]
async fn file_stamp(path: String) -> Result<Option<FileStamp>, String> {
    blocking(move || Ok(files::stamp(Path::new(&path)))).await
}

#[tauri::command]
async fn read_text_file(path: String) -> Result<String, String> {
    blocking(move || files::read_text(Path::new(&path))).await
}

#[tauri::command]
fn allow_project_dir(app: AppHandle, root: String) {
    allow_asset_directory(&app, Path::new(&root));
}

// ---------------------------------------------------------------- Bilder

#[tauri::command]
async fn import_image(
    source: String,
    project_root: String,
) -> Result<images::ImportedImage, String> {
    blocking(move || images::import_image_file(Path::new(&source), Path::new(&project_root))).await
}

#[tauri::command]
async fn import_image_data(
    data_url: String,
    project_root: String,
) -> Result<images::ImportedImage, String> {
    blocking(move || images::import_image_data_url(&data_url, Path::new(&project_root))).await
}

// ---------------------------------------------------------------- Literatur

fn bib_path(project_root: &str, file: &str) -> Result<PathBuf, String> {
    let file = file.trim().replace('\\', "/");
    if file.is_empty()
        || !is_safe_relative_path(&file)
        || !file.to_ascii_lowercase().ends_with(".bib")
    {
        return Err("Bitte zuerst eine Literaturdatei (.bib) im Projektordner festlegen.".into());
    }
    Ok(Path::new(project_root).join(file))
}

#[tauri::command]
async fn read_bib(project_root: String, file: String) -> Result<Vec<BibEntry>, String> {
    blocking(move || {
        let path = bib_path(&project_root, &file)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        Ok(bibtex::parse(&files::read_text(&path)?))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BibMerge {
    added: Vec<String>,
    skipped: Vec<String>,
    keys: Vec<String>,
}

fn merge_into_bib(path: &Path, additions: &str) -> Result<BibMerge, String> {
    let existing = if path.exists() {
        files::read_text(path)?
    } else {
        String::new()
    };
    let keys: Vec<String> = bibtex::parse(additions)
        .into_iter()
        .map(|entry| entry.key)
        .collect();
    if keys.is_empty() {
        return Err("Es wurden keine gültigen BibTeX-Einträge gefunden.".into());
    }
    let (merged, added, skipped) = bibtex::merge(&existing, additions);
    if !added.is_empty() {
        files::write_text_atomic(path, &merged)?;
    }
    Ok(BibMerge {
        added,
        skipped,
        keys,
    })
}

#[tauri::command]
async fn add_bib_entries(
    project_root: String,
    file: String,
    bibtex: String,
) -> Result<BibMerge, String> {
    blocking(move || merge_into_bib(&bib_path(&project_root, &file)?, &bibtex)).await
}

#[tauri::command]
async fn zotero_status() -> Result<zotero::ZoteroStatus, String> {
    blocking(|| Ok(zotero::status())).await
}

#[tauri::command]
async fn zotero_search(query: String) -> Result<Vec<zotero::ZoteroItem>, String> {
    blocking(move || zotero::search(&query)).await
}

#[tauri::command]
async fn zotero_import(
    ids: Vec<String>,
    source: String,
    project_root: String,
    file: String,
) -> Result<BibMerge, String> {
    blocking(move || {
        let path = bib_path(&project_root, &file)?;
        let text = zotero::bibtex(&ids, &source)?;
        merge_into_bib(&path, &text)
    })
    .await
}

// ---------------------------------------------------------------- Add-ons und Vorlagen

#[tauri::command]
async fn addons_list(app: AppHandle) -> Result<Vec<InstalledAddon>, String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || Ok(dirs.list())).await
}

#[tauri::command]
async fn addons_install(
    app: AppHandle,
    path: String,
) -> Result<core::addon::AddonManifest, String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || {
        let path = PathBuf::from(path);
        if path.is_dir() {
            dirs.install_folder(&path)
        } else if path
            .file_name()
            .is_some_and(|name| name == addons::MANIFEST)
        {
            dirs.install_folder(&parent_dir(&path)?)
        } else {
            dirs.install_zip(&path)
        }
    })
    .await
}

#[tauri::command]
async fn addons_remove(app: AppHandle, id: String) -> Result<(), String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || dirs.remove(&id)).await
}

#[tauri::command]
async fn addons_set_enabled(app: AppHandle, id: String, enabled: bool) -> Result<(), String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || dirs.set_enabled(&id, enabled)).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewAddonFile {
    source: String,
    target: String,
}

#[tauri::command]
async fn addons_save(
    app: AppHandle,
    manifest: core::addon::AddonManifest,
    new_files: Vec<NewAddonFile>,
) -> Result<core::addon::AddonManifest, String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || {
        let files: Vec<(PathBuf, String)> = new_files
            .into_iter()
            .map(|file| (PathBuf::from(file.source), file.target))
            .collect();
        dirs.save_custom(manifest, &files)
    })
    .await
}

#[tauri::command]
async fn addons_export(app: AppHandle, id: String, path: String) -> Result<(), String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || dirs.export_zip(&id, Path::new(&path))).await
}

#[tauri::command]
async fn templates_list(app: AppHandle) -> Result<Vec<templates::TemplateInfo>, String> {
    let dirs = addon_dirs(&app)?;
    blocking(move || {
        let mut list = templates::list();
        for addon in dirs.enabled() {
            for template in &addon.manifest.templates {
                list.push(templates::TemplateInfo {
                    id: template.id.clone(),
                    name: template.name.clone(),
                    description: format!("Vorlage aus dem Add-on „{}“", addon.manifest.name),
                    source: format!("addon:{}", addon.manifest.id),
                });
            }
        }
        Ok(list)
    })
    .await
}

// ---------------------------------------------------------------- Start

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Start ohne eingebettete Dateien (Entwicklung): Bundle und Add-ons aus `resources/`.
pub fn run() {
    run_with(embedded::EmbeddedAssets::default())
}

/// Start mit den in die EXE eingebetteten Dateien (siehe `main.rs`).
pub fn run_with(assets: embedded::EmbeddedAssets) {
    embedded::install(assets);
    // Reste eines Updates bzw. einer Installation (`*.old`) aufräumen
    setup::cleanup_leftovers();
    // Fontconfig für die eingebaute TeX-Engine einrichten (vor allen weiteren Threads).
    fonts::configure_fontconfig();
    // Schriftliste im Hintergrund aufbauen (erste Kompilierung wird dadurch schneller).
    std::thread::spawn(|| {
        let _ = fonts::installed_families();
    });
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // nach einem Update: Versionsnummer unter „Apps“ nachziehen (im Hintergrund)
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Ok(dir) = data_dir(&handle) {
                    setup::refresh_after_update(&dir);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            export_document,
            import_latex,
            code_to_document,
            preamble_macros,
            sketch_to_latex,
            slides_to_latex,
            slides_load,
            slides_save,
            slides_read_image,
            slides_compile,
            sketch_from_code,
            sketch_catalog,
            setup_status,
            setup_install,
            setup_use_portable,
            setup_set_autostart,
            setup_uninstall,
            update_check,
            update_install,
            share_file,
            analyze_document,
            check_resources,
            compile_document,
            render_pdf_page,
            save_pdf,
            render_tikz,
            render_latex_previews,
            resolve_image,
            new_project,
            load_project,
            save_project,
            export_tex,
            file_stamp,
            read_text_file,
            allow_project_dir,
            import_image,
            import_image_data,
            read_bib,
            add_bib_entries,
            zotero_status,
            zotero_search,
            zotero_import,
            addons_list,
            addons_install,
            addons_remove,
            addons_set_enabled,
            addons_save,
            addons_export,
            templates_list,
            set_tex_options,
            system_check,
        ])
        .run(tauri::generate_context!())
        .expect("VisuTeX konnte nicht gestartet werden");
}
