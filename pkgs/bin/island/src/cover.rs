use gpui::{
    Bounds, Context, Entity, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, MouseDownEvent, Pixels, Render, Size,
    Subscription, WeakEntity, Window, div, point, prelude::*, px,
};

use crate::{
    island::{Island, SURFACE_W},
    shell::Shell,
};

/// Asks for the keyboard on demand rather than exclusively: Hyprland sends all pointer input to a
/// surface holding the keyboard exclusively, which would leave the island deaf to the mouse.
/// Covering the output around the island keeps the pointer off windows, so focus-follows-mouse
/// never takes the keyboard away.
pub struct KeyGrab {
    shell: WeakEntity<Shell>,
    island: WeakEntity<Island>,
    focus: FocusHandle,
    focused: bool,
    region: Vec<Bounds<Pixels>>,
    /// See [`ON_RELEASE`].
    held: Option<Keystroke>,
    _island_changed: Subscription,
}

/// Act when let go rather than when pressed. Closing hands the keyboard back to the window below,
/// and Wayland tells a window that gains focus which keys are down, so it would see the key too.
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

/// `holes` are in the island surface's coordinates; it is centred at the top of the same output.
fn outside(viewport: Size<Pixels>, holes: &[Bounds<Pixels>]) -> Vec<Bounds<Pixels>> {
    let shift = (viewport.width - px(SURFACE_W)) / 2.0;
    let full = Bounds::from_corners(point(px(0.0), px(0.0)), point(viewport.width, viewport.height));
    holes.iter().fold(vec![full], |region, hole| {
        let hole = Bounds::new(point(hole.origin.x + shift, hole.origin.y), hole.size);
        region.into_iter().flat_map(|rect| cut(rect, hole)).collect()
    })
}

fn follow(island: &WeakEntity<Island>, region: &mut Vec<Bounds<Pixels>>, window: &mut Window, cx: &gpui::App) {
    let holes = island.upgrade().map(|island| island.read(cx).input_region().to_vec()).unwrap_or_default();
    let wanted = outside(window.viewport_size(), &holes);
    if wanted != *region {
        window.set_input_region(Some(&wanted));
        *region = wanted;
    }
}

fn cut(rect: Bounds<Pixels>, hole: Bounds<Pixels>) -> Vec<Bounds<Pixels>> {
    let (left, top) = (hole.origin.x.max(rect.origin.x), hole.origin.y.max(rect.origin.y));
    let (right, bottom) = (hole.right().min(rect.right()), hole.bottom().min(rect.bottom()));
    if left >= right || top >= bottom {
        return vec![rect];
    }
    [
        Bounds::from_corners(rect.origin, point(rect.right(), top)),
        Bounds::from_corners(point(rect.origin.x, bottom), point(rect.right(), rect.bottom())),
        Bounds::from_corners(point(rect.origin.x, top), point(left, bottom)),
        Bounds::from_corners(point(right, top), point(rect.right(), bottom)),
    ]
    .into_iter()
    .filter(|part| part.size.width > px(0.0) && part.size.height > px(0.0))
    .collect()
}

impl Render for KeyGrab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused {
            self.focus.focus(window, cx);
            self.focused = true;
        }
        follow(&self.island, &mut self.region, window, cx);
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
                let _ = this.shell.update(cx, |shell, cx| shell.close_grabbed(cx));
            }))
    }
}

/// Like [`KeyGrab`] without the keyboard: a click on it closes what is open.
pub struct ClickCatcher {
    shell: WeakEntity<Shell>,
    island: Option<WeakEntity<Island>>,
    region: Vec<Bounds<Pixels>>,
    _island_changed: Option<Subscription>,
}

impl ClickCatcher {
    pub fn new(shell: WeakEntity<Shell>) -> Self {
        Self {
            shell,
            island: None,
            region: Vec::new(),
            _island_changed: None,
        }
    }

    pub fn around(shell: WeakEntity<Shell>, island: &Entity<Island>, cx: &mut Context<Self>) -> Self {
        Self {
            shell,
            island: Some(island.downgrade()),
            region: Vec::new(),
            _island_changed: Some(cx.observe(island, |_, _, cx| cx.notify())),
        }
    }
}

impl Render for ClickCatcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(island) = &self.island {
            follow(island, &mut self.region, window, cx);
        }
        div().size_full().on_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
            let _ = this.shell.update(cx, |shell, cx| shell.close_grabbed(cx));
        }))
    }
}

#[cfg(test)]
#[path = "cover_test.rs"]
mod tests;
