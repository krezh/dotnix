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

#[test]
fn dimension_pill_uses_space_below_the_selection() {
    let pill = dimension_label_rect(Rect::new(100, 100, 400, 300), 100.0, 30.0, 1920, 1080);

    assert_eq!(
        pill,
        LabelRect {
            x: 250.0,
            y: 408.0,
            width: 100.0,
            height: 30.0,
        }
    );
}

#[test]
fn dimension_pill_moves_above_a_bottom_edge_selection() {
    let pill = dimension_label_rect(Rect::new(100, 900, 400, 160), 100.0, 30.0, 1920, 1080);

    assert_eq!(pill.y, 862.0);
}

#[test]
fn dimension_pill_stays_inside_the_output() {
    let pill = dimension_label_rect(Rect::new(-200, 0, 100, 1080), 120.0, 30.0, 1920, 1080);

    assert_eq!(pill.x, 8.0);
    assert_eq!(pill.y, 8.0);
}
