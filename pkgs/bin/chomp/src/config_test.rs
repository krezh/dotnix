use super::*;
use clap::Parser;

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
fn defaults_to_catppuccin_with_optional_color_overrides() {
    let config = Config::default();
    assert_eq!(config.theme, Some(crate::theme::ThemeName::Catppuccin));
    assert!(config.border.color.is_empty());
    assert!(config.mode_select.background_color.is_empty());

    let settings = crate::cli::Args::parse_from(["chomp"]).resolve(config);
    assert_eq!(settings.border_color, "#89B4FA");
    assert_eq!(settings.mode_select.background_color, "#1E1E2E");
    assert_eq!(settings.mode_select.surface_color, "#313244");
    assert_eq!(settings.text_color, "#CDD6F4");
}

#[test]
fn accepts_theme_names_and_color_overrides() {
    let json = r##"{
  "theme": "nord",
  "border": { "color": "#112233" },
  "mode_select": { "background_color": "#445566" }
}"##;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.theme, Some(crate::theme::ThemeName::Nord));

    let settings = crate::cli::Args::parse_from(["chomp"]).resolve(config);
    assert_eq!(settings.border_color, "#112233");
    assert_eq!(settings.mode_select.background_color, "#445566");
    assert_eq!(settings.mode_select.surface_color, "#3B4252");
}

#[test]
fn migrates_generated_pre_theme_colors_to_catppuccin() {
    let json = r##"{
  "border": { "color": "#FFFFFF", "rounding": 15 },
  "mode_select": {
    "background_color": "#0D0D14",
    "description_color": "#FFFFFF",
    "recording_dot_color": "#F24040",
    "recording_highlight_color": "#F2BF33",
    "replay_color": "#38BDF8"
  }
}"##;
    let config: Config = serde_json::from_str(json).unwrap();
    let settings = crate::cli::Args::parse_from(["chomp"]).resolve(config);

    assert_eq!(settings.border_color, "#89B4FA");
    assert_eq!(settings.border_rounding, 15);
    assert_eq!(settings.mode_select.background_color, "#1E1E2E");
    assert_eq!(settings.mode_select.description_color, "#BAC2DE");
    assert_eq!(settings.mode_select.recording_dot_color, "#F38BA8");
    assert_eq!(settings.mode_select.recording_highlight_color, "#F9E2AF");
    assert_eq!(settings.mode_select.replay_color, "#74C7EC");
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
