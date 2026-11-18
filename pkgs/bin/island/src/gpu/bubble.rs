//! Widgets that split off the bar while they are active, as wgpu draws them: a chain of droplets
//! of the bar's glass, each splitting off the bar's end or the droplet before it, and melting back
//! into it when its widget stops splitting off.

use std::{collections::HashMap, time::Instant};

use super::{
    panel,
    renderer::{Sprite, inside},
    widget::{Drawn, Hit},
};
use crate::{
    bar::WIDGET_H,
    config::{Side, Widget},
    math::smooth,
    merge::{MAX_DROPS, Split},
    spring::{Spring, SpringConfig},
    stage::{COLLAPSED_H, CONTENT_DRIFT},
};

/// Between a settled droplet and what it split off.
const GAP: f32 = 12.0;
/// How far a droplet pulls clear before the thread lets go, and how near it comes back before
/// the thread catches it again.
const SNAP: f32 = 7.0;
const CATCH: f32 = 4.0;
/// How deep the waist presses in where a droplet parts, as a share of half the bar's height.
const WAIST_MAX: f32 = 0.55;
/// Out: response 0.5 s with a little overshoot. Back in: 0.45 s, none.
const SPLIT: SpringConfig = SpringConfig::new(157.9, 20.1, 1.0);
const MERGE: SpringConfig = SpringConfig::new(195.0, 27.9, 1.0);
/// How fast the thread takes hold and lets go: response 0.14 s.
const NECK: SpringConfig = SpringConfig::new(2014.2, 89.8, 1.0);
/// What it holds fading in and out: response 0.35 s.
const WORDS: SpringConfig = SpringConfig::new(322.3, 35.9, 1.0);
/// Closing the gap left by a droplet melting away further in: response 0.5 s, no overshoot.
const CLOSE: SpringConfig = SpringConfig::new(157.9, 25.1, 1.0);
/// Into the island as it grows into the droplet's own panel: response 0.22 s, quicker than the
/// panel grows, so it never hangs off it.
const INTO_PANEL: SpringConfig = SpringConfig::new(815.6, 57.1, 1.0);

/// One droplet in the chain.
struct Droplet {
    id: u64,
    widget: Widget,
    /// How far along its way out it is: 0 wholly inside what it splits off, 1 settled.
    out: Spring,
    words: Spring,
    /// How firmly the waist and thread hold it: 1 joined, 0 let go.
    neck: Spring,
    attached: bool,
    /// Its widget stopped splitting off, and it is melting back in.
    leaving: bool,
    /// There is no room for it before the screen's edge: it stays in, or goes back in, until
    /// there is.
    waiting: bool,
    /// How far it still has to ease to where it parts, after what it split off melted away.
    offset: Spring,
    /// What it splits off: the bar, or the droplet with this id; `None` until first placed.
    parent: Option<Option<u64>>,
    /// Where it parted last frame.
    anchor: f32,
    /// While a panel is open, its own island: the edge nearest the island it keeps to, pushed out
    /// of the panel's way but never pulled in.
    held: Option<f32>,
    /// How wide it stands.
    natural: f32,
    rect: [f32; 4],
    split: Option<Split>,
    /// Where its widget was last drawn.
    carried_at: Option<(f32, f32)>,
}

impl Droplet {
    fn new(id: u64, widget: Widget, animate: bool) -> Self {
        let from = if animate { 0.0 } else { 1.0 };
        let mut out = Spring::new(from, SPLIT, 0.0005);
        out.retarget(1.0, SPLIT);
        let mut words = Spring::new(from, WORDS, 0.002);
        words.retarget(1.0, WORDS);
        Self {
            id,
            widget,
            out,
            words,
            neck: Spring::new(0.0, NECK, 0.002),
            attached: animate,
            leaving: false,
            waiting: false,
            offset: Spring::new(0.0, CLOSE, 0.01),
            parent: None,
            anchor: 0.0,
            held: None,
            natural: 0.0,
            rect: [0.0; 4],
            split: None,
            carried_at: None,
        }
    }

