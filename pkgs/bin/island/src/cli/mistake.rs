//! What went wrong with a command, in a line, with a hint at what to type instead.

use super::paint::{Line, Tone};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mistake {
    pub what: Line,
    pub hints: Vec<Line>,
}

impl Mistake {
    pub fn new(what: Line) -> Self {
        Self {
            what,
            hints: Vec::new(),
        }
    }

    pub fn hint(mut self, hint: Line) -> Self {
        self.hints.push(hint);
        self
    }

    /// Points at the help for `path`, such as `island ctl volume`.
    pub fn see_help(self, path: &str) -> Self {
        self.hint(
            Line::new()
                .push(Tone::Faint, "See ")
                .push(Tone::Command, format!("{path} --help")),
        )
    }

    /// Suggests the closest of `known` to `typed`, if one is close enough to be a slip.
    pub fn did_you_mean<'a>(self, typed: &str, known: impl IntoIterator<Item = &'a str>) -> Self {
        match closest(typed, known) {
            Some(near) => self.hint(
                Line::new()
                    .push(Tone::Faint, "Did you mean ")
                    .push(Tone::Command, near)
                    .push(Tone::Faint, "?"),
            ),
            None => self,
        }
    }

    pub fn lines(&self) -> Vec<Line> {
        let mut lines = vec![Line::new().push(Tone::Bad, "✗ ").append(self.what.clone())];
        lines.extend(
            self.hints
                .iter()
                .map(|hint| Line::new().plain("  ").append(hint.clone())),
        );
        lines
    }

    /// One line without colour, such as for the running island to send back.
    pub fn plain(&self) -> String {
        self.what.render(false)
    }
}

pub fn not_running() -> Mistake {
    Mistake::new(Line::new().plain("island is not running")).hint(
        Line::new()
            .push(Tone::Faint, "Start it with ")
            .push(Tone::Command, "island"),
    )
}

/// `Panels: a b c`, for a hint.
pub fn listed(title: &str, words: &[&str]) -> Line {
    Line::new()
        .push(Tone::Faint, format!("{title}: "))
        .plain(words.join(" "))
}

/// `a, b or c`.
pub fn either(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

fn closest<'a>(typed: &str, known: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let length = typed.chars().count();
    // Two letters swapped count as two changes.
    let allowed = (length / 3).max(if length >= 3 { 2 } else { 1 });
    known
        .into_iter()
        .map(|word| (distance(typed, word), word))
        .filter(|(distance, _)| *distance <= allowed)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, word)| word)
}

/// How many letters to add, remove or change to turn one word into the other.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
#[path = "mistake_test.rs"]
mod tests;
