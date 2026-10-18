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

#[test]
fn replay_heading_shows_active_app_and_state() {
    use crate::replay::{ReplayStatus, ServiceState};

    // Unavailable when status is None
    let (text, active) = format_replay_heading(None);
    assert_eq!(text, "Replay unavailable");
    assert!(!active);

    // Waiting for target without active app
    let status = ReplayStatus {
        state: ServiceState::WaitingForTarget,
        ..Default::default()
    };
    let (text, active) = format_replay_heading(Some(&status));
    assert_eq!(text, "Replay waiting for target");
    assert!(!active);

    // Waiting for target with active app being probed
    let status = ReplayStatus {
        state: ServiceState::WaitingForTarget,
        target_app: Some("steam_app_default".to_string()),
        target_title: Some("World of Warcraft".to_string()),
        ..Default::default()
    };
    let (text, active) = format_replay_heading(Some(&status));
    assert_eq!(
        text,
        "Replay waiting for target · World of Warcraft (steam_app_default)"
    );
    assert!(!active);

    // Buffering with frames ready
    let status = ReplayStatus {
        state: ServiceState::Buffering,
        target_app: Some("steam_app_default".to_string()),
        target_title: Some("World of Warcraft".to_string()),
        buffered_millis: 15_400,
        buffered_bytes: 5_000_000,
        ..Default::default()
    };
    let (text, active) = format_replay_heading(Some(&status));
    assert_eq!(
        text,
        "Replay ready · 15.4s · World of Warcraft (steam_app_default)"
    );
    assert!(active);

    // Retaining after exit retains frames and identifies the exited app
    let status = ReplayStatus {
        state: ServiceState::RetainingAfterExit,
        target_app: Some("Battle.net".to_string()),
        target_title: Some("Battle.net".to_string()),
        buffered_millis: 30_000,
        buffered_bytes: 10_000_000,
        ..Default::default()
    };
    let (text, active) = format_replay_heading(Some(&status));
    assert_eq!(text, "Replay retained · 30.0s · Battle.net");
    assert!(active);
}

#[test]
fn truncate_to_width_keeps_short_text_and_truncates_overflow() {
    let surface = ImageSurface::create(Format::ARgb32, 400, 100).unwrap();
    let ctx = CairoContext::new(&surface).unwrap();
    ctx.select_font_face(
        "sans-serif",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    ctx.set_font_size(12.0);

    let short = "Replay ready · 10.0s · Game";
    let truncated_short = truncate_to_width(&ctx, short, 300.0).unwrap();
    assert_eq!(truncated_short, short);

    let long =
        "Replay ready · 30.0s · Super Long Game Title That Exceeds The Narrow Palette Width By Far";
    let truncated = truncate_to_width(&ctx, long, 150.0).unwrap();
    assert!(truncated.ends_with('…'));
    let measured = ctx.text_extents(&truncated).unwrap().x_advance();
    assert!(measured <= 150.0);
}

#[test]
fn mode_palette_renders_with_active_replay_app() {
    use crate::replay::{ReplayStatus, ServiceState};

    let config = RenderConfig::new(
        "#89B4FA",
        "#1E1E2E",
        "#CDD6F4",
        2,
        0,
        0.5,
        "sans-serif".to_string(),
        16,
        FontWeight::Normal,
    )
    .unwrap();
    let renderer = Renderer::new(1000, 700, config);
    let style = crate::config::ModeSelectConfig::default()
        .resolve(crate::theme::ThemeName::Catppuccin.palette());
    let keybinds = crate::config::KeybindsConfig::default();
    let mut buffer = vec![0; 1000 * 700 * 4];

    let status = ReplayStatus {
        state: ServiceState::Buffering,
        target_app: Some("steam_app_default".to_string()),
        target_title: Some("World of Warcraft".to_string()),
        buffered_millis: 12_500,
        buffered_bytes: 40_000_000,
        ..Default::default()
    };

    let result = renderer.render_mode_select(
        &mut buffer,
        &keybinds,
        &style,
        false,
        true,
        Some(&status),
        true,
        None,
        1.0,
    );
    assert!(result.is_ok());
    assert!(buffer.iter().any(|&byte| byte != 0));
}
