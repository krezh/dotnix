//! The island's own menus, such as a tray item's: they hang below the bar instead of morphing the
//! island into a panel. Only the island opens them; panels and features never use their rows.

use std::{
    cell::Cell,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
    time::Instant,
};

use gpui::{
    AnyElement, Bounds, Context, FontWeight, Pixels, SharedString, Size, div, point, prelude::*,
    px, rgb, rgba, size,
};

use crate::{
    components::{
        button,
        measure::{self, Measured},
    },
    island::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, Island, REVEAL_DELAY, SHAPE_SPRING},
    panel::{self, Handler},
    shape::Hang,
    spring::Spring,
    theme::{ACCENT, CONTROL_ALPHA, OVERLAY2, SUBTEXT1, SURFACE1, TEXT, with_alpha},
    ui::{self, CONTROL_RADIUS, PANEL_RADIUS, Radius, SMALL_SIZE},
};

const ENTRY_H: f32 = 28.0;

struct Entry {
    id: SharedString,
    label: SharedString,
    enabled: bool,
    separator: bool,
    check: Option<bool>,
    submenu: bool,
    on_click: Option<Handler>,
}

fn entry_element(entry: Entry, checks: bool, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    if entry.separator {
        return div()
            .py(px(4.0))
            .child(
                div()
                    .h(px(1.0))
                    .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))),
            )
            .into_any_element();
    }
    let clickable = entry.enabled && entry.on_click.is_some();
    let mark = entry.check.map(|on| if on { "✓" } else { "" });
    button::ghost(entry.id, hover && clickable)
        .h(px(ENTRY_H))
        .px(px(8.0))
        .justify_start()
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .text_color(rgb(if entry.enabled { TEXT } else { OVERLAY2 }))
        .when(!clickable, |row| row.cursor_default())
        .when(checks, |row| {
            row.child(
                div()
                    .w(px(14.0))
                    .flex_none()
                    .text_color(rgb(ACCENT))
                    .child(mark.unwrap_or_default()),
            )
        })
        .child(div().flex_1().min_w(px(0.0)).truncate().child(entry.label))
        .when(entry.submenu, |row| {
            row.child(div().text_color(rgb(SUBTEXT1)).child("›"))
        })
        .when_some(entry.on_click.filter(|_| entry.enabled), |row, handler| {
            row.on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        })
        .into_any_element()
}

/// Keeps the entries' hover pills concentric with the menu's corners.
const PAD: f32 = PANEL_RADIUS - CONTROL_RADIUS;
pub const MAX_W: f32 = 280.0;
const MIN_W: f32 = 160.0;
pub const FILLET: f32 = 10.0;
/// The share of its width the menu has when it starts to bulge out, and as it goes.
const BULGE: f32 = 0.6;

#[derive(Clone, Debug)]
pub struct MenuNode {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub toggle: Option<bool>,
    pub children: Vec<MenuNode>,
}

pub type Pick = Rc<dyn Fn(i32, &mut Island, &mut Context<Island>)>;

struct Leaving {
    node: MenuNode,
    sub: bool,
    size: Size<Pixels>,
    opacity: Spring,
}

pub struct Menu {
    source: String,
    root: MenuNode,
    path: Vec<usize>,
    pick: Pick,
    anchor: Spring,
    drawn_anchor: Cell<Option<f32>>,
    open: bool,
    limit: f32,
    w: Spring,
    h: Spring,
    content: Spring,
    leaving: Option<Leaving>,
    size: Measured,
    pending: Cell<Option<u64>>,
    frozen: Option<(u64, Size<Pixels>)>,
}

impl Menu {
    pub fn new(source: String, root: MenuNode, pick: Pick, anchor: f32) -> Self {
        Self {
            source,
            root,
            path: Vec::new(),
            pick,
            anchor: Spring::new(anchor, SHAPE_SPRING, 0.1),
            drawn_anchor: Cell::new(None),
            open: true,
            limit: MAX_W,
            w: Spring::new(0.0, SHAPE_SPRING, 0.1),
            h: Spring::new(0.0, SHAPE_SPRING, 0.1),
            content: Spring::new(0.0, CONTENT_OUT, 0.002),
            leaving: None,
            size: measure::measured(),
            pending: Cell::new(None),
            frozen: None,
        }
    }

