#[cfg(test)]
mod tests {
    use crate::animations::{create_animation, ANIMATION_NAMES};
    use crate::canvas::Canvas;
    use crate::config::{AppConfig, Cli};
    use crate::font::{get_glyph, FONT_WIDTH};
    use crate::rng::Rng;
    use crate::theme::{Color, Theme};
    use clap::Parser;

    #[test]
    fn test_theme_colors() {
        let mocha = Theme::mocha();
        assert_eq!(mocha.name, "mocha");
        assert_eq!(mocha.base, Color::rgb(0x1e, 0x1e, 0x2e));
        assert_eq!(mocha.crust, Color::rgb(0x11, 0x11, 0x1b));

        let macchiato = Theme::macchiato();
        assert_eq!(macchiato.name, "macchiato");

        let frappe = Theme::frappe();
        assert_eq!(frappe.name, "frappe");

        let latte = Theme::latte();
        assert_eq!(latte.name, "latte");
        assert_eq!(latte.base, Color::rgb(0xef, 0xf1, 0xf5));

        assert!(Theme::from_name("mocha").is_some());
        assert!(Theme::from_name("macchiato").is_some());
        assert!(Theme::from_name("frappe").is_some());
        assert!(Theme::from_name("latte").is_some());
        assert!(Theme::from_name("invalid").is_none());
    }

    #[test]
    fn test_color_math() {
        let c1 = Color::rgb(100, 100, 100);
        let c2 = Color::rgb(200, 200, 200);
        let mid = c1.lerp(c2, 0.5);
        assert_eq!(mid.r, 150);
        assert_eq!(mid.g, 150);
        assert_eq!(mid.b, 150);

        let fg = Color::rgb(255, 0, 0).with_alpha(128);
        let bg = Color::rgb(0, 0, 255);
        let blended = fg.blend_over(bg);
        assert!(blended.r > 120);
        assert!(blended.b > 120);

        let c = Color::rgb(0xcb, 0xa6, 0xf7);
        let argb = c.to_argb_u32();
        let back = Color::from_argb_u32(argb);
        assert_eq!(c, back);
    }

    #[test]
    fn test_rng() {
        let mut rng = Rng::new(42);
        let f = rng.gen_range_f32(0.0, 1.0);
        assert!((0.0..1.0).contains(&f));

        let idx = rng.gen_range_usize(5, 15);
        assert!((5..15).contains(&idx));

        let items = [1, 2, 3, 4, 5];
        let chosen = *rng.choice(&items);
        assert!(items.contains(&chosen));
    }

    #[test]
    fn test_font() {
        let glyph = get_glyph('A');
        assert_eq!(glyph.len(), 8);
        assert_eq!(FONT_WIDTH, 8);
    }

    #[test]
    fn test_canvas_primitives() {
        let width = 100;
        let height = 100;
        let mut buffer = vec![0u32; width * height];
        let mut canvas = Canvas::new(&mut buffer, width as u32, height as u32);

        let red = Color::rgb(255, 0, 0);
        canvas.clear(red);
        assert_eq!(canvas.pixels[0], red.to_argb_u32());

        let blue = Color::rgb(0, 0, 255);
        canvas.set_pixel(10, 10, blue);
        assert_eq!(canvas.pixels[10 * width + 10], blue.to_argb_u32());

        canvas.draw_line(0, 0, 10, 0, Color::rgb(0, 255, 0), false);
        for x in 0..=10 {
            assert_eq!(canvas.pixels[x], Color::rgb(0, 255, 0).to_argb_u32());
        }

        canvas.draw_rect(20, 20, 10, 10, Color::rgb(255, 255, 0), true);
        assert_eq!(canvas.pixels[25 * width + 25], Color::rgb(255, 255, 0).to_argb_u32());

        canvas.draw_str(0, 50, "TEST", 1, Color::rgb(255, 255, 255));
        canvas.draw_line_aa(0.0, 30.0, 50.0, 30.0, Color::rgb(100, 200, 255));
        canvas.draw_soft_circle(30.0, 30.0, 10.0, Color::rgb(255, 150, 50));
    }

    #[test]
    fn test_all_animations() {
        let width = 640;
        let height = 480;
        let mut buffer = vec![0u32; width * height];
        let theme = Theme::mocha();
        let mut rng = Rng::new(12345);

        for name in ANIMATION_NAMES {
            let mut anim = create_animation(name, &mut rng, "Rubik");
            anim.update(0.016, width as u32, height as u32);
            let mut canvas = Canvas::new(&mut buffer, width as u32, height as u32);
            anim.render(&mut canvas, &theme);
        }
    }

    #[test]
    fn test_cli_parsing() {
        let args = ["slumber", "--animation", "starfield", "--theme", "latte", "--fps", "120", "--clock", "--opacity", "0.75", "--font", "Rubik", "--speed", "0.6"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.animation.as_deref(), Some("starfield"));
        assert_eq!(cli.theme.as_deref(), Some("latte"));
        assert_eq!(cli.fps, Some(120));
        assert!(cli.show_clock);
        assert_eq!(cli.opacity, Some(0.75));
        assert_eq!(cli.font.as_deref(), Some("Rubik"));
        assert_eq!(cli.speed, Some(0.6));

        let config = AppConfig::load(&cli);
        assert_eq!(config.animation, "starfield");
        assert_eq!(config.fps, 120);
        assert!(config.show_clock);
        assert_eq!(config.theme.crust.a, (255.0 * 0.75) as u8);
        assert_eq!(config.font, "Rubik");
        assert_eq!(config.speed, 0.6);

        let font_renderer = crate::font::FontRenderer::new("Rubik");
        let mut buffer = vec![0u32; 640 * 480];
        let mut canvas = Canvas::new(&mut buffer, 640, 480);
        crate::animations::clock::draw_clock_overlay(&mut canvas, &font_renderer, &config.theme, 1.0);
    }

    #[test]
    fn test_single_instance_lock() {
        let path = std::env::temp_dir().join(format!("slumber-test-lock-{}", std::process::id()));
        let lock1 = crate::lock::SingleInstanceLock::acquire_path(&path);
        assert!(lock1.is_ok());

        // Second acquire while lock1 is held must fail
        let lock2 = crate::lock::SingleInstanceLock::acquire_path(&path);
        assert!(lock2.is_err());

        drop(lock1);
        let lock3 = crate::lock::SingleInstanceLock::acquire_path(&path);
        assert!(lock3.is_ok());

        let _ = std::fs::remove_file(path);
    }
}
