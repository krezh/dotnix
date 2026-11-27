//! A soft character showing a mood: a squircle of jelly that blinks, looks about, hops and wobbles.
//! [`Body`] moves it, one per island; [`mascot`] draws a [`Pose`] of it at any size.

use std::{
    f32::consts::{PI, TAU},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    drawing::{Color, Drawing, Shape},
    math::{self, rgb, smooth},
    panel::Mark,
    spring::Spring,
    spring::SpringConfig,
    theme::{CRITICAL, CRUST, GREEN, LAVENDER, OVERLAY1, OVERLAY2, PEACH, PINK, SAPPHIRE, YELLOW},
};

const SQUASH: SpringConfig = SpringConfig::new(320.0, 13.0, 1.0);
const LOOK: SpringConfig = SpringConfig::new(140.0, 18.0, 1.0);
const SOFT: SpringConfig = SpringConfig::new(160.0, 20.0, 1.0);
const LID: SpringConfig = SpringConfig::new(60.0, 14.0, 1.0);
const POP: SpringConfig = SpringConfig::new(220.0, 16.0, 1.0);
const DOZE: SpringConfig = SpringConfig::new(12.0, 7.0, 1.0);
const TILT: SpringConfig = SpringConfig::new(110.0, 8.0, 1.0);
const FACE: SpringConfig = SpringConfig::new(240.0, 9.0, 1.0);
const YAWN: SpringConfig = SpringConfig::new(40.0, 10.0, 1.0);
/// Each wobble of its edge, by how many bulges it has; lightly damped, so a knock rings.
const WOBBLES: [(f32, SpringConfig); 4] = [
    (2.0, SpringConfig::new(130.0, 3.2, 1.0)),
    (3.0, SpringConfig::new(160.0, 3.2, 1.0)),
    (4.0, SpringConfig::new(190.0, 3.2, 1.0)),
    (5.0, SpringConfig::new(220.0, 3.2, 1.0)),
];
const EPSILON: f32 = 0.002;
/// Springs whose motion is slower than this, per second, are only drifting: breathing, a sweep of
/// its eyes, a fading wobble. A landing or a glance moves far faster.
const DRIFTING: f32 = 0.5;
/// A colour this near where it is fading to is only drifting.
const COLOR_DRIFTING: f32 = 0.03;
/// In body heights per second squared.
const GRAVITY: f32 = 9.0;
const BLINK: Duration = Duration::from_millis(160);
const CHEER: Duration = Duration::from_millis(1800);
const POKE: Duration = Duration::from_millis(800);
/// Between pressing down and leaving the ground.
const CROUCH: Duration = Duration::from_millis(70);
const ENCORE: Duration = Duration::from_millis(900);
const DOZE_AFTER: Duration = Duration::from_secs(45);
const YAWNING: Duration = Duration::from_millis(1600);
const GRUMPY: Duration = Duration::from_secs(2);
const POKES: usize = 4;
const PATIENCE: Duration = Duration::from_millis(2200);
const BLINK_BEFORE: f32 = 0.7;
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
    /// Something it depends on has gone wrong, such as data that failed to load: it frowns, with a
    /// red "!" by its head.
    pub warning: bool,
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
    /// A still red "!".
    Warning,
}

