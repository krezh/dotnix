use std::sync::Arc;

use super::{mood, status};
use crate::{
    components::mascot::Activity,
    features::ai::state::{Freshness, Limit, Provider, Severity, Source, State},
    theme::LAVENDER,
};

fn claude(used: f64, severity: Severity) -> Provider {
    Provider {
        source: Source::Claude,
        id: "claude".to_owned(),
        provider_id: "anthropic".to_owned(),
        name: "Claude".to_owned(),
        account: String::new(),
        plan: String::new(),
        limits: vec![Limit {
            id: "five_hour".to_owned(),
            label: "5h".to_owned(),
            used,
            remaining: 100.0 - used,
            resets_at: None,
            duration_ms: None,
            severity,
            exhausts_at: None,
            expected: None,
        }],
        credits: None,
        fetched_at: 0,
        freshness: Freshness::Fresh,
    }
}

#[test]
fn with_nothing_going_on_it_rests() {
    let state = State::default();
    let mood = mood(&state, false);
    assert_eq!(mood.activity, Activity::Rest);
    assert_eq!(mood.tired, 0.0);
    assert_eq!(mood.color, LAVENDER);
    assert_eq!(status(&state), "No sessions");
}

#[test]
fn it_tires_as_the_limit_fills_and_sleeps_once_it_is_gone() {
    let mut state = State {
        providers: Arc::new(vec![claude(80.0, Severity::High)]),
        ..State::default()
    };
    let tired = mood(&state, false);
    assert_eq!(tired.activity, Activity::Rest);
    assert!((tired.tired - 0.5).abs() < 1e-6);
    assert_ne!(tired.color, LAVENDER);

    state.providers = Arc::new(vec![claude(100.0, Severity::Exhausted)]);
    assert_eq!(mood(&state, false).activity, Activity::Asleep);
    assert_eq!(status(&state), "Limit reached");
}
