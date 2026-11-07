//! What widgets are drawn from: shaped text, and glyphs, pictures and drawings in the atlas.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
    path::Path,
    rc::Rc,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use cosmic_text::{
    Attrs, Buffer, CacheKey, Color as TextColor, Family, FeatureTag, FontFeatures, FontSystem,
    LayoutGlyph, Metrics, Shaping, Style as FontStyle, SwashCache, SwashContent, Weight,
};
use resvg::{tiny_skia, usvg};
use tracing::warn;

use super::{
    panel::Slot,
    renderer::{ATLAS, Gpu, Sprite},
    wrap::{along, row_starts, shortened_line, shortened_rows},
};
use crate::{
    components::{motion::shared, text::Style},
    drawing::{Drawing, Shape},
    icons,
    picture::Picture,
    theme::{ACCENT, FONT, SURFACE1},
};

/// Text that changes, such as readings, would otherwise pile up.
const MOST_LINES: usize = 4096;
const SVG_DETAIL: f32 = 2.0;

#[derive(Clone, Copy)]
struct Entry {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    left: i32,
    top: i32,
    /// Its own colors, as an emoji has, rather than coverage to fill.
    colored: bool,
}

impl Entry {
    fn glyph(self, rect: [f32; 4], color: [f32; 4]) -> Sprite {
        if self.colored {
            Sprite::image(rect, self.uv(), [color[3]; 4])
        } else {
            Sprite::mask(rect, self.uv(), color)
        }
    }

    fn uv(self) -> [f32; 4] {
        let scale = 1.0 / ATLAS as f32;
        [
            self.x as f32 * scale,
            self.y as f32 * scale,
            self.w as f32 * scale,
            self.h as f32 * scale,
        ]
    }
}

/// A line of text, shaped once.
pub struct Line {
    pub width: f32,
    /// Down from the top of the line, in device pixels.
    baseline: f32,
    glyphs: Vec<LayoutGlyph>,
}

#[derive(Default)]
struct Shelves {
    x: u32,
    y: u32,
    row: u32,
}

impl Shelves {
    /// Whether something this big fits even in an empty atlas.
    fn fits(w: u32, h: u32) -> bool {
        w < ATLAS && h < ATLAS
    }

    fn place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w + 1 > ATLAS {
            self.x = 0;
            self.y += self.row + 1;
            self.row = 0;
        }
        if self.y + h >= ATLAS {
            return None;
        }
        let spot = (self.x, self.y);
        self.x += w + 1;
        self.row = self.row.max(h);
        Some(spot)
    }
}

const GLOW_SIDE: u32 = 260;

/// The least time between emptyings of the atlas, for what cannot fit in it even when empty.
const RECLAIM_EVERY: Duration = Duration::from_secs(1);

