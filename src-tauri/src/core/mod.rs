//! VisuTeX-Kern: reine, testbare Logik ohne Tauri-Abhängigkeiten.
//!
//! - `settings`: Dokumenteinstellungen (tolerantes Einlesen + Validierung)
//! - `escape`: Maskierung von Text/URLs für LaTeX
//! - `markers`: Kommentar-Marker für strukturierte Bereiche
//! - `preamble`: engine-neutrale Präambel (pdfLaTeX/XeLaTeX/LuaLaTeX/Tectonic)
//! - `export`: Tiptap-JSON → vollständiges LaTeX-Dokument
//! - `import`: LaTeX → Tiptap-JSON (verlustfrei, Roh-LaTeX als Fallback)
//! - `bibtex`: .bib-Parser und Zusammenführen von Einträgen
//! - `project`: Projektformat v2 und Migration alter Projektdateien
//! - `analysis`: Labels, Zitate und Statistik für Dialoge und Statusleiste
//! - `addon`: Manifest der deklarativen Add-ons
//! - `slides`: Folien-Editor (Datenmodell, Export nach LaTeX-Beamer)

pub mod addon;
pub mod analysis;
pub mod bibtex;
pub mod escape;
pub mod export;
pub mod import;
pub mod includes;
pub mod languages;
pub mod macros;
pub mod markers;
pub mod preamble;
pub mod project;
pub mod settings;
pub mod sketch;
pub mod sketch_catalog;
pub mod slides;
pub mod support;
