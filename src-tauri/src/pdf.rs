//! PDF-Vorschau in reinem Rust (`hayro`): Seiten werden als PNG gerendert, das
//! Frontend zeigt nur Bilder an (kein PDF-Viewer im WebView nötig, kein
//! Größenlimit von data:-URLs, funktioniert auch unter WebKitGTK).

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, RenderCache, RenderSettings};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Höchstens so viele PDFs bleiben im Speicher (älteste werden verworfen).
const MAX_DOCUMENTS: usize = 6;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSize {
    /// Breite/Höhe in PostScript-Punkten (1/72 Zoll).
    pub width: f32,
    pub height: f32,
}

pub struct StoredPdf {
    pub bytes: Arc<Vec<u8>>,
    pdf: Pdf,
    pub pages: Vec<PageSize>,
}

#[derive(Default)]
pub struct PdfStore {
    documents: Mutex<VecDeque<(u64, Arc<StoredPdf>)>>,
    next_id: Mutex<u64>,
}

pub fn load(bytes: Vec<u8>) -> Result<StoredPdf, String> {
    let bytes = Arc::new(bytes);
    let pdf = Pdf::new(bytes.clone())
        .map_err(|error| format!("Das erzeugte PDF konnte nicht gelesen werden: {error:?}"))?;
    let pages = pdf
        .pages()
        .iter()
        .map(|page| {
            let (width, height) = page.render_dimensions();
            PageSize { width, height }
        })
        .collect();
    Ok(StoredPdf { bytes, pdf, pages })
}

impl PdfStore {
    pub fn insert(&self, stored: StoredPdf) -> (u64, Vec<PageSize>) {
        let pages = stored.pages.clone();
        let id = {
            let mut next = self.next_id.lock().unwrap_or_else(|p| p.into_inner());
            *next += 1;
            *next
        };
        let mut documents = self.documents.lock().unwrap_or_else(|p| p.into_inner());
        documents.push_back((id, Arc::new(stored)));
        while documents.len() > MAX_DOCUMENTS {
            documents.pop_front();
        }
        (id, pages)
    }

    pub fn get(&self, id: u64) -> Result<Arc<StoredPdf>, String> {
        self.documents
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, stored)| stored.clone())
            .ok_or_else(|| "Das PDF ist nicht mehr verfügbar – bitte erneut kompilieren.".into())
    }
}

/// Rendert eine Seite als PNG. `scale` = Pixel je PostScript-Punkt (1.0 = 72 dpi).
pub fn render_page_png(stored: &StoredPdf, page: usize, scale: f32) -> Result<Vec<u8>, String> {
    let pages = stored.pdf.pages();
    let page = pages
        .get(page)
        .ok_or_else(|| format!("Seite {} existiert nicht.", page + 1))?;
    let scale = scale.clamp(0.1, 8.0);
    let (width, height) = page.render_dimensions();
    if (width * scale) > 16_000.0 || (height * scale) > 16_000.0 {
        return Err("Die angeforderte Vorschau ist zu groß.".into());
    }
    let cache = RenderCache::new();
    let settings = RenderSettings {
        x_scale: scale,
        y_scale: scale,
        bg_color: WHITE,
        ..Default::default()
    };
    let pixmap = render(page, &cache, &InterpreterSettings::default(), &settings);
    pixmap
        .into_png()
        .map_err(|error| format!("PNG konnte nicht erzeugt werden: {error:?}"))
}

// ------------------------------------------------------------------ Vektorgrafik (SVG)

/// Ausschnitt einer Seite in PostScript-Punkten (Ursprung oben links wie in der SVG).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgView {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Rendert eine Seite als SVG (Vektorgrafik, scharf in jeder Vergrößerung).
pub fn render_page_svg(stored: &StoredPdf, page: usize) -> Result<String, String> {
    let pages = stored.pdf.pages();
    let page = pages
        .get(page)
        .ok_or_else(|| format!("Seite {} existiert nicht.", page + 1))?;
    let cache = hayro_svg::RenderCache::new();
    Ok(hayro_svg::convert(
        page,
        &cache,
        &InterpreterSettings::default(),
        &hayro_svg::SvgRenderSettings::default(),
    ))
}

/// Ganze Seite als Ausschnitt.
pub fn page_view(stored: &StoredPdf, page: usize) -> Option<SvgView> {
    stored.pages.get(page).map(|size| SvgView {
        x: 0.0,
        y: 0.0,
        width: size.width,
        height: size.height,
    })
}