pub struct Painter {
    fonts: FontSystem,
    swash: SwashCache,
    shelves: Shelves,
    glyphs: HashMap<CacheKey, Option<Entry>>,
    pictures: HashMap<(String, u32, u32), Option<Entry>>,
    drawings: HashMap<String, Entry>,
    lines: HashMap<(String, &'static str, u32, u16, u32, bool), Rc<Line>>,
    rich: HashMap<(String, u64, u32, u16, u32, u32), Rc<Rich>>,
    reaches: HashMap<(&'static str, u32, u16), (f32, f32)>,
    /// Room kept for each island's glow.
    glows: Vec<(u32, u32)>,
    /// The pictures keyed by where they are, so nothing else comes to be there.
    addressed: Vec<Weak<image::RgbaImage>>,
    /// Something did not fit, so the atlas has to be emptied between frames.
    full: bool,
    reclaimed: Option<Instant>,
}

impl Line {
    /// Where it ends, past any spaces.
    fn reach(&self) -> f32 {
        right_of(&self.glyphs)
    }

    /// The glyphs in `range` as a line of their own, from nothing across, without the spaces that
    /// end it. `text` is what this line was shaped from.
    fn row(self: &Rc<Self>, text: &str, range: Range<usize>) -> Rc<Self> {
        let glyphs = &self.glyphs[range];
        let inked = |glyph: &LayoutGlyph| !text[glyph.start..glyph.end].trim_end().is_empty();
        let kept = glyphs.iter().rposition(inked).map_or(0, |last| last + 1);
        if kept == self.glyphs.len() {
            return self.clone();
        }
        let glyphs = &glyphs[..kept];
        let start = left_of(glyphs);
        let glyphs: Vec<LayoutGlyph> = glyphs
            .iter()
            .cloned()
            .map(|mut glyph| {
                glyph.x -= start;
                glyph
            })
            .collect();
        Rc::new(Self {
            width: right_of(&glyphs),
            baseline: self.baseline,
            glyphs,
        })
    }
}

/// How a picture sits in a box of another shape: all of it, or filling the box.
#[derive(Clone, Copy, Debug)]
pub enum Framing {
    Contain,
    Cover,
}

/// What a text has to fit: `room` across, known or not, whether it wraps in that, and whether
/// it ends in an ellipsis past that or past `most` rows.
#[derive(Clone, Copy)]
pub struct Fit {
    pub room: Option<f32>,
    pub wraps: bool,
    pub ellipsis: bool,
    pub most: Option<usize>,
}

impl Fit {
    /// The width it breaks into rows at, if it does.
    pub fn wrap(self) -> Option<f32> {
        self.room.filter(|_| self.wraps)
    }
}

/// A row of styled text.
pub struct RichRow {
    baseline: f32,
    glyphs: Vec<(LayoutGlyph, Option<[f32; 4]>)>,
    /// Where each styled stretch runs, from the row's start: start, width, its style.
    pub marks: Vec<(f32, f32, usize)>,
}

/// Text in ranges of styles, wrapped.
pub struct Rich {
    pub width: f32,
    pub rows: Vec<RichRow>,
}

fn upload(gpu: &Gpu, entry: Entry, rgba: &[u8]) {
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &gpu.atlas,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: entry.x,
                y: entry.y,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(entry.w * 4),
            rows_per_image: Some(entry.h),
        },
        wgpu::Extent3d {
            width: entry.w,
            height: entry.h,
            depth_or_array_layers: 1,
        },
    );
}

/// To the nearest whole number, halves going down, as text lands on its rows.
fn half_down(value: f32) -> f32 {
    (value - 0.5).ceil()
}

fn even() -> FontFeatures {
    let mut features = FontFeatures::new();
    features.enable(FeatureTag::new(b"tnum"));
    features
}

fn left_of(glyphs: &[LayoutGlyph]) -> f32 {
    let lefts = glyphs.iter().map(|glyph| glyph.x);
    lefts.reduce(f32::min).unwrap_or(0.0)
}

fn right_of(glyphs: &[LayoutGlyph]) -> f32 {
    let rights = glyphs.iter().map(|glyph| glyph.x + glyph.w);
    rights.reduce(f32::max).unwrap_or(0.0)
}

/// How far along each of `glyphs` is and how long they run, for breaking into rows.
fn spots_of(glyphs: &[LayoutGlyph]) -> (Vec<(usize, f32)>, f32) {
    let spots: Vec<(usize, f32, f32)> = glyphs
        .iter()
        .map(|glyph| (glyph.start, glyph.x, glyph.w))
        .collect();
    along(&spots)
}

/// `glyphs` as a row of their own, from nothing across.
fn rich_row(glyphs: &[LayoutGlyph], baseline: f32) -> RichRow {
    let start = left_of(glyphs);
    let mut marks: Vec<(f32, f32, usize)> = Vec::new();
    let mut kept = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        let tint = glyph.color_opt.map(|shade| {
            let (r, g, b, a) = shade.as_rgba_tuple();
            [r, g, b, a].map(|channel| f32::from(channel) / 255.0)
        });
        let mut glyph = glyph.clone();
        glyph.x -= start;
        if glyph.metadata != usize::MAX {
            match marks.last_mut() {
                Some(last) if last.2 == glyph.metadata => {
                    let end = (last.0 + last.1).max(glyph.x + glyph.w);
                    last.0 = last.0.min(glyph.x);
                    last.1 = end - last.0;
                }
                _ => marks.push((glyph.x, glyph.w, glyph.metadata)),
            }
        }
        kept.push((glyph, tint));
    }
    RichRow {
        baseline,
        glyphs: kept,
        marks,
    }
}

/// Straight colors as the sprites blend them.
fn premultiplied(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|pixel| {
            let alpha = u16::from(pixel[3]);
            let scaled = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
            [
                scaled(pixel[0]),
                scaled(pixel[1]),
                scaled(pixel[2]),
                pixel[3],
            ]
        })
        .collect()
}

fn rgb_of(tint: u32) -> [u8; 3] {
    [(tint >> 16) as u8, (tint >> 8) as u8, tint as u8]
}

