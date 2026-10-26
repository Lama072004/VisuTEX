//! Manifest deklarativer Add-ons (`addon.json`).
//!
//! Add-ons enthalten bewusst **keinen ausführbaren Code**: nur TeX-Dateien
//! (zusätzlicher Suchpfad für Tectonic), Präambelzeilen, Snippets, Ribbon-Einträge,
//! Vorlagen und SVG-Symbole. Alle Werte werden hier validiert.
//! Anleitung für Add-on-Ersteller: `docs/ADDONS.md`.

use super::settings::is_safe_relative_path;
use serde::{Deserialize, Serialize};

pub const SNIPPET_KINDS: &[&str] = &["inline", "block", "math", "inlineMath", "tikz"];
const TEX_EXTENSIONS: &[&str] = &[
    "sty", "cls", "tex", "bst", "bib", "def", "cfg", "clo", "fd", "ldf", "png", "jpg", "jpeg",
    "pdf", "otf", "ttf", "tfm", "pfb", "enc", "map", "vf",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub group: String,
    /// inline | block | math | inlineMath | tikz
    pub kind: String,
    pub latex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RibbonEntry {
    pub group: String,
    pub label: String,
    /// Kurzer Text (höchstens 4 Zeichen, z. B. „Ω“) oder Pfad zu einer SVG-Datei im Add-on
    /// (z. B. `icons/widerstand.svg`)
    pub icon: String,
    pub snippet: String,
}

/// Höchstgröße einer SVG-Symboldatei.
pub const MAX_ICON_BYTES: usize = 64 * 1024;

/// Verweist `icon` auf eine SVG-Datei (statt eines Textsymbols)?
pub fn is_svg_icon(icon: &str) -> bool {
    icon.to_ascii_lowercase().ends_with(".svg")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateRef {
    pub id: String,
    pub name: String,
    pub file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AddonManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub tex_files: Vec<String>,
    pub preamble: Vec<String>,
    pub snippets: Vec<Snippet>,
    pub ribbon: Vec<RibbonEntry>,
    pub templates: Vec<TemplateRef>,
    /// Symbol des Add-ons (SVG-Datei im Add-on, optional)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
}

impl AddonManifest {
    /// Alle SVG-Symboldateien (Add-on und Menüband-Einträge), ohne Doppelte.
    pub fn icon_files(&self) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        for icon in std::iter::once(&self.icon).chain(self.ribbon.iter().map(|entry| &entry.icon)) {
            if is_svg_icon(icon) && !files.contains(icon) {
                files.push(icon.clone());
            }
        }
        files
    }
}

/// Inhalt einer SVG-Symboldatei prüfen: Größe, SVG-Wurzelelement, kein Skript und keine
/// externen Verweise (angezeigt wird das Symbol ohnehin als Bild, wo Skripte nicht laufen).
pub fn check_svg_icon(name: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ICON_BYTES {
        return Err(format!("Das Symbol „{name}“ ist größer als 64 KB."));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| format!("Das Symbol „{name}“ ist keine UTF-8-Textdatei."))?;
    let lower = text.to_ascii_lowercase();
    if !lower.contains("<svg") {
        return Err(format!("„{name}“ ist keine SVG-Datei."));
    }
    for forbidden in [
        "<script",
        "javascript:",
        "<foreignobject",
        "<iframe",
        "<embed",
        "<object",
    ] {
        if lower.contains(forbidden) {
            return Err(format!(
                "Das Symbol „{name}“ enthält Unzulässiges ({forbidden})."
            ));
        }
    }
    // Ereignis-Attribute (onload=, onclick= …)
    let bytes_lower = lower.as_bytes();
    for (index, _) in lower.match_indices("on") {
        let before = index.checked_sub(1).map(|i| bytes_lower[i]);
        if !matches!(before, Some(b' ' | b'\n' | b'\t' | b'\r' | b'/')) {
            continue;
        }
        let name_end = lower[index + 2..]
            .find(|c: char| !c.is_ascii_lowercase())
            .map(|offset| index + 2 + offset)
            .unwrap_or(lower.len());
        if name_end > index + 2 && lower[name_end..].trim_start().starts_with('=') {
            return Err(format!(
                "Das Symbol „{name}“ enthält Ereignis-Attribute (z. B. onload)."
            ));
        }
    }
    // Nur eingebettete Inhalte: externe Verweise (http, Dateien) sind nicht erlaubt
    for attribute in ["href=\"", "href='", "src=\"", "src='"] {
        let mut rest = lower.as_str();
        while let Some(index) = rest.find(attribute) {
            let value = &rest[index + attribute.len()..];
            if !(value.starts_with('#') || value.starts_with("data:image/")) {
                return Err(format!(
                    "Das Symbol „{name}“ verweist auf eine externe Datei."
                ));
            }
            rest = value;
        }
    }
    if lower.contains("url(http") || lower.contains("@import") {
        return Err(format!(
            "Das Symbol „{name}“ verweist auf eine externe Datei."
        ));
    }
    Ok(())
}

pub fn is_valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn clean(text: &str, max: usize) -> String {
    text.replace(['\r', '\n'], " ")
        .trim()
        .chars()
        .take(max)
        .collect()
}

/// Liest und validiert ein Manifest. Unzulässige Einträge führen zu einem Fehler
/// (statt stillschweigend ignoriert zu werden), damit Add-on-Autoren sie bemerken.
pub fn parse_manifest(text: &str) -> Result<AddonManifest, String> {
    let manifest: AddonManifest = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("addon.json ist ungültig: {error}"))?;
    validate(manifest)
}

