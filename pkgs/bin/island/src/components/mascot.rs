//! A soft character showing a mood: a squircle of jelly that blinks, looks about, hops and wobbles.
//! [`Body`] moves it, one per island; [`mascot`] draws a [`Pose`] of it at any size.

use std::{
    f32::consts::TAU,
    time::{Duration, Instant},
};

use gpui::{
    Hsla, IntoElement, PathBuilder, PathStyle, Pixels, Point, Rgba, SpringConfig, StrokeOptions,
    Window, canvas, div, point, prelude::*, px, rgb,
};

use crate::{
    spring::Spring,
    theme::{CRUST, OVERLAY1, PEACH, PINK, SAPPHIRE, YELLOW},
};

const SQUASH: SpringConfig = SpringConfig::new(320.0, 13.0, 1.0);
const LOOK: SpringConfig = SpringConfig::new(140.0, 18.0, 1.0);
const SOFT: SpringConfig = SpringConfig::new(160.0, 20.0, 1.0);
const LID: SpringConfig = SpringConfig::new(60.0, 14.0, 1.0);
const POP: SpringConfig = SpringConfig::new(220.0, 16.0, 1.0);
/// Each wobble of its edge, by how many bulges it has; lightly damped, so a knock rings.
const WOBBLES: [(f32, SpringConfig); 4] = [
    (2.0, SpringConfig::new(130.0, 3.2, 1.0)),
    (3.0, SpringConfig::new(160.0, 3.2, 1.0)),
    (4.0, SpringConfig::new(190.0, 3.2, 1.0)),
    (5.0, SpringConfig::new(220.0, 3.2, 1.0)),
];
const EPSILON: f32 = 0.002;
/// In body heights per second squared.
const GRAVITY: f32 = 9.0;
const BLINK: Duration = Duration::from_millis(160);
const CHEER: Duration = Duration::from_millis(1800);
const POKE: Duration = Duration::from_millis(800);
/// Between pressing down and leaving the ground.
const CROUCH: Duration = Duration::from_millis(70);
/// Below this side its eyes are too small for a glint.
const GLINT_FROM: f32 = 44.0;
/// How much of its square's width the body takes at rest, and how much wider than tall it is.
const FILL: f32 = 0.82;
const WIDE: f32 = 1.4;
/// The most a squash spreads it, and its wobbling edge pushes out; with the hover grow, the
/// widest it gets still fits its square.
const MOST_SPREAD: f32 = 1.1;
const MOST_STRETCH: f32 = 1.16;
const MOST_WOBBLE: f32 = 1.06;
const NEAR_GROW: f32 = 1.04;
/// The dark ring round its state badge.
const BADGE_RING: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Rest,
    Busy,
    /// Waiting on you.
    Asking,
    Asleep,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    pub activity: Activity,
    /// From 0 to 1, such as how much of a limit is gone; its eyelids grow heavy.
    pub tired: f32,
    pub color: u32,
    /// While it cheers.
    pub cheer_color: u32,
    /// The pointer is near, so it looks up at you.
    pub near: bool,
    /// Something it waits on is loading: a spinner by its head, which it watches.
    pub loading: bool,
}

/// What floats by its head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extra {
    None,
    /// A "!" bobbing at `phase`.
    Asking(f32),
    /// Dots typing at `phase`.
    Busy(f32),
    /// z's rising at `phase`.
    Asleep(f32),
    /// A spinner turning at `phase`.
    Loading(f32),
}

/// One still of it, to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    color: [f32; 3],
    squash: f32,
    /// Off the ground, in body heights.
    lift: f32,
    grow: f32,
    look: (f32, f32),
    open: f32,
    lid: f32,
    happy: f32,
    alert: f32,
    cheeks: f32,
    wobble: [(f32, f32, f32); 4],
    asleep: bool,
    extra: Extra,
}

fn channels(color: u32) -> [f32; 3] {
    let rgba = rgb(color);
    [rgba.r, rgba.g, rgba.b]
}

