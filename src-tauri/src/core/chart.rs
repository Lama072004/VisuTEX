//! Diagramme aus Daten (wie „Diagramm einfügen“ in Word): Tabelle (aus Excel kopiert oder CSV)
//! und Diagrammart → `pgfplots`-Code (Linie, Säulen, Balken, Punkte, Fläche) bzw. reines TikZ
//! (Kreis). Ergebnis ist der Inhalt eines `tikzpicture` und landet als TikZ-Block im Dokument.
//!
//! Das Modell steht als Kommentar in der ersten Codezeile (`% VisuTeX-Diagramm: {…}`), damit ein
//! eingefügtes Diagramm im Diagramm-Editor wieder bearbeitet werden kann – wie bei Skizzen.

use super::escape::escape_latex;
use serde::{Deserialize, Serialize};

pub const CHART_COMMENT: &str = "% VisuTeX-Diagramm: ";

/// Farben der Kreisdiagramm-Segmente (dieselbe Reihenfolge wie pgfplots' Standardzyklus).
const PIE_COLORS: &[&str] = &[
    "blue",
    "red",
    "brown!60!black",
    "black",
    "teal",
    "orange",
    "violet",
    "cyan",
    "magenta",
    "lime!60!black",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Chart {
    /// line | bar | barh | scatter | area | pie
    pub kind: String,
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    /// Kopfzeile + Datenzeilen; erste Spalte = x-Werte bzw. Kategorien, weitere = Datenreihen
    pub data: Vec<Vec<String>>,
    /// north east | north west | south east | south west | below | none
    pub legend: String,
    pub grid: bool,
    /// Breite in cm (Höhe 62 % davon)
    pub width: f64,
    /// Säulen/Balken übereinander
    pub stacked: bool,
    /// Datenpunkte markieren (Linie, Fläche)
    pub marks: bool,
    /// Linien glätten
    pub smooth: bool,
    /// Dezimalkomma an den Achsen (deutsche Dokumente)
    pub decimal_comma: bool,
}

impl Default for Chart {
    fn default() -> Self {
        Self {
            kind: "line".into(),
            title: String::new(),
            x_label: String::new(),
            y_label: String::new(),
            data: Vec::new(),
            legend: "north east".into(),
            grid: true,
            width: 12.0,
            stacked: false,
            marks: true,
            smooth: false,
            decimal_comma: true,
        }
    }
}

/// Zahl aus einer Zelle: `1,5` (Dezimalkomma), `1.234,5`, `1,234.5`, `12 %`, `−3`.
pub fn number(cell: &str) -> Option<f64> {
    let mut text: String = cell
        .trim()
        .trim_end_matches('%')
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\'')
        .map(|c| if c == '−' { '-' } else { c })
        .collect();
    if text.is_empty() {
        return None;
    }
    match (text.rfind(','), text.rfind('.')) {
        // beide vorhanden: das hintere ist das Dezimalzeichen
        (Some(comma), Some(dot)) if comma > dot => text = text.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => text = text.replace(',', ""),
        (Some(_), None) => text = text.replace(',', "."),
        _ => {}
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// Tabelle aus Text: Tabulatoren (aus Excel/LibreOffice kopiert), sonst `;` (deutsche CSV) bzw.
/// `,`. Anführungszeichen wie in CSV; leere Zeilen werden ausgelassen.
pub fn parse_table(text: &str) -> Vec<Vec<String>> {
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let separator = if first.contains('\t') {
        '\t'
    } else if first.contains(';') {
        ';'
    } else {
        ','
    };
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut cells = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    current.push('"');
                    chars.next();
                }
                '"' => quoted = !quoted,
                c if c == separator && !quoted => {
                    cells.push(std::mem::take(&mut current).trim().to_string())
                }
                c => current.push(c),
            }
        }
        cells.push(current.trim().to_string());
        rows.push(cells);
    }
    rows
}

