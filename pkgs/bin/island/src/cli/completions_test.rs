use super::*;
use crate::cli::paint::render;

fn args(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn every_shell_s_script_asks_this_island() {
    for shell in SHELLS {
        let script = script(&args(shell)).unwrap();
        assert!(script.contains("__complete --"), "{shell}: {script}");
        assert!(!script.contains("ISLAND"), "{shell} still has the placeholder");
    }
}

#[test]
fn an_unknown_shell_is_named_with_the_ones_there_are() {
    let mistake = script(&args("fsh")).unwrap_err();
    let text = render(&mistake.lines(), false);
    assert!(text.contains("No shell fsh"), "{text}");
    assert!(text.contains("Did you mean fish?"), "{text}");
    assert!(text.contains("Shells: bash fish zsh"), "{text}");
}
