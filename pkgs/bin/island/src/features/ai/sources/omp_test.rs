use serde_json::json;

use super::*;
use crate::features::ai::{
    model::Tracker,
    settings::Settings,
    state::{Severity, State},
};

fn limit(id: &str, amount: Value) -> Value {
    json!({"id": id, "label": "Model", "amount": amount, "status": "ok",
        "window": {"id": "five-hour", "label": "5 Hour", "durationMs": 18000000,
            "resetsAt": 1800000000000_i64}})
}

#[test]
fn normalizes_fractions_counts_percent_and_overages() {
    for (amount, expected) in [
        (
            json!({"usedFraction": 0.375, "used": 900, "unit": "tokens"}),
            37.5,
        ),
        (json!({"remainingFraction": 0.2}), 80.0),
        (json!({"used": 25, "limit": 200, "unit": "requests"}), 12.5),
        (
            json!({"remaining": 150, "limit": 200, "unit": "tokens"}),
            25.0,
        ),
        (json!({"used": 0.5, "unit": "percent"}), 0.5),
        (json!({"used": 0.5, "unit": "fraction"}), 50.0),
        (json!({"used": 110, "unit": "percent"}), 110.0),
    ] {
        let parsed = parse_limit(&limit("limit", amount)).unwrap();
        assert!((parsed.used - expected).abs() < 0.00001);
        assert_eq!(parsed.remaining, (100.0 - expected).max(0.0));
    }
}

#[test]
fn rejects_missing_or_unusable_quota_instead_of_zero() {
    for amount in [
        json!({}),
        json!({"used": 10}),
        json!({"used": 1, "limit": 0}),
        json!({"usedFraction": -0.1}),
        json!({"used": "10", "unit": "percent"}),
    ] {
        assert!(parse_limit(&limit("limit", amount)).is_err());
    }
    for invalid in [
        json!({}),
        json!({"reports": {}}),
        json!({"reports": [null]}),
        json!({"reports": [{"provider": "test"}]}),
    ] {
        assert!(parse(&invalid, 1000).is_err());
    }
}

#[test]
fn ignores_accounts_without_usage_but_keeps_disabled_credentials() {
    let parsed = parse(
        &json!({"reports": [], "disabledCredentials": [
        {"provider": "anthropic", "email": "disabled@example.test", "cause": "expired"}],
        "accountsWithoutUsage": [{"provider": "opencode-go", "type": "api_key"}]}),
        1000,
    )
    .unwrap();
    assert!(parsed.providers.is_empty());
    assert_eq!(parsed.issues.len(), 1);
    assert_eq!(parsed.issues[0].account, "disabled@example.test");
}

#[test]
fn shared_groups_are_account_local_and_account_ids_are_stable() {
    let mut first = limit("model-a", json!({"usedFraction": 0.5}));
    first["scope"] = json!({"shared": true, "sharedGroup": "shared"});
    let mut second = first.clone();
    second["id"] = json!("model-b");
    let reports = json!({"generatedAt": 1234, "reports": [
        {"provider": "anthropic", "metadata": {"accountId": "one", "planType": "max"},
            "limits": [first, second]},
        {"provider": "anthropic", "metadata": {"accountId": "two"}, "limits": [first]}]});
    let parsed = parse(&reports, 9999).unwrap();
    assert_eq!(parsed.providers.len(), 2);
    assert_ne!(parsed.providers[0].id, parsed.providers[1].id);
    assert_eq!(parsed.providers[0].limits.len(), 1);
    assert_eq!(parsed.providers[1].limits.len(), 1);
    assert_eq!(parsed.providers[0].fetched_at, 1234);
    assert_eq!(parsed.providers[0].plan, "max");
    let mut reordered = reports.clone();
    reordered["reports"].as_array_mut().unwrap().reverse();
    assert_eq!(
        parse(&reordered, 9999).unwrap().providers[1].id,
        parsed.providers[0].id
    );
}

#[test]
fn parses_numeric_windows_authoritative_status_and_credit_expiry() {
    let mut raw = limit("five-hour", json!({"used": 20, "unit": "percent"}));
    raw["status"] = json!("warning");
    let parsed = parse(
        &json!({"reports": [{"provider": "openai-codex", "fetchedAt": 3210,
        "limits": [raw], "resetCredits": {"availableCount": 2, "credits": [
            {"status": "available", "expiresAt": "2027-01-03T00:00:00Z"},
            {"status": "spent", "expiresAt": "2027-01-01T00:00:00Z"},
            {"status": "available", "expiresAt": "2027-01-02T00:00:00+00:00"}]}}]}),
        9999,
    )
    .unwrap();
    let provider = &parsed.providers[0];
    assert_eq!(provider.fetched_at, 3210);
    assert_eq!(provider.limits[0].severity, Severity::High);
    assert_eq!(provider.limits[0].duration_ms, Some(18000000));
    assert_eq!(provider.limits[0].resets_at, Some(1800000000000));
    assert_eq!(provider.credits.as_ref().unwrap().count, 2);
    assert_eq!(
        provider.credits.as_ref().unwrap().expires_at,
        timestamp(&json!("2027-01-02T00:00:00Z"))
    );
    assert_eq!(
        parse_limit(&json!({"id": "bare", "amount": {"usedFraction": 1},
        "status": "exhausted"}))
        .unwrap()
        .severity,
        Severity::Exhausted
    );
}

#[test]
fn disabling_a_shared_pool_survives_model_order_changes_and_keeps_gemini() {
    let settings: Settings = toml::from_str(
        r#"
        [alerts]
        pace_alerts = false
        [disabled_limits.omp]
        google-antigravity = ["3p-weekly:weekly"]
    "#,
    )
    .unwrap();
    let mut claude = limit("claude-alias", json!({"usedFraction": 0.0}));
    claude["scope"] = json!({"sharedGroup": "3p-weekly:weekly"});
    let mut gpt = claude.clone();
    gpt["id"] = json!("gpt-alias");
    let gemini = limit("gemini", json!({"usedFraction": 0.2}));
    let mut payload = json!({"reports": [{
        "provider": "google-antigravity",
        "metadata": {"accountId": "one"},
        "limits": [claude, gpt, gemini],
    }]});
    let mut tracker = Tracker::default();
    let mut state = State::default();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(parse(&payload, 1000).unwrap()),
        &settings,
    );

    let limits = payload["reports"][0]["limits"].as_array_mut().unwrap();
    for raw in limits.iter_mut().take(2) {
        raw["amount"]["usedFraction"] = json!(1.0);
    }
    limits.swap(0, 1);
    let notices = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(parse(&payload, 2000).unwrap()),
        &settings,
    );
    assert!(notices.is_empty());
    assert!(state.worst().is_none());
    let visible: Vec<_> = state.providers[0]
        .limits
        .iter()
        .map(|limit| limit.id.as_str())
        .collect();
    assert_eq!(visible, ["gemini"]);
    assert_eq!(state.providers[0].limits[0].used, 20.0);
}

#[test]
fn hyphenated_window_labels_do_not_duplicate_into_compound_names() {
    let mut raw = limit("five-hour", json!({"usedFraction": 0.44}));
    raw["label"] = json!("5-Hour");
    let parsed = parse_limit(&raw).unwrap();
    assert_eq!(parsed.label, "5-Hour");
}
