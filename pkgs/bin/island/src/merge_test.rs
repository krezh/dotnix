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