pub fn validate(mut manifest: AddonManifest) -> Result<AddonManifest, String> {
    if !is_valid_id(&manifest.id) {
        return Err("Die Add-on-ID darf nur Kleinbuchstaben, Ziffern, „-“ und „_“ enthalten (1–64 Zeichen).".into());
    }
    manifest.name = clean(&manifest.name, 80);
    if manifest.name.is_empty() {
        manifest.name = manifest.id.clone();
    }
    manifest.version = clean(&manifest.version, 20);
    if manifest.version.is_empty() {
        manifest.version = "1.0.0".into();
    }
    manifest.author = clean(&manifest.author, 80);
    manifest.description = clean(&manifest.description, 500);

    for file in &mut manifest.tex_files {
        *file = file.trim().replace('\\', "/");
        let extension = file
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .unwrap_or_default();
        if !is_safe_relative_path(file) || !TEX_EXTENSIONS.contains(&extension.as_str()) {
            return Err(format!("Unzulässige TeX-Datei im Add-on: „{file}“"));
        }
    }
    if manifest.preamble.len() > 100 {
        return Err("Ein Add-on darf höchstens 100 Präambelzeilen enthalten.".into());
    }
    for line in &manifest.preamble {
        if line.len() > 1000 || line.contains("\\write18") || line.contains("\\openout") {
            return Err(format!(
                "Unzulässige Präambelzeile im Add-on: „{}“",
                clean(line, 60)
            ));
        }
        if line.contains("\\begin{document}") || line.contains("\\documentclass") {
            return Err(
                "Präambelzeilen dürfen weder \\documentclass noch \\begin{document} enthalten."
                    .into(),
            );
        }
    }
    let mut snippet_ids = Vec::new();
    for snippet in &mut manifest.snippets {
        if !is_valid_id(&snippet.id) || snippet_ids.contains(&snippet.id) {
            return Err(format!(
                "Ungültige oder doppelte Snippet-ID „{}“.",
                snippet.id
            ));
        }
        snippet_ids.push(snippet.id.clone());
        if !SNIPPET_KINDS.contains(&snippet.kind.as_str()) {
            snippet.kind = "inline".into();
        }
        snippet.name = clean(&snippet.name, 80);
        if snippet.name.is_empty() {
            snippet.name = snippet.id.clone();
        }
        snippet.group = clean(&snippet.group, 60);
        if snippet.latex.len() > 20_000 {
            return Err(format!("Snippet „{}“ ist zu lang.", snippet.id));
        }
    }
    for entry in &mut manifest.ribbon {
        if !snippet_ids.contains(&entry.snippet) {
            return Err(format!(
                "Ribbon-Eintrag „{}“ verweist auf ein unbekanntes Snippet „{}“.",
                entry.label, entry.snippet
            ));
        }
        entry.group = clean(&entry.group, 40);
        if entry.group.is_empty() {
            entry.group = manifest.name.clone();
        }
        entry.label = clean(&entry.label, 40);
        entry.icon = entry.icon.trim().replace('\\', "/");
        if is_svg_icon(&entry.icon) {
            if !is_safe_relative_path(&entry.icon) || entry.icon.len() > 200 {
                return Err(format!(
                    "Unzulässiger Symbolpfad „{}“ im Add-on.",
                    entry.icon
                ));
            }
        } else {
            entry.icon = entry.icon.chars().take(4).collect();
        }
    }
    manifest.icon = manifest.icon.trim().replace('\\', "/");
    if !manifest.icon.is_empty()
        && (!is_svg_icon(&manifest.icon)
            || !is_safe_relative_path(&manifest.icon)
            || manifest.icon.len() > 200)
    {
        return Err(format!(
            "Das Add-on-Symbol muss eine SVG-Datei im Add-on sein („{}“).",
            manifest.icon
        ));
    }
    for template in &mut manifest.templates {
        template.file = template.file.trim().replace('\\', "/");
        if !is_valid_id(&template.id) || !is_safe_relative_path(&template.file) {
            return Err(format!("Ungültige Vorlage „{}“ im Add-on.", template.id));
        }
        template.name = clean(&template.name, 80);
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_manifest() {
        let manifest = parse_manifest(
            r#"{ "id": "elektrotechnik", "name": "Elektrotechnik", "texFiles": ["tex/mein.sty"],
                 "preamble": ["\\usepackage{siunitx}"],
                 "snippets": [{ "id": "si", "name": "SI-Einheit", "kind": "inline", "latex": "\\SI{1}{\\ohm}" }],
                 "ribbon": [{ "label": "Widerstand", "icon": "Ω", "snippet": "si" }] }"#,
        )
        .unwrap();
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.ribbon[0].group, "Elektrotechnik");

        assert!(parse_manifest(r#"{ "id": "Böse ID" }"#).is_err());
        assert!(parse_manifest(r#"{ "id": "x", "texFiles": ["../x.sty"] }"#).is_err());
        assert!(
            parse_manifest(r#"{ "id": "x", "preamble": ["\\immediate\\write18{rm -rf /}"] }"#)
                .is_err()
        );
        assert!(parse_manifest(
            r#"{ "id": "x", "ribbon": [{ "label": "a", "snippet": "fehlt" }] }"#
        )
        .is_err());
    }

    #[test]
    fn accepts_svg_icons_and_rejects_unsafe_ones() {
        let manifest = parse_manifest(
            r#"{ "id": "x", "icon": "icons/addon.svg",
                 "snippets": [{ "id": "a", "latex": "a" }, { "id": "b", "latex": "b" }],
                 "ribbon": [{ "label": "A", "icon": "icons\\a.svg", "snippet": "a" },
                            { "label": "B", "icon": "Ωmega", "snippet": "b" }] }"#,
        )
        .unwrap();
        assert_eq!(manifest.ribbon[0].icon, "icons/a.svg");
        assert_eq!(manifest.ribbon[1].icon, "Ωmeg");
        assert_eq!(
            manifest.icon_files(),
            vec!["icons/addon.svg", "icons/a.svg"]
        );
        assert!(parse_manifest(r#"{ "id": "x", "icon": "logo.png" }"#).is_err());
        assert!(parse_manifest(r#"{ "id": "x", "icon": "../fremd.svg" }"#).is_err());

        let good = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><use href="#p"/><path id="p" d="M0 0h24" stroke="currentColor"/></svg>"##;
        assert!(check_svg_icon("gut.svg", good).is_ok());
        assert!(check_svg_icon("x.svg", b"<svg><script>alert(1)</script></svg>").is_err());
        assert!(check_svg_icon(
            "x.svg",
            br#"<svg onload="x"><image href="https://example.org/a.png"/></svg>"#
        )
        .is_err());
        assert!(check_svg_icon("x.svg", b"kein svg").is_err());
        assert!(check_svg_icon("x.svg", br#"<svg onload="alert(1)"/>"#).is_err());
        assert!(check_svg_icon("x.svg", br#"<svg><g font="Monotype"/></svg>"#).is_ok());
        assert!(check_svg_icon("x.svg", &vec![b' '; MAX_ICON_BYTES + 1]).is_err());
    }
}
