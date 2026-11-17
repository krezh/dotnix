//! The bar, a menu hanging from it and a chain of droplets splitting off one of its ends, as one
//! surface, blended like liquid where they meet.

#[cfg(test)]
use crate::stage::COLLAPSED_H;

/// How far the menu reaches up into the bar, as a share of the bar's height. Its top is hidden
/// there, so only the blend shows where the two meet, and the bar's top stays flat.
const TUCK: f32 = 0.5;
/// How round the two sides of a waist pressing in where a droplet parts are.
#[cfg(test)]
const WAIST_RADIUS: f32 = 30.0;
/// How softly a waist is cut in, and a thread joined on.
#[cfg(test)]
const WAIST_BLEND: f32 = 7.0;
#[cfg(test)]
const THREAD_BLEND: f32 = 6.0;
/// The most droplets out at once; the shader takes this many.
pub const MAX_DROPS: usize = 4;

/// What hangs from the bar, and how softly the two blend.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Merge {
    /// Left, top, width and height of what hangs, its top at the bar's bottom.
    pub hang: [f32; 4],
    /// Its corners' radius.
    pub radius: f32,
    /// How wide the blend between the two is; 0 joins them with a sharp corner. Never more than
    /// twice what hangs is tall, so the blend shrinks away with it rather than outlasting it.
    pub blend: f32,
}

/// A droplet of the bar's glass at the bar's height, its own size throughout, sliding out of the
/// bar's end or the droplet before it. Neither ever changes shape: while the droplet still
/// overlaps what it leaves a waist presses in there, and once it is clear a thread thinner than
/// the waist joins the two round ends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Split {
    /// Its left edge and width; it stands at the bar's top, [`COLLAPSED_H`] tall.
    pub x: f32,
    pub width: f32,
    /// How deep the waist presses in where it parts.
    pub waist: f32,
    /// Half the thread's thickness across the gap; none at 0.
    pub thread: f32,
    /// Where it parts: the end of the bar or of the droplet it splits off.
    pub anchor: f32,
    /// The far edge of the droplet it splits off, past which it never shows while still inside
    /// that droplet; none while it splits off the bar.
    pub cut: Option<f32>,
}

#[cfg(test)]
fn smooth_max(a: f32, b: f32, blend: f32) -> f32 {
    -smooth_min(-a, -b, blend)
}

/// `bar`, the distance to the bar `body` and what hangs from it, with the droplets in `chain`
/// added, their waists cut in and their threads joined on: the shader's `with_chain`, which draws
/// the shape, kept here for its tests.
#[cfg(test)]
pub fn with_chain(
    at: (f32, f32),
    bar: f32,
    body: [f32; 4],
    (radius, power): (f32, f32),
    chain: &[Split],
) -> f32 {
    let [x, y, w, _] = body;
    let half = COLLAPSED_H / 2.0;
    let middle = y + half;
    let shown = || chain.iter().filter(|split| split.width >= 0.5);
    let left = |split: &Split| split.x + split.width / 2.0 < x + w / 2.0;
    let mut d = bar;
    for split in shown() {
        let mut drop = corner_distance(
            at,
            (split.x + split.width / 2.0, middle),
            (split.width / 2.0, half),
            radius,
            power,
        );
        if let Some(cut) = split.cut {
            drop = drop.max(if left(split) { at.0 - cut } else { cut - at.0 });
        }
        d = d.min(drop);
    }
    for split in shown().filter(|split| split.waist > 0.05) {
        let r = WAIST_RADIUS;
        let edge = split.anchor;
        let up =
            ((at.0 - edge).powi(2) + (at.1 - (middle - half - r + split.waist)).powi(2)).sqrt() - r;
        let down =
            ((at.0 - edge).powi(2) + (at.1 - (middle + half + r - split.waist)).powi(2)).sqrt() - r;
        d = smooth_max(d, -up.min(down), WAIST_BLEND);
    }
    for split in shown().filter(|split| split.thread > 0.2) {
        let edge = split.anchor;
        let near = if left(split) {
            split.x + split.width
        } else {
            split.x
        };
        let gap = (edge - near).abs();
        if gap > 0.05 {
            let thread = corner_distance(
                at,
                ((edge + near) / 2.0, middle),
                (gap / 2.0 + split.thread + 2.0, split.thread),
                split.thread,
                2.0,
            );
            d = smooth_min(d, thread, THREAD_BLEND);
        }
    }
    d
}