    pub fn replace(&mut self, source: String, root: MenuNode, pick: Pick, anchor: f32) {
        let was_open = self.open;
        if self.h.value() >= 0.5 {
            self.swap();
            if !was_open {
                self.w.retarget(self.w.value(), SHAPE_SPRING);
                self.h.retarget(self.h.value(), SHAPE_SPRING);
            }
            if self.anchor.is_settled()
                && let Some(drawn) = self.drawn_anchor.get()
            {
                self.anchor.reset(drawn);
            }
            self.anchor.retarget(anchor, SHAPE_SPRING);
        } else {
            self.leaving = None;
            self.frozen = None;
            self.pending.set(None);
            self.size.set(None);
            self.w.reset(0.0);
            self.h.reset(0.0);
            self.anchor.reset(anchor);
            self.drawn_anchor.set(None);
            self.content.reset(0.0);
        }
        self.source = source;
        self.root = root;
        self.path.clear();
        self.pick = pick;
        self.open = true;
        self.grow();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_open_for(&self, source: &str) -> bool {
        self.open && self.source == source
    }

    pub fn pick(&self) -> Pick {
        self.pick.clone()
    }

    fn current(&self) -> &MenuNode {
        self.path.iter().fold(&self.root, |node, index| {
            node.children.get(*index).unwrap_or(node)
        })
    }

    fn swap(&mut self) {
        let current = self.content.value();
        let keep_leaving = self
            .leaving
            .as_ref()
            .is_some_and(|leaving| leaving.opacity.value() >= current);
        if !keep_leaving
            && current > 0.001
            && let Some((_, size)) = self.frozen.take()
        {
            let mut opacity = Spring::new(current, CONTENT_OUT, 0.002);
            opacity.retarget(0.0, CONTENT_OUT);
            self.leaving = Some(Leaving {
                node: self.current().clone(),
                sub: !self.path.is_empty(),
                size,
                opacity,
            });
        } else {
            self.frozen = None;
            if let Some(leaving) = &mut self.leaving {
                leaving.opacity.retarget(0.0, CONTENT_OUT);
            }
        }
        self.pending.set(None);
        self.size.set(None);
        self.content.reset(0.0);
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
            self.grow();
        }
    }

