use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::{Value, json};

use super::model::{PAGE_SIZE, RESULT_WINDOW, SearchRequest, SearchResultItem, SearchResults};
use crate::cache;

const BACKEND: &str = "https://search.nixos.org/backend";
const AUTHORIZATION: &str = "Basic YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=";
const ALIAS_CACHE: &str = "search-alias";
/// Shorter queries would match most of the catalog as a substring.
const WILDCARD_MIN_LEN: usize = 3;

pub(super) struct SearchClient {
    agent: ureq::Agent,
    alias: Option<String>,
    /// The alias came from the cache and has not answered a search this run yet.
    unconfirmed: bool,
    persist: bool,
    backend: String,
}

impl SearchClient {
    pub(super) fn new() -> Self {
        let alias = cache::read(ALIAS_CACHE)
            .map(|alias| alias.trim().to_owned())
            .filter(|alias| alias_version(alias).is_some());
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .into(),
            unconfirmed: alias.is_some(),
            alias,
            persist: true,
            backend: BACKEND.into(),
        }
    }

    #[cfg(test)]
    fn with_backend(backend: String) -> Self {
        Self {
            backend,
            alias: None,
            unconfirmed: false,
            persist: false,
            ..Self::new()
        }
    }

    /// Runs every request in one round trip and returns their pages in the same order.
    pub(super) fn search(
        &mut self,
        requests: &[SearchRequest],
        cancellation: &AtomicBool,
    ) -> Result<Vec<SearchResults>, String> {
        if requests
            .iter()
            .any(|request| request.offset >= RESULT_WINDOW || !request.offset.is_multiple_of(PAGE_SIZE))
        {
            return Err(
                "Search failed: page is outside the 10,000-result window; refine your query".into(),
            );
        }
        let mut result = self.search_inner(requests, cancellation);
        if self.unconfirmed && result.as_ref().is_err_and(|error| error != molt::command::CANCELLED) {
            // A cached alias outlives the catalog index it named once the index is rebuilt.
            self.forget_alias();
            result = self.search_inner(requests, cancellation);
        }
        match &result {
            Ok(_) => self.unconfirmed = false,
            Err(_) => self.forget_alias(),
        }
        result
    }

    fn forget_alias(&mut self) {
        self.alias = None;
        self.unconfirmed = false;
        if self.persist {
            cache::remove(ALIAS_CACHE);
        }
    }

    fn search_inner(
        &mut self,
        requests: &[SearchRequest],
        cancellation: &AtomicBool,
    ) -> Result<Vec<SearchResults>, String> {
        let started = Instant::now();
        check_cancelled(cancellation)?;
        let live: Vec<&SearchRequest> = requests
            .iter()
            .filter(|request| !request.query.trim().is_empty())
            .collect();
        let mut pages = if live.is_empty() {
            Vec::new()
        } else {
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
                let alias = discover_alias(&body)?;
                if self.persist {
                    cache::write(ALIAS_CACHE, &format!("{alias}\n"));
                }
                self.alias = Some(alias);
            }
            let mut payload = String::new();
            for request in &live {
                payload.push_str("{}\n");
                payload.push_str(&query_body(request).to_string());
                payload.push('\n');
            }
            check_cancelled(cancellation)?;
            let response = self
                .agent
                .post(&format!(
                    "{}/{}/_msearch",
                    self.backend,
                    self.alias.as_deref().unwrap()
                ))
                .header("User-Agent", "molt")
                .header("Authorization", AUTHORIZATION)
                .header("Content-Type", "application/x-ndjson")
                .send(payload.as_str());
            check_cancelled(cancellation)?;
            let body = read_response(response)?;
            check_cancelled(cancellation)?;
            parse_responses(&body, live.len(), started.elapsed())?
        }
        .into_iter();
        Ok(requests
            .iter()
            .map(|request| {
                if request.query.trim().is_empty() {
                    SearchResults {
                        total: 0,
                        elapsed: started.elapsed(),
                        items: Vec::new(),
                        revision: None,
                    }
                } else {
                    pages.next().unwrap()
                }
            })
            .collect())
    }
}

