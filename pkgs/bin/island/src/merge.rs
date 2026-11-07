//! The bar and a menu hanging from it as one surface, blended like liquid where they meet.

/// How far the menu reaches up into the bar, as a share of the bar's height. Its top is hidden
/// there, so only the blend shows where the two meet, and the bar's top stays flat.
const TUCK: f32 = 0.5;

/// What hangs from the bar, and how softly the two blend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Merge {
    /// Left, top, width and height of what hangs, its top at the bar's bottom.
    pub hang: [f32; 4],
    /// Its corners' radius.
    pub radius: f32,
    /// How wide the blend between the two is; 0 joins them with a sharp corner. Never more than
    /// twice what hangs is tall, so the blend shrinks away with it rather than outlasting it.
    pub blend: f32,
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

/// The rows of whole pixels the shape covers, as left, top, width and height, for a region the
/// compositor takes, such as where to blur.
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

#[cfg(test)]
#[path = "merge_test.rs"]
mod tests;