/// Each 2x2 block of `rgba`, `side` blocks across, as one pixel of its mean coverage.
fn halved(rgba: &[u8], side: usize) -> Vec<u8> {
    let alpha = |x: usize, y: usize| u16::from(rgba[(y * side * 2 + x) * 4 + 3]);
    (0..side * side)
        .flat_map(|index| {
            let (x, y) = (index % side * 2, index / side * 2);
            let sum = alpha(x, y) + alpha(x + 1, y) + alpha(x, y + 1) + alpha(x + 1, y + 1);
            [((sum + 2) / 4) as u8; 4]
        })
        .collect()
}

/// A symbol's coverage, drawn twice the size and halved so its curves come out smooth.
fn symbol_coverage(bytes: &[u8], side: u32) -> Option<Vec<u8>> {
    let text = String::from_utf8_lossy(bytes).replace("currentColor", "#ffffff");
    let tree = usvg::Tree::from_data(text.as_bytes(), &usvg::Options::default()).ok()?;
    let size = tree.size();
    let detailed = side * 2;
    let mut pixmap = tiny_skia::Pixmap::new(detailed, detailed)?;
    let scale = detailed as f32 / size.width().max(size.height());
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(halved(pixmap.data(), side as usize))
}

/// A picture file's own pixels. An SVG comes out at twice its size, with its colors unmultiplied.
fn picture_pixels(path: &Path) -> Option<image::RgbaImage> {
    if !icons::is_svg(path) {
        return image::open(path).ok().map(|image| image.to_rgba8());
    }
    let tree = usvg::Tree::from_data(&std::fs::read(path).ok()?, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let (width, height) = (size.width() * SVG_DETAIL, size.height() * SVG_DETAIL);
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(SVG_DETAIL, SVG_DETAIL),
        &mut pixmap.as_mut(),
    );
    let (width, height) = (pixmap.width(), pixmap.height());
    let mut rgba = pixmap.take();
    for pixel in rgba.chunks_exact_mut(4) {
        if pixel[3] > 0 {
            let alpha = f32::from(pixel[3]) / 255.0;
            for channel in &mut pixel[..3] {
                *channel = (f32::from(*channel) / alpha) as u8;
            }
        }
    }
    image::RgbaImage::from_raw(width, height, rgba)
}

/// `image` in a `width` by `height` box, read at each pixel's center between its four nearest
/// source pixels, however much smaller the box is. Premultiplied.
fn framed(image: &image::RgbaImage, (width, height): (u32, u32), framing: Framing) -> Vec<u8> {
    let (from_w, from_h) = (image.width() as f32, image.height() as f32);
    let (box_w, box_h) = (width as f32, height as f32);
    let ratios = (box_w / from_w, box_h / from_h);
    let ratio = match framing {
        Framing::Contain => ratios.0.min(ratios.1),
        Framing::Cover => ratios.0.max(ratios.1),
    };
    let (left, top) = (
        (box_w - from_w * ratio) / 2.0,
        (box_h - from_h * ratio) / 2.0,
    );
    let mut out = vec![0; (width * height * 4) as usize];
    if image.width() == 0 || image.height() == 0 {
        return out;
    }
    let at = |x: i64, y: i64| {
        image
            .get_pixel(
                x.clamp(0, i64::from(image.width()) - 1) as u32,
                y.clamp(0, i64::from(image.height()) - 1) as u32,
            )
            .0
            .map(f32::from)
    };
    for (index, pixel) in out.chunks_exact_mut(4).enumerate() {
        let (x, y) = (
            (index as u32 % width) as f32 + 0.5,
            (index as u32 / width) as f32 + 0.5,
        );
        let (u, v) = ((x - left) / ratio, (y - top) / ratio);
        if u < 0.0 || v < 0.0 || u > from_w || v > from_h {
            continue;
        }
        let (u, v) = (u - 0.5, v - 0.5);
        let (cell_x, cell_y) = (u.floor() as i64, v.floor() as i64);
        let (across, down) = (u - u.floor(), v - v.floor());
        let corners = [
            at(cell_x, cell_y),
            at(cell_x + 1, cell_y),
            at(cell_x, cell_y + 1),
            at(cell_x + 1, cell_y + 1),
        ];
        let mixed = [0, 1, 2, 3].map(|channel| {
            let upper = corners[0][channel] * (1.0 - across) + corners[1][channel] * across;
            let lower = corners[2][channel] * (1.0 - across) + corners[3][channel] * across;
            upper * (1.0 - down) + lower * down
        });
        let alpha = mixed[3] / 255.0;
        for channel in 0..3 {
            pixel[channel] = (mixed[channel] * alpha).round() as u8;
        }
        pixel[3] = mixed[3].round() as u8;
    }
    out
}