/// Zahl für den Code (Punkt als Dezimalzeichen, ohne überflüssige Nullen).
fn format_number(value: f64) -> String {
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.to_string()
    }
}

struct Series {
    name: String,
    values: Vec<Option<f64>>,
}

/// Kopfzeile erkannt? Dann (Reihennamen, Datenzeilen), sonst „Reihe 1 …“.
fn split_data(data: &[Vec<String>]) -> (Vec<String>, Vec<String>, Vec<Series>) {
    let width = data.iter().map(Vec::len).max().unwrap_or(0);
    let header = data.first().is_some_and(|row| {
        row.iter()
            .skip(1)
            .any(|cell| !cell.trim().is_empty() && number(cell).is_none())
    });
    let rows: Vec<&Vec<String>> = data
        .iter()
        .skip(usize::from(header))
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect();
    let cell = |row: &Vec<String>, column: usize| row.get(column).cloned().unwrap_or_default();
    let categories: Vec<String> = rows.iter().map(|row| cell(row, 0)).collect();
    let mut series = Vec::new();
    for column in 1..width.max(1) {
        let name = if header {
            cell(&data[0], column)
        } else {
            format!("Reihe {column}")
        };
        series.push(Series {
            name,
            values: rows.iter().map(|row| number(&cell(row, column))).collect(),
        });
    }
    let x_label = if header {
        cell(&data[0], 0)
    } else {
        String::new()
    };
    (categories, vec![x_label], series)
}

