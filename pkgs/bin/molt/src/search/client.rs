use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::{Value, json};

use super::model::{PAGE_SIZE, RESULT_WINDOW, SearchRequest, SearchResultItem, SearchResults};

const BACKEND: &str = "https://search.nixos.org/backend";
const AUTHORIZATION: &str = "Basic YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=";

pub(super) struct SearchClient {
    agent: ureq::Agent,
    alias: Option<String>,
    backend: String,
}

impl SearchClient {
    pub(super) fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .into(),
            alias: None,
            backend: BACKEND.into(),
        }
    }

    #[cfg(test)]
    fn with_backend(backend: String) -> Self {
        Self {
            backend,
            ..Self::new()
        }
    }

    pub(super) fn search(
        &mut self,
        request: &SearchRequest,
        cancellation: &AtomicBool,
    ) -> Result<SearchResults, String> {
        let result = self.search_inner(request, cancellation);
        if result.is_err() {
            self.alias = None;
        }
        result
    }

    fn search_inner(
        &mut self,
        request: &SearchRequest,
        cancellation: &AtomicBool,
    ) -> Result<SearchResults, String> {
        let started = Instant::now();
        check_cancelled(cancellation)?;
        if request.query.trim().is_empty() {
            return Ok(SearchResults {
                total: 0,
                elapsed: started.elapsed(),
                items: Vec::new(),
            });
        }
        if request.offset >= RESULT_WINDOW || !request.offset.is_multiple_of(PAGE_SIZE) {
            return Err(
                "Search failed: page is outside the 10,000-result window; refine your query".into(),
            );
        }
        if self.alias.is_none() {
            let response = self
                .agent
                .get(&format!("{}/_cat/aliases?format=json", self.backend))
                .header("User-Agent", "molt")
                .header("Authorization", AUTHORIZATION)
                .call();
            check_cancelled(cancellation)?;
            let body = read_response(response)?;
            check_cancelled(cancellation)?;
            self.alias = Some(discover_alias(&body)?);
        }
        let payload = json!({
            "size": PAGE_SIZE,
            "from": request.offset,
            "track_total_hits": true,
            "_source": ["type", "package_attr_name", "package_pversion", "package_description", "package_programs", "package_license_set", "package_homepage", "package_position", "option_name", "option_description", "option_type", "option_default", "option_example", "option_source"],
            "query": {"bool": {
                "filter": [{"terms": {"type": request.scope.types()}}],
                "must": [{"multi_match": {
                    "query": request.query,
                    "fields": ["package_attr_name^9", "package_pname^6", "package_description^1.3", "package_programs^7.5", "option_name^9", "option_description^2"],
                    "type": "best_fields", "operator": "and"
                }}]
            }}
        }).to_string();
        check_cancelled(cancellation)?;
        let response = self
            .agent
            .post(&format!(
                "{}/{}/_search",
                self.backend,
                self.alias.as_deref().unwrap()
            ))
            .header("User-Agent", "molt")
            .header("Authorization", AUTHORIZATION)
            .header("Content-Type", "application/json")
            .send(payload.as_str());
        check_cancelled(cancellation)?;
        let body = read_response(response)?;
        check_cancelled(cancellation)?;
        parse_results(&body, started.elapsed())
    }
}

fn check_cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::Relaxed) {
        Err(molt::command::CANCELLED.into())
    } else {
        Ok(())
    }
}

fn read_response(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<String, String> {
    let mut response = result.map_err(|error| match error {
        ureq::Error::StatusCode(status) => {
            format!("Search failed: server returned HTTP {status}; Retry to rediscover the catalog")
        }
        error => format!("Search failed: request transport/timeout error: {error}"),
    })?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Search failed: could not read response: {error}"))
}

fn discover_alias(body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Alias {
        alias: String,
    }
    let aliases: Vec<Alias> = serde_json::from_str(body)
        .map_err(|error| format!("Search failed: invalid catalog discovery response: {error}"))?;
    aliases
        .into_iter()
        .filter_map(|entry| {
            let version = entry
                .alias
                .strip_prefix("latest-")?
                .strip_suffix("-nixos-unstable")?;
            if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let version = version.parse::<u64>().ok()?;
            Some((version, entry.alias))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, alias)| alias)
        .ok_or_else(|| {
            "Search failed: no unstable catalog alias found; Retry to rediscover the catalog".into()
        })
}

fn parse_results(body: &str, elapsed: Duration) -> Result<SearchResults, String> {
    #[derive(Deserialize)]
    struct Envelope {
        timed_out: bool,
        hits: Hits,
    }
    #[derive(Deserialize)]
    struct Hits {
        total: Total,
        hits: Vec<Hit>,
    }
    #[derive(Deserialize)]
    struct Total {
        value: usize,
        relation: String,
    }
    #[derive(Deserialize)]
    struct Hit {
        #[serde(rename = "_source")]
        source: Value,
    }
    let invalid = |error| format!("Search failed: invalid search response: {error}");
    let envelope: Envelope = serde_json::from_str(body).map_err(invalid)?;
    if envelope.timed_out {
        return Err("Search failed: catalog query timed out; Retry or refine your query".into());
    }
    if envelope.hits.total.relation != "eq" {
        return Err("Search failed: catalog returned an inexact result total".into());
    }
    let mut items = Vec::with_capacity(envelope.hits.hits.len());
    for hit in envelope.hits.hits {
        let item = match hit.source.get("type").and_then(Value::as_str) {
            Some("package") => {
                SearchResultItem::Package(serde_json::from_value(hit.source).map_err(invalid)?)
            }
            Some("option") => {
                SearchResultItem::NixOsOption(serde_json::from_value(hit.source).map_err(invalid)?)
            }
            Some("home-manager-option") => SearchResultItem::HomeManagerOption(
                serde_json::from_value(hit.source).map_err(invalid)?,
            ),
            _ => {
                return Err(
                    "Search failed: invalid search response: unsupported result type".into(),
                );
            }
        };
        let name = match &item {
            SearchResultItem::Package(hit) => &hit.attr_name,
            SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
                &hit.name
            }
        };
        if name.trim().is_empty() {
            return Err("Search failed: invalid search response: empty result name".into());
        }
        items.push(item);
    }
    if items.len() > PAGE_SIZE || items.len() > envelope.hits.total.value {
        return Err("Search failed: invalid search response: inconsistent result count".into());
    }
    Ok(SearchResults {
        total: envelope.hits.total.value,
        elapsed,
        items,
    })
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
