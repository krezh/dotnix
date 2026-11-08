use super::{rubber, unrubber};

const REACH: f32 = 24.0;

#[test]
fn stretches_nothing_without_a_pull() {
    assert_eq!(rubber(0.0, REACH), 0.0);
}

#[test]
fn stretches_the_way_it_is_pulled() {
    assert!(rubber(30.0, REACH) > 0.0);
    assert_eq!(rubber(-30.0, REACH), -rubber(30.0, REACH));
}

#[test]
fn stretches_ever_less_and_never_past_its_reach() {
    let mut last = 0.0;
    for over in [5.0, 20.0, 80.0, 400.0, 5000.0] {
        let now = rubber(over, REACH);
        assert!(now > last, "{over} px stretches further than less");
        assert!(now < REACH, "{over} px stays within reach");
        assert!(now < over, "{over} px stretches less than it is pulled");
        last = now;
    }
}

#[test]
fn a_stretch_gives_back_the_pull_behind_it() {
    for over in [-60.0, -5.0, 0.0, 12.0, 90.0] {
        let back = unrubber(rubber(over, REACH), REACH);
        assert!((back - over).abs() < 0.01, "{over} px comes back as {back}");
    }
}
