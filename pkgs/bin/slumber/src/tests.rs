#[cfg(test)]
mod tests {
    use crate::animations::{ANIMATION_NAMES, create_animation};
    use crate::canvas::Canvas;
    use crate::rng::Rng;
    use crate::theme::{Color, Theme, blend_premultiplied};

    #[test]
    fn translucent_pixels_use_premultiplied_source_over() {
        let red = Color::rgb(255, 0, 0).with_alpha(128);
        let blue = Color::rgb(0, 0, 255).with_alpha(128);
        assert_eq!(red.to_argb_u32(), 0x8080_0000);
        assert_eq!(blue.to_argb_u32(), 0x8000_0080);
        assert_eq!(red.blend_over_pixel(blue.to_argb_u32()), 0xc080_0040);
        assert_eq!(red.blend_over_pixel(0), red.to_argb_u32());
        assert_eq!(
            blend_premultiplied(0, blue.to_argb_u32()),
            blue.to_argb_u32()
        );
        assert_eq!(Color::rgb(255, 255, 255).with_alpha(0).to_argb_u32(), 0);
        assert_eq!(
            Color::rgb(10, 20, 30).blend_over_pixel(blue.to_argb_u32()),
            0xff0a_141e
        );
    }

    #[test]
    fn canvas_blends_coverage_without_unpremultiplying() {
        let mut pixels = [0; 4];
        let mut canvas = Canvas::new(&mut pixels, 2, 2);
        canvas.clear(Color::rgb(0, 0, 255).with_alpha(128));
        canvas.blend_pixel(0, 0, Color::rgb(255, 0, 0).with_alpha(128));
        canvas.blend_pixel_f32(1, 1, Color::rgb(255, 0, 0), 0.5);
        canvas.blend_pixel(-1, 0, Color::rgb(0, 255, 0));
        canvas.blend_pixel(2, 0, Color::rgb(0, 255, 0));
        assert_eq!(pixels, [0xc080_0040, 0x8000_0080, 0x8000_0080, 0xc080_0040]);
    }

    #[test]
    fn translucent_animations_preserve_premultiplied_pixel_invariant() {
        let (width, height) = (321, 241);
        let mut pixels = vec![0; width * height];
        let theme = Theme::mocha().with_background_opacity(0.25);
        for name in ANIMATION_NAMES {
            let mut rng = Rng::new(12345);
            let mut animation = create_animation(name, &mut rng);
            for _ in 0..3 {
                animation.update(1.0 / 60.0, width as u32, height as u32);
                animation.render(
                    &mut Canvas::new(&mut pixels, width as u32, height as u32),
                    &theme,
                );
                for &pixel in &pixels {
                    let alpha = pixel >> 24;
                    assert!((pixel >> 16) & 255 <= alpha, "{name}: red exceeds alpha");
                    assert!((pixel >> 8) & 255 <= alpha, "{name}: green exceeds alpha");
                    assert!(pixel & 255 <= alpha, "{name}: blue exceeds alpha");
                }
            }
        }
    }

    #[test]
    fn rng_respects_requested_ranges() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            assert!((0.0..1.0).contains(&rng.gen_range_f32(0.0, 1.0)));
            assert!((5..15).contains(&rng.gen_range_usize(5, 15)));
        }
    }

    #[test]
    fn single_instance_lock_is_released_on_drop() {
        let path = std::env::temp_dir().join(format!("slumber-test-lock-{}", std::process::id()));
        let lock = crate::lock::SingleInstanceLock::acquire_path(&path).unwrap();
        assert!(crate::lock::SingleInstanceLock::acquire_path(&path).is_err());
        drop(lock);
        let next = crate::lock::SingleInstanceLock::acquire_path(&path).unwrap();
        drop(next);
        std::fs::remove_file(path).unwrap();
    }
}
