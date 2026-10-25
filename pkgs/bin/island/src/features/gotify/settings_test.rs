use super::check;

#[test]
fn validates_gotify_toml_settings() {
    let empty: toml::Value = toml::from_str("").unwrap();
    assert!(check(&empty).is_ok());

    let valid: toml::Value = toml::from_str(
        r#"
        url = "wss://gotify.example.com"
        token_file = "/run/secrets/gotify/token"
        min_priority = 4
        auto_delete = true
        "#,
    )
    .unwrap();
    assert!(check(&valid).is_ok());

    for bad in [
        r#"url = "wss://gotify.example.com""#,
        r#"token_file = "/run/secrets/gotify""#,
        r#"url = "not-a-url"
           token_file = "/run/secrets/gotify""#,
        r#"unknown = true"#,
    ] {
        let value: toml::Value = toml::from_str(bad).unwrap();
        assert!(check(&value).is_err(), "{bad}");
    }
}