/// Moves one character: give it a [`Mood`] as things change, step it each frame, and draw its
/// [`Body::pose`]. Calm, it bobs while busy or asking and rests between blinks otherwise, so it
/// costs no frames while nothing happens; lively, it breathes and bounces all the time.
pub struct Body {
    lively: bool,
    mood: Mood,
    color: [f32; 3],
    squash: Spring,
    grow: Spring,
    look_x: Spring,
    look_y: Spring,
    lid: Spring,
    happy: Spring,
    alert: Spring,
    cheeks: Spring,
    /// Each wobble's bulges, size and phase.
    wobble: [(f32, Spring, f32); 4],
    lift: f32,
    rise: f32,
    jump_at: Option<(Instant, f32)>,
    blink_at: Option<Instant>,
    next_blink: Instant,
    next_glance: Instant,
    next_beat: Instant,
    cheer_until: Option<Instant>,
    /// Seconds it has moved, for what cycles.
    clock: f32,
    seed: u32,
}

impl Body {
    /// Bodies with different `seed`s blink and look about at different times, so several shown
    /// at once do not move in step.
    pub fn new(lively: bool, mood: Mood, now: Instant, seed: u64) -> Self {
        let color = channels(mood.color);
        let mut body = Self {
            lively,
            mood,
            color,
            squash: Spring::new(1.0, SQUASH, EPSILON),
            grow: Spring::new(1.0, SOFT, EPSILON),
            look_x: Spring::new(0.0, LOOK, EPSILON),
            look_y: Spring::new(0.0, LOOK, EPSILON),
            lid: Spring::new(0.0, LID, EPSILON),
            happy: Spring::new(0.0, SOFT, EPSILON),
            alert: Spring::new(0.0, POP, EPSILON),
            cheeks: Spring::new(0.0, SOFT, EPSILON),
            wobble: WOBBLES.map(|(bulges, config)| (bulges, Spring::new(0.0, config, EPSILON), 0.0)),
            lift: 0.0,
            rise: 0.0,
            jump_at: None,
            blink_at: None,
            next_blink: now + Duration::from_secs(2),
            next_glance: now + Duration::from_secs(1),
            next_beat: now,
            cheer_until: None,
            clock: 0.0,
            // Xorshift never leaves zero, so no seed may give it.
            seed: (0x9e37_79b9 ^ (seed as u32).wrapping_mul(0x85eb_ca6b)).max(1),
        };
        body.next_blink = body.after(now, 1.5, 3.0);
        body.next_glance = body.after(now, 0.8, 1.6);
        body.aim(now);
        body
    }

