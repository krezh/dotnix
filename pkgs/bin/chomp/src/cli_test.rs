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
fn rejects_ocr_with_an_explicit_capture_mode() {
    assert!(Args::try_parse_from(["chomp", "--ocr", "--mode", "image-area"]).is_err());
}

#[test]
fn rejects_invalid_visual_values() {
    assert!(Args::try_parse_from(["chomp", "--dim-opacity", "1.2"]).is_err());
    assert!(Args::try_parse_from(["chomp", "--border-color", "not-a-color"]).is_err());
    assert!(Args::try_parse_from(["chomp", "--font-size", "0"]).is_err());
}
