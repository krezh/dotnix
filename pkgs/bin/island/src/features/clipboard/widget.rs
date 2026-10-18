use gpui::{AnyElement, prelude::*};

use super::runtime::Clipboard;
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    features::{PanelCx, WidgetCx},
    island::Mode,
    panel::{IconButton, on},
    theme::TEXT,
};

pub(super) fn widget(w: &mut WidgetCx<'_>) -> Option<AnyElement> {
    Some(
        w.base("bar-clipboard")
            .tip(w.tips(), "bar-clipboard", "Clipboard")
            .when_some(w.glyph("clipboard"), |button, path| {
                button.child(icon::glyph(path, GLYPH, TEXT))
            })
            .on_click(w.toggle(Clipboard::panel_handle()))
            .into_any_element(),
    )
}

pub(super) fn system_button(p: &PanelCx<'_>) -> Option<IconButton> {
    Some(IconButton {
        id: "open-clipboard".into(),
        glyph: p.glyph("clipboard"),
        rising: None,
        label: "Clipboard".into(),
        active: false,
        badge: None,
        on_click: on(|island, cx| island.set_mode(Mode::Open(Clipboard::panel_handle()), cx)),
    })
}