impl Extra {
    fn at(self, phase: f32) -> Self {
        match self {
            Self::None => Self::None,
            Self::Asking(_) => Self::Asking(phase),
            Self::Busy(_) => Self::Busy(phase),
            Self::Asleep(_) => Self::Asleep(phase),
            Self::Loading(_) => Self::Loading(phase),
            Self::Warning => Self::Warning,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feeling {
    Happy,
    Asking,
    Tired,
    Asleep,
}

impl Feeling {
    const ALL: [Self; 4] = [Self::Happy, Self::Asking, Self::Tired, Self::Asleep];

    pub fn name(self) -> &'static str {
        match self {
            Self::Happy => "happy",
            Self::Asking => "asking",
            Self::Tired => "tired",
            Self::Asleep => "asleep",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|feeling| feeling.name() == name)
    }
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
    worry: f32,
    wobble: [(f32, f32, f32); 4],
    extra: Extra,
    /// How far its badge has grown in, from 0 to 1.
    badge: f32,
    tilt: f32,
    face: (f32, f32),
    yawn: f32,
    puff: f32,
    grumpy: bool,
    near: bool,
    tired: f32,
}

#[derive(Clone, Copy, Debug)]
struct Blink {
    at: Instant,
    length: f32,
    depth: f32,
}

impl Blink {
    fn end(&self) -> Instant {
        self.at + Duration::from_secs_f32(self.length)
    }
}

fn before(until: Option<Instant>, now: Instant) -> bool {
    until.is_some_and(|until| now < until)
}

fn expire(until: &mut Option<Instant>, now: Instant) -> bool {
    let due = until.is_some_and(|until| now >= until);
    if due {
        *until = None;
    }
    due
}

/// Moves one character: give it a [`Mood`] as things change, step it each frame, and draw its
/// [`Body::pose`]. Calm, it bobs while busy or asking and otherwise rests between blinks, glances
/// and idle habits, so it costs no frames while nothing happens; lively, it breathes and bounces
/// all the time.
pub struct Body {
    lively: bool,
    mood: Mood,
    color: [f32; 3],
    /// The colour it is fading to.
    goal: [f32; 3],
    squash: Spring,
    grow: Spring,
    look_x: Spring,
    look_y: Spring,
    lid: Spring,
    happy: Spring,
    alert: Spring,
    cheeks: Spring,
    worry: Spring,
    badge: Spring,
    /// The badge showing, or last shown while it shrinks away.
    shown: Extra,
    /// Seconds its badge has moved; it stops where it is when it stops moving.
    badge_clock: f32,
    /// Each wobble's bulges, size and phase.
    wobble: [(f32, Spring, f32); 4],
    tilt: Spring,
    face_y: Spring,
    yawn: Spring,
    puff: Spring,
    lift: f32,
    rise: f32,
    jump_at: Option<(Instant, f32)>,
    blink: Option<Blink>,
    next_too: Option<Blink>,
    next_blink: Instant,
    next_glance: Instant,
    glance_at: Option<(Instant, f32, f32)>,
    hold_look: Option<Instant>,
    look_back: Option<(f32, f32)>,
    next_beat: Instant,
    cheer_until: Option<Instant>,
    encore: u32,
    encore_side: f32,
    resting_since: Instant,
    next_habit: Instant,
    dozing: bool,
    yawn_until: Option<Instant>,
    pokes: Vec<Instant>,
    grumpy_until: Option<Instant>,
    pointer: Option<(f32, f32)>,
    breath: f32,
    /// Seconds it has moved, for what cycles.
    clock: f32,
    seed: u32,
}

impl Body {
    /// Bodies with different `seed`s blink and look about at different times, so several shown
    /// at once do not move in step.
    pub fn new(lively: bool, mood: Mood, now: Instant, seed: u64) -> Self {
        let color = rgb(mood.color);
        let mut body = Self {
            lively,
            mood,
            color,
            goal: color,
            squash: Spring::new(1.0, SQUASH, EPSILON),
            grow: Spring::new(1.0, SOFT, EPSILON),
            look_x: Spring::new(0.0, LOOK, EPSILON),
            look_y: Spring::new(0.0, LOOK, EPSILON),
            lid: Spring::new(0.0, LID, EPSILON),
            happy: Spring::new(0.0, SOFT, EPSILON),
            alert: Spring::new(0.0, POP, EPSILON),
            cheeks: Spring::new(0.0, SOFT, EPSILON),
            worry: Spring::new(0.0, SOFT, EPSILON),
            badge: Spring::new(0.0, POP, EPSILON),
            shown: Extra::None,
            badge_clock: 0.3,
            wobble: WOBBLES
                .map(|(bulges, config)| (bulges, Spring::new(0.0, config, EPSILON), 0.0)),
            tilt: Spring::new(0.0, TILT, EPSILON),
            face_y: Spring::new(0.0, FACE, EPSILON),
            yawn: Spring::new(0.0, YAWN, EPSILON),
            puff: Spring::new(0.0, SOFT, EPSILON),
            lift: 0.0,
            rise: 0.0,
            jump_at: None,
            blink: None,
            next_too: None,
            next_blink: now + Duration::from_secs(2),
            next_glance: now + Duration::from_secs(1),
            glance_at: None,
            hold_look: None,
            look_back: None,
            next_beat: now,
            cheer_until: None,
            encore: 0,
            encore_side: 1.0,
            resting_since: now,
            next_habit: now,
            dozing: false,
            yawn_until: None,
            pokes: Vec::new(),
            grumpy_until: None,
            pointer: None,
            breath: 1.0,
            clock: 0.0,
            // Xorshift never leaves zero, so no seed may give it.
            seed: (0x9e37_79b9 ^ (seed as u32).wrapping_mul(0x85eb_ca6b)).max(1),
        };
        body.next_blink = body.after(now, 1.5, 3.0);
        body.next_glance = body.after(now, 0.8, 1.6);
        body.next_habit = body.after(now, 8.0, 16.0);
        body.aim(now);
        body.shown = body.wanted();
        if body.shown != Extra::None {
            body.badge.reset(1.0);
        }
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

    fn either(&mut self, amount: f32) -> f32 {
        if self.random(0.0, 1.0) < 0.5 {
            -amount
        } else {
            amount
        }
    }

    fn lean(&mut self, most: f32) {
        let lean = self.random(-most, most);
        self.tilt.kick(lean);
    }

    fn cheering(&self, now: Instant) -> bool {
        before(self.cheer_until, now)
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

    fn glancing(&self) -> bool {
        self.mood.activity == Activity::Rest && !self.mood.near && !self.mood.loading
    }

    fn idle(&self) -> bool {
        self.glancing() && !self.dozing
    }

    fn grumpy(&self, now: Instant) -> bool {
        before(self.grumpy_until, now)
    }

    fn tracking(&self) -> bool {
        self.pointer.is_some() && self.mood.near && !self.asleep() && !self.dozing
    }

    fn look_at(&mut self, (x, y): (f32, f32)) {
        self.look_x.retarget(x, LOOK);
        self.look_y.retarget(y, LOOK);
    }

    fn follow(&mut self) {
        if let Some((x, y)) = self.pointer {
            self.look_at(((x * 0.8).tanh() * 0.9, (y * 0.8).tanh() * 0.6));
        }
    }

    /// What the mood says should float by its head.
    fn wanted(&self) -> Extra {
        let phase = self.badge_clock;
        match self.mood.activity {
            Activity::Asking => Extra::Asking(phase),
            _ if self.mood.loading => Extra::Loading(phase),
            _ if self.mood.warning => Extra::Warning,
            Activity::Asleep => Extra::Asleep(phase),
            // Cheering, it stops typing.
            Activity::Busy if self.happy.target() < 0.5 => Extra::Busy(phase),
            _ => Extra::None,
        }
    }

    /// Points each part where the mood says.
    fn aim(&mut self, now: Instant) {
        let tired = self.mood.tired.clamp(0.0, 1.0);
        let asleep = self.asleep();
        let asking = self.mood.activity == Activity::Asking && !asleep;
        let grumpy = self.grumpy(now);
        let happy = if self.cheering(now) && !asleep && !grumpy {
            1.0
        } else {
            0.0
        };
        let (lid, pace) = if self.dozing {
            (0.82, DOZE)
        } else if asleep {
            (1.0, LID)
        } else if asking {
            (tired * 0.3, LID)
        } else {
            (tired * 0.55, LID)
        };
        self.lid
            .retarget(if grumpy { lid.max(0.5) } else { lid }, pace);
        self.alert.retarget(if asking { 1.0 } else { 0.0 }, POP);
        self.happy.retarget(happy, SOFT);
        let near = if self.mood.near { 0.6 } else { 0.0 };
        let puffed = if grumpy { 1.0 } else { 0.0 };
        self.cheeks.retarget(happy.max(near).max(puffed), SOFT);
        self.puff.retarget(puffed, SOFT);
        // Asking, cheering or asleep, it forgets its worry for a while.
        let worried = self.mood.warning && !asking && !asleep && happy == 0.0;
        self.worry.retarget(if worried { 1.0 } else { 0.0 }, SOFT);
        self.grow
            .retarget(if self.mood.near { NEAR_GROW } else { 1.0 }, SOFT);
        if before(self.hold_look, now) {
            return;
        }
        if self.tracking() {
            self.follow();
            return;
        }
        let look = if self.mood.near {
            Some((0.0, -0.25))
        } else if asleep || self.dozing {
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
        if let Some(look) = look {
            self.look_at(look);
        }
    }

    pub fn mood(&self) -> Mood {
        self.mood
    }

    pub fn set_lively(&mut self, lively: bool) {
        self.lively = lively;
    }

    pub fn set(&mut self, mood: Mood, now: Instant) {
        if mood == self.mood {
            return;
        }
        let was = self.mood;
        let was_idle = self.idle();
        self.mood = mood;
        if self.dozing && (mood.activity != was.activity || mood.near || mood.loading) {
            self.startle(now);
        }
        if mood.activity != was.activity {
            if was.activity == Activity::Asleep && !self.asleep() {
                self.startle(now);
            }
            if mood.activity == Activity::Asking {
                self.jump(1.7, now);
                self.look_at((-0.85, -0.6));
                self.hold_look = Some(now + Duration::from_millis(500));
            } else if was.activity == Activity::Asleep || mood.activity == Activity::Busy {
                self.jump(1.1, now);
            }
            self.next_beat = now + Duration::from_millis(600);
        }
        if mood.warning && !was.warning {
            self.flinch(now);
        }
        if self.idle() && !was_idle {
            self.resting_since = now;
            self.next_habit = self.after(now, 8.0, 16.0);
        }
        self.aim(now);
    }

    pub fn point(&mut self, at: Option<(f32, f32)>, now: Instant) {
        if at == self.pointer {
            return;
        }
        let was = self.tracking();
        self.pointer = at;
        if self.tracking() {
            self.follow();
        } else if was {
            self.aim(now);
        }
    }

    /// A hop of joy, such as when a task is done, and one more for each other done at once.
    pub fn cheer(&mut self, now: Instant, count: u32) {
        let more = count.saturating_sub(1);
        self.cheer_until = Some(now + CHEER + ENCORE * more);
        self.encore = more;
        self.encore_side = 1.0;
        self.jump(2.6, now);
        if more > 0 {
            self.tilt.kick(2.4);
        }
        self.aim(now);
    }

    /// Clicked: it wobbles and hops.
    pub fn poke(&mut self, now: Instant) {
        self.pokes.retain(|at| now.duration_since(*at) < PATIENCE);
        self.pokes.push(now);
        if self.pokes.len() >= POKES && !self.asleep() {
            self.grumpy_until = Some(now + GRUMPY);
            self.cheer_until = None;
            self.jiggle(1.1);
            self.squash.kick(-2.4);
            self.lean(1.0);
            self.pokes.clear();
            self.aim(now);
            return;
        }
        if self.grumpy(now) {
            self.jiggle(0.7);
            self.aim(now);
            return;
        }
        if self.dozing {
            self.startle(now);
        }
        self.jiggle(1.4);
        if !self.asleep() {
            self.cheer_until = Some(now + POKE);
        }
        self.jump(if self.asleep() { 1.5 } else { 2.1 }, now);
        self.lean(1.4);
        self.aim(now);
    }

    fn startle(&mut self, now: Instant) {
        self.dozing = false;
        self.resting_since = now;
        self.alert.kick(9.0);
        self.jiggle(1.3);
        self.squash.kick(2.5);
        self.lean(1.2);
        self.lid.reset(0.0);
    }

    fn flinch(&mut self, now: Instant) {
        self.squash.kick(-4.0);
        self.jiggle(1.1);
        self.start_blink(now, 0.22, 1.0);
        let way = self.either(1.6);
        let lean = self.random(0.5, 1.0) * way;
        self.tilt.kick(lean);
        self.face_y.kick(0.5);
    }

    fn start_blink(&mut self, now: Instant, length: f32, depth: f32) {
        self.blink = Some(Blink {
            at: now,
            length,
            depth,
        });
    }

    fn habit(&mut self, now: Instant) {
        let roll = self.random(0.0, 1.0);
        if self.mood.tired > 0.35 && roll < 0.55 {
            if self.yawn_until.is_none() {
                self.yawn.retarget(1.0, YAWN);
                self.yawn_until = Some(now + YAWNING);
            }
            return;
        }
        match self.random(0.0, 3.0) as u32 {
            0 => self.jump(0.8, now),
            1 => {
                self.jiggle(0.9);
                self.squash.kick(-1.8);
                let way = self.either(1.8);
                self.tilt.kick(way);
            }
            _ => {
                let x = self.random(-0.2, 0.2);
                self.look_at((x, 0.75));
                self.hold_look = Some(now + Duration::from_millis(1200));
                self.next_glance = now + Duration::from_millis(1400);
            }
        }
    }

    fn glance(&mut self, now: Instant) {
        let (from_x, from_y) = (self.look_x.target(), self.look_y.target());
        let away = from_x.hypot(from_y);
        let roll = self.random(0.0, 1.0);
        let (to, next) = if let Some(back) = self.look_back.take() {
            (back, self.after(now, 1.2, 3.2))
        } else if roll < 0.22 && away > 0.2 {
            ((0.0, 0.0), self.after(now, 1.5, 3.5))
        } else if roll < 0.34 && away > 0.3 {
            self.look_back = Some((from_x, from_y));
            ((0.0, 0.0), now + Duration::from_millis(280))
        } else if roll < 0.48 {
            let to = (self.random(-0.6, 0.6), self.random(-0.3, 0.2));
            (to, self.after(now, 4.5, 7.0))
        } else {
            let to = (self.random(-0.75, 0.75), self.random(-0.4, 0.3));
            (to, self.after(now, 1.2, 3.2))
        };
        if (to.0 - from_x).hypot(to.1 - from_y) > BLINK_BEFORE && self.blink.is_none() {
            self.start_blink(now, 0.14, 1.0);
            self.glance_at = Some((now + Duration::from_millis(60), to.0, to.1));
        } else {
            self.look_at(to);
        }
        self.next_glance = next;
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
        if self.blink.is_some_and(|blink| now >= blink.end()) {
            self.blink = None;
        }
        if self.blink.is_none()
            && let Some(next) = self.next_too.filter(|next| now >= next.at)
        {
            self.next_too = None;
            self.start_blink(now, next.length, next.depth);
        }
        if now >= self.next_blink {
            let tired = self.mood.tired.clamp(0.0, 1.0);
            let length = BLINK.as_secs_f32() * (1.0 + tired * 1.5);
            let roll = self.random(0.0, 1.0);
            self.start_blink(now, length, if roll < 0.12 { 0.55 } else { 1.0 });
            if roll > 0.84 {
                self.next_too = Some(Blink {
                    at: now + Duration::from_secs_f32(length + 0.07),
                    length: length * 0.9,
                    depth: 1.0,
                });
            }
            self.next_blink = self.after(now, 1.6, 6.5);
        }
        if let Some((at, x, y)) = self.glance_at
            && now >= at
        {
            self.glance_at = None;
            self.look_at((x, y));
        }
        if expire(&mut self.hold_look, now) {
            self.aim(now);
        }
        if self.idle() && self.hold_look.is_none() && now >= self.next_glance {
            self.glance(now);
        }
        if expire(&mut self.cheer_until, now) {
            self.aim(now);
        }
        if expire(&mut self.grumpy_until, now) {
            self.aim(now);
        }
        if expire(&mut self.yawn_until, now) {
            self.yawn.retarget(0.0, YAWN);
        }
        if self.idle() {
            if now >= self.resting_since + DOZE_AFTER {
                self.dozing = true;
                self.aim(now);
            } else if now >= self.next_habit {
                self.habit(now);
                self.next_habit = self.after(now, 9.0, 20.0);
            }
        }
        if self.moving() && !self.asleep() && now >= self.next_beat {
            let tired = self.mood.tired.clamp(0.0, 1.0);
            // Calm, it keeps a slower, smaller beat.
            let (pace, height) = if self.lively { (1.0, 1.0) } else { (1.5, 0.8) };
            match self.mood.activity {
                Activity::Busy => {
                    self.jump(1.1 * height * (1.0 - tired * 0.6), now);
                    let swing = self.random(0.85, 1.2);
                    self.next_beat =
                        now + Duration::from_secs_f32((0.62 + tired * 0.5) * pace * swing);
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
            self.lean(0.5);
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
                let impact = self.rise;
                if impact < -0.6 {
                    self.squash.kick(impact * 2.4);
                    self.jiggle(-impact * 0.6);
                    self.face_y.kick(-impact * 0.55);
                }
                self.lift = 0.0;
                self.rise = 0.0;
                if self.encore > 0 {
                    self.encore -= 1;
                    self.encore_side = -self.encore_side;
                    self.jump(2.2, now);
                    self.tilt.kick(2.4 * self.encore_side);
                }
            }
            self.squash
                .retarget(1.0 + (self.rise.abs() * 0.05).min(0.16), SQUASH);
        } else if self.yawn.value() > 0.02 {
            self.squash.retarget(1.0 + 0.12 * self.yawn.value(), SQUASH);
        } else if self.lively {
            let (depth, pace) = if self.asleep() {
                (0.05, 1.3)
            } else {
                (0.03, 2.2)
            };
            let wander = self.random(0.85, 1.15);
            self.breath += (wander - self.breath) * dt * 0.3;
            let t = self.clock * pace * self.breath;
            let breath = (t.sin() + 0.45 * (t * 0.41 + 1.3).sin()) / 1.3;
            self.squash.retarget(1.0 + depth * breath, SQUASH);
        } else {
            self.squash.retarget(1.0, SQUASH);
        }
        self.face_y
            .retarget((self.rise * 0.03).clamp(-0.07, 0.07), FACE);
        self.tilt.retarget(-0.05 * self.yawn.value(), TILT);
        if self.moving() {
            self.badge_clock += dt;
        }
        let wanted = self.wanted();
        if wanted != Extra::None {
            // Swapped for another, it dips and pops back.
            if std::mem::discriminant(&wanted) != std::mem::discriminant(&self.shown)
                && self.badge.value() > 0.5
            {
                self.badge.kick(-6.0);
            }
            self.shown = wanted;
        }
        self.badge
            .retarget(if wanted == Extra::None { 0.0 } else { 1.0 }, POP);
        self.goal = rgb(if self.cheering(now) && !self.asleep() {
            self.mood.cheer_color
        } else {
            self.mood.color
        });
        let ease = 1.0 - (-dt * 6.0).exp();
        for (channel, goal) in self.color.iter_mut().zip(self.goal) {
            *channel += (goal - *channel) * ease;
            if (goal - *channel).abs() < 0.002 {
                *channel = goal;
            }
        }
        for spring in self.springs_mut() {
            spring.advance(dt, now);
        }
        for (bulges, size, phase) in &mut self.wobble {
            size.advance(dt, now);
            *phase = (*phase + dt * (1.2 + *bulges * 0.4)) % TAU;
        }
    }

    fn springs(&self) -> [&Spring; 14] {
        [
            &self.squash,
            &self.grow,
            &self.look_x,
            &self.look_y,
            &self.lid,
            &self.happy,
            &self.alert,
            &self.cheeks,
            &self.worry,
            &self.badge,
            &self.tilt,
            &self.face_y,
            &self.yawn,
            &self.puff,
        ]
    }

    fn springs_mut(&mut self) -> [&mut Spring; 14] {
        [
            &mut self.squash,
            &mut self.grow,
            &mut self.look_x,
            &mut self.look_y,
            &mut self.lid,
            &mut self.happy,
            &mut self.alert,
            &mut self.cheeks,
            &mut self.worry,
            &mut self.badge,
            &mut self.tilt,
            &mut self.face_y,
            &mut self.yawn,
            &mut self.puff,
        ]
    }

    pub fn is_settled(&self) -> bool {
        !self.moving()
            && self.lift == 0.0
            && self.jump_at.is_none()
            && self.blink.is_none()
            && self.color == rgb(self.mood.color)
            && self.springs().iter().all(|spring| spring.is_settled())
            && self.wobble.iter().all(|(_, size, _)| size.is_settled())
    }

    /// Moving, but only drifting, with no hop or blink under way.
    pub fn is_drifting(&self) -> bool {
        self.lift == 0.0
            && self.jump_at.is_none()
            && self.blink.is_none()
            && self
                .color
                .iter()
                .zip(self.goal)
                .all(|(channel, goal)| (goal - channel).abs() < COLOR_DRIFTING)
            && self
                .springs()
                .into_iter()
                .chain(self.wobble.iter().map(|(_, size, _)| size))
                .all(|spring| spring.motion() < DRIFTING)
    }

    /// When it next has something to do while calm and still.
    pub fn due(&self) -> Option<Instant> {
        let glance = self.idle().then(|| {
            self.hold_look
                .map_or(self.next_glance, |held| held.max(self.next_glance))
        });
        let idle = self
            .idle()
            .then(|| self.next_habit.min(self.resting_since + DOZE_AFTER));
        let second = self
            .next_too
            .map(|next| self.blink.map_or(next.at, |blink| next.at.max(blink.end())));
        [
            Some(self.next_blink),
            glance,
            idle,
            second,
            self.cheer_until,
            self.grumpy_until,
            self.yawn_until,
            self.hold_look,
            self.glance_at.map(|(at, _, _)| at),
            self.jump_at.map(|(at, _)| at),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    pub fn pose(&self, now: Instant) -> Pose {
        let blink = self.blink.map_or(0.0, |blink| {
            let progress = now.saturating_duration_since(blink.at).as_secs_f32() / blink.length;
            if progress <= 1.0 {
                (1.0 - (progress * 2.0 - 1.0).abs()) * blink.depth
            } else {
                0.0
            }
        });
        let alert = self.alert.value();
        // Gone, the last badge shrinks away.
        let extra = match self.wanted() {
            Extra::None => self.shown.at(self.badge_clock),
            wanted => wanted,
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
            worry: self.worry.value(),
            wobble,
            extra,
            badge: self.badge.value().max(0.0),
            tilt: self.tilt.value() + self.look_x.value() * 0.07,
            face: (self.look_x.value() * 0.05, self.face_y.value()),
            yawn: self.yawn.value().max(0.0),
            puff: self.puff.value().max(0.0),
            grumpy: self.grumpy(now),
            near: self.mood.near,
            tired: self.mood.tired.clamp(0.0, 1.0),
        }
    }
}

fn color([r, g, b]: [f32; 3], a: f32) -> Color {
    [r, g, b, a]
}

fn black(a: f32) -> Color {
    [0.0, 0.0, 0.0, a]
}

fn white(a: f32) -> Color {
    [1.0, 1.0, 1.0, a]
}

fn mix(from: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| math::mix(from[i], to[i], t))
}

fn fill(shapes: &mut Vec<Shape>, points: &[(f32, f32)], color: Color) {
    shapes.push(Shape::Fill {
        points: points.to_vec(),
        color,
    });
}

fn stroke(shapes: &mut Vec<Shape>, points: &[(f32, f32)], width: f32, color: Color) {
    shapes.push(Shape::Stroke {
        points: points.to_vec(),
        width,
        color,
    });
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

/// An eye as one outline that bends from open toward a shut arc and toward a happy arc, so it never
/// swaps shape from one frame to the next.
struct Eye {
    ew: f32,
    eh: f32,
    /// From 0 to 1, how far it bends toward the shut arc, which sags by `sag`.
    shut: f32,
    sag: f32,
    /// From 0 to 1, how far it bends toward the happy arc, over being shut.
    joy: f32,
    /// Half the arcs' thickness.
    half: f32,
}

impl Eye {
    /// Round `(x, y)`, open `open` high under a lid at `lid`.
    fn outline(&self, x: f32, y: f32, open: f32, lid: f32) -> Vec<(f32, f32)> {
        const STEPS: usize = 12;
        let Self {
            ew,
            eh,
            shut,
            sag,
            joy,
            half,
        } = *self;
        let width = math::mix(math::mix(ew, ew * 0.9, shut), ew, joy);
        let at = |u: f32, top: bool| {
            let round = (1.0 - u * u).max(0.0).sqrt();
            // A quadratic curve whose control is midway, as `curve` draws it.
            let bend = (1.0 - u * u) / 2.0;
            let (open_y, thick) = if top {
                ((y - open * round).max(lid).min(y + open * round), -half)
            } else {
                (y + open * round, half)
            };
            let shut_y = y + eh * (0.2 + (sag - 0.2) * bend) + thick;
            let happy_y = y + eh * (0.2 - bend) + thick;
            (
                x + u * width,
                math::mix(math::mix(open_y, shut_y, shut), happy_y, joy),
            )
        };
        let along = |i: usize| -(i as f32 / STEPS as f32 * PI).cos();
        (0..=STEPS)
            .map(|i| at(along(i), true))
            .chain((0..=STEPS).rev().map(|i| at(along(i), false)))
            .collect()
    }
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

fn paint(pose: &Pose, x0: f32, y0: f32, side: f32, shapes: &mut Vec<Shape>) {
    let small = side < GLINT_FROM;
    let (w, h) = (side * FILL, side * FILL / WIDE);
    // At rest it sits in the middle of its square.
    let ground = y0 + (side + h) / 2.0;
    let sy = pose.squash.min(MOST_STRETCH);
    let sx = (1.0 + (1.0 - sy) * 0.9).min(MOST_SPREAD) * (1.0 + 0.05 * pose.puff);
    let g = pose.grow;
    // Never higher than the room above it, so a hop does not leave its square.
    let hop = if small { 0.22 } else { 0.45 };
    let room = (ground - y0 - h * sy * g * MOST_WOBBLE - 1.0).max(0.0);
    let lift = (pose.lift * side * hop).min(room);
    let (base_x, base_y) = (x0 + side / 2.0, ground - lift);
    let (sin, cos) = pose.tilt.sin_cos();
    let at = |(x, y): (f32, f32)| {
        let (x, y) = (x * sx * g, y * sy * g);
        (base_x + x * cos - y * sin, base_y + x * sin + y * cos)
    };
    let place = |points: Vec<(f32, f32)>| points.into_iter().map(at).collect::<Vec<_>>();
    let (face_x, face_y) = (pose.face.0 * w, pose.face.1 * h);
    let face = |points: Vec<(f32, f32)>| {
        points
            .into_iter()
            .map(|(x, y)| at((x + face_x, y + face_y)))
            .collect::<Vec<_>>()
    };
    let ink = color(rgb(CRUST), 1.0);

    let off = (lift / (side * 0.2)).clamp(0.0, 1.0);
    let shadow = ellipse(
        base_x,
        (ground + 1.0).min(y0 + side - 1.5),
        w * 0.42 * sx * (1.0 - off * 0.3),
        (side * 0.03).max(1.0),
    );
    fill(shapes, &shadow, black(0.32 * (1.0 - off * 0.6)));

    fill(
        shapes,
        &place(squircle(pose, 0.0, -h / 2.0, w / 2.0, h / 2.0)),
        color(pose.color, 0.92),
    );
    // A darker core shows through the jelly.
    let core = mix(pose.color, rgb(CRUST), 0.12);
    fill(
        shapes,
        &place(squircle(pose, 0.0, -h * 0.42, w * 0.3, h * 0.28)),
        color(core, 0.6),
    );
    fill(
        shapes,
        &place(ellipse(-w * 0.2, -h * 0.8, w * 0.13, h * 0.07)),
        white(0.2),
    );
    fill(
        shapes,
        &place(ellipse(-w * 0.3, -h * 0.68, w * 0.025, w * 0.025)),
        white(0.35),
    );

    if pose.cheeks > 0.02 {
        let blush = color(rgb(PINK), pose.cheeks * 0.55);
        let big = 1.0 + 0.45 * pose.puff;
        for side in [-1.0, 1.0] {
            fill(
                shapes,
                &face(ellipse(
                    side * w * 0.3,
                    -h * 0.36,
                    w * 0.07 * big,
                    w * 0.042 * big,
                )),
                blush,
            );
        }
    }

    let (cy, dx, ew, eh) = (-h * 0.52, w * 0.18, w * 0.075, h * 0.17);
    let joy = pose.happy.clamp(0.0, 1.0);
    let shut = smooth(0.35, 0.1, pose.open)
        .max(smooth(0.75, 0.95, pose.lid))
        .max(smooth(0.1, 0.6, pose.yawn));
    let eye = Eye {
        ew,
        eh,
        shut,
        sag: 0.2 + 0.35 * smooth(0.9, 1.0, pose.lid),
        joy,
        // Never thinner than a pixel once drawn.
        half: (ew * 0.3).max(0.5 / g),
    };
    for side in [-1.0, 1.0] {
        let x = side * dx + pose.look.0 * w * 0.08;
        let y = cy + pose.look.1 * h * 0.08;
        let open = eh * pose.open;
        let lid = y - open + open * 2.0 * pose.lid;
        fill(shapes, &face(eye.outline(x, y, open, lid)), ink);
        let glint = 0.85 * (1.0 - shut) * (1.0 - joy);
        if !small && glint > 0.02 {
            // Cut off by its lid, as its eye is.
            let spot = ellipse(x - ew * 0.32, y - open * 0.4, ew * 0.32, ew * 0.32)
                .into_iter()
                .map(|(gx, gy)| (gx, gy.max(lid)))
                .collect();
            fill(shapes, &face(spot), white(glint));
        }
    }

    // The "o" of asking gives way to a smile as it cheers.
    let (mx, my, mw) = (pose.look.0 * w * 0.04, -h * 0.3, w * 0.06);
    if joy > 0.05 {
        let (sw, dip) = (mw * joy, mw * 1.3 * joy);
        stroke(
            shapes,
            &face(curve((mx - sw, my), (mx, my + dip), (mx + sw, my))),
            (mw * 0.28 * g * joy).max(1.0),
            ink,
        );
    }
    // Worried, its mouth bends into a small frown.
    let frown = pose.worry.clamp(0.0, 1.0) * (1.0 - joy);
    if frown > 0.05 {
        let (sw, base) = (mw * 0.9, my + mw * 0.35);
        stroke(
            shapes,
            &face(curve(
                (mx - sw, base),
                (mx, base - mw * 1.1 * frown),
                (mx + sw, base),
            )),
            (mw * 0.28 * g).max(1.0),
            ink,
        );
    }
    let gasp = pose.alert * (1.0 - joy) * (1.0 - pose.yawn);
    if gasp > 0.02 {
        fill(
            shapes,
            &face(ellipse(
                mx,
                my + mw * 0.2,
                mw * 0.38 * gasp,
                mw * 0.48 * gasp,
            )),
            ink,
        );
    }
    if pose.yawn > 0.03 {
        fill(
            shapes,
            &face(ellipse(
                mx,
                my + mw * 0.35,
                mw * 0.5 * pose.yawn,
                mw * 0.85 * pose.yawn,
            )),
            ink,
        );
    }
    if pose.grumpy && pose.puff > 0.05 {
        let (sw, y) = (mw * 0.55, my + mw * 0.3);
        stroke(
            shapes,
            &face(vec![(mx - sw, y), (mx + sw, y)]),
            (mw * 0.26 * g).max(1.0),
            color(rgb(CRUST), pose.puff.min(1.0)),
        );
    }
    let rest = (1.0 - joy)
        * (1.0 - (frown * 2.0).clamp(0.0, 1.0))
        * (1.0 - (gasp * 3.0).clamp(0.0, 1.0))
        * (1.0 - (pose.yawn * 3.0).clamp(0.0, 1.0))
        * (1.0 - pose.puff.min(1.0));
    if !small && rest > 0.05 {
        let smile = if pose.near { 0.85 } else { 0.4 } - pose.tired * 0.7;
        let (sw, y) = (mw * 0.6, my + mw * 0.1);
        stroke(
            shapes,
            &face(curve(
                (mx - sw, y),
                (mx, y + mw * 0.75 * smile),
                (mx + sw, y),
            )),
            (mw * 0.22 * g).max(1.0),
            color(rgb(CRUST), rest),
        );
    }

    // Its state, on a dot at the body's top-left corner, kept inside the square.
    let (tint, phase) = match pose.extra {
        Extra::Asking(phase) => (YELLOW, phase),
        Extra::Busy(phase) => (PEACH, phase),
        Extra::Asleep(phase) => (OVERLAY1, phase),
        Extra::Loading(phase) => (SAPPHIRE, phase),
        Extra::Warning => (CRITICAL, 0.0),
        Extra::None => return,
    };
    if pose.badge < 0.02 {
        return;
    }
    let pop = if matches!(pose.extra, Extra::Asking(_)) {
        1.0 + 0.08 * (phase * 7.0).sin()
    } else {
        1.0
    };
    let r = (side * 0.13).max(4.5) * pop * pose.badge;
    let ring = BADGE_RING * pose.badge.min(1.0);
    let edge = r + ring + 0.5;
    let top = ground - lift - h * sy * g;
    let x = (x0 + side / 2.0 - w * sx * g / 2.0 + r * 0.55).clamp(x0 + edge, x0 + side - edge);
    let y = (top + r * 0.45).clamp(y0 + edge, y0 + side - edge);
    fill(shapes, &(ellipse(x, y, r + ring, r + ring)), ink);
    fill(shapes, &(ellipse(x, y, r, r)), color(rgb(tint), 1.0));
    match pose.extra {
        Extra::Asking(_) | Extra::Warning => {
            stroke(
                shapes,
                &(vec![(x, y - r * 0.5), (x, y + r * 0.08)]),
                r * 0.26,
                ink,
            );
            fill(shapes, &(ellipse(x, y + r * 0.45, r * 0.14, r * 0.14)), ink);
        }
        Extra::Busy(_) => {
            for i in 0..3 {
                let up = (phase * 7.0 - i as f32 * 0.7).sin().max(0.0) * r * 0.22;
                let dot = ellipse(x + (i as f32 - 1.0) * r * 0.42, y - up, r * 0.13, r * 0.13);
                fill(shapes, &(dot), ink);
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
            stroke(shapes, &(arc), r * 0.24, ink);
        }
        _ => {
            let z = r * 0.36;
            let path = vec![
                (x - z, y - z),
                (x + z, y - z),
                (x - z, y + z),
                (x + z, y + z),
            ];
            stroke(shapes, &(path), r * 0.2, ink);
        }
    }
}

pub fn mark(pose: Pose) -> Mark {
    Mark::Drawn(Rc::new(move |side| drawing(&pose, side)))
}

impl Pose {
    pub fn feeling(feeling: Option<Feeling>) -> Self {
        let rest = Self {
            color: rgb(LAVENDER),
            squash: 1.0,
            lift: 0.0,
            grow: 1.0,
            look: (0.0, 0.0),
            open: 1.0,
            lid: 0.0,
            happy: 0.0,
            alert: 0.0,
            cheeks: 0.0,
            worry: 0.0,
            wobble: WOBBLES.map(|(bulges, _)| (bulges, 0.0, 0.0)),
            extra: Extra::None,
            badge: 0.0,
            tilt: 0.0,
            face: (0.0, 0.0),
            yawn: 0.0,
            puff: 0.0,
            grumpy: false,
            near: false,
            tired: 0.0,
        };
        match feeling {
            None => rest,
            Some(Feeling::Happy) => Self {
                color: rgb(GREEN),
                happy: 1.0,
                cheeks: 1.0,
                ..rest
            },
            Some(Feeling::Asking) => Self {
                color: rgb(YELLOW),
                open: 1.18,
                alert: 1.0,
                look: (0.0, -0.15),
                extra: Extra::Asking(0.0),
                badge: 1.0,
                ..rest
            },
            Some(Feeling::Tired) => Self {
                color: mix(rgb(LAVENDER), rgb(OVERLAY2), 0.55),
                lid: 0.55,
                look: (0.0, 0.3),
                tired: 1.0,
                ..rest
            },
            Some(Feeling::Asleep) => Self {
                color: rgb(OVERLAY1),
                lid: 1.0,
                look: (0.0, 0.3),
                extra: Extra::Asleep(0.0),
                badge: 1.0,
                ..rest
            },
        }
    }
}

/// `pose` in a square of `side`.
pub fn drawing(pose: &Pose, side: f32) -> Drawing {
    let mut shapes = Vec::new();
    paint(pose, 0.0, 0.0, side, &mut shapes);
    Drawing { side, shapes }
}

#[cfg(test)]
#[path = "mascot_test.rs"]
mod tests;