    fn random(&mut self, low: f32, high: f32) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        low + (self.seed as f32 / u32::MAX as f32) * (high - low)
    }

    fn after(&mut self, now: Instant, low: f32, high: f32) -> Instant {
        now + Duration::from_secs_f32(self.random(low, high))
    }

    fn cheering(&self, now: Instant) -> bool {
        self.cheer_until.is_some_and(|until| now < until)
    }

    fn asleep(&self) -> bool {
        self.mood.activity == Activity::Asleep
    }

    /// Lively, it moves all the time; calm, only while busy, asking or loading.
    fn moving(&self) -> bool {
        self.lively
            || self.mood.loading
            || matches!(self.mood.activity, Activity::Busy | Activity::Asking)
    }

    /// Points each part where the mood says.
    fn aim(&mut self, now: Instant) {
        let tired = self.mood.tired.clamp(0.0, 1.0);
        let asleep = self.asleep();
        let asking = self.mood.activity == Activity::Asking && !asleep;
        let happy = if self.cheering(now) && !asleep { 1.0 } else { 0.0 };
        self.lid.retarget(
            if asleep {
                1.0
            } else if asking {
                tired * 0.3
            } else {
                tired * 0.55
            },
            LID,
        );
        self.alert.retarget(if asking { 1.0 } else { 0.0 }, POP);
        self.happy.retarget(happy, SOFT);
        self.cheeks
            .retarget(happy.max(if self.mood.near { 0.6 } else { 0.0 }), SOFT);
        self.grow
            .retarget(if self.mood.near { NEAR_GROW } else { 1.0 }, SOFT);
        let look = if self.mood.near {
            Some((0.0, -0.25))
        } else if asleep {
            Some((0.0, 0.3))
        } else {
            match self.mood.activity {
                Activity::Busy => Some((self.look_x.target(), 0.5)),
                Activity::Asking => Some((0.0, -0.15)),
                // Up at the spinner by its head.
                _ if self.mood.loading => Some((-0.6, -0.4)),
                // Resting, it glances about on its own.
                _ => None,
            }
        };
        if let Some((x, y)) = look {
            self.look_x.retarget(x, LOOK);
            self.look_y.retarget(y, LOOK);
        }
    }

    pub fn mood(&self) -> Mood {
        self.mood
    }

    pub fn set(&mut self, mood: Mood, now: Instant) {
        if mood == self.mood {
            return;
        }
        let was = self.mood.activity;
        self.mood = mood;
        if mood.activity != was {
            if mood.activity == Activity::Asking {
                self.jump(1.7, now);
            } else if was == Activity::Asleep || mood.activity == Activity::Busy {
                self.jump(1.1, now);
            }
            self.next_beat = now + Duration::from_millis(600);
        }
        self.aim(now);
    }

    /// A hop of joy, such as when a task is done.
    pub fn cheer(&mut self, now: Instant) {
        self.cheer_until = Some(now + CHEER);
        self.jump(2.6, now);
        self.aim(now);
    }

    /// Clicked: it wobbles and hops.
    pub fn poke(&mut self, now: Instant) {
        self.jiggle(1.4);
        if !self.asleep() {
            self.cheer_until = Some(now + POKE);
        }
        self.jump(if self.asleep() { 1.5 } else { 2.1 }, now);
        self.aim(now);
    }

    fn jiggle(&mut self, amount: f32) {
        for index in 0..self.wobble.len() {
            let push = self.random(-1.0, 1.0) * amount / self.wobble[index].0.sqrt();
            self.wobble[index].1.kick(push);
        }
    }

    fn jump(&mut self, speed: f32, now: Instant) {
        if self.lift > 0.0 || self.jump_at.is_some() {
            return;
        }
        self.squash.kick(-3.2);
        self.jump_at = Some((now + CROUCH, speed));
    }

    /// Starts what its timers say is due: a blink, a glance, a hop, the end of a cheer.
    pub fn wake(&mut self, now: Instant) {
        if self.blink_at.is_some_and(|at| now >= at + BLINK) {
            self.blink_at = None;
        }
        if now >= self.next_blink {
            self.blink_at = Some(now);
            self.next_blink = self.after(now, 2.0, 5.5);
        }
        if self.mood.activity == Activity::Rest
            && !self.mood.near
            && !self.mood.loading
            && now >= self.next_glance
        {
            let (x, y) = (self.random(-0.7, 0.7), self.random(-0.4, 0.3));
            self.look_x.retarget(x, LOOK);
            self.look_y.retarget(y, LOOK);
            self.next_glance = self.after(now, 1.2, 3.5);
        }
        if self.cheer_until.is_some_and(|until| now >= until) {
            self.cheer_until = None;
            self.aim(now);
        }
        if self.moving() && !self.asleep() && now >= self.next_beat {
            let tired = self.mood.tired.clamp(0.0, 1.0);
            // Calm, it keeps a slower, smaller beat.
            let (pace, height) = if self.lively { (1.0, 1.0) } else { (1.5, 0.8) };
            match self.mood.activity {
                Activity::Busy => {
                    self.jump(1.1 * height * (1.0 - tired * 0.6), now);
                    self.next_beat = now + Duration::from_secs_f32((0.62 + tired * 0.5) * pace);
                }
                Activity::Asking => {
                    self.jump(1.6 * height, now);
                    self.next_beat = now + Duration::from_secs_f32(1.3 * pace);
                }
                _ => self.next_beat = now + Duration::from_millis(200),
            }
        }
        if let Some((at, speed)) = self.jump_at
            && now >= at
        {
            self.jump_at = None;
            self.rise = speed;
            self.lift = f32::EPSILON;
            self.jiggle(0.5);
        }
    }

    pub fn step(&mut self, dt: f32, now: Instant) {
        self.wake(now);
        self.clock += dt;
        if self.mood.activity == Activity::Busy && !self.mood.near {
            self.look_x
                .retarget((self.clock * 0.8).fract() * 1.6 - 0.8, LOOK);
        }
        if self.lift > 0.0 {
            self.rise -= GRAVITY * dt;
            self.lift += self.rise * dt;
            if self.lift <= 0.0 {
                if self.rise < -0.6 {
                    self.squash.kick(self.rise * 2.4);
                    self.jiggle(-self.rise * 0.6);
                }
                self.lift = 0.0;
                self.rise = 0.0;
            }
            self.squash
                .retarget(1.0 + (self.rise.abs() * 0.05).min(0.16), SQUASH);
        } else if self.lively {
            let (depth, pace) = if self.asleep() { (0.05, 1.3) } else { (0.03, 2.2) };
            self.squash
                .retarget(1.0 + depth * (self.clock * pace).sin(), SQUASH);
        } else {
            self.squash.retarget(1.0, SQUASH);
        }
        let target = channels(if self.cheering(now) && !self.asleep() {
            self.mood.cheer_color
        } else {
            self.mood.color
        });
        let ease = 1.0 - (-dt * 6.0).exp();
        for (channel, goal) in self.color.iter_mut().zip(target) {
            *channel += (goal - *channel) * ease;
            if (goal - *channel).abs() < 0.002 {
                *channel = goal;
            }
        }
        for spring in [
            &mut self.squash,
            &mut self.grow,
            &mut self.look_x,
            &mut self.look_y,
            &mut self.lid,
            &mut self.happy,
            &mut self.alert,
            &mut self.cheeks,
        ] {
            spring.advance(dt, now);
        }
        for (bulges, size, phase) in &mut self.wobble {
            size.advance(dt, now);
            *phase = (*phase + dt * (1.2 + *bulges * 0.4)) % TAU;
        }
    }

    pub fn is_settled(&self) -> bool {
        !self.moving()
            && self.lift == 0.0
            && self.jump_at.is_none()
            && self.blink_at.is_none()
            && self.color == channels(self.mood.color)
            && [
                &self.squash,
                &self.grow,
                &self.look_x,
                &self.look_y,
                &self.lid,
                &self.happy,
                &self.alert,
                &self.cheeks,
            ]
            .iter()
            .all(|spring| spring.is_settled())
            && self.wobble.iter().all(|(_, size, _)| size.is_settled())
    }

    /// When it next has something to do while calm and still.
    pub fn due(&self) -> Option<Instant> {
        let glance = (self.mood.activity == Activity::Rest).then_some(self.next_glance);
        [Some(self.next_blink), glance, self.cheer_until, self.jump_at.map(|(at, _)| at)]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn pose(&self, now: Instant) -> Pose {
        let blink = self.blink_at.map_or(0.0, |at| {
            let progress = now.saturating_duration_since(at).as_secs_f32() / BLINK.as_secs_f32();
            if progress <= 1.0 {
                1.0 - (progress * 2.0 - 1.0).abs()
            } else {
                0.0
            }
        });
        let alert = self.alert.value();
        // At rest and calm, what floats by its head stands still.
        let phase = if self.moving() { self.clock } else { 0.3 };
        let extra = match self.mood.activity {
            Activity::Asking => Extra::Asking(phase),
            _ if self.mood.loading => Extra::Loading(phase),
            Activity::Asleep => Extra::Asleep(phase),
            Activity::Busy if self.happy.value() < 0.3 => Extra::Busy(phase),
            _ => Extra::None,
        };
        let mut wobble = [(0.0, 0.0, 0.0); 4];
        for (out, (bulges, size, phase)) in wobble.iter_mut().zip(&self.wobble) {
            *out = (*bulges, size.value(), *phase);
        }
        Pose {
            color: self.color,
            squash: self.squash.value(),
            lift: self.lift,
            grow: self.grow.value(),
            look: (self.look_x.value(), self.look_y.value()),
            open: (1.0 - blink) * (1.0 + 0.18 * alert),
            lid: self.lid.value().clamp(0.0, 1.0),
            happy: self.happy.value(),
            alert,
            cheeks: self.cheeks.value(),
            wobble,
            asleep: self.asleep(),
            extra,
        }
    }
}

