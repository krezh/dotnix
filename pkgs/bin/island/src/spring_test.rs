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

#[test]
fn steps_exactly_as_gpui_does_in_every_damping_regime() {
    for (stiffness, damping) in [
        (420.0, 41.0),
        (170.0, 26.0),
        (1100.0, 66.0),
        (150.0, 24.5),
        (130.0, 3.2),
        (100.0, 20.0),
        (100.0, 40.0),
    ] {
        let ours = super::SpringConfig::new(stiffness, damping, 1.0);
        let theirs = gpui::SpringConfig::new(stiffness, damping, 1.0);
        let mut state = super::SpringState {
            position: 0.0,
            velocity: 3.0,
        };
        let mut expected = gpui::SpringState {
            position: 0.0,
            velocity: 3.0,
        };
        for frame in 0..600 {
            let target = if frame < 300 { 1.0 } else { -2.5 };
            state = ours.step(state, target, 1.0 / 144.0);
            expected = theirs.step(expected, target, 1.0 / 144.0);
            assert_eq!(state.position.to_bits(), expected.position.to_bits());
            assert_eq!(state.velocity.to_bits(), expected.velocity.to_bits());
            assert_eq!(
                ours.is_settled(state, target, 0.002),
                theirs.is_settled(expected, target, 0.002)
            );
        }
    }
}
