use std::collections::HashMap;

use super::{Endpoint, Entry, RawMessage, urgency};

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

#[test]
fn maps_priority_to_urgency_and_resolves_app_names_and_timestamps() {
    assert_eq!(urgency(0), 0);
    assert_eq!(urgency(3), 0);
    assert_eq!(urgency(4), 1);
    assert_eq!(urgency(7), 1);
    assert_eq!(urgency(8), 2);

    let apps = HashMap::from([(5_u64, "Gatus".to_owned())]);
    let entry = Entry::from_raw(
        RawMessage {
            id: 12,
            appid: 5,
            title: "   ".to_owned(),
            message: "Endpoint down".to_owned(),
            priority: 8,
            date: "2026-10-25T16:00:00Z".to_owned(),
        },
        &apps,
    );
    assert_eq!(entry.app, "Gatus");
    assert_eq!(entry.title, "Gatus");
    assert_eq!(entry.body, "Endpoint down");
    assert!(entry.critical());
    assert_eq!(entry.timestamp, Some(1_792_944_000));
}
