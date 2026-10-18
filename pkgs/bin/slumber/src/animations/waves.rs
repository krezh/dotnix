use std::num::NonZeroUsize;
use std::thread;

use crate::animations::Animation;
use crate::canvas::Canvas;
use crate::rng::Rng;
use crate::theme::{Color, Theme, blend_premultiplied};

/// Upper bound on the horizontal bands a frame is rendered in, one thread each.
const MAX_BANDS: usize = 16;

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
    /// Every layer's crest at every column pair of the current frame, reused across frames.
    crests: Vec<Crest>,
    bands: usize,
}

#[derive(Clone, Copy)]
struct Crest {
    /// Canvas row of the crest.
    y: f32,
    color: Color,
    spine: Color,
}

/// What every band of a frame shares; each band renders its own rows of the canvas.
#[derive(Clone, Copy)]
struct Frame<'a> {
    layers: &'a [WaveLayer],
    crests: &'a [Crest],
    width: usize,
    columns: usize,
    scale: f32,
    glow_height: f32,
    background: u32,
}

impl Frame<'_> {
    fn render_band(self, pixels: &mut [u32], first_row: usize) {
        pixels.fill(self.background);
        let top = first_row as i32;
        let bottom = top + (pixels.len() / self.width) as i32 - 1;
        for (layer, crests) in self
            .layers
            .iter()
            .zip(self.crests.chunks_exact(self.columns))
        {
            let curtain_h = layer.curtain_height * self.scale;
            let inv_curtain_h = 1.0 / curtain_h;
            let spine_alpha = (0.85 * layer.alpha).clamp(0.0, 1.0);
            for (column, crest) in crests.iter().enumerate() {
                let x = column * 2;
                let y_start = ((crest.y - curtain_h).floor() as i32).max(top);
                let y_end = ((crest.y + self.glow_height).ceil() as i32).min(bottom);
                for y in y_start..=y_end {
                    let diff = y as f32 - crest.y;
                    let intensity = if diff < 0.0 {
                        let progress = 1.0 - (-diff * inv_curtain_h);
                        if progress > 0.0 {
                            progress * progress * (3.0 - 2.0 * progress) * 0.8
                        } else {
                            0.0
                        }
                    } else {
                        let progress = 1.0 - (diff / self.glow_height);
                        if progress > 0.0 {
                            progress * progress * 0.6
                        } else {
                            0.0
                        }
                    };
                    if intensity > 0.015 {
                        let alpha =
                            (crest.color.a as f32 * intensity * layer.alpha).min(255.0) as u8;
                        if alpha > 0 {
                            let row = &mut pixels[(y - top) as usize * self.width..][..self.width];
                            Self::blend_pair(row, x, crest.color.with_alpha(alpha).to_argb_u32());
                        }
                    }
                }

                // Glowing spine
                let crest_row = crest.y.round() as i32;
                if (top..=bottom).contains(&crest_row) {
                    let alpha = (crest.spine.a as f32 * spine_alpha).round() as u8;
                    if alpha > 0 {
                        let row =
                            &mut pixels[(crest_row - top) as usize * self.width..][..self.width];
                        Self::blend_pair(row, x, crest.spine.with_alpha(alpha).to_argb_u32());
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn blend_pair(row: &mut [u32], x: usize, foreground: u32) {
        row[x] = blend_premultiplied(foreground, row[x]);
        if let Some(next) = row.get_mut(x + 1) {
            *next = blend_premultiplied(foreground, *next);
        }
    }
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

        let bands = thread::available_parallelism()
            .map_or(1, NonZeroUsize::get)
            .min(MAX_BANDS);
        Self {
            layers,
            time: 0.0,
            crests: Vec::new(),
            bands,
        }
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

    /// Lays the waves out across the canvas viewport, so outputs sharing a spanning scene each draw
    /// their slice of one set of waves.
    fn render(&mut self, canvas: &mut Canvas, theme: &Theme) {
        let view = canvas.viewport;
        let width = canvas.width as usize;
        let height = canvas.height as usize;
        if width == 0 || height == 0 {
            return;
        }
        // Waves scale with the scene's height and keep their proportions across its width, so a
        // scene spanning several outputs continues the waves instead of stretching them.
        let scale = view.height / 1080.0;
        let glow_height = 35.0 * scale;
        let columns = width.div_ceil(2);

        // Rows the curtains touch; the rest of the canvas is only backdrop.
        let (mut active_top, mut active_bottom) = (f32::MAX, f32::MIN);
        self.crests.clear();
        for layer in &self.layers {
            let drift = (self.time * layer.drift_speed + layer.drift_phase).sin()
                * (layer.drift_amp * view.height);
            let base_y = layer.base_ratio * view.height + drift;
            let curtain_h = layer.curtain_height * scale;

            for column in 0..columns {
                let scene_x = (column * 2) as f32 + view.x;
                let wave_x = scene_x / scale;

                // Multi-harmonic wide sweeping curve
                let y1 = (wave_x * layer.freq1 + self.time * layer.speed1).sin() * layer.amp1;
                let y2 = (wave_x * layer.freq2 - self.time * layer.speed2 + 1.4).cos() * layer.amp2;
                let y3 = (wave_x * layer.freq3 + self.time * layer.speed3 + 3.1).sin() * layer.amp3;
                let crest_y = base_y + (y1 + y2 + y3) * scale;

                let height_factor = (crest_y / view.height).clamp(0.0, 1.0) * 0.4;
                let color_t = (scene_x / view.width) * 0.75
                    + self.time * 0.018
                    + layer.color_shift
                    + height_factor;
                let color = Self::sample_palette(theme, color_t);
                let y = crest_y - view.y;
                active_top = active_top.min(y - curtain_h);
                active_bottom = active_bottom.max(y + glow_height);
                self.crests.push(Crest {
                    y,
                    color,
                    spine: color.lerp(theme.text, 0.45),
                });
            }
        }

        let frame = Frame {
            layers: &self.layers,
            crests: &self.crests,
            width,
            columns,
            scale,
            glow_height,
            // Soft backdrop with configured opacity
            background: theme.crust.to_argb_u32(),
        };
        // Split the active rows evenly between bands, so no band is left with most of the curtains;
        // the first and last bands also take the backdrop-only rows above and below them.
        let active_top = (active_top.max(0.0) as usize).min(height);
        let active_rows = (active_bottom.max(0.0) as usize)
            .min(height)
            .saturating_sub(active_top);
        let bands = self.bands;
        let mut rest = &mut *canvas.pixels;
        let mut first_row = 0;
        thread::scope(|scope| {
            for band in 1..=bands {
                let end_row = if band == bands {
                    height
                } else {
                    active_top + active_rows * band / bands
                };
                let (pixels, tail) = rest.split_at_mut((end_row - first_row) * width);
                rest = tail;
                if !pixels.is_empty() {
                    let start = first_row;
                    if band == bands {
                        frame.render_band(pixels, start);
                    } else {
                        scope.spawn(move || frame.render_band(pixels, start));
                    }
                }
                first_row = end_row;
            }
        });
    }
}
