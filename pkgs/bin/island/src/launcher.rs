use std::{collections::HashMap, sync::Arc};

use gpui::{
    Bounds, Context, Entity, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, MouseDownEvent, Pixels, Render, Size,
    Subscription, WeakEntity, Window, div, point, prelude::*, px,
};

use crate::{
    desktop::{self, App},
    island::{Island, SURFACE_W},
    launch, launches,
    shell::Shell,
};

pub const ROWS: usize = 8;

pub struct Search {
    pub apps: Arc<Vec<App>>,
    /// How much each app is used, by desktop id, as it was when the launcher opened.
    ranks: HashMap<String, f64>,
    pub query: String,
    pub results: Vec<usize>,
    pub selected: usize,
    /// The first result shown.
    pub top: usize,
}

/// The first of `count` rows to show so that `selected` stays in view, moving on from `top` as
/// little as it can, so the list only scrolls when the selection reaches its edge.
pub fn reveal(selected: usize, top: usize, count: usize) -> usize {
    let top = top.min(count.saturating_sub(ROWS));
    if selected < top {
        selected
    } else if selected >= top + ROWS {
        selected + 1 - ROWS
    } else {
        top
    }
}

/// `selected` moved by `step` rows, kept within `count`.
pub fn stepped(selected: usize, step: i32, count: usize) -> usize {
    let moved = i64::try_from(selected).unwrap_or(i64::MAX).saturating_add(i64::from(step));
    usize::try_from(moved.max(0)).unwrap_or(0).min(count.saturating_sub(1))
}

pub enum Outcome {
    Open,
    Close,
}

impl Search {
    pub fn new(apps: Arc<Vec<App>>) -> Self {
        let ranks = launches::ranks();
        let results = desktop::search(&apps, "", &ranks);
        Self {
            apps,
            ranks,
            query: String::new(),
            results,
            selected: 0,
            top: 0,
        }
    }

    fn refresh(&mut self) {
        self.results = desktop::search(&self.apps, &self.query, &self.ranks);
        self.selected = 0;
        self.top = 0;
    }

    pub fn select(&mut self, position: usize) {
        self.selected = position.min(self.results.len().saturating_sub(1));
        self.top = reveal(self.selected, self.top, self.results.len());
    }

    pub fn step(&mut self, step: i32) {
        self.select(stepped(self.selected, step, self.results.len()));
    }

    pub fn visible(&self) -> impl Iterator<Item = (usize, &App)> {
        self.results
            .iter()
            .enumerate()
            .skip(self.top)
            .take(ROWS)
            .filter_map(|(position, index)| Some((position, self.apps.get(*index)?)))
    }

    pub fn launch(&self, position: usize) {
        let app = self.results.get(position).and_then(|index| self.apps.get(*index));
        match app.map(|app| (app, desktop::launch(app))) {
            Some((app, Some(launch))) => {
                launches::record(&app.id);
                launch::spawn(launch);
            }
            Some((app, None)) => tracing::error!(app = app.name, exec = app.exec, "cannot launch: unusable Exec line"),
            None => {}
        }
    }

    pub fn key(&mut self, keystroke: &Keystroke) -> Outcome {
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                self.launch(self.selected);
                return Outcome::Close;
            }
            "up" => self.step(-1),
            "down" | "tab" => self.step(1),
            "backspace" => {
                self.query.pop();
                self.refresh();
            }
            "u" if keystroke.modifiers.control => {
                self.query.clear();
                self.refresh();
            }
            _ => {
                let typed = keystroke
                    .key_char
                    .as_deref()
                    .filter(|_| !keystroke.modifiers.control && !keystroke.modifiers.alt)
                    .filter(|text| text.chars().all(|char| !char.is_control()));
                if let Some(text) = typed {
                    self.query.push_str(text);
                    self.refresh();
                }
            }
        }
        Outcome::Open
    }
}

