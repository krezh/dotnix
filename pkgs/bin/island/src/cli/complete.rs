//! What can come next on an `island` command line, worked out from the same descriptions its help
//! and checks come from, for the shells' completion scripts to ask.

use super::program::{self, Kind, Opt, Sub, Takes};
use crate::control::{self, Command};

#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    /// Each with what it does, or empty.
    Words(Vec<(String, String)>),
    Files,
}

/// Where on the command line the next word goes.
enum At {
    Island,
    Ctl,
    Forms(&'static Command),
    Panel,
    Views(&'static [&'static str]),
    Options(&'static [Opt]),
    One(&'static [&'static str]),
    /// After an option that takes this.
    OptionValue(Kind),
    /// Nothing more to offer, such as after a percentage the caller types.
    Done,
}

fn word(word: &str, about: &str) -> (String, String) {
    (word.to_owned(), about.to_owned())
}

fn options(opts: &[Opt]) -> Vec<(String, String)> {
    opts.iter()
        .map(|opt| word(&opt.spelled(), opt.about))
        .chain([word("--help", "Show the help")])
        .collect()
}

fn find_option<'a>(opts: &'a [Opt], token: &str) -> Option<&'a Opt> {
    opts.iter().find(|opt| {
        token == opt.spelled() || opt.short.is_some_and(|short| token == format!("-{short}"))
    })
}

/// After `token`, an option of `opts` or a word that is not one: where the next word goes, or
/// `None` when `token` is not an option.
fn after_option(opts: &[Opt], token: &str, here: At) -> Option<At> {
    if !token.starts_with('-') {
        return None;
    }
    Some(match find_option(opts, token).and_then(|opt| opt.value) {
        Some((_, kind)) if !token.contains('=') => At::OptionValue(kind),
        _ => here,
    })
}

fn step(at: At, token: &str, resume: &mut Option<At>) -> At {
    match at {
        At::Island => {
            if let Some(next) = after_option(program::ISLAND, token, At::Island) {
                if matches!(next, At::OptionValue(_)) {
                    *resume = Some(At::Island);
                }
                return next;
            }
            match program::find(token).filter(|sub| !sub.for_programs) {
                Some(Sub {
                    takes: Takes::Ctl, ..
                }) => At::Ctl,
                Some(Sub {
                    takes: Takes::Options(opts),
                    ..
                }) => At::Options(opts),
                Some(Sub {
                    takes: Takes::One(_, values),
                    ..
                }) => At::One(values),
                None => At::Done,
            }
        }
        At::Ctl if token.starts_with('-') => At::Ctl,
        At::Ctl => match control::commands()
            .into_iter()
            .find(|(_, command)| command.name == token)
        {
            Some((_, command)) => match command.takes {
                control::Takes::Forms { .. } => At::Forms(command),
                control::Takes::Panel => At::Panel,
                control::Takes::Nothing | control::Takes::Word(_) => At::Done,
            },
            None => At::Done,
        },
        // A percentage, if one follows, is the caller's to type.
        At::Forms(_) => At::Done,
        At::Panel => At::Views(
            control::panels()
                .into_iter()
                .find(|(panel, _)| *panel == token)
                .map_or(&[][..], |(_, views)| views),
        ),
        At::Options(opts) => match after_option(opts, token, At::Options(opts)) {
            Some(At::OptionValue(kind)) => {
                *resume = Some(At::Options(opts));
                At::OptionValue(kind)
            }
            Some(next) => next,
            None => At::Done,
        },
        At::OptionValue(_) => resume.take().unwrap_or(At::Done),
        At::Views(_) | At::One(_) | At::Done => At::Done,
    }
}

fn offered(at: At) -> Answer {
    let words = match at {
        At::Island => program::commands()
            .iter()
            .filter(|sub| !sub.for_programs)
            .map(|sub| word(sub.name, sub.about))
            .chain(options(program::ISLAND))
            .collect(),
        At::Ctl => control::commands()
            .iter()
            .map(|(_, command)| word(command.name, command.about))
            .chain([
                word("--json", "Print what island answers as JSON"),
                word("--help", "Show the help"),
            ])
            .collect(),
        At::Forms(command) => match &command.takes {
            control::Takes::Forms { forms, .. } => forms
                .iter()
                .map(|form| word(form.word, form.about))
                .collect(),
            _ => Vec::new(),
        },
        At::Panel => control::panels()
            .into_iter()
            .map(|(panel, views)| word(panel, &views.join(" ")))
            .collect(),
        At::Views(views) | At::One(views) => views.iter().map(|view| word(view, "")).collect(),
        At::Options(opts) => options(opts),
        At::OptionValue(Kind::Path) => return Answer::Files,
        At::OptionValue(Kind::OneOf(values)) => {
            values.iter().map(|value| word(value, "")).collect()
        }
        At::OptionValue(Kind::Suggested(suggest)) => {
            suggest().into_iter().map(|value| word(value, "")).collect()
        }
        At::OptionValue(Kind::Number | Kind::Text) | At::Done => Vec::new(),
    };
    Answer::Words(words)
}

/// What can follow the words after `island`, the last of which is the one being typed.
pub fn candidates(words: &[String]) -> Answer {
    let (current, before) = match words.split_last() {
        Some((current, before)) => (current.as_str(), before),
        None => ("", words),
    };
    let mut resume = None;
    let at = before
        .iter()
        .fold(At::Island, |at, token| step(at, token, &mut resume));
    match offered(at) {
        Answer::Words(words) => Answer::Words(
            words
                .into_iter()
                .filter(|(word, _)| word.starts_with(current))
                .collect(),
        ),
        Answer::Files => Answer::Files,
    }
}

/// `island __complete -- WORDS`: a line for each word, a tab and what it does, or `<files>` for
/// the shell to complete a path itself.
pub fn print(words: &[String]) {
    match candidates(words) {
        Answer::Files => println!("<files>"),
        Answer::Words(words) => {
            for (word, about) in words {
                println!("{word}\t{about}");
            }
        }
    }
}

#[cfg(test)]
#[path = "complete_test.rs"]
mod tests;
