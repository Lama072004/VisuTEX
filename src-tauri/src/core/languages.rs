//! Dokumentsprachen: babel-Option, Anführungszeichen, Dezimalzeichen,
//! siunitx-Locale und Beschriftungen (Titelseite, Verzeichnisse).
//!
//! Einzige Quelle ist `src/latex/document-languages.json` – dieselbe Datei
//! nutzt die Oberfläche (`src/latex/languages.ts`).

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../../../src/latex/document-languages.json");
pub const DEFAULT_LANGUAGE: &str = "ngerman";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLabels {
    pub author: String,
    pub supervisor: String,
    pub institution: String,
    pub title: String,
    pub summary: String,
    pub acronyms: String,
    pub contents: String,
    pub figures: String,
    pub tables: String,
    pub bibliography: String,
    pub list: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLanguage {
    /// Wert in den Dokumenteinstellungen
    pub id: String,
    /// babel-Option
    pub babel: String,
    /// Name in der Sprache selbst
    pub name: String,
    /// BCP-47 (Datumsformat, Rechtschreibprüfung)
    pub bcp47: String,
    /// weitere babel-Namen (Import fremder Dokumente)
    pub aliases: Vec<String>,
    pub quotes: [String; 2],
    pub decimal: String,
    /// siunitx `locale` (leer: nur Dezimalzeichen setzen)
    pub siunitx: String,
    pub range: String,
    pub german_shorthands: bool,
    /// zusätzliche babel-Optionen (z. B. ohne aktive Zeichen)
    pub babel_options: Vec<String>,
    /// zusätzliche Paketoptionen (z. B. `shorthands=off`)
    pub package_options: String,
    pub labels: LanguageLabels,
}

pub fn all() -> &'static [DocumentLanguage] {
    static LANGUAGES: OnceLock<Vec<DocumentLanguage>> = OnceLock::new();
    LANGUAGES.get_or_init(|| {
        serde_json::from_str(SOURCE).expect("document-languages.json ist gültig (Test)")
    })
}

pub fn ids() -> Vec<&'static str> {
    all().iter().map(|language| language.id.as_str()).collect()
}

/// Sprache zu einer ID (unbekannt → Deutsch).
pub fn get(id: &str) -> &'static DocumentLanguage {
    all()
        .iter()
        .find(|language| language.id == id)
        .or_else(|| {
            all()
                .iter()
                .find(|language| language.id == DEFAULT_LANGUAGE)
        })
        .expect("Standardsprache vorhanden")
}

/// Sprache zu einer babel-Option bzw. einem Alias aus einem fremden Dokument.
pub fn from_babel(option: &str) -> Option<&'static str> {
    let option = option.trim();
    all()
        .iter()
        .find(|language| {
            language.id == option
                || language.babel == option
                || language.aliases.iter().any(|alias| alias == option)
        })
        .map(|language| language.id.as_str())
}

/// `\usepackage[…]{babel}` für die Sprache.
pub fn babel_line(id: &str) -> String {
    let language = get(id);
    let mut options: Vec<&str> = Vec::new();
    if !language.package_options.is_empty() {
        options.push(&language.package_options);
    }
    options.push(&language.babel);
    options.extend(language.babel_options.iter().map(String::as_str));
    format!("\\usepackage[{}]{{babel}}", options.join(","))
}

/// `\sisetup{…}` passend zur Sprache.
pub fn siunitx_setup(id: &str) -> String {
    let language = get(id);
    if language.siunitx.is_empty() {
        format!(
            "\\sisetup{{output-decimal-marker={{{}}}}}",
            language.decimal
        )
    } else {
        format!("\\sisetup{{locale={}}}", language.siunitx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn table_is_complete_and_unique() {
        let languages = all();
        assert!(languages.len() >= 20);
        let mut names = BTreeSet::new();
        for language in languages {
            assert!(
                names.insert(language.id.clone()),
                "doppelt: {}",
                language.id
            );
            for alias in &language.aliases {
                assert!(
                    names.insert(alias.clone()) || alias == &language.id,
                    "Alias doppelt: {alias}"
                );
            }
            assert!(!language.babel.is_empty() && !language.bcp47.is_empty());
            assert!(language.decimal == "," || language.decimal == ".");
            assert!(
                !language.labels.contents.is_empty() && !language.labels.bibliography.is_empty()
            );
        }
        assert_eq!(get("gibtesnicht").id, "ngerman");
        assert_eq!(from_babel("frenchb"), Some("french"));
        assert_eq!(from_babel("USenglish"), Some("american"));
        assert_eq!(from_babel("british"), Some("english"));
        assert_eq!(
            babel_line("spanish"),
            "\\usepackage[spanish,es-noshorthands,es-nodecimaldot]{babel}"
        );
        assert_eq!(
            babel_line("turkish"),
            "\\usepackage[shorthands=off,turkish]{babel}"
        );
        assert_eq!(siunitx_setup("ngerman"), "\\sisetup{locale=DE}");
        assert_eq!(
            siunitx_setup("italian"),
            "\\sisetup{output-decimal-marker={,}}"
        );
    }
}
