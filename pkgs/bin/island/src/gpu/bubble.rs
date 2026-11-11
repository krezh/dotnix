//! Work a feature carries on with after its panel closes, as wgpu draws it: a droplet of the bar's
//! glass splitting off one of its ends with a ring filling round the feature's glyph and a line
//! saying what and how long, a card dropping from it under the pointer, and over a fullscreen
//! window only the ring.

use std::{collections::HashMap, rc::Rc, time::Instant};

use super::{
    blocks::Blocks,
    node::{BOLD, Painting, SEMIBOLD, div},
    paint::{Line, Painter},
    panel::{self, Built, Shown, mixed},
    renderer::{Gpu, Sprite, inside},
    widget::{Drawn, Hit, Layout, color, tinted},
};
use crate::{
    activity::{Activity, Standing},
    bar::WIDGET_H,
    bar_widget::{BarWidget, Emphasis, Frame, Part},
    components::{
        badge::Mark,
        card::{FrameView, Look as FrameLook, Motion},
        item::{Item, Lead, Trail},
    },
    config::Side,
    merge::Split,
    panel::{Block, Handler, SharedString},
    spring::{Spring, SpringConfig},
    stage::{
        Activities, COLLAPSED_H, CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, SHAPE_SPRING, SURFACE_W,
        Shape,
    },
    theme::{ACCENT, HOVER_ALPHA, SUBTEXT1, TEXT},
    ui::{PANEL_RADIUS, SMALL_SIZE},
};

/// Between the settled droplet and the bar it split from.
const GAP: f32 = 12.0;
/// How far the droplet pulls clear of the bar before the thread lets go, and how near it comes
/// back before the thread catches it again.
const SNAP: f32 = 7.0;
const CATCH: f32 = 4.0;
/// How deep the waist presses in at the bar's end, as a share of half the bar's height.
const WAIST_MAX: f32 = 0.55;
/// Out of the bar: response 0.5 s with a little overshoot. Back in: 0.45 s, none.
const SPLIT: SpringConfig = SpringConfig::new(157.9, 20.1, 1.0);
const MERGE: SpringConfig = SpringConfig::new(195.0, 27.9, 1.0);
/// How fast the thread takes hold and lets go: response 0.14 s.
const NECK: SpringConfig = SpringConfig::new(2014.2, 89.8, 1.0);
/// The words: response 0.35 s.
const WORDS: SpringConfig = SpringConfig::new(322.3, 35.9, 1.0);
/// Between the bubble and the card dropping from it.
const DROP_GAP: f32 = 6.0;
/// Inside the bubble, before the ring and after the last words.
const PAD: (f32, f32) = (9.0, 14.0);
/// Between the ring, the words and the count.
const SPACE: f32 = 7.0;
/// Between the title and its detail: a space.
const WORD_SPACE: f32 = 4.0;
/// The ring's slot, the ring in it, and the glyph inside.
const RING: (f32, f32, f32) = (20.0, 17.2, 10.0);
const RING_STROKE: f32 = 2.2;
const TRACK_ALPHA: f32 = 0.16;
/// Seconds a turning ring takes to go round, and how much of it is drawn.
const TURN: f32 = 1.1;
const SWEEP: f32 = 0.3;
const COUNT_SIZE: f32 = 11.0;
const COUNT_PAD: (f32, f32) = (6.0, 1.0);
const COUNT_RADIUS: f32 = 6.0;
const COUNT_ALPHA: f32 = 0.12;
const CARD_W: f32 = 420.0;
/// Inside the card: across, at the top and at the bottom, and between its blocks.
const CARD_PAD: (f32, f32, f32) = (14.0, 4.0, 14.0);
const CARD_GAP: f32 = 11.0;
/// How wide the card starts and ends, as a share of the bubble's width.
const BULGE: f32 = 1.0;
/// Over a fullscreen window: how tall the pill is, its ring's slot, the ring in it and its glyph.
pub const PILL_H: f32 = 30.0;
const PILL_RING: (f32, f32, f32) = (24.0, 20.4, 11.0);
const PILL_STROKE: f32 = 2.4;
const PILL_TRACK_ALPHA: f32 = 0.14;
/// Round the ring, between it and the words, and after them.
const PILL_PAD: f32 = 3.0;
const PILL_GAP: f32 = 8.0;
const PILL_END: f32 = 10.0;

