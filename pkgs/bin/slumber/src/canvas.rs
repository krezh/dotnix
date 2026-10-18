use crate::theme::Color;

/// The part of the scene a canvas shows, in canvas pixels. A canvas normally shows the whole scene;
/// when outputs share a spanning scene, each output's canvas is its slice of the desktop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: u32,
    pub height: u32,
    pub viewport: Viewport,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: width as f32,
                height: height as f32,
            },
        }
    }

    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    #[inline(always)]
    pub fn clear(&mut self, color: Color) {
        let val = color.to_argb_u32();
        self.pixels.fill(val);
    }

    #[inline(always)]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && (x as u32) < self.width && y >= 0 && (y as u32) < self.height {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx] = color.blend_over_pixel(self.pixels[idx]);
        }
    }
    #[inline(always)]
    pub fn blend_pixel_f32(&mut self, x: i32, y: i32, color: Color, factor: f32) {
        if factor <= 0.0 {
            return;
        }
        let alpha = (color.a as f32 * factor.clamp(0.0, 1.0)).round() as u8;
        if alpha != 0 {
            self.blend_pixel(x, y, color.with_alpha(alpha));
        }
    }

    pub fn draw_line_aa(
        &mut self,
        mut x0: f32,
        mut y0: f32,
        mut x1: f32,
        mut y1: f32,
        color: Color,
    ) {
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

    /// Measures from the exact center, so circles moving by fractions of a pixel glide smoothly.
    pub fn draw_soft_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        if radius <= 0.0 {
            return;
        }
        let r_sq = radius * radius;
        let top = ((cy - radius).ceil() as i32).max(0);
        let bottom = ((cy + radius).floor() as i32).min(self.height as i32 - 1);
        for py in top..=bottom {
            let dy = py as f32 - cy;
            let dy_sq = dy * dy;
            let half_width = (r_sq - dy_sq).max(0.0).sqrt();
            let left = ((cx - half_width).ceil() as i32).max(0);
            let right = ((cx + half_width).floor() as i32).min(self.width as i32 - 1);
            for px in left..=right {
                let dx = px as f32 - cx;
                let t = 1.0 - (dx * dx + dy_sq).sqrt() / radius;
                self.blend_pixel_f32(px, py, color, t * t * (3.0 - 2.0 * t));
            }
        }
    }
}
