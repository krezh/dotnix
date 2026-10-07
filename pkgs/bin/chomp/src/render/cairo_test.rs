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
        "#89B4FA",
        "#1E1E2E",
        "#CDD6F4",
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

#[test]
fn selection_hud_hides_only_when_pointer_approaches_it() {
    assert!(selection_hud_pointer_near((960.0, 100.0), 1920));
    assert!(!selection_hud_pointer_near((650.0, 100.0), 1920));
    assert!(!selection_hud_pointer_near((960.0, 123.0), 1920));
}

#[test]
fn selection_hud_proximity_covers_narrow_outputs() {
    assert!(selection_hud_pointer_near((10.0, 40.0), 400));
    assert!(!selection_hud_pointer_near((10.0, 200.0), 400));
}

#[test]
fn mode_palette_intro_scales_from_its_center() {
    fn alpha_bounds(buffer: &[u8], width: usize) -> (usize, usize, usize, usize) {
        let mut left = width;
        let mut top = usize::MAX;
        let mut right = 0;
        let mut bottom = 0;

        for (index, pixel) in buffer.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let x = index % width;
            let y = index / width;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }

        (left, top, right, bottom)
    }

    let config = RenderConfig::new(
        "#89B4FA",
        "#1E1E2E",
        "#CDD6F4",
        2,
        0,
        0.5,
        "Inter Nerd Font".to_string(),
        18,
        FontWeight::Bold,
    )
    .unwrap();
    let renderer = Renderer::new(1000, 700, config);
    let style = crate::config::ModeSelectConfig::default()
        .resolve(crate::theme::ThemeName::Catppuccin.palette());
    let keybinds = crate::config::KeybindsConfig::default();
    let mut entering = vec![0; 1000 * 700 * 4];
    let mut settled = vec![0; 1000 * 700 * 4];

    renderer
        .render_mode_select(
            &mut entering,
            &keybinds,
            &style,
            false,
            true,
            None,
            false,
            None,
            0.0,
        )
        .unwrap();
    renderer
        .render_mode_select(
            &mut settled,
            &keybinds,
            &style,
            false,
            true,
            None,
            false,
            None,
            1.0,
        )
        .unwrap();

    let entering_max_alpha = entering.chunks_exact(4).map(|pixel| pixel[3]).max();
    let settled_max_alpha = settled.chunks_exact(4).map(|pixel| pixel[3]).max();
    assert_eq!(entering_max_alpha, settled_max_alpha);

    let entering = alpha_bounds(&entering, 1000);
    let settled = alpha_bounds(&settled, 1000);
    assert!(entering.2 - entering.0 < settled.2 - settled.0);
    assert!(entering.3 - entering.1 < settled.3 - settled.1);
    assert!((entering.0 + entering.2).abs_diff(settled.0 + settled.2) <= 2);
    assert!((entering.1 + entering.3).abs_diff(settled.1 + settled.3) <= 2);
}
