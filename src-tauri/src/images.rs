//! Bilder in den Projektordner übernehmen (`Abbildungen/`).
//!
//! LaTeX (alle Engines) unterstützt PNG, JPEG und PDF. Andere Formate werden
//! beim Einfügen konvertiert: GIF/WebP/BMP/TIFF → PNG, SVG → PDF (Vektor bleibt
//! Vektor). EPS wird abgelehnt (bräuchte Ghostscript/Shell-Escape).

use crate::core::export::{content_hash, parse_data_url};
use base64::Engine;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const IMAGE_FOLDER: &str = "Abbildungen";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedImage {
    /// Relativer Pfad im Projektordner, z. B. `Abbildungen/messung.png`.
    pub latex_path: String,
    pub converted_from: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Dateiname ohne Sonderzeichen (LaTeX-/Engine-sicher).
pub fn sanitize_stem(stem: &str) -> String {
    let mut result = String::new();
    for character in stem.chars() {
        match character {
            'ä' => result.push_str("ae"),
            'ö' => result.push_str("oe"),
            'ü' => result.push_str("ue"),
            'Ä' => result.push_str("Ae"),
            'Ö' => result.push_str("Oe"),
            'Ü' => result.push_str("Ue"),
            'ß' => result.push_str("ss"),
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => result.push(c),
            _ => {
                if !result.ends_with('-') {
                    result.push('-');
                }
            }
        }
    }
    let trimmed = result
        .trim_matches('-')
        .chars()
        .take(60)
        .collect::<String>();
    if trimmed.is_empty() {
        "bild".into()
    } else {
        trimmed
    }
}

/// Schreibt `bytes` als `Abbildungen/<stem>.<ext>`; gleicher Inhalt wird
/// wiederverwendet, abweichender Inhalt bekommt einen Zähler-Suffix.
fn store(project_root: &Path, stem: &str, extension: &str, bytes: &[u8]) -> Result<String, String> {
    let folder = project_root.join(IMAGE_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|error| {
        format!("Ordner „{IMAGE_FOLDER}“ konnte nicht angelegt werden: {error}")
    })?;
    let stem = sanitize_stem(stem);
    for counter in 0..10_000 {
        let name = if counter == 0 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{counter}.{extension}")
        };
        let target = folder.join(&name);
        match std::fs::read(&target) {
            Ok(existing) if existing == bytes => return Ok(format!("{IMAGE_FOLDER}/{name}")),
            Ok(_) => continue,
            Err(_) => {
                std::fs::write(&target, bytes)
                    .map_err(|error| format!("Bild konnte nicht gespeichert werden: {error}"))?;
                return Ok(format!("{IMAGE_FOLDER}/{name}"));
            }
        }
    }
    Err("Kein freier Dateiname im Bildordner gefunden.".into())
}

fn dimensions(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or((None, None))
}

fn to_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("Bild konnte nicht gelesen werden: {error}"))?;
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|error| format!("Bild konnte nicht nach PNG konvertiert werden: {error}"))?;
    Ok(output.into_inner())
}

pub fn svg_to_pdf(svg: &[u8]) -> Result<Vec<u8>, String> {
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = svg2pdf::usvg::Tree::from_data(svg, &options)
        .map_err(|error| format!("SVG konnte nicht gelesen werden: {error}"))?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|error| format!("SVG konnte nicht nach PDF konvertiert werden: {error}"))
}

/// Konvertiert Bilddaten in ein LaTeX-taugliches Format und legt sie ab.
pub fn store_image_bytes(
    project_root: &Path,
    stem: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<ImportedImage, String> {
    let extension = extension.to_ascii_lowercase();
    let (target_extension, data, converted_from) = match extension.as_str() {
        "png" => ("png", bytes.to_vec(), None),
        "jpg" | "jpeg" => ("jpg", bytes.to_vec(), None),
        "pdf" => ("pdf", bytes.to_vec(), None),
        "gif" | "webp" | "bmp" | "tif" | "tiff" => ("png", to_png(bytes)?, Some(extension.clone())),
        "svg" => ("pdf", svg_to_pdf(bytes)?, Some(extension.clone())),
        "eps" | "ps" => {
            return Err(
                "EPS-Grafiken werden nicht unterstützt – bitte als PDF oder SVG exportieren."
                    .into(),
            )
        }
        other => return Err(format!("Das Bildformat „{other}“ wird nicht unterstützt.")),
    };
    let (width, height) = if target_extension == "pdf" {
        (None, None)
    } else {
        dimensions(&data)
    };
    let latex_path = store(project_root, stem, target_extension, &data)?;
    Ok(ImportedImage {
        latex_path,
        converted_from,
        width,
        height,
    })
}

pub fn import_image_file(source: &Path, project_root: &Path) -> Result<ImportedImage, String> {
    let bytes = std::fs::read(source).map_err(|error| {
        format!(
            "Bild „{}“ konnte nicht gelesen werden: {error}",
            source.display()
        )
    })?;
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bild".into());
    let extension = source
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_default();
    store_image_bytes(project_root, &stem, &extension, &bytes)
}

/// Eingefügtes/abgelegtes Bild (data:-URL aus der Zwischenablage).
pub fn import_image_data_url(data_url: &str, project_root: &Path) -> Result<ImportedImage, String> {
    let (mime, base64) =
        parse_data_url(data_url).ok_or("Die eingefügten Bilddaten sind ungültig.")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64.as_bytes())
        .map_err(|_| "Die eingefügten Bilddaten sind ungültig.".to_string())?;
    let extension = match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        other => return Err(format!("Das Bildformat „{other}“ wird nicht unterstützt.")),
    };
    store_image_bytes(
        project_root,
        &format!("eingefuegt-{}", content_hash(&bytes)),
        extension,
        &bytes,
    )
}

