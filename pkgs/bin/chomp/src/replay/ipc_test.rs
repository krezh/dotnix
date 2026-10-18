use super::*;

#[test]
fn save_requests_have_no_response_deadline() {
    let command = ReplayCommand::Save {
        output: None,
        duration_seconds: None,
    };

    assert_eq!(response_timeout(&command), None);
}

#[test]
fn short_control_requests_keep_a_bounded_response_deadline() {
    assert_eq!(
        response_timeout(&ReplayCommand::Status),
        Some(Duration::from_secs(30))
    );
}

#[test]
fn active_app_formats_distinct_class_and_title() {
    let status = ReplayStatus {
        state: ServiceState::Buffering,
        target: Some("steam_app_default — World of Warcraft (0x1234)".to_string()),
        target_app: Some("steam_app_default".to_string()),
        target_title: Some("World of Warcraft".to_string()),
        duration_seconds: 30,
        buffered_millis: 10_000,
        buffered_bytes: 1_000_000,
        message: None,
    };

    assert_eq!(
        status.active_app().as_deref(),
        Some("World of Warcraft (steam_app_default)")
    );
}

#[test]
fn active_app_deduplicates_matching_class_and_title() {
    let status = ReplayStatus {
        state: ServiceState::Buffering,
        target: Some("Battle.net — Battle.net (0x1234)".to_string()),
        target_app: Some("Battle.net".to_string()),
        target_title: Some("Battle.net".to_string()),
        duration_seconds: 30,
        buffered_millis: 10_000,
        buffered_bytes: 1_000_000,
        message: None,
    };

    assert_eq!(status.active_app().as_deref(), Some("Battle.net"));
}

#[test]
fn active_app_handles_missing_title_or_app() {
    let only_app = ReplayStatus {
        target_app: Some("games".to_string()),
        target_title: None,
        ..Default::default()
    };
    assert_eq!(only_app.active_app().as_deref(), Some("games"));

    let only_title = ReplayStatus {
        target_app: None,
        target_title: Some("vkcube".to_string()),
        ..Default::default()
    };
    assert_eq!(only_title.active_app().as_deref(), Some("vkcube"));

    let whitespace_app = ReplayStatus {
        target_app: Some("  ".to_string()),
        target_title: Some("vkcube".to_string()),
        ..Default::default()
    };
    assert_eq!(whitespace_app.active_app().as_deref(), Some("vkcube"));
}

#[test]
fn active_app_falls_back_to_parsing_legacy_target_string() {
    let status = ReplayStatus {
        target: Some("steam_app_default — World of Warcraft (0x55f9a12b34)".to_string()),
        target_app: None,
        target_title: None,
        ..Default::default()
    };
    assert_eq!(
        status.active_app().as_deref(),
        Some("World of Warcraft (steam_app_default)")
    );

    let matching = ReplayStatus {
        target: Some("Battle.net — Battle.net (0x1234)".to_string()),
        target_app: None,
        target_title: None,
        ..Default::default()
    };
    assert_eq!(matching.active_app().as_deref(), Some("Battle.net"));

    let plain = ReplayStatus {
        target: Some("custom_game".to_string()),
        target_app: None,
        target_title: None,
        ..Default::default()
    };
    assert_eq!(plain.active_app().as_deref(), Some("custom_game"));
}

#[test]
fn replay_status_json_deserializes_with_missing_app_fields() {
    let json = r#"{"state":"buffering","target":"game","buffered_millis":5000,"buffered_bytes":100000,"message":null}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();
    assert_eq!(status.state, ServiceState::Buffering);
    assert_eq!(status.target.as_deref(), Some("game"));
    assert_eq!(status.target_app, None);
    assert_eq!(status.target_title, None);
    assert_eq!(status.active_app().as_deref(), Some("game"));
}

#[test]
fn replay_command_save_serializes_and_deserializes_optional_duration() {
    let with_duration = ReplayCommand::Save {
        output: Some(PathBuf::from("/tmp/replay.mp4")),
        duration_seconds: Some(15),
    };
    let json = serde_json::to_string(&with_duration).unwrap();
    let parsed: ReplayCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, with_duration);

    // Legacy JSON without duration_seconds defaults to None
    let legacy_json = r#"{"save":{"output":"/tmp/replay.mp4"}}"#;
    let parsed: ReplayCommand = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(
        parsed,
        ReplayCommand::Save {
            output: Some(PathBuf::from("/tmp/replay.mp4")),
            duration_seconds: None,
        }
    );
}

#[test]
fn replay_command_configure_serializes_and_deserializes() {
    let configure = ReplayCommand::Configure {
        duration_seconds: 60,
    };
    let json = serde_json::to_string(&configure).unwrap();
    let parsed: ReplayCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, configure);
}

#[test]
fn replay_status_carries_duration_seconds() {
    let status = ReplayStatus {
        duration_seconds: 60,
        ..Default::default()
    };
    let json = serde_json::to_string(&status).unwrap();
    let parsed: ReplayStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.duration_seconds, 60);
}
