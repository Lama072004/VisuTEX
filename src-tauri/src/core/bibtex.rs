//! Minimaler, robuster BibTeX-Parser für die Zitatauswahl und zum Einfügen
//! von Zotero-Einträgen in die Projekt-.bib-Datei.

use super::import::latex_to_plain_text;
use super::import::read_group;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BibEntry {
    pub key: String,
    pub entry_type: String,
    pub title: String,
    pub author: String,
    pub year: String,
    pub container: String,
    /// Unveränderter Quelltext des Eintrags.
    pub raw: String,
}

fn field_value(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let mut index = start;
    let mut value = String::new();
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        match bytes.get(index)? {
            b'{' => {
                let (content, end) = read_group(source, index, b'{', b'}')?;
                value.push_str(content);
                index = end;
            }
            b'"' => {
                let mut depth = 0;
                let mut end = index + 1;
                while end < bytes.len() {
                    match bytes[end] {
                        b'\\' => end += 1,
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'"' if depth == 0 => break,
                        _ => {}
                    }
                    end += 1;
                }
                value.push_str(source.get(index + 1..end)?);
                index = end + 1;
            }
            _ => {
                let end = source[index..]
                    .find(|c: char| {
                        c == ',' || c == '}' || c == ')' || c == '#' || c.is_whitespace()
                    })
                    .map(|i| index + i)
                    .unwrap_or(bytes.len());
                value.push_str(&source[index..end]);
                index = end;
            }
        }
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) == Some(&b'#') {
            index += 1;
            continue;
        }
        return Some((value, index));
    }
}

/// Parst alle Einträge einer .bib-Datei (Kommentare/@string/@preamble werden übersprungen).
pub fn parse(source: &str) -> Vec<BibEntry> {
    let mut entries = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    while let Some(offset) = source[index..].find('@') {
        let start = index + offset;
        let type_end = source[start + 1..]
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .map(|i| start + 1 + i)
            .unwrap_or(bytes.len());
        let entry_type = source[start + 1..type_end].to_ascii_lowercase();
        let mut open = type_end;
        while open < bytes.len() && bytes[open].is_ascii_whitespace() {
            open += 1;
        }
        let (body, end) = match bytes.get(open) {
            Some(b'{') => match read_group(source, open, b'{', b'}') {
                Some(group) => group,
                None => break,
            },
            Some(b'(') => match read_group(source, open, b'(', b')') {
                Some(group) => group,
                None => break,
            },
            _ => {
                index = start + 1;
                continue;
            }
        };
        index = end;
        if matches!(entry_type.as_str(), "comment" | "string" | "preamble") || entry_type.is_empty()
        {
            continue;
        }
        let Some(comma) = body.find(',') else {
            continue;
        };
        let key = body[..comma].trim().to_string();
        if key.is_empty() {
            continue;
        }
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut cursor = comma + 1;
        while cursor < body.len() {
            let rest = &body[cursor..];
            let Some(equals) = rest.find('=') else { break };
            let name = rest[..equals]
                .trim()
                .trim_start_matches(',')
                .trim()
                .to_ascii_lowercase();
            let Some((value, value_end)) = field_value(body, cursor + equals + 1) else {
                break;
            };
            fields.push((name, value));
            cursor = value_end;
            while cursor < body.len()
                && (body.as_bytes()[cursor] == b','
                    || body.as_bytes()[cursor].is_ascii_whitespace())
            {
                cursor += 1;
            }
        }
        let get = |name: &str| -> String {
            fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, value)| latex_to_plain_text(value))
                .unwrap_or_default()
        };
        let year = {
            let year = get("year");
            if year.is_empty() {
                get("date").chars().take(4).collect()
            } else {
                year
            }
        };
        let container = [
            get("journal"),
            get("journaltitle"),
            get("booktitle"),
            get("publisher"),
            get("school"),
            get("institution"),
        ]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or_default();
        entries.push(BibEntry {
            key,
            entry_type,
            title: get("title"),
            author: get("author").replace(" and ", "; "),
            year,
            container,
            raw: source[start..end].to_string(),
        });
    }
    entries
}

/// Hängt neue Einträge an, deren Schlüssel noch nicht existieren.
/// Liefert (neuer Inhalt, hinzugefügte Schlüssel, bereits vorhandene Schlüssel).
pub fn merge(existing: &str, additions: &str) -> (String, Vec<String>, Vec<String>) {
    let known: Vec<String> = parse(existing).into_iter().map(|entry| entry.key).collect();
    let mut result = existing.trim_end().to_string();
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    for entry in parse(additions) {
        if known.contains(&entry.key) || added.contains(&entry.key) {
            skipped.push(entry.key);
            continue;
        }
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(entry.raw.trim());
        added.push(entry.key);
    }
    result.push('\n');
    (result, added, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
% Kommentar
@string{aw = "Addison-Wesley"}
@book{knuth1984,
  author = {Donald E. Knuth and Duane Bibby},
  title = {The {\TeX}book},
  publisher = aw,
  year = 1984
}
@article{m{\"u}ller_2020, title="Schutz in {\"O}sterreich", journal = {E \& I}, date = {2020-05-01}, author = {M{\"u}ller, Anna}}
"#;

    #[test]
    fn parses_entries_with_braces_quotes_and_macros() {
        let entries = parse(SAMPLE);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "knuth1984");
        assert_eq!(entries[0].title, "The TeXbook");
        assert_eq!(entries[0].author, "Donald E. Knuth; Duane Bibby");
        assert_eq!(entries[0].year, "1984");
        assert_eq!(entries[1].title, "Schutz in Österreich");
        assert_eq!(entries[1].year, "2020");
        assert_eq!(entries[1].container, "E & I");
        assert_eq!(entries[1].author, "Müller, Anna");
    }

    #[test]
    fn merge_skips_existing_keys() {
        let (merged, added, skipped) = merge(
            SAMPLE,
            "@misc{knuth1984, title={x}}\n@misc{neu, title={Neu}}",
        );
        assert_eq!(added, vec!["neu"]);
        assert_eq!(skipped, vec!["knuth1984"]);
        assert!(merged.ends_with("@misc{neu, title={Neu}}\n"));
    }
}
