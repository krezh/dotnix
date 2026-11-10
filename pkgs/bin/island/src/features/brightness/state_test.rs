use super::{Levels, Screen, moved, osd, reread};

fn screen(name: &str, level: f32) -> Screen {
    Screen {
        name: name.to_owned(),
        model: String::new(),
        level: Some(level),
    }
}

fn changed(screens: Vec<Screen>, changed: &[&str]) -> Levels {
    Levels {
        screens,
        changed: changed.iter().map(|name| (*name).to_owned()).collect(),
        changes: 1,
    }
}

#[test]
fn one_of_several_screens_is_named() {
    let after = changed(vec![screen("DP-1", 0.4), screen("DP-2", 0.9)], &["DP-2"]);
    let shown = osd(&Levels::default(), &after).unwrap();
    assert_eq!(
        (shown.caption, shown.label.as_str()),
        ("Brightness", "DP-2")
    );
    assert_eq!(shown.level, Some((0.9, false)));
}

#[test]
fn all_screens_or_the_only_one_go_unnamed() {
    let all = changed(
        vec![screen("DP-1", 0.4), screen("DP-2", 0.4)],
        &["DP-1", "DP-2"],
    );
    let only = changed(vec![screen("eDP-1", 0.7)], &["eDP-1"]);
    for after in [all, only] {
        let shown = osd(&Levels::default(), &after).unwrap();
        assert_eq!((shown.caption, shown.label.as_str()), ("", "Brightness"));
    }
}

#[test]
fn reading_shows_nothing() {
    let levels = changed(vec![screen("DP-1", 0.4)], &["DP-1"]);
    assert_eq!(osd(&levels, &levels.clone()), None);
}

#[test]
fn a_reread_updates_without_an_osd() {
    let before = changed(vec![screen("DP-1", 0.4), screen("DP-2", 0.9)], &["DP-1"]);
    let mut after = before.clone();
    reread(
        &mut after,
        vec![
            ("DP-1".to_owned(), Some(0.7)),
            ("DP-2".to_owned(), Some(0.9)),
        ],
    );
    assert_eq!(after.screens[0].level, Some(0.7));
    assert_eq!(osd(&before, &after), None);
}

#[test]
fn a_screen_that_misses_a_reread_keeps_its_level() {
    let mut levels = changed(vec![screen("DP-1", 0.4)], &["DP-1"]);
    reread(&mut levels, vec![("DP-1".to_owned(), None)]);
    assert_eq!(levels.screens[0].level, Some(0.4));
}

#[test]
fn a_backlight_moved_elsewhere_shows() {
    let before = changed(vec![screen("eDP-1", 0.4)], &[]);
    let mut after = before.clone();
    moved(&mut after, "eDP-1", 0.6);
    let shown = osd(&before, &after).unwrap();
    assert_eq!(shown.level, Some((0.6, false)));
}

#[test]
fn a_backlight_left_where_island_set_it_shows_nothing_more() {
    let before = changed(vec![screen("eDP-1", 0.6)], &["eDP-1"]);
    let mut after = before.clone();
    moved(&mut after, "eDP-1", 0.6);
    assert_eq!(osd(&before, &after), None);
}
