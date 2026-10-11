use crate::font::{get_glyph, FONT_WIDTH};
use crate::theme::Color;

pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: u32,
    pub height: u32,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self, color: Color) {
        let val = color.to_argb_u32();
        self.pixels.fill(val);
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx] = color.to_argb_u32();
        }
    }

    #[inline(always)]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let idx = (y as u32 * self.width + x as u32) as usize;
            let bg = Color::from_argb_u32(self.pixels[idx]);
            self.pixels[idx] = color.blend_over(bg).to_argb_u32();
        }
    }
    #[inline(always)]
    pub fn blend_pixel_f32(&mut self, x: i32, y: i32, color: Color, factor: f32) {
        if factor <= 0.0 {
            return;
        }
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let alpha = (color.a as f32 * factor.clamp(0.0, 1.0)).round() as u8;
            if alpha == 0 {
                return;
            }
            let c = color.with_alpha(alpha);
            let idx = (y as u32 * self.width + x as u32) as usize;
            let bg = Color::from_argb_u32(self.pixels[idx]);
            self.pixels[idx] = c.blend_over(bg).to_argb_u32();
        }
    }

    pub fn draw_line_aa(&mut self, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32, color: Color) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;

        if steep {
            self.blend_pixel_f32(ypxl1, xpxl1, color, (1.0 - yend.fract()) * xgap);
            self.blend_pixel_f32(ypxl1 + 1, xpxl1, color, yend.fract() * xgap);
        } else {
            self.blend_pixel_f32(xpxl1, ypxl1, color, (1.0 - yend.fract()) * xgap);
            self.blend_pixel_f32(xpxl1, ypxl1 + 1, color, yend.fract() * xgap);
        }

        let mut intery = yend + gradient;

        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;

        if steep {
            self.blend_pixel_f32(ypxl2, xpxl2, color, (1.0 - yend.fract()) * xgap);
            self.blend_pixel_f32(ypxl2 + 1, xpxl2, color, yend.fract() * xgap);
        } else {
            self.blend_pixel_f32(xpxl2, ypxl2, color, (1.0 - yend.fract()) * xgap);
            self.blend_pixel_f32(xpxl2, ypxl2 + 1, color, yend.fract() * xgap);
        }

        if steep {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                let f = intery.fract();
                self.blend_pixel_f32(y, x, color, 1.0 - f);
                self.blend_pixel_f32(y + 1, x, color, f);
                intery += gradient;
            }
        } else {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                let f = intery.fract();
                self.blend_pixel_f32(x, y, color, 1.0 - f);
                self.blend_pixel_f32(x, y + 1, color, f);
                intery += gradient;
            }
        }
    }

    pub fn draw_soft_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        if radius <= 0.0 {
            return;
        }
        let r_ceil = radius.ceil() as i32;
        let icx = cx.round() as i32;
        let icy = cy.round() as i32;
        let r_sq = radius * radius;

        for dy in -r_ceil..=r_ceil {
            let py = icy + dy;
            if py < 0 || (py as u32) >= self.height {
                continue;
            }
            let dy_f = dy as f32;
            let dy_sq = dy_f * dy_f;
            if dy_sq > r_sq {
                continue;
            }

            let max_dx = (r_sq - dy_sq).sqrt().ceil() as i32;
            for dx in -max_dx..=max_dx {
                let px = icx + dx;
                if px < 0 || (px as u32) >= self.width {
                    continue;
                }
                let dist = ((dx as f32).powi(2) + dy_sq).sqrt();
                if dist <= radius {
                    let t = 1.0 - (dist / radius);
                    let smooth = t * t * (3.0 - 2.0 * t);
                    self.blend_pixel_f32(px, py, color, smooth);
                }
            }
        }
    }

    pub fn draw_line(
        &mut self,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        color: Color,
        blend: bool,
    ) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if blend {
                self.blend_pixel(x0, y0, color);
            } else {
                self.set_pixel(x0, y0, color);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        if color.a == 0 || w <= 0 || h <= 0 {
            return;
        }
        let x_start = x.max(0);
        let y_start = y.max(0);
        let x_end = (x + w).min(self.width as i32);
        let y_end = (y + h).min(self.height as i32);

        for cy in y_start..y_end {
            for cx in x_start..x_end {
                self.blend_pixel(cx, cy, color);
            }
        }
    }


    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color, filled: bool) {
        if filled {
            if color.a < 255 {
                self.blend_rect(x, y, w, h, color);
                return;
            }
            let x_start = x.max(0);
            let y_start = y.max(0);
            let x_end = (x + w).min(self.width as i32);
            let y_end = (y + h).min(self.height as i32);

            let val = color.to_argb_u32();
            for cy in y_start..y_end {
                let start_idx = (cy as u32 * self.width + x_start as u32) as usize;
                let end_idx = (cy as u32 * self.width + x_end as u32) as usize;
                self.pixels[start_idx..end_idx].fill(val);
            }
        } else {
            self.draw_line(x, y, x + w - 1, y, color, false);
            self.draw_line(x, y + h - 1, x + w - 1, y + h - 1, color, false);
            self.draw_line(x, y, x, y + h - 1, color, false);
            self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, color, false);
        }
    }


    pub fn draw_glyph_8x8(
        &mut self,
        x: i32,
        y: i32,
        glyph: &[u8; 8],
        scale: i32,
        color: Color,
    ) {
        let scale = scale.max(1);
        for row in 0..8 {
            let bits = glyph[row as usize];
            if bits == 0 {
                continue;
            }
            for col in 0..8 {
                if (bits & (0x80 >> col)) != 0 {
                    let px = x + col * scale;
                    let py = y + row * scale;
                    if scale == 1 {
                        self.set_pixel(px, py, color);
                    } else {
                        self.draw_rect(px, py, scale, scale, color, true);
                    }
                }
            }
        }
    }

    pub fn draw_char(&mut self, x: i32, y: i32, ch: char, scale: i32, color: Color) {
        let glyph = get_glyph(ch);
        self.draw_glyph_8x8(x, y, glyph, scale, color);
    }

    pub fn draw_str(&mut self, mut x: i32, y: i32, s: &str, scale: i32, color: Color) {
        let char_w = FONT_WIDTH * scale;
        for ch in s.chars() {
            if ch == '\n' {
                continue;
            }
            self.draw_char(x, y, ch, scale, color);
            x += char_w;
        }
    }
}