/// Code für das Modell (Inhalt des `tikzpicture`, erste Zeile = Modell als Kommentar).
pub fn to_latex(chart: &Chart) -> Result<String, String> {
    let (categories, header, series) = split_data(&chart.data);
    if categories.is_empty()
        || series
            .iter()
            .all(|serie| serie.values.iter().all(Option::is_none))
    {
        return Err("Keine Zahlen in der Tabelle – erste Spalte: Beschriftungen bzw. x-Werte, weitere Spalten: Werte.".into());
    }
    let model = serde_json::to_string(chart).map_err(|error| error.to_string())?;
    let mut lines = vec![format!("{CHART_COMMENT}{model}")];
    if chart.kind == "pie" {
        lines.extend(pie(chart, &categories, &series[0]));
        return Ok(lines.join("\n"));
    }
    let numeric_x = chart.kind != "bar"
        && chart.kind != "barh"
        && categories.iter().all(|category| number(category).is_some());
    let width = chart.width.clamp(4.0, 30.0);
    let mut options = vec![
        format!("width={}cm", format_number(width)),
        format!("height={}cm", format_number(width * 0.62)),
    ];
    let title = chart.title.trim();
    if !title.is_empty() {
        options.push(format!("title={{{}}}", escape_latex(title)));
    }
    let x_label = if chart.x_label.trim().is_empty() {
        header[0].trim()
    } else {
        chart.x_label.trim()
    };
    let (horizontal_label, vertical_label) = if chart.kind == "barh" {
        (chart.y_label.trim(), x_label)
    } else {
        (x_label, chart.y_label.trim())
    };
    if !horizontal_label.is_empty() {
        options.push(format!("xlabel={{{}}}", escape_latex(horizontal_label)));
    }
    if !vertical_label.is_empty() {
        options.push(format!("ylabel={{{}}}", escape_latex(vertical_label)));
    }
    if chart.grid {
        options.push("grid=major".into());
    }
    if chart.decimal_comma {
        options.push("/pgf/number format/use comma".into());
        options.push("/pgf/number format/1000 sep={\\,}".into());
    }
    // Kategorien als Positionen 1…n mit Beschriftung – beliebiger Text bleibt so sicher
    let positions: Vec<String> = (1..=categories.len())
        .map(|index| index.to_string())
        .collect();
    let tick_labels = categories
        .iter()
        .map(|category| format!("{{{}}}", escape_latex(category.trim())))
        .collect::<Vec<_>>()
        .join(",");
    match chart.kind.as_str() {
        "bar" => {
            options.push(if chart.stacked {
                "ybar stacked".into()
            } else {
                "ybar".into()
            });
            options.push(format!(
                "bar width={}cm",
                format_number(
                    (width * 0.6
                        / (categories.len()
                            * if chart.stacked {
                                1
                            } else {
                                series.len().max(1)
                            }) as f64)
                        .clamp(0.12, 1.2)
                )
            ));
            options.push(format!("xtick={{{}}}", positions.join(",")));
            options.push(format!("xticklabels={{{tick_labels}}}"));
            options.push("enlarge x limits=0.15".into());
            options.push("ymin=0".into());
        }
        "barh" => {
            options.push(if chart.stacked {
                "xbar stacked".into()
            } else {
                "xbar".into()
            });
            options.push(format!(
                "bar width={}cm",
                format_number(
                    (width * 0.4
                        / (categories.len()
                            * if chart.stacked {
                                1
                            } else {
                                series.len().max(1)
                            }) as f64)
                        .clamp(0.12, 1.0)
                )
            ));
            options.push(format!("ytick={{{}}}", positions.join(",")));
            options.push(format!("yticklabels={{{tick_labels}}}"));
            options.push("enlarge y limits=0.15".into());
            options.push("xmin=0".into());
        }
        _ if !numeric_x => {
            options.push(format!("xtick={{{}}}", positions.join(",")));
            options.push(format!("xticklabels={{{tick_labels}}}"));
        }
        _ => {
            // Ganze Zahlen (z. B. Jahre): Striche genau an den Daten statt 2021,5
            let xs: Vec<f64> = categories.iter().filter_map(|c| number(c)).collect();
            if xs.len() <= 20 && xs.iter().all(|x| x.fract() == 0.0) {
                options.push("xtick=data".into());
            }
            // Jahreszahlen ohne Tausendertrennung („2021“ statt „2 021“)
            if xs
                .iter()
                .all(|x| x.fract() == 0.0 && (1000.0..=9999.0).contains(x))
            {
                options.push("x tick label style={/pgf/number format/1000 sep={}}".into());
            }
        }
    }
    if chart.kind == "area" {
        if chart.stacked {
            options.push("stack plots=y".into());
        }
        // Flächen von der Nulllinie aus (sofern keine negativen Werte)
        if series
            .iter()
            .flat_map(|serie| serie.values.iter().flatten())
            .all(|value| *value >= 0.0)
        {
            options.push("ymin=0".into());
        }
    }
    let legend = series.len() > 1 && chart.legend != "none";
    if legend {
        if chart.legend == "below" {
            options.push("legend style={at={(0.5,-0.18)},anchor=north,legend columns=-1}".into());
        } else {
            let position = match chart.legend.as_str() {
                "north west" | "south east" | "south west" => chart.legend.as_str(),
                _ => "north east",
            };
            options.push(format!("legend pos={position}"));
        }
    }
    lines.push(format!("\\begin{{axis}}[\n  {},\n]", options.join(",\n  ")));
    for serie in &series {
        let coordinates: Vec<String> = serie
            .values
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let value = (*value)?;
                let x = if numeric_x {
                    number(&categories[index]).map(format_number)?
                } else {
                    (index + 1).to_string()
                };
                Some(if chart.kind == "barh" {
                    format!("({},{x})", format_number(value))
                } else {
                    format!("({x},{})", format_number(value))
                })
            })
            .collect();
        let mut style: Vec<&str> = Vec::new();
        match chart.kind.as_str() {
            "scatter" => style.push("only marks"),
            "line" | "area" => {
                if !chart.marks {
                    style.push("no marks");
                }
                if chart.smooth {
                    style.push("smooth");
                }
                if chart.kind == "area" {
                    style.push("fill");
                    style.push("fill opacity=0.25");
                }
            }
            _ => {}
        }
        let closing = if chart.kind == "area" {
            " \\closedcycle"
        } else {
            ""
        };
        lines.push(format!(
            "\\addplot+[{}] coordinates {{{}}}{closing};",
            style.join(", "),
            coordinates.join(" ")
        ));
        if legend {
            lines.push(format!(
                "\\addlegendentry{{{}}}",
                escape_latex(serie.name.trim())
            ));
        }
    }
    lines.push("\\end{axis}".into());
    Ok(lines.join("\n"))
}

