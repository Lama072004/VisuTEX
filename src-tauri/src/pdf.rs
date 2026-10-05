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
