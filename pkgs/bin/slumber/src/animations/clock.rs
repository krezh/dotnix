use std::sync::Arc;

use chrono::{DateTime, Local, TimeZone};

use crate::animations::Animation;
use crate::canvas::Canvas;
use crate::font::{FontRenderer, TextMask};
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
}

impl MinimalClock {
    pub fn new(rng: &mut Rng) -> Self {
        let motes = (0..75).map(|_| Mote {
            x: rng.gen_range_f32(0.0, 1.0),
            y: rng.gen_range_f32(0.0, 1.0),
            vx: rng.gen_range_f32(-0.012, 0.012),
            vy: rng.gen_range_f32(-0.022, -0.006),
            radius: rng.gen_range_f32(2.0, 4.5),
            color_idx: rng.gen_range_usize(0, 14),
            phase: rng.gen_range_f32(0.0, std::f32::consts::TAU),
        }).collect();
        Self { motes, time: 0.0 }
    }
}

impl Animation for MinimalClock {
    fn update(&mut self, dt: f32, _width: u32, _height: u32) {
        self.time += dt;
        for mote in &mut self.motes {
            mote.x = (mote.x + mote.vx * dt).rem_euclid(1.0);
            mote.y = (mote.y + mote.vy * dt).rem_euclid(1.0);
        }
    }

    fn render(&mut self, canvas: &mut Canvas, theme: &Theme) {
        canvas.clear(theme.base);
        let accents = theme.accents();
        for mote in &self.motes {
            let pulse = ((self.time * 2.0 + mote.phase).sin() * 0.5 + 0.5) * 160.0 + 60.0;
            let color = accents[mote.color_idx % accents.len()].with_alpha(pulse as u8);
            let x = mote.x * canvas.width as f32;
            let y = mote.y * canvas.height as f32;
            canvas.draw_soft_circle(x, y, mote.radius * 2.2, color);
            canvas.draw_soft_circle(x, y, mote.radius, color.lerp(theme.text, 0.4));
        }
    }
}

#[derive(Default)]
struct CachedText {
    text: String,
    size: f32,
    mask: TextMask,
}

impl CachedText {
    fn refresh(&mut self, font: &FontRenderer, text: &str, size: f32) {
        if self.text != text || self.size != size {
            self.text.clear();
            self.text.push_str(text);
            self.size = size;
            self.mask = font.rasterize_text(text, size);
        }
    }
}

pub struct ClockOverlay {
    font: Arc<FontRenderer>,
    time_format: String,
    date_format: String,
    drift: bool,
    seconds: bool,
    subseconds: bool,
    checked_at: Option<(i64, i32)>,
    dimensions: (u32, u32),
    time_text: String,
    date_text: String,
    time: CachedText,
    date: CachedText,
    gap: f32,
}

impl ClockOverlay {
    pub fn new(font: Arc<FontRenderer>, time_format: &str, date_format: &str, drift: bool) -> Self {
        let (time_seconds, time_subseconds) = format_resolution(time_format);
        let (date_seconds, date_subseconds) = format_resolution(date_format);
        Self {
            font,
            time_format: time_format.into(),
            date_format: date_format.into(),
            drift,
            seconds: time_seconds || date_seconds,
            subseconds: time_subseconds || date_subseconds,
            checked_at: None,
            dimensions: (0, 0),
            time_text: String::new(),
            date_text: String::new(),
            time: CachedText::default(),
            date: CachedText::default(),
            gap: 0.0,
        }
    }

    fn refresh<Tz: TimeZone>(&mut self, now: &DateTime<Tz>, offset: i32, dimensions: (u32, u32))
    where Tz::Offset: std::fmt::Display {
        let key = (now.timestamp().div_euclid(if self.seconds { 1 } else { 60 }), offset);
        if self.checked_at == Some(key) && self.dimensions == dimensions && !self.subseconds {
            return;
        }
        self.checked_at = Some(key);
        let time_text = now.format(&self.time_format).to_string();
        let date_text = now.format(&self.date_format).to_string();
        if dimensions == self.dimensions && time_text == self.time_text && date_text == self.date_text {
            return;
        }
        self.dimensions = dimensions;
        self.time_text = time_text;
        self.date_text = date_text;
        let base = dimensions.0.min(dimensions.1) as f32;
        let time_size = (base * 0.14).max(1.0);
        let date_size = (time_size * 0.28).max(1.0);
        let available_width = (dimensions.0 as f32 * 0.85 - 6.0).max(1.0);
        let text_width = self.font.measure_text(&self.time_text, time_size).0
            .max(self.font.measure_text(&self.date_text, date_size).0);
        let fit = (available_width / text_width.max(1.0)).min(1.0);
        self.time.refresh(&self.font, &self.time_text, (time_size * fit).max(1.0));
        self.date.refresh(&self.font, &self.date_text, (date_size * fit).max(1.0));
        self.gap = (date_size * fit * 0.9).max(2.0);
    }

