//! Dokumentanalyse für Dialoge und Statusleiste: Labels (Querverweise),
//! Gliederung (Navigationsbereich), verwendete Zitate/Abkürzungen, Wortzahl.

use super::export::{attr, attr_bool, attr_str, children, node_type, text_content};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LabelInfo {
    pub label: String,
    /// heading | figure | table | equation | drawing | raw
    pub kind: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutlineEntry {
    pub level: u64,
    pub title: String,
    /// Index des Blocks auf oberster Ebene (für die Navigation).
    pub block_index: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcronymInfo {
    pub key: String,
    pub short: String,
    pub long: String,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub labels: Vec<LabelInfo>,
    pub outline: Vec<OutlineEntry>,
    pub citation_keys: Vec<String>,
    pub acronyms: Vec<AcronymInfo>,
    pub words: usize,
    pub characters: usize,
    pub duplicate_labels: Vec<String>,
}

fn shorten(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        text
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// `\label{…}` in Roh-LaTeX.
fn raw_labels(source: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut rest = source;
    while let Some(index) = rest.find("\\label{") {
        let after = &rest[index + "\\label{".len()..];
        match after.find('}') {
            Some(end) => {
                let label = after[..end].trim();
                if !label.is_empty() {
                    labels.push(label.to_string());
                }
                rest = &after[end..];
            }
            None => break,
        }
    }
    labels
}

pub fn analyze(document: &Value) -> Analysis {
    let mut analysis = Analysis::default();
    for (index, block) in children(document).iter().enumerate() {
        walk(block, index, &mut analysis);
    }
    let text = plain_text(document);
    analysis.words = text.split_whitespace().count();
    analysis.characters = text.chars().filter(|c| !c.is_whitespace()).count();
    let mut seen = std::collections::BTreeSet::new();
    for label in &analysis.labels {
        if !seen.insert(label.label.clone()) && !analysis.duplicate_labels.contains(&label.label) {
            analysis.duplicate_labels.push(label.label.clone());
        }
    }
    analysis
}

fn push_label(analysis: &mut Analysis, label: String, kind: &str, title: String) {
    let label = label.trim().to_string();
    if !label.is_empty() {
        analysis.labels.push(LabelInfo {
            label,
            kind: kind.into(),
            title: shorten(&title, 80),
        });
    }
}

fn walk(node: &Value, block_index: usize, analysis: &mut Analysis) {
    match node_type(node) {
        "heading" => {
            let title = text_content(node);
            let level = node
                .get("attrs")
                .and_then(|attrs| attrs.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1);
            analysis.outline.push(OutlineEntry {
                level,
                title: shorten(&title, 120),
                block_index,
            });
            push_label(analysis, attr_str(node, "label"), "heading", title);
        }
        "image" => push_label(
            analysis,
            attr_str(node, "label"),
            "figure",
            attr_str(node, "caption"),
        ),
        "subfigures" => {
            push_label(
                analysis,
                attr_str(node, "label"),
                "figure",
                attr_str(node, "caption"),
            );
            for item in attr(node, "items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let text = |key: &str| {
                    item.get(key)
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                };
                push_label(analysis, text("label"), "figure", text("caption"));
            }
        }
        "table" => push_label(
            analysis,
            attr_str(node, "label"),
            "table",
            attr_str(node, "caption"),
        ),
        "mathBlock" => {
            if attr_bool(node, "numbered").unwrap_or(false) {
                push_label(
                    analysis,
                    attr_str(node, "label"),
                    "equation",
                    attr_str(node, "latex"),
                );
            }
        }
        "tikzBlock" => push_label(
            analysis,
            attr_str(node, "label"),
            "drawing",
            attr_str(node, "caption"),
        ),
        "rawLatexBlock" => {
            for label in raw_labels(&attr_str(node, "rawLatex")) {
                push_label(analysis, label, "raw", String::new());
            }
        }
        "citation" => {
            for key in attr_str(node, "keys").split(',') {
                let key = key.trim();
                if !key.is_empty() && !analysis.citation_keys.iter().any(|known| known == key) {
                    analysis.citation_keys.push(key.to_string());
                }
            }
        }
        "acronym" => {
            let key = attr_str(node, "key");
            if !key.is_empty() && !analysis.acronyms.iter().any(|known| known.key == key) {
                analysis.acronyms.push(AcronymInfo {
                    key,
                    short: attr_str(node, "short"),
                    long: attr_str(node, "long"),
                });
            }
        }
        _ => {}
    }
    for child in children(node) {
        walk(child, block_index, analysis);
    }
}

/// Sichtbarer Text (für Wortzählung), Blöcke durch Zeilenumbrüche getrennt.
pub fn plain_text(node: &Value) -> String {
    match node_type(node) {
        "text" => node
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        "acronym" => attr_str(node, "short"),
        "hardBreak" => "\n".into(),
        _ => {
            let parts: Vec<String> = children(node).iter().map(plain_text).collect();
            let inline = children(node)
                .first()
                .is_some_and(|child| matches!(node_type(child), "text" | "hardBreak"));
            parts.join(if inline { "" } else { "\n" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collects_labels_outline_and_counts() {
        let document = json!({ "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 1, "label": "chap:a" }, "content": [{ "type": "text", "text": "Einleitung" }] },
            { "type": "paragraph", "content": [
                { "type": "text", "text": "Zwei Wörter " },
                { "type": "citation", "attrs": { "keys": "a, b" } },
                { "type": "acronym", "attrs": { "key": "API", "short": "API", "long": "Schnittstelle" } }
            ]},
            { "type": "mathBlock", "attrs": { "latex": "x", "numbered": true, "label": "eq:x" } },
            { "type": "mathBlock", "attrs": { "latex": "y", "numbered": false, "label": "eq:y" } },
            { "type": "rawLatexBlock", "attrs": { "rawLatex": "\\begin{figure}\\label{fig:raw}\\end{figure}" } },
            { "type": "heading", "attrs": { "level": 2, "label": "chap:a" }, "content": [{ "type": "text", "text": "Doppelt" }] }
        ]});
        let analysis = analyze(&document);
        let labels: Vec<&str> = analysis.labels.iter().map(|l| l.label.as_str()).collect();
        assert_eq!(labels, vec!["chap:a", "eq:x", "fig:raw", "chap:a"]);
        assert_eq!(analysis.duplicate_labels, vec!["chap:a".to_string()]);
        assert_eq!(analysis.outline.len(), 2);
        assert_eq!(analysis.outline[1].block_index, 5);
        assert_eq!(
            analysis.citation_keys,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(analysis.acronyms[0].long, "Schnittstelle");
        assert_eq!(analysis.words, 5); // Einleitung, Zwei, Wörter, API, Doppelt
    }
}
