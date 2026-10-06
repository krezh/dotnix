use super::*;

#[test]
fn wide_palette_is_bounded_and_uses_single_rows() {
    let layout = ModePaletteLayout::new(
        1920,
        1080,
        56,
        false,
        true,
        ReplayPaletteState::default(),
        1.0,
    );

    assert_eq!(layout.bounds.width, 760.0);
    assert!(layout.bounds.x > 0.0);
    assert!(layout.bounds.y + layout.bounds.height < 1080.0);
    assert_eq!(layout.items().len(), 7);
    assert_eq!(layout.items()[0].rect.y, layout.items()[3].rect.y);
    assert_eq!(layout.items()[4].rect.y, layout.items()[6].rect.y);
}

#[test]
fn narrow_palette_wraps_without_leaving_the_output() {
    let layout = ModePaletteLayout::new(
        320,
        900,
        56,
        false,
        true,
        ReplayPaletteState::default(),
        1.0,
    );

    assert!(layout.bounds.x >= 8.0);
    assert!(layout.bounds.x + layout.bounds.width <= 312.0);
    assert!(layout.bounds.y >= 8.0);
    assert!(layout.bounds.y + layout.bounds.height <= 892.0);
    assert!(layout.items()[2].rect.y > layout.items()[0].rect.y);
}

#[test]
fn hit_testing_distinguishes_actions_from_dismissal_space() {
    let layout = ModePaletteLayout::new(
        1280,
        720,
        56,
        false,
        true,
        ReplayPaletteState::default(),
        1.0,
    );
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
    let layout = ModePaletteLayout::new(
        1280,
        720,
        56,
        true,
        true,
        ReplayPaletteState::default(),
        1.0,
    );
    let actions: Vec<_> = layout.items().iter().map(|item| item.action).collect();

    assert_eq!(actions.len(), 5);
    assert!(actions.contains(&PaletteAction::StopRecording));
    assert!(!actions.contains(&PaletteAction::RecordArea));
    assert!(!actions.contains(&PaletteAction::RecordScreen));
    assert!(!actions.contains(&PaletteAction::RecordWindow));
}

#[test]
fn replay_ready_adds_save_action_and_hit_testing() {
    let replay = ReplayPaletteState {
        visible: true,
        can_save: true,
    };
    let layout = ModePaletteLayout::new(1920, 1080, 56, false, true, replay, 1.0);
    let actions: Vec<_> = layout.items().iter().map(|item| item.action).collect();

    assert!(actions.contains(&PaletteAction::SaveReplay));
    let save_item = layout
        .items()
        .iter()
        .find(|item| item.action == PaletteAction::SaveReplay)
        .unwrap();
    assert_eq!(
        layout.action_at(
            save_item.rect.x + save_item.rect.width / 2.0,
            save_item.rect.y + save_item.rect.height / 2.0
        ),
        Some(PaletteAction::SaveReplay)
    );
    assert!(layout.replay_heading_y > layout.record_heading_y);
}

#[test]
fn replay_offline_or_waiting_shows_heading_without_dead_save_action() {
    let replay = ReplayPaletteState {
        visible: true,
        can_save: false,
    };
    let layout = ModePaletteLayout::new(1920, 1080, 56, false, true, replay, 1.0);
    let actions: Vec<_> = layout.items().iter().map(|item| item.action).collect();

    assert!(!actions.contains(&PaletteAction::SaveReplay));
    assert!(layout.replay_heading_y > layout.record_heading_y);
}

#[test]
fn extremely_short_and_narrow_outputs_stay_strictly_within_bounds() {
    let test_resolutions = [(640, 360), (480, 272), (320, 240), (240, 200), (200, 160)];

    let replay = ReplayPaletteState {
        visible: true,
        can_save: true,
    };

    for (w, h) in test_resolutions {
        let layout = ModePaletteLayout::new(w, h, 56, true, true, replay, 1.0);
        let sw = f64::from(w);
        let sh = f64::from(h);

        assert!(
            layout.bounds.x >= 0.0,
            "bounds.x {} < 0 for {}x{}",
            layout.bounds.x,
            w,
            h
        );
        assert!(
            layout.bounds.y >= 0.0,
            "bounds.y {} < 0 for {}x{}",
            layout.bounds.y,
            w,
            h
        );
        assert!(
            layout.bounds.x + layout.bounds.width <= sw,
            "bounds right {} > {} for {}x{}",
            layout.bounds.x + layout.bounds.width,
            sw,
            w,
            h
        );
        assert!(
            layout.bounds.y + layout.bounds.height <= sh,
            "bounds bottom {} > {} for {}x{}",
            layout.bounds.y + layout.bounds.height,
            sh,
            w,
            h
        );

        for item in layout.items() {
            assert!(
                item.rect.x >= 0.0 && item.rect.y >= 0.0,
                "item at ({}, {}) < 0 for {}x{}",
                item.rect.x,
                item.rect.y,
                w,
                h
            );
            assert!(
                item.rect.x + item.rect.width <= sw,
                "item right {} > {} for {}x{}",
                item.rect.x + item.rect.width,
                sw,
                w,
                h
            );
            assert!(
                item.rect.y + item.rect.height <= sh,
                "item bottom {} > {} for {}x{}",
                item.rect.y + item.rect.height,
                sh,
                w,
                h
            );
        }
    }
}
