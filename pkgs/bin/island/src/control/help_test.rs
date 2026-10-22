use super::*;
use crate::cli::render;

#[test]
fn the_overview_lists_every_command_and_every_panel() {
    let text = render(&overview(), false);
    for (_, command) in spec::all() {
        assert!(text.contains(command.about), "{} is missing", command.name);
    }
    for (panel, _) in spec::panels() {
        assert!(text.contains(panel), "the {panel} panel is missing");
    }
    assert!(text.contains("dnd [on|off|toggle]"), "{text}");
    assert!(text.contains("session <action>"), "{text}");
}

#[test]
fn a_command_s_help_shows_each_form_and_its_examples() {
    let (_, volume) = spec::find("volume").unwrap();
    let text = render(&command(volume), false);
    assert!(
        text.contains("island ctl volume [up [percent] | down [percent] | set <percent> | mute]"),
        "{text}"
    );
    assert!(text.contains("Raise it, by 5% unless given"), "{text}");
    assert!(text.contains("Examples"), "{text}");

    let (_, dnd) = spec::find("dnd").unwrap();
    let text = render(&command(dnd), false);
    assert!(text.contains("(nothing)"), "{text}");
}
