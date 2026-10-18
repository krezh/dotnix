use std::collections::HashMap;

use serde_json::Value;

use super::{TIMEOUT, credits, number, percentage, severity, text, timestamp};
use crate::{
    features::ai::state::{Issue, Limit, Provider, Source, SourceReport},
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
                stale: false,
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
    let label =
        if window_label.is_empty() || label.to_lowercase().contains(&window_label.to_lowercase()) {
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
mod tests {
    use super::*;
    use crate::features::ai::state::Severity;
    use serde_json::json;

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
        use crate::features::ai::{model::Tracker, settings::Settings, state::State};

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
}
