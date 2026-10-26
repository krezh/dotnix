//! Reads options such as `--level debug` or `-n 20`, and says plainly what is wrong with them.

use super::{
    mistake::{Mistake, listed},
    paint::{Line, Tone},
    program::{Kind, Opt},
};

/// The options found in `args`, each with its value. `path` names the command for help, such as
/// `island logs`; `commands` are the words that could have been meant by a bare one.
pub fn parse<'a>(
    opts: &'a [Opt],
    args: impl IntoIterator<Item = String>,
    path: &str,
    commands: &[&str],
) -> Result<Vec<(&'a Opt, Option<String>)>, Mistake> {
    let mut found = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (spelled, inline) = match arg.split_once('=') {
            Some((name, value)) if arg.starts_with("--") => {
                (name.to_owned(), Some(value.to_owned()))
            }
            _ => (arg.clone(), None),
        };
        let opt = opts.iter().find(|opt| {
            spelled == opt.spelled()
                || opt
                    .short
                    .is_some_and(|short| spelled == format!("-{short}"))
        });
        let Some(opt) = opt else {
            return Err(unknown(opts, &arg, path, commands));
        };
        let value = match opt.value {
            None if inline.is_some() => {
                return Err(Mistake::new(
                    Line::new()
                        .push(Tone::Command, opt.spelled())
                        .plain(" takes no value"),
                )
                .see_help(path));
            }
            None => None,
            Some((name, kind)) => {
                let Some(value) = inline.or_else(|| args.next()) else {
                    return Err(Mistake::new(
                        Line::new()
                            .push(Tone::Command, opt.spelled())
                            .plain(" needs a ")
                            .push(Tone::Value, name),
                    )
                    .see_help(path));
                };
                check(opt, name, kind, &value)?;
                Some(value)
            }
        };
        found.push((opt, value));
    }
    Ok(found)
}

fn check(opt: &Opt, name: &str, kind: Kind, value: &str) -> Result<(), Mistake> {
    match kind {
        Kind::OneOf(choices) if !choices.contains(&value) => Err(Mistake::new(
            Line::new()
                .push(Tone::Strong, value)
                .plain(format!(" is not a {name}")),
        )
        .did_you_mean(value, choices.iter().copied())
        .hint(listed(&format!("{} takes", opt.spelled()), choices))),
        Kind::Number if value.parse::<f64>().is_err() => Err(Mistake::new(
            Line::new()
                .push(Tone::Command, opt.spelled())
                .plain(" needs a number, got ")
                .push(Tone::Strong, value),
        )),
        _ => Ok(()),
    }
}

fn unknown(opts: &[Opt], arg: &str, path: &str, commands: &[&str]) -> Mistake {
    if arg.starts_with('-') {
        let spelled: Vec<String> = opts.iter().map(Opt::spelled).collect();
        Mistake::new(Line::new().plain("No option ").push(Tone::Strong, arg))
            .did_you_mean(arg, spelled.iter().map(String::as_str))
            .see_help(path)
    } else if commands.is_empty() {
        Mistake::new(
            Line::new()
                .plain("Nothing goes after ")
                .push(Tone::Command, path)
                .push(Tone::Faint, ", got ")
                .push(Tone::Strong, arg),
        )
        .see_help(path)
    } else {
        Mistake::new(Line::new().plain("No command ").push(Tone::Strong, arg))
            .did_you_mean(arg, commands.iter().copied())
            .see_help(path)
    }
}

#[cfg(test)]
#[path = "options_test.rs"]
mod tests;
