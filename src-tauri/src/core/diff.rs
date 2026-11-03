//! Vergleich zweier Fassungen (Versionsverlauf, „Mit Datei vergleichen“): Zeilen per
//! Myers-Algorithmus, geänderte Zeilen zusätzlich wortweise – wie „Vergleichen“ in Word,
//! nur auf dem LaTeX-Quelltext. Ergebnis sind Abschnitte (Hunks) mit etwas Kontext.

use serde::Serialize;

/// Höchstzahl an Änderungsschritten, bis der Rest als „ersetzt“ gilt (Speicher ~ D²).
const MAX_EDITS: usize = 3000;
/// Unveränderte Zeilen vor und nach einer Änderung.
pub const CONTEXT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Segment {
    pub text: String,
    /// hervorgehoben (geändertes Wort innerhalb einer geänderten Zeile)
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// same | removed | added
    pub kind: &'static str,
    /// Zeilennummern (ab 1) in der alten bzw. neuen Fassung
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Hunk {
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub added_lines: usize,
    pub removed_lines: usize,
    pub added_words: usize,
    pub removed_words: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TextDiff {
    pub identical: bool,
    pub hunks: Vec<Hunk>,
    pub summary: DiffSummary,
}

/// Kürzeste Änderungsfolge von `a` nach `b` (gemeinsamer Anfang/Ende vorab abgeschnitten).
fn diff_ops<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Op> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (a_mid, b_mid) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let mut ops: Vec<Op> = (0..prefix).map(|i| Op::Equal(i, i)).collect();
    let middle = myers(a_mid, b_mid).unwrap_or_else(|| {
        // zu viele Änderungen: Mittelteil vollständig ersetzt
        (0..a_mid.len())
            .map(Op::Delete)
            .chain((0..b_mid.len()).map(Op::Insert))
            .collect()
    });
    ops.extend(middle.into_iter().map(|op| match op {
        Op::Equal(i, j) => Op::Equal(i + prefix, j + prefix),
        Op::Delete(i) => Op::Delete(i + prefix),
        Op::Insert(j) => Op::Insert(j + prefix),
    }));
    let (a_end, b_end) = (a.len() - suffix, b.len() - suffix);
    ops.extend((0..suffix).map(|i| Op::Equal(a_end + i, b_end + i)));
    ops
}

/// Myers (1986), O((N+M)·D); `None`, wenn mehr als `MAX_EDITS` Schritte nötig wären.
fn myers<T: PartialEq>(a: &[T], b: &[T]) -> Option<Vec<Op>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = (n + m) as usize;
    if max == 0 {
        return Some(Vec::new());
    }
    let offset = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    // trace[d]: Stand vor Schritt d, nur Diagonalen -d..=d
    let mut trace: Vec<Vec<isize>> = Vec::new();
    for d in 0..=max.min(MAX_EDITS) as isize {
        trace.push(v[(offset - d) as usize..=(offset + d) as usize].to_vec());
        let mut k = -d;
        while k <= d {
            let index = (offset + k) as usize;
            let mut x = if k == -d || (k != d && v[index - 1] < v[index + 1]) {
                v[index + 1]
            } else {
                v[index - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[index] = x;
            if x >= n && y >= m {
                return Some(backtrack(&trace, n, m));
            }
            k += 2;
        }
    }
    None
}

fn backtrack(trace: &[Vec<isize>], n: isize, m: isize) -> Vec<Op> {
    let mut ops = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (1..trace.len() as isize).rev() {
        let v = &trace[d as usize];
        let at = |k: isize| v[(k + d) as usize];
        let k = x - y;
        let previous_k = if k == -d || (k != d && at(k - 1) < at(k + 1)) {
            k + 1
        } else {
            k - 1
        };
        let previous_x = at(previous_k);
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            x -= 1;
            y -= 1;
            ops.push(Op::Equal(x as usize, y as usize));
        }
        if x == previous_x {
            ops.push(Op::Insert(previous_y as usize));
        } else {
            ops.push(Op::Delete(previous_x as usize));
        }
        x = previous_x;
        y = previous_y;
    }
    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        ops.push(Op::Equal(x as usize, y as usize));
    }
    ops.reverse();
    ops
}

