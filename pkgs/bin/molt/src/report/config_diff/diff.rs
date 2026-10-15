//! The line diff behind a changed file's unified diff: an edit script aligned by longest common
//! subsequence, grouped into hunks with context.

use std::collections::VecDeque;

use super::{DiffLine, DiffLineKind};

/// Line-pair budget for the exact diff of a file's changed middle; beyond it the middle is shown
/// as removed, then added.
const MAX_DIFF_CELLS: usize = 4_000_000;
/// Diff lines kept per file.
const MAX_DIFF_LINES: usize = 4000;
const CONTEXT: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Equal,
    Removed,
    Added,
}

/// A unified diff with `CONTEXT` lines around each change, and the added and removed line counts.
pub(super) fn line_diff(old: &str, new: &str) -> (Vec<DiffLine>, usize, usize) {
    let old = old.lines().collect::<Vec<_>>();
    let new = new.lines().collect::<Vec<_>>();
    let mut output = Output::default();
    let (mut added, mut removed) = (0, 0);
    // Line numbers of the next old and new line.
    let (mut old_line, mut new_line) = (1, 1);
    let mut hunk: Option<Hunk> = None;
    // Equal lines since the last change, and the latest `CONTEXT` of them not yet shown.
    let mut gap = 0;
    let mut pending = VecDeque::with_capacity(CONTEXT + 1);
    for (op, text) in edit_script(&old, &new) {
        if op == Op::Equal {
            gap += 1;
            old_line += 1;
            new_line += 1;
            if let Some(open) = &mut hunk
                && gap <= CONTEXT
            {
                output.push(DiffLineKind::Context, text);
                open.old_count += 1;
                open.new_count += 1;
                continue;
            }
            if hunk.is_some() && gap > 2 * CONTEXT {
                output.close(hunk.take());
            }
            pending.push_back(text);
            if pending.len() > CONTEXT {
                pending.pop_front();
            }
            continue;
        }

        let open = hunk.get_or_insert_with(|| Hunk {
            header: output.push(DiffLineKind::Hunk, ""),
            old_start: old_line - pending.len(),
            new_start: new_line - pending.len(),
            old_count: 0,
            new_count: 0,
        });
        for text in pending.drain(..) {
            output.push(DiffLineKind::Context, text);
            open.old_count += 1;
            open.new_count += 1;
        }
        gap = 0;
        if op == Op::Removed {
            output.push(DiffLineKind::Removed, text);
            open.old_count += 1;
            old_line += 1;
            removed += 1;
        } else {
            output.push(DiffLineKind::Added, text);
            open.new_count += 1;
            new_line += 1;
            added += 1;
        }
    }
    if added + removed == 0 {
        let note = "Only line endings or the final newline differ";
        return (
            vec![DiffLine {
                kind: DiffLineKind::Hunk,
                text: note.to_owned(),
            }],
            0,
            0,
        );
    }
    output.close(hunk);

    let Output { mut lines, hidden } = output;
    if hidden > 0 {
        lines.push(DiffLine {
            kind: DiffLineKind::Hunk,
            text: format!("… {hidden} more diff lines not shown"),
        });
    }
    (lines, added, removed)
}

/// The open hunk: where it starts, how many old and new lines it spans so far, and its header's
/// index in the output when shown.
struct Hunk {
    header: Option<usize>,
    old_start: usize,
    new_start: usize,
    old_count: usize,
    new_count: usize,
}

/// Diff lines up to `MAX_DIFF_LINES`, and how many more there were.
#[derive(Default)]
struct Output {
    lines: Vec<DiffLine>,
    hidden: usize,
}

impl Output {
    /// Appends a line, or counts it as hidden once full; returns its index when appended.
    fn push(&mut self, kind: DiffLineKind, text: &str) -> Option<usize> {
        if self.lines.len() == MAX_DIFF_LINES {
            self.hidden += 1;
            return None;
        }
        self.lines.push(DiffLine {
            kind,
            text: text.to_owned(),
        });
        Some(self.lines.len() - 1)
    }

    /// Fills in a finished hunk's header.
    fn close(&mut self, hunk: Option<Hunk>) {
        let Some(hunk) = hunk else {
            return;
        };
        let Some(header) = hunk.header else {
            return;
        };
        let (old_count, new_count) = (hunk.old_count, hunk.new_count);
        self.lines[header].text = format!(
            "@@ -{},{old_count} +{},{new_count} @@",
            if old_count == 0 {
                hunk.old_start - 1
            } else {
                hunk.old_start
            },
            if new_count == 0 {
                hunk.new_start - 1
            } else {
                hunk.new_start
            },
        );
    }
}

