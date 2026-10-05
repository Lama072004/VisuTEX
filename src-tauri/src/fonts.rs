//! Installierte Schriften (einmalig ermittelt, im Hintergrund).
//!
//! Damit beantwortet Rust die Schriftprüfung der Präambel (`\visutexfontcheck`)
//! vorab, statt Tectonic bei jedem Lauf alle Systemschriften durchsuchen zu lassen
//! (gemessen: 4 min 45 s statt 15,7 s). Außerdem zeigt die Oberfläche an,
//! welche Schriften vorhanden sind.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};
use svg2pdf::usvg::fontdb;

static FAMILIES: OnceLock<BTreeSet<String>> = OnceLock::new();
static FONTCONFIG: Once = Once::new();

/// Ordner mit Systemschriften des Betriebssystems.
fn system_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    if cfg!(windows) {
        dirs.push(
            env("WINDIR")
                .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
                .join("Fonts"),
        );
        if let Some(local) = env("LOCALAPPDATA") {
            dirs.push(local.join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = env("HOME") {
            dirs.push(home.join("Library/Fonts"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(data) = env("XDG_DATA_HOME") {
            dirs.push(data.join("fonts"));
        }
        if let Some(home) = env("HOME") {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
    }
    dirs.into_iter().filter(|dir| dir.is_dir()).collect()
}

fn cache_base() -> PathBuf {
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| home.join("Library/Caches"))
    } else {
        env("XDG_CACHE_HOME").or_else(|| env("HOME").map(|home| home.join(".cache")))
    };
    base.unwrap_or_else(std::env::temp_dir).join("VisuTeX")
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Fontconfig-Konfiguration für die eingebaute TeX-Engine (XeTeX sucht Systemschriften
/// über Fontconfig). Die statisch gelinkte Bibliothek kennt den Ort der
/// System-Konfiguration nicht: Unter Linux wird die vorhandene `/etc/fonts/fonts.conf`
/// verwendet, sonst (Windows) eine eigene mit den Schriftordnern und einem Cache.
pub fn configure_fontconfig() {
    FONTCONFIG.call_once(|| {
        if std::env::var_os("FONTCONFIG_FILE").is_some() {
            return;
        }
        if cfg!(all(unix, not(target_os = "macos"))) {
            for candidate in ["/etc/fonts/fonts.conf", "/usr/local/etc/fonts/fonts.conf"] {
                if Path::new(candidate).is_file() {
                    std::env::set_var("FONTCONFIG_FILE", candidate);
                    return;
                }
            }
        }
        let base = cache_base();
        let cache = base.join("fontconfig");
        if std::fs::create_dir_all(&cache).is_err() {
            return;
        }
        let mut config = String::from(
            "<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n<fontconfig>\n",
        );
        for dir in system_font_dirs() {
            config.push_str(&format!(
                "  <dir>{}</dir>\n",
                xml_escape(&dir.display().to_string())
            ));
        }
        config.push_str(&format!(
            "  <cachedir>{}</cachedir>\n</fontconfig>\n",
            xml_escape(&cache.display().to_string())
        ));
        let path = base.join("fonts.conf");
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != config && std::fs::write(&path, &config).is_err() {
            return;
        }
        std::env::set_var("FONTCONFIG_FILE", &path);
    });
}

/// Familiennamen (klein geschrieben) aller installierten Schriften.
pub fn installed_families() -> &'static BTreeSet<String> {
    FAMILIES.get_or_init(|| {
        let mut database = fontdb::Database::new();
        database.load_system_fonts();
        let mut families = BTreeSet::new();
        for face in database.faces() {
            for (name, _) in &face.families {
                families.insert(name.to_lowercase());
            }
            families.insert(face.post_script_name.to_lowercase());
        }
        families
    })
}

pub fn is_installed(font: &str) -> bool {
    installed_families().contains(&font.trim().to_lowercase())
}

/// Präfix mit Vorab-Ergebnissen für alle im Dokument geprüften Schriften, die
/// sicher installiert sind (siehe `core::preamble::font_hint_prefix`).
pub fn hint_prefix(latex: &str) -> String {
    let fonts = crate::core::preamble::checked_fonts(latex);
    let installed: Vec<&str> = fonts
        .iter()
        .map(String::as_str)
        .filter(|font| is_installed(font))
        .collect();
    crate::core::preamble::font_hint_prefix(installed)
}
