//! Kompilieren mit der eingebauten Tectonic-Engine.
//!
//! - läuft immer in einem Hintergrund-Thread (`spawn_blocking` im Command),
//! - Tectonic-Engines sind global – eine Warteschlange (`ENGINE_LOCK`) serialisiert Läufe,
//! - Projektordner als Dateisystemwurzel (Bilder, .bib, eigene .sty),
//! - zusätzliche Suchpfade für TeX-Dateien aktivierter Add-ons,
//! - BibTeX wird von Tectonic automatisch ausgeführt,
//! - Fehler werden mit Zeilennummer aus dem Log zurückgegeben.

use crate::texbundle::{open_bundle, BundleConfig};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tectonic_bridge_core::{SecuritySettings, SecurityStance};
use tectonic_errors::Error as TexError;
use tectonic_status_base::{MessageKind, StatusBackend};

static ENGINE_LOCK: Mutex<()> = Mutex::new(());

pub const MAIN_NAME: &str = "dokument";

pub struct CompileRequest {
    pub latex: String,
    pub project_root: Option<PathBuf>,
    pub extra_search_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TexMessage {
    /// Zeile im Quelltext (1-basiert), sofern im Log angegeben.
    pub line: Option<u32>,
    pub message: String,
    /// error | warning
    pub severity: String,
}

pub struct CompileOutput {
    pub pdf: Vec<u8>,
    pub messages: Vec<TexMessage>,
    pub log: String,
    /// Inhalt der .aux-Datei (z. B. für die Seitenzuordnung der Vorschau).
    pub aux: String,
    /// TeX-Dateien, die fehlten (PDF trotzdem erzeugt) – z. B. zum Nachladen anbieten.
    pub missing_files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileFailure {
    pub message: String,
    pub messages: Vec<TexMessage>,
    pub log: String,
    /// TeX-Dateien, die weder im Bundle noch im Projekt gefunden wurden.
    pub missing_files: Vec<String>,
}

impl CompileFailure {
    fn simple(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            messages: Vec::new(),
            log: String::new(),
            missing_files: Vec::new(),
        }
    }
}

/// Sammelt Tectonic-Meldungen und leitet Hinweise (z. B. Downloads) weiter.
struct Collector<'a> {
    messages: Vec<String>,
    progress: &'a mut dyn FnMut(&str),
}

impl StatusBackend for Collector<'_> {
    fn report(&mut self, kind: MessageKind, args: std::fmt::Arguments, error: Option<&TexError>) {
        let mut text = args.to_string();
        if let Some(error) = error {
            text.push_str(&format!(" ({error:#})"));
        }
        if kind == MessageKind::Note {
            (self.progress)(&text);
        }
        self.messages.push(format!("{kind:?}: {text}"));
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        self.messages
            .push(String::from_utf8_lossy(output).into_owned());
    }
}

/// Fehler und Warnungen aus einem TeX-Log.
pub fn parse_log(log: &str) -> Vec<TexMessage> {
    let lines: Vec<&str> = log.lines().collect();
    let mut messages = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(error) = line.strip_prefix("! ") {
            let mut text = error.trim().trim_end_matches('.').to_string();
            text.push('.');
            let mut source_line = None;
            for follow in lines.iter().skip(index + 1).take(12) {
                if let Some(rest) = follow.strip_prefix("l.") {
                    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                    source_line = digits.parse().ok();
                    let context = rest[digits.len()..].trim();
                    if !context.is_empty() {
                        text.push_str(&format!(" – bei „{context}“"));
                    }
                    break;
                }
            }
            messages.push(TexMessage {
                line: source_line,
                message: text,
                severity: "error".into(),
            });
        } else if line.contains("Warning:")
            && (line.starts_with("LaTeX")
                || line.starts_with("Package")
                || line.starts_with("Class"))
        {
            // Mehrzeilige Warnungen zusammenfassen: Fortsetzungszeilen von Paketen
            // beginnen mit „(paket)   “, umbrochene Zeilen folgen auf volle 79 Zeichen.
            let mut text = line.trim().to_string();
            let mut next = index + 1;
            while next < lines.len() && next - index <= 6 {
                let follow = lines[next];
                let package_continuation = follow.starts_with('(')
                    && follow
                        .find(')')
                        .is_some_and(|end| follow[end + 1..].starts_with("  "));
                let wrapped = lines[next - 1].chars().count() >= 79 && !follow.trim().is_empty();
                if !(package_continuation || wrapped) {
                    break;
                }
                let content = if package_continuation {
                    follow[follow.find(')').unwrap_or(0) + 1..].trim()
                } else {
                    follow.trim()
                };
                if wrapped && !package_continuation {
                    text.push_str(content);
                } else {
                    text.push(' ');
                    text.push_str(content);
                }
                next += 1;
            }
            let source_line = text
                .rsplit_once("on input line ")
                .and_then(|(_, rest)| rest.trim_end_matches('.').trim().parse().ok());
            messages.push(TexMessage {
                line: source_line,
                message: text,
                severity: "warning".into(),
            });
            index = next;
            continue;
        }
        index += 1;
    }
    messages.dedup();
    messages
}

