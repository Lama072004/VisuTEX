//! Abgabe-Check: findet vor der Abgabe typische Probleme im Dokument – Querverweise ins Leere,
//! Zitate ohne Literatureintrag, Abbildungen ohne Beschriftung, fehlende Bilddateien, leere
//! Abschnitte, Platzhalter, vergessene Kommentare und (nach dem Kompilieren) Text, der über den
//! Rand ragt. Jeder Fund nennt den obersten Block, damit die Oberfläche dorthin springen kann.
//!
//! Funde sind Codes mit Detail (Label, Schlüssel, Titel …); die Oberfläche übersetzt sie.

use super::bibtex;
use super::export::{attr_str, children, node_type, text_content};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// error | warning | info
    pub severity: &'static str,
    /// z. B. `reference-unknown`, `citation-unknown`, `figure-no-caption` …
    pub code: &'static str,
    /// Label, Schlüssel, Titel, Ausschnitt … (leer, wenn nicht nötig)
    pub detail: String,
    /// Oberster Block (zum Hinspringen)
    pub block: Option<usize>,
}

/// Zu breite Zeile bzw. zu volle Seite aus dem TeX-Log.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutIssue {
    /// Zeile im kompilierten Dokument (falls im Log angegeben)
    pub line: Option<u32>,
    /// Überstand in pt
    pub amount: f32,
    /// hbox (Zeile zu breit) | vbox (Seite zu voll)
    pub kind: String,
}

/// Ergebnis des letzten Kompilierens.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompileInfo {
    pub errors: usize,
    pub layout_issues: Vec<LayoutIssue>,
    /// Zeilen je oberstem Block (Export), für die Zuordnung der Log-Zeilen
    pub block_lines: Vec<[u32; 2]>,
}

pub struct ReviewInput<'a> {
    pub document: &'a Value,
    /// Inhalt der Literaturdatei (`None`: keine gewählt bzw. nicht lesbar)
    pub bibliography: Option<&'a str>,
    /// Gibt es diese Bilddatei (Pfad relativ zum Projektordner, ggf. ohne Endung)?
    pub file_exists: &'a dyn Fn(&str) -> bool,
    /// `None`: seit dem Öffnen nicht kompiliert
    pub compile: Option<&'a CompileInfo>,
}

/// Zitierbefehle (natbib, biblatex), deren Schlüssel geprüft werden.
const CITE_COMMANDS: &[&str] = &[
    "cite",
    "citep",
    "citet",
    "citealt",
    "citealp",
    "citeauthor",
    "citeyear",
    "citeyearpar",
    "Citep",
    "Citet",
    "parencite",
    "textcite",
    "autocite",
    "footcite",
    "Parencite",
    "Textcite",
    "Autocite",
];

/// Überstände unter dieser Grenze (pt) sind unsichtbar und werden ignoriert.
const OVERFULL_LIMIT: f32 = 1.0;

/// Überlange Zeilen und zu volle Seiten aus einem TeX-Log.
pub fn layout_issues(log: &str) -> Vec<LayoutIssue> {
    let mut issues = Vec::new();
    for line in log.lines() {
        let kind = if line.starts_with("Overfull \\hbox (") {
            "hbox"
        } else if line.starts_with("Overfull \\vbox (") {
            "vbox"
        } else {
            continue;
        };
        let amount = line
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once("pt"))
            .and_then(|(number, _)| number.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        if amount < OVERFULL_LIMIT {
            continue;
        }
        // „in paragraph at lines 31--33“ bzw. „detected at line 45“
        let source_line = line
            .rsplit_once("at lines ")
            .or_else(|| line.rsplit_once("at line "))
            .and_then(|(_, rest)| {
                rest.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .ok()
            });
        issues.push(LayoutIssue {
            line: source_line,
            amount,
            kind: kind.into(),
        });
    }
    issues
}

/// Block zu einer Zeile: der letzte Block, der an oder vor ihr beginnt (wie `syncLines.ts`).
fn block_for_line(block_lines: &[[u32; 2]], line: u32) -> Option<usize> {
    let mut found = None;
    for (index, range) in block_lines.iter().enumerate() {
        if range[0] > 0 && range[0] <= line {
            found = Some(index);
        }
    }
    found.or_else(|| block_lines.iter().position(|range| range[0] > 0))
}

/// Alle Knoten unterhalb von `node` (einschließlich), jeweils mit dem obersten Block.
fn visit<'a>(node: &'a Value, block: usize, f: &mut dyn FnMut(&'a Value, usize)) {
    f(node, block);
    for child in children(node) {
        visit(child, block, f);
    }
}

