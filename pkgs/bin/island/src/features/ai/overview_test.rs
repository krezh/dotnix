use super::{resets, short_label};
use crate::{
    features::ai::{
        format,
        state::{Credits, Limit, Severity},
    },
    theme::{TEAL, YELLOW},
};

const DAY: i64 = 24 * 60 * 60 * 1000;

fn credits(count: u64, expires_at: Option<i64>) -> Credits {
    Credits { count, expires_at }
}

#[test]
fn resets_say_how_long_they_have_left() {
    let now = 1_000_000;
    assert_eq!(
        resets(&credits(2, Some(now + 3 * DAY + 4 * 60 * 60 * 1000)), now),
        Some((Some(TEAL), "2 resets · 3d 4h left".into()))
    );
    assert_eq!(
        resets(&credits(1, None), now),
        Some((Some(TEAL), "1 reset".into()))
    );
    assert_eq!(
        resets(&credits(1, Some(now - 1)), now),
        Some((Some(YELLOW), "1 reset expired".into()))
    );
    assert_eq!(resets(&credits(0, Some(now + DAY)), now), None);
}

fn limit(label: &str) -> Limit {
    Limit {
        id: label.to_owned(),
        label: label.to_owned(),
        used: 0.0,
        remaining: 100.0,
        resets_at: None,
        duration_ms: None,
        severity: Severity::Ok,
        exhausts_at: None,
        expected: None,
    }
}

#[test]
fn a_word_every_limit_starts_with_is_left_out() {
    let limits = [limit("Claude 5 Hour"), limit("Claude 7 Day")];
    assert_eq!(short_label("Claude 5 Hour", &limits), "5 Hour");
    let gemini = [limit("Gemini • Weekly"), limit("Gemini • 5 Hour")];
    assert_eq!(short_label("Gemini • Weekly", &gemini), "Weekly");
}

#[test]
fn labels_that_differ_or_stand_alone_stay_whole() {
    let mixed = [limit("5 hours"), limit("7 days")];
    assert_eq!(short_label("5 hours", &mixed), "5 hours");
    let alone = [limit("Claude 5 Hour")];
    assert_eq!(short_label("Claude 5 Hour", &alone), "Claude 5 Hour");
}

#[test]
fn time_text_next_change_lands_on_the_boundary() {
    let now = 1_000_000_000;
    for left in [1, 60_000, 60_001, 3_600_000, 2 * DAY + 123_456] {
        let at = now + left;
        let wait = format::next_countdown_ms(at, now).unwrap() as i64;
        assert_eq!(
            format::countdown(at, now + wait - 1),
            format::countdown(at, now)
        );
        assert_ne!(
            format::countdown(at, now + wait),
            format::countdown(at, now)
        );
    }
    for elapsed in [0, 59_999, 60_000, 3_600_000 + 500, 2 * DAY] {
        let at = now - elapsed;
        let wait = format::next_ago_ms(at, now).unwrap() as i64;
        assert_eq!(format::ago(at, now + wait - 1), format::ago(at, now));
        assert_ne!(format::ago(at, now + wait), format::ago(at, now));

        let wait = format::next_elapsed_ms(at, now) as i64;
        assert_eq!(
            format::countdown(now + wait - 1, at),
            format::countdown(now, at)
        );
        assert_ne!(
            format::countdown(now + wait, at),
            format::countdown(now, at)
        );
    }
}
