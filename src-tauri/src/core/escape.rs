//! Maskierung von Fließtext für LaTeX.
//!
//! Der erzeugte Code muss mit pdfLaTeX (T1 + utf8), XeLaTeX, LuaLaTeX und der
//! eingebauten Tectonic-Engine identisch kompilieren. Deshalb werden
//!  - alle LaTeX-Sonderzeichen maskiert (auch `\`, `^`, `~` und `"`, das mit
//!    babel/ngerman ein aktives Kurzbefehl-Zeichen ist),
//!  - mathematische Unicode-Symbole und griechische Buchstaben, die in Latin
//!    Modern bzw. unter pdfLaTeX fehlen, als `\ensuremath{…}` ausgegeben.

/// Unicode-Zeichen → portabler Math-Befehl.
pub const UNICODE_SYMBOLS: &[(char, &str)] = &[
    ('≤', "\\leq"),
    ('≥', "\\geq"),
    ('≠', "\\neq"),
    ('≈', "\\approx"),
    ('≡', "\\equiv"),
    ('±', "\\pm"),
    ('∓', "\\mp"),
    ('×', "\\times"),
    ('÷', "\\div"),
    ('⋅', "\\cdot"),
    ('→', "\\rightarrow"),
    ('←', "\\leftarrow"),
    ('↔', "\\leftrightarrow"),
    ('↑', "\\uparrow"),
    ('↓', "\\downarrow"),
    ('⇒', "\\Rightarrow"),
    ('⇐', "\\Leftarrow"),
    ('⇔', "\\Leftrightarrow"),
    ('∞', "\\infty"),
    ('∑', "\\sum"),
    ('∏', "\\prod"),
    ('√', "\\surd"),
    ('∫', "\\int"),
    ('∂', "\\partial"),
    ('∇', "\\nabla"),
    ('∈', "\\in"),
    ('∉', "\\notin"),
    ('⊂', "\\subset"),
    ('⊆', "\\subseteq"),
    ('⊃', "\\supset"),
    ('⊇', "\\supseteq"),
    ('∪', "\\cup"),
    ('∩', "\\cap"),
    ('∀', "\\forall"),
    ('∃', "\\exists"),
    ('∅', "\\emptyset"),
    ('∧', "\\wedge"),
    ('∨', "\\vee"),
    ('¬', "\\neg"),
    ('∝', "\\propto"),
    ('∠', "\\angle"),
    ('α', "\\alpha"),
    ('β', "\\beta"),
    ('γ', "\\gamma"),
    ('δ', "\\delta"),
    ('ε', "\\varepsilon"),
    ('ζ', "\\zeta"),
    ('η', "\\eta"),
    ('θ', "\\theta"),
    ('ι', "\\iota"),
    ('κ', "\\kappa"),
    ('λ', "\\lambda"),
    ('μ', "\\mu"),
    ('ν', "\\nu"),
    ('ξ', "\\xi"),
    ('π', "\\pi"),
    ('ρ', "\\rho"),
    ('σ', "\\sigma"),
    ('τ', "\\tau"),
    ('υ', "\\upsilon"),
    ('φ', "\\varphi"),
    ('χ', "\\chi"),
    ('ψ', "\\psi"),
    ('ω', "\\omega"),
    ('Γ', "\\Gamma"),
    ('Δ', "\\Delta"),
    ('∆', "\\Delta"),
    ('Θ', "\\Theta"),
    ('Λ', "\\Lambda"),
    ('Ξ', "\\Xi"),
    ('Π', "\\Pi"),
    ('Σ', "\\Sigma"),
    ('Φ', "\\Phi"),
    ('Ψ', "\\Psi"),
    ('Ω', "\\Omega"),
];

/// Textsymbol-Befehle, die beim Import wieder zu Unicode werden.
pub const TEXT_SYMBOL_COMMANDS: &[(&str, &str)] = &[
    ("textbackslash", "\\"),
    ("textasciicircum", "^"),
    ("textasciitilde", "~"),
    ("textquotedbl", "\""),
    ("textbar", "|"),
    ("textless", "<"),
    ("textgreater", ">"),
    ("textunderscore", "_"),
    ("textdollar", "$"),
    ("textbraceleft", "{"),
    ("textbraceright", "}"),
    ("textendash", "–"),
    ("textemdash", "—"),
    ("textellipsis", "…"),
    ("dots", "…"),
    ("ldots", "…"),
    ("textbullet", "•"),
    ("textdegree", "°"),
    ("textregistered", "®"),
    ("texttrademark", "™"),
    ("textcopyright", "©"),
    ("copyright", "©"),
    ("S", "§"),
    ("P", "¶"),
    ("euro", "€"),
    ("texteuro", "€"),
    ("glqq", "„"),
    ("grqq", "“"),
    ("glq", "‚"),
    ("grq", "‘"),
    ("quotedblbase", "„"),
    ("textquotedblleft", "“"),
    ("textquotedblright", "”"),
    ("guillemotleft", "«"),
    ("guillemotright", "»"),
    ("textperthousand", "‰"),
    ("textmu", "µ"),
];

pub fn text_symbol(command: &str) -> Option<&'static str> {
    TEXT_SYMBOL_COMMANDS
        .iter()
        .find(|(name, _)| *name == command)
        .map(|(_, value)| *value)
}

pub fn math_command_for(character: char) -> Option<&'static str> {
    UNICODE_SYMBOLS
        .iter()
        .find(|(symbol, _)| *symbol == character)
        .map(|(_, command)| *command)
}