    fn leave(&mut self, how: SpringConfig) {
        if self.leaving {
            return;
        }
        self.leaving = true;
        self.out.retarget(0.0, how);
        self.words.retarget(0.0, WORDS);
    }

    fn gone(&self) -> bool {
        self.leaving && self.out.value() < 0.002 && self.out.is_settled()
    }

    /// Out, or on its way out, to stay.
    fn staying(&self) -> bool {
        !self.leaving && !self.waiting
    }

    /// Waits for room, sinking back in if it had begun to come out.
    fn wait(&mut self) {
        if self.waiting {
            return;
        }
        self.waiting = true;
        self.out.retarget(0.0, MERGE);
        self.words.retarget(0.0, WORDS);
    }

    /// There is room for it again: it splits off as if new.
    fn come_out(&mut self) {
        if !self.waiting {
            return;
        }
        self.waiting = false;
        if self.out.value() < 0.5 {
            self.attached = true;
            self.neck.reset(0.0);
        }
        self.out.retarget(1.0, SPLIT);
        self.words.retarget(1.0, WORDS);
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.out.advance(dt, now);
        self.words.advance(dt, now);
        self.neck.advance(dt, now);
        self.offset.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.out.is_settled()
            && self.words.is_settled()
            && self.neck.is_settled()
            && self.offset.is_settled()
    }
}

pub struct Bubble {
    /// In the order they split off, innermost first.
    drops: Vec<Droplet>,
    next: u64,
    /// Widgets that went into the island with their panel, to split off next to it again.
    inside: Vec<Widget>,
}

impl Bubble {
    pub fn new() -> Self {
        Self {
            drops: Vec::new(),
            next: 0,
            inside: Vec::new(),
        }
    }

    /// Gives each widget `carried` that asked for a droplet, with how wide it is, a droplet of
    /// its own, and melts away those whose widget no longer asks. A droplet whose widget's panel
    /// is `open` goes into the island with it. New ones only split off a `bar` that shows and
    /// has its shape; `hidden`, none stand at all.
    pub fn aim(
        &mut self,
        carried: &[(Widget, f32)],
        open: &[Widget],
        (bar, hidden): (bool, bool),
        animate: bool,
    ) {
        let wanted: Vec<Widget> = if hidden {
            Vec::new()
        } else {
            carried.iter().map(|(widget, _)| *widget).collect()
        };
        for drop in &mut self.drops {
            if drop.leaving {
                continue;
            }
            if !wanted.contains(&drop.widget) {
                drop.leave(MERGE);
            } else if open.contains(&drop.widget) {
                drop.leave(INTO_PANEL);
                self.inside.push(drop.widget);
            }
        }
        self.inside.retain(|widget| wanted.contains(widget));
        for widget in wanted {
            let out = self
                .drops
                .iter()
                .any(|drop| !drop.leaving && drop.widget == widget);
            if out || !bar || open.contains(&widget) || self.drops.len() >= MAX_DROPS {
                continue;
            }
            self.next += 1;
            let drop = Droplet::new(self.next, widget, animate);
            match self.inside.iter().position(|inside| *inside == widget) {
                Some(at) => {
                    self.inside.remove(at);
                    self.drops.insert(0, drop);
                }
                None => self.drops.push(drop),
            }
        }
        for drop in self.drops.iter_mut().filter(|drop| !drop.leaving) {
            if let Some((_, width)) = carried.iter().find(|(widget, _)| *widget == drop.widget) {
                drop.natural = *width;
            }
        }
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for drop in &mut self.drops {
            drop.advance(dt, now);
        }
        self.drops.retain(|drop| !drop.gone());
    }

    pub fn is_settled(&self) -> bool {
        self.drops.iter().all(Droplet::is_settled)
    }

