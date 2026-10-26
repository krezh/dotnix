use super::*;
use crate::cli::{
    paint::render,
    program::{ISLAND, LOGS},
};

fn args(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

fn mistake(opts: &[Opt], words: &str, commands: &[&str]) -> String {
    let mistake = parse(opts, args(words), "island logs", commands).unwrap_err();
    render(&mistake.lines(), false)
}

#[test]
fn options_are_read_with_their_values_in_either_spelling() {
    let found = parse(
        LOGS,
        args("-n 20 --level=debug --no-follow"),
        "island logs",
        &[],
    )
    .unwrap();
    let found: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|(opt, value)| (opt.long, value.as_deref()))
        .collect();
    assert_eq!(
        found,
        [
            ("lines", Some("20")),
            ("level", Some("debug")),
            ("no-follow", None)
        ]
    );
}

#[test]
fn mistakes_name_the_option_and_what_it_takes() {
    let text = mistake(LOGS, "--levle debug", &[]);
    assert!(text.contains("No option --levle"), "{text}");
    assert!(text.contains("Did you mean --level?"), "{text}");

    let text = mistake(LOGS, "--level loud", &[]);
    assert!(text.contains("loud is not a level"), "{text}");
    assert!(
        text.contains("--level takes: error warn info debug trace"),
        "{text}"
    );

    let text = mistake(LOGS, "-n", &[]);
    assert!(text.contains("--lines needs a count"), "{text}");

    let text = mistake(LOGS, "-n many", &[]);
    assert!(text.contains("--lines needs a number, got many"), "{text}");

    let text = mistake(LOGS, "audio", &[]);
    assert!(
        text.contains("Nothing goes after island logs, got audio"),
        "{text}"
    );

    let text = mistake(ISLAND, "clt", &["ctl", "logs"]);
    assert!(text.contains("No command clt"), "{text}");
    assert!(text.contains("Did you mean ctl?"), "{text}");
}