fn title_color(activity: &Activity) -> u32 {
    if activity.standing == Standing::Running {
        TEXT
    } else {
        activity.color
    }
}

/// A ring round a glyph, filling with progress, or turning while that cannot be told.
struct Ring {
    /// Where the arc starts, in turns from the top; while turning it follows the clock.
    start: Spring,
    length: Spring,
    /// Where a turning arc stood at the clock's start, or `None` while measured.
    turning: Option<f32>,
    hue: Spring,
    from: [f32; 4],
    to: [f32; 4],
}

impl Ring {
    fn new() -> Self {
        Self {
            start: Spring::new(0.0, SHAPE_SPRING, 0.001),
            length: Spring::new(0.0, CONTENT_IN, 0.001),
            turning: None,
            hue: Spring::new(1.0, CONTENT_IN, 0.002),
            from: [0.0; 4],
            to: [0.0; 4],
        }
    }

    fn at(&self, seconds: f32) -> f32 {
        match self.turning {
            Some(phase) => phase + seconds / TURN,
            None => self.start.value(),
        }
    }

    fn aim(&mut self, progress: Option<f32>, tint: u32, seconds: f32) {
        match (progress, self.turning) {
            (Some(share), turning) => {
                if turning.is_some() {
                    let at = self.at(seconds);
                    self.turning = None;
                    self.start.reset(at);
                    self.start.retarget(at.ceil(), SHAPE_SPRING);
                }
                let share = share.clamp(0.0, 1.0);
                if (self.length.target() - share).abs() > 0.0005 {
                    self.length.retarget(share, CONTENT_IN);
                }
            }
            (None, None) => {
                self.turning = Some(self.start.value() - seconds / TURN);
                self.length.retarget(SWEEP, CONTENT_IN);
            }
            (None, Some(_)) => {}
        }
        let tint = color(tint);
        if tint != self.to {
            self.from = if self.to == [0.0; 4] {
                tint
            } else {
                self.color()
            };
            self.to = tint;
            self.hue.reset(0.0);
            self.hue.retarget(1.0, CONTENT_IN);
        }
    }

    fn color(&self) -> [f32; 4] {
        mixed(self.from, self.to, self.hue.value().clamp(0.0, 1.0))
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.start.advance(dt, now);
        self.length.advance(dt, now);
        self.hue.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.turning.is_none()
            && self.start.is_settled()
            && self.length.is_settled()
            && self.hue.is_settled()
    }

    /// The ring with its middle at `middle`, `side` across, and `glyph` in it.
    #[allow(clippy::too_many_arguments)]
    fn sprites(
        &self,
        middle: (f32, f32),
        (side, stroke, track): (f32, f32, f32),
        glyph: Option<(&std::path::Path, f32)>,
        seconds: f32,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let rect = [middle.0 - side / 2.0, middle.1 - side / 2.0, side, side];
        out.push(Sprite::arc(
            rect,
            stroke,
            0.0,
            1.0,
            tinted_alpha(TEXT, track),
        ));
        let tint = self.color();
        let length = self.length.value().clamp(0.0, 1.0);
        if length > 0.001 {
            let from = self.at(seconds).rem_euclid(1.0);
            let to = from + length;
            out.push(Sprite::arc(rect, stroke, from, to.min(1.0), tint));
            if to > 1.0 {
                out.push(Sprite::arc(rect, stroke, 0.0, to - 1.0, tint));
            }
        }
        if let Some((path, mark)) = glyph {
            painter.glyph_icon(
                gpu,
                path,
                [middle.0 - mark / 2.0, middle.1 - mark / 2.0, mark, mark],
                tint,
                scale,
                out,
            );
        }
    }
}