/// Wörter, LaTeX-Befehle, Leerraum und einzelne Zeichen.
fn tokens(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        let mut end = start + c.len_utf8();
        let same_class = |next: char| -> bool {
            if c.is_alphanumeric() || c == '\\' {
                next.is_alphanumeric()
            } else if c.is_whitespace() {
                next.is_whitespace()
            } else {
                false
            }
        };
        while let Some(&(index, next)) = chars.peek() {
            if !same_class(next) {
                break;
            }
            end = index + next.len_utf8();
            chars.next();
        }
        result.push(&text[start..end]);
    }
    result
}

fn is_word(token: &str) -> bool {
    token.chars().next().is_some_and(char::is_alphanumeric)
}

/// Wörter im Fließtext (ohne Befehle wie `\section`).
pub fn word_count(text: &str) -> usize {
    tokens(text)
        .into_iter()
        .filter(|token| is_word(token))
        .count()
}

fn plain(text: &str) -> Vec<Segment> {
    vec![Segment {
        text: text.to_string(),
        changed: false,
    }]
}

/// Aufeinanderfolgende Teile gleicher Art zusammenfassen.
fn push_segment(segments: &mut Vec<Segment>, text: &str, changed: bool) {
    match segments.last_mut() {
        Some(last) if last.changed == changed => last.text.push_str(text),
        _ => segments.push(Segment {
            text: text.to_string(),
            changed,
        }),
    }
}

/// Wortweiser Vergleich zweier Zeilen; `None`, wenn sie kaum etwas gemeinsam haben.
fn word_diff(old: &str, new: &str) -> Option<(Vec<Segment>, Vec<Segment>, usize, usize)> {
    let (a, b) = (tokens(old), tokens(new));
    let ops = diff_ops(&a, &b);
    let common: usize = ops
        .iter()
        .filter_map(|op| match op {
            Op::Equal(i, _) if !a[*i].trim().is_empty() => Some(a[*i].len()),
            _ => None,
        })
        .sum();
    let longer = old.trim().len().max(new.trim().len()).max(1);
    if common * 10 < longer * 4 {
        return None;
    }
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let (mut removed, mut added) = (0, 0);
    for op in ops {
        match op {
            Op::Equal(i, j) => {
                push_segment(&mut left, a[i], false);
                push_segment(&mut right, b[j], false);
            }
            Op::Delete(i) => {
                removed += usize::from(is_word(a[i]));
                push_segment(&mut left, a[i], true);
            }
            Op::Insert(j) => {
                added += usize::from(is_word(b[j]));
                push_segment(&mut right, b[j], true);
            }
        }
    }
    Some((left, right, removed, added))
}

