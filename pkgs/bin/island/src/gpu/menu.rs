//! The island's own menus, such as a tray item's, hanging below the bar.

use std::time::Instant;

use super::{
    node::{MEDIUM, SEMIBOLD},
    paint::Painter,
    renderer::{Gpu, Sprite, inside},
    widget::{color, tinted},
};
use crate::{
    menu::{MAX_W, MenuNode},
    spring::Spring,
    stage::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, SHAPE_SPRING},
    theme::{
        ACCENT, CONTROL_ALPHA, HOVER_ALPHA, OVERLAY2, SUBTEXT1, SURFACE, SURFACE_BORDER_ALPHA,
        SURFACE1, TEXT,
    },
    ui::{CONTROL_RADIUS, PANEL_RADIUS, SMALL_SIZE},
};

const LINE: f32 = 1.0;
/// How wide a menu starts and ends, as a share of its width.
const BULGE: f32 = 0.6;
const ENTRY_H: f32 = 28.0;
const SEPARATOR_H: f32 = 9.0;
const PAD: f32 = PANEL_RADIUS - CONTROL_RADIUS;
const MIN_W: f32 = 160.0;
const SIDE: f32 = 8.0;
const CHECK_W: f32 = 22.0;

#[derive(Clone, Copy)]
pub enum Action {
    Enter(usize),
    Back,
    Pick(i32),
}

struct Row {
    rect: [f32; 4],
    action: Option<Action>,
}

/// What showed before, fading out as the new contents come in.
struct Leaving {
    node: MenuNode,
    sub: bool,
    width: f32,
    opacity: Spring,
}

pub struct Menu {
    pub source: String,
    root: MenuNode,
    path: Vec<usize>,
    pub pick: Box<dyn Fn(i32)>,
    anchor: Spring,
    drawn_anchor: Option<f32>,
    open: bool,
    w: Spring,
    h: Spring,
    content: Spring,
    leaving: Option<Leaving>,
    /// How wide what is showing was last drawn.
    width: f32,
    rows: Vec<Row>,
    pub rect: [f32; 4],
}

impl Menu {
    pub fn new(source: String, root: MenuNode, pick: Box<dyn Fn(i32)>, anchor: f32) -> Self {
        Self {
            source,
            root,
            path: Vec::new(),
            pick,
            anchor: Spring::new(anchor, SHAPE_SPRING, 0.1),
            drawn_anchor: None,
            open: true,
            w: Spring::new(0.0, SHAPE_SPRING, 0.1),
            h: Spring::new(0.0, SHAPE_SPRING, 0.1),
            content: Spring::new(0.0, CONTENT_OUT, 0.002),
            leaving: None,
            width: 0.0,
            rows: Vec::new(),
            rect: [0.0; 4],
        }
    }

    /// Turns into another menu, from wherever this one has got to.
    pub fn replace(&mut self, source: String, root: MenuNode, pick: Box<dyn Fn(i32)>, anchor: f32) {
        if self.h.value() >= 0.5 {
            self.swap();
            if self.anchor.is_settled()
                && let Some(drawn) = self.drawn_anchor
            {
                self.anchor.reset(drawn);
            }
            self.anchor.retarget(anchor, SHAPE_SPRING);
        } else {
            self.leaving = None;
            self.w.reset(0.0);
            self.h.reset(0.0);
            self.anchor.reset(anchor);
            self.drawn_anchor = None;
            self.content.reset(0.0);
        }
        self.source = source;
        self.root = root;
        self.path.clear();
        self.pick = pick;
        self.open = true;
    }