    /// The droplets against the island `body` at its `side`, innermost first, for the island to
    /// be drawn with them. `room` is how far across the screen reaches, left and right; one with
    /// no room before its edge waits. While a panel is `open` they stand as islands of their own;
    /// they only join an island `shaped` as the bar.
    pub fn chain(
        &mut self,
        body: [f32; 4],
        side: Side,
        room: (f32, f32),
        (open, shaped): (bool, bool),
        scale: f32,
    ) -> Vec<Split> {
        let dir = match side {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let bar = match side {
            Side::Left => body[0],
            Side::Right => body[0] + body[2],
        };
        let hh = COLLAPSED_H / 2.0;
        // Each placed this frame: its id, then its outer and inner edges.
        let mut placed: Vec<(u64, f32, f32)> = Vec::new();
        let mut chain = Vec::new();
        for index in 0..self.drops.len() {
            let drop = &self.drops[index];
            // One on its way back keeps melting into what it split off, while that is still
            // there; any other splits off the nearest staying droplet further in, or the bar.
            let back = !drop.staying();
            let kept = back
                && match drop.parent {
                    Some(None) => true,
                    Some(Some(id)) => placed.iter().any(|(placed, ..)| *placed == id),
                    None => false,
                };
            let parent = if kept {
                drop.parent.flatten()
            } else {
                self.drops[..index]
                    .iter()
                    .rev()
                    .find(|other| {
                        (other.staying() || back)
                            && placed.iter().any(|(placed, ..)| *placed == other.id)
                    })
                    .map(|other| other.id)
            };
            let found = parent.and_then(|id| placed.iter().find(|(placed, ..)| *placed == id));
            let edge = found.map_or(bar, |(_, outer, _)| *outer);
            let cut = found.map(|(_, _, inner)| *inner);
            let drop = &mut self.drops[index];
            if drop.held.is_none() && drop.parent.is_some_and(|was| was != parent) {
                drop.offset.reset(dir * (drop.anchor - edge));
                drop.offset.retarget(0.0, CLOSE);
            }
            drop.parent = Some(parent);
            if drop.natural <= 0.0 {
                drop.rect = [0.0; 4];
                drop.split = None;
                continue;
            }
            let width = drop.natural;
            let half = width / 2.0;
            // Where its edge nearest the island would stand while it keeps to itself: where it
            // is, unless what it split off now reaches past that.
            let need = edge + dir * GAP;
            let near = if dir < 0.0 {
                drop.rect[0] + drop.rect[2]
            } else {
                drop.rect[0]
            };
            let kept = drop
                .held
                .unwrap_or(if drop.rect[2] > 0.0 { near } else { need });
            let kept = if dir * (need - kept) > 0.0 {
                need
            } else {
                kept
            };
            let alone = open && drop.staying();
            if !drop.leaving {
                let reach = if alone { kept } else { need } + dir * width;
                if reach - GAP < room.0 || reach + GAP > room.1 {
                    drop.wait();
                } else {
                    drop.come_out();
                }
            }
            if drop.waiting && drop.out.value() < 0.002 {
                drop.held = None;
                drop.rect = [0.0; 4];
                drop.split = None;
                continue;
            }
            let (left, anchor, waist, thread) = if open && drop.staying() {
                drop.held = Some(kept);
                drop.attached = false;
                if drop.neck.target() != 0.0 {
                    drop.neck.retarget(0.0, NECK);
                }
                let middle = kept + dir * half;
                let left = ((middle - half) * scale).round() / scale;
                (left, kept - dir * GAP, 0.0, 0.0)
            } else {
                if let Some(kept) = drop.held.take() {
                    drop.offset.reset(dir * ((kept - dir * GAP) - edge));
                    drop.offset.retarget(0.0, CLOSE);
                }
                let anchor = edge + dir * drop.offset.value();
                let along = drop.out.value().max(0.0);
                let inside = anchor - dir * half;
                let outside = anchor + dir * (GAP + half);
                let middle = inside + (outside - inside) * along;
                let left = ((middle - half) * scale).round() / scale;
                let apart = dir * ((middle - dir * half) - anchor);
                if drop.attached && drop.staying() && apart > SNAP {
                    drop.attached = false;
                } else if !drop.attached && !drop.staying() && apart < CATCH {
                    drop.attached = true;
                }
                let neck = if drop.attached { 1.0 } else { 0.0 };
                if drop.neck.target() != neck {
                    drop.neck.retarget(neck, NECK);
                }
                // Nothing pinches or threads into the island while it is not the bar.
                let joins = if found.is_some() || shaped { 1.0 } else { 0.0 };
                let held = joins * drop.neck.value().clamp(0.0, 1.0);
                let coming = smooth(-2.0 * half, 0.0, apart);
                let parting = smooth(0.0, SNAP, apart);
                (
                    left,
                    anchor,
                    held * WAIST_MAX * hh * coming * (1.0 - parting),
                    held * (1.0 - WAIST_MAX) * hh * (1.0 - parting),
                )
            };
            drop.anchor = anchor;
            drop.rect = [left, body[1], width, COLLAPSED_H];
            let split = Split {
                x: left,
                width,
                waist,
                thread,
                anchor,
                cut,
            };
            drop.split = Some(split);
            chain.push(split);
            let (outer, inner) = match side {
                Side::Left => (left, left + width),
                Side::Right => (left + width, left),
            };
            placed.push((drop.id, outer, inner));
        }
        chain
    }

    /// Nothing of it is drawn this frame.
    pub fn unplace(&mut self) {
        for drop in &mut self.drops {
            drop.rect = [0.0; 4];
            drop.split = None;
        }
    }

    /// Where it takes the pointer: each droplet out.
    pub fn rects(&self) -> Vec<[f32; 4]> {
        self.out_rects().collect()
    }

    /// Where its glass is, for the screen behind to blur.
    pub fn glass_rects(&self) -> Vec<[f32; 4]> {
        self.out_rects()
            .filter(|rect| rect[2] >= 1.0 && rect[3] >= 0.5)
            .collect()
    }

    fn out_rects(&self) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.drops
            .iter()
            .filter(|drop| drop.split.is_some())
            .map(|drop| drop.rect)
    }

