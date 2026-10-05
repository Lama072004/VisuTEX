//! Erzeugt `resources/sketch-symbols.json`: Vorschaubilder und Anschlusspunkte
//! aller Einträge aus `core::sketch_catalog` – kompiliert mit der eingebauten
//! Engine und dem mitgelieferten TeX-Bundle (also exakt der ausgelieferten
//! CircuiTikZ-Version).
//!
//! Aufruf (Projektordner `src-tauri`):
//! ```text
//! cargo run --features dev-tools --bin build-sketch-symbols
//! ```
//!
//! Ablauf: Jeder Eintrag steht auf einer eigenen Seite (8 × 8 cm, Ursprung in
//! der Mitte). Einträge, die Fehler erzeugen (unbekannter Name), werden
//! entfernt und es wird erneut kompiliert. Anschlusspunkte schreibt TeX per
//! `\typeout` ins Protokoll. Jede Seite wird gerastert, auf den Inhalt
//! zugeschnitten und als PNG mit Transparenz (schwarz, Deckkraft = Dunkelheit)
//! gespeichert.

use base64::Engine;
use std::collections::BTreeMap;
use std::path::PathBuf;
use visutex_lib::compile::{compile, CompileRequest};
use visutex_lib::core::sketch_catalog::{symbols_document, CatalogEntry, ENTRIES, PAGE_CM};
use visutex_lib::texbundle::BundleConfig;

const RENDER_SCALE: f32 = 3.0;
/// 1 cm in PDF-Punkten (bp)
const BP_PER_CM: f64 = 72.0 / 2.54;
/// 1 cm in TeX-Punkten (pt)
const PT_PER_CM: f64 = 72.27 / 2.54;
/// Ohne eigene Anschlüsse: Verbindung über die Mitte
const GENERIC_ANCHORS: &[&str] = &["left", "right", "center", "mid"];

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bundle() -> BundleConfig {
    let embedded = manifest().join("resources").join("tex-bundle.zip");
    BundleConfig {
        embedded_zip: embedded.is_file().then_some(embedded),
        allow_online: false,
        cache_dir: manifest().join("target").join("visutex-cache"),
        online_cache: None,
    }
}

/// Startzeile → Eintrag (für Fehlermeldungen mit Zeilennummer).
fn entry_at(starts: &[usize], line: usize) -> Option<usize> {
    starts.iter().rposition(|start| *start <= line)
}

fn parse_pt(value: &str) -> Option<f64> {
    value.trim().trim_end_matches("pt").parse().ok()
}

