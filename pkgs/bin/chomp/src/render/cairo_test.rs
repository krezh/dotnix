use super::*;

#[test]
fn test_color_from_hex() {
    let color = Color::from_hex("#FF8800").unwrap();
    assert!((color.r - 1.0).abs() < 0.01);
    assert!((color.g - 0.533).abs() < 0.01);
    assert!((color.b - 0.0).abs() < 0.01);

    let color = Color::from_hex("#FFF").unwrap();
    assert!((color.r - 1.0).abs() < 0.01);
    assert!((color.g - 1.0).abs() < 0.01);
    assert!((color.b - 1.0).abs() < 0.01);
}

#[test]
fn test_renderer_creation() {
    let config = RenderConfig::new(
        "#FFFFFF",
        2,
        0,
        0.5,
        "Inter Nerd Font".to_string(),
        18,
        FontWeight::Bold,
    )
    .unwrap();
    let renderer = Renderer::new(1920, 1080, config);
    assert_eq!(renderer.width, 1920);
    assert_eq!(renderer.height, 1080);
}
