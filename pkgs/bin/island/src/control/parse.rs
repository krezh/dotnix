use super::spec::{self, Command, Form, Takes, Value};
use crate::cli::{Line, Mistake, Tone, either, listed};

const CTL: &str = "island ctl";

fn path(command: &Command) -> String {
    format!("{CTL} {}", command.name)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Step(f32),
    Set(f32),
    ToggleMute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    Toggle,
}

impl Switch {
    pub fn apply(self, current: bool) -> bool {
        match self {
            Switch::On => true,
            Switch::Off => false,
            Switch::Toggle => !current,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    /// A panel, and the view to open it on.
    Open(String, Option<String>),
    Toggle(String, Option<String>),
    Close,
    Log(String),
    Feature(&'static str, Vec<String>),
    /// What island is doing, for `island ctl` on its own.
    Status,
}

const STEP: f32 = 0.05;

pub fn change(args: &[&str], mute: bool) -> Result<Change, String> {
    match args {
        ["up"] => Ok(Change::Step(STEP)),
        ["down"] => Ok(Change::Step(-STEP)),
        ["up", amount] => percent(amount).map(Change::Step),
        ["down", amount] => percent(amount).map(|value| Change::Step(-value)),
        ["mute"] if mute => Ok(Change::ToggleMute),
        ["set", amount] => percent(amount).map(Change::Set),
        _ if mute => Err("expected up, down, set PERCENT or mute".to_owned()),
        _ => Err("expected up, down or set PERCENT".to_owned()),
    }
}

fn percent(text: &str) -> Result<f32, String> {
    text.trim_end_matches('%')
        .parse::<f32>()
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
        .map_err(|_| format!("'{text}' is not a percentage"))
}

pub fn switch(args: &[&str]) -> Result<Switch, String> {
    match args {
        ["on"] => Ok(Switch::On),
        ["off"] => Ok(Switch::Off),
        [] | ["toggle"] => Ok(Switch::Toggle),
        _ => Err("expected on, off or toggle".to_owned()),
    }
}

pub fn parse(args: &[String]) -> Result<Control, Mistake> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some((&name, rest)) = words.split_first() else {
        return Ok(Control::Status);
    };
    let Some((feature, command)) = spec::find(name) else {
        let names: Vec<&str> = spec::all().iter().map(|(_, command)| command.name).collect();
        return Err(
            Mistake::new(Line::new().plain("No command ").push(Tone::Strong, name))
                .did_you_mean(name, names)
                .see_help(CTL),
        );
    };
    check(command, rest)?;
    let owned = |index: usize| rest.get(index).map(|word| (*word).to_owned());
    Ok(match (feature, name) {
        (Some(feature), _) => Control::Feature(feature, args.to_vec()),
        (None, "open") => Control::Open(rest[0].to_owned(), owned(1)),
        (None, "toggle") => Control::Toggle(rest[0].to_owned(), owned(1)),
        (None, "close") => Control::Close,
        (None, _) => Control::Log(rest[0].to_owned()),
    })
}

fn too_much(command: &Command, after: String, extra: &[&str]) -> Mistake {
    Mistake::new(
        Line::new()
            .plain("Nothing goes after ")
            .push(Tone::Command, after)
            .push(Tone::Faint, ", got ")
            .push(Tone::Strong, extra.join(" ")),
    )
    .see_help(&path(command))
}

fn needs(command: &Command, what: impl Into<String>) -> Mistake {
    Mistake::new(
        Line::new()
            .push(Tone::Command, command.name)
            .plain(" needs ")
            .plain(what),
    )
}

fn check(command: &Command, rest: &[&str]) -> Result<(), Mistake> {
    let name = command.name;
    let help = path(command);
    match &command.takes {
        Takes::Nothing => match rest {
            [] => Ok(()),
            extra => Err(too_much(command, name.to_owned(), extra)),
        },
        Takes::Forms { forms, bare } => check_forms(command, forms, *bare, rest),
        Takes::Aimed { forms, bare, .. } => {
            let rest = match rest.split_first() {
                Some((first, after)) if !forms.iter().any(|form| form.word == *first) => after,
                _ => rest,
            };
            check_forms(command, forms, *bare, rest)
        }
        Takes::Panel => {
            let panels = spec::panels();
            let names: Vec<&str> = panels.iter().map(|(panel, _)| *panel).collect();
            let Some((&panel, view)) = rest.split_first() else {
                return Err(needs(command, "a panel").hint(listed("Panels", &names)));
            };
            let Some((_, views)) = panels.iter().find(|(known, _)| *known == panel) else {
                return Err(
                    Mistake::new(Line::new().plain("No panel ").push(Tone::Strong, panel))
                        .did_you_mean(panel, names.iter().copied())
                        .hint(listed("Panels", &names)),
                );
            };
            match view {
                [] => Ok(()),
                [view] if views.contains(view) => Ok(()),
                [_] if views.is_empty() => Err(Mistake::new(
                    Line::new()
                        .plain("The ")
                        .push(Tone::Strong, panel)
                        .plain(" panel has no views"),
                )),
                [view] => Err(Mistake::new(
                    Line::new()
                        .plain("The ")
                        .push(Tone::Strong, panel)
                        .plain(" panel has no view ")
                        .push(Tone::Strong, *view),
                )
                .did_you_mean(view, views.iter().copied())
                .hint(listed("Views", views))),
                [view, extra @ ..] => Err(too_much(command, format!("{name} {panel} {view}"), extra)),
            }
        }
        Takes::Word(what) => match rest {
            [_] => Ok(()),
            [] => Err(needs(command, format!("a {what}")).see_help(&help)),
            [word, extra @ ..] => Err(too_much(command, format!("{name} {word}"), extra)),
        },
    }
}

fn check_forms(
    command: &Command,
    forms: &[Form],
    bare: Option<&str>,
    rest: &[&str],
) -> Result<(), Mistake> {
    let name = command.name;
    let help = path(command);
    let words: Vec<&str> = forms.iter().map(|form| form.word).collect();
    let Some((&word, after)) = rest.split_first() else {
        return match bare {
            Some(_) => Ok(()),
            None => Err(needs(command, format!("one of {}", either(&words))).see_help(&help)),
        };
    };
    let Some(form) = forms.iter().find(|form| form.word == word) else {
        return Err(Mistake::new(
            Line::new()
                .push(Tone::Command, name)
                .plain(" has no ")
                .push(Tone::Strong, word),
        )
        .did_you_mean(word, words.iter().copied())
        .hint(
            Line::new()
                .push(Tone::Faint, "It takes ")
                .plain(either(&words)),
        ));
    };
    match (form.value, after) {
        (Value::Nothing | Value::MaybePercent, []) => Ok(()),
        (Value::Percent, []) => Err(Mistake::new(
            Line::new()
                .push(Tone::Command, format!("{name} {word}"))
                .plain(" needs a percentage"),
        )
        .hint(
            Line::new()
                .push(Tone::Faint, "Such as ")
                .push(Tone::Command, format!("island ctl {name} {word} 40")),
        )),
        (Value::Percent | Value::MaybePercent, [value]) => percent(value).map(drop).map_err(|_| {
            Mistake::new(
                Line::new()
                    .push(Tone::Strong, *value)
                    .plain(" is not a percentage"),
            )
            .hint(
                Line::new()
                    .push(Tone::Faint, "Give a number such as ")
                    .push(Tone::Value, "40")
                    .push(Tone::Faint, " or ")
                    .push(Tone::Value, "40%"),
            )
            .see_help(&help)
        }),
        (Value::Nothing, extra) => Err(too_much(command, format!("{name} {word}"), extra)),
        (_, [value, extra @ ..]) => Err(too_much(
            command,
            format!("{name} {word} {value}"),
            extra,
        )),
    }
}

#[cfg(test)]
#[path = "parse_test.rs"]
mod tests;
