//! Literatur per DOI bzw. ISBN: Eingabe erkennen und prüfen, Buchdaten (OpenLibrary) in BibTeX
//! umwandeln, Zitierschlüssel im Stil `knuth1984` bilden und BibTeX lesbar formatieren.
//! Die Netzabfragen selbst stehen in `references.rs`.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Identifier {
    /// z. B. `10.1038/nature14539`
    Doi(String),
    /// 10 oder 13 Ziffern (ISBN-10 ggf. mit `X`), ohne Bindestriche
    Isbn(String),
}

/// Prüfziffer einer ISBN-10 bzw. ISBN-13.
pub fn isbn_valid(isbn: &str) -> bool {
    let chars: Vec<char> = isbn.chars().collect();
    match chars.len() {
        10 => {
            let mut sum = 0;
            for (index, c) in chars.iter().enumerate() {
                let value = match c {
                    '0'..='9' => c.to_digit(10).unwrap_or(0),
                    'X' | 'x' if index == 9 => 10,
                    _ => return false,
                };
                sum += value * (10 - index as u32);
            }
            sum % 11 == 0
        }
        13 => {
            if !chars.iter().all(char::is_ascii_digit) {
                return false;
            }
            let sum: u32 = chars
                .iter()
                .enumerate()
                .map(|(index, c)| c.to_digit(10).unwrap_or(0) * if index % 2 == 0 { 1 } else { 3 })
                .sum();
            sum % 10 == 0
        }
        _ => false,
    }
}

/// Erkennt DOI (auch als Link oder mit `doi:`) bzw. ISBN (mit oder ohne Bindestriche).
pub fn parse_identifier(input: &str) -> Result<Identifier, String> {
    let text = input.trim();
    let lower = text.to_lowercase();
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi.org/",
        "doi:",
        "doi ",
    ] {
        if lower.starts_with(prefix) {
            return parse_doi(text[prefix.len()..].trim());
        }
    }
    if text.starts_with("10.") {
        return parse_doi(text);
    }
    let digits: String = text
        .trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ':' || c.is_whitespace())
        .chars()
        .filter(|c| !matches!(c, '-' | ' ' | '\u{2010}' | '\u{2011}' | '\u{2013}'))
        .collect();
    if (digits.len() == 10 || digits.len() == 13)
        && digits.chars().take(9).all(|c| c.is_ascii_digit())
    {
        return if isbn_valid(&digits) {
            Ok(Identifier::Isbn(digits.to_uppercase()))
        } else {
            Err("Die ISBN hat eine falsche Prüfziffer – bitte die Eingabe kontrollieren.".into())
        };
    }
    Err("Weder eine DOI (10.xxxx/…) noch eine ISBN erkannt.".into())
}

fn parse_doi(doi: &str) -> Result<Identifier, String> {
    let doi = doi.trim().trim_end_matches(['.', ',', ';']);
    match doi.split_once('/') {
        Some((prefix, suffix))
            if prefix.starts_with("10.")
                && !suffix.is_empty()
                && !doi.contains(char::is_whitespace) =>
        {
            Ok(Identifier::Doi(doi.to_string()))
        }
        _ => Err("Das ist keine gültige DOI (Form: 10.xxxx/…).".into()),
    }
}

