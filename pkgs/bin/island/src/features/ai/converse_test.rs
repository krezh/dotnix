use std::path::Path;

use super::{Streamed, parse, project_folder};

#[test]
fn stream_lines_read_as_session_text_and_result() {
    assert_eq!(
        parse(r#"{"type":"system","subtype":"init","session_id":"9ebe","tools":[]}"#),
        Some(Streamed::Session("9ebe".to_owned()))
    );
    assert_eq!(
        parse(
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}}"#
        ),
        Some(Streamed::Text("ok".to_owned()))
    );
    assert_eq!(
        parse(r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ok"}]}}"#),
        Some(Streamed::Whole("ok".to_owned()))
    );
    assert_eq!(
        parse(r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#),
        Some(Streamed::Done(Ok(())))
    );
    assert_eq!(
        parse(r#"{"type":"result","is_error":true,"result":"rate limited"}"#),
        Some(Streamed::Done(Err("rate limited".to_owned())))
    );
    assert_eq!(parse(r#"{"type":"rate_limit_event"}"#), None);
    assert_eq!(parse("not json"), None);
}

#[test]
fn project_folders_follow_claude_codes_naming() {
    assert_eq!(
        project_folder(Path::new("/run/user/1000/island/chat")),
        "-run-user-1000-island-chat"
    );
}