fn main() {
    let target = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest().join("resources").join("sketch-symbols.json"));
    let mut entries: Vec<&CatalogEntry> = ENTRIES.iter().collect();
    let mut rejected: Vec<String> = Vec::new();
    let output = loop {
        let (latex, starts) = symbols_document(&entries);
        eprintln!("Kompiliere {} Einträge …", entries.len());
        let result = compile(
            CompileRequest {
                latex,
                project_root: None,
                extra_search_paths: Vec::new(),
            },
            &bundle(),
            &mut |_| {},
        );
        let output = match result {
            Ok(output) => output,
            Err(failure) => panic!("{}\n{:?}", failure.message, failure.missing_files),
        };
        let mut bad: Vec<usize> = output
            .messages
            .iter()
            .filter(|message| message.severity == "error")
            .filter_map(|message| {
                message
                    .line
                    .and_then(|line| entry_at(&starts, line as usize))
            })
            .collect();
        bad.sort_unstable();
        bad.dedup();
        let unlocated = output
            .messages
            .iter()
            .filter(|message| message.severity == "error" && message.line.is_none())
            .count();
        if bad.is_empty() {
            if unlocated > 0 {
                eprintln!("Warnung: {unlocated} Fehler ohne Zeilenangabe:");
                for message in output.messages.iter().filter(|m| m.severity == "error") {
                    eprintln!("  {}", message.message);
                }
            }
            if !output.missing_files.is_empty() {
                eprintln!("Fehlende Dateien: {:?}", output.missing_files);
            }
            break output;
        }
        for message in output.messages.iter().filter(|m| m.severity == "error") {
            if let Some(index) = message
                .line
                .and_then(|line| entry_at(&starts, line as usize))
            {
                eprintln!(
                    "  {} (Zeile {:?}): {}",
                    entries[index].id,
                    message.line,
                    message.message.lines().next().unwrap_or("")
                );
            }
        }
        for index in bad.iter().rev() {
            rejected.push(entries[*index].id.to_string());
            entries.remove(*index);
        }
    };
    if !rejected.is_empty() {
        eprintln!(
            "Unbekannt in dieser CircuiTikZ-Version ({}): {:?}",
            rejected.len(),
            rejected
        );
    }

    // Anschlüsse aus dem Protokoll: `VA|<Eintrag>|<Anschluss>|<x>pt|<y>pt`
    let mut anchors: BTreeMap<usize, Vec<(String, f64, f64)>> = BTreeMap::new();
    for line in output.log.lines() {
        let parts: Vec<&str> = line.trim().split('|').collect();
        if parts.len() != 5 || parts[0] != "VA" {
            continue;
        }
        if let (Ok(index), Some(x), Some(y)) = (
            parts[1].parse::<usize>(),
            parse_pt(parts[3]),
            parse_pt(parts[4]),
        ) {
            anchors.entry(index).or_default().push((
                parts[2].to_string(),
                x / PT_PER_CM,
                y / PT_PER_CM,
            ));
        }
    }

    let stored = visutex_lib::pdf::load(output.pdf).expect("PDF lesbar");
    assert_eq!(
        stored.pages.len(),
        entries.len(),
        "Seitenzahl passt nicht zur Zahl der Einträge"
    );
    let px_per_cm = BP_PER_CM * RENDER_SCALE as f64;
    let origin = PAGE_CM / 2.0 * px_per_cm;
    let mut symbols = serde_json::Map::new();
    for (index, entry) in entries.iter().enumerate() {
        let png = visutex_lib::pdf::render_page_png(&stored, index, RENDER_SCALE).expect("Rastern");
        let image = image::load_from_memory(&png).expect("PNG").to_rgba8();
        let (width, height) = image.dimensions();
        let mut alpha = image::RgbaImage::new(width, height);
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0u32, 0u32);
        for (x, y, pixel) in image.enumerate_pixels() {
            let darkness = 255 - ((pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3) as u8;
            if darkness > 6 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
            alpha.put_pixel(x, y, image::Rgba([0, 0, 0, darkness]));
        }
        if right < left || bottom < top {
            eprintln!("Leeres Symbol: {}", entry.id);
            continue;
        }
        let pad = 2;
        let left = left.saturating_sub(pad);
        let top = top.saturating_sub(pad);
        let right = (right + pad).min(width - 1);
        let bottom = (bottom + pad).min(height - 1);
        let cropped =
            image::imageops::crop_imm(&alpha, left, top, right - left + 1, bottom - top + 1)
                .to_image();
        let mut bytes = std::io::Cursor::new(Vec::new());
        cropped
            .write_to(&mut bytes, image::ImageFormat::Png)
            .expect("PNG schreiben");
        let mut node_anchors: Vec<(String, f64, f64)> = anchors.remove(&index).unwrap_or_default();
        if node_anchors
            .iter()
            .any(|(name, _, _)| !GENERIC_ANCHORS.contains(&name.as_str()))
        {
            node_anchors.retain(|(name, _, _)| {
                !GENERIC_ANCHORS.contains(&name.as_str()) || name == "wiper"
            });
        } else {
            node_anchors.retain(|(name, _, _)| name == "center");
        }
        let round = |value: f64| (value * 1000.0).round() / 1000.0;
        symbols.insert(
            entry.id.to_string(),
            serde_json::json!({
                "png": base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
                "ox": round(origin - left as f64),
                "oy": round(origin - top as f64),
                "w": right - left + 1,
                "h": bottom - top + 1,
                "anchors": node_anchors
                    .iter()
                    .map(|(name, x, y)| serde_json::json!([name, round(*x), round(*y)]))
                    .collect::<Vec<_>>(),
            }),
        );
    }
    let json = serde_json::json!({
        "generator": "build-sketch-symbols",
        "pxPerCm": (px_per_cm * 1000.0).round() / 1000.0,
        "rejected": rejected,
        "symbols": symbols,
    });
    std::fs::write(&target, serde_json::to_string(&json).expect("JSON")).expect("schreiben");
    eprintln!(
        "{} Symbole geschrieben: {} ({} kB)",
        symbols_len(&json),
        target.display(),
        std::fs::metadata(&target)
            .map(|m| m.len() / 1024)
            .unwrap_or(0)
    );
}

fn symbols_len(json: &serde_json::Value) -> usize {
    json["symbols"]
        .as_object()
        .map_or(0, |symbols| symbols.len())
}