/// Vergleicht zwei Texte zeilenweise; geänderte Zeilenpaare zusätzlich wortweise.
pub fn compare(old: &str, new: &str, context: usize) -> TextDiff {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let ops = diff_ops(&a, &b);
    let mut summary = DiffSummary::default();
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Equal(..)))
        .map(|(index, _)| index)
        .collect();
    // Bereiche mit Kontext, überlappende zusammengelegt
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for index in changed {
        let start = index.saturating_sub(context);
        let end = (index + context + 1).min(ops.len());
        match ranges.last_mut() {
            Some(last) if start <= last.1 => last.1 = end,
            _ => ranges.push((start, end)),
        }
    }
    let mut hunks = Vec::new();
    for (start, end) in ranges {
        let mut lines = Vec::new();
        let mut index = start;
        while index < end {
            if let Op::Equal(i, j) = ops[index] {
                lines.push(DiffLine {
                    kind: "same",
                    old_line: Some(i + 1),
                    new_line: Some(j + 1),
                    segments: plain(a[i]),
                });
                index += 1;
                continue;
            }
            // zusammenhängende Änderung: erst alle gelöschten, dann alle neuen Zeilen
            let mut deleted = Vec::new();
            let mut inserted = Vec::new();
            while index < end {
                match ops[index] {
                    Op::Delete(i) => deleted.push(i),
                    Op::Insert(j) => inserted.push(j),
                    Op::Equal(..) => break,
                }
                index += 1;
            }
            summary.removed_lines += deleted.len();
            summary.added_lines += inserted.len();
            let mut left: Vec<Option<Vec<Segment>>> = vec![None; deleted.len()];
            let mut right: Vec<Option<Vec<Segment>>> = vec![None; inserted.len()];
            for pair in 0..deleted.len().min(inserted.len()) {
                if let Some((old_segments, new_segments, removed, added)) =
                    word_diff(a[deleted[pair]], b[inserted[pair]])
                {
                    left[pair] = Some(old_segments);
                    right[pair] = Some(new_segments);
                    summary.removed_words += removed;
                    summary.added_words += added;
                }
            }
            for (slot, &i) in left.into_iter().zip(&deleted) {
                let segments = slot.unwrap_or_else(|| {
                    summary.removed_words += word_count(a[i]);
                    plain(a[i])
                });
                lines.push(DiffLine {
                    kind: "removed",
                    old_line: Some(i + 1),
                    new_line: None,
                    segments,
                });
            }
            for (slot, &j) in right.into_iter().zip(&inserted) {
                let segments = slot.unwrap_or_else(|| {
                    summary.added_words += word_count(b[j]);
                    plain(b[j])
                });
                lines.push(DiffLine {
                    kind: "added",
                    old_line: None,
                    new_line: Some(j + 1),
                    segments,
                });
            }
        }
        hunks.push(Hunk { lines });
    }
    TextDiff {
        identical: hunks.is_empty(),
        hunks,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(a: &[&str], b: &[&str], ops: &[Op]) -> Vec<String> {
        // aus den Operationen muss genau `b` entstehen, und jede Zeile von `a` kommt vor
        let mut result = Vec::new();
        let mut seen = 0;
        for op in ops {
            match *op {
                Op::Equal(i, j) => {
                    assert_eq!(a[i], b[j]);
                    assert_eq!(i, seen);
                    seen += 1;
                    result.push(b[j].to_string());
                }
                Op::Delete(i) => {
                    assert_eq!(i, seen);
                    seen += 1;
                }
                Op::Insert(j) => result.push(b[j].to_string()),
            }
        }
        assert_eq!(seen, a.len());
        result
    }

    #[test]
    fn finds_shortest_line_edits() {
        let a = ["a", "b", "c", "a", "b", "b", "a"];
        let b = ["c", "b", "a", "b", "a", "c"];
        let ops = diff_ops(&a, &b);
        assert_eq!(apply(&a, &b, &ops), b);
        // Myers' Beispiel: D = 5
        assert_eq!(
            ops.iter().filter(|op| !matches!(op, Op::Equal(..))).count(),
            5
        );
        for (a, b) in [
            (vec![], vec!["x"]),
            (vec!["x"], vec![]),
            (vec![], vec![]),
            (vec!["x", "y"], vec!["x", "y"]),
            (vec!["1", "2", "3"], vec!["0", "2", "4", "5"]),
        ] {
            let ops = diff_ops(&a, &b);
            assert_eq!(apply(&a, &b, &ops), b);
        }
    }

    #[test]
    fn compares_lines_and_words() {
        let old = "\\section{Einleitung}\nDas ist der alte Satz.\nGleich\nGleich\nGleich\nGleich\nGleich\nGleich\nGleich\nEnde alt\n";
        let new = "\\section{Einleitung}\nDas ist der neue Satz.\nGleich\nGleich\nGleich\nGleich\nGleich\nGleich\nGleich\nEnde alt\nGanz neue Zeile hier\n";
        let diff = compare(old, new, 1);
        assert!(!diff.identical);
        // zwei getrennte Änderungen → zwei Abschnitte
        assert_eq!(diff.hunks.len(), 2);
        let first = &diff.hunks[0].lines;
        assert_eq!(first[0].kind, "same");
        assert_eq!(first[1].kind, "removed");
        assert_eq!(first[2].kind, "added");
        assert_eq!(first[2].new_line, Some(2));
        // nur das geänderte Wort ist hervorgehoben
        let highlighted: Vec<&str> = first[2]
            .segments
            .iter()
            .filter(|segment| segment.changed)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(highlighted, vec!["neue"]);
        assert_eq!(
            diff.summary,
            DiffSummary {
                added_lines: 2,
                removed_lines: 1,
                added_words: 1 + 4,
                removed_words: 1,
            }
        );
        assert!(compare("a\nb", "a\nb", CONTEXT).identical);
    }

    #[test]
    fn unrelated_lines_are_not_compared_word_by_word() {
        let diff = compare("Völlig anderer Inhalt\n", "\\clearpage\n", CONTEXT);
        let lines = &diff.hunks[0].lines;
        assert!(lines.iter().all(|line| line.segments.len() == 1));
        assert_eq!(diff.summary.removed_words, 3);
        assert_eq!(
            diff.summary.added_words, 0,
            "Befehle zählen nicht als Wörter"
        );
    }

    #[test]
    fn counts_words_without_commands() {
        assert_eq!(
            word_count("\\textbf{Fett} und ü-Laute, 2024 \\cite{knuth}"),
            6
        );
    }
}