/// 0 at `from` or before, 1 at `to` or past, easing in and out between.
fn smooth(from: f32, to: f32, at: f32) -> f32 {
    let t = ((at - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn tinted_alpha(tint: u32, alpha: f32) -> [f32; 4] {
    color(tint).map(|channel| channel * alpha)
}

/// The line as last drawn, with what it was drawn from.
struct Drawing {
    key: (SharedString, Option<SharedString>, u32, usize),
    drawn: Drawn,
    count: Option<Rc<Line>>,
    glyph: Option<std::path::PathBuf>,
    /// How wide the whole bubble is for it.
    width: f32,
}

/// The card dropping from the bubble.
struct Drop {
    open: bool,
    w: Spring,
    h: Spring,
    content: Spring,
    built: Option<Built>,
    eases: HashMap<u64, Spring>,
    texts: HashMap<u64, Shown>,
    rect: [f32; 4],
}

impl Drop {
    fn new() -> Self {
        Self {
            open: false,
            w: Spring::new(0.0, SHAPE_SPRING, 0.1),
            h: Spring::new(0.0, SHAPE_SPRING, 0.1),
            content: Spring::new(0.0, CONTENT_OUT, 0.002),
            built: None,
            eases: HashMap::new(),
            texts: HashMap::new(),
            rect: [0.0; 4],
        }
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.w.advance(dt, now);
        self.h.advance(dt, now);
        self.content.advance(dt, now);
        for spring in self.eases.values_mut() {
            spring.advance(dt, now);
        }
    }

    fn is_settled(&self) -> bool {
        self.w.is_settled()
            && self.h.is_settled()
            && self.content.is_settled()
            && self.eases.values().all(Spring::is_settled)
    }

    fn shown(&self) -> bool {
        self.h.value() > 0.5
    }
}

pub struct Bubble {
    /// How far along its way out of the bar it is: 0 wholly inside the bar's end, 1 settled.
    out: Spring,
    /// How firmly the waist and thread hold it to the bar: 1 joined, 0 let go.
    neck: Spring,
    attached: bool,
    /// Its shape against the bar this frame.
    split: Option<Split>,
    /// How wide it stands.
    natural: f32,
    /// It carries a widget split off the bar rather than what is under way, and where that
    /// widget was last drawn.
    carrying: bool,
    pub carried_at: Option<(f32, f32)>,
    content: Spring,
    ring: Ring,
    line: Option<Drawing>,
    texts: HashMap<u64, Shown>,
    /// Where it stands this frame.
    pub rect: [f32; 4],
    present: bool,
    hovered: bool,
    drop: Drop,
    /// Lay the card out again before drawing it.
    stale: bool,
    /// What the line opens.
    on_open: Option<Handler>,
    /// Over a fullscreen window: how far the words show.
    words: Spring,
    pill_words: Option<(SharedString, Rc<Line>)>,
    pill: Ring,
    started: Instant,
}

impl Bubble {
    pub fn new() -> Self {
        Self {
            out: Spring::new(0.0, SPLIT, 0.0005),
            neck: Spring::new(0.0, NECK, 0.002),
            attached: true,
            split: None,
            natural: 0.0,
            carrying: false,
            carried_at: None,
            content: Spring::new(0.0, WORDS, 0.002),
            ring: Ring::new(),
            line: None,
            texts: HashMap::new(),
            rect: [0.0; 4],
            present: false,
            hovered: false,
            drop: Drop::new(),
            stale: true,
            on_open: None,
            words: Spring::new(0.0, CONTENT_OUT, 0.002),
            pill_words: None,
            pill: Ring::new(),
            started: Instant::now(),
        }
    }

    fn seconds(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    /// What it shows changed: lay the card out again.
    pub fn changed(&mut self) {
        self.stale = true;
    }

    /// Follows what is under way, or else a widget `carried` that wide split off the bar; `bar`
    /// is whether the bar shows, so it can stand beside it.
    #[allow(clippy::too_many_arguments)]
    pub fn aim(
        &mut self,
        activities: &mut Activities,
        carried: Option<f32>,
        bar: bool,
        animate: bool,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        owner: &str,
    ) {
        let seconds = self.seconds();
        let more = activities.shown.len().saturating_sub(1);
        let top = activities.shown.first().map(|(_, activity)| activity);
        let present = bar && (top.is_some() || carried.is_some());
        if let Some(top) = top {
            self.on_open = Some(top.on_open.clone());
            self.draw_line(top, more, painter, gpu, scale, owner);
            self.ring.aim(top.progress, top.color, seconds);
            self.pill.aim(top.progress, top.color, seconds);
            self.carrying = false;
        } else if let Some(width) = carried {
            self.carrying = true;
            self.natural = width;
        }
        if !self.carrying {
            self.natural = self.line.as_ref().map_or(0.0, |line| line.width);
        }
        let natural = self.natural;
        if present {
            if !animate {
                self.out.reset(1.0);
                self.content.reset(1.0);
                self.neck.reset(0.0);
                self.attached = false;
            }
            if self.out.target() != 1.0 {
                if self.out.value() < 0.5 {
                    self.attached = true;
                    self.neck.reset(0.0);
                }
                self.out.retarget(1.0, SPLIT);
            }
            if self.content.target() != 1.0 {
                self.content.retarget(1.0, WORDS);
            }
        } else {
            if self.out.target() != 0.0 {
                self.out.retarget(0.0, MERGE);
            }
            if self.content.target() != 0.0 {
                self.content.retarget(0.0, WORDS);
            }
            self.hovered = false;
        }
        self.present = present;
        if self.stale && present && !self.carrying {
            self.stale = false;
            self.lay_card(activities, painter, gpu, scale);
        }
        let open = present && self.hovered && !self.carrying && self.drop.built.is_some();
        self.aim_drop(open, natural);
    }

    fn aim_drop(&mut self, open: bool, from: f32) {
        let drop = &mut self.drop;
        let (w, h) = drop
            .built
            .as_ref()
            .map_or((0.0, 0.0), |built| (built.width, built.height));
        if open {
            if !drop.shown() {
                drop.w.reset(from * BULGE);
            }
            if drop.w.target() != w {
                drop.w.retarget(w, SHAPE_SPRING);
            }
            if drop.h.target() != h {
                drop.h.retarget(h, SHAPE_SPRING);
            }
            if drop.content.target() != 1.0 {
                drop.content.retarget(1.0, CONTENT_IN);
            }
        } else if drop.open {
            drop.w.retarget(from * BULGE, SHAPE_SPRING);
            drop.h.retarget(0.0, SHAPE_SPRING);
            drop.content.retarget(0.0, CONTENT_OUT);
        }
        drop.open = open;
    }

    fn draw_line(
        &mut self,
        activity: &Activity,
        more: usize,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        owner: &str,
    ) {
        let tint = title_color(activity);
        let key = (activity.title.clone(), activity.detail.clone(), tint, more);
        if self.line.as_ref().is_some_and(|line| line.key == key) {
            return;
        }
        let mut widget = BarWidget::new(Frame::Row)
            .gap(WORD_SPACE)
            .part(Part::Cluster {
                color: tint,
                parts: vec![Part::text(activity.title.clone(), Emphasis::Strong)],
            });
        if let Some(detail) = &activity.detail {
            widget = widget.part(Part::Cluster {
                color: SUBTEXT1,
                parts: vec![Part::text(format!("· {detail}"), Emphasis::Strong)],
            });
        }
        let mut layout = Layout {
            painter,
            gpu,
            scale,
            hover_inset: 0.0,
            half: 0.0,
            grouped: false,
            started: self.started,
            before: &self.texts,
            owner: format!("activity-{owner}"),
        };
        let mut drawn = layout.draw(&widget);
        self.texts = std::mem::take(&mut drawn.texts);
        let count = (more > 0).then(|| painter.line(&format!("+{more}"), COUNT_SIZE, BOLD, scale));
        let count_w = count
            .as_ref()
            .map_or(0.0, |line| SPACE + line.width.ceil() + 2.0 * COUNT_PAD.0);
        let width = (PAD.0 + RING.0 + SPACE + drawn.width + count_w + PAD.1).ceil();
        self.line = Some(Drawing {
            key,
            drawn,
            count,
            glyph: activity.glyph.clone(),
            width,
        });
    }

    /// Lays the card out: the most pressing one's own, or, with several, a line for each.
    fn lay_card(
        &mut self,
        activities: &mut Activities,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
    ) {
        let blocks = if activities.shown.len() > 1 {
            several(&activities.shown)
        } else {
            match activities.shown.first_mut() {
                Some((_, activity)) if !activity.card.is_empty() => {
                    std::mem::take(&mut activity.card)
                }
                Some(_) => return,
                None => {
                    self.drop.built = None;
                    return;
                }
            }
        };
        let offsets = HashMap::new();
        let mut painting = Painting {
            painter,
            gpu,
            scale,
            built: Built::default(),
            offsets: &offsets,
            holder_rect: [0.0; 4],
            lighting: None,
            before: &self.drop.texts,
            path: 0,
            align: 0.0,
            paints: 0,
            still: false,
        };
        let mut laid = Blocks {
            painting: &mut painting,
            popovers: Vec::new(),
            prompt: None,
            island_w: CARD_W,
            sliding: false,
            chosen: HashMap::new(),
            grow: None,
        };
        let children = laid.all(blocks, "activity");
        let node = div()
            .w(CARD_W)
            .col()
            .gap(CARD_GAP)
            .px(CARD_PAD.0)
            .pt(CARD_PAD.1)
            .pb(CARD_PAD.2)
            .children(children);
        let (width, height) = painting.paint(&node, None, (0.0, 0.0));
        let mut built = painting.built;
        built.width = width.ceil();
        built.height = height.ceil();
        let shown = self.drop.built.is_some();
        self.drop
            .eases
            .retain(|key, _| built.eases.iter().any(|ease| ease.key == *key));
        for ease in &built.eases {
            match self.drop.eases.get_mut(&ease.key) {
                Some(spring) if shown => spring.retarget(ease.value, ease.config),
                Some(spring) => spring.reset(ease.value),
                None => {
                    self.drop
                        .eases
                        .insert(ease.key, Spring::new(ease.value, ease.config, 0.001));
                }
            }
        }
        self.drop.texts = std::mem::take(&mut built.texts);
        self.drop.built = Some(built);
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.out.advance(dt, now);
        self.neck.advance(dt, now);
        self.content.advance(dt, now);
        self.words.advance(dt, now);
        self.ring.advance(dt, now);
        self.pill.advance(dt, now);
        self.drop.advance(dt, now);
    }

    pub fn is_settled(&self) -> bool {
        let shown = self.shown();
        self.out.is_settled()
            && self.neck.is_settled()
            && self.content.is_settled()
            && self.words.is_settled()
            && (!shown || self.ring.is_settled())
            && self.drop.is_settled()
            && !self.morphing()
    }

    fn morphing(&self) -> bool {
        self.line
            .as_ref()
            .is_some_and(|line| line.drawn.morphs.iter().any(panel::MorphText::is_running))
    }

    /// Something in it moves on its own, such as a ring turning, so it wants frames.
    pub fn moving(&self, pill: bool) -> bool {
        let ring = if pill { &self.pill } else { &self.ring };
        let shown = pill || self.shown();
        shown && ring.turning.is_some()
    }

    fn shown(&self) -> bool {
        self.out.value() > 0.002 || self.out.target() > 0.5
    }

    /// The droplet against the bar `body` at its `side`, for the bar to be drawn with it; none
    /// while it is wholly back inside.
    pub fn split(&mut self, body: [f32; 4], side: Side, scale: f32) -> Option<Split> {
        let natural = self.natural;
        self.split = None;
        if natural <= 0.0 || !self.shown() {
            self.rect = [0.0; 4];
            return None;
        }
        let s = self.out.value().max(0.0);
        let half = natural / 2.0;
        let hh = COLLAPSED_H / 2.0;
        let dir = match side {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let edge = match side {
            Side::Left => body[0],
            Side::Right => body[0] + body[2],
        };
        let inside = edge - dir * half;
        let outside = edge + dir * (GAP + half);
        let middle = inside + (outside - inside) * s;
        let left = ((middle - half) * scale).round() / scale;
        let apart = dir * ((middle - dir * half) - edge);
        let leaving = self.out.target() > 0.5;
        if self.attached && leaving && apart > SNAP {
            self.attached = false;
        } else if !self.attached && !leaving && apart < CATCH {
            self.attached = true;
        }
        let neck = if self.attached { 1.0 } else { 0.0 };
        if self.neck.target() != neck {
            self.neck.retarget(neck, NECK);
        }
        let held = self.neck.value().clamp(0.0, 1.0);
        let coming = smooth(-2.0 * half, 0.0, apart);
        let parting = smooth(0.0, SNAP, apart);
        self.rect = [left, body[1], natural, COLLAPSED_H];
        let split = Split {
            x: left,
            width: natural,
            waist: held * WAIST_MAX * hh * coming * (1.0 - parting),
            thread: held * (1.0 - WAIST_MAX) * hh * (1.0 - parting),
        };
        self.split = Some(split);
        Some(split)
    }

    /// Nothing of it is drawn this frame.
    pub fn unplace(&mut self) {
        self.rect = [0.0; 4];
        self.drop.rect = [0.0; 4];
    }

    /// Where it takes the pointer: itself, the card under it and between them.
    pub fn rects(&self) -> Vec<[f32; 4]> {
        if self.rect[2] < 1.0 {
            return Vec::new();
        }
        let mut rects = vec![self.rect];
        let drop = self.drop.rect;
        if drop[3] >= 0.5 {
            let top = self.rect[1] + self.rect[3];
            rects.push([self.rect[0], top, self.rect[2], drop[1] - top]);
            rects.push(drop);
        }
        rects
    }

    /// Where its glass is, for the screen behind to blur.
    pub fn glass_rects(&self) -> Vec<[f32; 4]> {
        [self.rect, self.drop.rect]
            .into_iter()
            .filter(|rect| rect[2] >= 1.0 && rect[3] >= 0.5)
            .collect()
    }

    pub fn covers(&self, at: (f32, f32)) -> bool {
        self.present && self.rects().iter().any(|rect| inside(*rect, at))
    }

    /// The pointer came onto it or left it.
    pub fn hover(&mut self, over: bool) {
        self.hovered = over && self.present;
    }

    /// What a click at `at` does, and where it must come back up.
    pub fn press(&self, at: (f32, f32)) -> Option<(Handler, [f32; 4])> {
        if !self.present {
            return None;
        }
        let [x, y, ..] = self.drop.rect;
        if self.drop.content.value() > 0.5
            && let Some(built) = &self.drop.built
            && let Some(hit) = built
                .under((at.0 - x, at.1 - y))
                .find(|hit| hit.on_click.is_some())
        {
            let rect = [hit.rect[0] + x, hit.rect[1] + y, hit.rect[2], hit.rect[3]];
            return hit.on_click.clone().map(|handler| (handler, rect));
        }
        if self.carrying {
            return None;
        }
        inside(self.rect, at)
            .then(|| self.on_open.clone().map(|handler| (handler, self.rect)))
            .flatten()
    }

    /// It is out with a widget split off the bar.
    pub fn carries(&self) -> bool {
        self.present && self.carrying
    }

    /// Something under the pointer answers a click.
    pub fn clickable(&self, at: (f32, f32)) -> bool {
        self.press(at).is_some()
    }

    /// The innermost part of the widget `carried` it shows under `at`, and where it stands; all
    /// of the droplet round it answers as the widget itself.
    pub fn carried_hit<'a>(
        &self,
        carried: &'a Drawn,
        at: (f32, f32),
    ) -> Option<(&'a Hit, [f32; 4])> {
        let (x, y) = self.carried_at.filter(|_| self.present && self.carrying)?;
        if !inside(self.rect, at) {
            return None;
        }
        carried
            .hits
            .iter()
            .rev()
            .find_map(|hit| {
                let rect = [x + hit.rect[0], y, hit.rect[2], WIDGET_H];
                inside(rect, at).then_some((hit, rect))
            })
            .or_else(|| carried.hits.first().map(|hit| (hit, self.rect)))
    }

    /// What the droplet shows, where [`Self::split`] placed it in the bar's glass, `corner` as its
    /// corners, `glass` and `edge` the colors of the card dropping from it.
    #[allow(clippy::too_many_arguments)]
    pub fn sprites(
        &mut self,
        side: Side,
        carried: Option<&Drawn>,
        (radius, power): (f32, f32),
        (glass, edge, border, shadow): ([f32; 4], [f32; 4], f32, f32),
        pointer: Option<(f32, f32)>,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        self.carried_at = None;
        if self.split.is_none() {
            self.rect = [0.0; 4];
            self.drop.rect = [0.0; 4];
            return;
        }
        let rect = self.rect;
        let [start, top, _, height] = rect;
        let radius = radius.min(height / 2.0).min(rect[2] / 2.0);
        let shown = self.content.value().clamp(0.0, 1.0) * smooth(0.7, 0.98, self.out.value());
        let first = out.len();
        let middle = top + height / 2.0;
        let seconds = self.seconds();
        if self.carrying {
            if let Some(drawn) = carried {
                let at = (
                    ((start + (rect[2] - drawn.width) / 2.0) * scale).round() / scale,
                    middle - WIDGET_H / 2.0,
                );
                out.extend(drawn.sprites.iter().copied());
                for morph in &drawn.morphs {
                    panel::morph_sprites(morph, out);
                }
                for sprite in &mut out[first..] {
                    *sprite = sprite.moved(at.0, at.1);
                }
                self.carried_at = Some(at);
            }
        } else if let Some(line) = &self.line {
            self.ring.sprites(
                (start + PAD.0 + RING.0 / 2.0, middle),
                (RING.1, RING_STROKE, TRACK_ALPHA),
                line.glyph.as_deref().map(|path| (path, RING.2)),
                seconds,
                painter,
                gpu,
                scale,
                out,
            );
            let words = start + PAD.0 + RING.0 + SPACE;
            let row = middle - WIDGET_H / 2.0;
            let from = out.len();
            out.extend(line.drawn.sprites.iter().copied());
            for morph in &line.drawn.morphs {
                panel::morph_sprites(morph, out);
            }
            for sprite in &mut out[from..] {
                *sprite = sprite.moved(words, row);
            }
            if let Some(count) = &line.count {
                let x = words + line.drawn.width + SPACE;
                let w = count.width.ceil() + 2.0 * COUNT_PAD.0;
                let h = COUNT_SIZE * 1.2 + 2.0 * COUNT_PAD.1;
                out.push(Sprite::shape(
                    [x, middle - h / 2.0, w, h],
                    tinted_alpha(TEXT, COUNT_ALPHA),
                    COUNT_RADIUS,
                    2.0,
                ));
                painter.text(
                    gpu,
                    count,
                    (x + COUNT_PAD.0, middle - COUNT_SIZE * 1.2 / 2.0),
                    color(SUBTEXT1),
                    scale,
                    out,
                );
            }
        }
        let clip = [rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]];
        for sprite in &mut out[first..] {
            *sprite = sprite
                .moved(0.0, (1.0 - shown) * CONTENT_DRIFT / 2.0)
                .faded(shown)
                .within(clip);
        }
        self.drop_sprites(
            rect,
            side,
            (radius, power),
            (glass, edge, border, shadow),
            pointer,
            scale,
            out,
        );
    }

    fn glass(
        &self,
        rect: [f32; 4],
        radius: f32,
        power: f32,
        (glass, edge, border, shadow): ([f32; 4], [f32; 4], f32, f32),
        out: &mut Vec<Sprite>,
    ) {
        if shadow > 0.0 {
            out.push(Sprite::drop_shadow(
                rect,
                [0.0, 0.0, 0.0, 0.35 * shadow],
                radius,
                15.0,
                10.0,
            ));
            out.push(Sprite::ring(
                [rect[0] - 1.0, rect[1] - 1.0, rect[2] + 2.0, rect[3] + 2.0],
                [0.0, 0.0, 0.0, 0.3 * shadow],
                radius + 1.0,
                power,
            ));
        }
        out.push(Sprite::shape(rect, glass, radius, power));
        if border > 0.0 {
            out.push(Sprite::ring(rect, edge, radius, power).lined(border));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn drop_sprites(
        &mut self,
        bubble: [f32; 4],
        side: Side,
        (_, power): (f32, f32),
        look: ([f32; 4], [f32; 4], f32, f32),
        pointer: Option<(f32, f32)>,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let (w, h) = (self.drop.w.value().max(0.0), self.drop.h.value().max(0.0));
        let Some(built) = self.drop.built.as_ref().filter(|_| h >= 0.5 && w >= 1.0) else {
            self.drop.rect = [0.0; 4];
            return;
        };
        let x = match side {
            Side::Left => bubble[0].max(DROP_GAP),
            Side::Right => (bubble[0] + bubble[2] - w).min(SURFACE_W - DROP_GAP - w),
        };
        let x = (x * scale).round() / scale;
        let y = bubble[1] + bubble[3] + DROP_GAP;
        let rect = [x, y, w, h];
        self.drop.rect = rect;
        let radius = PANEL_RADIUS.min(h / 2.0);
        self.glass(rect, radius, power, look, out);
        let shown = self.drop.content.value().clamp(0.0, 1.0);
        if shown <= 0.001 {
            return;
        }
        let first = out.len();
        out.extend(built.sprites.iter().copied());
        let own = &mut out[first..];
        for ease in &built.eases {
            if let Some(spring) = self.drop.eases.get(&ease.key) {
                panel::ease_sprites(ease, spring.value(), &mut own[ease.from..ease.to]);
            }
        }
        let local = pointer.map(|at| (at.0 - x, at.1 - y));
        if let Some(fill) = local
            .and_then(|local| built.answering(local, |hit| hit.hover.is_some()))
            .and_then(|hit| hit.hover)
        {
            own[fill].color = tinted(ACCENT, HOVER_ALPHA);
        }
        for morph in &built.morphs {
            panel::morph_sprites(morph, out);
        }
        let clip = [rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]];
        for sprite in &mut out[first..] {
            *sprite = sprite
                .moved(x, y - (1.0 - shown) * CONTENT_DRIFT)
                .faded(shown)
                .within(clip);
        }
    }

    /// The island's shape over a fullscreen window: the ring, widening to say what changed.
    pub fn pill_shape(
        &mut self,
        activities: &Activities,
        now: Instant,
        painter: &mut Painter,
        scale: f32,
    ) -> Option<Shape> {
        let top = activities.ringed()?;
        let announcing = activities.announcing(now);
        let words = &top.title;
        if self
            .pill_words
            .as_ref()
            .is_none_or(|(shown, _)| shown != words)
        {
            let line = painter.line(words, SMALL_SIZE, SEMIBOLD, scale);
            self.pill_words = Some((words.clone(), line));
        }
        let to = if announcing { 1.0 } else { 0.0 };
        if self.words.target() != to {
            self.words
                .retarget(to, if announcing { CONTENT_IN } else { CONTENT_OUT });
        }
        let ring = 2.0 * PILL_PAD + PILL_RING.0;
        let wide = self.pill_words.as_ref().map_or(0.0, |(_, line)| {
            PILL_RING.0 + PILL_PAD + PILL_GAP + line.width.ceil() + PILL_END
        });
        Some(Shape {
            w: if announcing { wide } else { ring },
            h: PILL_H,
        })
    }

    /// The ring and what it says, in the island's glass `body`.
    pub fn pill_sprites(
        &self,
        body: [f32; 4],
        activities: &Activities,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let Some(top) = activities.ringed() else {
            return;
        };
        let first = out.len();
        let middle = (
            body[0] + PILL_PAD + PILL_RING.0 / 2.0,
            body[1] + body[3] / 2.0,
        );
        self.pill.sprites(
            middle,
            (PILL_RING.1, PILL_STROKE, PILL_TRACK_ALPHA),
            top.glyph.as_deref().map(|path| (path, PILL_RING.2)),
            self.seconds(),
            painter,
            gpu,
            scale,
            out,
        );
        let words = self.words.value().clamp(0.0, 1.0);
        if words > 0.001
            && let Some((_, line)) = &self.pill_words
        {
            let from = out.len();
            painter.text(
                gpu,
                line,
                (
                    body[0] + PILL_PAD + PILL_RING.0 + PILL_GAP,
                    middle.1 - SMALL_SIZE * 1.2 / 2.0,
                ),
                color(title_color(top)),
                scale,
                out,
            );
            for sprite in &mut out[from..] {
                *sprite = sprite.faded(words);
            }
        }
        let clip = [body[0], body[1], body[0] + body[2], body[1] + body[3]];
        for sprite in &mut out[first..] {
            *sprite = sprite.within(clip);
        }
    }
}

/// With several under way, the card lists them all, most pressing first.
fn several(shown: &[(&'static str, Activity)]) -> Vec<Block> {
    let mut blocks = vec![Block::Item(
        Item {
            id: "activity-several".into(),
            lead: None,
            overline: None,
            title: format!("{} things under way", shown.len()).into(),
            title_color: None,
            wrap: false,
            detail: "".into(),
            tinted: Vec::new(),
            trail: Vec::new(),
        },
        Vec::new(),
    )];
    blocks.extend(shown.iter().map(|(_, activity)| {
        Block::Frame(
            FrameView {
                id: format!("activity-{}", activity.key).into(),
                look: FrameLook::Plain,
                on_click: Some(activity.on_open.clone()),
                on_dismiss: None,
                countdown: None,
                selected: false,
                motion: Motion::Still,
                shown_height: None,
            },
            vec![Block::Item(
                Item {
                    id: format!("activity-item-{}", activity.key).into(),
                    lead: Some(Lead {
                        mark: activity
                            .glyph
                            .clone()
                            .map(|path| Mark::Glyph(path, activity.color)),
                        tint: Some(activity.color),
                        glow: None,
                        faded: false,
                        size: None,
                        on_click: None,
                        top: false,
                    }),
                    overline: None,
                    title: activity.title.clone(),
                    title_color: None,
                    wrap: false,
                    detail: "".into(),
                    tinted: Vec::new(),
                    trail: activity
                        .aside
                        .iter()
                        .map(|(text, tint)| Trail::Text(text.clone(), *tint))
                        .collect(),
                },
                vec![Block::note(activity.summary.clone())],
            )],
        )
    }));
    blocks
}
