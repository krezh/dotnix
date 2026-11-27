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
fn block_starts_read_as_tools_and_paragraphs() {
    assert_eq!(
        parse(
            r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"WebSearch","input":{}}}}"#
        ),
        Some(Streamed::Tool("WebSearch".to_owned()))
    );
    assert_eq!(
        parse(
            r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}}"#
        ),
        Some(Streamed::Paragraph)
    );
    assert_eq!(
        parse(
            r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}}"#
        ),
        None
    );
    assert_eq!(
        parse(
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{"}}}"#
        ),
        None
    );
}

#[test]
fn a_finished_tool_call_reads_as_what_it_was_used_on() {
    assert_eq!(
        parse(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"WebSearch","input":{"query":"dreadsteed"}}]}}"#
        ),
        Some(Streamed::Used(
            "WebSearch".to_owned(),
            "dreadsteed".to_owned()
        ))
    );
    assert_eq!(
        parse(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"WebFetch","input":{"url":"https://x.org","prompt":"p"}}]}}"#
        ),
        Some(Streamed::Used(
            "WebFetch".to_owned(),
            "https://x.org".to_owned()
        ))
    );
}

#[test]
fn project_folders_follow_claude_codes_naming() {
    assert_eq!(
        project_folder(Path::new("/run/user/1000/island/chat")),
        "-run-user-1000-island-chat"
    );
}