/// `\alpha` → „α“ (für `\ensuremath{\alpha}` bzw. `$\alpha$`), sonst None.
pub fn unicode_for_math_command(command: &str) -> Option<char> {
    let command = command.trim();
    // ∆ (U+2206) und Δ (U+0394) teilen sich \Delta – beim Rückweg gewinnt das griechische Δ.
    UNICODE_SYMBOLS
        .iter()
        .filter(|(symbol, _)| *symbol != '∆')
        .find(|(_, name)| *name == command)
        .map(|(symbol, _)| *symbol)
}

/// Maskiert beliebigen Text für den Fließtext.
pub fn escape_latex(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 8);
    let mut previous_space = false;
    for character in text.chars() {
        let character = if matches!(character, '\t' | '\r' | '\n') {
            ' '
        } else {
            character
        };
        if character == ' ' {
            if previous_space {
                continue;
            }
            previous_space = true;
        } else {
            previous_space = false;
        }
        match character {
            '\\' => result.push_str("\\textbackslash{}"),
            '{' => result.push_str("\\{"),
            '}' => result.push_str("\\}"),
            '$' => result.push_str("\\$"),
            '&' => result.push_str("\\&"),
            '#' => result.push_str("\\#"),
            '%' => result.push_str("\\%"),
            '_' => result.push_str("\\_"),
            '^' => result.push_str("\\textasciicircum{}"),
            '~' => result.push_str("\\textasciitilde{}"),
            '"' => result.push_str("\\textquotedbl{}"),
            '\u{00a0}' => result.push('~'),
            '\u{202f}' => result.push_str("\\,"),
            '\u{00ad}' => result.push_str("\\-"),
            other => match math_command_for(other) {
                Some(command) => {
                    result.push_str("\\ensuremath{");
                    result.push_str(command);
                    result.push('}');
                }
                None => result.push(other),
            },
        }
    }
    result
}

/// Label-/Schlüssel-Validierung für \label, \ref, \cite, \ac.
pub fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '+' | '-' | '/'))
}

/// Erzeugt aus beliebigem Text einen gültigen Label-Bestandteil.
pub fn slugify(text: &str, fallback: &str) -> String {
    let mut slug = String::new();
    for character in text.to_lowercase().chars() {
        let replacement = match character {
            'ä' => "ae",
            'ö' => "oe",
            'ü' => "ue",
            'ß' => "ss",
            c if c.is_ascii_alphanumeric() => {
                slug.push(c);
                continue;
            }
            _ => "-",
        };
        slug.push_str(replacement);
    }
    let collapsed: Vec<&str> = slug.split('-').filter(|part| !part.is_empty()).collect();
    let result: String = collapsed.join("-").chars().take(40).collect();
    if result.is_empty() {
        fallback.to_string()
    } else {
        result
    }
}

/// Hyperref-sichere URL (Prozent/Raute maskieren, Klammern kodieren).
pub fn escape_url(url: &str) -> String {
    let mut encoded = String::with_capacity(url.len());
    let bytes: Vec<char> = url.chars().collect();
    let mut index = 0;
    while index < bytes.len() {
        let character = bytes[index];
        match character {
            '\\' => encoded.push_str("%5C"),
            '{' => encoded.push_str("%7B"),
            '}' => encoded.push_str("%7D"),
            ' ' | '\t' | '\n' | '\r' => encoded.push_str("%20"),
            '%' => {
                let valid = bytes.get(index + 1).is_some_and(|c| c.is_ascii_hexdigit())
                    && bytes.get(index + 2).is_some_and(|c| c.is_ascii_hexdigit());
                encoded.push_str(if valid { "%" } else { "%25" });
            }
            other => encoded.push(other),
        }
        index += 1;
    }
    encoded.replace('%', "\\%").replace('#', "\\#")
}

pub fn unescape_url(url: &str) -> String {
    let mut result = String::with_capacity(url.len());
    let mut chars = url.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(&next) = chars.peek() {
                if matches!(next, '%' | '#' | '&' | '_' | '~' | '$') {
                    result.push(next);
                    chars.next();
                    continue;
                }
            }
        }
        result.push(character);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_special_characters() {
        assert_eq!(
            escape_latex("a\\b{c}$&#%_^~\"d"),
            "a\\textbackslash{}b\\{c\\}\\$\\&\\#\\%\\_\\textasciicircum{}\\textasciitilde{}\\textquotedbl{}d"
        );
        assert_eq!(
            escape_latex("x ≤ α"),
            "x \\ensuremath{\\leq} \\ensuremath{\\alpha}"
        );
        assert_eq!(escape_latex("a\u{00a0}b"), "a~b");
        assert_eq!(escape_latex("Umlaute äöüß € – …"), "Umlaute äöüß € – …");
    }

    #[test]
    fn url_escaping_round_trips() {
        let url = "https://example.com/a_b?x=1&y=50%25#teil";
        let escaped = escape_url(url);
        assert_eq!(escaped, "https://example.com/a_b?x=1&y=50\\%25\\#teil");
        assert_eq!(unescape_url(&escaped), url);
        assert_eq!(escape_url("https://a.b/50%"), "https://a.b/50\\%25");
    }

    #[test]
    fn slug_and_keys() {
        assert_eq!(slugify("Größe & Wert", "x"), "groesse-wert");
        assert!(is_valid_key("fig:mess-1"));
        assert!(!is_valid_key("a b"));
        assert_eq!(unicode_for_math_command("\\Delta"), Some('Δ'));
    }
}