/// Signed distance from `at` to the edge of the box centred on `center`, `half` its size, with
/// corners of `radius` and `power`: below 0 inside.
pub fn corner_distance(
    (x, y): (f32, f32),
    (cx, cy): (f32, f32),
    (hw, hh): (f32, f32),
    radius: f32,
    power: f32,
) -> f32 {
    let r = radius.min(hw.min(hh));
    let (qx, qy) = ((x - cx).abs() - hw + r, (y - cy).abs() - hh + r);
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    let bend = (ox.powf(power) + oy.powf(power)).powf(power.recip());
    bend + qx.max(qy).min(0.0) - r
}

fn smooth_min(a: f32, b: f32, blend: f32) -> f32 {
    if blend <= 0.0 {
        return a.min(b);
    }
    let h = (blend - (a - b).abs()).max(0.0) / blend;
    a.min(b) - h * h * blend * 0.25
}

/// Signed distance from `at` to the edge of the bar `body`, its corners `corner`, with `merge`
/// hanging from it: below 0 inside. The shader draws the same shape.
pub fn distance(at: (f32, f32), body: [f32; 4], (radius, power): (f32, f32), merge: Merge) -> f32 {
    let [x, y, w, h] = body;
    let bar = corner_distance(
        at,
        (x + w / 2.0, y + h / 2.0),
        (w / 2.0, h / 2.0),
        radius,
        power,
    );
    let [mx, my, mw, mh] = merge.hang;
    if mh < 0.5 || mw < 1.0 {
        return bar;
    }
    let top = y + h * TUCK;
    let half = (mw / 2.0, (my + mh - top) / 2.0);
    let menu = corner_distance(at, (mx + half.0, top + half.1), half, merge.radius, 2.0);
    if at.1 < top {
        bar.min(menu)
    } else {
        smooth_min(bar, menu, merge.blend.min(2.0 * mh))
    }
}

/// The rows of whole pixels the bar and its menu cover, as left, top, width and height, for a
/// region the compositor takes, such as where to blur. Each droplet gives its own.
pub fn rows(body: [f32; 4], corner: (f32, f32), merge: Merge) -> Vec<[i32; 4]> {
    let [x, y, w, h] = body;
    let [mx, my, mw, mh] = merge.hang;
    let bottom = (my + mh).max(y + h);
    let (left, right) = (
        x.min(mx - merge.blend) - 1.0,
        (x + w).max(mx + mw + merge.blend) + 1.0,
    );
    let mut found: Vec<[i32; 4]> = Vec::new();
    for row in y.floor() as i32..bottom.ceil() as i32 {
        let at_y = row as f32 + 0.5;
        let inner = if at_y < y + h {
            x + w / 2.0
        } else {
            mx + mw / 2.0
        };
        let reach = |at_x: f32| distance((at_x, at_y), body, corner, merge);
        if reach(inner) > 0.0 {
            continue;
        }
        let edge = |mut outside: f32, mut within: f32| {
            for _ in 0..20 {
                let middle = (outside + within) / 2.0;
                if reach(middle) > 0.0 {
                    outside = middle;
                } else {
                    within = middle;
                }
            }
            ((outside + within) / 2.0).round() as i32
        };
        let (from, to) = (edge(left, inner), edge(right, inner));
        match found.last_mut() {
            Some(last) if last[0] == from && last[2] == to - from && last[1] + last[3] == row => {
                last[3] += 1;
            }
            _ => found.push([from, row, to - from, 1]),
        }
    }
    found
}

/// A rounded `body` with corners `(radius, power)`, as whole-pixel rows.
pub fn region([x, y, w, h]: [i32; 4], (radius, power): (f32, f32)) -> Vec<[i32; 4]> {
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

#[cfg(test)]
#[path = "merge_test.rs"]
mod tests;
