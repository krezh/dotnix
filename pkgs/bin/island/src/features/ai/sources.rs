use std::time::Duration;

use serde_json::Value;

use super::state::{Credits, Severity, Source, SourceReport};

mod claude;
mod omp;

const TIMEOUT: Duration = Duration::from_secs(20);

pub(super) fn fetch(source: Source) -> Result<SourceReport, String> {
    match source {
        Source::Omp => omp::fetch(),
        Source::Claude => claude::fetch(),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str().filter(|text| !text.is_empty())
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64().filter(|value| value.is_finite())
}

fn timestamp(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| {
            chrono::DateTime::parse_from_rfc3339(value.as_str()?)
                .ok()
                .map(|date| date.timestamp_millis())
        })
        .filter(|time| *time > 0)
}

fn severity(status: Option<&str>, used: f64) -> Severity {
    match status {
        Some("exhausted") => Severity::Exhausted,
        Some("ok") => Severity::Ok,
        Some(_) => Severity::High,
        None if used >= 100.0 => Severity::Exhausted,
        None => Severity::Ok,
    }
}

fn credits(value: &Value) -> Option<Credits> {
    let count = value.get("availableCount")?.as_u64()?;
    if count == 0 {
        return None;
    }
    let expires_at = value
        .get("credits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|credit| text(credit, "status") == Some("available"))
        .filter_map(|credit| credit.get("expiresAt").and_then(timestamp))
        .min();
    Some(Credits { count, expires_at })
}

fn percentage(used: f64) -> Result<(f64, f64), String> {
    if !used.is_finite() || used < 0.0 {
        return Err("Usage response contains an invalid quota amount".to_owned());
    }
    Ok((used, (100.0 - used).max(0.0)))
}
