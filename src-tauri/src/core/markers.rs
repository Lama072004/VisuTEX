//! Kommentar-Marker für strukturierte VisuTeX-Bereiche im LaTeX-Code.
//!
//! Marker sind gewöhnliche LaTeX-Kommentare und werden von jedem LaTeX-System
//! ignoriert. Sie erlauben den verlustfreien Rückweg in die visuelle Ansicht
//! für Elemente, deren LaTeX-Form nicht eindeutig ist:
//!
//! ```text
//! %% VisuTeX-begin: frontmatter {"kind":"abstract","title":"Abstract"}
//! …
//! %% VisuTeX-end: frontmatter
//! ```

use serde_json::{Map, Value};

pub const PREFIX_BEGIN: &str = "%% VisuTeX-begin: ";
pub const PREFIX_END: &str = "%% VisuTeX-end: ";

pub fn wrap(kind: &str, data: &Value, content: &str) -> String {
    let data = if data.is_object() {
        data.clone()
    } else {
        Value::Object(Map::new())
    };
    format!(
        "{PREFIX_BEGIN}{kind} {}\n{}\n{PREFIX_END}{kind}",
        serde_json::to_string(&data).unwrap_or_else(|_| "{}".into()),
        content.trim_end_matches('\n')
    )
}

#[derive(Debug, Clone)]
pub struct Region {
    pub kind: String,
    pub data: Value,
    /// Inhalt zwischen den Markerzeilen.
    pub inner: String,
    /// Byte-Bereich inklusive Markerzeilen.
    pub start: usize,
    pub end: usize,
}

struct Line<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

fn lines(source: &str) -> Vec<Line<'_>> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, character) in source.char_indices() {
        if character == '\n' {
            result.push(Line {
                start,
                end: index + 1,
                text: &source[start..index],
            });
            start = index + 1;
        }
    }
    if start < source.len() {
        result.push(Line {
            start,
            end: source.len(),
            text: &source[start..],
        });
    }
    result
}

fn parse_begin(line: &str) -> Option<(String, Value)> {
    let rest = line
        .trim()
        .strip_prefix(PREFIX_BEGIN.trim_end())?
        .trim_start();
    let (kind, json) = match rest.find(' ') {
        Some(index) => (&rest[..index], rest[index..].trim()),
        None => (rest, ""),
    };
    if kind.is_empty() || !kind.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        return None;
    }
    let data = if json.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(json).ok()?
    };
    Some((kind.to_string(), data))
}

fn parse_end(line: &str) -> Option<&str> {
    let kind = line.trim().strip_prefix(PREFIX_END.trim_end())?.trim();
    (!kind.is_empty()).then_some(kind)
}

/// Findet Markerbereiche auf oberster Ebene (verschachtelte gleichen Typs werden gezählt).
pub fn find_regions(source: &str) -> Vec<Region> {
    let all = lines(source);
    let mut regions = Vec::new();
    let mut index = 0;
    while index < all.len() {
        let Some((kind, data)) = parse_begin(all[index].text) else {
            index += 1;
            continue;
        };
        let mut depth = 1;
        let mut end_line = None;
        for (offset, line) in all.iter().enumerate().skip(index + 1) {
            if parse_begin(line.text).is_some_and(|(other, _)| other == kind) {
                depth += 1;
            } else if parse_end(line.text) == Some(kind.as_str()) {
                depth -= 1;
                if depth == 0 {
                    end_line = Some(offset);
                    break;
                }
            }
        }
        let Some(end_line) = end_line else {
            index += 1;
            continue;
        };
        let inner_start = all[index].end;
        let inner_end = all[end_line].start;
        let inner = source[inner_start.min(inner_end)..inner_end]
            .trim_end_matches('\n')
            .to_string();
        regions.push(Region {
            kind,
            data,
            inner,
            start: all[index].start,
            end: all[end_line].end,
        });
        index = end_line + 1;
    }
    regions
}

/// Whitespace-normalisierter Vergleich für „unverändert generiert?“-Prüfungen.
pub fn same_latex(left: &str, right: &str) -> bool {
    let normalize = |value: &str| value.split_whitespace().collect::<Vec<_>>().join(" ");
    normalize(left) == normalize(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_nested_and_ignores_broken_regions() {
        let source = format!(
            "a\n{}\nb\n{PREFIX_BEGIN}x {{\"k\":1}}\nbroken\n",
            wrap(
                "frontmatter",
                &json!({"kind": "abstract"}),
                "inner\n%% VisuTeX-begin: frontmatter {}\nx\n%% VisuTeX-end: frontmatter"
            )
        );
        let regions = find_regions(&source);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].kind, "frontmatter");
        assert_eq!(regions[0].data["kind"], "abstract");
        assert!(regions[0].inner.contains("%% VisuTeX-end: frontmatter"));
        assert_eq!(
            &source[regions[0].end..],
            "b\n%% VisuTeX-begin: x {\"k\":1}\nbroken\n"
        );
    }
}
