use gpui::{AnyElement, FontWeight, MouseButton, prelude::*, px, rgb};

use super::{
    runtime::{Gotify, Request},
    state::State,
};
use crate::{
    bar::{GLYPH, ICON_GAP},
    components::{icon, tooltip::Tip},
    features::{Feature, PanelCx, WidgetCx},
    island::{Mode, Panel},
    panel::{IconButton, on},
    theme::{ACCENT, CRITICAL, RED, SUBTEXT1},
    ui::SMALL_SIZE,
};

pub fn widget(state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
    let count = state.messages.len();
    let color = if state.has_critical() {
        CRITICAL
    } else if state.error.is_some() {
        RED
    } else if count > 0 {
        ACCENT
    } else {
        SUBTEXT1
    };
    Some(
        w.base("bar-gotify")
            .gap(px(ICON_GAP))
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .tip(w.tips(), "bar-gotify", state.summary())
            .when_some(w.glyph("gotify"), |badge, path| {
                badge.child(icon::glyph(path, GLYPH, color))
            })
            .child(w.count(count.to_string()))
            .child(w.energy(state.busy))
            .on_click(w.toggle(Panel::Feature(Gotify::NAME)))
            .on_mouse_down(
                MouseButton::Right,
                w.call(|gotify: &Gotify| gotify.request(Request::Refresh)),
            )
            .into_any_element(),
    )
}

pub fn system_button(state: &State, p: &PanelCx<'_>) -> Option<IconButton> {
    let count = state.messages.len();
    Some(IconButton {
        id: "open-gotify".into(),
        glyph: p.glyph("gotify"),
        rising: None,
        label: state.summary().into(),
        active: count > 0,
        badge: (count > 0).then(|| count.to_string().into()),
        on_click: on(|island, cx| island.set_mode(Mode::Open(Panel::Feature(Gotify::NAME)), cx)),
    })
}
