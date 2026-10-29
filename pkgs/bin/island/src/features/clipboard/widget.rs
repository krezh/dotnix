use gpui::{AnyElement, prelude::*};

use super::runtime::Clipboard;
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    features::WidgetCx,
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
