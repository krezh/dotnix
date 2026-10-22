//! `island --help` and the help for `island logs`, `island completions` and the features' own
//! commands; and the layout every help shares.

use super::{
    paint::{Line, Tone},
    program::{self, Opt, Sub, Takes},
};

/// Where descriptions start, counting the indent.
pub const COLUMN: usize = 32;

/// A command or option with what it does, the description lined up with the others.
pub fn row(left: Line, about: &str) -> Line {
    let left = Line::new().plain("  ").append(left);
    let left = if left.width() + 2 > COLUMN {
        left.plain("  ")
    } else {
        left.pad_to(COLUMN)
    };
    left.plain(about)
}

/// A list under a row's description, such as the values it takes.
pub fn under(text: impl Into<String>) -> Line {
    Line::new().pad_to(COLUMN).push(Tone::Faint, text)
}

pub fn title(name: &str, about: &str) -> Line {
    Line::new()
        .push(Tone::Strong, name)
        .plain("  ")
        .push(Tone::Muted, about)
}

pub fn usage(rest: Line) -> Line {
    Line::new()
        .push(Tone::Heading, "Usage")
        .plain("  ")
        .append(rest)
}

pub fn heading(text: &str) -> Line {
    Line::new().push(Tone::Heading, text)
}

pub fn help_row() -> Line {
    row(
        Line::new()
            .push(Tone::Command, "-h")
            .plain(", ")
            .push(Tone::Command, "--help"),
        "Show this help",
    )
}

fn option(opt: &Opt) -> Line {
    let mut line = Line::new();
    if let Some(short) = opt.short {
        line = line.push(Tone::Command, format!("-{short}")).plain(", ");
    }
    line = line.push(Tone::Command, opt.spelled());
    match opt.value {
        Some((name, _)) => line.plain(" ").push(Tone::Value, format!("<{name}>")),
        None => line,
    }
}

fn options(opts: &[Opt]) -> Vec<Line> {
    let mut lines = vec![heading("Options")];
    for opt in opts {
        lines.push(row(option(opt), opt.about));
        if let Some((_, program::Kind::OneOf(choices))) = opt.value {
            lines.push(under(choices.join(" ")));
        }
    }
    lines.push(help_row());
    lines
}

/// How a command reads after `island`, such as `logs [options]`.
fn signature(sub: &Sub) -> Line {
    let line = Line::new().push(Tone::Command, sub.name);
    match sub.takes {
        Takes::Ctl => line.plain(" ").push(Tone::Value, "<command>"),
        Takes::Options(_) => line.push(Tone::Faint, " [options]"),
        Takes::One(value, _) => line.plain(" ").push(Tone::Value, format!("<{value}>")),
    }
}

pub fn overview() -> Vec<Line> {
    let mut lines = vec![
        title("island", "A dynamic island status bar and notification daemon"),
        Line::new(),
        usage(
            Line::new()
                .push(Tone::Command, "island")
                .push(Tone::Faint, " [options]"),
        ),
        Line::new()
            .plain("       ")
            .push(Tone::Command, "island")
            .plain(" ")
            .push(Tone::Value, "<command>")
            .push(Tone::Faint, " ..."),
        Line::new(),
        heading("Commands"),
    ];
    let (programs, typed): (Vec<Sub>, Vec<Sub>) = program::commands()
        .into_iter()
        .partition(|sub| sub.for_programs);
    lines.extend(typed.iter().map(|sub| row(signature(sub), sub.about)));
    if !programs.is_empty() {
        lines.push(Line::new());
        lines.push(heading("For other programs"));
        lines.extend(programs.iter().map(|sub| row(signature(sub), sub.about)));
    }
    lines.push(Line::new());
    lines.extend(options(program::ISLAND));
    lines.extend([
        Line::new(),
        heading("Environment"),
        row(
            Line::new().push(Tone::Command, "ISLAND_LOG"),
            "What is logged, over the config's [log] level",
        ),
        under("such as debug, audio=debug,tray=trace or frames=debug"),
        Line::new(),
        Line::new().push(
            Tone::Faint,
            "One island opens on every output and serves org.freedesktop.Notifications.",
        ),
        Line::new()
            .push(Tone::Faint, "See ")
            .push(Tone::Command, "island <command> --help")
            .push(Tone::Faint, " for each command."),
    ]);
    lines
}

/// Help for one of `island`'s commands, other than `island ctl`, which has its own.
pub fn command(sub: &Sub) -> Vec<Line> {
    let path = format!("island {}", sub.name);
    let mut lines = vec![
        title(&path, sub.about),
        Line::new(),
        usage(
            Line::new()
                .push(Tone::Command, path.clone())
                .append(Line(signature(sub).0.into_iter().skip(1).collect())),
        ),
        Line::new(),
    ];
    match sub.takes {
        Takes::Options(opts) => lines.extend(options(opts)),
        Takes::One(value, values) => {
            let mut title = value.to_owned();
            if let Some(first) = title.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            lines.push(heading(&format!("{title}s")));
            lines.push(Line::new().plain("  ").plain(values.join(" ")));
        }
        Takes::Ctl => {}
    }
    if let Some(note) = sub.note {
        lines.push(Line::new());
        lines.push(Line::new().push(Tone::Faint, note));
    }
    lines
}

#[cfg(test)]
#[path = "help_test.rs"]
mod tests;
