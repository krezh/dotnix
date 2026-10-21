use super::*;
use crate::features::ai::state::Severity;
use serde_json::json;

fn login() -> Login {
    Login {
        token: "synthetic-test-token".to_owned(),
        account_id: "account-one".to_owned(),
        organization_id: "org-one".to_owned(),
        account: "test@example.test".to_owned(),
        plan: "max".to_owned(),
        expires_at: None,
    }
}

#[test]
fn parses_real_windows_without_filling_missing_ones() {
    let parsed = parse(
        &json!({
            "five_hour": {"utilization": 0.5, "resets_at": "2027-01-01T05:00:00Z"},
            "seven_day": null,
            "seven_day_sonnet": {"utilization": 100, "resets_at": null},
            "seven_day_opus": {"resets_at": "2027-01-01T00:00:00Z"},
            "seven_day_oauth_apps": {"utilization": 20}
        }),
        &login(),
        1000,
    )
    .unwrap();
    let provider = &parsed.providers[0];
    assert_eq!(provider.limits.len(), 3);
    let five = provider
        .limits
        .iter()
        .find(|limit| limit.id == "five_hour")
        .unwrap();
    assert_eq!(five.used, 0.5);
    assert_eq!(five.remaining, 99.5);
    assert_eq!(five.duration_ms, Some(FIVE_HOURS));
    assert_eq!(five.resets_at, timestamp(&json!("2027-01-01T05:00:00Z")));
    let sonnet = provider
        .limits
        .iter()
        .find(|limit| limit.id == "seven_day_sonnet")
        .unwrap();
    assert_eq!(sonnet.severity, Severity::Exhausted);
    assert_eq!(sonnet.duration_ms, Some(SEVEN_DAYS));
    assert_eq!(sonnet.resets_at, None);
    assert_eq!(provider.fetched_at, 1000);
    assert_eq!(provider.plan, "max");
}

#[test]
fn handles_structured_scoped_windows_and_extra_usage() {
    let parsed = parse(
        &json!({
            "five_hour": {"utilization": 10},
            "seven_day_sonnet": {"utilization": 15},
            "limits": [
                {"kind": "weekly_scoped", "percent": 30, "is_active": false,
                    "scope": {"model": {"id": "sonnet", "display_name": "Sonnet"}}},
                {"kind": "weekly_scoped", "percent": 60,
                    "scope": {"model": {"display_name": "Fable"}}}
            ],
            "extra_usage": {"is_enabled": true, "used_credits": 250, "monthly_limit": 1000}
        }),
        &login(),
        1000,
    )
    .unwrap();
    let limits = &parsed.providers[0].limits;
    assert_eq!(limits.len(), 4);
    assert_eq!(
        limits
            .iter()
            .find(|limit| limit.id == "seven_day_sonnet")
            .unwrap()
            .used,
        30.0
    );
    assert_eq!(
        limits
            .iter()
            .find(|limit| limit.id == "seven_day_fable")
            .unwrap()
            .used,
        60.0
    );
    assert_eq!(
        limits
            .iter()
            .find(|limit| limit.id == "extra_usage")
            .unwrap()
            .used,
        25.0
    );
    assert_eq!(
        limits
            .iter()
            .find(|limit| limit.id == "extra_usage")
            .unwrap()
            .duration_ms,
        None
    );
}

#[test]
fn disabled_or_unbounded_extra_usage_never_becomes_zero_quota() {
    for extra in [
        json!({"is_enabled": false, "utilization": 100}),
        json!({"is_enabled": true, "used_credits": 20, "monthly_limit": null}),
        json!({"is_enabled": true, "used_credits": 20, "monthly_limit": 0}),
    ] {
        let parsed = parse(
            &json!({"five_hour": {"utilization": 10}, "extra_usage": extra}),
            &login(),
            1000,
        )
        .unwrap();
        assert_eq!(parsed.providers[0].limits.len(), 1);
    }
}

#[test]
fn valid_empty_windows_are_visible_but_invalid_shapes_fail() {
    let parsed = parse(
        &json!({"five_hour": null, "seven_day": {"utilization": null}}),
        &login(),
        1000,
    )
    .unwrap();
    assert!(parsed.providers[0].limits.is_empty());
    assert_eq!(parsed.issues.len(), 1);
    for invalid in [
        json!(null),
        json!({}),
        json!({"error": {"message": "private diagnostic"}}),
        json!({"five_hour": []}),
        json!({"five_hour": {"utilization": "50"}}),
        json!({"five_hour": {"utilization": -1}}),
        json!({"limits": {}}),
        json!({"limits": [{"kind": "weekly_scoped", "percent": 20}]}),
    ] {
        assert!(parse(&invalid, &login(), 1000).is_err());
    }
}

