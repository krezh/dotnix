use std::time::Duration;

use super::{Settings, check};
use crate::features::parse_settings;

#[test]
fn asks_the_server_again_every_half_minute_unless_told_otherwise() {
    let parse = |toml: &str| parse_settings::<Settings>(&toml::from_str(toml).unwrap()).unwrap();
    assert_eq!(parse("").sync_interval(), Some(Duration::from_secs(30)));
    assert_eq!(
        parse("sync_seconds = 120").sync_interval(),
        Some(Duration::from_secs(120))
    );
    assert_eq!(parse("sync_seconds = 0").sync_interval(), None);
}

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
