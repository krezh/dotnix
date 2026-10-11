use chrono::Local;

use crate::animations::Animation;
use crate::canvas::Canvas;
use crate::font::FontRenderer;
use crate::rng::Rng;
use crate::theme::Theme;

struct Mote {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    color_idx: usize,
    phase: f32,
}

pub struct MinimalClock {
    motes: Vec<Mote>,
    time: f32,
    rng: Rng,
    initialized: bool,
    font_renderer: FontRenderer,
}

impl MinimalClock {
    pub fn new(rng: &mut Rng, font_family: &str) -> Self {
        let font_renderer = FontRenderer::new(font_family);
        Self {
            motes: Vec::new(),
            time: 0.0,
            rng: Rng::new(rng.next_u64()),
            initialized: false,
            font_renderer,
        }
    }

    fn init(&mut self, width: u32, height: u32) {
        let count = 75;
        self.motes.clear();
        for _ in 0..count {
            self.motes.push(Mote {
                x: self.rng.gen_range_f32(0.0, width as f32),
                y: self.rng.gen_range_f32(0.0, height as f32),
                vx: self.rng.gen_range_f32(-18.0, 18.0),
                vy: self.rng.gen_range_f32(-24.0, -6.0),
                radius: self.rng.gen_range_f32(2.0, 4.5),
                color_idx: self.rng.gen_range_usize(0, 14),
                phase: self.rng.gen_range_f32(0.0, 6.28),
            });
        }
        self.initialized = true;
    }
}

pub fn draw_clock_overlay(
    canvas: &mut Canvas,
    font_renderer: &FontRenderer,
    theme: &Theme,
    _time: f32,
) {
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let date_str = now.format("%A, %d %B %Y").to_string();

    let base_dim = canvas.width.min(canvas.height) as f32;
    let time_size = (base_dim * 0.14).clamp(54.0, 140.0);
    let date_size = (time_size * 0.28).clamp(16.0, 34.0);

    let (time_w, _time_h) = font_renderer.measure_text(&time_str, time_size);
    let (date_w, _date_h) = font_renderer.measure_text(&date_str, date_size);

    let cx = canvas.width as f32 / 2.0;
    let cy = canvas.height as f32 / 2.0;

    let time_x = cx - time_w / 2.0;
    let time_y = cy - 8.0;

    let date_x = cx - date_w / 2.0;
    let date_y = cy + date_size + 24.0;

    let shadow_color = theme.crust.with_alpha(190);

    // 1. Soft drop shadow
    font_renderer.draw_text(canvas, &time_str, time_x + 3.0, time_y + 4.0, time_size, shadow_color);
    font_renderer.draw_text(canvas, &date_str, date_x + 2.0, date_y + 2.0, date_size, shadow_color);

    // 2. High-res TrueType / OpenType font rendering in Rubik (or selected font)
    font_renderer.draw_text(canvas, &time_str, time_x, time_y, time_size, theme.text);
    font_renderer.draw_text(canvas, &date_str, date_x, date_y, date_size, theme.subtext1);

    // 3. Subtle horizontal accent divider
    let sep_w = (time_w.max(date_w) * 0.85).min(canvas.width as f32 - 80.0);
    let sep_x0 = cx - sep_w / 2.0;
    let sep_x1 = cx + sep_w / 2.0;
    let sep_y = cy + 8.0;
    canvas.draw_line_aa(sep_x0, sep_y + 1.0, sep_x1, sep_y + 1.0, shadow_color);
    canvas.draw_line_aa(sep_x0, sep_y, sep_x1, sep_y, theme.surface1.with_alpha(150));
}

impl Animation for MinimalClock {
    fn update(&mut self, dt: f32, width: u32, height: u32) {
        if !self.initialized || width == 0 || height == 0 {
            self.init(width, height);
        }

        self.time += dt;
        let w = width as f32;
        let h = height as f32;

        for m in &mut self.motes {
            m.x += m.vx * dt;
            m.y += m.vy * dt;

            if m.y < 0.0 {
                m.y = h;
                m.x = self.rng.gen_range_f32(0.0, w);
            }
            if m.x < 0.0 {
                m.x = w;
            } else if m.x > w {
                m.x = 0.0;
            }
        }
    }

    fn render(&mut self, canvas: &mut Canvas, theme: &Theme) {
        canvas.clear(theme.base);

        let accents = theme.accents();

        // Ambient glowing stardust motes
        for m in &self.motes {
            let pulse = ((self.time * 2.0 + m.phase).sin() * 0.5 + 0.5) * 160.0 + 60.0;
            let c = accents[m.color_idx % accents.len()].with_alpha(pulse as u8);
            canvas.draw_soft_circle(m.x, m.y, m.radius * 2.2, c);
            canvas.draw_soft_circle(m.x, m.y, m.radius, c.lerp(theme.text, 0.4));
        }

        draw_clock_overlay(canvas, &self.font_renderer, theme, self.time);
    }
}
