use super::*;

const BODY: [f32; 4] = [0.0, 10.0, 600.0, 36.0];
const CORNER: (f32, f32) = (18.0, 2.0);

fn hanging(blend: f32) -> Merge {
    Merge {
        hang: [200.0, 46.0, 150.0, 90.0],
        radius: 16.0,
        blend,
    }
}

const BAR: [f32; 4] = [0.0, 10.0, 600.0, 38.0];

/// A droplet 80 wide at `x`, parting from the bar's end nearest it.
fn drop(x: f32, waist: f32, thread: f32) -> Split {
    Split {
        x,
        width: 80.0,
        waist,
        thread,
        anchor: if x < 300.0 { 0.0 } else { 600.0 },
        cut: None,
    }
}

fn reach(at: (f32, f32), chain: &[Split]) -> f32 {
    let bar = distance(at, BAR, CORNER, Merge::default());
    with_chain(at, bar, BAR, CORNER, chain)
}

#[test]
fn a_droplet_never_reaches_above_or_below_the_bar() {
    for x in [-40.0, -20.0, -5.0, -92.0, 560.0, 612.0] {
        for (waist, thread) in [(0.0, 0.0), (10.0, 0.0), (5.0, 3.0), (0.0, 8.0)] {
            let chain = [drop(x, waist, thread)];
            for at in (-150..750).step_by(5) {
                let at = at as f32;
                assert!(reach((at, 9.5), &chain) > 0.0, "{x} {at} above");
                assert!(reach((at, 48.5), &chain) > 0.0, "{x} {at} below");
            }
        }
    }
}

#[test]
fn a_droplet_clear_of_the_bar_is_its_own_round_shape() {
    let chain = [drop(-92.0, 0.0, 0.0)];
    assert!(reach((-52.0, 29.0), &chain) < -10.0);
    assert!(reach((-6.0, 29.0), &chain) > 0.0);
    assert!(reach((300.0, 29.0), &chain) < 0.0);
}

#[test]
fn a_thread_joins_a_droplet_across_the_gap() {
    let chain = [drop(-92.0, 0.0, 4.0)];
    assert!(reach((-6.0, 29.0), &chain) < 0.0);
    assert!(reach((-6.0, 20.0), &chain) > 0.0);
}

#[test]
fn a_waist_presses_in_at_the_bars_end() {
    let near_top = |waist| reach((0.0, 13.0), &[drop(-40.0, waist, 0.0)]);
    assert!(near_top(10.0) > near_top(0.0));
    assert!(reach((0.0, 29.0), &[drop(-40.0, 10.0, 0.0)]) < 0.0);
}

#[test]
fn a_droplet_splits_off_the_right_end_too() {
    let chain = [drop(612.0, 0.0, 0.0)];
    assert!(reach((652.0, 29.0), &chain) < -10.0);
    assert!(reach((606.0, 29.0), &chain) > 0.0);
}

#[test]
fn a_droplet_splits_off_the_one_before_it() {
    let first = drop(-92.0, 0.0, 0.0);
    let second = Split {
        x: -184.0,
        anchor: -92.0,
        cut: Some(-12.0),
        thread: 4.0,
        ..drop(-184.0, 0.0, 0.0)
    };
    let chain = [first, second];
    assert!(reach((-144.0, 29.0), &chain) < -10.0);
    assert!(
        reach((-98.0, 29.0), &chain) < 0.0,
        "the thread between them"
    );
    assert!(reach((-98.0, 20.0), &chain) > 0.0);
    assert!(
        reach((-6.0, 29.0), &chain) > 0.0,
        "the gap to the bar stays open"
    );
}

#[test]
fn a_droplet_still_inside_the_one_before_it_never_shows_in_the_gap_behind() {
    let first = drop(-92.0, 0.0, 0.0);
    let hidden = Split {
        x: -160.0,
        width: 160.0,
        anchor: -92.0,
        cut: Some(-12.0),
        ..drop(-160.0, 0.0, 0.0)
    };
    for at in [-11.0, -8.0, -4.0, -1.0] {
        assert!(reach((at, 29.0), &[first, hidden]) > 0.0, "{at}");
    }
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
