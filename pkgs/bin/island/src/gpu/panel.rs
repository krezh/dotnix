//! A panel laid out into sprites once, at the origin; the island moves and fades them, so a morph
//! reuses them and only new data lays them out again. Charts and countdowns move on their own, so
//! their sprites are made each frame.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use super::{
    renderer::{IMAGE, NO_CLIP, RING, SHAPE, Sprite, inside, overlap},
    widget::{color, tinted},
};
use crate::{
    components::{
        card,
        chart::{BASELINE_ALPHA, FILL_ALPHA, LINE, Phase, Series, history_points},
        energy,
        marquee::{FADE as MARQUEE_FADE, GAP as MARQUEE_GAP, SPEED as MARQUEE_SPEED, smoothstep},
        motion::{MORPH, stagger},
        popover::{SHADOW_SPREAD, cover, opener_shown, rows_shown},
        search::Scroll as Rows,
        slider::OnChange,
    },
    panel::{Handler, SharedString},
    spring::SpringConfig,
    theme::{POPOVER_OPACITY, SHADOW_ALPHA, SURFACE, SURFACE_BORDER_ALPHA, SURFACE1, TINT},
    ui::CONTROL_RADIUS,
};

/// How far a surface that drops from what opens it falls below it.
pub const DROP: f32 = 6.0;

/// Takes down by `stretch` those of `sprites` in `rect`, a scrolling area pulled past its end.
pub fn pulled_along(sprites: &mut [Sprite], rect: [f32; 4], stretch: f32) {
    for sprite in sprites {
        let [x, y, w, h] = sprite.rect;
        if inside(rect, (x + w / 2.0, y + h / 2.0)) {
            sprite.rect[1] += stretch;
        }
    }
}

/// A history to plot.
pub struct Chart {
    pub rect: [f32; 4],
    pub series: Vec<Series>,
    pub phase: Option<Phase>,
    pub clip: Option<[f32; 4]>,
    pub fade: f32,
}

/// Something answering the pointer, innermost last.
#[derive(Default)]
pub struct Hit {
    pub rect: [f32; 4],
    /// The sprite under what it holds that shows the pointer is on it.
    pub hover: Option<usize>,
    pub on_click: Option<Handler>,
    pub on_right: Option<Handler>,
    pub on_hover: Option<Handler>,
    pub on_rows: Option<Rows>,
    pub tip: Option<SharedString>,
    /// Acts as the button goes down rather than when it comes back up.
    pub immediate: bool,
    /// Nothing under it shows a hover or a tip.
    pub blocks: bool,
    /// The sprites it dims while held.
    pub from: usize,
    pub to: usize,
    /// What its words turn to under the pointer, and which sprites they are.
    pub lit: Option<([f32; 4], Vec<(usize, usize)>)>,
}

/// Sprites that show only while the pointer is in `group`'s rect.
pub struct Reveal {
    pub group: SharedString,
    pub from: usize,
    pub to: usize,
}

/// Text too wide for its slot, gliding in a loop and dissolving into both edges.
pub struct Marquee {
    pub key: u64,
    /// Glides only while the pointer is on this, otherwise resting at its start.
    pub hover: Option<[f32; 4]>,
    pub rect: [f32; 4],
    pub width: f32,
    /// At the start of the slot.
    pub sprites: Vec<Sprite>,
    pub clip: Option<[f32; 4]>,
    pub fade: f32,
}

#[derive(Clone, Copy)]
pub enum PulseKind {
    /// Breathing in and out, as something thinking.
    Glow,
    /// The nth of a row of dots waiting.
    Dot(usize),
}

/// A shape whose strength follows the clock.
pub struct Pulse {
    pub rect: [f32; 4],
    pub tint: [f32; 4],
    pub radius: f32,
    pub kind: PulseKind,
    pub clip: Option<[f32; 4]>,
    pub fade: f32,
}

