//! How `island ctl` prints what the running island answered.

use super::report::{Report, Row, Shown};
use crate::cli::{Line, Tone};

const BAR: usize = 20;
/// The least room labels get, so short reports line up with longer ones.
const LABEL: usize = 14;

fn level(value: f32, bars: bool) -> Line {
    let value = value.clamp(0.0, 1.0);
    let percent = Line::new().push(Tone::Strong, format!("{:>3}%", (value * 100.0).round()));
    if !bars {
        return percent;
    }
    let filled = (value * BAR as f32).round() as usize;
    Line::new()
        .push(Tone::Command, "━".repeat(filled))
        .push(Tone::Faint, "━".repeat(BAR - filled))
        .plain("  ")
        .append(percent)
}

fn value(row: &Row, bars: bool) -> Line {
    let line = match &row.value {
        Shown::Switch(true) => Line::new().push(Tone::Good, "on"),
        Shown::Switch(false) => Line::new().push(Tone::Faint, "off"),
        Shown::Level { value, muted } => {
            let line = level(*value, bars);
            if *muted {
                line.plain("  ").push(Tone::Warning, "muted")
            } else {
                line
            }
        }
        Shown::Count(count) => Line::new().push(Tone::Strong, count.to_string()),
        Shown::Text(text) => Line::new().plain(text.clone()),
        Shown::Unknown => Line::new().push(Tone::Faint, "not known yet"),
        Shown::Nothing => Line::new(),
    };
    if row.detail.is_empty() {
        line
    } else {
        line.plain("  ").push(Tone::Faint, row.detail.clone())
    }
}

/// `bars` draws levels as bars, for a terminal; without, they are plain numbers.
pub fn lines(report: &Report, bars: bool) -> Vec<Line> {
    let mut lines = Vec::new();
    if let Some(title) = &report.title {
        let (name, rest) = title.split_once(' ').unwrap_or((title, ""));
        lines.push(
            Line::new()
                .plain("  ")
                .push(Tone::Strong, name)
                .plain(" ")
                .push(Tone::Faint, rest),
        );
        lines.push(Line::new());
    }
    let width = report
        .rows
        .iter()
        .filter(|row| row.value != Shown::Nothing)
        .map(|row| row.label.chars().count() + 3)
        .max()
        .unwrap_or(0)
        .max(LABEL);
    for (index, row) in report.rows.iter().enumerate() {
        let done = report.changed && index == 0;
        let start = if done {
            Line::new().push(Tone::Good, "✓ ")
        } else {
            Line::new().plain("  ")
        };
        let label = if done { Tone::Plain } else { Tone::Muted };
        let line = start.push(label, row.label.clone());
        lines.push(if row.value == Shown::Nothing {
            line
        } else {
            line.pad_to(width + 2).append(value(row, bars))
        });
    }
    lines
}

#[cfg(test)]
#[path = "show_test.rs"]
mod tests;
