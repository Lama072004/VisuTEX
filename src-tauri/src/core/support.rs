//! Eigene TeX-Dateien von VisuTeX (nicht im TeX-Bundle): werden beim Kompilieren über einen
//! Suchpfad bereitgestellt und beim LaTeX-Export neben das Dokument geschrieben, damit das
//! exportierte Dokument auch mit TeX Live, MiKTeX und Overleaf kompiliert.

/// Deutsche Fassung von `IEEEtranN.bst` (Literaturstil „IEEE (deutsch)“).
pub const IEEE_DE_BST: &str = "visutex-ieee-de";

const FILES: &[(&str, &str, &str)] = &[(
    IEEE_DE_BST,
    "visutex-ieee-de.bst",
    include_str!("../../resources/tex/visutex-ieee-de.bst"),
)];

/// Dateien (Name, Inhalt), die `latex` braucht – erkannt am `\bibliographystyle{…}`.
pub fn files_for(latex: &str) -> Vec<(&'static str, &'static str)> {
    FILES
        .iter()
        .filter(|(style, _, _)| uses_bibliography_style(latex, style))
        .map(|(_, name, content)| (*name, *content))
        .collect()
}

fn uses_bibliography_style(latex: &str, style: &str) -> bool {
    latex
        .match_indices("\\bibliographystyle")
        .any(|(index, command)| {
            let rest = latex[index + command.len()..].trim_start();
            rest.strip_prefix('{')
                .and_then(|rest| rest.split_once('}'))
                .is_some_and(|(name, _)| name.trim() == style)
        })
}

/// Schreibt die benötigten Dateien nach `dir`, sofern sie fehlen (bzw. mit `overwrite`, sofern
/// sie abweichen – nicht im Projektordner, dort könnte der Nutzer die Datei angepasst haben);
/// liefert die Namen der geschriebenen bzw. vorhandenen Dateien.
pub fn write_files(
    latex: &str,
    dir: &std::path::Path,
    overwrite: bool,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for (name, content) in files_for(latex) {
        let path = dir.join(name);
        let current = std::fs::read_to_string(&path).ok();
        if current.is_none() || (overwrite && current.as_deref() != Some(content)) {
            std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
            std::fs::write(&path, content)
                .map_err(|error| format!("„{name}“ konnte nicht geschrieben werden: {error}"))?;
        }
        written.push(name.to_string());
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_german_ieee_style() {
        assert_eq!(
            files_for("\\bibliographystyle{visutex-ieee-de}\n\\bibliography{literatur}").len(),
            1
        );
        assert_eq!(
            files_for("\\bibliographystyle { visutex-ieee-de }").len(),
            1
        );
        assert!(files_for("\\bibliographystyle{IEEEtranN}").is_empty());
        let (_, content) = files_for("\\bibliographystyle{visutex-ieee-de}")[0];
        assert!(content.contains("FUNCTION {bbl.and}{ \"und\" }"));
        assert!(content.contains("MODIFIED FILE"));
    }

    #[test]
    fn german_ieee_style_maps_both_ways() {
        use crate::core::preamble::{bibliography_style, ieee_style_for_language, style_for_bst};
        assert_eq!(bibliography_style("ieee-de").0, IEEE_DE_BST);
        assert_eq!(style_for_bst(IEEE_DE_BST), Some("ieee-de"));
        // fremde Dokumente mit IEEEtranN bleiben bei IEEEtranN (verlustfreier Round-Trip)
        assert_eq!(style_for_bst("IEEEtranN"), Some("ieee"));
        assert_eq!(ieee_style_for_language("ngerman"), "ieee-de");
        assert_eq!(ieee_style_for_language("english"), "ieee");
    }

    #[test]
    fn export_keeps_files_edited_by_the_user() {
        let dir = std::env::temp_dir().join(format!("visutex-support-{}", std::process::id()));
        let latex = "\\bibliographystyle{visutex-ieee-de}";
        write_files(latex, &dir, false).unwrap();
        let path = dir.join("visutex-ieee-de.bst");
        std::fs::write(&path, "% angepasst").unwrap();
        write_files(latex, &dir, false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "% angepasst");
        write_files(latex, &dir, true).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("bbl.and"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
