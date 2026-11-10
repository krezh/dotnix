//! Where an island stands, as wgpu draws it: on its own room at the top of the screen, or, while a
//! feature takes the screen, on the whole of it, the island moved
//! to the middle and at the end switched off like an old screen.

use std::time::Instant;

use super::{
    panel::{Beam, edge_beam},
    renderer::{NO_CLIP, NO_EDGES, Sprite, View},
    widget::{color, tinted},
};
use crate::{
    components::{energy::Hue, marquee::smoothstep},
    spring::Spring,
    stage::{SHAPE_SPRING, SURFACE_H, SURFACE_W, SWITCH_OFF, Stance},
    theme::{BLUE, RED, TEXT},
};

/// The line a switched-off island folds into, and how far its colors part either side.
const LINE_H: f32 = 3.0;
const FRINGE: f32 = 3.0;
/// The dot it pinches into, and the flare round it.
const DOT: f32 = 4.0;
const FLARE: f32 = 240.0;
/// Each part of switching off, in seconds from its start.
const FOLD: (f32, f32) = (0.0, 0.26);
const LINE: (f32, f32) = (0.24, 0.78);
const FLARE_AT: (f32, f32) = (0.60, 1.12);
const BLACK: (f32, f32) = (0.0, 0.9);

/// How far along `span` the time `t` is, from 0 to 1.
fn along(t: f32, (from, to): (f32, f32)) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn lerp(from: f32, to: f32, by: f32) -> f32 {
    from + (to - from) * by
}

/// What switching off shows `t` seconds in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crt {
    /// How far the island has folded towards its line, 0 to 1, and how white it has gone.
    pub fold: f32,
    pub island: f32,
    /// The line's width as a share of the island's, its height and its strength.
    pub line: (f32, f32, f32),
    /// The flare's size as a share of `FLARE`, and its strength.
    pub flare: (f32, f32),
    pub black: f32,
}

pub fn crt(t: f32) -> Crt {
    let fold = along(t, FOLD);
    let fold = fold * fold * fold;
    let island = 1.0 - along(t, (FOLD.1 - 0.02, FOLD.1));
    let line_at = along(t, LINE);
    let pinch = smoothstep(along(line_at, (0.35, 0.9)));
    let line_strength = if t < LINE.0 {
        0.0
    } else {
        along(t, (LINE.0, LINE.0 + 0.03)) * (1.0 - along(t, (LINE.1 - 0.08, LINE.1)))
    };
    let flare_at = along(t, FLARE_AT);
    let (flare_size, flare_strength) = if flare_at <= 0.0 || flare_at >= 1.0 {
        (0.0, 0.0)
    } else if flare_at < 0.3 {
        let up = flare_at / 0.3;
        (lerp(0.2, 0.45, up), up)
    } else {
        let down = (flare_at - 0.3) / 0.7;
        (lerp(0.45, 0.05, down), 1.0 - down)
    };
    Crt {
        fold,
        island,
        line: (1.0 - pinch, lerp(LINE_H, 2.0, pinch), line_strength),
        flare: (flare_size, flare_strength),
        black: smoothstep(along(t, BLACK)),
    }
}

/// `sprite` folded about `(cx, cy)` by `sx` across and `sy` down, and `white` of the way to white.
pub fn folded(
    mut sprite: Sprite,
    (cx, cy): (f32, f32),
    (sx, sy): (f32, f32),
    white: f32,
) -> Sprite {
    let x = |value: f32| cx + (value - cx) * sx;
    let y = |value: f32| cy + (value - cy) * sy;
    sprite.rect = [
        x(sprite.rect[0]),
        y(sprite.rect[1]),
        sprite.rect[2] * sx,
        sprite.rect[3] * sy,
    ];
    if sprite.clip != NO_CLIP {
        sprite.clip = [
            x(sprite.clip[0]),
            y(sprite.clip[1]),
            x(sprite.clip[2]),
            y(sprite.clip[3]),
        ];
    }
    if sprite.edges != NO_EDGES {
        sprite.edges[0] = y(sprite.edges[0]);
        sprite.edges[2] = y(sprite.edges[2]);
    }
    sprite.params[0] *= sx.min(sy);
    sprite.color = sprite.color.map(|channel| lerp(channel, 1.0, white));
    sprite
}

