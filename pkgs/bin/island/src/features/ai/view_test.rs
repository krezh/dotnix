use super::*;
use crate::features::ai::state::{Credits, Freshness, Limit, Severity};

fn provider(source: Source, id: &str, used: f64) -> Provider {
    Provider {
        source,
        id: id.to_owned(),
        provider_id: "example".to_owned(),
        name: "Provider".to_owned(),
        account: format!("{id}@example.test"),
        plan: String::new(),
        limits: vec![Limit {
            id: "weekly".to_owned(),
            label: "Weekly".to_owned(),
            used,
            remaining: 100.0 - used,
            resets_at: None,
            duration_ms: None,
            severity: Severity::Ok,
            exhausts_at: None,
        }],
        credits: None,
        fetched_at: 1000,
        freshness: Freshness::Fresh,
    }
}

fn state(providers: Vec<Provider>) -> State {
    State {
        providers: Arc::new(providers),
        ..State::default()
    }
}

fn order(view: &View, state: &State) -> Vec<(Source, String)> {
    view.ordered(state)
        .into_iter()
        .map(|provider| (provider.source, provider.id.clone()))
        .collect()
}

#[test]
fn refresh_changes_values_without_moving_open_tiles_or_losing_selection() {
    let initial = state(vec![
        provider(Source::Omp, "one", 20.0),
        provider(Source::Omp, "two", 10.0),
    ]);
    let mut view = View::default();
    view.sync(&initial, false);
    view.sync(&initial, true);
    view.select(Account::of(&initial.providers[1]));

    let updated = state(vec![
        provider(Source::Omp, "two", 90.0),
        provider(Source::Omp, "one", 25.0),
    ]);
    view.sync(&updated, true);
    assert_eq!(
        order(&view, &updated),
        [
            (Source::Omp, "one".to_owned()),
            (Source::Omp, "two".to_owned())
        ]
    );
    let selected = view.selected(&updated).unwrap();
    assert_eq!(selected.id, "two");
    assert_eq!(selected.limits[0].used, 90.0);
    view.back();
    assert!(view.selected(&updated).is_none());
    assert_eq!(
        order(&view, &updated),
        [
            (Source::Omp, "one".to_owned()),
            (Source::Omp, "two".to_owned())
        ]
    );
}

#[test]
fn removed_selection_returns_to_overview_and_new_accounts_append() {
    let initial = state(vec![
        provider(Source::Omp, "one", 10.0),
        provider(Source::Omp, "two", 20.0),
    ]);
    let mut view = View::default();
    view.sync(&initial, true);
    view.select(Account::of(&initial.providers[0]));

    let updated = state(vec![
        provider(Source::Omp, "new", 100.0),
        provider(Source::Omp, "two", 20.0),
    ]);
    view.sync(&updated, true);
    assert!(view.selected(&updated).is_none());
    assert_eq!(
        order(&view, &updated),
        [
            (Source::Omp, "two".to_owned()),
            (Source::Omp, "new".to_owned())
        ]
    );
}

#[test]
fn matching_account_ids_from_different_clients_never_cross_select() {
    let initial = state(vec![
        provider(Source::Omp, "same", 10.0),
        provider(Source::Claude, "same", 90.0),
    ]);
    let mut view = View::default();
    view.sync(&initial, true);
    view.select(Account::of(&initial.providers[1]));
    assert_eq!(view.selected(&initial).unwrap().source, Source::Claude);

    let updated = state(vec![provider(Source::Omp, "same", 15.0)]);
    view.sync(&updated, true);
    assert!(view.selected(&updated).is_none());
    assert_ne!(
        Account::of(&initial.providers[0]).element_id(),
        Account::of(&initial.providers[1]).element_id()
    );
}

