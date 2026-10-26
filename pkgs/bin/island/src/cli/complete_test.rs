use super::*;

fn next(line: &str) -> Vec<String> {
    let mut words: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    if line.ends_with(' ') || line.is_empty() {
        words.push(String::new());
    }
    match candidates(&words) {
        Answer::Words(words) => words.into_iter().map(|(word, _)| word).collect(),
        Answer::Files => vec!["<files>".to_owned()],
    }
}

#[test]
fn a_panel_s_views_complete_after_it_even_when_it_shares_a_command_s_name() {
    assert_eq!(next("ctl toggle ai "), ["sessions", "chat", "usage"]);
    assert_eq!(next("ctl open ai c"), ["chat"]);
    assert!(next("ctl toggle media ").is_empty());
    assert!(next("ctl toggle ").contains(&"launcher".to_owned()));
}

#[test]
fn each_level_offers_what_can_go_there() {
    assert_eq!(next("ctl tog"), ["toggle"]);
    assert_eq!(next("ctl volume "), ["up", "down", "set", "mute"]);
    assert!(next("ctl volume up ").is_empty());
    assert_eq!(next("ctl --json dn"), ["dnd"]);
    assert_eq!(next("completions "), ["bash", "fish", "zsh"]);
}

#[test]
fn options_and_their_values_complete() {
    assert_eq!(
        next("logs --level "),
        ["error", "warn", "info", "debug", "trace"]
    );
    assert!(next("logs -n 20 --").contains(&"--no-follow".to_owned()));
    assert!(next("logs --module ").contains(&"audio".to_owned()));
    assert_eq!(next("--config "), ["<files>"]);
    assert!(next("--opacity 0.9 ").contains(&"ctl".to_owned()));
}

#[test]
fn what_other_programs_run_is_not_offered() {
    let top = next("");
    assert!(top.contains(&"ctl".to_owned()) && top.contains(&"logs".to_owned()));
    assert!(!top.contains(&"ai-hook".to_owned()));
}
