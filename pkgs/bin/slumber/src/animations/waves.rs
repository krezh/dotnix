use crate::animations::Animation;
use crate::canvas::Canvas;
use crate::rng::Rng;
use crate::theme::{Color, Theme};

struct WaveLayer {
    base_ratio: f32,
    drift_amp: f32,
    drift_speed: f32,
    drift_phase: f32,
    amp1: f32,
    amp2: f32,
    amp3: f32,
    freq1: f32,
    freq2: f32,
    freq3: f32,
    speed1: f32,
    speed2: f32,
    speed3: f32,
    curtain_height: f32,
    color_shift: f32,
    alpha: f32,
}

pub struct AuroraWaves {
    layers: Vec<WaveLayer>,
    time: f32,
}

impl AuroraWaves {
    pub fn new(rng: &mut Rng) -> Self {
        let mut layers = Vec::new();

        // 3 wide sweeping billowing aurora curtains moving across entire screen
        let configs = [
            (0.35, 0.24, 0.08, 0.0, 160.0, 80.0, 35.0, 220.0, 0.0, 0.85),
            (0.50, 0.28, 0.06, 2.1, 210.0, 110.0, 50.0, 260.0, 2.4, 0.90),
            (0.65, 0.22, 0.07, 4.2, 170.0, 90.0, 40.0, 200.0, 4.8, 0.80),
        ];

        for (base, d_amp, d_speed, d_phase, a1, a2, a3, curtain, shift, alpha) in configs {
            layers.push(WaveLayer {
                base_ratio: base,
                drift_amp: d_amp,
                drift_speed: d_speed,
                drift_phase: d_phase,
                amp1: a1,
                amp2: a2,
                amp3: a3,
                freq1: rng.gen_range_f32(0.0010, 0.0018),
                freq2: rng.gen_range_f32(0.0022, 0.0038),
                freq3: rng.gen_range_f32(0.005, 0.009),
                speed1: rng.gen_range_f32(0.16, 0.28),
                speed2: rng.gen_range_f32(0.20, 0.35),
                speed3: rng.gen_range_f32(0.30, 0.50),
                curtain_height: curtain,
                color_shift: shift,
                alpha,
            });
        }

        Self { layers, time: 0.0 }
    }

    #[inline(always)]
    fn sample_palette(theme: &Theme, t: f32) -> Color {
        let palette = [
            theme.mauve,
            theme.blue,
            theme.sapphire,
            theme.teal,
            theme.green,
            theme.lavender,
            theme.pink,
            theme.peach,
            theme.mauve,
        ];

        let t = t.rem_euclid(1.0) * (palette.len() - 1) as f32;
        let idx = (t.floor() as usize).min(palette.len() - 2);
        let frac = t.fract();
        palette[idx].lerp(palette[idx + 1], frac)
    }
}

impl Animation for AuroraWaves {
    fn update(&mut self, dt: f32, _width: u32, _height: u32) {
        self.time += dt;
    }

    fn render(&mut self, canvas: &mut Canvas, theme: &Theme) {
        // Soft backdrop with configured opacity
        canvas.clear(theme.crust);

        let w = canvas.width as i32;
        let h = canvas.height as f32;
        let stride = canvas.width as usize;

        for layer in &self.layers {
            let drift = (self.time * layer.drift_speed + layer.drift_phase).sin() * (layer.drift_amp * h);
            let base_y = layer.base_ratio * h + drift;
            let curtain_h = layer.curtain_height;
            let inv_curtain_h = 1.0 / curtain_h;

            for x in (0..w).step_by(2) {
                let xf = x as f32;

                // Multi-harmonic wide sweeping curve
                let y1 = (xf * layer.freq1 + self.time * layer.speed1).sin() * layer.amp1;
                let y2 = (xf * layer.freq2 - self.time * layer.speed2 + 1.4).cos() * layer.amp2;
                let y3 = (xf * layer.freq3 + self.time * layer.speed3 + 3.1).sin() * layer.amp3;
                let crest_y = base_y + y1 + y2 + y3;

                let height_factor = (crest_y / h).clamp(0.0, 1.0) * 0.4;
                let color_t = (xf / w as f32) * 0.75 + self.time * 0.018 + layer.color_shift + height_factor;
                let wave_color = Self::sample_palette(theme, color_t);

                let y_start = ((crest_y - curtain_h).floor() as i32).max(0);
                let y_end = ((crest_y + 35.0).ceil() as i32).min(canvas.height as i32 - 1);
                if y_end <= y_start {
                    continue;
                }

                let has_next = (x + 1) < w;
                let mut idx0 = y_start as usize * stride + x as usize;
                let mut idx1 = idx0 + 1;

                for y in y_start..=y_end {
                    let yf = y as f32;
                    let diff = yf - crest_y;

                    let intensity = if diff < 0.0 {
                        let progress = 1.0 - (-diff * inv_curtain_h);
                        if progress > 0.0 {
                            progress * progress * (3.0 - 2.0 * progress) * 0.8
                        } else {
                            0.0
                        }
                    } else {
                        let progress = 1.0 - (diff / 35.0);
                        if progress > 0.0 {
                            progress * progress * 0.6
                        } else {
                            0.0
                        }
                    };

                    if intensity > 0.015 {
                        let alpha = (wave_color.a as f32 * intensity * layer.alpha).min(255.0) as u32;
                        if alpha > 0 {
                            let inv_a = 255 - alpha;
                            let wr = wave_color.r as u32 * alpha;
                            let wg = wave_color.g as u32 * alpha;
                            let wb = wave_color.b as u32 * alpha;

                            let p0 = canvas.pixels[idx0];
                            let r0 = ((p0 >> 16) & 0xFF) * inv_a + wr;
                            let g0 = ((p0 >> 8) & 0xFF) * inv_a + wg;
                            let b0 = (p0 & 0xFF) * inv_a + wb;
                            let a0 = ((p0 >> 24) & 0xFF).max(alpha);
                            canvas.pixels[idx0] = (a0 << 24) | ((r0 / 255) << 16) | ((g0 / 255) << 8) | (b0 / 255);

                            if has_next {
                                let p1 = canvas.pixels[idx1];
                                let r1 = ((p1 >> 16) & 0xFF) * inv_a + wr;
                                let g1 = ((p1 >> 8) & 0xFF) * inv_a + wg;
                                let b1 = (p1 & 0xFF) * inv_a + wb;
                                let a1 = ((p1 >> 24) & 0xFF).max(alpha);
                                canvas.pixels[idx1] = (a1 << 24) | ((r1 / 255) << 16) | ((g1 / 255) << 8) | (b1 / 255);
                            }
                        }
                    }

                    idx0 += stride;
                    idx1 += stride;
                }

                // Glowing spine
                let crest_i = crest_y.round() as i32;
                if crest_i >= 0 && crest_i < canvas.height as i32 {
                    let spine_color = wave_color.lerp(theme.text, 0.45);
                    canvas.blend_pixel_f32(x, crest_i, spine_color, 0.85 * layer.alpha);
                    if has_next {
                        canvas.blend_pixel_f32(x + 1, crest_i, spine_color, 0.85 * layer.alpha);
                    }
                }
            }
        }
    }
}