    fn current(&self) -> &MenuNode {
        self.path.iter().fold(&self.root, |node, index| {
            node.children.get(*index).unwrap_or(node)
        })
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn swap(&mut self) {
        let current = self.content.value();
        let keep_leaving = self
            .leaving
            .as_ref()
            .is_some_and(|leaving| leaving.opacity.value() >= current);
        if !keep_leaving && current > 0.001 {
            let mut opacity = Spring::new(current, CONTENT_OUT, 0.002);
            opacity.retarget(0.0, CONTENT_OUT);
            self.leaving = Some(Leaving {
                node: self.current().clone(),
                sub: !self.path.is_empty(),
                width: self.width,
                opacity,
            });
        } else if let Some(leaving) = &mut self.leaving {
            leaving.opacity.retarget(0.0, CONTENT_OUT);
        }
        self.content.reset(0.0);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.h.retarget(0.0, SHAPE_SPRING);
        self.w.retarget(self.w.target() * BULGE, SHAPE_SPRING);
        self.content.retarget(0.0, CONTENT_OUT);
        if let Some(leaving) = &mut self.leaving {
            leaving.opacity.retarget(0.0, CONTENT_OUT);
        }
    }

    pub fn is_gone(&self) -> bool {
        !self.open && self.h.value() <= 0.5 && self.is_settled()
    }

    pub fn is_settled(&self) -> bool {
        self.w.is_settled()
            && self.h.is_settled()
            && self.anchor.is_settled()
            && self.content.is_settled()
            && self.leaving.is_none()
            && (!self.open || self.h.target() > 0.0)
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.w.advance(dt, now);
        self.h.advance(dt, now);
        self.anchor.advance(dt, now);
        self.content.advance(dt, now);
        if let Some(leaving) = &mut self.leaving {
            leaving.opacity.advance(dt, now);
            if leaving.opacity.is_settled() {
                self.leaving = None;
            }
        }
    }

    pub fn enter(&mut self, index: usize) {
        if self
            .current()
            .children
            .get(index)
            .is_some_and(|node| !node.children.is_empty())
        {
            self.swap();
            self.path.push(index);
        }
    }

    pub fn back(&mut self) {
        if !self.path.is_empty() {
            self.swap();
            self.path.pop();
        }
    }

    /// The row under `at` that does something, with where it is.
    pub fn action_at(&self, at: (f32, f32)) -> Option<([f32; 4], Action)> {
        self.rows
            .iter()
            .find(|row| inside(row.rect, at))
            .and_then(|row| row.action.map(|action| (row.rect, action)))
    }

    fn size_of(
        node: &MenuNode,
        sub: bool,
        room: f32,
        painter: &mut Painter,
        scale: f32,
    ) -> (f32, f32) {
        let checks = node.children.iter().any(|child| child.toggle.is_some());
        let lead = if checks { CHECK_W } else { 0.0 };
        let mut widest: f32 = MIN_W - 2.0 * PAD;
        for child in node.children.iter().filter(|child| !child.separator) {
            let line = painter.line(&child.label, SMALL_SIZE, MEDIUM, scale);
            widest = widest.max(line.width + 2.0 * SIDE + lead + 16.0);
        }
        let width = (widest + 2.0 * PAD).min(MAX_W).min(room).ceil();
        let mut total = 2.0 * PAD + if sub { ENTRY_H + SEPARATOR_H } else { 0.0 };
        for child in &node.children {
            total += if child.separator {
                SEPARATOR_H
            } else {
                ENTRY_H
            };
        }
        (width, total)
    }

    /// Hung from `bottom`, within the bar from `left` to `right`.
    pub fn draw(
        &mut self,
        (left, right, bottom): (f32, f32, f32),
        opacity: f32,
        pointer: Option<(f32, f32)>,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) {
        let sub = !self.path.is_empty();
        let (width, total) = Self::size_of(self.current(), sub, right - left, painter, scale);
        self.width = width;
        if self.open {
            if self.h.value() < 0.5 {
                self.w.reset(width * BULGE);
            }
            if self.w.target() != width {
                self.w.retarget(width, SHAPE_SPRING);
            }
            if self.h.target() != total {
                self.h.retarget(total, SHAPE_SPRING);
            }
            if self.content.target() != 1.0 {
                self.content.retarget(1.0, CONTENT_IN);
            }
        }
        let (w, h) = (
            self.w.value().clamp(0.0, right - left),
            self.h.value().max(0.0),
        );
        let x = (self.anchor.value() - w / 2.0).clamp(left, (right - w).max(left));
        self.drawn_anchor = Some(x + w / 2.0);
        let x = (x * scale).round() / scale;
        self.rect = [x, bottom, w, h];
        let clip = [x, bottom, x + w, bottom + h];
        let first = out.len();
        let body = [x, bottom - PANEL_RADIUS, w, h + PANEL_RADIUS];
        out.push(Sprite::shape(
            body,
            color(SURFACE).map(|channel| channel * opacity),
            PANEL_RADIUS,
            2.0,
        ));
        out.push(Sprite::ring(
            body,
            tinted(SURFACE1, SURFACE_BORDER_ALPHA),
            PANEL_RADIUS,
            2.0,
        ));
        let spot = (x, bottom);
        if let Some(leaving) = &self.leaving {
            let shown = leaving.opacity.value().clamp(0.0, 1.0);
            if shown > 0.001 {
                let from = out.len();
                let row_w = leaving.width - 2.0 * PAD;
                Self::paint_rows(
                    &leaving.node,
                    leaving.sub,
                    spot,
                    row_w,
                    None,
                    painter,
                    gpu,
                    scale,
                    out,
                );
                for sprite in &mut out[from..] {
                    *sprite = sprite
                        .moved(0.0, -(1.0 - shown) * CONTENT_DRIFT)
                        .faded(shown);
                }
            }
        }
        let shown = self.content.value().clamp(0.0, 1.0);
        let from = out.len();
        self.rows = Self::paint_rows(
            self.current(),
            sub,
            spot,
            width - 2.0 * PAD,
            pointer,
            painter,
            gpu,
            scale,
            out,
        );
        for sprite in &mut out[from..] {
            *sprite = sprite
                .moved(0.0, -(1.0 - shown) * CONTENT_DRIFT)
                .faded(shown);
        }
        for sprite in &mut out[first..] {
            *sprite = sprite.within(clip);
        }
        let fillet = crate::menu::FILLET.min(h / 2.0);
        if fillet >= 1.0 {
            let side = fillet + LINE;
            let top = bottom - LINE;
            let glass = color(SURFACE).map(|channel| channel * opacity);
            let edge = tinted(SURFACE1, SURFACE_BORDER_ALPHA);
            for (at, right) in [(x - fillet, false), (x + w + fillet - side, true)] {
                let rect = [at, top, side, side];
                out.push(Sprite::fillet(rect, glass, fillet, LINE, right, false));
                out.push(Sprite::fillet(rect, edge, fillet, LINE, right, true));
            }
        }
    }

    /// With no `pointer`, nothing is hovered.
    #[allow(clippy::too_many_arguments)]
    fn paint_rows(
        node: &MenuNode,
        sub: bool,
        (x, bottom): (f32, f32),
        row_w: f32,
        pointer: Option<(f32, f32)>,
        painter: &mut Painter,
        gpu: &Gpu,
        scale: f32,
        out: &mut Vec<Sprite>,
    ) -> Vec<Row> {
        let checks = node.children.iter().any(|child| child.toggle.is_some());
        let mut rows = Vec::new();
        let mut y = bottom + PAD;
        let (inner, text_x) = (x + PAD, x + PAD + SIDE);
        let hovered = |rect: [f32; 4]| pointer.is_some_and(|at| inside(rect, at));
        let hover = tinted(ACCENT, HOVER_ALPHA);
        let text = |painter: &mut Painter,
                    words: &str,
                    size_weight: (f32, u16),
                    at: (f32, f32),
                    tint: u32,
                    out: &mut Vec<Sprite>| {
            let line = painter.line(words, size_weight.0, size_weight.1, scale);
            painter.text(
                gpu,
                &line,
                (at.0, at.1 + (ENTRY_H - size_weight.0 * 1.2) / 2.0),
                color(tint),
                scale,
                out,
            );
        };
        if sub {
            let rect = [inner, y, row_w, ENTRY_H];
            if hovered(rect) {
                out.push(Sprite::shape(rect, hover, CONTROL_RADIUS, 2.0));
            }
            text(
                painter,
                "‹",
                (SMALL_SIZE, MEDIUM),
                (text_x, y),
                SUBTEXT1,
                out,
            );
            text(
                painter,
                &node.label,
                (SMALL_SIZE, SEMIBOLD),
                (text_x + 14.0, y),
                TEXT,
                out,
            );
            rows.push(Row {
                rect,
                action: Some(Action::Back),
            });
            y += ENTRY_H;
            out.push(Sprite::shape(
                [inner, y + 4.0, row_w, 1.0],
                tinted(SURFACE1, CONTROL_ALPHA),
                0.0,
                2.0,
            ));
            y += SEPARATOR_H;
        }
        for (index, child) in node.children.iter().enumerate() {
            if child.separator {
                out.push(Sprite::shape(
                    [inner, y + 4.0, row_w, 1.0],
                    tinted(SURFACE1, CONTROL_ALPHA),
                    0.0,
                    2.0,
                ));
                y += SEPARATOR_H;
                continue;
            }
            let rect = [inner, y, row_w, ENTRY_H];
            if child.enabled && hovered(rect) {
                out.push(Sprite::shape(rect, hover, CONTROL_RADIUS, 2.0));
            }
            let mut at = text_x;
            if checks {
                if child.toggle == Some(true) {
                    text(painter, "✓", (SMALL_SIZE, MEDIUM), (at, y), ACCENT, out);
                }
                at += CHECK_W;
            }
            let tint = if child.enabled { TEXT } else { OVERLAY2 };
            text(
                painter,
                &child.label,
                (SMALL_SIZE, MEDIUM),
                (at, y),
                tint,
                out,
            );
            let submenu = !child.children.is_empty();
            if submenu {
                text(
                    painter,
                    "›",
                    (SMALL_SIZE, MEDIUM),
                    (inner + row_w - SIDE - 6.0, y),
                    SUBTEXT1,
                    out,
                );
            }
            let action = child.enabled.then_some(if submenu {
                Action::Enter(index)
            } else {
                Action::Pick(child.id)
            });
            rows.push(Row { rect, action });
            y += ENTRY_H;
        }
        rows
    }
}