/// Argumente von `\command{…}` (genau die genannten Befehle) in Roh-LaTeX – optionale
/// `[…]`-Argumente und Sternform werden übersprungen, Listen (`a,b`) aufgeteilt.
fn command_arguments(source: &str, names: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    while let Some(found) = source[index..].find('\\') {
        let start = index + found + 1;
        let name_end = source[start..]
            .find(|c: char| !c.is_ascii_alphabetic())
            .map_or(source.len(), |offset| start + offset);
        let name = &source[start..name_end];
        index = name_end.max(start);
        if !names.contains(&name) {
            continue;
        }
        let mut position = name_end;
        if bytes.get(position) == Some(&b'*') {
            position += 1;
        }
        // optionale Argumente überspringen
        while bytes.get(position) == Some(&b'[') {
            match source[position..].find(']') {
                Some(end) => position += end + 1,
                None => break,
            }
        }
        if bytes.get(position) != Some(&b'{') {
            continue;
        }
        if let Some(end) = source[position..].find('}') {
            for value in source[position + 1..position + end].split(',') {
                let value = value.trim();
                if !value.is_empty() {
                    result.push(value.to_string());
                }
            }
            index = position + end;
        }
    }
    result
}

fn shorten(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        text
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// Platzhalter oder Notiz im Text: `[Titel]`, `TODO`, `FIXME`, `XXX`, „Lorem ipsum“.
fn placeholder(text: &str) -> Option<String> {
    for word in ["TODO", "FIXME", "XXX"] {
        if let Some(index) = text.find(word) {
            let before = text[..index].chars().next_back();
            let after = text[index + word.len()..].chars().next();
            let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
            if boundary(before) && boundary(after) {
                return Some(shorten(&text[index..], 40));
            }
        }
    }
    if let Some(index) = text.to_lowercase().find("lorem ipsum") {
        return Some(shorten(&text[index.min(text.len())..], 40));
    }
    // Vorlagen-Platzhalter: [Großbuchstabe …] ohne Ziffern, z. B. „[Titel der Arbeit]“
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            break;
        };
        let inner = &after[..close];
        let starts_upper = inner.chars().next().is_some_and(char::is_uppercase);
        if starts_upper
            && (2..=40).contains(&inner.chars().count())
            && !inner.chars().any(|c| c.is_ascii_digit() || c == '[')
        {
            return Some(format!("[{inner}]"));
        }
        rest = &after[close..];
    }
    None
}

fn is_comment_only(latex: &str) -> bool {
    let lines: Vec<&str> = latex
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    !lines.is_empty() && lines.iter().all(|line| line.starts_with('%'))
}

/// Block mit sichtbarem Inhalt (leere Absätze zählen nicht).
fn has_content(node: &Value) -> bool {
    !(node_type(node) == "paragraph"
        && text_content(node).trim().is_empty()
        && children(node).is_empty())
}