#[test]
fn closing_discards_detail_and_next_open_uses_current_urgency_order() {
    let initial = state(vec![
        provider(Source::Omp, "one", 90.0),
        provider(Source::Omp, "two", 10.0),
    ]);
    let mut view = View::default();
    view.sync(&initial, true);
    view.select(Account::of(&initial.providers[0]));
    view.panel_closed();

    let updated = state(vec![
        provider(Source::Omp, "two", 100.0),
        provider(Source::Omp, "one", 0.0),
    ]);
    view.sync(&updated, false);
    view.sync(&updated, true);
    assert!(view.selected(&updated).is_none());
    assert_eq!(
        order(&view, &updated),
        [
            (Source::Omp, "two".to_owned()),
            (Source::Omp, "one".to_owned())
        ]
    );
}

#[test]
fn empty_accounts_are_hidden_but_zero_usage_and_saved_credits_remain_visible() {
    let mut empty = provider(Source::Claude, "empty", 0.0);
    empty.limits.clear();
    let mut unused_credits = provider(Source::Omp, "unused-credits", 0.0);
    unused_credits.limits.clear();
    unused_credits.credits = Some(Credits {
        count: 0,
        expires_at: Some(5000),
    });
    let mut zero_usage = provider(Source::Omp, "zero", 0.0);
    zero_usage.freshness = Freshness::Stale;
    let mut credits = provider(Source::Omp, "credits", 0.0);
    credits.limits.clear();
    credits.credits = Some(Credits {
        count: 2,
        expires_at: None,
    });
    let report = state(vec![empty, unused_credits, zero_usage, credits]);
    let mut view = View::default();
    view.sync(&report, true);

    assert_eq!(
        order(&view, &report),
        [
            (Source::Omp, "zero".to_owned()),
            (Source::Omp, "credits".to_owned())
        ]
    );
}

#[test]
fn an_account_becoming_empty_closes_its_detail_and_returns_at_the_end_when_populated() {
    let initial = state(vec![
        provider(Source::Omp, "one", 30.0),
        provider(Source::Omp, "two", 10.0),
    ]);
    let mut view = View::default();
    view.sync(&initial, true);
    view.select(Account::of(&initial.providers[0]));
    let mut empty = initial.providers[0].clone();
    empty.limits.clear();
    let updated = state(vec![empty, initial.providers[1].clone()]);
    view.sync(&updated, true);
    assert!(view.selected(&updated).is_none());
    assert_eq!(order(&view, &updated), [(Source::Omp, "two".to_owned())]);

    let restored = state(vec![
        provider(Source::Omp, "one", 60.0),
        provider(Source::Omp, "two", 10.0),
    ]);
    view.sync(&restored, true);
    assert!(view.selected(&restored).is_none());
    assert_eq!(
        order(&view, &restored),
        [
            (Source::Omp, "two".to_owned()),
            (Source::Omp, "one".to_owned())
        ]
    );
}

#[test]
fn unverified_badges_preserve_usage_instead_of_counting_down_to_reset() {
    use crate::features::ai::format;

    let mut account = provider(Source::Claude, "cached", 87.0);
    let limit = &mut account.limits[0];
    limit.resets_at = Some(120_000);
    limit.severity = Severity::Exhausted;
    for freshness in [Freshness::Cached, Freshness::Stale] {
        let before_reset = format::badge(limit, freshness, 60_000, true);
        let at_reset = format::badge(limit, freshness, 120_000, true);
        let after_reset = format::badge(limit, freshness, 180_000, true);
        assert_eq!(before_reset, at_reset);
        assert_eq!(at_reset, after_reset);
        assert!(after_reset.contains(&format::percent(87.0)));
        assert_ne!(
            before_reset,
            format::badge(limit, Freshness::Fresh, 60_000, true),
        );
    }
    assert_eq!(limit.used, 87.0);
}

#[test]
fn unverified_reset_changes_at_the_boundary_without_claiming_a_new_window() {
    use crate::features::ai::format;

    let at = Some(120_000);
    for freshness in [Freshness::Cached, Freshness::Stale] {
        let before = format::reset(at, freshness, 119_999);
        let passed = format::reset(at, freshness, 120_000);
        assert_ne!(before, passed);
        assert_eq!(passed, format::reset(at, freshness, 180_000));
        assert_ne!(passed, format::reset(at, Freshness::Fresh, 120_000));
    }
}
