//! Vorschau-Bilder für den visuellen Modus.
//!
//! - Roh-LaTeX-Blöcke (Tabellen mit eigenen Befehlen, Titelseiten, Abbildungen
//!   mit mehreren Bildern, eigene Umgebungen …) werden mit der Präambel des
//!   Dokuments in **einem** Lauf kompiliert; jeder Block beginnt auf einer neuen
//!   Seite, die Seitenzuordnung steht in der .aux-Datei. Die Seiten werden auf
//!   den Inhalt zugeschnitten und als PNG (data:-URL) geliefert.
//! - Projektbilder werden wie in LaTeX aufgelöst (Unterordner, `\graphicspath`,
//!   Dateinamen ohne Endung); PDF-Grafiken werden als PNG gerendert.

use crate::compile::{compile, CompileRequest};
use crate::texbundle::BundleConfig;
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Höchstens so viele Blöcke je Anfrage.
const MAX_BLOCKS: usize = 80;
/// Höchstens so viele Seiten je Block.
const MAX_PAGES: usize = 3;
/// Pixel je PostScript-Punkt (≈ 144 dpi).
const SCALE: f32 = 2.0;

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockPreview {
    /// PNG als data:-URL
    pub image: Option<String>,
    pub error: Option<String>,
    /// Ganzseitiger Block (`titlepage`): Bild ist die ganze Seite samt Rändern, nicht beschnitten.
    pub full_page: bool,
}

/// Füllt der Block eine eigene Seite (Titelseite)? Kommentarzeilen davor zählen nicht.
pub fn is_full_page_block(latex: &str) -> bool {
    latex
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('%'))
        .is_some_and(|line| line.starts_with("\\begin{titlepage}"))
}

/// Präambel bis vor `\begin{document}`.
fn preamble_only(preamble: &str) -> &str {
    match preamble.find("\\begin{document}") {
        Some(index) => &preamble[..index],
        None => preamble,
    }
}

fn batch_document(preamble: &str, blocks: &[&str]) -> String {
    let mut document = String::with_capacity(preamble.len() + 4096);
    document.push_str(preamble_only(preamble).trim_end());
    // Absolute Seitennummer (unabhängig von \pagenumbering) je Block in die .aux schreiben.
    document.push_str(
        "\n\\makeatletter\n\\newcount\\visutex@abspage\n\\AddToHook{shipout/before}{\\global\\advance\\visutex@abspage\\@ne}\n\\newcommand\\visutexblock[1]{\\clearpage\\write\\@auxout{\\string\\visutexblockpage{#1}{\\the\\visutex@abspage}}}\n\\makeatother\n\\begin{document}\n\\pagestyle{empty}\n",
    );
    for (index, block) in blocks.iter().enumerate() {
        document.push_str(&format!("\\visutexblock{{{index}}}\n{block}\n"));
    }
    document.push_str("\\clearpage\n\\end{document}\n");
    document
}

/// `\visutexblockpage{N}{P}` aus der .aux-Datei → Block N beginnt auf Seite P (1-basiert).
fn block_pages(aux: &str) -> HashMap<usize, usize> {
    let mut pages = HashMap::new();
    for line in aux.lines() {
        let Some(rest) = line.trim().strip_prefix("\\visutexblockpage{") else {
            continue;
        };
        let mut parts = rest.split(['{', '}']).filter(|part| !part.is_empty());
        if let (Some(block), Some(page)) = (parts.next(), parts.next()) {
            if let (Ok(block), Ok(page)) = (block.parse(), page.parse()) {
                pages.insert(block, page);
            }
        }
    }
    pages
}

/// Weißränder entfernen (mit etwas Rand), mehrere Seiten untereinander.
fn crop_and_stack(pages: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut images = Vec::new();
    for png in pages {
        let image = image::load_from_memory(&png)
            .map_err(|error| format!("Vorschau konnte nicht gelesen werden: {error}"))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (x, y, pixel) in image.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if a > 0 && (r < 245 || g < 245 || b < 245) {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
        if right < left || bottom < top {
            continue; // leere Seite
        }
        let margin = 8;
        let left = left.saturating_sub(margin);
        let top = top.saturating_sub(margin);
        let right = (right + margin).min(width - 1);
        let bottom = (bottom + margin).min(height - 1);
        images.push(
            image::imageops::crop_imm(&image, left, top, right - left + 1, bottom - top + 1)
                .to_image(),
        );
    }
    if images.is_empty() {
        return Err("Der Block erzeugt keine sichtbare Ausgabe.".into());
    }
    let width = images.iter().map(|image| image.width()).max().unwrap_or(1);
    let height: u32 = images.iter().map(|image| image.height()).sum();
    let mut canvas = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 255, 255, 255]));
    let mut offset = 0;
    for image in &images {
        image::imageops::overlay(&mut canvas, image, 0, i64::from(offset));
        offset += image.height();
    }
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|error| format!("Vorschau konnte nicht gespeichert werden: {error}"))?;
    Ok(output.into_inner())
}