    pub fn draw(&mut self, canvas: &mut Canvas, theme: &Theme, elapsed: f32) {
        let now = Local::now();
        self.refresh(&now, now.offset().local_minus_utc(), (canvas.width, canvas.height));
        let width = self.time.mask.width.max(self.date.mask.width) as f32 + 4.0;
        let height = self.time.mask.height as f32 + self.date.mask.height as f32 + self.gap + 4.0;
        let (x, y) = overlay_position(canvas.width, canvas.height, width, height, elapsed, self.drift);
        let time_x = x + (width - self.time.mask.width as f32) * 0.5;
        let date_x = x + (width - self.date.mask.width as f32) * 0.5;
        let time_y = y;
        let date_y = y + self.time.mask.height as f32 + self.gap;
        let shadow = theme.crust.with_alpha(190);
        self.time.mask.draw(canvas, time_x + 2.0, time_y + 3.0, shadow);
        self.date.mask.draw(canvas, date_x + 2.0, date_y + 2.0, shadow);
        self.time.mask.draw(canvas, time_x, time_y, theme.text);
        self.date.mask.draw(canvas, date_x, date_y, theme.subtext1);
        if self.time.mask.width > 0 && self.date.mask.width > 0 {
            let divider_y = y + self.time.mask.height as f32 + self.gap * 0.5;
            canvas.draw_line_aa(x + width * 0.075, divider_y, x + width * 0.925, divider_y, theme.surface1.with_alpha(150));
        }
    }
}

fn format_resolution(format: &str) -> (bool, bool) {
    let mut chars = format.chars();
    let (mut seconds, mut subseconds) = (false, false);
    while let Some(ch) = chars.next() {
        if ch != '%' { continue; }
        for specifier in chars.by_ref() {
            if specifier.is_ascii_digit() || matches!(specifier, '.' | ':' | '-' | '_' | '#') {
                continue;
            }
            seconds |= matches!(specifier, 'S' | 's' | 'T' | 'X' | 'r' | 'c' | 'f' | '+');
            subseconds |= matches!(specifier, 'f' | '+');
            break;
        }
    }
    (seconds, subseconds)
}

fn overlay_position(width: u32, height: u32, content_width: f32, content_height: f32, elapsed: f32, drift: bool) -> (f32, f32) {
    let room_x = (width as f32 - content_width).max(0.0);
    let room_y = (height as f32 - content_height).max(0.0);
    let dx = if drift { (elapsed * 0.011).sin() * (width as f32 * 0.04).min(room_x * 0.45) } else { 0.0 };
    let dy = if drift { (elapsed * 0.009).sin() * (height as f32 * 0.04).min(room_y * 0.45) } else { 0.0 };
    (room_x * 0.5 + dx, room_y * 0.5 + dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn clock_cache_tracks_minutes_seconds_dates_and_resize() {
        let font = Arc::new(FontRenderer::new("/nonexistent-slumber-font"));
        let mut clock = ClockOverlay::new(font.clone(), "%H:%M", "%Y-%m-%d", true);
        let now = Utc.with_ymd_and_hms(2026, 10, 14, 23, 59, 58).unwrap();
        clock.refresh(&now, 0, (800, 600));
        assert_eq!(clock.time.text, "23:59");
        assert_eq!(clock.date.text, "2026-10-14");
        let old_size = clock.time.size;
        clock.refresh(&(now + chrono::Duration::seconds(1)), 0, (800, 600));
        assert_eq!(clock.time.text, "23:59");
        clock.refresh(&(now + chrono::Duration::seconds(2)), 0, (800, 600));
        assert_eq!(clock.time.text, "00:00");
        assert_eq!(clock.date.text, "2026-10-15");
        clock.refresh(&now, 0, (320, 240));
        assert!(clock.time.size < old_size);
        let mut seconds = ClockOverlay::new(font, "%T", "", false);
        seconds.refresh(&now, 0, (800, 600));
        assert_eq!(seconds.time.text, "23:59:58");
        seconds.refresh(&(now + chrono::Duration::seconds(1)), 0, (800, 600));
        assert_eq!(seconds.time.text, "23:59:59");
    }

    #[test]
    fn drift_stays_within_available_space() {
        for (width, height, text_width, text_height) in [(320, 240, 280.0, 100.0), (3840, 2160, 500.0, 200.0), (8, 8, 8.0, 8.0)] {
            for second in 0..3600 {
                let (x, y) = overlay_position(width, height, text_width, text_height, second as f32, true);
                assert!(x >= 0.0 && x + text_width <= width as f32);
                assert!(y >= 0.0 && y + text_height <= height as f32);
            }
        }
        assert_eq!(overlay_position(800, 600, 400.0, 100.0, 99.0, false), (200.0, 250.0));
    }
}
