use std::{env, fs, path::PathBuf, time::Duration};

use serde_json::Value;

use super::{TIMEOUT, number, percentage, severity, text, timestamp};
use crate::features::ai::state::{Credits, Issue, Limit, Provider, Source, SourceReport};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const FIVE_HOURS: i64 = 5 * 60 * 60 * 1000;
const SEVEN_DAYS: i64 = 7 * 24 * 60 * 60 * 1000;

struct Login {
    token: String,
    account_id: String,
    organization_id: String,
    account: String,
    plan: String,
}

pub(super) fn fetch() -> Result<SourceReport, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut login = discover(now)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .timeout_connect(Some(Duration::from_secs(8)))
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();
    let usage = get(&agent, USAGE_URL, &login.token)?;
    // Account identity must survive token refreshes, including environment-token logins.
    if login.account_id.is_empty() {
        let profile = get(&agent, PROFILE_URL, &login.token)?;
        apply_profile(&mut login, &profile);
        if login.account_id.is_empty() {
            return Err(
                "Claude usage login has no account identity; sign in with a Claude subscription"
                    .to_owned(),
            );
        }
    }
    parse(&usage, &login, now)
}

fn discover(now: i64) -> Result<Login, String> {
    if let Some(token) = env::var("CLAUDE_CODE_OAUTH_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
    {
        validate_token(&token)?;
        return Ok(Login {
            token,
            account_id: String::new(),
            organization_id: String::new(),
            account: String::new(),
            plan: String::new(),
        });
    }
    let home = env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        "Claude login discovery requires HOME or CLAUDE_CODE_OAUTH_TOKEN".to_owned()
    })?;
    let configured = env::var_os("CLAUDE_CONFIG_DIR").filter(|path| !path.is_empty());
    let root = configured
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));
    let credentials_root = match env::var_os("CLAUDE_SECURESTORAGE_CONFIG_DIR") {
        Some(path) if path.is_empty() => home.join(".claude"),
        Some(path) => PathBuf::from(path),
        None => root.clone(),
    };
    let bytes = fs::read(credentials_root.join(".credentials.json")).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "No Claude subscription OAuth login; sign in to Claude Code (API keys and cloud-provider auth are unsupported)"
        } else {
            "Cannot read Claude Code credentials"
        }.to_owned()
    })?;
    let credentials: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "Claude Code credentials are not valid JSON".to_owned())?;
    let oauth = credentials.get("claudeAiOauth").filter(|value| value.is_object())
        .ok_or_else(|| "Claude usage requires a subscription OAuth login; API keys and cloud-provider auth are unsupported".to_owned())?;
    let mut login = parse_oauth(oauth, now)?;
    let profile_config = root.join(".config.json");
    let config = if profile_config.is_file() {
        profile_config
    } else if configured.is_some() {
        root.join(".claude.json")
    } else {
        home.join(".claude.json")
    };
    if let Ok(bytes) = fs::read(config) {
        if let Ok(config) = serde_json::from_slice::<Value>(&bytes) {
            if let Some(account) = config.get("oauthAccount") {
                if let Some(id) = text(account, "accountUuid") {
                    login.account_id = id.to_owned();
                }
                if let Some(id) = text(account, "organizationUuid") {
                    login.organization_id = id.to_owned();
                }
                if let Some(email) = text(account, "emailAddress") {
                    login.account = email.to_owned();
                }
            }
        }
    }
    Ok(login)
}

fn validate_token(token: &str) -> Result<(), String> {
    if token.is_empty()
        || token
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err("Claude OAuth token is invalid; sign in to Claude Code again".to_owned());
    }
    Ok(())
}

fn parse_oauth(oauth: &Value, now: i64) -> Result<Login, String> {
    let token = text(oauth, "accessToken").ok_or_else(|| {
        "Claude OAuth login has no access token; sign in to Claude Code again".to_owned()
    })?;
    validate_token(token)?;
    if oauth
        .get("expiresAt")
        .and_then(Value::as_i64)
        .is_some_and(|expires| expires <= now)
    {
        return Err("Claude OAuth login expired; open Claude Code to renew it (Island never refreshes credentials)".to_owned());
    }
    if let Some(scopes) = oauth.get("scopes").and_then(Value::as_array) {
        if !scopes
            .iter()
            .any(|scope| scope.as_str() == Some("user:profile"))
        {
            return Err("Claude OAuth login lacks user:profile permission for usage; sign in to Claude Code again".to_owned());
        }
    }
    Ok(Login {
        token: token.to_owned(),
        account_id: text(oauth, "accountUuid").unwrap_or("").to_owned(),
        organization_id: text(oauth, "organizationUuid").unwrap_or("").to_owned(),
        account: text(oauth, "emailAddress").unwrap_or("").to_owned(),
        plan: text(oauth, "subscriptionType")
            .or_else(|| text(oauth, "rateLimitTier"))
            .unwrap_or("")
            .to_owned(),
    })
}