#[test]
fn identity_does_not_depend_on_token_email_or_usage() {
    let first = login();
    let mut changed = login();
    changed.token = "a-different-synthetic-token".to_owned();
    changed.account = "new@example.test".to_owned();
    let first_id = parse(&json!({"five_hour": {"utilization": 10}}), &first, 1000)
        .unwrap()
        .providers[0]
        .id
        .clone();
    assert_eq!(
        first_id,
        parse(&json!({"five_hour": {"utilization": 90}}), &changed, 2000)
            .unwrap()
            .providers[0]
            .id
    );
    changed.account_id = "account-two".to_owned();
    assert_ne!(
        first_id,
        parse(&json!({"five_hour": null}), &changed, 2000)
            .unwrap()
            .providers[0]
            .id
    );
}

#[test]
fn expired_login_keeps_identity_but_blocks_fetch_until_renewed() {
    let mut oauth = json!({"accessToken": "synthetic-token", "expiresAt": 2000,
            "scopes": ["user:profile", "user:inference"], "subscriptionType": "pro",
            "accountUuid": "one", "organizationUuid": "org"});
    let expired = parse_oauth(&oauth).unwrap();
    assert!(expired.check_expiry(2000).is_err());
    assert!(expired.check_expiry(1999).is_ok());
    assert_eq!(
        expired.identity().as_deref(),
        Some("[\"claude\",\"one\",\"org\"]")
    );
    oauth["expiresAt"] = json!(3000);
    let renewed = parse_oauth(&oauth).unwrap();
    assert!(renewed.check_expiry(2000).is_ok());
    assert_eq!(renewed.identity(), expired.identity());
    assert!(
        parse_oauth(&json!({"accessToken": "synthetic-token",
            "scopes": ["user:inference"]}))
        .is_err()
    );
    assert!(parse_oauth(&json!({"refreshToken": "synthetic-refresh"})).is_err());
    assert!(validate_token("synthetic\ninjection").is_err());
}

#[test]
fn stale_profile_config_cannot_relabel_a_different_login() {
    let mut login = login();
    let before = login.identity();
    apply_account(
        &mut login,
        &json!({"accountUuid": "other-account",
            "organizationUuid": "other-org", "emailAddress": "other@example.test"}),
    );
    assert_eq!(login.identity(), before);
    assert_eq!(login.account, "test@example.test");
    apply_account(
        &mut login,
        &json!({"accountUuid": "account-one",
            "organizationUuid": "other-org"}),
    );
    assert_eq!(login.identity(), before);
}

#[test]
fn reads_profile_metadata_for_environment_logins() {
    let mut login = login();
    login.account_id.clear();
    login.plan.clear();
    apply_profile(
        &mut login,
        &json!({"account": {"uuid": "remote-account", "email": "remote@example.test"},
            "organization": {"uuid": "remote-org", "rate_limit_tier": "default_claude_max_20x"}}),
    );
    assert_eq!(login.account_id, "remote-account");
    assert_eq!(login.organization_id, "remote-org");
    assert_eq!(login.account, "remote@example.test");
    assert_eq!(login.plan, "max_20x");
}

#[test]
fn counts_only_available_reset_grants_and_nearest_expiry() {
    let now = timestamp(&json!("2027-01-02T00:00:00Z")).unwrap();
    let inventory = json!({"cedar_ember": {"eligible": true, "grants": [
        {"resets_left": 2, "paused": false, "ends_at": "2027-01-05T00:00:00Z"},
        {"resets_left": 1, "paused": false, "ends_at": "2027-01-03T00:00:00Z"},
        {"resets_left": 9, "paused": true},
        {"resets_left": 9, "paused": false, "starts_at": "2027-01-03T00:00:00Z"},
        {"resets_left": 9, "paused": false, "ends_at": "2027-01-01T00:00:00Z"},
        {"resets_left": 9, "paused": false, "ends_at": "not a date"},
        {"resets_left": 9},
        {"resets_left": 9, "resets_total": 1, "paused": false}
    ]}});
    let credits = reset_credits(&inventory, now).unwrap();
    assert_eq!(credits.count, 3);
    assert_eq!(
        credits.expires_at,
        timestamp(&json!("2027-01-03T00:00:00Z"))
    );
    assert!(
        reset_credits(
            &json!({"cedar_ember": {"eligible": false, "grants": []}}),
            now
        )
        .is_none()
    );
    assert!(reset_credits(&json!({}), now).is_none());
}

#[test]
fn parses_explicit_weekly_reset_offer_without_inventing_expiry() {
    let credits = reset_credits(
        &json!({"juniper_tide": {
            "eligible": true, "arm": "reset", "available": true,
            "next_available_at": "2027-01-03T00:00:00Z"
        }}),
        1000,
    )
    .unwrap();
    assert_eq!(credits.count, 1);
    assert_eq!(credits.expires_at, None);
    assert!(
        reset_credits(
            &json!({"juniper_tide": {"eligible": true, "available": true}}),
            1000
        )
        .is_none()
    );
}