/// Bereich mit sichtbarem Inhalt (in Punkten, plus `margin`) – bestimmt an einer groben
/// Rastervorschau (1 Pixel je Punkt). `None`: leere Seite.
pub fn content_view(
    stored: &StoredPdf,
    page: usize,
    margin: f32,
) -> Result<Option<SvgView>, String> {
    let full =
        page_view(stored, page).ok_or_else(|| format!("Seite {} existiert nicht.", page + 1))?;
    let png = render_page_png(stored, page, 1.0)?;
    let image = image::load_from_memory(&png)
        .map_err(|error| format!("Vorschau konnte nicht gelesen werden: {error}"))?
        .to_rgba8();
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        if a > 0 && (r < 245 || g < 245 || b < 245) {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    if right <= left || bottom <= top {
        return Ok(None);
    }
    // Pixel → Punkte (Skalierung 1); Rasterbild und Seite können um Rundung abweichen
    let sx = full.width / image.width().max(1) as f32;
    let sy = full.height / image.height().max(1) as f32;
    let x = (left as f32 * sx - margin).max(0.0);
    let y = (top as f32 * sy - margin).max(0.0);
    let right = (right as f32 * sx + margin).min(full.width);
    let bottom = (bottom as f32 * sy + margin).min(full.height);
    Ok(Some(SvgView {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }))
}

/// Inhalt einer von hayro-svg erzeugten SVG (zwischen dem öffnenden und schließenden Wurzelelement).
fn svg_body(svg: &str) -> &str {
    let start = svg
        .find("<svg")
        .and_then(|index| svg[index..].find('>').map(|end| index + end + 1))
        .unwrap_or(0);
    let end = svg
        .rfind("</svg>")
        .filter(|end| *end >= start)
        .unwrap_or(svg.len());
    &svg[start..end]
}

/// IDs und Verweise eindeutig machen, wenn mehrere Seiten in einer SVG landen.
fn prefix_ids(body: &str, prefix: &str) -> String {
    body.replace("id=\"", &format!("id=\"{prefix}"))
        .replace("url(#", &format!("url(#{prefix}"))
        .replace("href=\"#", &format!("href=\"#{prefix}"))
}

/// Ausschnitte (Seiten-SVG, Bereich) untereinander zu einer SVG zusammensetzen. Die
/// Ausgabegröße ist `scale` Pixel je Punkt – wie bei der gleichwertigen PNG-Vorschau, damit
/// die Anzeige im Editor genauso groß bleibt.
pub fn stack_svg(parts: &[(String, SvgView)], scale: f32) -> String {
    let width = parts
        .iter()
        .map(|(_, view)| view.width)
        .fold(1.0_f32, f32::max);
    let height: f32 = parts
        .iter()
        .map(|(_, view)| view.height)
        .sum::<f32>()
        .max(1.0);
    let mut output = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{:.1}\" height=\"{:.1}\" viewBox=\"0 0 {width} {height}\">",
        width * scale,
        height * scale
    );
    let mut offset = 0.0_f32;
    for (index, (svg, view)) in parts.iter().enumerate() {
        output.push_str(&format!(
            "<svg x=\"0\" y=\"{offset}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" overflow=\"hidden\">",
            view.width, view.height, view.x, view.y, view.width, view.height
        ));
        output.push_str(&prefix_ids(svg_body(svg), &format!("v{index}-")));
        output.push_str("</svg>");
        offset += view.height;
    }
    output.push_str("</svg>");
    output
}

/// SVG als data:-URL.
pub fn svg_data_url(svg: &str) -> String {
    use base64::Engine;
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "<svg viewBox=\"0 0 100 50\" width=\"100\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><defs><path id=\"g0\" d=\"M0 0\"/></defs><use xlink:href=\"#g0\"/><g clip-path=\"url(#c0)\"/></svg>";

    #[test]
    fn stacks_svg_parts_with_unique_ids_and_scaled_size() {
        let view = SvgView {
            x: 10.0,
            y: 5.0,
            width: 40.0,
            height: 20.0,
        };
        let svg = stack_svg(&[(PAGE.into(), view), (PAGE.into(), view)], 2.0);
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("width=\"80.0\" height=\"80.0\" viewBox=\"0 0 40 40\""));
        assert!(svg.contains("viewBox=\"10 5 40 20\""));
        assert!(svg.contains("id=\"v0-g0\"") && svg.contains("id=\"v1-g0\""));
        assert!(svg.contains("href=\"#v1-g0\"") && svg.contains("url(#v1-c0)"));
        assert_eq!(svg.matches("<svg").count(), 3);
        assert_eq!(svg.matches("</svg>").count(), 3);
        assert!(svg_data_url(&svg).starts_with("data:image/svg+xml;base64,"));
    }
}
