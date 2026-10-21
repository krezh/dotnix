use serde_json::json;

use super::{after, before};
use crate::features::ai::session::{LineKind, Live};

#[test]
fn a_read_shows_its_lines_numbered_from_where_it_started() {
    let response = json!({"type": "text", "file": {
        "filePath": "/x/src/features/ai/overview.rs",
        "content": "fn a() {}\nfn b() {}",
        "startLine": 27,
    }});
    let Some(Live::Reading { file, lines }) = after("Read", &json!({}), &response) else {
        panic!("expected a read");
    };
    assert_eq!(file, "ai/overview.rs");
    assert_eq!(lines[0].number, Some(27));
    assert_eq!(lines[1].number, Some(28));
    assert_eq!(lines[1].text, "fn b() {}");
}

#[test]
fn an_edit_follows_its_patch_with_the_new_numbering() {
    let response = json!({
        "filePath": "/x/src/sessions.rs",
        "structuredPatch": [{"oldStart": 10, "oldLines": 3, "newStart": 10, "newLines": 3, "lines": [
            "     keep();",
            "-    old();",
            "+    new();",
            "     after();",
        ]}],
    });
    let Some(Live::Editing { file, lines }) = after("Edit", &json!({}), &response) else {
        panic!("expected an edit");
    };
    assert_eq!(file, "src/sessions.rs");
    let kinds: Vec<_> = lines.iter().map(|line| (line.number, line.kind)).collect();
    assert_eq!(
        kinds,
        [
            (Some(10), LineKind::Same),
            (Some(11), LineKind::Removed),
            (Some(11), LineKind::Added),
            (Some(12), LineKind::Same),
        ]
    );
    assert_eq!(lines[2].text, "    new();");
}

#[test]
fn a_command_runs_then_keeps_the_end_of_its_output() {
    let input = json!({"command": "cargo test"});
    assert_eq!(
        before("Bash", &input),
        Some(Live::Running {
            command: "cargo test".to_owned(),
            output: Vec::new(),
            done: false,
        })
    );
    let printed: String = (1..=12).map(|n| format!("line {n}\n")).collect();
    let Some(Live::Running { output, done, .. }) =
        after("Bash", &input, &json!({"stdout": printed, "stderr": ""}))
    else {
        panic!("expected a command");
    };
    assert!(done);
    assert_eq!(output.first().map(String::as_str), Some("line 5"));
    assert_eq!(output.len(), 8);
}
