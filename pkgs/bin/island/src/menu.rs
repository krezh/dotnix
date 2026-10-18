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
    island::{CONTENT_IN, CONTENT_OUT, Island, REVEAL_DELAY, SHAPE_SPRING},
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
    let enabled = entry.enabled && entry.on_click.is_some();
    let mark = entry.check.map(|on| if on { "✓" } else { "" });
    button::ghost(entry.id, hover && enabled)
        .h(px(ENTRY_H))
        .px(px(8.0))
        .justify_start()
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .text_color(rgb(if enabled { TEXT } else { OVERLAY2 }))
        .when(!enabled, |row| row.cursor_default())
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

pub struct Menu {
    source: String,
    root: MenuNode,
    path: Vec<usize>,
    pick: Pick,
    anchor: f32,
    open: bool,
    limit: f32,
    w: Spring,
    h: Spring,
    content: Spring,
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
            anchor,
            open: true,
            limit: MAX_W,
            w: Spring::new(0.0, SHAPE_SPRING, 0.1),
            h: Spring::new(0.0, SHAPE_SPRING, 0.1),
            content: Spring::new(0.0, CONTENT_OUT, 0.002),
            size: measure::measured(),
            pending: Cell::new(None),
            frozen: None,
        }
    }

    pub fn replace(&mut self, source: String, root: MenuNode, pick: Pick, anchor: f32) {
        self.source = source;
        self.root = root;
        self.path.clear();
        self.pick = pick;
        self.anchor = anchor;
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

    pub fn enter(&mut self, index: usize) {
        if self
            .current()
            .children
            .get(index)
            .is_some_and(|node| !node.children.is_empty())
        {
            self.path.push(index);
            self.grow();
        }
    }

    pub fn back(&mut self) {
        self.path.pop();
        self.grow();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.h.retarget(0.0, SHAPE_SPRING);
        self.w.retarget(self.w.target() * BULGE, SHAPE_SPRING);
        self.content.retarget(0.0, CONTENT_OUT);
    }

    pub fn is_gone(&self) -> bool {
        !self.open && self.h.value() <= 0.0 && self.is_settled()
    }

    pub fn is_settled(&self) -> bool {
        self.w.is_settled() && self.h.is_settled() && self.content.is_settled()
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.w.advance(dt, now);
        self.h.advance(dt, now);
        self.content.advance(dt, now);
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
        let x = (self.anchor - w / 2.0).clamp(left, (right - w).max(left));
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
        let content = self.contents(hover, cx);
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
        div()
            .absolute()
            .left(hang.bounds.origin.x)
            .top(hang.bounds.origin.y)
            .w(hang.bounds.size.width)
            .h(hang.bounds.size.height)
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(MAX_W))
                    .flex()
                    .items_start()
                    .opacity(self.content.value().clamp(0.0, 1.0))
                    .child(content),
            )
            .into_any_element()
    }

    fn contents(&self, hover: bool, cx: &mut Context<Island>) -> AnyElement {
        let node = self.current();
        let checks = node.children.iter().any(|child| child.toggle.is_some());
        let back = (!self.path.is_empty()).then(|| {
            button::ghost("menu-back", hover)
                .h(px(ENTRY_H))
                .px(px(8.0))
                .justify_start()
                .gap(px(8.0))
                .text_size(px(SMALL_SIZE))
                .child(div().flex_none().text_color(rgb(SUBTEXT1)).child("‹"))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(node.label.clone()),
                )
                .on_click(cx.listener(|island, _, _, cx| island.menu_back(cx)))
                .into_any_element()
        });
        let separator = back.is_some().then(|| {
            let line = Entry {
                id: "menu-back-line".into(),
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
                    id: format!("menu-{id}-{index}").into(),
                    label: child.label.clone().into(),
                    enabled: child.enabled,
                    separator: child.separator,
                    check: child.toggle,
                    submenu,
                    on_click: Some(if submenu {
                        panel::on(move |island, cx| island.menu_enter(index, cx))
                    } else {
                        panel::on(move |island, cx| island.menu_pick(id, cx))
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
