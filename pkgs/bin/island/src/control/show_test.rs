use super::*;
use crate::cli::render;

fn volume(value: f32) -> Row {
    Row::new(
        "Volume",
        Shown::Level {
            value,
            muted: false,
        },
    )
    .detail("Speakers")
}

#[test]
fn a_change_is_ticked_and_a_level_drawn_as_a_bar() {
    let text = render(&lines(&Report::did(volume(0.5)), true), false);
    assert_eq!(text, "✓ Volume        ━━━━━━━━━━━━━━━━━━━━   50%  Speakers");
}

#[test]
fn piped_levels_are_plain_numbers() {
    let report = Report::shows(vec![
        volume(0.69),
        Row::new("Do not disturb", Shown::Switch(true)),
    ]);
    assert_eq!(
        render(&lines(&report, false), false),
        "  Volume            69%  Speakers\n  Do not disturb   on"
    );
}

#[test]
fn an_action_is_only_its_label() {
    let text = render(&lines(&Report::did(Row::said("Skipped to the next track")), true), false);
    assert_eq!(text, "✓ Skipped to the next track");
}
