//! `island ctl --help`, and the help for one command, drawn from what the commands declare.

use super::spec::{self, Command, Form, Group, Takes, Value};
use crate::cli::{Line, Tone, heading, row, title, under, usage};

/// Longer choices show as a placeholder in the overview, listed under the description.
const CHOICES_INLINE: usize = 16;

fn value(value: Value) -> Line {
    match value {
        Value::Nothing => Line::new(),
        Value::Percent => Line::new().plain(" ").push(Tone::Value, "<percent>"),
        Value::MaybePercent => Line::new()
            .push(Tone::Faint, " [")
            .push(Tone::Value, "percent")
            .push(Tone::Faint, "]"),
    }
}

fn choices_inline(command: &Command) -> bool {
    match &command.takes {
        Takes::Forms { forms, .. } | Takes::Aimed { forms, .. } => {
            forms.iter().map(|form| form.word.len() + 1).sum::<usize>() <= CHOICES_INLINE + 1
        }
        _ => true,
    }
}

/// Such as ` [monitor]`, for the word an aimed command may take first.
fn aimed_at(at: &str) -> Line {
    Line::new()
        .push(Tone::Faint, " [")
        .push(Tone::Value, at)
        .push(Tone::Faint, "]")
}

fn forms_signature(command: &Command, forms: &[Form], bare: Option<&str>) -> Line {
    let line = Line::new();
    match (forms, bare) {
        ([only], None) => line
            .plain(" ")
            .push(Tone::Command, only.word)
            .append(value(only.value)),
        _ => {
            let (open, close) = if bare.is_some() { ("[", "]") } else { ("<", ">") };
            if choices_inline(command) {
                let words: Vec<&str> = forms.iter().map(|form| form.word).collect();
                line.plain(" ")
                    .push(Tone::Faint, format!("{open}{}{close}", words.join("|")))
            } else if bare.is_some() {
                line.push(Tone::Faint, " [")
                    .push(Tone::Value, "action")
                    .push(Tone::Faint, "]")
            } else {
                line.plain(" ").push(Tone::Value, "<action>")
            }
        }
    }
}

/// How a command reads in the overview, such as `volume [up|down|set|mute]`.
fn signature(command: &Command) -> Line {
    let line = Line::new().push(Tone::Command, command.name);
    match &command.takes {
        Takes::Nothing => line,
        Takes::Forms { forms, bare } => line.append(forms_signature(command, forms, *bare)),
        Takes::Aimed { at, forms, bare } => line
            .append(aimed_at(at))
            .append(forms_signature(command, forms, *bare)),
        Takes::Panel => line
            .plain(" ")
            .push(Tone::Value, "<panel>")
            .push(Tone::Faint, " [view]"),
        Takes::Word(what) => line.plain(" ").push(Tone::Value, format!("<{what}>")),
    }
}