pub fn pulse_sprite(pulse: &Pulse, seconds: f32) -> Sprite {
    const GLOW_ALPHA: f32 = 0.55;
    let strength = match pulse.kind {
        PulseKind::Glow => {
            let t = (seconds / 1.6).fract();
            (0.5 - 0.5 * (t * std::f32::consts::TAU).cos()) * GLOW_ALPHA
        }
        PulseKind::Dot(index) => {
            let t = (seconds / 1.1).fract();
            let phase = (t - index as f32 * 0.14).rem_euclid(1.0);
            0.35 + 0.65 * (phase * std::f32::consts::TAU).sin().max(0.0)
        }
    };
    let sprite =
        Sprite::shape(pulse.rect, pulse.tint, pulse.radius, 2.0).faded(strength * pulse.fade);
    sprite.within_any(pulse.clip)
}

/// One character turning into another, or appearing or going.
pub struct Slot {
    pub from: Vec<Sprite>,
    pub from_w: f32,
    pub to: Vec<Sprite>,
    pub to_w: f32,
}

/// Text turning into its new value: what the two share stays and the rest turns in place, one
/// character after another. Laid out once, at the left of the text.
pub struct MorphText {
    pub rect: [f32; 4],
    /// Where in `rect` it sits as its width changes: 0 at the start, 1 at the end.
    pub align: f32,
    pub since: Instant,
    pub start: (Vec<Sprite>, f32),
    pub slots: Vec<Slot>,
    pub end: Vec<Sprite>,
    /// As it is once done, in place.
    pub rest: Vec<Sprite>,
    pub clip: Option<[f32; 4]>,
    pub fade: f32,
}

impl MorphText {
    pub fn is_running(&self) -> bool {
        self.since.elapsed() < MORPH
    }
}

/// What was last shown at a place in a panel, for morphing from.
pub struct Shown {
    pub text: SharedString,
    pub from: Option<(SharedString, Instant)>,
    pub changed: Option<Instant>,
}

/// What a place shows now that it shows `text`, having shown `prior`. A change while the last is
/// still morphing shows at once, so text changing all the time keeps up.
pub fn noted(prior: Option<&Shown>, text: &SharedString) -> Shown {
    let now = Instant::now();
    let running = |since: &Instant| now.duration_since(*since) < MORPH;
    let (from, changed) = match prior {
        Some(prior) if prior.text != *text => {
            let busy = prior.changed.as_ref().is_some_and(running);
            ((!busy).then(|| (prior.text.clone(), now)), Some(now))
        }
        Some(prior) => (
            prior.from.clone().filter(|(_, since)| running(since)),
            prior.changed,
        ),
        None => (None, None),
    };
    Shown {
        text: text.clone(),
        from,
        changed,
    }
}

pub fn morph_sprites(morph: &MorphText, out: &mut Vec<Sprite>) {
    let shown = |sprite: &Sprite, x: f32, fade: f32| {
        sprite
            .moved(x, 0.0)
            .faded(fade * morph.fade)
            .within_any(morph.clip)
    };
    let progress = morph.since.elapsed().as_secs_f32() / MORPH.as_secs_f32();
    if progress >= 1.0 {
        out.extend(morph.rest.iter().map(|sprite| shown(sprite, 0.0, 1.0)));
        return;
    }
    let count = morph.slots.len();
    let turned: Vec<f32> = (0..count)
        .map(|index| smoothstep(stagger(progress, index, count)))
        .collect();
    let width = |slot: &Slot, turned: f32| slot.from_w + (slot.to_w - slot.from_w) * turned;
    let end_w = morph.end.last().map_or(0.0, |last| {
        last.rect[0] + last.rect[2] - morph.end.first().map_or(0.0, |first| first.rect[0])
    });
    let total = morph.start.1
        + morph
            .slots
            .iter()
            .zip(&turned)
            .map(|(slot, turned)| width(slot, *turned))
            .sum::<f32>()
        + end_w;
    let [x, y, room, h] = morph.rect;
    let left = (x + morph.align * (room - total)).round();
    out.extend(morph.start.0.iter().map(|sprite| shown(sprite, left, 1.0)));
    let mut at = left + morph.start.1;
    for (slot, turned) in morph.slots.iter().zip(&turned) {
        let wide = width(slot, *turned);
        let cell = [at, y, at + wide, y + h];
        let place = at.round();
        out.extend(
            slot.from
                .iter()
                .map(|sprite| shown(sprite, place, 1.0 - turned).within(cell)),
        );
        out.extend(
            slot.to
                .iter()
                .map(|sprite| shown(sprite, place, *turned).within(cell)),
        );
        at += wide;
    }
    out.extend(
        morph
            .end
            .iter()
            .map(|sprite| shown(sprite, at.round(), 1.0)),
    );
}