/// Kreisdiagramm (reines TikZ): erste Datenreihe, Segmente mit Prozentangabe, Legende rechts.
fn pie(chart: &Chart, categories: &[String], serie: &Series) -> Vec<String> {
    let values: Vec<(String, f64)> = categories
        .iter()
        .zip(&serie.values)
        .filter_map(|(category, value)| {
            value
                .filter(|value| *value > 0.0)
                .map(|value| (category.clone(), value))
        })
        .collect();
    let total: f64 = values.iter().map(|(_, value)| value).sum();
    let radius = (chart.width.clamp(4.0, 30.0) * 0.22).clamp(1.5, 4.5);
    let mut lines = Vec::new();
    if !chart.title.trim().is_empty() {
        lines.push(format!(
            "\\node[font=\\bfseries] at (0,{}) {{{}}};",
            format_number(radius + 0.6),
            escape_latex(chart.title.trim())
        ));
    }
    let mut angle = 90.0_f64;
    for (index, (category, value)) in values.iter().enumerate() {
        let share = value / total;
        let end = angle - share * 360.0;
        let color = PIE_COLORS[index % PIE_COLORS.len()];
        lines.push(format!(
            "\\fill[{color}!70, draw=white, thick] (0,0) -- ({}:{r}) arc[start angle={}, end angle={}, radius={r}] -- cycle;",
            format_number(angle),
            format_number(angle),
            format_number(end),
            r = format_number(radius)
        ));
        let percent = share * 100.0;
        let label = if chart.decimal_comma {
            format!("{:.1}", percent).replace('.', ",")
        } else {
            format!("{percent:.1}")
        };
        if share >= 0.04 {
            lines.push(format!(
                "\\node[font=\\small] at ({}:{}) {{{label}\\,\\%}};",
                format_number((angle + end) / 2.0),
                format_number(radius * 0.65)
            ));
        }
        let y = format_number(radius - 0.6 * index as f64);
        lines.push(format!(
            "\\fill[{color}!70] ({},{y}) rectangle ++(0.35,0.35);",
            format_number(radius + 0.6)
        ));
        lines.push(format!(
            "\\node[anchor=west, font=\\small] at ({},{}) {{{}}};",
            format_number(radius + 1.05),
            format_number(radius - 0.6 * index as f64 + 0.175),
            escape_latex(category.trim())
        ));
        angle = end;
    }
    lines
}

/// Modell aus dem Code eines TikZ-Blocks (zum erneuten Bearbeiten).
pub fn from_code(code: &str) -> Option<Chart> {
    let first = code.lines().next()?.trim();
    let json = first.strip_prefix(CHART_COMMENT.trim_end())?.trim();
    serde_json::from_str(json).ok()
}

