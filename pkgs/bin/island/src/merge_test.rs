use super::*;

const BODY: [f32; 4] = [0.0, 10.0, 600.0, 36.0];
const CORNER: (f32, f32) = (18.0, 2.0);

fn hanging(blend: f32) -> Merge {
    Merge {
        hang: [200.0, 46.0, 150.0, 90.0],
        radius: 16.0,
        blend,
        split: None,
    }
}

fn dropping(x: f32, waist: f32, thread: f32) -> Merge {
    Merge {
        split: Some(Split {
            x,
            width: 80.0,
            waist,
            thread,
        }),
        ..Merge::default()
    }
}

const BAR: [f32; 4] = [0.0, 10.0, 600.0, 38.0];

#[test]
fn a_droplet_never_reaches_above_or_below_the_bar() {
    for x in [-40.0, -20.0, -5.0, -92.0, 560.0, 612.0] {
        for (waist, thread) in [(0.0, 0.0), (10.0, 0.0), (5.0, 3.0), (0.0, 8.0)] {
            let merge = dropping(x, waist, thread);
            for at in (-150..750).step_by(5) {
                let at = at as f32;
                assert!(
                    distance((at, 9.5), BAR, CORNER, merge) > 0.0,
                    "{x} {at} above"
                );
                assert!(
                    distance((at, 48.5), BAR, CORNER, merge) > 0.0,
                    "{x} {at} below"
                );
            }
        }
    }
}

#[test]
fn a_droplet_clear_of_the_bar_is_its_own_round_shape() {
    let merge = dropping(-92.0, 0.0, 0.0);
    assert!(distance((-52.0, 29.0), BAR, CORNER, merge) < -10.0);
    assert!(distance((-6.0, 29.0), BAR, CORNER, merge) > 0.0);
    assert!(distance((300.0, 29.0), BAR, CORNER, merge) < 0.0);
}

#[test]
fn a_thread_joins_a_droplet_across_the_gap() {
    let merge = dropping(-92.0, 0.0, 4.0);
    assert!(distance((-6.0, 29.0), BAR, CORNER, merge) < 0.0);
    assert!(distance((-6.0, 20.0), BAR, CORNER, merge) > 0.0);
}

#[test]
fn a_waist_presses_in_at_the_bars_end() {
    let open = dropping(-40.0, 0.0, 0.0);
    let pressed = dropping(-40.0, 10.0, 0.0);
    let near_top = |merge| distance((0.0, 13.0), BAR, CORNER, merge);
    assert!(near_top(pressed) > near_top(open));
    assert!(distance((0.0, 29.0), BAR, CORNER, pressed) < 0.0);
}

#[test]
fn a_droplet_splits_off_the_right_end_too() {
    let merge = dropping(612.0, 0.0, 0.0);
    assert!(distance((652.0, 29.0), BAR, CORNER, merge) < -10.0);
    assert!(distance((606.0, 29.0), BAR, CORNER, merge) > 0.0);
}

#[test]
fn far_from_the_menu_it_is_the_bar() {
    let bar = corner_distance((40.0, 30.0), (300.0, 28.0), (300.0, 18.0), 18.0, 2.0);
    assert_eq!(distance((40.0, 30.0), BODY, CORNER, hanging(36.0)), bar);
}

#[test]
fn the_top_of_the_bar_stays_flat_however_soft_the_blend() {
    for x in [190.0, 210.0, 275.0, 360.0] {
        let sharp = distance((x, 10.5), BODY, CORNER, hanging(0.0));
        let soft = distance((x, 10.5), BODY, CORNER, hanging(40.0));
        assert_eq!(sharp, soft);
    }
}

#[test]
fn a_softer_blend_widens_where_they_meet() {
    let width = |blend: f32| {
        rows(BODY, CORNER, hanging(blend))
            .into_iter()
            .find(|row| row[1] <= 48 && row[1] + row[3] > 48)
            .map_or(0, |row| row[2])
    };
    assert!(width(36.0) > width(12.0));
    assert!(width(12.0) >= 150);
}

#[test]
fn the_rows_cover_the_bar_and_the_menu() {
    let found = rows(BODY, CORNER, hanging(12.0));
    let covers = |x: i32, y: i32| {
        found
            .iter()
            .any(|[l, t, w, h]| x >= *l && x < l + w && y >= *t && y < t + h)
    };
    assert!(covers(300, 28));
    assert!(covers(275, 120));
    assert!(!covers(275, 140));
    assert!(!covers(100, 80));
}

#[test]
fn a_menu_all_but_gone_leaves_no_bump() {
    let thin = Merge {
        hang: [200.0, 46.0, 150.0, 0.6],
        radius: 30.0,
        blend: 36.0,
        split: None,
    };
    for y in [47.0, 48.0, 52.0] {
        assert!(distance((275.0, y), BODY, CORNER, thin) > 0.0);
    }
}

#[test]
fn a_rounded_region_is_full_width_in_the_middle_and_inset_at_its_corners() {
    let rects = region([10, 20, 100, 40], (12.0, 2.0));
    let covers = |x: i32, y: i32| {
        rects
            .iter()
            .any(|[rx, ry, rw, rh]| x >= *rx && x < rx + rw && y >= *ry && y < ry + rh)
    };
    assert!(covers(10, 40) && covers(109, 40), "full width halfway down");
    assert!(!covers(10, 20) && !covers(109, 59), "the corners are cut");
    assert!(
        covers(60, 20) && covers(60, 59),
        "the top and bottom edges are there"
    );
    assert!(region([0, 0, 0, 10], (12.0, 2.0)).is_empty());
}