/// A border that moves while something is busy, kept across frames.
#[derive(Default)]
pub struct Beam {
    since: Option<Instant>,
    fading: Option<Instant>,
}

impl Beam {
    /// Where it has got to and how strong it is, if it shows.
    pub fn shine(&mut self, busy: bool) -> Option<(f32, f32)> {
        let now = Instant::now();
        if busy {
            self.fading = None;
            self.since.get_or_insert(now);
        } else if self.since.is_some() {
            self.fading.get_or_insert(now);
        }
        let since = self.since?;
        let fade = self.fading.map_or(0.0, |fading| {
            now.duration_since(fading).as_secs_f32() / energy::FADE.as_secs_f32()
        });
        if fade >= 1.0 {
            *self = Self::default();
            return None;
        }
        let phase =
            energy::START + now.duration_since(since).as_secs_f32() / energy::LAP.as_secs_f32();
        Some((phase, 1.0 - smoothstep(fade)))
    }
}

/// Round the inside of `rect`.
pub fn beam_sprites(rect: [f32; 4], phase: f32, opacity: f32, out: &mut Vec<Sprite>) {
    let (width, height) = (rect[2] - 2.0, rect[3] - 2.0);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let edge = energy::Edge {
        x: rect[0] + 1.0,
        y: rect[1] + 1.0,
        width,
        height,
        radius: (CONTROL_RADIUS - 1.0).max(0.0).min(width.min(height) / 2.0),
    };
    for dot in energy::dots(&edge, phase, opacity) {
        let (x, y) = dot.at;
        let side = dot.diameter;
        out.push(Sprite::shape(
            [x - side / 2.0, y - side / 2.0, side, side],
            color(dot.color).map(|channel| channel * dot.alpha.clamp(0.0, 1.0)),
            side / 2.0,
            2.0,
        ));
    }
}

/// Sprites coming into view or going, as one.
pub struct Rise {
    pub key: u64,
    pub from: usize,
    pub to: usize,
    pub leaving: bool,
}

/// A list growing out of what opens it, with what it needs to morph from that to itself.
pub struct Surface {
    pub key: SharedString,
    pub open: bool,
    /// Under what opens it, keeping their right edges together; otherwise from it, both ways.
    pub drops: bool,
    pub anchor: [f32; 4],
    /// The list, as laid out.
    pub rect: [f32; 4],
    pub rest_fill: [f32; 4],
    pub rest_border: [f32; 4],
    pub rest_radius: f32,
    pub rows: (usize, usize),
    pub label: (usize, usize),
    /// What opens it, which gives way to it while it is out.
    pub chip: Option<(usize, usize)>,
    /// Its rows' hits, which wait for the rows to show.
    pub hits: (usize, usize),
    /// Its rows show, so they take the pointer.
    pub live: bool,
}

