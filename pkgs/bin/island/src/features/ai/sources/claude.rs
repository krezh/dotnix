use std::{env, fs, path::PathBuf, time::Duration};

use serde_json::Value;

use super::{Failure, TIMEOUT, number, percentage, severity, text, timestamp};
use crate::features::ai::{
    cache,
    state::{Credits, Freshness, Issue, Limit, Provider, Source, SourceReport},
};

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
    expires_at: Option<i64>,
}

pub(super) fn fetch(
    publish_cached: impl FnOnce(Option<String>, Option<Provider>),
) -> Result<SourceReport, Failure> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut login = match discover() {
        Ok(login) => login,
        Err(error) => {
            publish_cached(None, None);
            return Err(error.into());
        }
    };
    let identity = login.identity();
    let snapshot = identity
        .as_deref()
        .and_then(|identity| match cache::load(identity) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!("cannot restore Claude usage cache: {error}");
                None
            }
        });
    publish_cached(identity, snapshot);
    login.check_expiry(now)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .timeout_connect(Some(Duration::from_secs(8)))
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();
    // Account identity must survive token refreshes, including environment-token logins.
    if login.account_id.is_empty() || login.organization_id.is_empty() {
        let profile = get(&agent, PROFILE_URL, &login.token)?;
        apply_profile(&mut login, &profile);
        if login.account_id.is_empty() {
            return Err(
                "Claude usage login has no account identity; sign in with a Claude subscription"
                    .to_owned()
                    .into(),
            );
        }
    }
    let usage = get(&agent, USAGE_URL, &login.token)?;
    let report = parse(&usage, &login, now)?;
    if login.identity().is_some() {
        for provider in &report.providers {
            if let Err(error) = cache::save(provider) {
                tracing::warn!("cannot save Claude usage cache: {error}");
            }
        }
    }
    Ok(report)
}

impl Login {
    fn identity(&self) -> Option<String> {
        if self.account_id.is_empty() || self.organization_id.is_empty() {
            return None;
        }
        serde_json::to_string(&["claude", &self.account_id, &self.organization_id]).ok()
    }

    fn check_expiry(&self, now: i64) -> Result<(), String> {
        if self.expires_at.is_some_and(|expires| expires <= now) {
            Err("Claude OAuth login expired; open Claude Code to renew it".to_owned())
        } else {
            Ok(())
        }
    }
}

fn discover() -> Result<Login, String> {
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
            expires_at: None,
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
    let mut login = parse_oauth(oauth)?;
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
                apply_account(&mut login, account);
            }
        }
    }
    Ok(login)
}

fn apply_account(login: &mut Login, account: &Value) {
    let Some(id) = text(account, "accountUuid") else {
        return;
    };
    if !login.account_id.is_empty() && login.account_id != id {
        return;
    }
    if let Some(organization) = text(account, "organizationUuid") {
        if !login.organization_id.is_empty() && login.organization_id != organization {
            return;
        }
        login.organization_id = organization.to_owned();
    }
    login.account_id = id.to_owned();
    if let Some(email) = text(account, "emailAddress") {
        login.account = email.to_owned();
    }
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

fn parse_oauth(oauth: &Value) -> Result<Login, String> {
    let token = text(oauth, "accessToken").ok_or_else(|| {
        "Claude OAuth login has no access token; sign in to Claude Code again".to_owned()
    })?;
    validate_token(token)?;
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
        expires_at: oauth.get("expiresAt").and_then(Value::as_i64),
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

fn get(agent: &ureq::Agent, url: &'static str, token: &str) -> Result<Value, Failure> {
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
    if status == 429 {
        let asked = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|seconds| seconds.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        return Err(Failure {
            message: http_error(status),
            wait: Some(asked),
        });
    }
    if status != 200 {
        return Err(http_error(status).into());
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_vec()
        .map_err(|_| "Could not read Claude usage API response".to_owned())?;
    serde_json::from_slice(&bytes)
        .map_err(|_| "Claude usage API returned invalid JSON".to_owned().into())
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
        freshness: Freshness::Fresh,
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
        expected: None,
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
#[path = "claude_test.rs"]
mod tests;
