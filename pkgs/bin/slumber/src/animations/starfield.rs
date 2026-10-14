use crate::animations::Animation;
use crate::canvas::Canvas;
use crate::rng::Rng;
use crate::theme::Theme;

struct Star {
    x: f32,
    y: f32,
    z: f32,
    speed: f32,
    color_idx: usize,
}

pub struct Starfield {
    stars: Vec<Star>,
    rng: Rng,
}

impl Starfield {
    pub fn new(rng: &mut Rng) -> Self {
        let mut r = Rng::new(rng.next_u64());
        let count = 800;
        let mut stars = Vec::with_capacity(count);

        for _ in 0..count {
            stars.push(Star {
                x: r.gen_range_f32(-1000.0, 1000.0),
                y: r.gen_range_f32(-1000.0, 1000.0),
                z: r.gen_range_f32(10.0, 1000.0),
                speed: r.gen_range_f32(140.0, 340.0),
                color_idx: r.gen_range_usize(0, 14),
            });
        }

        Self { stars, rng: r }
    }
}

/// Streak length in seconds of travel, independent of frame rate and of how often a shared scene updates.
const STREAK_SECONDS: f32 = 1.0 / 60.0;

impl Animation for Starfield {
    fn update(&mut self, dt: f32, _width: u32, _height: u32) {
        for star in &mut self.stars {
            star.z -= star.speed * dt;
            if star.z <= 2.0 {
                star.z = 1000.0;
                star.x = self.rng.gen_range_f32(-1000.0, 1000.0);
                star.y = self.rng.gen_range_f32(-1000.0, 1000.0);
                star.speed = self.rng.gen_range_f32(140.0, 340.0);
                star.color_idx = self.rng.gen_range_usize(0, 14);
            }
        }
    }

    fn render(&mut self, canvas: &mut Canvas, theme: &Theme) {
        canvas.clear(theme.base);

        let cx = canvas.width as i32 / 2;
        let cy = canvas.height as i32 / 2;
        let fov = (canvas.width.min(canvas.height) as f32) * 0.7;
        let accents = theme.accents();

        for star in &self.stars {
            let inv_z = 1.0 / star.z;
            let sx = cx + (star.x * inv_z * fov) as i32;
            let sy = cy + (star.y * inv_z * fov) as i32;

            if sx < 0 || sx >= canvas.width as i32 || sy < 0 || sy >= canvas.height as i32 {
                continue;
            }

            let base_color = accents[star.color_idx % accents.len()];
            let brightness = (1.0 - (star.z / 1000.0)).clamp(0.1, 1.0);
            let color = base_color.lerp(theme.text, brightness * 0.5);

            let inv_previous_z = 1.0 / (star.z + star.speed * STREAK_SECONDS);
            canvas.draw_line_aa(
                cx as f32 + star.x * inv_previous_z * fov,
                cy as f32 + star.y * inv_previous_z * fov,
                sx as f32,
                sy as f32,
                color,
            );

            // Head glow for nearby stars
            if star.z < 350.0 {
                let radius = (1.0 - star.z / 350.0) * 3.0 + 1.0;
                canvas.draw_soft_circle(sx as f32, sy as f32, radius, color);
            }
        }
    }
}
