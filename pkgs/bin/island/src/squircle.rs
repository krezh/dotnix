use std::{cell::RefCell, collections::HashMap, sync::Arc};

use gpui::{Bounds, Canvas, Corners, Hsla, Pixels, RenderImage, Rgba, Window, canvas, point, prelude::*, px, size};
use image::{Frame, RgbaImage};

pub const DEFAULT_RADIUS: f32 = 28.0;
pub const DEFAULT_POWER: f32 = 2.0;
const SPRITE_LIMIT: usize = 512;

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
        [color.r, color.g, color.b, color.a].map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
    })
}

fn distance(x: f32, y: f32, radius: f32, power: f32) -> f32 {
    let reach = (x.powf(power) + y.powf(power)).powf(power.recip());
    if reach <= f32::EPSILON {
        return -radius;
    }
    let slope = (x.powf(2.0 * power - 2.0) + y.powf(2.0 * power - 2.0)).sqrt() / reach.powf(power - 1.0);
    (reach - radius) / slope.max(1e-3)
}

fn render(sprite: Sprite) -> Arc<RenderImage> {
    let radius = sprite.radius as f32;
    let power = f32::from_bits(sprite.power);
    let line = sprite.line as f32;
    let [fill, border] = [sprite.fill, sprite.border].map(|color| color.map(|value| f32::from(value) / 255.0));
    let last = sprite.radius - 1;
    let mut image = RgbaImage::new(sprite.radius, sprite.radius);
    for (column, row, pixel) in image.enumerate_pixels_mut() {
        let column = if matches!(sprite.corner, 1 | 2) { last - column } else { column };
        let row = if matches!(sprite.corner, 2 | 3) { last - row } else { row };
        let signed = distance(radius - (column as f32 + 0.5), radius - (row as f32 + 0.5), radius, power);
        let outer = (0.5 - signed).clamp(0.0, 1.0);
        let ring = outer - (0.5 - signed - line).clamp(0.0, 1.0);
        let fill_alpha = fill[3] * outer;
        let border_alpha = border[3] * ring;
        let alpha = border_alpha + fill_alpha * (1.0 - border_alpha);
        let channel = |index: usize| {
            if alpha <= 0.0 {
                return 0;
            }
            let premultiplied = border[index] * border_alpha + fill[index] * fill_alpha * (1.0 - border_alpha);
            ((premultiplied / alpha).clamp(0.0, 1.0) * 255.0).round() as u8
        };
        *pixel = image::Rgba([channel(2), channel(1), channel(0), (alpha * 255.0).round() as u8]);
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

fn sprite(sprite: Sprite) -> Arc<RenderImage> {
    SPRITES.with_borrow_mut(|sprites| {
        if sprites.len() > SPRITE_LIMIT {
            sprites.clear();
        }
        sprites.entry(sprite).or_insert_with(|| render(sprite)).clone()
    })
}

pub fn paint(
    bounds: Bounds<Pixels>,
    radius: f32,
    power: f32,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    window: &mut Window,
) {
    if fill.is_none() && border.is_none() {
        return;
    }
    let scale = window.scale_factor();
    let snap = |value: Pixels| (f32::from(value) * scale).round();
    let (left, top) = (snap(bounds.origin.x), snap(bounds.origin.y));
    let (right, bottom) = (snap(bounds.right()), snap(bounds.bottom()));
    let (width, height) = (right - left, bottom - top);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let corner = (radius * scale).round().min((width.min(height) / 2.0).floor()).max(0.0);
    let line = scale.round().max(1.0);
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
    let column = height - 2.0 * corner;
    if let Some(color) = fill {
        rect(window, left + corner, top, span, corner, color);
        rect(window, left, top + corner, width, column, color);
        rect(window, left + corner, bottom - corner, span, corner, color);
    }
    if let Some(color) = border {
        rect(window, left + corner, top, span, line, color);
        rect(window, left + corner, bottom - line, span, line, color);
        rect(window, left, top + corner, line, column, color);
        rect(window, right - line, top + corner, line, column, color);
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
pub fn inset_fill(radius: f32, power: f32, inset: f32, rest: Option<Hsla>, hover: Option<Hsla>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let hovered = hover.is_some() && bounds.contains(&window.mouse_position());
            let shape = Bounds::new(
                point(bounds.origin.x + px(inset), bounds.origin.y),
                size((bounds.size.width - px(2.0 * inset)).max(px(0.0)), bounds.size.height),
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
        image::Rgba([blue, green, red, (f32::from(alpha) * coverage).round().clamp(0.0, 255.0) as u8])
    };
    let side = piece.radius + piece.spread;
    let last = side - 1;
    let reach = side as f32;
    let mut image = RgbaImage::new(side, side);
    for (column, row, out) in image.enumerate_pixels_mut() {
        let column = if matches!(piece.piece, 1 | 2) { last - column } else { column };
        let row = if matches!(piece.piece, 2 | 3) { last - row } else { row };
        let outside = distance(reach - (column as f32 + 0.5), reach - (row as f32 + 0.5), radius, power);
        *out = pixel(shadow_alpha(outside, spread));
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

fn shadow_piece(piece: ShadowPiece) -> Arc<RenderImage> {
    SHADOWS.with_borrow_mut(|pieces| {
        if pieces.len() > SPRITE_LIMIT {
            pieces.clear();
        }
        pieces.entry(piece).or_insert_with(|| render_shadow(piece)).clone()
    })
}

/// A soft shadow `spread` px wide around a squircle filling the element. Drawn from cached
/// corner images and stretched one-pixel edge gradients, so it costs a few image draws per frame
/// whatever the size, instead of tessellating rings of paths.
pub fn outer_shadow(radius: f32, power: f32, color: Hsla, spread: f32) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let scale = window.scale_factor();
            let snap = |value: Pixels| (f32::from(value) * scale).round();
            let (left, top) = (snap(bounds.origin.x), snap(bounds.origin.y));
            let (right, bottom) = (snap(bounds.right()), snap(bounds.bottom()));
            let (width, height) = (right - left, bottom - top);
            let spread_px = (spread * scale).round();
            if width <= 0.0 || height <= 0.0 || spread_px < 1.0 {
                return;
            }
            let corner = (radius * scale).round().min((width.min(height) / 2.0).floor()).max(0.0);
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
            let column = height - 2.0 * corner;
            // Corners are images drawn one texel per pixel.
            let corners = [
                (0, area(left - spread_px, top - spread_px, side, side)),
                (1, area(right + spread_px - side, top - spread_px, side, side)),
                (2, area(right + spread_px - side, bottom + spread_px - side, side, side)),
                (3, area(left - spread_px, bottom + spread_px - side, side, side)),
            ];
            for (piece, at) in corners {
                let image = shadow_piece(ShadowPiece { piece, ..template });
                let _ = window.paint_image(at, at, Corners::default(), image, 0, false);
            }
            // Edges are one solid quad per pixel of falloff. Stretching a thin image instead lets
            // linear filtering blend in neighbouring atlas tiles, which shows as a coloured fringe.
            for step in 0..spread_px as u32 {
                let reach = step as f32;
                let shade = color.opacity(shadow_alpha(reach + 0.5, spread_px));
                if shade.a <= 0.0 {
                    continue;
                }
                let strips = [
                    area(left + corner, top - 1.0 - reach, span, 1.0),
                    area(left + corner, bottom + reach, span, 1.0),
                    area(left - 1.0 - reach, top + corner, 1.0, column),
                    area(right + reach, top + corner, 1.0, column),
                ];
                for strip in strips {
                    if f32::from(strip.size.width) > 0.0 && f32::from(strip.size.height) > 0.0 {
                        window.paint_quad(gpui::fill(strip, shade));
                    }
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

pub fn region_rects([x, y, w, h]: [i32; 4], radius: f32, power: f32) -> Vec<[i32; 4]> {
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
        rects.push([x + inset, y + row, width, 1]);
        rects.push([x + inset, y + h - 1 - row, width, 1]);
    }
    if h > 2 * rows {
        rects.push([x, y + rows, w, h - 2 * rows]);
    }
    rects
}