/// DOI für eine URL: alles außer Buchstaben, Ziffern und `-._~/;()` prozentkodieren.
pub fn encode_doi(doi: &str) -> String {
    let mut result = String::new();
    for byte in doi.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/;()".contains(&byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

/// Text für BibTeX: geschweifte Klammern ausgleichen, Sonderzeichen maskieren.
fn bib_value(text: &str) -> String {
    let mut result = String::new();
    for c in text.trim().chars() {
        match c {
            '{' | '}' | '\\' => {}
            '&' | '%' | '$' | '#' | '_' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Buchdaten aus der OpenLibrary-Antwort (`api/books?…&jscmd=data`) als BibTeX-Eintrag.
pub fn bibtex_from_openlibrary(isbn: &str, response: &Value) -> Option<String> {
    let book = response.as_object()?.values().next()?;
    let title = book.get("title")?.as_str()?;
    let full_title = match book.get("subtitle").and_then(Value::as_str) {
        Some(subtitle) if !subtitle.trim().is_empty() => format!("{title}: {subtitle}"),
        _ => title.to_string(),
    };
    let names = |key: &str| -> Vec<String> {
        book.get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("name").and_then(Value::as_str))
            .map(bib_value)
            .filter(|name| !name.is_empty())
            .collect()
    };
    let authors = names("authors");
    let publisher = names("publishers");
    let places = names("publish_places");
    let year: String = book
        .get("publish_date")
        .and_then(Value::as_str)
        .and_then(|date| {
            date.split(|c: char| !c.is_ascii_digit())
                .find(|part| part.len() == 4)
                .map(str::to_string)
        })
        .unwrap_or_default();
    let mut fields: Vec<(&str, String)> = Vec::new();
    if !authors.is_empty() {
        fields.push(("author", authors.join(" and ")));
    }
    fields.push(("title", bib_value(&full_title)));
    if let Some(publisher) = publisher.first() {
        fields.push(("publisher", publisher.clone()));
    }
    if let Some(place) = places.first() {
        fields.push(("address", place.clone()));
    }
    if !year.is_empty() {
        fields.push(("year", year));
    }
    fields.push(("isbn", isbn.to_string()));
    let body: Vec<String> = fields
        .into_iter()
        .map(|(name, value)| format!("  {name} = {{{value}}}"))
        .collect();
    Some(format!("@book{{schluessel,\n{}\n}}\n", body.join(",\n")))
}

/// ASCII-Kleinbuchstaben für Schlüssel (ä → ae, é → e …).
fn ascii_key_part(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        match c {
            'ä' | 'Ä' => result.push_str("ae"),
            'ö' | 'Ö' => result.push_str("oe"),
            'ü' | 'Ü' => result.push_str("ue"),
            'ß' => result.push_str("ss"),
            'á' | 'à' | 'â' | 'ã' | 'å' | 'Á' | 'À' | 'Â' => result.push('a'),
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' => result.push('e'),
            'í' | 'ì' | 'î' | 'ï' => result.push('i'),
            'ó' | 'ò' | 'ô' | 'õ' | 'ø' | 'Ó' => result.push('o'),
            'ú' | 'ù' | 'û' => result.push('u'),
            'ç' | 'Ç' => result.push('c'),
            'ñ' | 'Ñ' => result.push('n'),
            c if c.is_ascii_alphanumeric() => result.push(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    result
}

/// Zitierschlüssel `nachnameJahr` (z. B. `knuth1984`), bei Gleichheit `knuth1984a`, `…b`.
/// `author` wie in BibTeX (`Knuth, Donald and Bibby, Duane` bzw. `Donald Knuth`).
pub fn citation_key(author: &str, year: &str, title: &str, existing: &[String]) -> String {
    let first = author.split(" and ").next().unwrap_or("").trim();
    let surname = match first.split_once(',') {
        Some((surname, _)) => surname.trim(),
        None => first.split_whitespace().last().unwrap_or(""),
    };
    let mut base = ascii_key_part(surname);
    if base.is_empty() {
        base = title
            .split_whitespace()
            .map(ascii_key_part)
            .find(|word| word.len() > 3)
            .unwrap_or_else(|| "quelle".into());
    }
    base.push_str(&ascii_key_part(year));
    if !existing.contains(&base) {
        return base;
    }
    for suffix in 'a'..='z' {
        let candidate = format!("{base}{suffix}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    format!("{base}{}", existing.len())
}

/// Ersten Eintrag neu verschlüsseln und je Feld eine Zeile schreiben.
pub fn rekey_and_format(entry: &str, key: &str) -> Option<String> {
    let entry = entry.trim();
    let at = entry.find('@')?;
    let open = at + entry[at..].find('{')?;
    let entry_type = entry[at + 1..open].trim().to_lowercase();
    let comma = open + entry[open..].find(',')?;
    // Inhalt bis zur schließenden Klammer des Eintrags
    let mut depth = 0i32;
    let mut close = None;
    for (index, c) in entry[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &entry[comma + 1..close?];
    // Felder an Kommas der obersten Ebene trennen
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut level = 0i32;
    let mut quoted = false;
    for c in body.chars() {
        match c {
            '{' => level += 1,
            '}' => level -= 1,
            '"' if level == 0 => quoted = !quoted,
            ',' if level == 0 && !quoted => {
                fields.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    fields.push(current);
    let lines: Vec<String> = fields
        .iter()
        .map(|field| field.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|field| !field.is_empty())
        .map(|field| match field.split_once('=') {
            Some((name, value)) => format!("  {} = {}", name.trim().to_lowercase(), value.trim()),
            None => format!("  {field}"),
        })
        .collect();
    Some(format!(
        "@{entry_type}{{{key},\n{}\n}}\n",
        lines.join(",\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_dois_and_isbns() {
        let doi = Identifier::Doi("10.1038/nature14539".into());
        assert_eq!(parse_identifier("10.1038/nature14539").unwrap(), doi);
        assert_eq!(
            parse_identifier("https://doi.org/10.1038/nature14539").unwrap(),
            doi
        );
        assert_eq!(parse_identifier("doi: 10.1038/nature14539.").unwrap(), doi);
        assert_eq!(parse_identifier("DOI:10.1038/nature14539").unwrap(), doi);
        // The TeXbook
        assert_eq!(
            parse_identifier("978-0-201-13447-6").unwrap(),
            Identifier::Isbn("9780201134476".into())
        );
        assert_eq!(
            parse_identifier("ISBN 0-201-13447-0").unwrap(),
            Identifier::Isbn("0201134470".into())
        );
        assert_eq!(
            parse_identifier("0-8044-2957-x").unwrap(),
            Identifier::Isbn("080442957X".into())
        );
        assert!(parse_identifier("978-0-201-13447-7")
            .unwrap_err()
            .contains("Prüfziffer"));
        assert!(parse_identifier("hallo welt").is_err());
        assert!(parse_identifier("10.1038").is_err());
        assert_eq!(
            encode_doi("10.1002/(SICI)1097-0258<1::AID>3.0.CO;2-#"),
            "10.1002/(SICI)1097-0258%3C1%3A%3AAID%3E3.0.CO;2-%23"
        );
    }

    #[test]
    fn builds_bibtex_from_openlibrary_data() {
        let response = json!({ "ISBN:9780201134476": {
            "title": "The TeXbook", "subtitle": "",
            "authors": [{ "name": "Donald E. Knuth" }],
            "publishers": [{ "name": "Addison-Wesley" }],
            "publish_places": [{ "name": "Reading, Mass" }],
            "publish_date": "1984"
        }});
        let bibtex = bibtex_from_openlibrary("9780201134476", &response).unwrap();
        let entries = crate::core::bibtex::parse(&bibtex);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "The TeXbook");
        assert_eq!(entries[0].year, "1984");
        assert!(bibtex.contains("publisher = {Addison-Wesley}"));
        assert!(bibtex_from_openlibrary("1", &json!({})).is_none());
    }

    #[test]
    fn makes_unique_readable_keys() {
        assert_eq!(
            citation_key("Knuth, Donald E.", "1984", "", &[]),
            "knuth1984"
        );
        assert_eq!(
            citation_key("Anna Müller and Bernd Schmidt", "2020", "", &[]),
            "mueller2020"
        );
        assert_eq!(
            citation_key("Knuth, Donald", "1984", "", &["knuth1984".into()]),
            "knuth1984a"
        );
        assert_eq!(
            citation_key("", "2001", "Die Kunst der Programmierung", &[]),
            "kunst2001"
        );
    }

    #[test]
    fn reformats_one_line_bibtex_with_new_key() {
        let crossref = " @article{LeCun_2015, title={Deep learning}, volume={521}, url={http://dx.doi.org/10.1038/nature14539}, DOI={10.1038/nature14539}, number={7553}, journal={Nature}, publisher={Springer Science and Business Media LLC}, author={LeCun, Yann and Bengio, Yoshua and Hinton, Geoffrey}, year={2015}, month=may, pages={436–444} }";
        let formatted = rekey_and_format(crossref, "lecun2015").unwrap();
        assert!(
            formatted.starts_with("@article{lecun2015,\n  title = {Deep learning},\n"),
            "{formatted}"
        );
        assert!(formatted.contains("\n  doi = {10.1038/nature14539},\n"));
        assert!(formatted.contains("\n  month = may,\n"));
        assert!(formatted.trim_end().ends_with("pages = {436–444}\n}"));
        let entries = crate::core::bibtex::parse(&formatted);
        assert_eq!(entries[0].key, "lecun2015");
        assert_eq!(entries[0].year, "2015");
    }
}
