use std::{collections::HashMap, sync::Arc};

use super::{
    settings::Settings,
    state::{Issue, Limit, Provider, Severity, Source, SourceReport, State},
};

#[derive(Debug, PartialEq)]
pub(super) struct Notice {
    pub(super) title: String,
    pub(super) body: String,
    pub(super) critical: bool,
}

#[derive(Default)]
pub(super) struct Tracker {
    previous: HashMap<(Source, String, String), Severity>,
}

fn project(limit: &mut Limit, measured_at: i64, settings: &Settings) {
    if limit.used >= 100.0 {
        limit.severity = Severity::Exhausted;
    } else if limit.used >= settings.alerts.high_usage_threshold {
        limit.severity = limit.severity.max(Severity::High);
    }
    limit.exhausts_at = match (limit.resets_at, limit.duration_ms) {
        (Some(reset), Some(duration))
            if limit.severity != Severity::Exhausted
                && duration > 0
                && reset > measured_at
                && (5.0..100.0).contains(&limit.used) =>
        {
            let elapsed = duration.saturating_sub(reset.saturating_sub(measured_at));
            if elapsed as f64 >= duration as f64 * 0.1 && elapsed <= duration {
                let remaining = (100.0 - limit.used) * elapsed as f64 / limit.used;
                let exhaustion = measured_at.saturating_add(remaining as i64);
                (exhaustion < reset).then_some(exhaustion)
            } else {
                None
            }
        }
        _ => None,
    };
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
        title,
        body,
        critical,
    })
}

impl Tracker {
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
                    provider.stale = true;
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
                stale.stale = true;
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
            if !provider.stale {
                for limit in &mut provider.limits {
                    project(limit, provider.fetched_at, settings);
                }
            }
            for limit in &provider.limits {
                let key = (source, provider.id.clone(), limit.id.clone());
                if settings.alerts.notifications && !provider.stale {
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
                next.insert(key, limit.severity);
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