fn color([r, g, b]: [f32; 3], a: f32) -> Hsla {
    Rgba { r, g, b, a }.into()
}

fn mix(from: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * t)
}

fn fill(window: &mut Window, points: &[Point<Pixels>], color: Hsla) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    let mut path = PathBuilder::fill();
    path.move_to(*first);
    for at in rest {
        path.line_to(*at);
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn stroke(window: &mut Window, points: &[Point<Pixels>], width: f32, color: Hsla) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    let width = width.max(1.0);
    let style = StrokeOptions::default()
        .with_line_width(width)
        .with_miter_limit(1.0);
    let mut path = PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(style));
    path.move_to(*first);
    for at in rest {
        path.line_to(*at);
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    (0..24)
        .map(|i| {
            let th = i as f32 / 24.0 * TAU;
            (cx + rx * th.cos(), cy + ry * th.sin())
        })
        .collect()
}

/// A curve from `a` to `b` pulled toward `c`.
fn curve(a: (f32, f32), c: (f32, f32), b: (f32, f32)) -> Vec<(f32, f32)> {
    (0..=10)
        .map(|i| {
            let t = i as f32 / 10.0;
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
            )
        })
        .collect()
}

/// A squircle of half-width `a` and half-height `b` round `(cx, cy)`, its edge pushed out by the
/// wobbles.
fn squircle(pose: &Pose, cx: f32, cy: f32, a: f32, b: f32) -> Vec<(f32, f32)> {
    (0..72)
        .map(|i| {
            let th = i as f32 / 72.0 * TAU;
            let push = 1.0
                + pose
                    .wobble
                    .iter()
                    .map(|(bulges, size, phase)| size * 0.09 * (bulges * th + phase).sin())
                    .sum::<f32>()
                    .clamp(-(MOST_WOBBLE - 1.0), MOST_WOBBLE - 1.0);
            let (c, s) = (th.cos(), th.sin());
            (
                cx + a * push * c.signum() * c.abs().sqrt(),
                cy + b * push * s.signum() * s.abs().sqrt(),
            )
        })
        .collect()
}