fn apply_profile(login: &mut Login, profile: &Value) {
    let account = profile.get("account").unwrap_or(&Value::Null);
    let organization = profile.get("organization").unwrap_or(&Value::Null);
    login.account_id = text(account, "uuid")
        .or_else(|| text(profile, "accountUuid"))
        .or_else(|| text(profile, "account_uuid"))
        .unwrap_or("")
        .to_owned();
    login.organization_id = text(organization, "uuid")
        .or_else(|| text(profile, "organizationUuid"))
        .or_else(|| text(profile, "organization_uuid"))
        .unwrap_or("")
        .to_owned();
    login.account = text(account, "email")
        .or_else(|| text(account, "email_address"))
        .or_else(|| text(account, "emailAddress"))
        .or_else(|| text(profile, "email"))
        .or_else(|| text(profile, "email_address"))
        .or_else(|| text(profile, "emailAddress"))
        .or_else(|| text(organization, "name"))
        .unwrap_or("")
        .to_owned();
    if login.plan.is_empty() {
        login.plan = text(organization, "rate_limit_tier")
            .unwrap_or("")
            .strip_prefix("default_claude_")
            .unwrap_or_else(|| text(organization, "rate_limit_tier").unwrap_or(""))
            .to_owned();
    }
}

fn get(agent: &ureq::Agent, url: &'static str, token: &str) -> Result<Value, String> {
    let mut response = agent
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .header("User-Agent", concat!("island/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|_| {
            "Could not reach Claude usage API (connection, TLS, timeout, or redirect failure)"
                .to_owned()
        })?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(http_error(status));
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_vec()
        .map_err(|_| "Could not read Claude usage API response".to_owned())?;
    serde_json::from_slice(&bytes).map_err(|_| "Claude usage API returned invalid JSON".to_owned())
}

fn http_error(status: u16) -> String {
    match status {
        401 => "Claude OAuth login expired or was rejected; sign in to Claude Code again".to_owned(),
        403 => "Claude usage requires a subscription OAuth token with user:profile permission; this login is unsupported or forbidden".to_owned(),
        429 => "Claude usage API is rate limited; wait before refreshing again".to_owned(),
        300..=399 => "Claude usage API redirected; Island does not forward OAuth credentials".to_owned(),
        _ => format!("Claude usage API failed (HTTP {status})"),
    }
}

fn parse(value: &Value, login: &Login, now: i64) -> Result<SourceReport, String> {
    let object = value.as_object().ok_or_else(invalid_usage)?;
    let mut provider = Provider {
        source: Source::Claude,
        id: serde_json::to_string(&["claude", &login.account_id, &login.organization_id])
            .map_err(|_| invalid_usage())?,
        provider_id: "anthropic".to_owned(),
        name: "Anthropic".to_owned(),
        account: if login.account.is_empty() {
            login.account_id.clone()
        } else {
            login.account.clone()
        },
        plan: login.plan.clone(),
        limits: Vec::new(),
        credits: reset_credits(value, now),
        fetched_at: now,
        stale: false,
    };
    let mut recognized = provider.credits.is_some();
    for (key, window) in object {
        let duration = if key == "five_hour" {
            Some(FIVE_HOURS)
        } else if key == "seven_day" || key.starts_with("seven_day_") {
            Some(SEVEN_DAYS)
        } else {
            None
        };
        let Some(duration) = duration else { continue };
        recognized = true;
        if window.is_null() {
            continue;
        }
        if !window.is_object() {
            return Err(invalid_usage());
        }
        let Some(used) = number(window, "utilization") else {
            if window.get("utilization").is_none_or(Value::is_null) {
                continue;
            }
            return Err(invalid_usage());
        };
        let label = match key.as_str() {
            "five_hour" => "5 Hour".to_owned(),
            "seven_day" => "7 Day".to_owned(),
            _ => format!("7 Day · {}", title(key.trim_start_matches("seven_day_"))),
        };
        provider.limits.push(make_limit(
            key,
            label,
            used,
            window.get("resets_at").and_then(timestamp),
            Some(duration),
        )?);
    }
    if let Some(limits) = object.get("limits").filter(|limits| !limits.is_null()) {
        recognized = true;
        for raw in limits.as_array().ok_or_else(invalid_usage)? {
            if !raw.is_object() {
                return Err(invalid_usage());
            }
            if text(raw, "kind") != Some("weekly_scoped") {
                continue;
            }
            let model = raw.pointer("/scope/model").unwrap_or(&Value::Null);
            let name = text(model, "display_name")
                .or_else(|| text(model, "id"))
                .ok_or_else(invalid_usage)?;
            let id = format!(
                "seven_day_{}",
                text(model, "id").unwrap_or(name).to_lowercase()
            );
            let used = number(raw, "percent").ok_or_else(invalid_usage)?;
            let limit = make_limit(
                &id,
                format!("7 Day · {name}"),
                used,
                raw.get("resets_at").and_then(timestamp),
                Some(SEVEN_DAYS),
            )?;
            if let Some(index) = provider
                .limits
                .iter()
                .position(|existing| existing.id == id)
            {
                provider.limits[index] = limit;
            } else {
                provider.limits.push(limit);
            }
        }
    }
    if let Some(extra) = object.get("extra_usage").filter(|extra| !extra.is_null()) {
        recognized = true;
        if !extra.is_object() {
            return Err(invalid_usage());
        }
        if extra.get("is_enabled").and_then(Value::as_bool) == Some(true) {
            let used = number(extra, "utilization").or_else(|| {
                Some(
                    number(extra, "used_credits")?
                        / number(extra, "monthly_limit").filter(|limit| *limit > 0.0)?
                        * 100.0,
                )
            });
            if let Some(used) = used {
                provider.limits.push(make_limit(
                    "extra_usage",
                    "Extra usage · Monthly".to_owned(),
                    used,
                    extra.get("resets_at").and_then(timestamp),
                    None,
                )?);
            }
        }
    }
    if !recognized {
        return Err(invalid_usage());
    }
    let issues = if provider.limits.is_empty() {
        vec![Issue {
            source: Source::Claude,
            account: provider.account.clone(),
            message: "No usage windows reported for this Claude login".to_owned(),
        }]
    } else {
        Vec::new()
    };
    Ok(SourceReport {
        providers: vec![provider],
        issues,
    })
}

