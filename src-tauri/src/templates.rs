//! Mitgelieferte Dokumentvorlagen für den Startbildschirm.

use crate::core::project::Project;
use crate::core::settings::DocumentSettings;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// builtin | addon:<id>
    pub source: String,
}

pub fn list() -> Vec<TemplateInfo> {
    [
        ("leer-bericht", "Leerer Bericht", "KOMA-Bericht (scrreprt) mit Kapiteln, A4"),
        ("leer-artikel", "Leerer Artikel", "Artikel (article) ohne Kapitel, A4"),
        (
            "abschlussarbeit",
            "Abschlussarbeit",
            "Bachelor-/Masterarbeit: Titelseite, Sperrvermerk, Kurzreferat/Abstract, Verzeichnisse, Erklärung",
        ),
        ("projektbericht-englisch", "Project Report", "Englischsprachiger Projektbericht mit Abstract und Gliederung"),
    ]
    .into_iter()
    .map(|(id, name, description)| TemplateInfo {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        source: "builtin".into(),
    })
    .collect()
}

fn text(content: &str) -> Value {
    json!({ "type": "text", "text": content })
}

fn paragraph(content: &str) -> Value {
    if content.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": [text(content)] })
    }
}

fn heading(level: u8, content: &str) -> Value {
    json!({ "type": "heading", "attrs": { "level": level }, "content": [text(content)] })
}

fn frontmatter(kind: &str, title: &str, in_toc: bool, paragraphs: &[&str]) -> Value {
    json!({
        "type": "frontmatterBlock",
        "attrs": { "kind": kind, "title": title, "inToc": in_toc },
        "content": paragraphs.iter().map(|content| paragraph(content)).collect::<Vec<_>>(),
    })
}

fn directory(kind: &str) -> Value {
    json!({ "type": "directoryBlock", "attrs": { "kind": kind } })
}

pub fn build(id: &str) -> Result<Project, String> {
    let mut settings = DocumentSettings::default();
    let content: Vec<Value> = match id {
        "leer-bericht" => vec![heading(1, "Einleitung"), paragraph("")],
        "leer-artikel" => {
            settings.document_class = "article".into();
            vec![heading(1, "Einleitung"), paragraph("")]
        }
        // Ältere Kennung weiterhin akzeptieren
        "abschlussarbeit" | "fhv-abschlussarbeit" => {
            settings.twoside = true;
            settings.binding_offset = 8.0;
            settings.margins.left = 25.0;
            settings.margins.right = 25.0;
            settings.bibliography.style = "ieee".into();
            settings.bibliography.file = "literatur.bib".into();
            settings.header_footer.enabled = false;
            vec![
                json!({ "type": "titlePage", "attrs": {
                    "title": "[Titel der Arbeit]",
                    "subtitle": "[evtl. Untertitel]",
                    "thesisType": "Bachelorarbeit",
                    "degreeIntro": "zur Erlangung des akademischen Grades",
                    "degree": "Bachelor of Science in Engineering (BSc)",
                    "institution": "[Hochschule]",
                    "program": "[Studiengang]",
                    "authorLabel": "Vorgelegt von",
                    "author": "[Vor- und Nachname]",
                    "placeDate": "[Ort], [Monat Jahr]",
                    "supervisorLabel": "Betreut von",
                    "supervisor": "[Titel Vor- und Nachname]",
                    "logoPath": ""
                }}),
                frontmatter(
                    "confidentiality",
                    "Sperrvermerk",
                    false,
                    &["[Nur falls nötig – Text gemäß den Vorgaben Ihrer Hochschule, z. B.: Diese Arbeit enthält vertrauliche Informationen und darf bis [Datum] nur mit Zustimmung der Verfasserin bzw. des Verfassers eingesehen oder weitergegeben werden.]"],
                ),
                frontmatter("dedication", "Widmung", false, &["[evtl. Widmung]"]),
                frontmatter("summary", "Kurzreferat", false, &["[Kurzreferat auf Deutsch, ca. eine halbe Seite]"]),
                frontmatter("abstract", "Abstract", false, &["[Abstract in English, about half a page]"]),
                frontmatter("preface", "Vorwort", false, &["[evtl. Vorwort]"]),
                directory("contents"),
                directory("figures"),
                directory("tables"),
                directory("acronyms"),
                heading(1, "Einleitung"),
                paragraph("[Ausgangslage, Problemstellung, Zielsetzung und Aufbau der Arbeit]"),
                heading(1, "Grundlagen"),
                heading(2, "[Unterkapitel zweite Ebene]"),
                paragraph(""),
                heading(1, "Zusammenfassung und Ausblick"),
                paragraph(""),
                directory("bibliography"),
                frontmatter("appendix", "Anhang", true, &["[evtl. Anhang]"]),
                frontmatter(
                    "declaration",
                    "Eidesstattliche Erklärung",
                    true,
                    &[
                        "[Wortlaut gemäß den Vorgaben Ihrer Hochschule einfügen, z. B.: Ich versichere, diese Arbeit selbst verfasst und keine anderen als die angegebenen Quellen und Hilfsmittel verwendet zu haben.]",
                        "[Ort], am [Datum]                [Vor- und Nachname]",
                    ],
                ),
            ]
        }
        "projektbericht-englisch" | "jamk-bericht" => {
            settings.language = "english".into();
            settings.header_footer.enabled = true;
            settings.metadata.title = "Project Report".into();
            vec![
                json!({ "type": "titlePage", "attrs": {
                    "title": "Project Report",
                    "subtitle": "[Subtitle]",
                    "thesisType": "Project report",
                    "institution": "[University]",
                    "program": "[School / Programme]",
                    "authorLabel": "Author",
                    "author": "[Student Name]",
                    "placeDate": "[City], [Month Year]",
                    "supervisorLabel": "Course",
                    "supervisor": "[Course name]"
                }}),
                frontmatter(
                    "abstract",
                    "Abstract",
                    false,
                    &["[Background, objectives, methods and main results in brief.]"],
                ),
                directory("contents"),
                heading(1, "Introduction"),
                paragraph("[Context of the project, research problem, scope and key terminology.]"),
                heading(1, "Objectives"),
                json!({ "type": "bulletList", "content": [
                    { "type": "listItem", "content": [paragraph("Define a clear research question and its practical relevance.")] },
                    { "type": "listItem", "content": [paragraph("Describe the methods used to collect and analyse evidence.")] },
                    { "type": "listItem", "content": [paragraph("Present findings in a reproducible and accessible format.")] }
                ]}),
                heading(1, "Methodology"),
                paragraph(""),
                heading(1, "Results and Discussion"),
                paragraph(""),
                heading(1, "Conclusion"),
                paragraph(""),
            ]
        }
        other => return Err(format!("Unbekannte Vorlage „{other}“.")),
    };
    Ok(Project::new(
        settings,
        json!({ "type": "doc", "content": content }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::export::{export_document, ExportOptions};

    #[test]
    fn all_templates_export_without_warnings() {
        for template in list() {
            let project = build(&template.id).unwrap();
            let exported = export_document(
                &project.document,
                &ExportOptions {
                    settings: &project.settings,
                    custom_preamble: None,
                    addon_preamble: &[],
                },
            );
            assert!(
                exported.latex.contains("\\begin{document}"),
                "{}",
                template.id
            );
            assert!(
                exported.warnings.is_empty(),
                "{}: {:?}",
                template.id,
                exported.warnings
            );
        }
    }
}
