use super::*;
use crate::cli::paint::render;

#[test]
fn the_overview_lists_every_command_and_option() {
    let text = render(&overview(), false);
    for sub in program::commands() {
        assert!(text.contains(sub.about), "{} is missing", sub.name);
    }
    for opt in program::ISLAND {
        assert!(text.contains(&opt.spelled()), "{} is missing", opt.long);
    }
    assert!(text.contains("For other programs\n  ai-hook <event>"), "{text}");
    assert!(text.contains("ISLAND_LOG"), "{text}");
}

#[test]
fn a_command_s_help_lists_what_it_takes() {
    let text = render(&command(&program::find("logs").unwrap()), false);
    assert!(text.contains("island logs [options]"), "{text}");
    assert!(text.contains("-n, --lines <count>"), "{text}");
    assert!(text.contains("error warn info debug trace"), "{text}");

    let text = render(&command(&program::find("completions").unwrap()), false);
    assert!(text.contains("Shells"), "{text}");
    assert!(text.contains("bash fish zsh"), "{text}");
}
