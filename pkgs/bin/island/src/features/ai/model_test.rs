use super::*;
use crate::features::ai::state::Credits;

const NOW: i64 = 1_800_000_000_000;

fn report(source: Source, used: f64) -> SourceReport {
    SourceReport {
        providers: vec![Provider {
            source,
            id: "anthropic:account-one".to_owned(),
            provider_id: "anthropic".to_owned(),
            name: "Anthropic".to_owned(),
            account: "one@example.test".to_owned(),
            plan: "Pro".to_owned(),
            limits: vec![Limit {
                id: "five_hour".to_owned(),
                label: "5 hours".to_owned(),
                used,
                remaining: 100.0 - used,
                resets_at: Some(NOW + 3_600_000),
                duration_ms: Some(18_000_000),
                severity: Severity::Ok,
                exhausts_at: None,
            }],
            credits: Some(Credits {
                count: 2,
                expires_at: Some(NOW + 86_400_000),
            }),
            fetched_at: NOW,
            freshness: Freshness::Fresh,
        }],
        issues: Vec::new(),
    }
}

fn settings() -> Settings {
    let mut settings = Settings::default();
    settings.alerts.pace_alerts = false;
    settings
}

#[test]
fn only_quota_transitions_notify_and_a_reset_is_recovery() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Omp,
                Ok(report(Source::Omp, 10.0)),
                &settings
            )
            .is_empty()
    );
    let high = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 80.0)),
        &settings,
    );
    assert_eq!(high.len(), 1);
    assert!(!high[0].critical);
    assert_eq!(state.providers[0].limits[0].severity, Severity::High);
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Omp,
                Ok(report(Source::Omp, 90.0)),
                &settings
            )
            .is_empty()
    );
    let exhausted = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 100.0)),
        &settings,
    );
    assert_eq!(exhausted.len(), 1);
    assert!(exhausted[0].critical);
    let recovered = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 0.0)),
        &settings,
    );
    assert_eq!(recovered.len(), 1);
    assert!(!recovered[0].critical);
    assert_eq!(state.providers[0].limits[0].severity, Severity::Ok);
}

#[test]
fn failed_source_retains_data_and_does_not_announce_false_recovery() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 100.0)),
        &settings,
    );
    tracker.apply(
        &mut state,
        Source::Claude,
        Ok(report(Source::Claude, 10.0)),
        &settings,
    );
    let notices = tracker.apply(
        &mut state,
        Source::Omp,
        Err("source unavailable".to_owned()),
        &settings,
    );
    assert!(notices.is_empty());
    let stale = state
        .providers
        .iter()
        .find(|provider| provider.source == Source::Omp)
        .unwrap();
    assert_eq!(stale.freshness, Freshness::Stale);
    assert_eq!(stale.limits[0].used, 100.0);
    assert_eq!(stale.credits.as_ref().unwrap().count, 2);
    assert!(
        state
            .providers
            .iter()
            .find(|provider| provider.source == Source::Claude)
            .unwrap()
            .freshness
            .is_fresh()
    );
    assert_eq!(state.issues.len(), 1);
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Omp,
                Ok(report(Source::Omp, 100.0)),
                &settings
            )
            .is_empty()
    );
    assert!(state.issues.is_empty());
    assert!(
        state
            .providers
            .iter()
            .find(|provider| provider.source == Source::Omp)
            .unwrap()
            .freshness
            .is_fresh()
    );
}

#[test]
fn clients_and_accounts_have_independent_notification_baselines() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 10.0)),
        &settings,
    );
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Claude,
                Ok(report(Source::Claude, 100.0)),
                &settings
            )
            .is_empty()
    );
    let mut accounts = report(Source::Omp, 10.0);
    let mut second = report(Source::Omp, 100.0).providers.remove(0);
    second.id = "anthropic:account-two".to_owned();
    second.account = "two@example.test".to_owned();
    accounts.providers.push(second);
    assert!(
        tracker
            .apply(&mut state, Source::Omp, Ok(accounts), &settings)
            .is_empty()
    );
    assert_eq!(state.providers.len(), 3);
    assert_eq!(
        state
            .providers
            .iter()
            .filter(|provider| provider.severity() == Severity::Exhausted)
            .count(),
        2
    );
}

#[test]
fn pace_requires_history_and_only_warns_before_the_reset() {
    let settings = Settings::default();
    let mut projected = report(Source::Omp, 50.0)
        .providers
        .remove(0)
        .limits
        .remove(0);
    projected.resets_at = Some(NOW + 7_200_000);
    projected.duration_ms = Some(10_000_000);
    project(&mut projected, NOW, &settings);
    assert_eq!(projected.exhausts_at, Some(NOW + 2_800_000));
    assert_eq!(projected.severity, Severity::High);

    let mut burst = report(Source::Omp, 50.0)
        .providers
        .remove(0)
        .limits
        .remove(0);
    burst.resets_at = Some(NOW + 9_500_000);
    burst.duration_ms = Some(10_000_000);
    project(&mut burst, NOW, &settings);
    assert_eq!(burst.exhausts_at, None);
    assert_eq!(burst.severity, Severity::Ok);

    let mut sustainable = report(Source::Omp, 10.0)
        .providers
        .remove(0)
        .limits
        .remove(0);
    sustainable.resets_at = Some(NOW + 7_200_000);
    sustainable.duration_ms = Some(10_000_000);
    project(&mut sustainable, NOW, &settings);
    assert_eq!(sustainable.exhausts_at, None);
    assert_eq!(sustainable.severity, Severity::Ok);
}

