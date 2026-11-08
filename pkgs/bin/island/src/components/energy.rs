//! A moving border showing something is busy, such as a refresh on its way.

use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Duration,
};

use crate::theme::{SAPPHIRE, SKY, TEXT};

pub const LAP: Duration = Duration::from_millis(1400);
pub const FADE: Duration = Duration::from_millis(260);
/// A fraction of the way round from the top-left.
pub const START: f32 = 0.12;
/// The fraction of the edge the beam covers.
const BEAM: f32 = 0.35;
/// The height the beam's widths are drawn for; it scales with what it runs round.
const UNIT_H: f32 = 30.0;
/// Between the dots that make up the beam.
const STEP: f32 = 0.75;

pub struct Edge {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

/// One of the dots a beam is made of.
pub struct Dot {
    pub at: (f32, f32),
    pub diameter: f32,
    pub color: u32,
    pub alpha: f32,
}

impl Edge {
    fn length(&self) -> f32 {
        2.0 * (self.width - 2.0 * self.radius)
            + 2.0 * (self.height - 2.0 * self.radius)
            + TAU * self.radius
    }

    /// `u` of the way round, clockwise from the start of the top edge.
    fn point(&self, u: f32) -> (f32, f32) {
        let Edge {
            x,
            y,
            width,
            height,
            radius: r,
        } = *self;
        let across = width - 2.0 * r;
        let down = height - 2.0 * r;
        let arc = FRAC_PI_2 * r;
        let corner = |cx: f32, cy: f32, from: f32, d: f32| {
            let angle = from + if r > 0.0 { d / r } else { 0.0 };
            (cx + r * angle.cos(), cy + r * angle.sin())
        };
        let mut d = u.rem_euclid(1.0) * self.length();
        if d < across {
            return (x + r + d, y);
        }
        d -= across;
        if d < arc {
            return corner(x + width - r, y + r, -FRAC_PI_2, d);
        }
        d -= arc;
        if d < down {
            return (x + width, y + r + d);
        }
        d -= down;
        if d < arc {
            return corner(x + width - r, y + height - r, 0.0, d);
        }
        d -= arc;
        if d < across {
            return (x + width - r - d, y + height);
        }
        d -= across;
        if d < arc {
            return corner(x + r, y + height - r, FRAC_PI_2, d);
        }
        d -= arc;
        if d < down {
            return (x, y + height - r - d);
        }
        d -= down;
        corner(x + r, y + r, -2.0 * FRAC_PI_2, d.min(arc))
    }
}

/// The head is `phase` of the way round; the tail thins and fades behind it.
pub fn dots(edge: &Edge, phase: f32, opacity: f32) -> Vec<Dot> {
    let scale = edge.height / UNIT_H;
    let mut shown = Vec::new();
    let dots = ((BEAM * edge.length() / STEP).ceil() as usize).max(24);
    for index in (0..=dots).rev() {
        let behind = index as f32 / dots as f32;
        let lit = 1.0 - behind;
        let diameter = (1.4 + 1.8 * lit) * scale;
        // Each spot is covered by about `diameter / STEP` overlapping dots, so each dot is fainter
        // than the beam is meant to look there.
        let target = lit * lit * 0.95 * opacity;
        let alpha = 1.0 - (1.0 - target).powf(STEP / diameter);
        shown.push(Dot {
            at: edge.point(phase - BEAM * behind),
            diameter,
            color: SAPPHIRE,
            alpha,
        });
    }
    shown.push(Dot {
        at: edge.point(phase - BEAM),
        diameter: 1.4 * scale,
        color: SAPPHIRE,
        alpha: 0.12 * opacity,
    });
    let head = edge.point(phase);
    shown.push(Dot {
        at: head,
        diameter: 4.0 * scale,
        color: SKY,
        alpha: 0.70 * opacity,
    });
    shown.push(Dot {
        at: head,
        diameter: 1.8 * scale,
        color: TEXT,
        alpha: 0.80 * opacity,
    });
    shown
}

#[cfg(test)]
#[path = "energy_test.rs"]
mod tests;
