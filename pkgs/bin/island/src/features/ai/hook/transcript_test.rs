use super::last_model;

#[test]
fn the_last_claude_model_named_wins() {
    let tail = concat!(
        r#"{"type":"assistant","message":{"model":"claude-sonnet-5-5","content":[]}}"#,
        "\n",
        r#"{"type":"assistant","message":{"model":"<synthetic>","content":[]}}"#,
        "\n",
        r#"{"type":"assistant","message":{"model":"claude-opus-5-5","content":[]}}"#,
        "\n",
    );
    assert_eq!(last_model(tail).as_deref(), Some("claude-opus-5-5"));
    assert_eq!(last_model(r#"{"type":"user"}"#), None);
}
