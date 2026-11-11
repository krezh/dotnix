//! The bar, a menu hanging from it and a droplet splitting off one of its ends, as one surface,
//! blended like liquid where they meet.

use crate::stage::COLLAPSED_H;

/// How far the menu reaches up into the bar, as a share of the bar's height. Its top is hidden
/// there, so only the blend shows where the two meet, and the bar's top stays flat.
const TUCK: f32 = 0.5;
/// How round the two sides of the waist pressing in at the bar's end are.
pub const WAIST_RADIUS: f32 = 30.0;
/// How softly the waist is cut in, and the thread joined on.
const WAIST_BLEND: f32 = 7.0;
const THREAD_BLEND: f32 = 6.0;

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
    /// A droplet splitting off one end of the bar.
    pub split: Option<Split>,
}

/// A droplet of the bar's glass at the bar's height, its own size throughout, sliding out of one
/// end. The bar never changes shape: while the droplet still overlaps it a waist presses in at
/// the bar's end, and once it is clear a thread thinner than the waist joins the two round ends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Split {
    /// Its left edge and width; it stands at the bar's top, [`COLLAPSED_H`] tall.
    pub x: f32,
    pub width: f32,
    /// How deep the waist presses in at the bar's end.
    pub waist: f32,
    /// Half the thread's thickness across the gap; none at 0.
    pub thread: f32,
}

fn smooth_max(a: f32, b: f32, blend: f32) -> f32 {
    -smooth_min(-a, -b, blend)
}

/// `bar` with the droplet `split` of the bar `body` added, its waist cut in and its thread
/// joined on.
fn with_split(at: (f32, f32), bar: f32, body: [f32; 4], corner: (f32, f32), split: Split) -> f32 {
    let [x, y, w, _] = body;
    let half = COLLAPSED_H / 2.0;
    let middle = y + half;
    if split.width < 0.5 {
        return bar;
    }
    let drop = corner_distance(
        at,
        (split.x + split.width / 2.0, middle),
        (split.width / 2.0, half),
        corner.0,
        corner.1,
    );
    let mut d = bar.min(drop);
    let left = split.x + split.width / 2.0 < x + w / 2.0;
    let edge = if left { x } else { x + w };
    if split.waist > 0.05 {
        let r = WAIST_RADIUS;
        let up =
            ((at.0 - edge).powi(2) + (at.1 - (middle - half - r + split.waist)).powi(2)).sqrt() - r;
        let down =
            ((at.0 - edge).powi(2) + (at.1 - (middle + half + r - split.waist)).powi(2)).sqrt() - r;
        d = smooth_max(d, -up.min(down), WAIST_BLEND);
    }
    let near = if left { split.x + split.width } else { split.x };
    let gap = if left { edge - near } else { near - edge };
    if split.thread > 0.2 && gap > 0.05 {
        let thread = corner_distance(
            at,
            ((edge + near) / 2.0, middle),
            (gap / 2.0 + split.thread + 2.0, split.thread),
            split.thread,
            2.0,
        );
        d = smooth_min(d, thread, THREAD_BLEND);
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
/// hanging from it and its droplet: below 0 inside. The shader draws the same shape.
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
    let joined = if mh < 0.5 || mw < 1.0 {
        bar
    } else {
        let top = y + h * TUCK;
        let half = (mw / 2.0, (my + mh - top) / 2.0);
        let menu = corner_distance(at, (mx + half.0, top + half.1), half, merge.radius, 2.0);
        if at.1 < top {
            bar.min(menu)
        } else {
            smooth_min(bar, menu, merge.blend.min(2.0 * mh))
        }
    };
    match merge.split {
        Some(split) => with_split(at, joined, body, (radius, power), split),
        None => joined,
    }
}

/// The rows of whole pixels the bar and its menu cover, as left, top, width and height, for a
/// region the compositor takes, such as where to blur. The droplet gives its own.
pub fn rows(body: [f32; 4], corner: (f32, f32), merge: Merge) -> Vec<[i32; 4]> {
    let merge = Merge {
        split: None,
        ..merge
    };
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