fn paint(pose: &Pose, x0: f32, y0: f32, side: f32, window: &mut Window) {
    let small = side < GLINT_FROM;
    let (w, h) = (side * FILL, side * FILL / WIDE);
    // At rest it sits in the middle of its square.
    let ground = y0 + (side + h) / 2.0;
    let sy = pose.squash.min(MOST_STRETCH);
    let sx = (1.0 + (1.0 - sy) * 0.9).min(MOST_SPREAD);
    let g = pose.grow;
    // Never higher than the room above it, so a hop does not leave its square.
    let hop = if small { 0.22 } else { 0.45 };
    let room = (ground - y0 - h * sy * g * MOST_WOBBLE - 1.0).max(0.0);
    let lift = (pose.lift * side * hop).min(room);
    let at = |(x, y): (f32, f32)| point(px(x0 + side / 2.0 + x * sx * g), px(ground - lift + y * sy * g));
    let place = |points: Vec<(f32, f32)>| points.into_iter().map(at).collect::<Vec<_>>();
    let ink = color(channels(CRUST), 1.0);

    let off = (lift / (side * 0.2)).clamp(0.0, 1.0);
    let shadow: Vec<_> = ellipse(
        x0 + side / 2.0,
        (ground + 1.0).min(y0 + side - 1.5),
        w * 0.42 * sx * (1.0 - off * 0.3),
        (side * 0.03).max(1.0),
    )
    .into_iter()
    .map(|(x, y)| point(px(x), px(y)))
    .collect();
    fill(window, &shadow, Hsla::black().opacity(0.32 * (1.0 - off * 0.6)));

    fill(window, &place(squircle(pose, 0.0, -h / 2.0, w / 2.0, h / 2.0)), color(pose.color, 0.92));
    // A darker core shows through the jelly.
    let core = mix(pose.color, channels(CRUST), 0.12);
    fill(window, &place(squircle(pose, 0.0, -h * 0.42, w * 0.3, h * 0.28)), color(core, 0.6));
    fill(window, &place(ellipse(-w * 0.2, -h * 0.8, w * 0.13, h * 0.07)), Hsla::white().opacity(0.2));
    fill(window, &place(ellipse(-w * 0.3, -h * 0.68, w * 0.025, w * 0.025)), Hsla::white().opacity(0.35));

    if pose.cheeks > 0.02 {
        let blush = color(channels(PINK), pose.cheeks * 0.55);
        for side in [-1.0, 1.0] {
            fill(window, &place(ellipse(side * w * 0.3, -h * 0.36, w * 0.07, w * 0.042)), blush);
        }
    }

    let (cy, dx, ew, eh) = (-h * 0.52, w * 0.18, w * 0.075, h * 0.17);
    let line = (ew * 0.6 * g).max(1.0);
    for side in [-1.0, 1.0] {
        let x = side * dx + pose.look.0 * w * 0.08;
        let y = cy + pose.look.1 * h * 0.08;
        if pose.happy > 0.5 && !pose.asleep {
            let arc = curve((x - ew, y + eh * 0.2), (x, y - eh * 0.8), (x + ew, y + eh * 0.2));
            stroke(window, &place(arc), line, ink);
        } else if pose.asleep || pose.open < 0.2 || pose.lid > 0.9 {
            let sag = if pose.asleep { 0.55 } else { 0.2 };
            let arc = curve((x - ew * 0.9, y + eh * 0.2), (x, y + eh * sag), (x + ew * 0.9, y + eh * 0.2));
            stroke(window, &place(arc), line, ink);
        } else {
            let open = eh * pose.open;
            fill(window, &place(ellipse(x, y, ew, open)), ink);
            if !small {
                let glint = ellipse(x - ew * 0.32, y - open * 0.4, ew * 0.32, ew * 0.32);
                fill(window, &place(glint), Hsla::white().opacity(0.85));
            }
            if pose.lid > 0.02 {
                let (top, bottom) = (y - open * 1.3, y - open + open * 2.0 * pose.lid);
                let lid = vec![(x - ew * 1.4, top), (x + ew * 1.4, top), (x + ew * 1.4, bottom), (x - ew * 1.4, bottom)];
                fill(window, &place(lid), color(pose.color, 1.0));
            }
        }
    }

    if !pose.asleep {
        let (mx, my, mw) = (pose.look.0 * w * 0.04, -h * 0.3, w * 0.06);
        if pose.happy > 0.3 {
            stroke(window, &place(curve((mx - mw, my), (mx, my + mw * 1.3), (mx + mw, my))), (mw * 0.28 * g).max(1.0), ink);
        } else if pose.alert > 0.3 {
            fill(window, &place(ellipse(mx, my + mw * 0.2, mw * 0.38 * pose.alert, mw * 0.48 * pose.alert)), ink);
        }
    }

    // Its state, on a dot at the body's top-left corner, kept inside the square.
    let (tint, phase) = match pose.extra {
        Extra::Asking(phase) => (YELLOW, phase),
        Extra::Busy(phase) => (PEACH, phase),
        Extra::Asleep(phase) => (OVERLAY1, phase),
        Extra::Loading(phase) => (SAPPHIRE, phase),
        Extra::None => return,
    };
    let pop = if matches!(pose.extra, Extra::Asking(_)) {
        1.0 + 0.08 * (phase * 7.0).sin()
    } else {
        1.0
    };
    let r = (side * 0.13).max(4.5) * pop;
    let edge = r + BADGE_RING + 0.5;
    let top = ground - lift - h * sy * g;
    let x = (x0 + side / 2.0 - w * sx * g / 2.0 + r * 0.55).clamp(x0 + edge, x0 + side - edge);
    let y = (top + r * 0.45).clamp(y0 + edge, y0 + side - edge);
    let world = |points: Vec<(f32, f32)>| points.into_iter().map(|(x, y)| point(px(x), px(y))).collect::<Vec<_>>();
    fill(window, &world(ellipse(x, y, r + BADGE_RING, r + BADGE_RING)), ink);
    fill(window, &world(ellipse(x, y, r, r)), color(channels(tint), 1.0));
    match pose.extra {
        Extra::Asking(_) => {
            stroke(window, &world(vec![(x, y - r * 0.5), (x, y + r * 0.08)]), r * 0.26, ink);
            fill(window, &world(ellipse(x, y + r * 0.45, r * 0.14, r * 0.14)), ink);
        }
        Extra::Busy(_) => {
            for i in 0..3 {
                let up = (phase * 7.0 - i as f32 * 0.7).sin().max(0.0) * r * 0.22;
                let dot = ellipse(x + (i as f32 - 1.0) * r * 0.42, y - up, r * 0.13, r * 0.13);
                fill(window, &world(dot), ink);
            }
        }
        Extra::Loading(_) => {
            // Three quarters of a ring, going round once a second.
            let start = phase * TAU;
            let arc: Vec<_> = (0..=16)
                .map(|i| {
                    let th = start + i as f32 / 16.0 * TAU * 0.75;
                    (x + r * 0.5 * th.cos(), y + r * 0.5 * th.sin())
                })
                .collect();
            stroke(window, &world(arc), r * 0.24, ink);
        }
        _ => {
            let z = r * 0.36;
            let path = vec![(x - z, y - z), (x + z, y - z), (x - z, y + z), (x + z, y + z)];
            stroke(window, &world(path), r * 0.2, ink);
        }
    }
}

/// `pose` drawn in a square of `side`.
pub fn mascot(pose: Pose, side: f32) -> impl IntoElement {
    div().relative().flex_none().size(px(side)).child(
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| {
                paint(&pose, f32::from(bounds.origin.x), f32::from(bounds.origin.y), side, window);
            },
        )
        .size_full(),
    )
}

#[cfg(test)]
#[path = "mascot_test.rs"]
mod tests;
