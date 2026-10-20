use super::super::model::SearchScope;
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
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

fn response(sources: Vec<Value>, total: usize) -> String {
    json!({"timed_out": false, "hits": {"total": {"value": total, "relation": "eq"}, "hits": sources.into_iter().map(|source| json!({"_source": source})).collect::<Vec<_>>()}}).to_string()
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
fn preserves_variants_nullable_metadata_and_json_values() {
    let body = response(
        vec![
            json!({"type":"package", "package_attr_name":"ripgrep", "package_pversion":null, "package_programs":null, "package_homepage":["ftp://bad", "https://example.org"]}),
            json!({"type":"option", "option_name":"a.enable", "option_default":false, "option_example":0}),
            json!({"type":"home-manager-option", "option_name":"b.list", "option_default":[], "option_example":""}),
        ],
        80,
    );
    let results = parse_results(&body, Duration::ZERO).unwrap();
    assert_eq!(results.total, 80);
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
}

#[test]
fn rejects_invalid_or_partial_responses() {
    for body in [
        "{}".into(),
        response(vec![json!({"type":"package"})], 1),
        response(vec![json!({"type":"option", "option_name":null})], 1),
        response(vec![json!({"type":"darwin-option", "option_name":"a"})], 1),
        response(vec![json!({"type":"option", "option_name":""})], 1),
        r#"{"timed_out":true,"hits":{"total":{"value":0,"relation":"eq"},"hits":[]}}"#.into(),
        r#"{"timed_out":false,"hits":{"total":{"value":10000,"relation":"gte"},"hits":[]}}"#.into(),
    ] {
        assert!(parse_results(&body, Duration::ZERO).is_err(), "{body}");
    }
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
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let (status, body) = if headers.starts_with("GET /_cat/aliases?format=json ") {
                discoveries += 1;
                (200, r#"[{"alias":"latest-52-nixos-unstable"}]"#.into())
            } else {
                assert!(headers.starts_with("POST /latest-52-nixos-unstable/_search "));
                searches += 1;
                let payload: Value =
                    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                let types = payload["query"]["bool"]["filter"][0]["terms"]["type"]
                    .as_array()
                    .unwrap();
                let query = payload["query"]["bool"]["must"][0]["multi_match"]["query"]
                    .as_str()
                    .unwrap();
                if query == "transient" && searches == 5 {
                    (503, "unavailable".into())
                } else {
                    let offset = payload["from"].as_u64().unwrap();
                    let docs = types.iter().map(|kind| match kind.as_str().unwrap() {
                        "package" => json!({"type":"package", "package_attr_name":format!("{query}-{offset}")}),
                        "option" => json!({"type":"option", "option_name":format!("nixos-{offset}")}),
                        "home-manager-option" => json!({"type":"home-manager-option", "option_name":format!("home-{offset}")}),
                        other => panic!("unexpected category {other}"),
                    }).collect();
                    (200, response(docs, 103))
                }
            };
            write!(stream, "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        assert_eq!(discoveries, 2);
        assert_eq!(searches, 6);
    });
    let mut client = SearchClient::with_backend(backend);
    let cancellation = AtomicBool::new(false);
    assert_eq!(
        client
            .search(&request("   ", SearchScope::All, 0), &cancellation)
            .unwrap()
            .total,
        0
    );
    assert_eq!(
        client
            .search(&request("x", SearchScope::All, 0), &AtomicBool::new(true))
            .unwrap_err(),
        molt::command::CANCELLED
    );
    assert!(
        client
            .search(
                &request("x", SearchScope::All, RESULT_WINDOW),
                &cancellation
            )
            .unwrap_err()
            .contains("10,000-result window")
    );
    let page = client
        .search(
            &request("literal \"quotes\"", SearchScope::Packages, 50),
            &cancellation,
        )
        .unwrap();
    assert!(
        matches!(&page.items[..], [SearchResultItem::Package(hit)] if hit.attr_name == "literal \"quotes\"-50")
    );
    let first = client
        .search(
            &request("literal \"quotes\"", SearchScope::Packages, 0),
            &cancellation,
        )
        .unwrap();
    assert!(
        matches!(&first.items[..], [SearchResultItem::Package(hit)] if hit.attr_name == "literal \"quotes\"-0")
    );
    let all = client
        .search(&request("mixed", SearchScope::All, 0), &cancellation)
        .unwrap();
    assert!(matches!(
        &all.items[..],
        [
            SearchResultItem::Package(_),
            SearchResultItem::NixOsOption(_),
            SearchResultItem::HomeManagerOption(_)
        ]
    ));
    let options = client
        .search(
            &request("option", SearchScope::NixOsOptions, 0),
            &cancellation,
        )
        .unwrap();
    assert!(matches!(
        &options.items[..],
        [SearchResultItem::NixOsOption(_)]
    ));
    assert!(
        client
            .search(&request("transient", SearchScope::All, 0), &cancellation)
            .unwrap_err()
            .contains("HTTP 503")
    );
    assert!(client.alias.is_none());
    let retry = client
        .search(
            &request("transient", SearchScope::HomeManagerOptions, 0),
            &cancellation,
        )
        .unwrap();
    assert!(matches!(
        &retry.items[..],
        [SearchResultItem::HomeManagerOption(_)]
    ));
    server.join().unwrap();
}