fn mix(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

pub fn mixed(from: [f32; 4], to: [f32; 4], at: f32) -> [f32; 4] {
    [0, 1, 2, 3].map(|channel| mix(from[channel], to[channel], at))
}

/// `grown` is 0 at what opens it and 1 at the list, and may pass them; `revealed` is how far its
/// rows have come. `sprites` starts at sprite `above` of the panel.
pub fn surface_sprites(
    surface: &Surface,
    sprites: &[Sprite],
    above: usize,
    (ax, aw): (f32, f32),
    (grown, revealed): (f32, f32),
    out: &mut Vec<Sprite>,
) {
    let look = grown.clamp(0.0, 1.0);
    if !surface.open && grown.abs() <= 0.01 {
        return;
    }
    let [home_x, ay, home_w, ah] = surface.anchor;
    let along = (ax + aw / 2.0) - (home_x + home_w / 2.0);
    let [open_x, open_y, open_w, open_h] = surface.rect;
    let w = mix(aw, open_w, grown).max(0.0);
    let h = mix(ah, open_h, grown).max(0.0).round();
    let shown_cover = cover(look);
    let glass = color(SURFACE).map(|channel| channel * POPOVER_OPACITY);
    let edge = tinted(SURFACE1, SURFACE_BORDER_ALPHA);
    let (left, right, top, radius, fill, border) = if surface.drops {
        let right = ax + aw;
        (
            (right - w).round(),
            right,
            (ay + mix(0.0, ah + DROP, look)).round(),
            CONTROL_RADIUS,
            glass.map(|channel| channel * shown_cover),
            edge.map(|channel| channel * shown_cover),
        )
    } else {
        let middle = ax + aw / 2.0;
        let pushed = open_x - (middle - along - open_w / 2.0);
        (
            (middle - w / 2.0 + pushed * look).round(),
            (middle + w / 2.0 + pushed * look).round(),
            ay.round(),
            mix(surface.rest_radius, CONTROL_RADIUS, look),
            mixed(surface.rest_fill, glass, shown_cover),
            mixed(surface.rest_border, edge, shown_cover),
        )
    };
    let rect = [left, top, right - left, h];
    let bounds = [left, top, right, top + h];
    out.push(Sprite::shadow(
        [
            left - SHADOW_SPREAD,
            top - SHADOW_SPREAD,
            rect[2] + 2.0 * SHADOW_SPREAD,
            h + 2.0 * SHADOW_SPREAD,
        ],
        [0.0, 0.0, 0.0, SHADOW_ALPHA * shown_cover],
        radius,
        SHADOW_SPREAD,
        0.0,
    ));
    out.push(Sprite::shape(rect, fill, radius, 2.0));
    out.push(Sprite::ring(rect, border, radius, 2.0));
    let opener = opener_shown(look);
    if opener > 0.0 {
        out.extend(
            sprites[surface.label.0 - above..surface.label.1 - above]
                .iter()
                .map(|sprite| sprite.moved(along, 0.0).faded(opener)),
        );
    }
    let rows = rows_shown(look).min(revealed.clamp(0.0, 1.0));
    if rows > 0.0 {
        let down = top - open_y;
        out.extend(
            sprites[surface.rows.0 - above..surface.rows.1 - above]
                .iter()
                .map(|sprite| sprite.moved(along, down).faded(rows).within(bounds)),
        );
    }
}

/// How a value easing shows on the sprites it was laid out with, which are at its target.
#[derive(Clone, Copy)]
pub enum Easing {
    /// The first sprite is this share of what holds it; the rest are cut off at its end.
    Wide { whole: f32 },
    /// The first sprite is this wide.
    Size,
    /// Moved by this much for each unit it is off.
    Shift { x: f32, y: f32 },
    /// Moved across as a share of what holds it; becomes a [`Easing::Shift`] when painted.
    Along,
    /// Faded to this strength.
    Strength,
    /// Turned this far, for pictures.
    Turn,
    /// Pictures dip to `dip` of their size halfway from 0 to 1.
    Pop { dip: f32 },
    /// Widened from `left` in step with this value.
    Stretch { left: f32 },
    /// The first sprite is an arc ending here.
    Arc,
}

/// A value that eases to its target, and the sprites showing it.
pub struct Ease {
    pub key: u64,
    pub from: usize,
    pub to: usize,
    pub easing: Easing,
    pub value: f32,
    pub config: SpringConfig,
    /// Follows the pointer outright while this track is held.
    pub follows: Option<[f32; 4]>,
}

/// `reach` is as far as it stretches from: what cuts it off past there stays.
fn stretched(sprite: &mut Sprite, left: f32, reach: f32, moved: f32, scale: f32) {
    let across = |x: f32| left + moved + (x - left) * scale;
    let shaped = sprite.params[1] == SHAPE || sprite.params[1] == RING;
    if shaped {
        sprite.rect[2] *= scale;
    }
    sprite.rect[0] = across(sprite.rect[0]);
    if sprite.clip != NO_CLIP {
        for edge in [0, 2] {
            if (left..=left + reach + 0.5).contains(&sprite.clip[edge]) {
                sprite.clip[edge] = across(sprite.clip[edge]);
            }
        }
    }
}

/// Shows `value` of `ease` on the sprites it covers.
pub fn ease_sprites(ease: &Ease, value: f32, sprites: &mut [Sprite]) {
    let Some(first) = sprites.first().copied() else {
        return;
    };
    let off = value - ease.value;
    match ease.easing {
        Easing::Wide { whole } => {
            let width = whole * value.clamp(0.0, 1.0);
            sprites[0].rect[2] = width;
            for sprite in sprites.iter_mut() {
                if sprite.clip != NO_CLIP {
                    sprite.clip[2] = first.rect[0] + width;
                }
            }
        }
        Easing::Size => sprites[0].rect[2] = value,
        Easing::Shift { x, y } => {
            for sprite in sprites {
                *sprite = sprite.moved(off * x, off * y);
            }
        }
        Easing::Along => {}
        Easing::Strength => {
            for sprite in sprites {
                *sprite = sprite.faded(value.clamp(0.0, 1.0));
            }
        }
        Easing::Turn => {
            for sprite in sprites
                .iter_mut()
                .filter(|sprite| sprite.params[1] == IMAGE)
            {
                sprite.params[3] = value;
            }
        }
        Easing::Pop { dip } => {
            let lit = value.clamp(0.0, 1.0);
            let size = 1.0 - (1.0 - dip) * 4.0 * lit * (1.0 - lit);
            for sprite in sprites
                .iter_mut()
                .filter(|sprite| sprite.params[1] == IMAGE)
            {
                let [x, y, w, h] = sprite.rect;
                sprite.rect = [
                    x + w * (1.0 - size) / 2.0,
                    y + h * (1.0 - size) / 2.0,
                    w * size,
                    h * size,
                ];
            }
        }
        Easing::Stretch { left } => {
            if ease.value > 0.0 {
                let scale = value / ease.value;
                for sprite in sprites {
                    stretched(sprite, left, f32::MAX, 0.0, scale);
                }
            }
        }
        Easing::Arc => {
            let to = value.clamp(0.0, 1.0);
            sprites[0].uv[2] = to;
            if to <= 0.001 {
                sprites[0].color = [0.0; 4];
            }
        }
    }
}

/// A track pulled `by` past an end, its sprites widened that way.
pub fn pulled_sprites(track: [f32; 4], by: f32, sprites: &mut [Sprite]) {
    let scale = (track[2] + by.abs()) / track[2].max(1.0);
    for sprite in sprites {
        stretched(sprite, track[0], track[2], by.min(0.0), scale);
    }
}

/// Time running out across a frame, draining toward its start.
#[derive(Clone)]
pub struct Countdown {
    /// Changes with each pause and resume.
    pub key: SharedString,
    pub rect: [f32; 4],
    pub left: f32,
    /// Zero while paused.
    pub running: Duration,
    pub color: u32,
    pub opacity: f32,
}

/// What scrolls inside `rect`, by up to `reach`.
pub struct Scrolling {
    pub key: SharedString,
    pub rect: [f32; 4],
    pub reach: f32,
    /// The sprites of what it holds.
    pub from: usize,
    pub to: usize,
}

/// A track the pointer can drag along.
pub struct Sliding {
    pub key: u64,
    pub rect: [f32; 4],
    /// What of it shows, and so takes the pointer.
    pub seen: [f32; 4],
    pub on_change: OnChange,
    /// The ease of its fill.
    pub fill: Option<(u64, f32)>,
    pub from: usize,
    pub to: usize,
}

#[derive(Default)]
pub struct Built {
    pub width: f32,
    pub height: f32,
    pub sprites: Vec<Sprite>,
    pub charts: Vec<Chart>,
    pub scrolling: Vec<Scrolling>,
    pub hits: Vec<Hit>,
    pub sliding: Vec<Sliding>,
    pub countdowns: Vec<Countdown>,
    pub marquees: Vec<Marquee>,
    pub pulses: Vec<Pulse>,
    pub eases: Vec<Ease>,
    pub morphs: Vec<MorphText>,
    pub rises: Vec<Rise>,
    /// What each place shows now, for the next time it is built.
    pub texts: HashMap<u64, Shown>,
    pub surfaces: Vec<Surface>,
    /// What runs a beam round it, by name: whether it is busy, and where.
    pub beams: Vec<(u64, bool, [f32; 4])>,
    /// The sprites of each anchored part.
    pub anchored: HashMap<SharedString, (usize, usize)>,
    /// Sprites from here on are drawn by the surfaces, over everything, past the island's edge.
    pub above: Option<usize>,
    /// Where what hangs past the island's edge reaches.
    pub reaches: Vec<[f32; 4]>,
    /// Where each caret is, with what cuts it off.
    pub carets: Vec<([f32; 4], Option<[f32; 4]>)>,
    /// Where each anchored part came out, for what opens from it.
    pub anchors: HashMap<SharedString, [f32; 4]>,
    pub reveals: Vec<Reveal>,
    /// Sprites that fade in and out, each from and to.
    pub pulsing: Vec<(usize, usize)>,
}

/// `at` is where the glyph's middle is across a slot `slot` wide; 1 clear of both edges.
pub fn edge_fade(at: f32, slot: f32) -> f32 {
    smoothstep((slot - at) / MARQUEE_FADE).min(smoothstep(at / MARQUEE_FADE))
}

/// How far it has glided in `seconds`, for how much its start edge fades.
pub fn glided(seconds: f32) -> f32 {
    (seconds * MARQUEE_SPEED / MARQUEE_FADE).clamp(0.0, 1.0)
}

/// `seconds` on the shared clock; `start` is how much its start edge fades, 0 to 1.
pub fn marquee_sprites(marquee: &Marquee, seconds: f32, start: f32, out: &mut Vec<Sprite>) {
    let [x, y, slot, h] = marquee.rect;
    let cycle = marquee.width + MARQUEE_GAP;
    let moved = (seconds * MARQUEE_SPEED) % cycle;
    let own = [x, y, x + slot, y + h];
    let clip = marquee.clip.map_or(own, |outer| overlap(own, outer));
    for pass in 0..2 {
        let shift = pass as f32 * cycle - moved;
        for sprite in &marquee.sprites {
            let at = sprite.rect[0] - x + shift + sprite.rect[2] / 2.0;
            let shown = smoothstep((slot - at) / MARQUEE_FADE)
                .min(1.0 - start * (1.0 - smoothstep(at / MARQUEE_FADE)));
            if shown > 0.01 {
                out.push(
                    sprite
                        .moved(shift, 0.0)
                        .faded(shown * marquee.fade)
                        .within(clip),
                );
            }
        }
    }
}

/// Fully lit most of the cycle, easing out and back.
pub fn caret_sprite(rect: [f32; 4], clip: Option<[f32; 4]>, seconds: f32) -> Sprite {
    let t = (seconds / 1.1).fract();
    let opacity = ((t * std::f32::consts::TAU).cos() * 1.6).clamp(-1.0, 1.0) * 0.5 + 0.5;
    let sprite = Sprite::shape(rect, color(crate::theme::ACCENT), rect[2], 2.0).faded(opacity);
    sprite.within_any(clip)
}

/// `phase` is how far the newest sample has scrolled in, 0 to 1. Relative to the panel.
pub fn chart_sprites(chart: &Chart, phase: f32, pixel: f32, out: &mut Vec<Sprite>) {
    let [x0, y0, w, h] = chart.rect;
    let phase = phase.clamp(0.0, 1.0);
    let bottom = y0 + h;
    let own = [x0, y0, x0 + w, bottom];
    let clip = chart.clip.map_or(own, |outer| overlap(own, outer));
    let first = out.len();
    out.push(Sprite::shape(
        [x0, bottom - pixel, w, pixel],
        tinted(TINT, BASELINE_ALPHA),
        0.0,
        2.0,
    ));
    let floor = bottom - LINE / 2.0;
    for (index, series) in chart.series.iter().enumerate() {
        let values = &series.values;
        if values.len() < 2 {
            continue;
        }
        let y = |value: f32, max: f32| {
            floor - (value / max.max(f32::EPSILON)).clamp(0.0, 1.0) * (h - LINE)
        };
        let (points, history) = history_points(values, series.max, card::SLOTS, phase, x0, w, &y);
        if index == 0 {
            let tint = tinted(series.color, FILL_ALPHA);
            for pair in points[points.len() - history..].windows(2) {
                out.push(Sprite::area(pair[0], pair[1], bottom, tint));
            }
        }
        let tint = color(series.color);
        for pair in points.windows(2) {
            out.push(Sprite::line(pair[0], pair[1], LINE, tint));
        }
    }
    for sprite in &mut out[first..] {
        *sprite = sprite.within(clip).faded(chart.fade);
    }
}

/// `gone` is the share of its time since it last changed. Relative to the panel.
pub fn countdown_sprites(countdown: &Countdown, gone: f32, out: &mut Vec<Sprite>) {
    const BLUR: f32 = 46.0;
    const SPREAD: f32 = 18.0;
    const START: f32 = 10.0;
    const END: f32 = 28.0;
    let [x, y, w, h] = countdown.rect;
    let left = countdown.left * (1.0 - gone.clamp(0.0, 1.0));
    let clip = [x, y, x + w, y + h];
    let reach = SPREAD + BLUR;
    for (from, alpha) in [(0.0, 0x38), (0.5, 0x30)] {
        let span = w * left;
        let start = x + from * span + START + SPREAD;
        let end = x + span - END - SPREAD;
        if end <= start {
            continue;
        }
        let middle = y + h / 2.0;
        out.push(
            Sprite::glow(
                [
                    start - reach,
                    middle - reach,
                    end - start + 2.0 * reach,
                    2.0 * reach,
                ],
                tinted(countdown.color, alpha),
                reach,
                BLUR,
            )
            .within(clip)
            .faded(countdown.opacity),
        );
    }
}

impl Built {
    /// What is under `local`, innermost first.
    pub fn under(&self, local: (f32, f32)) -> impl Iterator<Item = &Hit> {
        self.hits
            .iter()
            .enumerate()
            .rev()
            .filter(move |(index, hit)| {
                inside(hit.rect, local)
                    && !self.surfaces.iter().any(|surface| {
                        !surface.live && (surface.hits.0..surface.hits.1).contains(index)
                    })
            })
            .map(|(_, hit)| hit)
    }

    /// The innermost hit under `local` that `wanted` picks, unless one that blocks is over it.
    pub fn answering(&self, local: (f32, f32), wanted: impl Fn(&Hit) -> bool) -> Option<&Hit> {
        self.under(local)
            .find(|hit| wanted(hit) || hit.blocks)
            .filter(|hit| wanted(hit))
    }
}

#[cfg(test)]
#[path = "panel_test.rs"]
mod tests;
