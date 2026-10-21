use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use gpui::{
    Bounds, BoxShadow, Canvas, ContentMask, Corners, Hsla, Pixels, RenderImage, Rgba, Window,
    canvas, hsla, point, prelude::*, px, size,
};
use image::{Frame, RgbaImage};

pub const DEFAULT_RADIUS: f32 = 28.0;
pub const DEFAULT_POWER: f32 = 2.0;
pub const DEFAULT_BORDER: f32 = 1.0;
const SPRITE_LIMIT: usize = 512;

/// Outline width in logical pixels, as `f32` bits; set once from the config.
static BORDER: AtomicU32 = AtomicU32::new(DEFAULT_BORDER.to_bits());

pub fn set_border(width: f32) {
    BORDER.store(width.max(0.0).to_bits(), Ordering::Relaxed);
}

/// The outline width in device pixels, or `None` when outlines are off.
fn line(scale: f32) -> Option<f32> {
    let width = f32::from_bits(BORDER.load(Ordering::Relaxed));
    (width > 0.0).then(|| (width * scale).round().max(1.0))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Sprite {
    corner: u8,
    radius: u32,
    power: u32,
    line: u32,
    fill: [u8; 4],
    border: [u8; 4],
}

thread_local! {
    static SPRITES: RefCell<HashMap<Sprite, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

fn channels(color: Option<Hsla>) -> [u8; 4] {
    color.map_or([0; 4], |color| {
        let color = Rgba::from(color);
        [color.r, color.g, color.b, color.a]
            .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
    })
}

fn distance(x: f32, y: f32, radius: f32, power: f32) -> f32 {
    let reach = (x.powf(power) + y.powf(power)).powf(power.recip());
    if reach <= f32::EPSILON {
        return -radius;
    }
    let slope =
        (x.powf(2.0 * power - 2.0) + y.powf(2.0 * power - 2.0)).sqrt() / reach.powf(power - 1.0);
    (reach - radius) / slope.max(1e-3)
}

fn render(sprite: Sprite) -> Arc<RenderImage> {
    let radius = sprite.radius as f32;
    let power = f32::from_bits(sprite.power);
    let line = sprite.line as f32;
    let [fill, border] =
        [sprite.fill, sprite.border].map(|color| color.map(|value| f32::from(value) / 255.0));
    let last = sprite.radius - 1;
    let mut image = RgbaImage::new(sprite.radius, sprite.radius);
    for (column, row, pixel) in image.enumerate_pixels_mut() {
        let column = if matches!(sprite.corner, 1 | 2) {
            last - column
        } else {
            column
        };
        let row = if matches!(sprite.corner, 2 | 3) {
            last - row
        } else {
            row
        };
        let signed = distance(
            radius - (column as f32 + 0.5),
            radius - (row as f32 + 0.5),
            radius,
            power,
        );
        let outer = (0.5 - signed).clamp(0.0, 1.0);
        let ring = outer - (0.5 - signed - line).clamp(0.0, 1.0);
        *pixel = blend(fill, border, fill[3] * outer, border[3] * ring);
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

fn blend(fill: [f32; 4], border: [f32; 4], fill_alpha: f32, border_alpha: f32) -> image::Rgba<u8> {
    let alpha = border_alpha + fill_alpha * (1.0 - border_alpha);
    let channel = |index: usize| {
        if alpha <= 0.0 {
            return 0;
        }
        let premultiplied =
            border[index] * border_alpha + fill[index] * fill_alpha * (1.0 - border_alpha);
        ((premultiplied / alpha).clamp(0.0, 1.0) * 255.0).round() as u8
    };
    image::Rgba([
        channel(2),
        channel(1),
        channel(0),
        (alpha * 255.0).round() as u8,
    ])
}

/// The image reaches one line past the corner, over the upper shape's edge and the lower one's
/// side, so its border joins theirs; it fills only the corner itself.
fn render_fillet(sprite: Sprite) -> Arc<RenderImage> {
    let radius = sprite.radius as f32;
    let line = sprite.line as f32;
    let [fill, border] =
        [sprite.fill, sprite.border].map(|color| color.map(|value| f32::from(value) / 255.0));
    let side = sprite.radius + sprite.line;
    let last = side - 1;
    let mut image = RgbaImage::new(side, side);
    for (column, row, pixel) in image.enumerate_pixels_mut() {
        let column = if sprite.corner == 5 {
            last - column
        } else {
            column
        };
        let (x, y) = (column as f32 + 0.5, row as f32 + 0.5 - line);
        let outside = x.hypot(radius - y) - radius;
        let outer = (0.5 + outside).clamp(0.0, 1.0);
        let ring = outer - (0.5 + outside - line).clamp(0.0, 1.0);
        let fill_alpha = if x < radius && y > 0.0 {
            fill[3] * outer
        } else {
            0.0
        };
        *pixel = blend(fill, border, fill_alpha, border[3] * ring);
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

fn sprite(sprite: Sprite) -> Arc<RenderImage> {
    SPRITES.with_borrow_mut(|sprites| {
        if sprites.len() > SPRITE_LIMIT {
            sprites.clear();
        }
        sprites
            .entry(sprite)
            .or_insert_with(|| {
                if sprite.corner >= 4 {
                    render_fillet(sprite)
                } else {
                    render(sprite)
                }
            })
            .clone()
    })
}

#[derive(Clone, Copy)]
struct Parts {
    top: bool,
    /// Device pixels of the bottom edge left open, where another shape hangs.
    gap: Option<(f32, f32)>,
    /// Device pixels at the top of the sides left to the concave corners' border.
    lip: f32,
}

const WHOLE: Parts = Parts {
    top: true,
    gap: None,
    lip: 0.0,
};

fn around(from: f32, to: f32, gap: Option<(f32, f32)>) -> Vec<(f32, f32)> {
    let spans = match gap {
        Some((start, end)) => vec![(from, start.min(to)), (end.max(from), to)],
        None => vec![(from, to)],
    };
    spans
        .into_iter()
        .filter(|(start, end)| end > start)
        .collect()
}

pub fn paint(
    bounds: Bounds<Pixels>,
    radius: f32,
    power: f32,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    window: &mut Window,
) {
    paint_parts(bounds, radius, power, fill, border, WHOLE, window);
}

fn paint_parts(
    bounds: Bounds<Pixels>,
    radius: f32,
    power: f32,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    parts: Parts,
    window: &mut Window,
) {
    let scale = window.scale_factor();
    let border = border.filter(|_| line(scale).is_some());
    let line = line(scale).unwrap_or(0.0);
    if fill.is_none() && border.is_none() {
        return;
    }
    let snap = |value: Pixels| (f32::from(value) * scale).round();
    let (left, top) = (snap(bounds.origin.x), snap(bounds.origin.y));
    let (right, bottom) = (snap(bounds.right()), snap(bounds.bottom()));
    let (width, height) = (right - left, bottom - top);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let corner = (radius * scale)
        .round()
        .min((width.min(height) / 2.0).floor())
        .max(0.0);
    let area = |x: f32, y: f32, w: f32, h: f32| Bounds {
        origin: point(px(x / scale), px(y / scale)),
        size: size(px(w / scale), px(h / scale)),
    };
    let rect = |window: &mut Window, x: f32, y: f32, w: f32, h: f32, color: Hsla| {
        if w > 0.0 && h > 0.0 {
            window.paint_quad(gpui::fill(area(x, y, w, h), color));
        }
    };
    let span = width - 2.0 * corner;
    let sides = if parts.top { top + corner } else { top };
    let column = bottom - corner - sides;
    if let Some(color) = fill {
        if parts.top {
            rect(window, left + corner, top, span, corner, color);
        }
        rect(window, left, sides, width, column, color);
        rect(window, left + corner, bottom - corner, span, corner, color);
    }
    if let Some(color) = border {
        if parts.top {
            rect(window, left + corner, top, span, line, color);
        }
        for (from, to) in around(left + corner, right - corner, parts.gap) {
            rect(window, from, bottom - line, to - from, line, color);
        }
        let lip = parts.lip.min(column);
        rect(window, left, sides + lip, line, column - lip, color);
        rect(window, right - line, sides + lip, line, column - lip, color);
    }
    if corner < 1.0 {
        return;
    }
    let origins = [
        (left, top),
        (right - corner, top),
        (right - corner, bottom - corner),
        (left, bottom - corner),
    ];
    for (index, (x, y)) in origins.into_iter().enumerate() {
        if !parts.top && index < 2 {
            continue;
        }
        let image = sprite(Sprite {
            corner: index as u8,
            radius: corner as u32,
            power: power.to_bits(),
            line: if border.is_some() { line as u32 } else { 0 },
            fill: channels(fill),
            border: channels(border),
        });
        let at = area(x, y, corner, corner);
        let _ = window.paint_image(at, at, Corners::default(), image, 0, false);
    }
}

/// A shape hanging from the bottom edge of another, joined to it by concave corners of radius
/// `fillet`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hang {
    pub bounds: Bounds<Pixels>,
    pub radius: f32,
    pub power: f32,
    pub fillet: f32,
}

impl Hang {
    fn gap(&self, scale: f32) -> (f32, f32) {
        let fillet = (self.fillet * scale).round();
        let snap = |value: Pixels| (f32::from(value) * scale).round();
        (
            snap(self.bounds.origin.x) - fillet,
            snap(self.bounds.right()) + fillet,
        )
    }
}

/// `body` and `hang` are in the coordinates of the element this fills.
pub fn joined(
    body: Bounds<Pixels>,
    radius: f32,
    power: f32,
    hang: Option<Hang>,
    fill: Hsla,
    border: Option<Hsla>,
) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            let Some(hang) = hang else {
                paint(body, radius, power, Some(fill), border, window);
                return;
            };
            let scale = window.scale_factor();
            let gap = hang.gap(scale);
            let fillet = (hang.fillet * scale).round();
            let above = Parts {
                top: true,
                gap: Some(gap),
                lip: 0.0,
            };
            let below = Parts {
                top: false,
                gap: None,
                lip: fillet,
            };
            paint_parts(body, radius, power, Some(fill), border, above, window);
            paint_parts(
                hang.bounds,
                hang.radius,
                hang.power,
                Some(fill),
                border,
                below,
                window,
            );
            if fillet < 1.0 {
                return;
            }
            let border = border.filter(|_| line(scale).is_some());
            let line = line(scale).unwrap_or(0.0);
            let side = fillet + line;
            let top = (f32::from(body.bottom()) * scale).round() - line;
            for (corner, x) in [(4, gap.0), (5, gap.1 - side)] {
                let image = sprite(Sprite {
                    corner,
                    radius: fillet as u32,
                    power: DEFAULT_POWER.to_bits(),
                    line: line as u32,
                    fill: channels(Some(fill)),
                    border: channels(border),
                });
                let at = Bounds {
                    origin: point(px(x / scale), px(top / scale)),
                    size: size(px(side / scale), px(side / scale)),
                };
                let _ = window.paint_image(at, at, Corners::default(), image, 0, false);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

pub fn background(radius: f32, power: f32, fill: Hsla, border: Option<Hsla>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| paint(bounds, radius, power, Some(fill), border, window),
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

pub fn fill(radius: f32, power: f32, rest: Option<Hsla>, hover: Option<Hsla>) -> Canvas<()> {
    inset_fill(radius, power, 0.0, rest, hover)
}

/// `rest`, with `hover` laid over it while the pointer is inside, so a hover tint shows on any fill.
pub fn inset_fill(
    radius: f32,
    power: f32,
    inset: f32,
    rest: Option<Hsla>,
    hover: Option<Hsla>,
) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let hovered = hover.is_some() && bounds.contains(&window.mouse_position());
            let shape = Bounds::new(
                point(bounds.origin.x + px(inset), bounds.origin.y),
                size(
                    (bounds.size.width - px(2.0 * inset)).max(px(0.0)),
                    bounds.size.height,
                ),
            );
            paint(shape, radius, power, rest, None, window);
            if hovered {
                paint(shape, radius, power, hover, None, window);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A corner of a shadow: 0–3, top-left clockwise.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ShadowPiece {
    piece: u8,
    radius: u32,
    spread: u32,
    power: u32,
    color: [u8; 4],
}

thread_local! {
    static SHADOWS: RefCell<HashMap<ShadowPiece, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// Shadow coverage at `outside` device px beyond the outline: `(1 - d/spread)²`, nothing inside.
fn shadow_alpha(outside: f32, spread: f32) -> f32 {
    if outside <= 0.0 || outside >= spread {
        return 0.0;
    }
    let falloff = 1.0 - outside / spread;
    falloff * falloff
}

fn render_shadow(piece: ShadowPiece) -> Arc<RenderImage> {
    let (radius, spread) = (piece.radius as f32, piece.spread as f32);
    let power = f32::from_bits(piece.power);
    let [red, green, blue, alpha] = piece.color;
    let pixel = |coverage: f32| {
        image::Rgba([
            blue,
            green,
            red,
            (f32::from(alpha) * coverage).round().clamp(0.0, 255.0) as u8,
        ])
    };
    let side = piece.radius + piece.spread;
    let last = side - 1;
    let reach = side as f32;
    let mut image = RgbaImage::new(side, side);
    for (column, row, out) in image.enumerate_pixels_mut() {
        let column = if matches!(piece.piece, 1 | 2) {
            last - column
        } else {
            column
        };
        let row = if matches!(piece.piece, 2 | 3) {
            last - row
        } else {
            row
        };
        let outside = distance(
            reach - (column as f32 + 0.5),
            reach - (row as f32 + 0.5),
            radius,
            power,
        );
        *out = pixel(shadow_alpha(outside, spread));
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

fn shadow_piece(piece: ShadowPiece) -> Arc<RenderImage> {
    SHADOWS.with_borrow_mut(|pieces| {
        if pieces.len() > SPRITE_LIMIT {
            pieces.clear();
        }
        pieces
            .entry(piece)
            .or_insert_with(|| render_shadow(piece))
            .clone()
    })
}

/// A soft shadow `spread` px wide around a rounded rectangle filling the element. Drawn from cached
/// corner images and stretched one-pixel edge gradients, so it costs a few image draws per frame
/// whatever the size, instead of tessellating rings of paths.
pub fn outer_shadow(radius: f32, power: f32, color: Hsla, spread: f32) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            paint_shadow(bounds, radius, power, color, spread, WHOLE, window)
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// One layer of a drop shadow, in logical pixels. `blur` is the Gaussian's sigma, half of a CSS
/// blur radius.
#[derive(Clone, Copy)]
pub struct Layer {
    pub alpha: f32,
    pub drop: f32,
    pub blur: f32,
    pub spread: f32,
}

/// Black drop shadows around `body` and a shape hanging from it, blurred on the GPU. They stay
/// outside both shapes, so neither translucent fill darkens; corners are circular, as GPUI's are.
pub fn joined_drop_shadow<const N: usize>(
    body: Bounds<Pixels>,
    radius: f32,
    hang: Option<Hang>,
    layers: [Layer; N],
) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let shadows: Vec<BoxShadow> = layers
                .iter()
                .filter(|layer| layer.alpha > 0.0)
                .map(|layer| BoxShadow {
                    color: hsla(0.0, 0.0, 0.0, layer.alpha.min(1.0)),
                    offset: point(px(0.0), px(layer.drop)),
                    blur_radius: px(layer.blur),
                    spread_radius: px(layer.spread),
                    inset: false,
                })
                .collect();
            if shadows.is_empty() {
                return;
            }
            let Some(hang) = hang else {
                window.paint_drop_shadows(body, Corners::all(px(radius)), &shadows);
                return;
            };
            // Each shape's shadow is kept off the other: the body's above the seam and beside the
            // hanging shape, the hanging shape's below the seam.
            let seam = body.bottom();
            let left = hang.bounds.left() - px(hang.fillet);
            let right = hang.bounds.right() + px(hang.fillet);
            let clip = |x0: Pixels, y0: Pixels, x1: Pixels, y1: Pixels| ContentMask {
                bounds: Bounds::from_corners(point(x0, y0), point(x1, y1)),
            };
            let body_masks = [
                clip(bounds.left(), bounds.top(), bounds.right(), seam),
                clip(bounds.left(), seam, left, bounds.bottom()),
                clip(right, seam, bounds.right(), bounds.bottom()),
            ];
            for mask in body_masks {
                window.with_content_mask(Some(mask), |window| {
                    window.paint_drop_shadows(body, Corners::all(px(radius)), &shadows);
                });
            }
            let hang_corners = Corners {
                top_left: px(0.0),
                top_right: px(0.0),
                bottom_right: px(hang.radius),
                bottom_left: px(hang.radius),
            };
            let below = clip(bounds.left(), seam, bounds.right(), bounds.bottom());
            window.with_content_mask(Some(below), |window| {
                window.paint_drop_shadows(hang.bounds, hang_corners, &shadows);
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

fn paint_shadow(
    bounds: Bounds<Pixels>,
    radius: f32,
    power: f32,
    color: Hsla,
    spread: f32,
    parts: Parts,
    window: &mut Window,
) {
    let scale = window.scale_factor();
    let snap = |value: Pixels| (f32::from(value) * scale).round();
    let (left, top) = (snap(bounds.origin.x), snap(bounds.origin.y));
    let (right, bottom) = (snap(bounds.right()), snap(bounds.bottom()));
    let (width, height) = (right - left, bottom - top);
    let spread_px = (spread * scale).round();
    if width <= 0.0 || height <= 0.0 || spread_px < 1.0 {
        return;
    }
    let corner = (radius * scale)
        .round()
        .min((width.min(height) / 2.0).floor())
        .max(0.0);
    let side = corner + spread_px;
    let area = |x: f32, y: f32, w: f32, h: f32| Bounds {
        origin: point(px(x / scale), px(y / scale)),
        size: size(px(w / scale), px(h / scale)),
    };
    let template = ShadowPiece {
        piece: 0,
        radius: corner as u32,
        spread: spread_px as u32,
        power: power.to_bits(),
        color: channels(Some(color)),
    };
    let span = width - 2.0 * corner;
    let sides = if parts.top { top + corner } else { top };
    let column = bottom - corner - sides;
    // Corners are images drawn one texel per pixel.
    let corners = [
        (0, area(left - spread_px, top - spread_px, side, side)),
        (
            1,
            area(right + spread_px - side, top - spread_px, side, side),
        ),
        (
            2,
            area(
                right + spread_px - side,
                bottom + spread_px - side,
                side,
                side,
            ),
        ),
        (
            3,
            area(left - spread_px, bottom + spread_px - side, side, side),
        ),
    ];
    for (piece, at) in corners {
        if !parts.top && piece < 2 {
            continue;
        }
        let image = shadow_piece(ShadowPiece { piece, ..template });
        let _ = window.paint_image(at, at, Corners::default(), image, 0, false);
    }
    // Edges are one solid quad per pixel of falloff. Stretching a thin image instead lets
    // linear filtering blend in neighbouring atlas tiles, which shows as a coloured fringe.
    let bottoms = around(left + corner, right - corner, parts.gap);
    for step in 0..spread_px as u32 {
        let reach = step as f32;
        let shade = color.opacity(shadow_alpha(reach + 0.5, spread_px));
        if shade.a <= 0.0 {
            continue;
        }
        let mut strips = vec![
            area(left - 1.0 - reach, sides, 1.0, column),
            area(right + reach, sides, 1.0, column),
        ];
        if parts.top {
            strips.push(area(left + corner, top - 1.0 - reach, span, 1.0));
        }
        strips.extend(
            bottoms
                .iter()
                .map(|(from, to)| area(*from, bottom + reach, to - from, 1.0)),
        );
        for strip in strips {
            if f32::from(strip.size.width) > 0.0 && f32::from(strip.size.height) > 0.0 {
                window.paint_quad(gpui::fill(strip, shade));
            }
        }
    }
}

fn whole(bounds: Bounds<Pixels>) -> [i32; 4] {
    [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ]
    .map(|value| f32::from(value).round() as i32)
}

pub fn region(body: Bounds<Pixels>, radius: f32, power: f32, hang: Option<Hang>) -> Vec<[i32; 4]> {
    let mut rects = region_rects(whole(body), radius, power, true);
    let Some(hang) = hang else {
        return rects;
    };
    let [x, y, w, h] = whole(hang.bounds);
    if w <= 0 || h <= 0 {
        return rects;
    }
    rects.extend(region_rects([x, y, w, h], hang.radius, hang.power, false));
    let fillet = hang.fillet.round();
    for row in 0..fillet as i32 {
        let rise = fillet - (row as f32 + 0.5);
        let cut = (fillet * fillet - rise * rise).max(0.0).sqrt().round() as i32;
        let width = fillet as i32 - cut;
        if width > 0 {
            rects.push([x - width, y + row, width, 1]);
            rects.push([x + w, y + row, width, 1]);
        }
    }
    rects
}

fn region_rects([x, y, w, h]: [i32; 4], radius: f32, power: f32, top: bool) -> Vec<[i32; 4]> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let r = radius.clamp(0.0, w.min(h) as f32 / 2.0);
    let rows = r.ceil() as i32;
    let mut rects = Vec::with_capacity(2 * rows as usize + 1);
    for row in 0..rows {
        let from_centre = ((r - (row as f32 + 0.5)) / r).max(0.0);
        let reach = r * (1.0 - from_centre.powf(power)).max(0.0).powf(power.recip());
        let inset = (r - reach).round() as i32;
        let width = w - 2 * inset;
        if top {
            rects.push([x + inset, y + row, width, 1]);
        }
        rects.push([x + inset, y + h - 1 - row, width, 1]);
    }
    let rounded = if top { 2 * rows } else { rows };
    if h > rounded {
        rects.push([x, if top { y + rows } else { y }, w, h - rounded]);
    }
    rects
}