    fn under(&self, at: (f32, f32)) -> Option<&Droplet> {
        self.drops
            .iter()
            .find(|drop| drop.split.is_some() && inside(drop.rect, at))
    }

    pub fn covers(&self, at: (f32, f32)) -> bool {
        self.under(at).is_some()
    }

    /// The innermost part under `at` of a widget out in a droplet, as `carried` drew it, and
    /// where it stands; all of the droplet round it answers as the widget itself.
    pub fn carried_hit<'a>(
        &self,
        carried: &'a HashMap<Widget, (Drawn, bool)>,
        at: (f32, f32),
    ) -> Option<(&'a Hit, [f32; 4])> {
        let drop = self.under(at).filter(|drop| !drop.leaving)?;
        let (x, y) = drop.carried_at?;
        let (drawn, _) = carried.get(&drop.widget)?;
        drawn
            .hits
            .iter()
            .rev()
            .find_map(|hit| {
                let rect = [x + hit.rect[0], y, hit.rect[2], WIDGET_H];
                inside(rect, at).then_some((hit, rect))
            })
            .or_else(|| drawn.hits.first().map(|hit| (hit, drop.rect)))
    }

    /// Each droplet's widget, as `carried` drew it, where [`Self::chain`] placed the droplet in
    /// the bar's glass.
    pub fn sprites(
        &mut self,
        carried: &HashMap<Widget, (Drawn, bool)>,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        for drop in &mut self.drops {
            drop.carried_at = None;
            if drop.split.is_none() {
                continue;
            }
            let Some((drawn, _)) = carried.get(&drop.widget) else {
                continue;
            };
            let [start, top, width, height] = drop.rect;
            let shown = drop.words.value().clamp(0.0, 1.0) * smooth(0.7, 0.98, drop.out.value());
            let at = (
                ((start + (width - drawn.width) / 2.0) * scale).round() / scale,
                top + height / 2.0 - WIDGET_H / 2.0,
            );
            let first = out.len();
            out.extend(drawn.sprites.iter().copied());
            for morph in &drawn.morphs {
                panel::morph_sprites(morph, out);
            }
            let clip = [start, top, start + width, top + height];
            for sprite in &mut out[first..] {
                *sprite = sprite
                    .moved(at.0, at.1 + (1.0 - shown) * CONTENT_DRIFT / 2.0)
                    .faded(shown)
                    .within(clip);
            }
            drop.carried_at = Some(at);
        }
    }
}
