use super::*;

#[test]
fn wide_palette_is_bounded_and_uses_single_rows() {
    let layout = ModePaletteLayout::new(1920, 1080, 56, false, true, 1.0);

    assert_eq!(layout.bounds.width, 760.0);
    assert!(layout.bounds.x > 0.0);
    assert!(layout.bounds.y + layout.bounds.height < 1080.0);
    assert_eq!(layout.items().len(), 7);
    assert_eq!(layout.items()[0].rect.y, layout.items()[3].rect.y);
    assert_eq!(layout.items()[4].rect.y, layout.items()[6].rect.y);
}

#[test]
fn narrow_palette_wraps_without_leaving_the_output() {
    let layout = ModePaletteLayout::new(320, 900, 56, false, true, 1.0);

    assert!(layout.bounds.x >= 12.0);
    assert!(layout.bounds.x + layout.bounds.width <= 308.0);
    assert!(layout.bounds.y >= 12.0);
    assert!(layout.bounds.y + layout.bounds.height <= 888.0);
    assert!(layout.items()[2].rect.y > layout.items()[0].rect.y);
}

#[test]
fn hit_testing_distinguishes_actions_from_dismissal_space() {
    let layout = ModePaletteLayout::new(1280, 720, 56, false, true, 1.0);
    let item = layout.items()[1];

    assert_eq!(
        layout.action_at(
            item.rect.x + item.rect.width / 2.0,
            item.rect.y + item.rect.height / 2.0
        ),
        Some(PaletteAction::ScreenshotScreen)
    );
    assert_eq!(layout.action_at(0.0, 0.0), None);
    assert_eq!(
        layout.action_at(layout.bounds.x + 2.0, layout.bounds.y + 2.0),
        None
    );
}

#[test]
fn active_recording_replaces_record_start_actions() {
    let layout = ModePaletteLayout::new(1280, 720, 56, true, true, 1.0);
    let actions: Vec<_> = layout.items().iter().map(|item| item.action).collect();

    assert_eq!(actions.len(), 5);
    assert!(actions.contains(&PaletteAction::StopRecording));
    assert!(!actions.contains(&PaletteAction::RecordArea));
    assert!(!actions.contains(&PaletteAction::RecordScreen));
    assert!(!actions.contains(&PaletteAction::RecordWindow));
}
