//! The glow behind the bar that follows what is playing.

use std::{
    cell::RefCell,
    collections::HashMap,
    f32::consts::TAU,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Bounds, Canvas, Corners, Pixels, RenderImage, Window, canvas, point, prelude::*, px, size,
};
use image::{Frame, RgbaImage};

use crate::theme::{BLUE, FLAMINGO, LAVENDER, MAUVE, PEACH, PINK, SAPPHIRE, SKY, YELLOW};

/// Bands of light; levels beyond them are ignored and missing ones are silent.
const BANDS: usize = 10;
/// Colours from bass to treble; the assignment drifts slowly along this ring.
const PALETTE: [u32; BANDS] = [
    BLUE, SAPPHIRE, SKY, LAVENDER, MAUVE, MAUVE, PINK, FLAMINGO, PEACH, YELLOW,
];
/// How quickly the light follows the music, per second; lower breathes, higher flickers.
const RESPONSE: f32 = 6.0;
/// Seconds for the palette to advance one step.
const DRIFT: f32 = 9.0;
/// Extra softness added to every light, in logical px.
const SOFTNESS: f32 = 11.0;
const SILENT: f32 = 0.002;
/// Logical px between samples. The light is soft by design and the GPU scales the image up with
/// linear filtering, so a coarse grid looks identical to a full-resolution one at a fraction of the cost.
const SAMPLE: f32 = 6.0;
const MAX_SAMPLES: usize = 256;
/// The image is rebuilt at most this often; frames in between reuse it as is.
const REFRESH: Duration = Duration::from_millis(33);
/// Images not painted for this long are released.
const UNUSED: Duration = Duration::from_secs(2);

struct Cached {
    image: Arc<RenderImage>,
    made: Instant,
    painted: Instant,
}

type Shape = (usize, usize, u32, u32);

struct State {
    levels: [f32; BANDS],
    last: Option<Instant>,
    epoch: Instant,
    images: HashMap<Shape, Cached>,
    retired: Vec<Arc<RenderImage>>,
}

/// Music turned into soft, drifting light. One `Aura` holds the eased spectrum; any number of
/// [`Aura::element`]s can paint it, each at the size of the container it fills.
#[derive(Clone)]
pub struct Aura(Rc<RefCell<State>>);

