//! Manifest deklarativer Add-ons (`addon.json`).
//!
//! Add-ons enthalten bewusst **keinen ausführbaren Code**: nur TeX-Dateien
//! (zusätzlicher Suchpfad für Tectonic), Präambelzeilen, Snippets, Ribbon-Einträge
//! und Vorlagen. Alle Werte werden hier validiert.

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
    pub icon: String,
    pub snippet: String,
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
        entry.icon = entry.icon.chars().take(4).collect();
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
}
