//! Literatur per DOI bzw. ISBN nachschlagen (braucht Internet, nur auf Anfrage des Nutzers):
//! - DOI → `https://doi.org/<doi>` mit `Accept: application/x-bibtex` (Crossref, DataCite …)
//! - ISBN → OpenLibrary (`openlibrary.org/api/books`), daraus BibTeX
//!
//! Ergebnis ist ein Vorschlag (BibTeX mit lesbarem Schlüssel wie `knuth1984`); in die
//! Literaturdatei schreibt erst `add_bib_entries`, wenn der Nutzer bestätigt.

use crate::core::bibtex;
use crate::core::identifiers::{self, Identifier};
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundReference {
    pub key: String,
    pub bibtex: String,
    pub title: String,
    pub author: String,
    pub year: String,
    pub container: String,
    /// doi | isbn
    pub source: &'static str,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .user_agent(format!(
            "VisuTeX/{} (LaTeX-Editor)",
            crate::updates::current_version()
        ))
        .build()
        .map_err(|error| format!("HTTP-Client konnte nicht erstellt werden: {error}"))
}

fn offline(_: reqwest::Error) -> String {
    "Keine Verbindung – für die Suche per DOI/ISBN wird Internet benötigt.".into()
}

/// Sucht die Quelle und liefert BibTeX mit einem Schlüssel, der in `existing` noch nicht vorkommt.
pub fn lookup(input: &str, existing: &[String]) -> Result<FoundReference, String> {
    let identifier = identifiers::parse_identifier(input)?;
    let (raw, source) = match &identifier {
        Identifier::Doi(doi) => {
            let response = client()?
                .get(format!("https://doi.org/{}", identifiers::encode_doi(doi)))
                .header("Accept", "application/x-bibtex; charset=utf-8")
                .send()
                .map_err(offline)?;
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(format!("Die DOI „{doi}“ ist nicht registriert."));
            }
            if !response.status().is_success() {
                return Err(format!(
                    "Der DOI-Dienst antwortet mit Fehler {}.",
                    response.status()
                ));
            }
            let text = response.text().map_err(offline)?;
            if !text.trim_start().starts_with('@') {
                return Err(format!("Zur DOI „{doi}“ gibt es keine Literaturangabe."));
            }
            (text, "doi")
        }
        Identifier::Isbn(isbn) => {
            let response = client()?
                .get(format!(
                    "https://openlibrary.org/api/books?bibkeys=ISBN:{isbn}&format=json&jscmd=data"
                ))
                .send()
                .map_err(offline)?;
            if !response.status().is_success() {
                return Err(format!(
                    "OpenLibrary antwortet mit Fehler {}.",
                    response.status()
                ));
            }
            let text = response.text().map_err(offline)?;
            let json: serde_json::Value = serde_json::from_str(&text)
                .map_err(|_| "Die Antwort von OpenLibrary ist unlesbar.".to_string())?;
            let bibtex = identifiers::bibtex_from_openlibrary(isbn, &json)
                .ok_or_else(|| format!("Zur ISBN {isbn} wurde kein Buch gefunden."))?;
            (bibtex, "isbn")
        }
    };
    let entry = bibtex::parse(&raw)
        .into_iter()
        .next()
        .ok_or("Die gelieferte Literaturangabe ist unlesbar.")?;
    // `author` im BibTeX-Original (für den Schlüssel); `entry.author` ist für die Anzeige aufbereitet
    let author_field = entry.author.replace("; ", " and ");
    let key = identifiers::citation_key(&author_field, &entry.year, &entry.title, existing);
    let bibtex = identifiers::rekey_and_format(&raw, &key)
        .ok_or("Die gelieferte Literaturangabe ist unlesbar.")?;
    Ok(FoundReference {
        key,
        bibtex,
        title: entry.title,
        author: entry.author,
        year: entry.year,
        container: entry.container,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Echte Abfrage (Internet): `cargo test --lib references -- --ignored`
    #[test]
    #[ignore]
    fn looks_up_doi_and_isbn_online() {
        let paper = lookup("https://doi.org/10.1038/nature14539", &[]).unwrap();
        assert_eq!(paper.key, "lecun2015");
        assert_eq!(paper.year, "2015");
        assert!(
            paper.bibtex.starts_with("@article{lecun2015,"),
            "{}",
            paper.bibtex
        );
        // Jahr laut OpenLibrary (Nachdruck) – Schlüssel daher nur auf den Namen prüfen
        let book = lookup("978-0-201-13447-6", &[]).unwrap();
        assert!(book.key.starts_with("knuth"), "{}", book.key);
        assert!(book.title.contains("TeXbook"), "{}", book.title);
        let again = lookup("978-0-201-13447-6", std::slice::from_ref(&book.key)).unwrap();
        assert_eq!(again.key, format!("{}a", book.key));
        assert!(lookup("10.9999/gibt-es-nicht-4711", &[]).is_err());
    }
}
