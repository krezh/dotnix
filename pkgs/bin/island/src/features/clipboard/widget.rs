use super::runtime::Clipboard;
use crate::{
    bar_widget::BarWidget,
    features::{Surface, WidgetCx},
    theme::TEXT,
};

pub(super) fn widget(w: &mut WidgetCx<'_>) -> Option<BarWidget> {
    Some(
        w.base("bar-clipboard")
            .tip("Clipboard")
            .glyph(w.glyph("clipboard"), TEXT)
            .on_click(w.toggle(Clipboard::panel_handle())),
    )
}