/// What the overview lists under a command's description, such as the panels.
fn listed(command: &Command) -> Option<String> {
    match &command.takes {
        Takes::Forms { forms, .. } | Takes::Aimed { forms, .. } if !choices_inline(command) => Some(
            forms
                .iter()
                .map(|form| form.word)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        Takes::Panel => Some(
            spec::panels()
                .iter()
                .map(|(panel, _)| *panel)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

/// Under the overview of what island is doing.
pub fn more() -> Line {
    Line::new()
        .plain("  ")
        .push(Tone::Faint, "Run ")
        .push(Tone::Command, "island ctl --help")
        .push(Tone::Faint, " for every command.")
}

pub fn overview() -> Vec<Line> {
    let mut lines = vec![
        title("island ctl", "Control the running island"),
        Line::new(),
        usage(
            Line::new()
                .push(Tone::Command, "island ctl")
                .plain(" ")
                .push(Tone::Value, "<command>")
                .push(Tone::Faint, " [args] [--json]"),
        ),
        Line::new()
            .plain("       ")
            .push(Tone::Command, "island ctl")
            .push(Tone::Faint, "  shows what island is doing"),
    ];
    let all = spec::all();
    for group in Group::ALL {
        // Features' commands first, then island's own.
        let commands: Vec<&Command> = all
            .iter()
            .filter(|(feature, _)| feature.is_some() || group == Group::Panels)
            .chain(all.iter().filter(|(feature, _)| feature.is_none() && group != Group::Panels))
            .map(|(_, command)| *command)
            .filter(|command| command.group == group)
            .collect();
        if commands.is_empty() {
            continue;
        }
        lines.push(Line::new());
        lines.push(heading(group.title()));
        let mut last = None;
        for command in commands {
            lines.push(row(signature(command), command.about));
            let listed = listed(command);
            // A list the command above already showed, such as the panels, is not repeated.
            if let Some(listed) = listed.as_ref().filter(|listed| last.as_ref() != Some(*listed)) {
                lines.push(under(listed.clone()));
            }
            last = listed;
        }
    }
    lines.push(Line::new());
    lines.push(heading("Options"));
    lines.push(row(
        Line::new().push(Tone::Command, "--json"),
        "Print what island answers as JSON",
    ));
    lines.push(row(
        Line::new()
            .push(Tone::Command, "-h")
            .plain(", ")
            .push(Tone::Command, "--help"),
        "Help for island ctl, or for one command",
    ));
    lines
}

/// What follows the command's name in its usage, such as `[up|down [percent] | set <percent>]`.
fn forms_usage(forms: &[Form], bare: Option<&str>) -> Line {
    let spaced = forms.iter().any(|form| form.value != Value::Nothing);
    let mut line = Line::new().plain(" ");
    let wrap = bare.is_some() || forms.len() > 1;
    let (open, close) = if bare.is_some() { ("[", "]") } else { ("<", ">") };
    if wrap {
        line = line.push(Tone::Faint, open);
    }
    for (index, form) in forms.iter().enumerate() {
        if index > 0 {
            line = line.push(Tone::Faint, if spaced { " | " } else { "|" });
        }
        line = line.push(Tone::Command, form.word).append(value(form.value));
    }
    if wrap {
        line = line.push(Tone::Faint, close);
    }
    line
}

fn takes(command: &Command) -> Line {
    match &command.takes {
        Takes::Nothing => Line::new(),
        Takes::Forms { forms, bare } => forms_usage(forms, *bare),
        Takes::Aimed { at, forms, bare } => aimed_at(at).append(forms_usage(forms, *bare)),
        _ => {
            let signature = signature(command);
            Line(signature.0.into_iter().skip(1).collect())
        }
    }
}

fn form_rows<'a>(
    forms: &'a [Form],
    bare: Option<&'a str>,
) -> impl Iterator<Item = (Line, String)> + 'a {
    bare.map(|about| (Line::new().push(Tone::Faint, "(nothing)"), about.to_owned()))
        .into_iter()
        .chain(forms.iter().map(|form| {
            (
                Line::new().push(Tone::Command, form.word).append(value(form.value)),
                form.about.to_owned(),
            )
        }))
}

/// Help for one command: every form it takes, what each does, and examples.
pub fn command(command: &Command) -> Vec<Line> {
    let name = format!("island ctl {}", command.name);
    let mut lines = vec![
        title(&name, command.about),
        Line::new(),
        usage(Line::new().push(Tone::Command, name).append(takes(command))),
    ];
    let rows: Vec<(Line, String)> = match &command.takes {
        Takes::Forms { forms, bare } => form_rows(forms, *bare).collect(),
        Takes::Aimed { at, forms, bare } => std::iter::once((
            Line::new().push(Tone::Value, *at),
            format!("Which {at}; all of them when left out"),
        ))
        .chain(form_rows(forms, *bare))
        .collect(),
        Takes::Panel => spec::panels()
            .into_iter()
            .map(|(panel, views)| (Line::new().push(Tone::Command, panel), views.join(" ")))
            .collect(),
        Takes::Nothing | Takes::Word(_) => Vec::new(),
    };
    if !rows.is_empty() {
        let width = rows.iter().map(|(left, _)| left.width()).max().unwrap_or(0) + 4;
        let faint = matches!(command.takes, Takes::Panel);
        lines.push(Line::new());
        if faint {
            lines.push(
                Line::new()
                    .plain("  ")
                    .push(Tone::Faint, "panel")
                    .pad_to(width + 2)
                    .push(Tone::Faint, "views"),
            );
        }
        for (left, about) in rows {
            let about = if faint {
                Line::new().push(Tone::Faint, about)
            } else {
                Line::new().plain(about)
            };
            lines.push(Line::new().plain("  ").append(left).pad_to(width + 2).append(about));
        }
    }
    if !command.examples.is_empty() {
        lines.push(Line::new());
        lines.push(heading("Examples"));
        lines.extend(
            command
                .examples
                .iter()
                .map(|example| Line::new().plain("  ").push(Tone::Command, *example)),
        );
    }
    lines
}

#[cfg(test)]
#[path = "help_test.rs"]
mod tests;