fn path_of(points: &[(f32, f32)], close: bool, scale: f32) -> Option<tiny_skia::Path> {
    let (first, rest) = points.split_first()?;
    let mut path = tiny_skia::PathBuilder::new();
    path.move_to(first.0 * scale, first.1 * scale);
    for at in rest {
        path.line_to(at.0 * scale, at.1 * scale);
    }
    if close {
        path.close();
    }
    path.finish()
}

fn paint_of([r, g, b, a]: [f32; 4]) -> tiny_skia::Paint<'static> {
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
        (a * 255.0) as u8,
    );
    paint.anti_alias = true;
    paint
}

impl Painter {
    pub fn new() -> Self {
        Self {
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            shelves: Shelves::default(),
            glyphs: HashMap::new(),
            pictures: HashMap::new(),
            drawings: HashMap::new(),
            lines: HashMap::new(),
            rich: HashMap::new(),
            reaches: HashMap::new(),
            glows: Vec::new(),
            addressed: Vec::new(),
            full: false,
            reclaimed: None,
        }
    }

    /// When the atlas should next be emptied, if it has to be.
    pub fn reclaim_at(&self) -> Option<Instant> {
        self.full.then(|| {
            self.reclaimed
                .map_or_else(Instant::now, |at| at + RECLAIM_EVERY)
        })
    }

    /// Empties the atlas if that has come due. Whether it did: what was laid out from it is then
    /// no good, and has to be made again.
    pub fn reclaim(&mut self, now: Instant) -> bool {
        let waited = self.reclaimed.is_none_or(|at| at + RECLAIM_EVERY <= now);
        if !self.full || !waited {
            return false;
        }
        warn!("the atlas is full, emptying it");
        self.shelves = Shelves::default();
        self.glyphs.clear();
        self.pictures.clear();
        self.drawings.clear();
        self.glows.clear();
        self.addressed.clear();
        self.full = false;
        self.reclaimed = Some(now);
        true
    }

    /// Keeps room for a glow, and names it.
    pub fn glow_room(&mut self) -> Option<usize> {
        let spot = self.shelves.place(GLOW_SIDE, GLOW_SIDE)?;
        self.glows.push(spot);
        Some(self.glows.len() - 1)
    }

    /// Puts a `w` by `h` glow in its room; what to draw of it, past its one extra sample a side.
    pub fn glow(
        &mut self,
        gpu: &Gpu,
        room: usize,
        (w, h, rgba): (u32, u32, Vec<u8>),
    ) -> Option<[f32; 4]> {
        let (x, y) = *self.glows.get(room)?;
        if w > GLOW_SIDE || h > GLOW_SIDE {
            return None;
        }
        let entry = Entry {
            x,
            y,
            w,
            h,
            left: 0,
            top: 0,
            colored: false,
        };
        upload(gpu, entry, &rgba);
        let atlas = ATLAS as f32;
        Some([
            (x as f32 + 1.5) / atlas,
            (y as f32 + 1.5) / atlas,
            (w as f32 - 3.0) / atlas,
            (h as f32 - 3.0) / atlas,
        ])
    }

    fn placed(
        &mut self,
        gpu: &Gpu,
        w: u32,
        h: u32,
        rgba: &[u8],
        left: i32,
        top: i32,
    ) -> Option<Entry> {
        if !Shelves::fits(w, h) {
            warn!(w, h, "too big for the atlas");
            return None;
        }
        let Some((x, y)) = self.shelves.place(w, h) else {
            self.full = true;
            return None;
        };
        let entry = Entry {
            x,
            y,
            w,
            h,
            left,
            top,
            colored: false,
        };
        upload(gpu, entry, rgba);
        Some(entry)
    }