/// Braucht der Code `pgfplots` (Achsenumgebung)?
pub fn uses_pgfplots(code: &str) -> bool {
    [
        "\\begin{axis}",
        "\\begin{semilogxaxis}",
        "\\begin{semilogyaxis}",
        "\\begin{loglogaxis}",
        "\\begin{polaraxis}",
        "\\addplot",
    ]
    .iter()
    .any(|needle| code.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Vec<Vec<String>> {
        parse_table(text)
    }

    #[test]
    fn reads_numbers_and_tables_like_excel() {
        assert_eq!(number("1,5"), Some(1.5));
        assert_eq!(number("1.234,5"), Some(1234.5));
        assert_eq!(number("1,234.5"), Some(1234.5));
        assert_eq!(number(" 12 % "), Some(12.0));
        assert_eq!(number("−3"), Some(-3.0));
        assert_eq!(number("Jan"), None);
        // aus Excel kopiert (Tabulatoren)
        let pasted = table("Monat\tUmsatz\tKosten\nJan\t10,5\t7\nFeb\t12\t8,25\n\n");
        assert_eq!(pasted.len(), 3);
        assert_eq!(pasted[2], vec!["Feb", "12", "8,25"]);
        // deutsche CSV mit Anführungszeichen
        let csv = table("Ort;\"Wert; netto\"\n\"Graz \"\"Süd\"\"\";3,5");
        assert_eq!(csv[0][1], "Wert; netto");
        assert_eq!(csv[1][0], "Graz \"Süd\"");
    }

    #[test]
    fn creates_line_and_bar_charts() {
        let chart = Chart {
            kind: "line".into(),
            title: "Messung & Auswertung".into(),
            y_label: "Spannung in V".into(),
            data: table("t in s\tU1\tU2\n0\t0\t1\n0,5\t2,5\t1,5\n1\t4\t2"),
            ..Chart::default()
        };
        let code = to_latex(&chart).unwrap();
        assert!(code.starts_with(CHART_COMMENT));
        assert!(code.contains("title={Messung \\& Auswertung}"), "{code}");
        assert!(
            code.contains("xlabel={t in s}"),
            "Kopfzeile der ersten Spalte als x-Achse"
        );
        assert!(code.contains("coordinates {(0,0) (0.5,2.5) (1,4)}"));
        assert!(code.contains("\\addlegendentry{U2}"));
        assert!(code.contains("/pgf/number format/use comma"));
        assert_eq!(from_code(&code), Some(chart.clone()));
        assert!(uses_pgfplots(&code));

        let bars = Chart {
            kind: "bar".into(),
            stacked: true,
            decimal_comma: false,
            legend: "below".into(),
            data: table("Quartal;A;B\nQ1;1;2\nQ2;3;4"),
            ..Chart::default()
        };
        let code = to_latex(&bars).unwrap();
        assert!(code.contains("ybar stacked"));
        assert!(code.contains("xticklabels={{Q1},{Q2}}"));
        assert!(code.contains("coordinates {(1,1) (2,3)}"));
        assert!(code.contains("legend style={at={(0.5,-0.18)}"));
        assert!(!code.contains("use comma"));
        // eine Reihe → keine Legende
        let single = Chart {
            data: table("x;y\n1;2\n2;3"),
            ..Chart::default()
        };
        assert!(!to_latex(&single).unwrap().contains("addlegendentry"));
        // Jahre: Striche an den Daten, ohne Tausendertrennung; Kommazahlen: automatische Striche
        let years = to_latex(&Chart {
            data: table("Jahr;Umsatz\n2021;12,5\n2022;14\n2023;17,8"),
            ..Chart::default()
        })
        .unwrap();
        assert!(years.contains("xtick=data"), "{years}");
        assert!(years.contains("x tick label style={/pgf/number format/1000 sep={}}"));
        assert!(!code.contains("xtick=data"));
        let fractions = to_latex(&chart).unwrap();
        assert!(!fractions.contains("xtick=data"));
        assert!(!fractions.contains("1000 sep={}"));
        // keine Zahlen
        assert!(to_latex(&Chart {
            data: table("a;b\nc;d"),
            ..Chart::default()
        })
        .is_err());
    }

    #[test]
    fn creates_pie_charts_without_pgfplots() {
        let chart = Chart {
            kind: "pie".into(),
            data: table("Anteil;Wert\nStrom;50\nGas;30\nÖl;20"),
            ..Chart::default()
        };
        let code = to_latex(&chart).unwrap();
        assert!(!uses_pgfplots(&code));
        assert_eq!(code.matches("arc[start angle").count(), 3);
        assert!(code.contains("{50,0\\,\\%}"), "{code}");
        assert!(code.contains("{Öl}"));
    }
}
