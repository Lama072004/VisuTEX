//! Eigene Makros aus einer Präambel (`\newcommand`, `\renewcommand`,
//! `\providecommand`, `\def`, `\DeclareMathOperator`, `\NewDocumentCommand`).
//!
//! Der Editor nutzt sie nur zur Anzeige: Formeln mit eigenen Makros werden in
//! KaTeX korrekt dargestellt, Roh-LaTeX wie `\REcv` zeigt seinen Inhalt.

use super::import::read_group;
use super::preamble::strip_comments;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MacroDef {
    /// Name ohne Backslash
    pub name: String,
    /// Anzahl der Argumente (inkl. optionalem)
    pub args: usize,
    /// Vorgabe des optionalen ersten Arguments
    pub default: Option<String>,
    pub body: String,
}

fn skip_spaces(text: &str, mut index: usize) -> usize {
    while text[index..].starts_with([' ', '\t', '\n', '\r']) {
        index += 1;
    }
    index
}

/// Name `\foo` bzw. `{\foo}` ab `index`; liefert Name und Ende.
fn macro_name(text: &str, index: usize) -> Option<(String, usize)> {
    let index = skip_spaces(text, index);
    let (inner, end) = if text[index..].starts_with('{') {
        let (inner, end) = read_group(text, index, b'{', b'}')?;
        (inner.trim(), end)
    } else {
        let rest = &text[index..];
        let letters = rest[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic() || *c == '@')
            .count();
        (&rest[..1 + letters], index + 1 + letters)
    };
    let name = inner.strip_prefix('\\')?;
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic() || c == '@'))
        .then(|| (name.to_string(), end))
}

pub fn collect(preamble: &str) -> Vec<MacroDef> {
    let text = strip_comments(preamble);
    let mut macros: Vec<MacroDef> = Vec::new();
    let mut push = |definition: MacroDef| {
        macros.retain(|existing| existing.name != definition.name);
        if definition.body.len() <= 2000 {
            macros.push(definition);
        }
    };
    let mut offset = 0;
    while let Some(found) = text[offset..].find('\\') {
        let start = offset + found;
        offset = start + 1;
        let rest = &text[start + 1..];
        let command: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let after = start + 1 + command.len();
        match command.as_str() {
            "newcommand" | "renewcommand" | "providecommand" => {
                let mut index = after;
                if text[index..].starts_with('*') {
                    index += 1;
                }
                let Some((name, name_end)) = macro_name(&text, index) else {
                    continue;
                };
                index = skip_spaces(&text, name_end);
                let mut args = 0;
                let mut default = None;
                if let Some((count, end)) = read_group(&text, index, b'[', b']') {
                    args = count.trim().parse().unwrap_or(0);
                    index = skip_spaces(&text, end);
                    if let Some((value, end)) = read_group(&text, index, b'[', b']') {
                        default = Some(value.to_string());
                        index = skip_spaces(&text, end);
                    }
                }
                if let Some((body, end)) = read_group(&text, index, b'{', b'}') {
                    push(MacroDef {
                        name,
                        args: args.min(9),
                        default,
                        body: body.trim().to_string(),
                    });
                    offset = end;
                }
            }
            "def" | "gdef" => {
                let Some((name, name_end)) = macro_name(&text, after) else {
                    continue;
                };
                // Nur einfache Definitionen ohne Parametertext
                let index = skip_spaces(&text, name_end);
                if let Some((body, end)) = read_group(&text, index, b'{', b'}') {
                    push(MacroDef {
                        name,
                        args: 0,
                        default: None,
                        body: body.trim().to_string(),
                    });
                    offset = end;
                }
            }
            "DeclareMathOperator" => {
                let mut index = after;
                let star = text[index..].starts_with('*');
                if star {
                    index += 1;
                }
                let Some((name, name_end)) = macro_name(&text, index) else {
                    continue;
                };
                let index = skip_spaces(&text, name_end);
                if let Some((body, end)) = read_group(&text, index, b'{', b'}') {
                    let operator = if star {
                        format!("\\operatorname*{{{}}}", body.trim())
                    } else {
                        format!("\\operatorname{{{}}}", body.trim())
                    };
                    push(MacroDef {
                        name,
                        args: 0,
                        default: None,
                        body: operator,
                    });
                    offset = end;
                }
            }
            "NewDocumentCommand" | "RenewDocumentCommand" | "ProvideDocumentCommand" => {
                let Some((name, name_end)) = macro_name(&text, after) else {
                    continue;
                };
                let index = skip_spaces(&text, name_end);
                let Some((spec, spec_end)) = read_group(&text, index, b'{', b'}') else {
                    continue;
                };
                // Nur m/o/O{…}-Argumente
                let mut args = 0;
                let mut default = None;
                let mut position = 0;
                let spec_bytes = spec.as_bytes();
                let mut supported = true;
                while position < spec_bytes.len() {
                    match spec_bytes[position] {
                        b' ' => {}
                        b'm' => args += 1,
                        b'o' if args == 0 => {
                            args += 1;
                            default = Some(String::new());
                        }
                        b'O' if args == 0 => {
                            args += 1;
                            if let Some((value, end)) = read_group(spec, position + 1, b'{', b'}') {
                                default = Some(value.to_string());
                                position = end - 1;
                            }
                        }
                        _ => supported = false,
                    }
                    position += 1;
                }
                let index = skip_spaces(&text, spec_end);
                if let Some((body, end)) = read_group(&text, index, b'{', b'}') {
                    if supported {
                        push(MacroDef {
                            name,
                            args: args.min(9),
                            default,
                            body: body.trim().to_string(),
                        });
                    }
                    offset = end;
                }
            }
            _ => {}
        }
    }
    macros
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_common_definitions() {
        let preamble = "\\newcommand{\\REcv}{3.04}\n\\newcommand\\teil[1]{\\textbf{#1)}~}\n\\renewcommand{\\vec}[1]{\\boldsymbol{#1}}\n\\newcommand{\\aufgabe}[2][Aufgabe]{\\section*{#1: #2}}\n\\def\\R{\\mathbb{R}}\n\\DeclareMathOperator{\\rot}{rot}\n\\NewDocumentCommand{\\erg}{m}{\\boxed{#1}}\n% \\newcommand{\\aus}{x}\n";
        let macros = collect(preamble);
        let find = |name: &str| macros.iter().find(|m| m.name == name).cloned();
        assert_eq!(find("REcv").map(|m| m.body), Some("3.04".into()));
        assert_eq!(find("teil").map(|m| m.args), Some(1));
        assert_eq!(
            find("aufgabe").and_then(|m| m.default),
            Some("Aufgabe".into())
        );
        assert_eq!(find("R").map(|m| m.body), Some("\\mathbb{R}".into()));
        assert_eq!(
            find("rot").map(|m| m.body),
            Some("\\operatorname{rot}".into())
        );
        assert_eq!(find("erg").map(|m| m.args), Some(1));
        assert!(find("aus").is_none(), "auskommentiert");
    }
}
