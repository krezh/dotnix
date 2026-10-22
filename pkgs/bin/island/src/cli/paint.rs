//! The command line's colours, island's own, and plain text when they would not reach a terminal.

use std::{fmt::Write, io::IsTerminal};

use crate::theme::{BLUE, GREEN, MAUVE, OVERLAY1, PEACH, RED, SUBTEXT1, YELLOW};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Strong,
    /// A command or word to type.
    Command,
    /// A value of the caller's to put in, such as `<percent>`.
    Value,
    Faint,
    Muted,
    Heading,
    Bad,
    Warning,
    Good,
    /// Such as a log entry's fields.
    Italic,
}

impl Tone {
    fn sgr(self) -> Option<String> {
        let rgb = |color: u32, bold: bool| {
            let (r, g, b) = (color >> 16 & 0xff, color >> 8 & 0xff, color & 0xff);
            let bold = if bold { "1;" } else { "" };
            Some(format!("{bold}38;2;{r};{g};{b}"))
        };
        match self {
            Tone::Plain => None,
            Tone::Strong => Some("1".to_owned()),
            Tone::Command => rgb(BLUE, false),
            Tone::Value => rgb(PEACH, false),
            Tone::Faint => rgb(OVERLAY1, false),
            Tone::Muted => rgb(SUBTEXT1, false),
            Tone::Heading => rgb(MAUVE, true),
            Tone::Bad => rgb(RED, true),
            Tone::Warning => rgb(YELLOW, true),
            Tone::Good => rgb(GREEN, true),
            Tone::Italic => rgb(SUBTEXT1, false).map(|sgr| format!("3;{sgr}")),
        }
    }
}

/// Text in parts, each with its tone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line(pub Vec<(Tone, String)>);

impl Line {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, tone: Tone, text: impl Into<String>) -> Self {
        self.0.push((tone, text.into()));
        self
    }

    pub fn plain(self, text: impl Into<String>) -> Self {
        self.push(Tone::Plain, text)
    }

    pub fn width(&self) -> usize {
        self.0.iter().map(|(_, text)| text.chars().count()).sum()
    }

    pub fn pad_to(self, width: usize) -> Self {
        let pad = width.saturating_sub(self.width());
        self.plain(" ".repeat(pad))
    }

    pub fn append(mut self, other: Line) -> Self {
        self.0.extend(other.0);
        self
    }

    pub fn render(&self, colour: bool) -> String {
        let mut out = String::new();
        for (tone, text) in &self.0 {
            match tone.sgr().filter(|_| colour) {
                Some(sgr) => {
                    let _ = write!(out, "\x1b[{sgr}m{text}\x1b[0m");
                }
                None => out.push_str(text),
            }
        }
        out
    }
}

/// Whether to colour what goes to standard output or standard error, as `NO_COLOR` and the
/// terminal allow.
pub fn colour(stderr: bool) -> bool {
    let terminal = if stderr {
        std::io::stderr().is_terminal()
    } else {
        std::io::stdout().is_terminal()
    };
    terminal && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}

pub fn render(lines: &[Line], colour: bool) -> String {
    lines
        .iter()
        .map(|line| line.render(colour).trim_end().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
