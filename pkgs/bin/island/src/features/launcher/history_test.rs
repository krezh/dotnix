use std::collections::HashMap;

use super::{HALF_LIFE, Launches, frecency, merge};

const NOW: i64 = 1_800_000_000;

#[test]
fn a_launch_counts_in_full_when_new_and_half_after_the_half_life() {
    let fresh = Launches {
        count: 4,
        last: NOW,
    };
    assert_eq!(frecency(fresh, NOW), 4.0);
    let old = Launches {
        count: 4,
        last: NOW - HALF_LIFE as i64,
    };
    assert!((frecency(old, NOW) - 2.0).abs() < 1e-9);
}

#[test]
fn a_habit_outranks_a_one_off_until_it_fades() {
    let habit = Launches {
        count: 30,
        last: NOW - 7 * 86_400,
    };
    let once = Launches {
        count: 1,
        last: NOW,
    };
    assert!(frecency(habit, NOW) > frecency(once, NOW));
    let abandoned = Launches {
        count: 30,
        last: NOW - 200 * 86_400,
    };
    assert!(frecency(abandoned, NOW) < frecency(once, NOW));
}

#[test]
fn merging_keeps_the_larger_count_and_the_later_launch() {
    let stored = HashMap::from([
        (
            "firefox.desktop".to_owned(),
            Launches {
                count: 10,
                last: NOW - 100,
            },
        ),
        (
            "kitty.desktop".to_owned(),
            Launches {
                count: 3,
                last: NOW - 50,
            },
        ),
    ]);
    let counted = HashMap::from([
        (
            "firefox.desktop".to_owned(),
            Launches {
                count: 1,
                last: NOW,
            },
        ),
        (
            "zed.desktop".to_owned(),
            Launches {
                count: 1,
                last: NOW,
            },
        ),
    ]);
    let merged = merge(stored, counted);
    assert_eq!(
        merged["firefox.desktop"],
        Launches {
            count: 10,
            last: NOW
        }
    );
    assert_eq!(
        merged["kitty.desktop"],
        Launches {
            count: 3,
            last: NOW - 50
        }
    );
    assert_eq!(
        merged["zed.desktop"],
        Launches {
            count: 1,
            last: NOW
        }
    );
}
