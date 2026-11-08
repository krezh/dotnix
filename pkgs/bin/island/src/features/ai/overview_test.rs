use std::sync::Arc;

use super::{accounts, horizon, pack_outages, pool_note, resets, short_label};
use crate::{
    components::timeline::Span,
    features::ai::{
        format, jelly,
        mascot::Activity,
        state::{Credits, Freshness, Limit, Provider, Severity, Source, State},
        view::{Account, Tab, View},
    },
    theme::{OVERLAY1, OVERLAY2, RED, TEAL, YELLOW},
};

const HOUR: i64 = 60 * 60 * 1000;
const DAY: i64 = 24 * HOUR;

fn credits(count: u64, expires_at: Option<i64>) -> Credits {
    Credits { count, expires_at }
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

fn provider(source: Source, id: &str, name: &str, limits: Vec<Limit>) -> Provider {
    Provider {
        source,
        id: id.to_owned(),
        provider_id: id.to_owned(),
        name: name.to_owned(),
        account: format!("{id}@example.test"),
        plan: "pro".to_owned(),
        limits,
        credits: None,
        fetched_at: 1_000_000,
        freshness: Freshness::Fresh,
    }
}

#[test]
fn resets_say_how_long_they_have_left() {
    let now = 1_000_000;
    assert_eq!(
        resets(&credits(2, Some(now + 3 * DAY + 4 * HOUR)), now),
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

#[test]
fn shared_prefix_and_orphaned_leading_parens_are_cleaned() {
    let claude = [limit("Claude 5 Hour"), limit("Claude 7 Day")];
    assert_eq!(short_label("Claude 5 Hour", &claude), "5 Hour");

    let gemini = [
        limit("Gemini (Argon) · Weekly"),
        limit("Gemini • Weekly"),
        limit("Gemini • 5 Hour"),
    ];
    assert_eq!(
        short_label("Gemini (Argon) · Weekly", &gemini),
        "Argon · Weekly"
    );
    assert_eq!(short_label("Gemini • Weekly", &gemini), "Weekly");
}

#[test]
fn labels_that_differ_or_stand_alone_stay_whole() {
    let mixed = [limit("5 hours"), limit("7 days")];
    assert_eq!(short_label("5 hours", &mixed), "5 hours");
    let alone = [limit("Claude 5 Hour")];
    assert_eq!(short_label("Claude 5 Hour", &alone), "Claude 5 Hour");
    let parens_alone = [limit("(Argon) · Weekly")];
    assert_eq!(
        short_label("(Argon) · Weekly", &parens_alone),
        "Argon · Weekly"
    );
}

#[test]
fn short_window_reports_ceiling_block_or_unstarted_state() {
    let now = 1_700_000_000_000;
    let mut seven_day = limit("Claude 7 Day");
    seven_day.used = 100.0;
    seven_day.severity = Severity::Exhausted;
    seven_day.duration_ms = Some(7 * DAY);
    seven_day.resets_at = Some(now + 2 * DAY + 8 * HOUR);

    let mut five_hour = limit("Claude 5 Hour");
    five_hour.duration_ms = Some(5 * HOUR);

    let blocked_set = [seven_day.clone(), five_hour.clone()];
    assert_eq!(
        pool_note(&five_hour, &blocked_set, true, now),
        ("blocked by 7-day".to_owned(), OVERLAY2)
    );
    let (_, exhausted_color) = pool_note(&seven_day, &blocked_set, true, now);
    assert_eq!(exhausted_color, RED);

    seven_day.used = 20.0;
    seven_day.severity = Severity::Ok;
    let healthy_set = [seven_day, five_hour.clone()];
    assert_eq!(
        pool_note(&five_hour, &healthy_set, true, now),
        ("starts on use".to_owned(), OVERLAY1)
    );
}

#[test]
fn horizon_packs_non_overlapping_outages_onto_one_lane_and_omits_idle_fleets() {
    let now = 1_700_000_000_000;

    let mut idle_a = limit("Weekly");
    idle_a.duration_ms = Some(7 * DAY);
    idle_a.resets_at = Some(now + 6 * DAY);
    let idle_provider = provider(Source::Omp, "idle", "Google Antigravity", vec![idle_a]);
    let idle_refs = [&idle_provider];
    assert!(horizon(&accounts(&idle_refs), now).is_none());

    let mut claude_7d = limit("7 Day");
    claude_7d.used = 100.0;
    claude_7d.severity = Severity::Exhausted;
    claude_7d.duration_ms = Some(7 * DAY);
    claude_7d.resets_at = Some(now + 2 * DAY);

    let mut argon_7d = limit("Gemini (Argon) · Weekly");
    argon_7d.used = 62.0;
    argon_7d.duration_ms = Some(7 * DAY);
    argon_7d.exhausts_at = Some(now + 4 * DAY);
    argon_7d.resets_at = Some(now + 6 * DAY + 12 * HOUR);

    let mut standard_7d = limit("Gemini · Weekly");
    standard_7d.used = 24.0;
    standard_7d.duration_ms = Some(7 * DAY);
    standard_7d.resets_at = Some(now + 6 * DAY + 12 * HOUR);

    let mut codex_7d = limit("Weekly");
    codex_7d.used = 31.0;
    codex_7d.duration_ms = Some(7 * DAY);
    codex_7d.resets_at = Some(now + 6 * DAY + 14 * HOUR);

    let mut monthly = limit("Premium");
    monthly.used = 19.0;
    monthly.duration_ms = Some(30 * DAY);
    monthly.resets_at = Some(now + 18 * DAY);

    let p_claude = provider(Source::Claude, "c1", "Anthropic", vec![claude_7d]);
    let p_ag = provider(
        Source::Omp,
        "ag",
        "Google Antigravity",
        vec![argon_7d, standard_7d],
    );
    let p_codex = provider(Source::Omp, "cx", "OpenAI Codex", vec![codex_7d]);
    let p_copilot = provider(Source::Omp, "cp", "GitHub Copilot", vec![monthly]);

    let all = [&p_claude, &p_ag, &p_codex, &p_copilot];
    let seen = accounts(&all);
    let week = horizon(&seen, now).expect("horizon present for active outages and resets");

    assert_eq!(week.lanes.len(), 1);
    assert_eq!(week.lanes[0].len(), 2);
    assert!(!week.lanes[0][0].start_tick);
    assert!(week.lanes[0][1].start_tick);
    assert_eq!(week.pins.len(), 1);
    assert!(week.pins[0].label.starts_with("+2 resets"));
    assert_eq!(
        week.side.as_ref().map(|side| side.detail.as_ref()),
        Some("2 outages · 2 resets")
    );
}

#[test]
fn overlapping_outages_cap_swimlane_height_at_two_while_counting_all() {
    let span = |key: &'static str, from: f32, to: f32| Span {
        label: key.into(),
        tooltip: key.into(),
        from,
        to,
        color: RED,
        start_tick: false,
        end_dot: true,
    };
    let (lanes, count) = pack_outages(vec![
        span("a", 0.0, 0.5),
        span("b", 0.1, 0.6),
        span("c", 0.2, 0.7),
    ]);
    assert_eq!(count, 3);
    assert_eq!(lanes.len(), 2);
}

#[test]
fn jelly_stays_awake_while_another_provider_remains_ready() {
    let mut spent = limit("7 Day");
    spent.used = 100.0;
    spent.severity = Severity::Exhausted;
    let open = limit("Weekly");

    let claude_out = provider(Source::Claude, "claude", "Anthropic", vec![spent.clone()]);
    let omp_ready = provider(Source::Omp, "ag", "Google Antigravity", vec![open]);
    let mixed = State {
        providers: Arc::new(vec![claude_out.clone(), omp_ready]),
        ..State::default()
    };
    let sources = [Source::Omp, Source::Claude];
    assert_eq!(jelly::activity(&mixed), Activity::Rest);
    assert_eq!(
        jelly::title(&sources, &mixed, Tab::Usage),
        "1 of 2 accounts ready"
    );

    let omp_out = provider(Source::Omp, "ag", "Google Antigravity", vec![spent]);
    let all_out = State {
        providers: Arc::new(vec![claude_out, omp_out]),
        ..State::default()
    };
    assert_eq!(jelly::activity(&all_out), Activity::Asleep);
    assert_eq!(
        jelly::title(&sources, &all_out, Tab::Usage),
        "All limits exhausted"
    );
}

#[test]
fn account_fold_toggle_is_reversible_and_resets_on_close() {
    use crate::features::Local;

    let p = provider(
        Source::Omp,
        "ag",
        "Google Antigravity",
        vec![limit("Weekly")],
    );
    let account = Account::of(&p);
    let mut view = View::default();
    assert!(!view.is_expanded(&account));
    assert!(!view.was_toggled(&account));

    view.toggle_account(account.clone());
    assert!(view.is_expanded(&account));
    assert!(view.was_toggled(&account));

    view.toggle_account(account.clone());
    assert!(!view.is_expanded(&account));
    assert!(view.was_toggled(&account));

    view.panel_closed();
    assert!(!view.is_expanded(&account));
    assert!(!view.was_toggled(&account));
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