    /// How far a line of this type reaches above and below its baseline.
    pub fn reach_of(&mut self, family: &'static str, size: f32, weight: u16) -> (f32, f32) {
        let key = (family, size.to_bits(), weight);
        if let Some(reach) = self.reaches.get(&key) {
            return *reach;
        }
        let attrs = Attrs::new()
            .family(Family::Name(family))
            .weight(Weight(weight));
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size * 1.2));
        buffer.set_text("x", &attrs, Shaping::Advanced, None);
        let reach = buffer
            .line_layout(&mut self.fonts, 0)
            .and_then(|lines| lines.first())
            .map_or((size, 0.0), |line| (line.max_ascent, line.max_descent));
        self.reaches.insert(key, reach);
        reach
    }

    /// In the bar's font, with even digits.
    pub fn line(&mut self, text: &str, size: f32, weight: u16, scale: f32) -> Rc<Line> {
        self.line_in(FONT, text, size, weight, scale)
    }

    pub fn line_in(
        &mut self,
        family: &'static str,
        text: &str,
        size: f32,
        weight: u16,
        scale: f32,
    ) -> Rc<Line> {
        self.shaped(family, text, (size, weight), true, scale)
    }

    fn shaped(
        &mut self,
        family: &'static str,
        text: &str,
        (size, weight): (f32, u16),
        even_digits: bool,
        scale: f32,
    ) -> Rc<Line> {
        let key = (
            text.to_owned(),
            family,
            size.to_bits(),
            weight,
            scale.to_bits(),
            even_digits,
        );
        if let Some(line) = self.lines.get(&key) {
            return line.clone();
        }
        if self.lines.len() > MOST_LINES {
            self.lines.clear();
        }
        let mut attrs = Attrs::new()
            .family(Family::Name(family))
            .weight(Weight(weight));
        if even_digits {
            attrs = attrs.font_features(even());
        }
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size * 1.2));
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.fonts, false);
        let mut width: f32 = 0.0;
        let mut baseline: f32 = 0.0;
        let mut glyphs = Vec::new();
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            baseline = run.line_y * scale;
            glyphs.extend(run.glyphs.iter().cloned());
        }
        let line = Rc::new(Line {
            width,
            baseline,
            glyphs,
        });
        self.lines.insert(key, line.clone());
        line
    }

    /// What is left of `text` once cut to `fit`, if it had to be.
    pub fn shortened(
        &mut self,
        family: &'static str,
        text: &str,
        (size, weight): (f32, u16),
        fit: Fit,
        scale: f32,
    ) -> Option<String> {
        let room = fit.room.filter(|_| fit.ellipsis || fit.most.is_some())?;
        let rows = fit.most.filter(|_| fit.wraps);
        if rows.is_none() && self.widest(family, text, (size, weight), scale) <= room {
            return None;
        }
        let mut advance = |c: char| {
            self.line_in(family, c.encode_utf8(&mut [0; 4]), size, weight, scale)
                .reach()
        };
        match rows {
            Some(most) => shortened_rows(text, room, most, &mut advance),
            None => shortened_line(text, room, &mut advance),
        }
    }

    /// `text` broken into rows, what is past `most` of them left out.
    fn rows(
        &mut self,
        family: &'static str,
        text: &str,
        style: (f32, u16),
        (room, most): (Option<f32>, Option<usize>),
        (even_digits, scale): (bool, f32),
    ) -> Vec<Rc<Line>> {
        let mut rows = Vec::new();
        let mut made = 0;
        for line in text.split('\n') {
            let shaped = self.shaped(family, line, style, even_digits, scale);
            let starts = room.map_or_else(Vec::new, |room| {
                let (spots, total) = spots_of(&shaped.glyphs);
                let left = most.map(|most| most.saturating_sub(made));
                row_starts(line, &spots, total, room, left)
            });
            made += starts.len();
            let mut from = 0;
            for until in starts.into_iter().chain([shaped.glyphs.len()]) {
                rows.push(shaped.row(line, from..until));
                from = until;
            }
        }
        if let Some(most) = most {
            rows.truncate(most);
        }
        rows
    }

    /// The rows `text` shows as, cut to `fit`.
    pub fn fitted(
        &mut self,
        family: &'static str,
        text: &str,
        style: (f32, u16),
        fit: Fit,
        scale: f32,
    ) -> Vec<Rc<Line>> {
        let short = self.shortened(family, text, style, fit, scale);
        let text = short.as_deref().unwrap_or(text);
        self.rows(family, text, style, (fit.wrap(), fit.most), (true, scale))
    }

    /// How many rows `text`, already cut to `fit`, takes.
    pub fn row_count(
        &mut self,
        family: &'static str,
        text: &str,
        style: (f32, u16),
        fit: Fit,
        scale: f32,
    ) -> usize {
        self.rows(family, text, style, (fit.wrap(), fit.most), (true, scale))
            .len()
    }

    /// How wide the widest line of `text` is unbroken, to the end of any spaces.
    pub fn widest(
        &mut self,
        family: &'static str,
        text: &str,
        (size, weight): (f32, u16),
        scale: f32,
    ) -> f32 {
        text.split('\n')
            .map(|line| self.line_in(family, line, size, weight, scale).reach())
            .fold(0.0, f32::max)
    }

    /// How many lines `text` takes at `size` in `width`, measured as a feature measures it.
    pub fn wrapped_count(
        &mut self,
        family: &'static str,
        text: &str,
        size: f32,
        width: f32,
    ) -> usize {
        if text.is_empty() {
            0
        } else {
            self.rows(family, text, (size, 500), (Some(width), None), (false, 1.0))
                .len()
        }
    }

    fn pieces(
        &mut self,
        gpu: &Gpu,
        family: &'static str,
        part: &str,
        (size, weight): (f32, u16),
        (tint, top): ([f32; 4], f32),
        scale: f32,
    ) -> (Vec<Sprite>, f32) {
        if part.is_empty() {
            return (Vec::new(), 0.0);
        }
        let line = self.line_in(family, part, size, weight, scale);
        let mut sprites = Vec::new();
        self.text(gpu, &line, (0.0, top), tint, scale, &mut sprites);
        (sprites, line.width)
    }

    /// What `old` and `new` share at the start, the characters between, and what they share at
    /// the end, each laid out from nothing across and at `top` down.
    pub fn morph_parts(
        &mut self,
        gpu: &Gpu,
        family: &'static str,
        (old, new): (&str, &str),
        style: (f32, u16),
        tint: [f32; 4],
        top: f32,
        scale: f32,
    ) -> ((Vec<Sprite>, f32), Vec<Slot>, Vec<Sprite>) {
        let (start, end) = shared(old, new);
        let gone: Vec<char> = old[start..old.len() - end].chars().collect();
        let come: Vec<char> = new[start..new.len() - end].chars().collect();
        let slots = (0..gone.len().max(come.len()))
            .map(|index| {
                let part =
                    |chars: &[char]| chars.get(index).map(char::to_string).unwrap_or_default();
                let (from, from_w) =
                    self.pieces(gpu, family, &part(&gone), style, (tint, top), scale);
                let (to, to_w) = self.pieces(gpu, family, &part(&come), style, (tint, top), scale);
                Slot {
                    from,
                    from_w,
                    to,
                    to_w,
                }
            })
            .collect();
        let first = self.pieces(gpu, family, &new[..start], style, (tint, top), scale);
        let (last, _) = self.pieces(
            gpu,
            family,
            &new[new.len() - end..],
            style,
            (tint, top),
            scale,
        );
        (first, slots, last)
    }

    /// `text` in `styles`, wrapped to `width`. A row's marks name the style by its place in
    /// `styles`.
    pub fn rich(
        &mut self,
        text: &str,
        styles: &[(Range<usize>, Style)],
        size: f32,
        weight: u16,
        width: Option<f32>,
        scale: f32,
    ) -> Rc<Rich> {
        let mut hasher = DefaultHasher::new();
        for (range, style) in styles {
            range.hash(&mut hasher);
            (
                style.bold,
                style.italic,
                style.underline,
                style.strike,
                style.code,
                style.covered,
                style.link,
            )
                .hash(&mut hasher);
        }
        let key = (
            text.to_owned(),
            hasher.finish(),
            size.to_bits(),
            weight,
            width.unwrap_or(-1.0).to_bits(),
            scale.to_bits(),
        );
        if let Some(rich) = self.rich.get(&key) {
            return rich.clone();
        }
        if self.rich.len() > MOST_LINES {
            self.rich.clear();
        }
        let plain = Attrs::new()
            .family(Family::Name(FONT))
            .weight(Weight(weight))
            .font_features(even())
            .metadata(usize::MAX);
        let mut cuts: Vec<usize> = styles
            .iter()
            .flat_map(|(range, _)| [range.start, range.end])
            .chain([0, text.len()])
            .filter(|cut| *cut <= text.len() && text.is_char_boundary(*cut))
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        let pieces = cuts.windows(2).map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            let found = styles
                .iter()
                .position(|(range, _)| range.start <= start && end <= range.end);
            let attrs = found.map_or(plain.clone(), |index| {
                let style = styles[index].1;
                let mut attrs = plain.clone().metadata(index);
                if style.bold {
                    attrs = attrs.weight(Weight(600));
                }
                if style.italic {
                    attrs = attrs.style(FontStyle::Italic);
                }
                if style.link {
                    let [r, g, b] = rgb_of(ACCENT);
                    attrs = attrs.color(TextColor::rgb(r, g, b));
                }
                if style.covered {
                    let [r, g, b] = rgb_of(SURFACE1);
                    attrs = attrs.color(TextColor::rgb(r, g, b));
                }
                attrs
            });
            (&text[start..end], attrs)
        });
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size * 1.2));
        buffer.set_rich_text(pieces, &plain, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.fonts, false);
        let mut rows = Vec::new();
        let mut widest: f32 = 0.0;
        for run in buffer.layout_runs() {
            let (spots, total) = spots_of(run.glyphs);
            let starts = width.map_or_else(Vec::new, |width| {
                row_starts(run.text, &spots, total, width, None)
            });
            widest = widest.max(width.map_or(total, |width| width.min(total)));
            let baseline = (run.line_y - run.line_top) * scale;
            let mut from = 0;
            for until in starts.into_iter().chain([run.glyphs.len()]) {
                rows.push(rich_row(&run.glyphs[from..until], baseline));
                from = until;
            }
        }
        let rich = Rc::new(Rich {
            width: widest,
            rows,
        });
        self.rich.insert(key, rich.clone());
        rich
    }

    /// A row with its box's top left at `x` and `y`.
    pub fn rich_row(
        &mut self,
        gpu: &Gpu,
        row: &RichRow,
        (x, y): (f32, f32),
        color: [f32; 4],
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        for (glyph, tint) in &row.glyphs {
            let tint = tint.unwrap_or(color);
            self.put_glyph(gpu, glyph, row.baseline, (x, y), tint, scale, out);
        }
    }

    fn put_glyph(
        &mut self,
        gpu: &Gpu,
        glyph: &LayoutGlyph,
        baseline: f32,
        (x, y): (f32, f32),
        color: [f32; 4],
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let physical = glyph.physical((x * scale, 0.0), scale);
        let Some(entry) = self.glyph(gpu, physical.cache_key) else {
            return;
        };
        let at = (
            (physical.x + entry.left) as f32,
            half_down(y * scale + baseline) + (physical.y - entry.top) as f32,
        );
        out.push(entry.glyph(
            [
                at.0 / scale,
                at.1 / scale,
                entry.w as f32 / scale,
                entry.h as f32 / scale,
            ],
            color,
        ));
    }

    /// With its box's top left at `x` and `y`, in `color`.
    pub fn text(
        &mut self,
        gpu: &Gpu,
        line: &Line,
        (x, y): (f32, f32),
        color: [f32; 4],
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        for glyph in &line.glyphs {
            self.put_glyph(gpu, glyph, line.baseline, (x, y), color, scale, out);
        }
    }

    fn glyph(&mut self, gpu: &Gpu, key: CacheKey) -> Option<Entry> {
        if let Some(entry) = self.glyphs.get(&key) {
            return *entry;
        }
        let image = self.swash.get_image_uncached(&mut self.fonts, key);
        let entry = image.and_then(|image| {
            let (w, h) = (image.placement.width, image.placement.height);
            if w == 0 || h == 0 {
                return None;
            }
            let colored = !matches!(image.content, SwashContent::Mask);
            let rgba: Vec<u8> = if colored {
                premultiplied(&image.data)
            } else {
                image.data.iter().flat_map(|alpha| [*alpha; 4]).collect()
            };
            self.placed(gpu, w, h, &rgba, image.placement.left, image.placement.top)
                .map(|entry| Entry { colored, ..entry })
        });
        if !self.full {
            self.glyphs.insert(key, entry);
        }
        entry
    }

    fn cached_picture(
        &mut self,
        gpu: &Gpu,
        key: String,
        (width, height): (u32, u32),
        make: impl FnOnce() -> Option<Vec<u8>>,
    ) -> Option<Entry> {
        let key = (key, width, height);
        if let Some(entry) = self.pictures.get(&key) {
            return *entry;
        }
        let entry = make().and_then(|rgba| self.placed(gpu, width, height, &rgba, 0, 0));
        if !self.full {
            self.pictures.insert(key, entry);
        }
        entry
    }

    /// A symbolic icon in `color`, in the square at `rect`.
    pub fn glyph_icon(
        &mut self,
        gpu: &Gpu,
        path: &Path,
        rect: [f32; 4],
        color: [f32; 4],
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let side = (rect[2] * scale).round() as u32;
        let key = path.to_string_lossy().into_owned();
        let entry = self.cached_picture(gpu, key.clone(), (side, side), || {
            let bytes = icons::load(&key)?;
            if icons::is_svg(path) {
                symbol_coverage(&bytes, side)
            } else {
                image::load_from_memory(&bytes)
                    .ok()
                    .map(|image| framed(&image.to_rgba8(), (side, side), Framing::Contain))
            }
        });
        if let Some(entry) = entry {
            out.push(if icons::is_svg(path) {
                Sprite::mask(rect, entry.uv(), color)
            } else {
                Sprite::image(rect, entry.uv(), [1.0; 4])
            });
        }
    }

    pub fn picture(
        &mut self,
        gpu: &Gpu,
        picture: &Picture,
        framing: Framing,
        rect: [f32; 4],
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let size = (
            (rect[2] * scale).round() as u32,
            (rect[3] * scale).round() as u32,
        );
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let entry = match picture {
            Picture::File(path) => {
                let key = format!("{}-{framing:?}", path.to_string_lossy());
                self.cached_picture(gpu, key, size, || {
                    picture_pixels(path).map(|image| framed(&image, size, framing))
                })
            }
            Picture::Pixels(image) => {
                let key = format!("pixels-{:p}-{framing:?}", Arc::as_ptr(image));
                let mut made = false;
                let entry = self.cached_picture(gpu, key, size, || {
                    made = true;
                    let mut rgba = framed(image, size, framing);
                    for pixel in rgba.chunks_exact_mut(4) {
                        pixel.swap(0, 2);
                    }
                    Some(rgba)
                });
                if made {
                    self.addressed.push(Arc::downgrade(image));
                }
                entry
            }
        };
        if let Some(entry) = entry {
            out.push(Sprite::image(rect, entry.uv(), [1.0; 4]));
        }
    }

    /// Rasterized into `key`'s own part of the atlas, which it redraws each time. Circles stay
    /// sprites, so their whole pixels stay whole.
    pub fn drawing(
        &mut self,
        gpu: &Gpu,
        key: &str,
        drawing: &Drawing,
        (x, y): (f32, f32),
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let side = (drawing.side * scale).ceil() as u32;
        let mut pixmap = None;
        for shape in &drawing.shapes {
            match shape {
                Shape::Circle {
                    left,
                    top,
                    size,
                    color,
                } => {
                    let color = super::widget::color(*color);
                    out.push(Sprite::shape(
                        [x + left, y + top, *size, *size],
                        color,
                        *size,
                        2.0,
                    ));
                }
                Shape::Fill { points, color } => {
                    let target = pixmap.get_or_insert_with(|| tiny_skia::Pixmap::new(side, side));
                    if let (Some(target), Some(path)) =
                        (target.as_mut(), path_of(points, true, scale))
                    {
                        target.fill_path(
                            &path,
                            &paint_of(*color),
                            tiny_skia::FillRule::Winding,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }
                Shape::Stroke {
                    points,
                    width,
                    color,
                } => {
                    let target = pixmap.get_or_insert_with(|| tiny_skia::Pixmap::new(side, side));
                    if let (Some(target), Some(path)) =
                        (target.as_mut(), path_of(points, false, scale))
                    {
                        let stroke = tiny_skia::Stroke {
                            width: width.max(1.0) * scale,
                            miter_limit: 1.0,
                            ..Default::default()
                        };
                        target.stroke_path(
                            &path,
                            &paint_of(*color),
                            &stroke,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }
        let Some(Some(pixmap)) = pixmap else {
            return;
        };
        let name = format!("{key}-{side}");
        let entry = match self.drawings.get(&name) {
            Some(entry) => {
                upload(gpu, *entry, pixmap.data());
                Some(*entry)
            }
            None => {
                let entry = self.placed(gpu, side, side, pixmap.data(), 0, 0);
                if let Some(entry) = entry {
                    self.drawings.insert(name, entry);
                }
                entry
            }
        };
        if let Some(entry) = entry {
            out.push(Sprite::image(
                [x, y, drawing.side, drawing.side],
                entry.uv(),
                [1.0; 4],
            ));
        }
    }
}

#[cfg(test)]
#[path = "paint_test.rs"]
mod tests;