pub fn review(input: &ReviewInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut add = |severity, code, detail: String, block: Option<usize>| {
        findings.push(Finding {
            severity,
            code,
            detail,
            block,
        });
    };
    let blocks = children(input.document);

    // ---- sammeln
    let mut labels: Vec<(String, usize)> = Vec::new();
    let mut references: Vec<(String, usize)> = Vec::new();
    let mut raw_references: BTreeSet<String> = BTreeSet::new();
    let mut citations: Vec<(String, usize)> = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        visit(block, block_index, &mut |node, block| {
            let kind = node_type(node);
            let label = attr_str(node, "label");
            let labelled = match kind {
                "heading" | "image" | "table" | "tikzBlock" | "subfigures" => true,
                "mathBlock" => node
                    .get("attrs")
                    .and_then(|attrs| attrs.get("numbered"))
                    .is_some_and(|value| value == &Value::Bool(true) || value == "true"),
                _ => false,
            };
            if labelled && !label.trim().is_empty() {
                labels.push((label.trim().to_string(), block));
            }
            if kind == "subfigures" {
                for item in node
                    .get("attrs")
                    .and_then(|attrs| attrs.get("items"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(label) = item.get("label").and_then(Value::as_str) {
                        if !label.trim().is_empty() {
                            labels.push((label.trim().to_string(), block));
                        }
                    }
                }
            }
            match kind {
                "crossReference" => {
                    let target = attr_str(node, "label");
                    if !target.trim().is_empty() {
                        references.push((target.trim().to_string(), block));
                    }
                }
                "citation" => {
                    for key in attr_str(node, "keys").split(',') {
                        if !key.trim().is_empty() {
                            citations.push((key.trim().to_string(), block));
                        }
                    }
                }
                "rawLatexBlock" | "rawLatexInline" => {
                    let source = attr_str(
                        node,
                        if kind == "rawLatexBlock" {
                            "rawLatex"
                        } else {
                            "latex"
                        },
                    );
                    for label in command_arguments(&source, &["label"]) {
                        labels.push((label, block));
                    }
                    raw_references.extend(command_arguments(
                        &source,
                        &[
                            "ref", "autoref", "eqref", "pageref", "cref", "Cref", "nameref", "vref",
                        ],
                    ));
                    for key in command_arguments(&source, CITE_COMMANDS) {
                        citations.push((key, block));
                    }
                }
                _ => {}
            }
        });
    }

    // ---- Querverweise und Labels
    let defined: BTreeSet<&str> = labels.iter().map(|(label, _)| label.as_str()).collect();
    let mut reported = BTreeSet::new();
    for (target, block) in &references {
        if !defined.contains(target.as_str()) && reported.insert(target.clone()) {
            add("error", "reference-unknown", target.clone(), Some(*block));
        }
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (label, block) in &labels {
        if seen.insert(label.as_str(), *block).is_some()
            && reported.insert(format!("doppelt:{label}"))
        {
            add("error", "label-duplicate", label.clone(), Some(*block));
        }
    }
    let used: BTreeSet<&str> = references
        .iter()
        .map(|(target, _)| target.as_str())
        .chain(raw_references.iter().map(String::as_str))
        .collect();
    let mut unused_reported = BTreeSet::new();
    for (label, block) in &labels {
        if !used.contains(label.as_str()) && unused_reported.insert(label.as_str()) {
            add("info", "label-unused", label.clone(), Some(*block));
        }
    }

    // ---- Literatur
    if let Some((_, first_block)) = citations.first() {
        match input.bibliography {
            None => add(
                "error",
                "citation-no-bibliography",
                String::new(),
                Some(*first_block),
            ),
            Some(source) => {
                let keys: BTreeSet<String> = bibtex::parse(source)
                    .into_iter()
                    .map(|entry| entry.key)
                    .collect();
                let mut missing = BTreeSet::new();
                for (key, block) in &citations {
                    if !keys.contains(key) && missing.insert(key.clone()) {
                        add("error", "citation-unknown", key.clone(), Some(*block));
                    }
                }
            }
        }
    }

    // ---- Abbildungen, Tabellen, Bilder, Platzhalter, Kommentare, Titelseite
    for (block_index, block) in blocks.iter().enumerate() {
        let mut placeholder_found = false;
        visit(block, block_index, &mut |node, block| {
            let caption = attr_str(node, "caption");
            match node_type(node) {
                "image" => {
                    let path = attr_str(node, "latexPath");
                    if caption.trim().is_empty() {
                        add("warning", "figure-no-caption", path.clone(), Some(block));
                    }
                    if !path.trim().is_empty() && !(input.file_exists)(path.trim()) {
                        add("error", "image-missing", path, Some(block));
                    }
                }
                "subfigures" => {
                    if caption.trim().is_empty() {
                        add("warning", "figure-no-caption", String::new(), Some(block));
                    }
                    for item in node
                        .get("attrs")
                        .and_then(|attrs| attrs.get("items"))
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let path = item
                            .get("latexPath")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .trim();
                        if !path.is_empty() && !(input.file_exists)(path) {
                            add("error", "image-missing", path.to_string(), Some(block));
                        }
                    }
                }
                "table" if caption.trim().is_empty() => {
                    add("warning", "table-no-caption", String::new(), Some(block));
                }
                "tikzBlock" if caption.trim().is_empty() => {
                    add("info", "drawing-no-caption", String::new(), Some(block));
                }
                "rawLatexBlock" if is_comment_only(&attr_str(node, "rawLatex")) => {
                    let text: String = attr_str(node, "rawLatex")
                        .lines()
                        .map(|line| line.trim().trim_start_matches('%').trim())
                        .filter(|line| {
                            !line.is_empty() && !line.chars().all(|c| "=-*#~_".contains(c))
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() {
                        add("info", "comment", shorten(&text, 60), Some(block));
                    }
                }
                "titlePage" => {
                    for (field, name) in [("title", "title"), ("author", "author")] {
                        if attr_str(node, field).trim().is_empty() {
                            add("warning", "title-empty", name.into(), Some(block));
                        }
                    }
                    for field in [
                        "title",
                        "subtitle",
                        "author",
                        "supervisor",
                        "institution",
                        "program",
                        "degree",
                        "placeDate",
                    ] {
                        if let Some(found) = placeholder(&attr_str(node, field)) {
                            add("warning", "placeholder", found, Some(block));
                            break;
                        }
                    }
                }
                "text" if !placeholder_found => {
                    if let Some(found) = node
                        .get("text")
                        .and_then(Value::as_str)
                        .and_then(placeholder)
                    {
                        placeholder_found = true;
                        add("warning", "placeholder", found, Some(block));
                    }
                }
                _ => {}
            }
        });
    }

    // ---- Gliederung: leere Abschnitte und übersprungene Ebenen
    let heading_level = |node: &Value| {
        (node_type(node) == "heading").then(|| {
            node.get("attrs")
                .and_then(|attrs| attrs.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1)
        })
    };
    let mut previous_level: Option<u64> = None;
    for (index, block) in blocks.iter().enumerate() {
        let Some(level) = heading_level(block) else {
            continue;
        };
        let title = shorten(&text_content(block), 60);
        if let Some(previous) = previous_level {
            if level > previous + 1 {
                add("warning", "heading-skip", title.clone(), Some(index));
            }
        }
        previous_level = Some(level);
        // bis zur nächsten Überschrift gleicher oder höherer Ebene muss Inhalt folgen
        let mut content = false;
        for next in &blocks[index + 1..] {
            match heading_level(next) {
                Some(next_level) if next_level <= level => break,
                Some(_) => {
                    content = true; // Unterabschnitt zählt als Inhalt
                    break;
                }
                None if has_content(next) => {
                    content = true;
                    break;
                }
                None => {}
            }
        }
        if !content {
            add("warning", "section-empty", title, Some(index));
        }
    }

    // ---- Ergebnis des letzten Kompilierens
    match input.compile {
        Some(compile) => {
            if compile.errors > 0 {
                add("error", "compile-errors", compile.errors.to_string(), None);
            }
            let mut worst: BTreeMap<Option<usize>, f32> = BTreeMap::new();
            for issue in compile
                .layout_issues
                .iter()
                .filter(|issue| issue.kind == "hbox")
            {
                let block = issue
                    .line
                    .and_then(|line| block_for_line(&compile.block_lines, line));
                let amount = worst.entry(block).or_insert(0.0);
                *amount = amount.max(issue.amount);
            }
            for (block, amount) in worst {
                add("warning", "overfull", format!("{amount:.1} pt"), block);
            }
            if compile
                .layout_issues
                .iter()
                .any(|issue| issue.kind == "vbox")
            {
                add("warning", "overfull-page", String::new(), None);
            }
        }
        None => add("info", "not-compiled", String::new(), None),
    }

    let rank = |severity: &str| match severity {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    };
    findings.sort_by_key(|finding| (rank(finding.severity), finding.block.unwrap_or(usize::MAX)));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn codes(findings: &[Finding]) -> Vec<(&'static str, String, Option<usize>)> {
        findings
            .iter()
            .map(|finding| (finding.code, finding.detail.clone(), finding.block))
            .collect()
    }

    fn paragraph(text: &str) -> Value {
        json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] })
    }

    #[test]
    fn finds_typical_problems_before_submission() {
        let doc = json!({ "type": "doc", "content": [
            { "type": "titlePage", "attrs": { "title": "[Titel der Arbeit]", "author": "" } },
            { "type": "heading", "attrs": { "level": 1, "label": "sec:einleitung" }, "content": [{ "type": "text", "text": "Einleitung" }] },
            { "type": "paragraph", "content": [
                { "type": "text", "text": "Siehe " },
                { "type": "crossReference", "attrs": { "label": "fig:fehlt", "kind": "ref" } },
                { "type": "text", "text": " und " },
                { "type": "citation", "attrs": { "keys": "knuth1984,unbekannt2020" } }
            ] },
            { "type": "image", "attrs": { "latexPath": "bilder/plan.png", "caption": "", "label": "fig:plan" } },
            { "type": "heading", "attrs": { "level": 3 }, "content": [{ "type": "text", "text": "Zu tief" }] },
            paragraph("TODO: Ergebnisse ergänzen"),
            { "type": "heading", "attrs": { "level": 1 }, "content": [{ "type": "text", "text": "Leer" }] },
            { "type": "paragraph" },
            { "type": "heading", "attrs": { "level": 1, "label": "sec:einleitung" }, "content": [{ "type": "text", "text": "Doppelt" }] },
            { "type": "rawLatexBlock", "attrs": { "rawLatex": "% Noch Quelle prüfen\n% ===" } },
            { "type": "table", "content": [] }
        ]});
        let bib =
            "@book{knuth1984, title = {The TeXbook}, author = {Knuth, Donald}, year = {1984}}";
        let exists = |path: &str| path != "bilder/plan.png";
        let compile = CompileInfo {
            errors: 2,
            layout_issues: layout_issues(
                "Overfull \\hbox (15.43pt too wide) in paragraph at lines 31--31\n\
                 Overfull \\hbox (0.4pt too wide) in paragraph at lines 40--40\n\
                 Overfull \\vbox (3.0pt too high) has occurred while \\output is active\n",
            ),
            block_lines: vec![
                [20, 22],
                [24, 24],
                [26, 26],
                [28, 31],
                [33, 33],
                [35, 35],
                [37, 37],
                [0, 0],
                [39, 39],
                [41, 42],
                [44, 50],
            ],
        };
        let findings = review(&ReviewInput {
            document: &doc,
            bibliography: Some(bib),
            file_exists: &exists,
            compile: Some(&compile),
        });
        let found = codes(&findings);
        let has = |code: &str, detail: &str, block: Option<usize>| {
            assert!(
                found
                    .iter()
                    .any(|(c, d, b)| *c == code && d == detail && *b == block),
                "{code}/{detail}/{block:?} fehlt in {found:#?}"
            );
        };
        has("reference-unknown", "fig:fehlt", Some(2));
        has("citation-unknown", "unbekannt2020", Some(2));
        has("label-duplicate", "sec:einleitung", Some(8));
        has("image-missing", "bilder/plan.png", Some(3));
        has("figure-no-caption", "bilder/plan.png", Some(3));
        has("table-no-caption", "", Some(10));
        has("heading-skip", "Zu tief", Some(4));
        has("section-empty", "Leer", Some(6));
        has("placeholder", "[Titel der Arbeit]", Some(0));
        has("placeholder", "TODO: Ergebnisse ergänzen", Some(5));
        has("title-empty", "author", Some(0));
        has("comment", "Noch Quelle prüfen", Some(9));
        has("label-unused", "fig:plan", Some(3));
        has("compile-errors", "2", None);
        // nur der sichtbare Überstand, dem Block zugeordnet (Zeile 31 → Bild, Block 3)
        has("overfull", "15.4 pt", Some(3));
        assert!(!found
            .iter()
            .any(|(c, d, _)| *c == "overfull" && d.starts_with("0.")));
        has("overfull-page", "", None);
        assert!(!found
            .iter()
            .any(|(c, d, _)| *c == "citation-unknown" && d == "knuth1984"));
        // Fehler zuerst, dann Warnungen, dann Hinweise
        let ranks: Vec<u8> = findings
            .iter()
            .map(|finding| match finding.severity {
                "error" => 0,
                "warning" => 1,
                _ => 2,
            })
            .collect();
        assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn clean_document_and_raw_latex_references() {
        let doc = json!({ "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 1, "label": "sec:a" }, "content": [{ "type": "text", "text": "A" }] },
            paragraph("Text mit [sic] und Abb. [1] – keine Platzhalter."),
            { "type": "rawLatexBlock", "attrs": { "rawLatex": "Wie in Abschnitt~\\ref{sec:a} und \\citep[S.~3]{knuth1984}.\n\\label{eq:raw}" } },
            { "type": "paragraph", "content": [{ "type": "crossReference", "attrs": { "label": "eq:raw" } }] }
        ]});
        let bib = "@book{knuth1984, title = {T}, year = {1984}}";
        let findings = review(&ReviewInput {
            document: &doc,
            bibliography: Some(bib),
            file_exists: &|_| true,
            compile: Some(&CompileInfo::default()),
        });
        assert!(findings.is_empty(), "{findings:#?}");
        // ohne Kompilieren: nur der Hinweis darauf
        let findings = review(&ReviewInput {
            document: &doc,
            bibliography: Some(bib),
            file_exists: &|_| true,
            compile: None,
        });
        assert_eq!(
            codes(&findings),
            vec![("not-compiled", String::new(), None)]
        );
        // Zitat ohne Literaturdatei
        let findings = review(&ReviewInput {
            document: &doc,
            bibliography: None,
            file_exists: &|_| true,
            compile: Some(&CompileInfo::default()),
        });
        assert_eq!(
            codes(&findings),
            vec![("citation-no-bibliography", String::new(), Some(2))]
        );
    }
}