fn data_url(png: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

/// Kompiliert Blöcke und liefert pro Block ein Bild oder einen Fehler.
fn compile_blocks(
    preamble: &str,
    blocks: &[&str],
    root: Option<&Path>,
    search_paths: &[PathBuf],
    bundle: &BundleConfig,
) -> Result<Vec<BlockPreview>, String> {
    let latex = batch_document(preamble, blocks);
    let latex = format!("{}{latex}", crate::fonts::hint_prefix(&latex));
    let output = compile(
        CompileRequest {
            latex,
            project_root: root.map(Path::to_path_buf),
            extra_search_paths: search_paths.to_vec(),
        },
        bundle,
        &mut |_| {},
    )
    .map_err(|failure| failure.message)?;
    let stored = crate::pdf::load(output.pdf)?;
    let starts = block_pages(&output.aux);
    let total = stored.pages.len();
    let mut previews = Vec::with_capacity(blocks.len());
    for index in 0..blocks.len() {
        let Some(&start) = starts.get(&index) else {
            previews.push(BlockPreview {
                image: None,
                error: Some("Keine Ausgabe für diesen Block.".into()),
                full_page: false,
            });
            continue;
        };
        // Ende: Seite vor dem nächsten Block (bzw. letzte Seite)
        let next = (index + 1..blocks.len())
            .find_map(|other| starts.get(&other).copied())
            .unwrap_or(total + 1);
        let end = next.saturating_sub(1).max(start).min(total);
        if is_full_page_block(blocks[index]) {
            // Titelseite: ganze Seite unbeschnitten, damit sie im Editor genau auf dem Blatt liegt
            let png = crate::pdf::render_page_png(&stored, start - 1, SCALE)?;
            previews.push(BlockPreview {
                image: Some(data_url(&png)),
                error: None,
                full_page: true,
            });
            continue;
        }
        let mut pages = Vec::new();
        for page in start..=end.min(start + MAX_PAGES - 1) {
            pages.push(crate::pdf::render_page_png(&stored, page - 1, SCALE)?);
        }
        previews.push(match crop_and_stack(pages) {
            Ok(png) => BlockPreview {
                image: Some(data_url(&png)),
                error: None,
                full_page: false,
            },
            Err(error) => BlockPreview {
                image: None,
                error: Some(error),
                full_page: false,
            },
        });
    }
    Ok(previews)
}

/// Vorschauen für Roh-LaTeX-Blöcke. Scheitert der gemeinsame Lauf, wird jeder
/// Block einzeln kompiliert (Fehler werden dem richtigen Block zugeordnet).
pub fn render_blocks(
    preamble: &str,
    blocks: &[String],
    root: Option<&Path>,
    search_paths: &[PathBuf],
    bundle: &BundleConfig,
) -> Vec<BlockPreview> {
    let blocks: Vec<&str> = blocks.iter().take(MAX_BLOCKS).map(String::as_str).collect();
    if blocks.is_empty() {
        return Vec::new();
    }
    match compile_blocks(preamble, &blocks, root, search_paths, bundle) {
        Ok(previews) => previews,
        Err(_) if blocks.len() > 1 => blocks
            .iter()
            .map(
                |block| match compile_blocks(preamble, &[block], root, search_paths, bundle) {
                    Ok(mut single) => single.pop().unwrap_or_default(),
                    Err(error) => BlockPreview {
                        image: None,
                        error: Some(error),
                        full_page: false,
                    },
                },
            )
            .collect(),
        Err(error) => vec![BlockPreview {
            image: None,
            error: Some(error),
            full_page: false,
        }],
    }
}

// ------------------------------------------------------------------ Bilder

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedImage {
    /// Gefundene Datei (absolut), für das Asset-Protokoll.
    pub file: Option<String>,
    /// Pfad relativ zum Projektordner (falls darin)
    pub relative: Option<String>,
    /// Gerenderte Vorschau (PDF-Grafiken bzw. Dateien außerhalb des Projektordners)
    pub data_url: Option<String>,
    /// image | pdf | eps | missing
    pub kind: String,
}

type PdfCache = Mutex<HashMap<(PathBuf, u64), String>>;

fn pdf_cache() -> &'static PdfCache {
    static CACHE: std::sync::OnceLock<PdfCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn modified_stamp(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// Erste Seite einer PDF-Grafik als PNG (zwischengespeichert).
fn pdf_preview(path: &Path) -> Result<String, String> {
    let key = (path.to_path_buf(), modified_stamp(path));
    if let Some(cached) = pdf_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return Ok(cached.clone());
    }
    let bytes = std::fs::read(path)
        .map_err(|error| format!("PDF-Grafik konnte nicht gelesen werden: {error}"))?;
    let stored = crate::pdf::load(bytes)?;
    let page = stored
        .pages
        .first()
        .ok_or("Die PDF-Grafik hat keine Seite.")?;
    // höchstens ca. 1600 Pixel breit
    let scale = (1600.0 / page.width.max(1.0)).clamp(0.5, 3.0);
    let png = crate::pdf::render_page_png(&stored, 0, scale)?;
    let url = data_url(&png);
    let mut cache = pdf_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.len() > 64 {
        cache.clear();
    }
    cache.insert(key, url.clone());
    Ok(url)
}

/// Bild wie LaTeX auflösen (für die Anzeige im Editor).
pub fn resolve_image(root: &Path, path: &str, preamble: &str) -> ResolvedImage {
    let search = crate::files::graphics_paths(preamble);
    let Some(found) = crate::files::resolve_graphic(root, path, &search) else {
        return ResolvedImage {
            kind: "missing".into(),
            ..Default::default()
        };
    };
    let extension = found
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let relative = found
        .strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"));
    let file = Some(found.display().to_string());
    match extension.as_str() {
        "pdf" => ResolvedImage {
            data_url: pdf_preview(&found).ok(),
            file,
            relative,
            kind: "pdf".into(),
        },
        "eps" | "ps" => ResolvedImage {
            file,
            relative,
            kind: "eps".into(),
            ..Default::default()
        },
        _ => {
            // Außerhalb des Projektordners (z. B. ../abbildungen) erlaubt das
            // Asset-Protokoll keinen Zugriff → als data:-URL liefern.
            let data_url = if relative.is_none() {
                std::fs::read(&found).ok().map(|bytes| {
                    let mime = if extension == "png" {
                        "image/png"
                    } else {
                        "image/jpeg"
                    };
                    format!(
                        "data:{mime};base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )
                })
            } else {
                None
            };
            ResolvedImage {
                file,
                relative,
                data_url,
                kind: "image".into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_full_page_blocks() {
        assert!(is_full_page_block(
            "% Titelseite\n\\begin{titlepage}\nx\n\\end{titlepage}"
        ));
        assert!(!is_full_page_block("\\begin{tabular}{l}x\\end{tabular}"));
    }

    #[test]
    fn reads_block_pages_from_aux() {
        let aux = "\\relax\n\\visutexblockpage{0}{1}\n\\visutexblockpage{1}{2}\n\\visutexblockpage{2}{4}\n";
        let pages = block_pages(aux);
        assert_eq!(pages.get(&0), Some(&1));
        assert_eq!(pages.get(&2), Some(&4));
        let document = batch_document(
            "\\documentclass{article}\n\\begin{document}\nalt",
            &["A", "B"],
        );
        assert!(document.starts_with("\\documentclass{article}\n\\makeatletter"));
        assert!(!document.contains("alt"));
        assert!(document.contains("\\visutexblock{1}\nB\n"));
    }

    #[test]
    fn resolves_images_like_latex() {
        let root = std::env::temp_dir().join(format!("visutex-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Pictures Nelprof")).unwrap();
        std::fs::create_dir_all(root.join("abb")).unwrap();
        std::fs::write(root.join("Pictures Nelprof/RE_Gain.png"), b"png").unwrap();
        std::fs::write(root.join("abb/logo.jpg"), b"jpg").unwrap();
        let direct = resolve_image(&root, "Pictures Nelprof/RE_Gain.png", "");
        assert_eq!(direct.kind, "image");
        assert_eq!(
            direct.relative.as_deref(),
            Some("Pictures Nelprof/RE_Gain.png")
        );
        let via_path = resolve_image(&root, "logo", "\\graphicspath{{abb/}}");
        assert_eq!(via_path.relative.as_deref(), Some("abb/logo.jpg"));
        assert_eq!(resolve_image(&root, "fehlt", "").kind, "missing");
        let _ = std::fs::remove_dir_all(&root);
    }
}