/// A transparent surface over the whole output of the island whose panel takes the keyboard: it
/// holds the keyboard, and a click anywhere but on the island closes the panel. Its input region
/// leaves a hole where the island takes input, following it as it morphs. It asks for the keyboard
/// on demand rather than exclusively: Hyprland sends all pointer input to a surface holding the
/// keyboard exclusively, which would leave the island deaf to the mouse. Covering the rest of the
/// output keeps the pointer off windows, so focus-follows-mouse never takes the keyboard away.
pub struct KeyGrab {
    shell: WeakEntity<Shell>,
    island: WeakEntity<Island>,
    focus: FocusHandle,
    focused: bool,
    region: Vec<Bounds<Pixels>>,
    /// A key that acts once it is let go, while it is held: see [`ON_RELEASE`].
    held: Option<Keystroke>,
    _island_changed: Subscription,
}

/// Keys that usually close the panel, which act when let go rather than when pressed. Closing hands
/// the keyboard back to the window below, and a key still held then reaches it: Wayland tells a
/// window that gains focus which keys are down, so it would see the Escape or Enter too.
const ON_RELEASE: [&str; 2] = ["escape", "enter"];

impl KeyGrab {
    pub fn new(shell: WeakEntity<Shell>, island: &Entity<Island>, cx: &mut Context<Self>) -> Self {
        Self {
            shell,
            island: island.downgrade(),
            focus: cx.focus_handle(),
            focused: false,
            region: Vec::new(),
            held: None,
            _island_changed: cx.observe(island, |_, _, cx| cx.notify()),
        }
    }
}

/// All of a `viewport`-sized surface except `hole`, given in the coordinates of the island's
/// surface, which is `SURFACE_W` wide and centred at the top of the same output.
fn outside(viewport: Size<Pixels>, hole: Option<Bounds<Pixels>>) -> Vec<Bounds<Pixels>> {
    let corner = |x, y| point(x, y);
    let full = Bounds::from_corners(corner(px(0.0), px(0.0)), corner(viewport.width, viewport.height));
    let Some(hole) = hole else {
        return vec![full];
    };
    let left = (viewport.width - px(SURFACE_W)) / 2.0 + hole.origin.x;
    let right = left + hole.size.width;
    let top = hole.origin.y;
    let bottom = top + hole.size.height;
    vec![
        Bounds::from_corners(corner(px(0.0), px(0.0)), corner(viewport.width, top)),
        Bounds::from_corners(corner(px(0.0), bottom), corner(viewport.width, viewport.height)),
        Bounds::from_corners(corner(px(0.0), top), corner(left, bottom)),
        Bounds::from_corners(corner(right, top), corner(viewport.width, bottom)),
    ]
}

impl Render for KeyGrab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused {
            self.focus.focus(window, cx);
            self.focused = true;
        }
        let hole = self.island.upgrade().and_then(|island| island.read(cx).input_region());
        let region = outside(window.viewport_size(), hole);
        if region != self.region {
            window.set_input_region(Some(&region));
            self.region = region;
        }
        div()
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let keystroke = event.keystroke.clone();
                if ON_RELEASE.contains(&keystroke.key.as_str()) {
                    this.held.get_or_insert(keystroke);
                    return;
                }
                let _ = this.shell.update(cx, |shell, cx| shell.key(&keystroke, cx));
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _, cx| {
                let Some(keystroke) = this.held.take_if(|held| held.key == event.keystroke.key) else {
                    return;
                };
                let _ = this.shell.update(cx, |shell, cx| shell.key(&keystroke, cx));
            }))
            .on_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                let _ = this.shell.update(cx, |shell, cx| shell.close_keyboard_panel(cx));
            }))
    }
}

/// A transparent surface over another output while a panel takes the keyboard: a click anywhere on
/// it closes the panel, as a click outside the island on its own output does.
pub struct ClickCatcher {
    shell: WeakEntity<Shell>,
}

impl ClickCatcher {
    pub fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }
}

impl Render for ClickCatcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().on_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
            let _ = this.shell.update(cx, |shell, cx| shell.close_keyboard_panel(cx));
        }))
    }
}

#[cfg(test)]
#[path = "launcher_test.rs"]
mod tests;
