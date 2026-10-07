use super::*;

#[test]
fn resolves_ocr_as_a_region_request() {
    let settings = Args::parse_from(["chomp", "--ocr"]).resolve(Config::default());

    assert!(settings.request.is_ocr());
    assert_eq!(settings.request.mode, None);
}

#[test]
fn preserves_clipboard_destination_without_an_explicit_mode() {
    let settings = Args::parse_from(["chomp", "--clipboard"]).resolve(Config::default());

    assert!(settings.request.to_clipboard());
}

#[test]
fn cli_theme_overrides_the_config_theme() {
    let mut config = Config::default();
    config.theme = Some(ThemeName::Nord);

    let settings = Args::parse_from(["chomp", "--theme", "dracula"]).resolve(config);

    assert_eq!(settings.border_color, "#BD93F9");
    assert_eq!(settings.mode_select.background_color, "#282A36");
}

#[test]
fn rejects_ocr_with_an_explicit_capture_mode() {
    assert!(Args::try_parse_from(["chomp", "--ocr", "--mode", "image-area"]).is_err());
}

#[test]
fn rejects_invalid_visual_values() {
    assert!(Args::try_parse_from(["chomp", "--dim-opacity", "1.2"]).is_err());
    assert!(Args::try_parse_from(["chomp", "--border-color", "not-a-color"]).is_err());
    assert!(Args::try_parse_from(["chomp", "--font-size", "0"]).is_err());
}