/// Dateinamen aus „File `x.sty' not found“ bzw. „I can't find file `x'“.
pub fn missing_files(log: &str) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for marker in ["File `", "I can't find file `", "can't find file `"] {
        let mut rest = log;
        while let Some(index) = rest.find(marker) {
            let after = &rest[index + marker.len()..];
            let Some(end) = after.find('\'') else {
                break;
            };
            let name = after[..end].trim();
            let tail = &after[end..];
            let is_missing = marker != "File `" || tail.starts_with("' not found");
            // Makro-Parameter wie `#1.#2` aus LaTeX-Hilfetexten sind keine Dateien.
            let plausible = !name.contains(['#', '\\', '{', '}']);
            if is_missing
                && plausible
                && !name.is_empty()
                && name.len() < 200
                && !files.iter().any(|f| f == name)
            {
                files.push(name.to_string());
            }
            rest = &after[end..];
        }
    }
    files
}
pub fn compile(
    request: CompileRequest,
    bundle: &BundleConfig,
    progress: &mut dyn FnMut(&str),
) -> Result<CompileOutput, CompileFailure> {
    let _guard = ENGINE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    crate::fonts::configure_fontconfig();
    (progress)("TeX-Bundle wird geöffnet …");
    let (bundle, format_cache) = open_bundle(bundle).map_err(CompileFailure::simple)?;
    // Anpassungen für die eingebaute Engine (biblatex/Biber, pdfx, fehlende Bilder) –
    // Zeilennummern bleiben gleich.
    let prepared =
        crate::prepare::prepare_for_engine(&request.latex, request.project_root.as_deref());
    let mut collector = Collector {
        messages: Vec::new(),
        progress,
    };
    // Eigene Dateien von VisuTeX (z. B. deutscher IEEE-Stil) über einen Suchpfad bereitstellen
    let mut search_paths = request.extra_search_paths.clone();
    if !crate::core::support::files_for(&prepared.latex).is_empty() {
        let dir = std::env::temp_dir().join("visutex-tex");
        crate::core::support::write_files(&prepared.latex, &dir, true)
            .map_err(CompileFailure::simple)?;
        search_paths.push(dir);
    }

    // Zusätzliche Suchpfade sind ein „unsicheres“ Tectonic-Feature – Add-ons sind
    // deklarativ und Shell-Escape bleibt deaktiviert.
    let security = SecuritySettings::new(SecurityStance::MaybeAllowInsecures);
    let mut builder = tectonic::driver::ProcessingSessionBuilder::new_with_security(security);
    builder
        .bundle(bundle)
        .primary_input_buffer(prepared.latex.as_bytes())
        .tex_input_name(&format!("{MAIN_NAME}.tex"))
        .format_name("latex")
        .format_cache_path(format_cache)
        .keep_logs(true)
        .keep_intermediates(false)
        .print_stdout(false)
        .output_format(tectonic::driver::OutputFormat::Pdf)
        .shell_escape_disabled()
        .do_not_write_output_files();
    if let Some(root) = &request.project_root {
        builder.filesystem_root(root);
    }
    // Wie TeXstudio/Overleaf (nonstopmode): Fehler im Dokument stoppen nicht, das PDF
    // entsteht trotzdem und die Fehler werden mit Zeilennummer gemeldet. Fehlende
    // Dateien bleiben fatal (→ Dialog zum Nachladen).
    builder.unstables(tectonic::unstable_opts::UnstableOptions {
        extra_search_paths: search_paths,
        continue_on_errors: true,
        ..Default::default()
    });

    (collector.progress)("Kompiliere …");
    let mut session = builder
        .create(&mut collector)
        .map_err(|error| CompileFailure {
            message: format!("Tectonic konnte nicht gestartet werden: {error:#}"),
            messages: Vec::new(),
            log: collector.messages.join("\n"),
            missing_files: Vec::new(),
        })?;
    let result = session.run(&mut collector);
    let files = session.into_file_data();
    let log = files
        .get(&format!("{MAIN_NAME}.log"))
        .map(|file| String::from_utf8_lossy(&file.data).into_owned())
        .unwrap_or_default();
    let mut messages = parse_log(&log);
    for note in &prepared.notes {
        messages.push(TexMessage {
            line: None,
            message: format!("Hinweis: {note}"),
            severity: "warning".into(),
        });
    }
    if let Some(blg) = files.get(&format!("{MAIN_NAME}.blg")) {
        for line in String::from_utf8_lossy(&blg.data).lines() {
            if line.starts_with("Warning--")
                || line.contains("I couldn't open")
                || line.contains("I found no")
            {
                messages.push(TexMessage {
                    line: None,
                    message: format!("BibTeX: {line}"),
                    severity: "warning".into(),
                });
            }
        }
    }

    match result {
        Ok(()) => match files.get(&format!("{MAIN_NAME}.pdf")) {
            Some(pdf) => Ok(CompileOutput {
                pdf: pdf.data.clone(),
                messages,
                missing_files: missing_files(&log),
                log,
                aux: files
                    .get(&format!("{MAIN_NAME}.aux"))
                    .map(|file| String::from_utf8_lossy(&file.data).into_owned())
                    .unwrap_or_default(),
            }),
            None => Err(CompileFailure {
                message: "Tectonic meldete keinen Fehler, erzeugte aber kein PDF.".into(),
                messages,
                missing_files: missing_files(&log),
                log,
            }),
        },
        Err(error) => {
            let first = messages
                .iter()
                .find(|message| message.severity == "error")
                .map(|message| match message.line {
                    Some(line) => format!("Zeile {line}: {}", message.message),
                    None => message.message.clone(),
                });
            let details = collector.messages.join("\n");
            let mut missing = missing_files(&log);
            for file in missing_files(&details) {
                if !missing.contains(&file) {
                    missing.push(file);
                }
            }
            Err(CompileFailure {
                message: first.unwrap_or_else(|| format!("Kompilierung fehlgeschlagen: {error:#}")),
                messages,
                missing_files: missing,
                log: if log.is_empty() { details } else { log },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_files() {
        let log = "! LaTeX Error: File `siunitx.sty' not found.\nType X to quit\nFile `ok.sty' loaded\n! I can't find file `kapitel1'.\n\\def\\x{File `#1.#2' not found}\n";
        assert_eq!(
            missing_files(log),
            vec!["siunitx.sty".to_string(), "kapitel1".to_string()]
        );
    }
    #[test]
    fn parses_errors_and_warnings_from_log() {
        let log = "This is XeTeX\n! Undefined control sequence.\nl.42 \\foo\n          bar\n\nLaTeX Warning: Reference `fig:x' on page 1 undefined on input line 17.\n\nPackage natbib Warning: Citation `abc' on page 1 undefined on input line 20.\n";
        let messages = parse_log(log);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].line, Some(42));
        assert_eq!(messages[0].severity, "error");
        assert!(messages[0].message.contains("\\foo"));
        assert_eq!(messages[1].line, Some(17));
        assert_eq!(messages[2].line, Some(20));
    }
}