/// Lines in order with their fate, produced lazily: the common prefix and suffix are kept as is,
/// and the middle is aligned by a longest-common-subsequence table when it fits `MAX_DIFF_CELLS`.
fn edit_script<'a>(
    old: &[&'a str],
    new: &[&'a str],
) -> impl Iterator<Item = (Op, &'a str)> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let old_middle = &old[prefix..old.len() - suffix];
    let new_middle = &new[prefix..new.len() - suffix];

    let width = new_middle.len() + 1;
    let table = (old_middle.len().saturating_mul(new_middle.len()) <= MAX_DIFF_CELLS).then(|| {
        // `table[i * width + j]`: common-subsequence length of old_middle[i..] and new_middle[j..].
        let mut table = vec![0_u32; (old_middle.len() + 1) * width];
        for i in (0..old_middle.len()).rev() {
            for j in (0..new_middle.len()).rev() {
                table[i * width + j] = if old_middle[i] == new_middle[j] {
                    table[(i + 1) * width + j + 1] + 1
                } else {
                    table[(i + 1) * width + j].max(table[i * width + j + 1])
                };
            }
        }
        table
    });
    // Without a table the middle is all removed, then all added.
    let (mut i, mut j) = (0, 0);
    let middle = std::iter::from_fn(move || {
        let op = match (old_middle.get(i), new_middle.get(j), &table) {
            (None, None, _) => return None,
            (Some(old), Some(new), Some(_)) if old == new => Op::Equal,
            (Some(_), Some(_), Some(table))
                if table[(i + 1) * width + j] < table[i * width + j + 1] =>
            {
                Op::Added
            }
            (Some(_), _, _) => Op::Removed,
            (None, Some(_), _) => Op::Added,
        };
        Some(match op {
            Op::Equal => {
                i += 1;
                j += 1;
                (op, old_middle[i - 1])
            }
            Op::Removed => {
                i += 1;
                (op, old_middle[i - 1])
            }
            Op::Added => {
                j += 1;
                (op, new_middle[j - 1])
            }
        })
    });
    old[..prefix]
        .iter()
        .map(|line| (Op::Equal, *line))
        .chain(middle)
        .chain(old[old.len() - suffix..].iter().map(|line| (Op::Equal, *line)))
}

#[cfg(test)]
mod tests {
    use crate::report::config_diff::tests::texts;

    use super::*;

    #[test]
    fn diffs_with_context_and_separate_hunks() {
        let old = (1..=20).map(|n| format!("line {n}\n")).collect::<String>();
        let new = old
            .replace("line 2\n", "line two\n")
            .replace("line 18\n", "line 18\nline 18b\n");
        let (lines, added, removed) = line_diff(&old, &new);
        assert_eq!((added, removed), (2, 1));
        assert_eq!(
            texts(&lines),
            [
                "@@ -1,5 +1,5 @@",
                " line 1",
                "-line 2",
                "+line two",
                " line 3",
                " line 4",
                " line 5",
                "@@ -16,5 +16,6 @@",
                " line 16",
                " line 17",
                " line 18",
                "+line 18b",
                " line 19",
                " line 20",
            ]
        );
        let (lines, added, removed) = line_diff("", "a\nb\n");
        assert_eq!((added, removed), (2, 0));
        assert_eq!(texts(&lines), ["@@ -0,0 +1,2 @@", "+a", "+b"]);
    }

    #[test]
    fn truncates_huge_insertion_with_full_counts() {
        let inserted = 100_000;
        let new = (0..inserted).map(|n| format!("{n}\n")).collect::<String>();
        let (lines, added, removed) = line_diff("start\nend\n", &format!("start\n{new}end\n"));
        assert_eq!((added, removed), (inserted, 0));
        assert_eq!(lines.len(), MAX_DIFF_LINES + 1);
        let texts = texts(&lines);
        assert_eq!(texts[..3], ["@@ -1,2 +1,100002 @@", " start", "+0"]);
        // The header and two context lines join the insertions in the full diff.
        let hidden = 3 + inserted - MAX_DIFF_LINES;
        assert_eq!(
            texts.last().unwrap(),
            &format!("… {hidden} more diff lines not shown")
        );
    }
}
