use std::collections::HashMap;

use super::{Endpoint, Entry, Extras, RawMessage};

#[test]
fn parses_urls_across_schemes_paths_and_stream_suffixes() {
    let root = Endpoint::parse("wss://gotify.example.com").unwrap();
    assert_eq!(
        root,
        Endpoint {
            tls: true,
            authority: "gotify.example.com".to_owned(),
            base_path: String::new(),
        }
    );
    assert_eq!(root.display_url(), "https://gotify.example.com");
    assert_eq!(root.stream_url(), "https://gotify.example.com/stream");
    assert_eq!(
        root.messages_url(50),
        "https://gotify.example.com/message?limit=50"
    );

    assert_eq!(
        Endpoint::parse("https://gotify.example.com/stream/").unwrap(),
        root
    );

    let sub = Endpoint::parse("ws://127.0.0.1:8080/push/stream").unwrap();
    assert_eq!(
        sub,
        Endpoint {
            tls: false,
            authority: "127.0.0.1:8080".to_owned(),
            base_path: "/push".to_owned(),
        }
    );
    assert_eq!(sub.stream_url(), "http://127.0.0.1:8080/push/stream");
    assert_eq!(sub.stream_path(), "/push/stream");
    assert_eq!(sub.delete_url(42), "http://127.0.0.1:8080/push/message/42");
    assert_eq!(sub.delete_all_url(), "http://127.0.0.1:8080/push/message");
    assert_eq!(
        sub.applications_url(),
        "http://127.0.0.1:8080/push/application"
    );

    for invalid in [
        "gotify.example.com",
        "ftp://gotify.example.com",
        "wss:///stream",
    ] {
        assert!(Endpoint::parse(invalid).is_err(), "{invalid}");
    }
}

fn raw(priority: i64) -> RawMessage {
    RawMessage {
        id: 12,
        appid: 5,
        title: "   ".to_owned(),
        message: "Endpoint down".to_owned(),
        priority,
        date: "2026-10-25T16:00:00Z".to_owned(),
        extras: Extras::default(),
    }
}

#[test]
fn resolves_app_names_and_timestamps() {
    let apps = HashMap::from([(5_u64, "Gatus".to_owned())]);
    let entry = Entry::from_raw(raw(8), &apps);
    assert_eq!(entry.app, "Gatus");
    assert_eq!(entry.title, "Gatus");
    assert_eq!(entry.body, "Endpoint down");
    assert!(entry.critical());
    assert_eq!(entry.timestamp, Some(1_792_944_000));
    assert_eq!(Entry::from_raw(raw(8), &HashMap::new()).app, "Gotify");
}

#[test]
fn posts_as_a_notice_marked_with_its_priority() {
    let apps = HashMap::new();
    let notice = |priority| Entry::from_raw(raw(priority), &apps).notice(false);
    assert_eq!(notice(3).detail, None);
    assert_eq!(notice(4).detail.as_deref(), Some("priority 4"));
    assert_eq!(notice(7).detail.as_deref(), Some("priority 7"));
    assert_eq!(notice(8).detail, None);
    assert!(notice(8).critical);
    assert!(!notice(7).critical);

    let posted = notice(5);
    assert_eq!(posted.origin.key, "12");
    assert_eq!(posted.origin.source, "Gotify");
    assert!(posted.via);
    assert!(posted.actions.is_empty());
    assert_eq!(
        posted.received,
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_792_944_000)
    );
}

#[test]
fn a_click_url_in_its_extras_opens_it() {
    let raw: RawMessage = serde_json::from_str(
        r#"{"id": 3, "message": "Backup done", "extras": {
            "client::display": {"contentType": "text/markdown"},
            "client::notification": {"click": {"url": "https://pbs.example.com"}}
        }}"#,
    )
    .unwrap();
    let entry = Entry::from_raw(raw, &HashMap::new());
    assert_eq!(entry.url.as_deref(), Some("https://pbs.example.com"));
    let keys: Vec<_> = entry
        .notice(true)
        .actions
        .into_iter()
        .map(|action| action.key)
        .collect();
    assert_eq!(keys, ["default", "open"]);
}