/// Absoluter Pfad einer Projektdatei (nur sichere relative Pfade).
pub fn resolve_in_project(project_root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = relative.replace('\\', "/");
    crate::core::settings::is_safe_relative_path(&relative).then(|| project_root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_and_converts() {
        assert_eq!(
            sanitize_stem("Messung März 2026 (final)"),
            "Messung-Maerz-2026-final"
        );
        assert_eq!(sanitize_stem("###"), "bild");
        let root = std::env::temp_dir().join(format!("visutex-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // 1×1-GIF
        let gif = base64::engine::general_purpose::STANDARD
            .decode("R0lGODlhAQABAIAAAP///wAAACH5BAEAAAAALAAAAAABAAEAAAICRAEAOw==")
            .unwrap();
        let first = store_image_bytes(&root, "Test Bild", "gif", &gif).unwrap();
        assert_eq!(first.latex_path, "Abbildungen/Test-Bild.png");
        assert_eq!(first.converted_from.as_deref(), Some("gif"));
        assert_eq!(first.width, Some(1));
        // Gleicher Inhalt → gleiche Datei
        let again = store_image_bytes(&root, "Test Bild", "gif", &gif).unwrap();
        assert_eq!(again.latex_path, first.latex_path);
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#;
        let vector = store_image_bytes(&root, "vektor", "svg", svg).unwrap();
        assert_eq!(vector.latex_path, "Abbildungen/vektor.pdf");
        let pdf = std::fs::read(root.join("Abbildungen/vektor.pdf")).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        assert!(store_image_bytes(&root, "x", "eps", b"%!PS").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Bild für den Folien-Editor: als data:-URL (PNG/JPEG/SVG bleiben erhalten,
/// GIF/WebP/BMP/TIFF werden zu PNG, PDF wird mit der ersten Seite gerastert).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideImage {
    pub src: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn slide_image(source: &Path) -> Result<SlideImage, String> {
    let bytes = std::fs::read(source).map_err(|error| {
        format!(
            "Bild „{}“ konnte nicht gelesen werden: {error}",
            source.display()
        )
    })?;
    let extension = source
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg")
        && image::load_from_memory(&bytes).is_err()
    {
        return Err(format!("Das Bild „{}“ ist beschädigt.", source.display()));
    }
    let (mime, data) = match extension.as_str() {
        "png" => ("image/png", bytes),
        "jpg" | "jpeg" => ("image/jpeg", bytes),
        "svg" => {
            // Prüfen, ob die SVG auch für LaTeX (PDF) umwandelbar ist.
            svg_to_pdf(&bytes)?;
            let (width, height) = svg_size(&bytes);
            return Ok(SlideImage {
                src: format!(
                    "data:image/svg+xml;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                ),
                width,
                height,
            });
        }
        "gif" | "webp" | "bmp" | "tif" | "tiff" => ("image/png", to_png(&bytes)?),
        "pdf" => {
            let stored = crate::pdf::load(bytes)?;
            ("image/png", crate::pdf::render_page_png(&stored, 0, 3.0)?)
        }
        "eps" | "ps" => {
            return Err(
                "EPS-Grafiken werden nicht unterstützt – bitte als PDF oder SVG exportieren."
                    .into(),
            )
        }
        other => return Err(format!("Das Bildformat „{other}“ wird nicht unterstützt.")),
    };
    let (width, height) = dimensions(&data);
    Ok(SlideImage {
        src: format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&data)
        ),
        width,
        height,
    })
}

fn svg_size(svg: &[u8]) -> (Option<u32>, Option<u32>) {
    let options = svg2pdf::usvg::Options::default();
    match svg2pdf::usvg::Tree::from_data(svg, &options) {
        Ok(tree) => {
            let size = tree.size();
            (
                Some(size.width().round() as u32),
                Some(size.height().round() as u32),
            )
        }
        Err(_) => (None, None),
    }
}