fn make_limit(
    id: &str,
    label: String,
    used: f64,
    resets_at: Option<i64>,
    duration_ms: Option<i64>,
) -> Result<Limit, String> {
    let (used, remaining) = percentage(used)?;
    Ok(Limit {
        id: id.to_owned(),
        label,
        used,
        remaining,
        resets_at,
        duration_ms,
        severity: severity(None, used),
        exhausts_at: None,
    })
}

fn title(value: &str) -> String {
    value
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn invalid_usage() -> String {
    "Claude usage API returned an unsupported or invalid usage response".to_owned()
}

fn reset_credits(value: &Value, now: i64) -> Option<Credits> {
    let Some(inventory) = value
        .get("cedar_ember")
        .filter(|inventory| !inventory.is_null())
    else {
        let offer = value.get("juniper_tide")?;
        if offer.get("eligible")?.as_bool()? != true || text(offer, "arm") != Some("reset") {
            return None;
        }
        return offer.get("available")?.as_bool()?.then_some(Credits {
            count: 1,
            expires_at: None,
        });
    };
    if inventory.get("eligible")?.as_bool()? != true {
        return None;
    }
    let grants = inventory.get("grants")?.as_array()?;
    let mut count = 0_u64;
    let mut expires_at: Option<i64> = None;
    for grant in grants {
        if grant.get("paused").and_then(Value::as_bool) != Some(false) {
            continue;
        }
        let Some(left) = grant.get("resets_left").and_then(Value::as_u64) else {
            continue;
        };
        if grant
            .get("resets_total")
            .and_then(Value::as_u64)
            .is_some_and(|total| total < left)
        {
            continue;
        }
        let bounds = ["starts_at", "ends_at"].map(|key| {
            let raw = grant.get(key).filter(|raw| !raw.is_null());
            raw.map(|raw| timestamp(raw).ok_or(())).transpose()
        });
        let [Ok(starts), Ok(ends)] = bounds else {
            continue;
        };
        if starts.is_some_and(|starts| starts > now)
            || ends.is_some_and(|ends| ends <= now)
            || left == 0
        {
            continue;
        }
        count = count.checked_add(left)?;
        if let Some(ends) = ends {
            expires_at = Some(expires_at.map_or(ends, |previous| previous.min(ends)));
        }
    }
    (count > 0).then_some(Credits { count, expires_at })
}

#[cfg(test)]
mod tests {
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
    fn rejects_expired_scopeless_and_malformed_oauth() {
        let valid = json!({"accessToken": "synthetic-token", "expiresAt": 2000,
            "scopes": ["user:profile", "user:inference"], "subscriptionType": "pro"});
        assert_eq!(parse_oauth(&valid, 1000).unwrap().plan, "pro");
        assert!(parse_oauth(&valid, 2000).is_err());
        assert!(
            parse_oauth(
                &json!({"accessToken": "synthetic-token", "scopes": ["user:inference"]}),
                1000
            )
            .is_err()
        );
        assert!(parse_oauth(&json!({"refreshToken": "synthetic-refresh"}), 1000).is_err());
        assert!(validate_token("synthetic\ninjection").is_err());
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
}