#[test]
fn authoritative_status_cannot_be_lowered_by_threshold_or_disabled_pace_alerts() {
    let mut limit = report(Source::Omp, 20.0)
        .providers
        .remove(0)
        .limits
        .remove(0);
    limit.severity = Severity::Exhausted;
    project(&mut limit, NOW, &settings());
    assert_eq!(limit.severity, Severity::Exhausted);
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let mut muted = settings();
    muted.alerts.notifications = false;
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 0.0)),
        &muted,
    );
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Omp,
                Ok(report(Source::Omp, 100.0)),
                &muted
            )
            .is_empty()
    );
    assert_eq!(state.providers[0].severity(), Severity::Exhausted);
}

#[test]
fn worst_alert_prefers_exhausted_limits_then_the_soonest_known_reset() {
    let mut report = report(Source::Omp, 80.0);
    let mut later = report.providers[0].limits[0].clone();
    later.id = "later".to_owned();
    later.used = 100.0;
    later.severity = Severity::Exhausted;
    later.resets_at = Some(NOW + 7_200_000);
    let mut sooner = later.clone();
    sooner.id = "sooner".to_owned();
    sooner.resets_at = Some(NOW + 1_800_000);
    report.providers[0].limits.extend([later, sooner]);
    let mut state = State::default();
    Tracker::default().apply(&mut state, Source::Omp, Ok(report), &settings());
    assert_eq!(state.worst().unwrap().1.id, "sooner");
    assert_eq!(state.providers[0].headline().unwrap().id, "sooner");
}

#[test]
fn partial_account_failure_keeps_that_accounts_last_quota() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 100.0)),
        &settings,
    );
    let partial = SourceReport {
        providers: Vec::new(),
        issues: vec![Issue {
            source: Source::Omp,
            account: "one@example.test".to_owned(),
            message: "Credentials expired".to_owned(),
        }],
    };
    assert!(
        tracker
            .apply(&mut state, Source::Omp, Ok(partial), &settings)
            .is_empty()
    );
    assert_eq!(state.providers[0].limits[0].severity, Severity::Exhausted);
    assert_eq!(state.providers[0].freshness, Freshness::Stale);
    assert_eq!(
        tracker
            .apply(
                &mut state,
                Source::Omp,
                Ok(report(Source::Omp, 0.0)),
                &settings
            )
            .len(),
        1
    );
}

#[test]
fn exclusions_are_scoped_to_the_harness_and_stable_provider_id() {
    let mut settings = settings();
    settings
        .disabled_limits
        .entry(Source::Omp)
        .or_default()
        .insert(
            "anthropic".to_owned(),
            ["five_hour".to_owned()].into_iter().collect(),
        );
    let mut omp = report(Source::Omp, 100.0);
    omp.providers[0].name = "Renamed account".to_owned();
    let mut other = report(Source::Omp, 100.0).providers.remove(0);
    other.id = "google:account-one".to_owned();
    other.provider_id = "google-antigravity".to_owned();
    omp.providers.push(other);
    let mut tracker = Tracker::default();
    let mut state = State::default();
    tracker.apply(&mut state, Source::Omp, Ok(omp), &settings);
    tracker.apply(
        &mut state,
        Source::Claude,
        Ok(report(Source::Claude, 100.0)),
        &settings,
    );

    let excluded = state
        .providers
        .iter()
        .find(|provider| provider.source == Source::Omp && provider.provider_id == "anthropic")
        .unwrap();
    assert!(excluded.limits.is_empty());
    let other = state
        .providers
        .iter()
        .find(|provider| provider.provider_id == "google-antigravity")
        .unwrap();
    assert_eq!(other.limits[0].severity, Severity::Exhausted);
    let claude = state
        .providers
        .iter()
        .find(|provider| provider.source == Source::Claude)
        .unwrap();
    assert_eq!(claude.limits[0].severity, Severity::Exhausted);
}