    pub fn back(&mut self) {
        if !self.path.is_empty() {
            self.swap();
            self.path.pop();
            self.grow();
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pending.set(None);
        self.h.retarget(0.0, SHAPE_SPRING);
        self.w.retarget(self.w.target() * BULGE, SHAPE_SPRING);
        self.content.retarget(0.0, CONTENT_OUT);
        if let Some(leaving) = &mut self.leaving {
            leaving.opacity.retarget(0.0, CONTENT_OUT);
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.get().is_some()
    }

    pub fn is_gone(&self) -> bool {
        !self.open && self.h.value() <= 0.0 && self.is_settled()
    }

    pub fn is_settled(&self) -> bool {
        (!self.open || self.frozen.is_some())
            && self.w.is_settled()
            && self.h.is_settled()
            && self.anchor.is_settled()
            && self.content.is_settled()
            && self.leaving.is_none()
            && !self.is_pending()
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

    pub fn set_limit(&mut self, limit: f32) {
        let limit = limit.clamp(0.0, MAX_W).floor();
        if limit != self.limit {
            self.limit = limit;
            self.grow();
        }
    }

    fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.limit.to_bits().hash(&mut hasher);
        for node in &self.current().children {
            node.label.hash(&mut hasher);
            (
                node.separator,
                node.toggle.is_some(),
                node.children.is_empty(),
            )
                .hash(&mut hasher);
        }
        hasher.finish()
    }

    /// True when the measurement set the menu moving.
    pub fn collect(&mut self) -> bool {
        if let (Some(size), Some(key)) = (self.size.take(), self.pending.take()) {
            self.frozen = Some((key, size));
            return self.grow();
        }
        false
    }

    fn grow(&mut self) -> bool {
        let Some((key, size)) = self.frozen else {
            return false;
        };
        if !self.open || key != self.key() {
            return false;
        }
        let (w, h) = (f32::from(size.width).ceil(), f32::from(size.height).ceil());
        if self.h.value() < 0.5 {
            self.w.reset(w * BULGE);
        }
        self.w.retarget(w, SHAPE_SPRING);
        self.h.retarget(h, SHAPE_SPRING);
        if self.content.target() != 1.0 {
            if self.content.value() < 0.05 {
                self.content
                    .retarget_at(1.0, CONTENT_IN, Instant::now() + REVEAL_DELAY);
            } else {
                self.content.retarget(1.0, CONTENT_IN);
            }
        }
        true
    }

    /// Kept to the straight part of `body`'s bottom edge, clear of its rounded ends.
    pub fn hang(&self, body: Bounds<Pixels>, radius: f32, scale: f32) -> Hang {
        let h = self.h.value().max(0.0);
        let fillet = FILLET.min(h / 2.0);
        let left = f32::from(body.origin.x) + radius + fillet;
        let right = f32::from(body.right()) - radius - fillet;
        let w = self.w.value().clamp(0.0, (right - left).max(0.0));
        let x = (self.anchor.value() - w / 2.0).clamp(left, (right - w).max(left));
        self.drawn_anchor.set(Some(x + w / 2.0));
        let x = (x * scale).round() / scale;
        let (corner, power) = Radius::Panel.geometry();
        Hang {
            bounds: Bounds::new(point(px(x), body.bottom()), size(px(w), px(h))),
            radius: corner,
            power,
            fillet,
        }
    }

    pub fn element(&self, hang: &Hang, hover: bool, cx: &mut Context<Island>) -> AnyElement {
        let key = self.key();
        let frozen = self.frozen.filter(|(frozen, _)| *frozen == key);
        let content = self.contents(self.current(), !self.path.is_empty(), hover, true, cx);
        let content = match frozen {
            Some((_, size)) => div()
                .flex_none()
                .w(size.width)
                .h(size.height)
                .child(content)
                .into_any_element(),
            None => {
                self.pending.set(Some(key));
                div()
                    .relative()
                    .flex_none()
                    .child(content)
                    .child(measure::probe(&self.size))
                    .into_any_element()
            }
        };
        let leaving = self
            .leaving
            .as_ref()
            .filter(|leaving| leaving.opacity.value() > 0.001)
            .map(|leaving| {
                let opacity = leaving.opacity.value().clamp(0.0, 1.0);
                let content = self.contents(&leaving.node, leaving.sub, false, false, cx);
                layer(
                    div()
                        .flex_none()
                        .w(leaving.size.width)
                        .h(leaving.size.height)
                        .child(content)
                        .into_any_element(),
                    opacity,
                )
            });
        div()
            .absolute()
            .left(hang.bounds.origin.x)
            .top(hang.bounds.origin.y)
            .w(hang.bounds.size.width)
            .h(hang.bounds.size.height)
            .overflow_hidden()
            .children(leaving)
            .child(layer(content, self.content.value().clamp(0.0, 1.0)))
            .into_any_element()
    }

    fn contents(
        &self,
        node: &MenuNode,
        sub: bool,
        hover: bool,
        interactive: bool,
        cx: &mut Context<Island>,
    ) -> AnyElement {
        let prefix = if interactive { "menu" } else { "leaving-menu" };
        let checks = node.children.iter().any(|child| child.toggle.is_some());
        let back = sub.then(|| {
            button::ghost(format!("{prefix}-back"), hover && interactive)
                .h(px(ENTRY_H))
                .px(px(8.0))
                .justify_start()
                .gap(px(8.0))
                .text_size(px(SMALL_SIZE))
                .when(!interactive, |row| row.cursor_default())
                .child(div().flex_none().text_color(rgb(SUBTEXT1)).child("‹"))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(node.label.clone()),
                )
                .when(interactive, |row| {
                    row.on_click(cx.listener(|island, _, _, cx| island.menu_back(cx)))
                })
                .into_any_element()
        });
        let separator = back.is_some().then(|| {
            let line = Entry {
                id: format!("{prefix}-back-line").into(),
                label: "".into(),
                enabled: false,
                separator: true,
                check: None,
                submenu: false,
                on_click: None,
            };
            entry_element(line, false, hover, cx)
        });
        let entries: Vec<AnyElement> = node
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                let id = child.id;
                let submenu = !child.children.is_empty();
                let entry = Entry {
                    id: format!("{prefix}-{id}-{index}").into(),
                    label: child.label.clone().into(),
                    enabled: child.enabled,
                    separator: child.separator,
                    check: child.toggle,
                    submenu,
                    on_click: interactive.then(|| {
                        if submenu {
                            panel::on(move |island, cx| island.menu_enter(index, cx))
                        } else {
                            panel::on(move |island, cx| island.menu_pick(id, cx))
                        }
                    }),
                };
                entry_element(entry, checks, hover, cx)
            })
            .collect();
        ui::base()
            .flex()
            .flex_col()
            .p(px(PAD))
            .min_w(px(MIN_W.min(self.limit)))
            .max_w(px(self.limit))
            .children(back)
            .children(separator)
            .children(entries)
            .into_any_element()
    }
}

fn layer(content: AnyElement, opacity: f32) -> gpui::Div {
    div()
        .absolute()
        .top(px(-(1.0 - opacity) * CONTENT_DRIFT))
        .left_0()
        .w(px(MAX_W))
        .flex()
        .items_start()
        .opacity(opacity)
        .child(content)
}

#[cfg(test)]
#[path = "menu_test.rs"]
mod tests;
