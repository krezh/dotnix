use super::*;

#[test]
fn test_parse_config() {
    let json_str = r##"{
  "font": {
    "family": "JetBrains Mono",
    "size": 14
  },
  "border": {
    "color": "#FF0000"
  },
  "display": {
    "dim_opacity": 0.7
  }
}"##;

    let config: Config = serde_json::from_str(json_str).unwrap();
    assert_eq!(config.font.family, "JetBrains Mono");
    assert_eq!(config.font.size, 14);
    assert_eq!(config.border.color, "#FF0000");
    assert_eq!(config.display.dim_opacity, 0.7);
    // Defaults should still work for unspecified values
    assert_eq!(config.border.thickness, 2);
}

#[test]
fn rejects_invalid_semantic_values() {
    let mut config = Config::default();
    config.display.dim_opacity = 1.5;
    assert!(config.validate().is_err());

    let mut config = Config::default();
    config.capture.video.encode_resolution = "1920".to_string();
    assert!(config.validate().is_err());

    let mut config = Config::default();
    config.keybinds.screenshot_screen = config.keybinds.screenshot_area.clone();
    assert!(config.validate().is_err());
}

#[test]
fn replay_requires_an_explicit_hyprland_tag() {
    let mut config = Config::default();
    config.capture.replay.enabled = true;
    assert!(config.clone().validate().is_err());

    config.capture.replay.hyprland_tag = Some("games".to_string());
    assert!(config.validate().is_ok());
}
#[test]
fn keybinds_detect_replay_save_collision_and_length() {
    let config = Config::default();
    assert!(config.validate().is_ok());

    // Collision with screenshot_area
    let mut collision = Config::default();
    collision.keybinds.replay_save = collision.keybinds.screenshot_area.clone();
    assert!(collision.validate().is_err());

    // Multi-character binding
    let mut multi_char = Config::default();
    multi_char.keybinds.replay_save = "replay".to_string();
    assert!(multi_char.validate().is_err());
}

#[test]
fn mode_select_configuration_validation() {
    let mut config = Config::default();
    config.mode_select.control_height = 0;
    assert!(config.validate().is_err());

    let mut config = Config::default();
    config.mode_select.control_border_opacity = 1.2;
    assert!(config.validate().is_err());

    let mut config = Config::default();
    config.mode_select.replay_color = "not-a-color".to_string();
    assert!(config.validate().is_err());

    let mut config = Config::default();
    config.mode_select.replay_color = "#4CD964".to_string();
    assert!(config.validate().is_ok());
}
