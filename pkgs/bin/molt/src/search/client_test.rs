use super::super::model::SearchScope;
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn request(query: &str, scope: SearchScope, offset: usize) -> SearchRequest {
    SearchRequest {
        generation: 1,
        query: query.into(),
        scope,
        offset,
    }
}

fn one(
    client: &mut SearchClient,
    request: SearchRequest,
    cancellation: &AtomicBool,
) -> Result<SearchResults, String> {
    Ok(client
        .search(std::slice::from_ref(&request), cancellation)?
        .remove(0))
}

fn response(sources: Vec<Value>, total: usize) -> Value {
    json!({"timed_out": false, "hits": {"total": {"value": total, "relation": "eq"}, "hits": sources.into_iter().map(|source| json!({"_index": "nixos-52-unstable-02c087f3fe673c9e317f206a627cb6b910336dbe", "_source": source})).collect::<Vec<_>>()}})
}

/// Reads one HTTP request and returns its head and body.
fn read_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let (header_end, length) = loop {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            break (end + 4, length);
        }
    };
    while bytes.len() < header_end + length {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    (
        String::from_utf8_lossy(&bytes[..header_end]).into_owned(),
        bytes[header_end..header_end + length].to_vec(),
    )
}

fn reply(stream: &mut TcpStream, status: u16, body: &str) {
    write!(stream, "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

/// The search bodies of an `_msearch` payload, skipping each one's header line.
fn search_bodies(payload: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(payload)
        .lines()
        .skip(1)
        .step_by(2)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn query_text(body: &Value) -> &str {
    body["query"]["bool"]["should"][0]["term"]["package_attr_name"]["value"]
        .as_str()
        .unwrap()
}

#[test]
fn discovers_highest_numeric_unstable_alias_only() {
    assert_eq!(discover_alias(r#"[{"alias":"latest-9-nixos-unstable"},{"alias":"latest-52-nixos-unstable"},{"alias":"latest-99-nixos-24.11"},{"alias":"latest-100-group-nixos-unstable"},{"alias":"latest--1-nixos-unstable"},{"alias":"latest-+90-nixos-unstable"},{"alias":"latest-x-nixos-unstable"}]"#).unwrap(), "latest-52-nixos-unstable");
    for invalid in [
        "{}",
        "not json",
        "[]",
        r#"[{"alias":"latest-52-nixos-stable"}]"#,
    ] {
        assert!(
            discover_alias(invalid)
                .unwrap_err()
                .starts_with("Search failed:")
        );
    }
}

#[test]
fn query_ranks_exact_prefix_and_substring_matches() {
    let body = query_body(&request("  nginx virt ", SearchScope::NixOsOptions, 50));
    assert_eq!(body["from"], 50);
    assert_eq!(body["query"]["bool"]["filter"][0]["terms"]["type"], json!(["option"]));
    assert_eq!(body["query"]["bool"]["minimum_should_match"], 1);
    let should = body["query"]["bool"]["should"].as_array().unwrap();
    assert_eq!(query_text(&body), "nginx virt");
    assert_eq!(should[1]["term"]["option_name"]["value"], "nginx virt");
    assert_eq!(should[5]["prefix"]["option_name"]["value"], "nginx virt");
    assert_eq!(should[7]["multi_match"]["fields"][3], "option_name.edge^4");
    assert_eq!(
        should[9]["wildcard"]["option_name"],
        json!({"value": "*nginx*virt*", "case_insensitive": true, "boost": 4})
    );
    let short = query_body(&request("rg", SearchScope::All, 0));
    assert!(
        !short["query"]["bool"]["should"]
            .as_array()
            .unwrap()
            .iter()
            .any(|clause| clause.get("wildcard").is_some())
    );
    let escaped = query_body(&request(r"a*b? c\", SearchScope::All, 0));
    assert_eq!(
        escaped["query"]["bool"]["should"][8]["wildcard"]["package_attr_name"]["value"],
        r"*a\*b\?*c\\*"
    );
}

#[test]
fn preserves_variants_nullable_metadata_and_json_values() {
    let body = response(
        vec![
            json!({"type":"package", "package_attr_name":"ripgrep", "package_pversion":null, "package_programs":null, "package_homepage":["ftp://bad", "https://example.org"]}),
            json!({"type":"option", "option_name":"a.enable", "option_default":false, "option_example":0}),
            json!({"type":"home-manager-option", "option_name":"b.list", "option_default":[], "option_example":""}),
        ],
        80,
    );
    let results = parse_results(body, Duration::ZERO).unwrap();
    assert_eq!(results.total, 80);
    assert_eq!(
        results.revision.as_deref(),
        Some("02c087f3fe673c9e317f206a627cb6b910336dbe")
    );
    let SearchResultItem::Package(package) = &results.items[0] else {
        panic!()
    };
    assert_eq!(package.version, None);
    assert_eq!(package.description, None);
    assert_eq!(package.programs, Vec::<String>::new());
    assert_eq!(package.homepage_url(), Some("https://example.org"));
    let SearchResultItem::NixOsOption(option) = &results.items[1] else {
        panic!()
    };
    assert_eq!(option.default_value, json!(false));
    assert_eq!(option.example, json!(0));
    let SearchResultItem::HomeManagerOption(option) = &results.items[2] else {
        panic!()
    };
    assert_eq!(option.default_value, json!([]));
    assert_eq!(option.example, json!(""));
    let mut package = package.clone();
    package.homepage = json!("http://example.org/path");
    assert_eq!(package.homepage_url(), Some("http://example.org/path"));
    package.homepage = Value::Null;
    assert_eq!(package.homepage_url(), None);
    for invalid in [
        "https://",
        "https://#fragment",
        "file:///tmp/example",
        "https://exa mple.org",
    ] {
        package.homepage = json!(invalid);
        assert_eq!(package.homepage_url(), None, "{invalid}");
    }
    assert_eq!(index_revision("nixos-52-unstable"), None);
    assert_eq!(index_revision("nixos-52-unstable-xyz"), None);
}

#[test]
fn rejects_invalid_or_partial_responses() {
    for body in [
        json!({}),
        response(vec![json!({"type":"package"})], 1),
        response(vec![json!({"type":"option", "option_name":null})], 1),
        response(vec![json!({"type":"darwin-option", "option_name":"a"})], 1),
        response(vec![json!({"type":"option", "option_name":""})], 1),
        json!({"timed_out":true,"hits":{"total":{"value":0,"relation":"eq"},"hits":[]}}),
        json!({"timed_out":false,"hits":{"total":{"value":10000,"relation":"gte"},"hits":[]}}),
    ] {
        assert!(parse_results(body.clone(), Duration::ZERO).is_err(), "{body}");
    }
    let page = response(Vec::new(), 0);
    for body in [
        "not json".to_owned(),
        json!({"responses": [page]}).to_string(),
        json!({"responses": [page, {"error": {"type": "x"}, "status": 400}]}).to_string(),
    ] {
        assert!(parse_responses(&body, 2, Duration::ZERO).is_err(), "{body}");
    }
    assert_eq!(
        parse_responses(&json!({"responses": [page, page]}).to_string(), 2, Duration::ZERO)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn loopback_filters_literal_queries_pages_and_manual_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let backend = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut discoveries = 0;
        let mut searches = 0;
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().unwrap();
            let (headers, payload) = read_request(&mut stream);
            let (status, body) = if headers.starts_with("GET /_cat/aliases?format=json ") {
                discoveries += 1;
                (200, r#"[{"alias":"latest-52-nixos-unstable"}]"#.into())
            } else {
                assert!(headers.starts_with("POST /latest-52-nixos-unstable/_msearch "));
                searches += 1;
                let bodies = search_bodies(&payload);
                if query_text(&bodies[0]) == "transient" && searches == 5 {
                    (503, "unavailable".into())
                } else {
                    let responses: Vec<Value> = bodies
                        .iter()
                        .map(|body| {
                            let query = query_text(body);
                            let offset = body["from"].as_u64().unwrap();
                            let docs = body["query"]["bool"]["filter"][0]["terms"]["type"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|kind| match kind.as_str().unwrap() {
                                    "package" => json!({"type":"package", "package_attr_name":format!("{query}-{offset}")}),
                                    "option" => json!({"type":"option", "option_name":format!("nixos-{offset}")}),
                                    "home-manager-option" => json!({"type":"home-manager-option", "option_name":format!("home-{offset}")}),
                                    other => panic!("unexpected category {other}"),
                                })
                                .collect();
                            response(docs, 103)
                        })
                        .collect();
                    (200, json!({"responses": responses}).to_string())
                }
            };
            reply(&mut stream, status, &body);
        }
        assert_eq!(discoveries, 2);
        assert_eq!(searches, 6);
    });
    let mut client = SearchClient::with_backend(backend);
    let cancellation = AtomicBool::new(false);
    assert_eq!(
        one(&mut client, request("   ", SearchScope::All, 0), &cancellation)
            .unwrap()
            .total,
        0
    );
    assert_eq!(
        one(&mut client, request("x", SearchScope::All, 0), &AtomicBool::new(true)).unwrap_err(),
        molt::command::CANCELLED
    );
    assert!(
        one(
            &mut client,
            request("x", SearchScope::All, RESULT_WINDOW),
            &cancellation
        )
        .unwrap_err()
        .contains("10,000-result window")
    );
    let page = one(
        &mut client,
        request("literal \"quotes\"", SearchScope::Packages, 50),
        &cancellation,
    )
    .unwrap();
    assert!(
        matches!(&page.items[..], [SearchResultItem::Package(hit)] if hit.attr_name == "literal \"quotes\"-50")
    );
    let first = one(
        &mut client,
        request("literal \"quotes\"", SearchScope::Packages, 0),
        &cancellation,
    )
    .unwrap();
    assert!(
        matches!(&first.items[..], [SearchResultItem::Package(hit)] if hit.attr_name == "literal \"quotes\"-0")
    );
    let batch = client
        .search(
            &SearchScope::ALL.map(|scope| request("mixed", scope, 0)),
            &cancellation,
        )
        .unwrap();
    assert!(matches!(
        &batch[0].items[..],
        [
            SearchResultItem::Package(_),
            SearchResultItem::NixOsOption(_),
            SearchResultItem::HomeManagerOption(_)
        ]
    ));
    assert!(matches!(&batch[1].items[..], [SearchResultItem::Package(_)]));
    assert!(matches!(&batch[3].items[..], [SearchResultItem::HomeManagerOption(_)]));
    let options = one(
        &mut client,
        request("option", SearchScope::NixOsOptions, 0),
        &cancellation,
    )
    .unwrap();
    assert!(matches!(
        &options.items[..],
        [SearchResultItem::NixOsOption(_)]
    ));
    assert!(
        one(&mut client, request("transient", SearchScope::All, 0), &cancellation)
            .unwrap_err()
            .contains("HTTP 503")
    );
    assert!(client.alias.is_none());
    let retry = one(
        &mut client,
        request("transient", SearchScope::HomeManagerOptions, 0),
        &cancellation,
    )
    .unwrap();
    assert!(matches!(
        &retry.items[..],
        [SearchResultItem::HomeManagerOption(_)]
    ));
    server.join().unwrap();
}

#[test]
fn stale_cached_alias_is_rediscovered_once() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let backend = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut paths = Vec::new();
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let (headers, _) = read_request(&mut stream);
            let path = headers.split(' ').nth(1).unwrap().to_owned();
            match path.as_str() {
                "/latest-9-nixos-unstable/_msearch" => reply(&mut stream, 404, "gone"),
                "/_cat/aliases?format=json" => {
                    reply(&mut stream, 200, r#"[{"alias":"latest-52-nixos-unstable"}]"#);
                }
                _ => {
                    let page = response(vec![json!({"type":"package", "package_attr_name":"hit"})], 1);
                    reply(&mut stream, 200, &json!({"responses": [page]}).to_string());
                }
            }
            paths.push(path);
        }
        assert_eq!(
            paths,
            [
                "/latest-9-nixos-unstable/_msearch",
                "/_cat/aliases?format=json",
                "/latest-52-nixos-unstable/_msearch"
            ]
        );
    });
    let mut client = SearchClient::with_backend(backend);
    client.alias = Some("latest-9-nixos-unstable".into());
    client.unconfirmed = true;
    let page = one(
        &mut client,
        request("hit", SearchScope::Packages, 0),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(client.alias.as_deref(), Some("latest-52-nixos-unstable"));
    assert!(!client.unconfirmed);
    server.join().unwrap();
}