impl Default for Aura {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(State {
            levels: [0.0; BANDS],
            last: None,
            epoch: Instant::now(),
            images: HashMap::new(),
            retired: Vec::new(),
        })))
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rgb(color: u32) -> [f32; 3] {
    [(color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff].map(|channel| channel as f32 / 255.0)
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

struct Light {
    color: [f32; 3],
    across: Vec<f32>,
    down: Vec<f32>,
}

impl Aura {
    /// Eases toward `spectrum` (silence when not `playing`). Returns whether the light is still
    /// fading out, in which case the caller should keep frames coming until it settles.
    pub fn update(&self, spectrum: &[f32], playing: bool) -> bool {
        let mut state = self.0.borrow_mut();
        let now = Instant::now();
        let dt = state
            .last
            .map_or(0.0, |last| (now - last).as_secs_f32().min(0.1));
        state.last = Some(now);
        let ease = (RESPONSE * dt).min(1.0);
        for (index, level) in state.levels.iter_mut().enumerate() {
            let target = if playing {
                spectrum.get(index).copied().unwrap_or(0.0)
            } else {
                0.0
            };
            *level += (target - *level) * ease;
            if *level < SILENT && target == 0.0 {
                *level = 0.0;
            }
        }
        !playing && state.levels.iter().any(|level| *level > 0.0)
    }

    pub fn is_lit(&self) -> bool {
        self.0.borrow().levels.iter().any(|level| *level > 0.0)
    }

    /// Fills its container with the light, clipped to rounded corners of `radius` so it never shows
    /// past the container's outline. `strength` scales its opacity (0..=1 is the useful range).
    pub fn element(&self, strength: f32, radius: f32) -> Canvas<()> {
        let aura = self.clone();
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| aura.paint(bounds, strength, radius, window),
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }

    fn paint(&self, bounds: Bounds<Pixels>, strength: f32, radius: f32, window: &mut Window) {
        let now = Instant::now();
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let lit = strength > 0.0 && self.is_lit();
        let mut state = self.0.borrow_mut();

        let mut stale: Vec<Shape> = state
            .images
            .iter()
            .filter(|(_, cached)| now - cached.painted > UNUSED)
            .map(|(shape, _)| *shape)
            .collect();
        if !lit {
            stale.extend(state.images.keys().copied());
        }
        for shape in stale {
            if let Some(cached) = state.images.remove(&shape) {
                state.retired.push(cached.image);
            }
        }
        for image in std::mem::take(&mut state.retired) {
            let _ = window.drop_image(image);
        }
        if !lit || width < 1.0 || height < 1.0 {
            return;
        }

        let columns = ((width / SAMPLE).ceil() as usize + 1).clamp(4, MAX_SAMPLES);
        let rows = ((height / SAMPLE).ceil() as usize + 1).clamp(2, MAX_SAMPLES);
        let shape = (columns, rows, strength.to_bits(), 0);
        let due = state
            .images
            .get(&shape)
            .is_none_or(|cached| now - cached.made >= REFRESH);
        if due {
            let image = render(&state, width, height, columns, rows, strength);
            let made = Cached {
                image,
                made: now,
                painted: now,
            };
            if let Some(previous) = state.images.insert(shape, made) {
                state.retired.push(previous.image);
            }
        }
        let Some(cached) = state.images.get_mut(&shape) else {
            return;
        };
        cached.painted = now;
        // The image carries one extra sample on every side and is placed that much larger than the
        // area, so only its interior is drawn: the edge texels that linear filtering would blend with
        // neighbouring atlas tiles stay outside.
        let (step_x, step_y) = (width / (columns - 1) as f32, height / (rows - 1) as f32);
        let placed = Bounds::new(
            point(bounds.origin.x - px(step_x), bounds.origin.y - px(step_y)),
            size(
                bounds.size.width + px(2.0 * step_x),
                bounds.size.height + px(2.0 * step_y),
            ),
        );
        let corners = Corners::all(px(radius.clamp(0.0, width.min(height) / 2.0)));
        let _ = window.paint_image(bounds, placed, corners, cached.image.clone(), 0, false);
    }
}

/// The light sampled on a `columns` × `rows` grid spanning the `width` × `height` area, plus one
/// extra sample on every side (so the image is `columns + 2` × `rows + 2`).
fn render(
    state: &State,
    width: f32,
    height: f32,
    columns: usize,
    rows: usize,
    strength: f32,
) -> Arc<RenderImage> {
    let time = state.epoch.elapsed().as_secs_f32();
    let step = time / DRIFT;
    let (shift, blend) = (step.floor() as usize, smoothstep(step.fract()));
    let x_at = |column: usize| (column as f32 - 1.0) * width / (columns - 1) as f32;
    let y_at = |row: usize| (row as f32 - 1.0) * height / (rows - 1) as f32;
    let (columns, rows) = (columns + 2, rows + 2);
    let lights: Vec<Light> = state
        .levels
        .iter()
        .enumerate()
        .filter(|(_, level)| **level > 0.0)
        .map(|(index, &level)| {
            let [from, to] =
                [shift, shift + 1].map(|offset| rgb(PALETTE[(index + offset) % BANDS]));
            let mix =
                [0, 1, 2].map(|channel| from[channel] + (to[channel] - from[channel]) * blend);
            let sway = (step * TAU * (1.0 + index as f32 * 0.13) + index as f32 * 1.7).sin()
                * width
                * 0.05;
            let center_x = (index as f32 + 0.5) / BANDS as f32 * width + sway;
            let center_y = height * 1.1;
            let spread_x = width / BANDS as f32 * (1.1 + level) * 0.75 + SOFTNESS;
            let spread_y = height * (0.3 + level * 0.6) * 0.75 + SOFTNESS;
            let alpha = (0.11 * smoothstep(level / 0.15) + 0.22 * level) * strength;
            let falloff =
                |count: usize, at: &dyn Fn(usize) -> f32, center: f32, spread: f32| -> Vec<f32> {
                    (0..count)
                        .map(|sample| {
                            let offset = (at(sample) - center) / spread;
                            (-offset * offset).exp()
                        })
                        .collect()
                };
            Light {
                color: mix,
                across: falloff(columns, &x_at, center_x, spread_x)
                    .into_iter()
                    .map(|value| value * alpha)
                    .collect(),
                down: falloff(rows, &y_at, center_y, spread_y),
            }
        })
        .collect();
    let mut image = RgbaImage::new(columns as u32, rows as u32);
    let pixels = image.as_mut();
    for row in 0..rows {
        for column in 0..columns {
            // `across` carries each light's opacity, so `weight` is that light's coverage here;
            // the pixel's colour is the coverage-weighted mix and its alpha the summed coverage.
            let (mut alpha, mut mixed) = (0.0f32, [0.0f32; 3]);
            for light in &lights {
                let weight = light.across[column] * light.down[row];
                alpha += weight;
                for (channel, value) in mixed.iter_mut().zip(light.color) {
                    *channel += value * weight;
                }
            }
            let at = (row * columns + column) * 4;
            if alpha <= 0.0 {
                continue;
            }
            let [red, green, blue] = mixed.map(|channel| byte(channel / alpha));
            pixels[at..at + 4].copy_from_slice(&[blue, green, red, byte(alpha.min(1.0))]);
        }
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}