/// How far down an island `top` from the top of a screen `screen` tall moves a panel `panel` tall
/// to stand in its middle.
pub fn lift_to_middle(screen: f32, panel: f32, top: f32) -> f32 {
    ((screen - panel) / 2.0 - top).max(0.0)
}

pub struct Stand {
    stance: Stance,
    /// The surface's size: the island's own room, or the whole screen.
    pub size: (f32, f32),
    /// The whole screen has been asked for.
    pub asked: bool,
    lift: Spring,
    /// Since when it has been switching off, and whether in the middle of the screen.
    off: Option<(Instant, bool)>,
    beam: Beam,
}

impl Stand {
    pub fn new() -> Self {
        Self {
            stance: Stance::Bar,
            size: (SURFACE_W, SURFACE_H),
            asked: false,
            lift: Spring::new(0.0, SHAPE_SPRING, 0.1),
            off: None,
            beam: Beam::default(),
        }
    }

    /// Its surface is the whole screen, whether or not it is still wanted.
    fn on_screen(&self) -> bool {
        self.size != (SURFACE_W, SURFACE_H)
    }

    /// Has the whole screen, and keeps it.
    fn holds_screen(&self) -> bool {
        self.asked && self.on_screen()
    }

    /// Back in the bar's room with nothing left to undo.
    fn home(&self) -> bool {
        self.stance == Stance::Bar
            && self.off.is_none()
            && self.lift.is_settled()
            && self.lift.value().abs() < 0.01
    }

    pub fn wants_screen(&self) -> bool {
        !self.home()
    }

    pub fn takes_input(&self) -> bool {
        self.stance.centered() && self.holds_screen()
    }

    /// Follows `stance`, moving to the middle for a panel `panel` tall below `top`.
    pub fn aim(&mut self, stance: Stance, panel: f32, top: f32, now: Instant) {
        if stance == Stance::Off && self.stance != Stance::Off {
            self.off = Some((now, self.stance.centered()));
        } else if stance != Stance::Off {
            self.off = None;
        }
        self.stance = stance;
        let middle = stance.centered() && self.holds_screen();
        let lift = if middle {
            lift_to_middle(self.size.1, panel, top)
        } else {
            0.0
        };
        if (self.lift.target() - lift).abs() > 0.5 {
            self.lift.retarget(lift, SHAPE_SPRING);
        }
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.lift.advance(dt, now);
    }

    pub fn is_settled(&self) -> bool {
        self.lift.is_settled()
            && self
                .off
                .is_none_or(|(since, _)| since.elapsed() >= SWITCH_OFF)
    }

    /// Where the island's sprites go on the surface, in whole pixels at `scale`.
    pub fn shift(&self, scale: f32) -> (f32, f32) {
        let whole = |value: f32| (value * scale).round() / scale;
        (
            whole((self.size.0 - SURFACE_W) / 2.0),
            whole(self.lift.value()),
        )
    }

    pub fn view(&self, scale: f32) -> View {
        let (x, y) = self.shift(scale);
        View {
            size: [self.size.0, self.size.1],
            shift: [x, y],
        }
    }

    /// The whole surface, in the island's own coordinates.
    fn whole(&self, scale: f32) -> [f32; 4] {
        let (x, y) = self.shift(scale);
        [-x, -y, self.size.0, self.size.1]
    }

