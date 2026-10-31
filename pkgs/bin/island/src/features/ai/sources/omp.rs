use std::collections::HashMap;

use serde_json::Value;

use super::{TIMEOUT, credits, number, percentage, severity, text, timestamp};
use crate::{
    features::ai::state::{Freshness, Issue, Limit, Provider, Source, SourceReport},
    run,
};

pub(super) fn fetch() -> Result<SourceReport, String> {
    let output = run::output("omp", &["usage", "--json"], TIMEOUT)?;
    if !output.success {
        return Err(match output.code {
            Some(code) => format!("omp usage failed (exit {code})"),
            None => "omp usage was interrupted".to_owned(),
        });
    }
    let value = serde_json::from_str(&output.stdout)
        .map_err(|_| "omp usage returned invalid JSON".to_owned())?;
    parse(&value, chrono::Utc::now().timestamp_millis())
}

fn parse(value: &Value, now: i64) -> Result<SourceReport, String> {
    let reports = value
        .get("reports")
        .and_then(Value::as_array)
        .ok_or_else(|| "omp usage response has no reports array".to_owned())?;
    let generated = value.get("generatedAt").and_then(timestamp).unwrap_or(now);
    let mut result = SourceReport::default();
    let mut accounts = HashMap::new();
    for report in reports {
        let provider = text(report, "provider").ok_or_else(invalid_report)?;
        let limits = report
            .get("limits")
            .and_then(Value::as_array)
            .ok_or_else(invalid_report)?;
        let meta = report.get("metadata").filter(|meta| !meta.is_null());
        if meta.is_some_and(|meta| !meta.is_object()) {
            return Err(invalid_report());
        }
        let meta = meta.unwrap_or(&Value::Null);
        let account_key = text(meta, "accountId")
            .or_else(|| text(meta, "email"))
            .or_else(|| text(meta, "orgId"))
            .unwrap_or("default");
        let id = serde_json::to_string(&[provider, account_key, text(meta, "orgId").unwrap_or("")])
            .map_err(|_| invalid_report())?;
        let account = text(meta, "email")
            .or_else(|| text(meta, "orgName"))
            .or_else(|| text(meta, "accountId"))
            .or_else(|| text(meta, "orgId"))
            .unwrap_or("");
        let fetched_at = report
            .get("fetchedAt")
            .and_then(timestamp)
            .unwrap_or(generated);
        let index = *accounts.entry(id.clone()).or_insert_with(|| {
            let index = result.providers.len();
            result.providers.push(Provider {
                source: Source::Omp,
                id: id.clone(),
                provider_id: provider.to_owned(),
                name: provider_name(provider).to_owned(),
                account: account.to_owned(),
                plan: text(meta, "planType").unwrap_or("").to_owned(),
                limits: Vec::new(),
                credits: report.get("resetCredits").and_then(credits),
                fetched_at,
                freshness: Freshness::Fresh,
            });
            index
        });
        let item = &mut result.providers[index];
        item.fetched_at = item.fetched_at.min(fetched_at);
        if item.credits.is_none() {
            item.credits = report.get("resetCredits").and_then(credits);
        }
        for raw in limits {
            let limit = parse_limit(raw)?;
            if !item.limits.iter().any(|existing| existing.id == limit.id) {
                item.limits.push(limit);
            }
        }
    }
    if let Some(entries) = value
        .get("disabledCredentials")
        .filter(|entries| !entries.is_null())
    {
        for entry in entries.as_array().ok_or_else(invalid_report)? {
            let provider = text(entry, "provider").ok_or_else(invalid_report)?;
            let account = text(entry, "email")
                .or_else(|| text(entry, "orgName"))
                .or_else(|| text(entry, "accountId"))
                .or_else(|| text(entry, "orgId"))
                .or_else(|| text(entry, "type"))
                .unwrap_or("");
            // Causes are enum-like identifiers; never relay arbitrary command diagnostics.
            let detail = match text(entry, "cause") {
                Some("expired") => "Credentials expired",
                Some("invalid") | Some("invalid_grant") => "Credentials invalid",
                Some("revoked") => "Credentials revoked",
                _ => "Credentials disabled",
            };
            result.issues.push(Issue {
                source: Source::Omp,
                account: if account.is_empty() {
                    provider_name(provider).to_owned()
                } else {
                    account.to_owned()
                },
                message: format!("{}: {detail}", provider_name(provider)),
            });
        }
    }
    Ok(result)
}

fn invalid_report() -> String {
    "omp usage response contains an invalid report".to_owned()
}

fn parse_limit(raw: &Value) -> Result<Limit, String> {
    let id = text(raw, "id").ok_or_else(invalid_report)?;
    let amount = raw
        .get("amount")
        .filter(|value| value.is_object())
        .ok_or_else(invalid_report)?;
    let (used, remaining) = normalize_amount(amount)?;
    let window = raw.get("window").unwrap_or(&Value::Null);
    let label = text(raw, "label")
        .or_else(|| text(window, "label"))
        .unwrap_or(id);
    let window_label = text(window, "label").unwrap_or("");
    let alnum = |s: &str| {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>()
    };
    let label = if window_label.is_empty() || alnum(label).contains(&alnum(window_label)) {
        label.to_owned()
    } else {
        format!("{label} · {window_label}")
    };
    // Shared pools keep one ID even when a different model is reported first.
    let id = raw
        .get("scope")
        .and_then(|scope| text(scope, "sharedGroup"))
        .unwrap_or(id);
    Ok(Limit {
        id: id.to_owned(),
        label,
        used,
        remaining,
        resets_at: window.get("resetsAt").and_then(timestamp),
        duration_ms: window
            .get("durationMs")
            .and_then(Value::as_i64)
            .filter(|duration| *duration > 0),
        severity: severity(text(raw, "status"), used),
        exhausts_at: None,
        expected: None,
    })
}

fn normalize_amount(amount: &Value) -> Result<(f64, f64), String> {
    let used = if let Some(fraction) = number(amount, "usedFraction") {
        fraction * 100.0
    } else if let Some(fraction) = number(amount, "remainingFraction") {
        (1.0 - fraction) * 100.0
    } else if let Some(total) = number(amount, "limit").filter(|limit| *limit > 0.0) {
        if let Some(used) = number(amount, "used") {
            used / total * 100.0
        } else if let Some(remaining) = number(amount, "remaining") {
            (total - remaining) / total * 100.0
        } else {
            return Err(invalid_amount());
        }
    } else {
        let scale = match text(amount, "unit") {
            Some("percent" | "percentage" | "%") => 1.0,
            Some("fraction" | "ratio") => 100.0,
            _ => return Err(invalid_amount()),
        };
        if let Some(used) = number(amount, "used") {
            used * scale
        } else if let Some(remaining) = number(amount, "remaining") {
            100.0 - remaining * scale
        } else {
            return Err(invalid_amount());
        }
    };
    percentage(used)
}

fn invalid_amount() -> String {
    "omp usage response has a quota without a usable fraction or limit".to_owned()
}

fn provider_name(id: &str) -> &str {
    match id {
        "openai-codex" => "OpenAI Codex",
        "google-antigravity" => "Google Antigravity",
        "github-copilot" => "GitHub Copilot",
        "anthropic" => "Anthropic",
        "openrouter" => "OpenRouter",
        "deepseek" => "DeepSeek",
        "minimax" => "MiniMax",
        "zai" => "Zhipu AI (GLM)",
        _ => id,
    }
}

#[cfg(test)]
#[path = "omp_test.rs"]
mod tests;
