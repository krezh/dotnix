use std::{collections::HashMap, sync::Arc};

use chrono::{Local, TimeZone, Timelike};

use super::{
    forecast,
    hook::Reading,
    settings::Settings,
    state::{Freshness, Issue, Limit, Provider, Severity, Source, SourceReport, State},
};

/// Claude's numbers as Claude Code's status line reads them, laid over the usage API's last
/// reading, which names the account and its other limits. Nothing while there is no such reading
/// yet, or more than one Claude account to choose between. A reading whose reset has passed is
/// from before it, as the status line repeats its last one until Claude Code asks again, so it is
/// left out.
pub(super) fn status_line(state: &State, readings: &[Reading], now: i64) -> Option<SourceReport> {
    let mut claude = state
        .providers
        .iter()
        .filter(|provider| provider.source == Source::Claude);
    let known = claude.next()?;
    let readings: Vec<&Reading> = readings
        .iter()
        .filter(|reading| reading.resets_at.is_none_or(|at| at > now))
        .collect();
    if claude.next().is_some() || readings.is_empty() {
        return None;
    }
    let mut provider = known.clone();
    for reading in readings {
        let Some(limit) = provider
            .limits
            .iter_mut()
            .find(|limit| limit.id == reading.id)
        else {
            continue;
        };
        limit.used = reading.used.clamp(0.0, 100.0);
        limit.remaining = 100.0 - limit.used;
        if reading.resets_at.is_some() {
            limit.resets_at = reading.resets_at;
        }
        limit.severity = if limit.used >= 100.0 {
            Severity::Exhausted
        } else {
            Severity::Ok
        };
        limit.exhausts_at = None;
    }
    provider.fetched_at = now;
    provider.freshness = Freshness::Fresh;
    Some(SourceReport {
        providers: vec![provider],
        issues: Vec::new(),
    })
}

/// The latest reset of a Claude limit that has passed, after which its reading is out of date.
pub(super) fn passed_reset(state: &State, now: i64) -> Option<i64> {
    state
        .providers
        .iter()
        .filter(|provider| provider.source == Source::Claude)
        .flat_map(|provider| &provider.limits)
        .filter_map(|limit| limit.resets_at)
        .filter(|at| *at <= now)
        .max()
}

#[derive(Debug, PartialEq)]
pub(super) struct Notice {
    /// Who it is from, as the notification names it.
    pub(super) app: &'static str,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) critical: bool,
}

#[derive(Default)]
pub(super) struct Tracker {
    previous: HashMap<(Source, String, String), Severity>,
}

/// The local hour of a time, which the forecast counts nights by.
fn local_hour(at: i64) -> u32 {
    Local
        .timestamp_millis_opt(at)
        .single()
        .map_or(12, |time| time.hour())
}

/// `readings` are the window's earlier readings, which the forecast reads the recent pace from.
fn project(limit: &mut Limit, measured_at: i64, readings: &[(i64, f64)], settings: &Settings) {
    if limit.used >= 100.0 {
        limit.severity = Severity::Exhausted;
    } else if limit.used >= settings.alerts.high_usage_threshold {
        limit.severity = limit.severity.max(Severity::High);
    }
    let seen = (limit.severity != Severity::Exhausted)
        .then(|| forecast::forecast(limit, readings, measured_at, local_hour))
        .flatten();
    limit.expected = seen.as_ref().map(|seen| seen.expected);
    limit.exhausts_at = seen.and_then(|seen| seen.exhausts_at);
    if settings.alerts.pace_alerts && limit.exhausts_at.is_some() {
        limit.severity = limit.severity.max(Severity::High);
    }
}

fn notice(provider: &Provider, limit: &Limit, before: Severity) -> Option<Notice> {
    let account = if provider.account.is_empty() {
        String::new()
    } else {
        format!(" · {}", provider.account)
    };
    let subject = format!(
        "{} · {}{account} · {}",
        provider.source.name(),
        provider.name,
        limit.label
    );
    let (title, body, critical) = match limit.severity {
        Severity::Exhausted => (
            format!("{subject} exhausted"),
            "Quota exhausted; see AI usage for the reset time.".to_owned(),
            true,
        ),
        _ if before == Severity::Exhausted => (
            format!("{subject} available again"),
            format!("{:.0}% used", limit.used),
            false,
        ),
        Severity::High => (
            format!("{subject} running high"),
            if limit.exhausts_at.is_some() {
                "On pace to run out before the quota resets.".to_owned()
            } else {
                format!("{:.0}% used", limit.used)
            },
            false,
        ),
        Severity::Ok => return None,
    };
    Some(Notice {
        app: "Island AI",
        title,
        body,
        critical,
    })
}