#[test]
fn disabling_a_limit_is_silent_and_other_limits_still_notify() {
    let quota_report = |hourly, weekly| {
        let mut value = report(Source::Omp, hourly);
        let mut kept = value.providers[0].limits[0].clone();
        kept.id = "seven_day".to_owned();
        kept.label = "7 days".to_owned();
        kept.used = weekly;
        kept.remaining = 100.0 - weekly;
        kept.duration_ms = Some(7 * 24 * 3_600_000);
        kept.resets_at = Some(NOW + 2 * 24 * 3_600_000);
        value.providers[0].limits.push(kept);
        value
    };
    let mut settings = settings();
    let mut tracker = Tracker::default();
    let mut state = State::default();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(quota_report(100.0, 0.0)),
        &settings,
    );
    settings
        .disabled_limits
        .entry(Source::Omp)
        .or_default()
        .insert(
            "anthropic".to_owned(),
            ["five_hour".to_owned()].into_iter().collect(),
        );

    let high = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(quota_report(0.0, 80.0)),
        &settings,
    );
    assert_eq!(high.len(), 1);
    assert!(!high[0].critical);
    assert_eq!(state.worst().unwrap().1.id, "seven_day");
    let exhausted = tracker.apply(
        &mut state,
        Source::Omp,
        Ok(quota_report(100.0, 100.0)),
        &settings,
    );
    assert_eq!(exhausted.len(), 1);
    assert!(exhausted[0].critical);
}

#[test]
fn restored_snapshot_is_historical_until_a_successful_refresh() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    let mut cached = report(Source::Claude, 100.0).providers.remove(0);
    let identity = cached.id.clone();
    cached.limits[0].resets_at = Some(NOW - 1);
    cached.limits[0].exhausts_at = Some(NOW - 100);
    tracker.restore_claude(&mut state, Some(&identity), Some(cached), &settings);
    assert_eq!(state.providers[0].freshness, Freshness::Cached);
    assert_eq!(state.providers[0].fetched_at, NOW);
    assert_eq!(state.providers[0].limits[0].used, 100.0);
    assert_eq!(state.providers[0].limits[0].resets_at, Some(NOW - 1));
    assert_eq!(state.providers[0].limits[0].exhausts_at, None);
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Claude,
                Err("Claude login expired".into()),
                &settings
            )
            .is_empty()
    );
    assert_eq!(state.providers[0].freshness, Freshness::Stale);
    assert_eq!(state.providers[0].limits[0].used, 100.0);
    assert!(
        tracker
            .apply(
                &mut state,
                Source::Claude,
                Ok(report(Source::Claude, 0.0)),
                &settings
            )
            .is_empty()
    );
    assert_eq!(state.providers[0].freshness, Freshness::Fresh);
    assert_eq!(state.providers[0].limits[0].used, 0.0);
    assert!(state.issues.is_empty());
}

#[test]
fn selecting_another_claude_account_removes_old_usage_even_without_a_snapshot() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    tracker.apply(
        &mut state,
        Source::Omp,
        Ok(report(Source::Omp, 20.0)),
        &settings,
    );
    tracker.apply(
        &mut state,
        Source::Claude,
        Ok(report(Source::Claude, 100.0)),
        &settings,
    );
    tracker.restore_claude(&mut state, Some("other-account"), None, &settings);
    assert!(
        state
            .providers
            .iter()
            .all(|provider| provider.source == Source::Omp)
    );
    assert!(
        tracker
            .apply(&mut state, Source::Claude, Err("offline".into()), &settings)
            .is_empty()
    );
    assert!(
        state
            .providers
            .iter()
            .all(|provider| provider.source == Source::Omp)
    );

    let mut cached = report(Source::Claude, 60.0).providers.remove(0);
    cached.id = "other-account".into();
    tracker.restore_claude(&mut state, Some("other-account"), Some(cached), &settings);
    let restored = state
        .providers
        .iter()
        .find(|provider| provider.source == Source::Claude)
        .unwrap();
    assert_eq!(restored.id, "other-account");
    assert_eq!(restored.limits[0].used, 60.0);
    tracker.restore_claude(&mut state, None, None, &settings);
    assert!(
        state
            .providers
            .iter()
            .all(|provider| provider.source == Source::Omp)
    );
}

#[test]
fn repeated_snapshot_reads_never_replace_live_usage_or_revive_a_failed_refresh() {
    let mut tracker = Tracker::default();
    let mut state = State::default();
    let settings = settings();
    let cached = report(Source::Claude, 100.0).providers.remove(0);
    let identity = cached.id.clone();
    tracker.apply(
        &mut state,
        Source::Claude,
        Ok(report(Source::Claude, 25.0)),
        &settings,
    );
    tracker.restore_claude(&mut state, Some(&identity), Some(cached.clone()), &settings);
    assert_eq!(state.providers[0].limits[0].used, 25.0);
    assert_eq!(state.providers[0].freshness, Freshness::Fresh);
    tracker.apply(&mut state, Source::Claude, Err("expired".into()), &settings);
    tracker.restore_claude(&mut state, Some(&identity), Some(cached), &settings);
    assert_eq!(state.providers[0].limits[0].used, 25.0);
    assert_eq!(state.providers[0].freshness, Freshness::Stale);
    assert_eq!(state.issues[0].message, "expired");
}