/// Exact names first, then names that start with the query, program names, whole words, word
/// prefixes and finally names that contain every word in order.
fn query_body(request: &SearchRequest) -> Value {
    let query = request.query.trim();
    let mut should = vec![
        json!({"term": {"package_attr_name": {"value": query, "boost": 100}}}),
        json!({"term": {"option_name": {"value": query, "boost": 100}}}),
        json!({"term": {"package_programs": {"value": query, "boost": 120}}}),
        json!({"term": {"package_pname": {"value": query, "boost": 60}}}),
        json!({"prefix": {"package_attr_name": {"value": query, "boost": 20}}}),
        json!({"prefix": {"option_name": {"value": query, "boost": 20}}}),
        json!({"multi_match": {
            "query": query,
            "fields": ["package_attr_name^9", "package_pname^6", "package_description^1.3", "package_programs^7.5", "option_name^9", "option_description^2"],
            "type": "best_fields", "operator": "and"
        }}),
        json!({"multi_match": {
            "query": query,
            "fields": ["package_attr_name.edge^4", "package_pname.edge^3", "package_programs.edge^3", "option_name.edge^4", "package_description.edge^0.5", "option_description.edge^0.5"],
            "type": "best_fields", "operator": "and"
        }}),
    ];
    if query.chars().count() >= WILDCARD_MIN_LEN {
        let words: Vec<String> = query.split_whitespace().map(escape_wildcard).collect();
        let pattern = format!("*{}*", words.join("*"));
        for field in ["package_attr_name", "option_name"] {
            should.push(json!({"wildcard": {field: {"value": pattern, "case_insensitive": true, "boost": 4}}}));
        }
    }
    json!({
        "size": PAGE_SIZE,
        "from": request.offset,
        "track_total_hits": true,
        "_source": ["type", "package_attr_name", "package_pversion", "package_description", "package_programs", "package_license_set", "package_homepage", "package_position", "option_name", "option_description", "option_type", "option_default", "option_example", "option_source"],
        "query": {"bool": {
            "filter": [{"terms": {"type": request.scope.types()}}],
            "should": should,
            "minimum_should_match": 1
        }}
    })
}

fn escape_wildcard(word: &str) -> String {
    let mut escaped = String::with_capacity(word.len());
    for character in word.chars() {
        if matches!(character, '*' | '?' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
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

fn alias_version(alias: &str) -> Option<u64> {
    let version = alias
        .strip_prefix("latest-")?
        .strip_suffix("-nixos-unstable")?;
    if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    version.parse().ok()
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
        .filter_map(|entry| Some((alias_version(&entry.alias)?, entry.alias)))
        .max_by_key(|(version, _)| *version)
        .map(|(_, alias)| alias)
        .ok_or_else(|| {
            "Search failed: no unstable catalog alias found; Retry to rediscover the catalog".into()
        })
}

/// Index names end in the nixpkgs commit they were built from, as in
/// `nixos-52-unstable-02c087f3fe673c9e317f206a627cb6b910336dbe`.
fn index_revision(index: &str) -> Option<String> {
    let revision = index.rsplit('-').next()?;
    (revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| revision.to_owned())
}

fn parse_responses(
    body: &str,
    expected: usize,
    elapsed: Duration,
) -> Result<Vec<SearchResults>, String> {
    #[derive(Deserialize)]
    struct Multi {
        responses: Vec<Value>,
    }
    let multi: Multi = serde_json::from_str(body)
        .map_err(|error| format!("Search failed: invalid search response: {error}"))?;
    if multi.responses.len() != expected {
        return Err("Search failed: invalid search response: missing category results".into());
    }
    multi
        .responses
        .into_iter()
        .map(|response| {
            if response.get("error").is_some() {
                let status = response.get("status").and_then(Value::as_u64).unwrap_or(0);
                return Err(format!(
                    "Search failed: catalog rejected the query (status {status}); Retry or refine your query"
                ));
            }
            parse_results(response, elapsed)
        })
        .collect()
}

fn parse_results(response: Value, elapsed: Duration) -> Result<SearchResults, String> {
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
        #[serde(rename = "_index", default)]
        index: Option<String>,
        #[serde(rename = "_source")]
        source: Value,
    }
    let invalid = |error| format!("Search failed: invalid search response: {error}");
    let envelope: Envelope = serde_json::from_value(response).map_err(invalid)?;
    if envelope.timed_out {
        return Err("Search failed: catalog query timed out; Retry or refine your query".into());
    }
    if envelope.hits.total.relation != "eq" {
        return Err("Search failed: catalog returned an inexact result total".into());
    }
    let revision = envelope
        .hits
        .hits
        .first()
        .and_then(|hit| index_revision(hit.index.as_deref()?));
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
        if item.name().trim().is_empty() {
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
        revision,
    })
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
