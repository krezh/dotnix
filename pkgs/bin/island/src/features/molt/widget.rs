use super::{runtime::Molt, state::Updates};
use crate::{
    bar_widget::{BarWidget, Part},
    features::{Surface, WidgetCx},
    theme::{ACCENT, CRITICAL, GREEN, SUBTEXT1},
};

/// With no incoming changes, the ready watched pull requests show in green, since an update now
/// brings their fixes.
pub(super) fn widget(state: &Updates, w: &WidgetCx<'_>) -> Option<BarWidget> {
    let (text, color) = match (&state.error, state.incoming(), state.ready()) {
        (Some(_), _, _) => ("!".to_owned(), CRITICAL),
        (None, 0, ready) if ready > 0 => (ready.to_string(), GREEN),
        (None, incoming, _) if state.report.is_some() => (incoming.to_string(), ACCENT),
        _ => (String::new(), SUBTEXT1),
    };
    let tip = match (&state.error, &state.report) {
        (Some(error), _) => format!("molt check failed: {error}"),
        (None, Some(_)) => state.summary(),
        (None, None) => "Molt".to_owned(),
    };
    Some(
        w.compact("bar-molt")
            .color(color)
            .tip(tip)
            .glyph(w.glyph("molt"), color)
            .part(Part::count(text))
            .part(Part::Busy(state.refreshing))
            .on_click(w.toggle(Molt::panel_handle()))
            .on_right(w.call(Molt::refresh)),
    )
}
