use super::model_name;

#[test]
fn model_ids_read_as_names() {
    assert_eq!(model_name("claude-haiku-5-5"), "Haiku 5.5");
    assert_eq!(model_name("claude-opus-5-5[1m]"), "Opus 5.5");
    assert_eq!(model_name("claude-sonnet-4-5-20250929"), "Sonnet 4.5");
    assert_eq!(model_name("gpt-5"), "gpt-5");
}