impl Tracker {
    pub(super) fn restore_claude(
        &mut self,
        state: &mut State,
        identity: Option<&str>,
        snapshot: Option<Provider>,
        settings: &Settings,
    ) {
        Arc::make_mut(&mut state.providers).retain(|provider| {
            provider.source != Source::Claude || Some(provider.id.as_str()) == identity
        });
        self.previous.retain(|(source, id, _), _| {
            *source != Source::Claude || Some(id.as_str()) == identity
        });
        if state
            .providers
            .iter()
            .any(|provider| provider.source == Source::Claude)
        {
            return;
        }
        if let Some(mut provider) = snapshot.filter(|provider| {
            provider.source == Source::Claude && Some(provider.id.as_str()) == identity
        }) {
            provider.freshness = Freshness::Cached;
            for limit in &mut provider.limits {
                limit.exhausts_at = None;
            }
            self.apply(
                state,
                Source::Claude,
                Ok(SourceReport {
                    providers: vec![provider],
                    issues: Vec::new(),
                }),
                settings,
            );
        }
    }

    pub(super) fn apply(
        &mut self,
        state: &mut State,
        source: Source,
        result: Result<SourceReport, String>,
        settings: &Settings,
    ) -> Vec<Notice> {
        let issues = Arc::make_mut(&mut state.issues);
        issues.retain(|issue| issue.source != source);
        let mut report = match result {
            Ok(report) => report,
            Err(message) => {
                for provider in Arc::make_mut(&mut state.providers)
                    .iter_mut()
                    .filter(|provider| provider.source == source)
                {
                    provider.freshness = Freshness::Stale;
                }
                issues.push(Issue {
                    source,
                    account: String::new(),
                    message,
                });
                return Vec::new();
            }
        };
        for previous in state
            .providers
            .iter()
            .filter(|provider| provider.source == source)
        {
            if !previous.account.is_empty()
                && !report
                    .providers
                    .iter()
                    .any(|provider| provider.id == previous.id)
                && report
                    .issues
                    .iter()
                    .any(|issue| issue.account == previous.account)
            {
                let mut stale = previous.clone();
                stale.freshness = Freshness::Stale;
                report.providers.push(stale);
            }
        }
        let mut notices = Vec::new();
        let mut next = HashMap::new();
        let disabled = settings.disabled_limits.get(&source);
        for provider in &mut report.providers {
            if let Some(limits) =
                disabled.and_then(|providers| providers.get(&provider.provider_id))
            {
                provider.limits.retain(|limit| !limits.contains(&limit.id));
            }
            if provider.freshness.is_fresh() {
                for limit in &mut provider.limits {
                    let readings = state
                        .windows
                        .iter()
                        .find(|window| window.provider == provider.id && window.limit == limit.id)
                        .map_or(&[][..], |window| &window.samples[..]);
                    project(limit, provider.fetched_at, readings, settings);
                }
            }
            for limit in &provider.limits {
                let key = (source, provider.id.clone(), limit.id.clone());
                if settings.alerts.notifications && provider.freshness.is_fresh() {
                    if let Some(before) = self
                        .previous
                        .get(&key)
                        .filter(|before| **before != limit.severity)
                    {
                        if let Some(notice) = notice(provider, limit, *before) {
                            notices.push(notice);
                        }
                    }
                }
                if provider.freshness != Freshness::Cached {
                    next.insert(key, limit.severity);
                }
            }
        }
        self.previous
            .retain(|(kept_source, _, _), _| *kept_source != source);
        self.previous.extend(next);
        let providers = Arc::make_mut(&mut state.providers);
        providers.retain(|provider| provider.source != source);
        providers.extend(report.providers);
        issues.extend(report.issues);
        providers.sort_by(|left, right| {
            right
                .severity()
                .cmp(&left.severity())
                .then_with(|| {
                    let used = |provider: &Provider| {
                        provider
                            .limits
                            .iter()
                            .map(|limit| limit.used)
                            .fold(0.0, f64::max)
                    };
                    used(right).total_cmp(&used(left))
                })
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.account.cmp(&right.account))
        });
        notices
    }
}

#[cfg(test)]
#[path = "model_test.rs"]
mod tests;