    /// While switching off, folds `sprites` away and turns the screen behind black; `body` is the
    /// island's shape and `radius` its corners.
    pub fn dress(&mut self, sprites: &mut Vec<Sprite>, body: [f32; 4], radius: f32, scale: f32) {
        let crt = self
            .off
            .map(|(since, centered)| (crt(since.elapsed().as_secs_f32()), centered));
        if let Stance::Center { busy } = self.stance {
            let shone = self.beam.shine(busy.is_some());
            if let (Some((phase, strength)), Some(tint)) = (shone, busy) {
                edge_beam(body, radius, 1.0, (phase, strength), Hue::of(tint), sprites);
            }
        }
        match crt {
            Some((crt, true)) => self.fold(sprites, body, crt),
            Some((crt, false)) => {
                for sprite in sprites.iter_mut() {
                    *sprite = sprite.faded(1.0 - crt.black);
                }
            }
            None => {}
        }
        let black = crt.map_or(0.0, |(crt, _)| crt.black);
        if black > 0.001 {
            let tint = [0.0, 0.0, 0.0, black];
            sprites.insert(0, Sprite::shape(self.whole(scale), tint, 0.0, 2.0));
        }
    }

    fn fold(&self, sprites: &mut Vec<Sprite>, body: [f32; 4], crt: Crt) {
        let middle = (body[0] + body[2] / 2.0, body[1] + body[3] / 2.0);
        let squash = lerp(1.0, LINE_H / body[3].max(LINE_H), crt.fold);
        let widen = lerp(1.0, 1.02, crt.fold);
        for sprite in sprites.iter_mut() {
            *sprite = folded(*sprite, middle, (widen, squash), crt.fold).faded(crt.island);
        }
        let (share, tall, strength) = crt.line;
        if strength > 0.001 {
            let wide = (body[2] * 1.02 * share).max(DOT);
            let rect = [middle.0 - wide / 2.0, middle.1 - tall / 2.0, wide, tall];
            let lit = |tint: u32, alpha: f32| color(tint).map(|channel| channel * alpha * strength);
            let grown = |by: f32| {
                [
                    rect[0] - by,
                    rect[1] - by,
                    rect[2] + 2.0 * by,
                    rect[3] + 2.0 * by,
                ]
            };
            sprites.push(Sprite::glow(grown(80.0), lit(BLUE, 0.35), 80.0, 80.0));
            sprites.push(Sprite::glow(grown(18.0), lit(TEXT, 0.8), 18.0, 18.0));
            sprites.push(Sprite::shape(rect, lit(RED, 0.75), tall / 2.0, 2.0).moved(-FRINGE, 0.0));
            sprites.push(Sprite::shape(rect, lit(BLUE, 0.75), tall / 2.0, 2.0).moved(FRINGE, 0.0));
            sprites.push(Sprite::shape(rect, [strength; 4], tall / 2.0, 2.0));
        }
        let (size, strength) = crt.flare;
        if strength > 0.001 {
            let side = FLARE * size;
            let rect = [middle.0 - side / 2.0, middle.1 - side / 2.0, side, side];
            sprites.push(Sprite::glow(
                rect,
                tinted(BLUE, 0x40).map(|channel| channel * strength),
                side / 2.0,
                side / 2.0,
            ));
            let core = side * 0.25;
            sprites.push(Sprite::glow(
                [middle.0 - core / 2.0, middle.1 - core / 2.0, core, core],
                [strength; 4],
                core / 2.0,
                core / 2.0,
            ));
        }
    }

    /// While it has the screen: where it takes the pointer, and where the screen behind is blurred,
    /// only under the island, `body` with `corner`, and nowhere once it switches off; both in the
    /// surface's coordinates.
    pub fn regions(
        &self,
        body: [f32; 4],
        corner: (f32, f32),
        scale: f32,
    ) -> Option<(Vec<[i32; 4]>, Vec<[i32; 4]>)> {
        if !self.on_screen() {
            return None;
        }
        let (x, y) = self.shift(scale);
        let whole = [0.0, 0.0, self.size.0, self.size.1];
        let input = if self.takes_input() {
            vec![whole.map(|value| value.round() as i32)]
        } else {
            vec![[body[0] + x, body[1] + y, body[2], body[3]].map(|value| value.round() as i32)]
        };
        let shown = [body[0] + x, body[1] + y, body[2], body[3]];
        let blur = if self.off.is_some() {
            Vec::new()
        } else {
            crate::merge::region(shown.map(|value| value.round() as i32), corner)
        };
        Some((input, blur))
    }
}

#[cfg(test)]
#[path = "stand_test.rs"]
mod tests;
