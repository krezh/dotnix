use super::{Cards, Notifications, State};
use crate::{
    bar_widget::{BarWidget, Part},
    features::{Surface, WidgetCx},
    theme::{ACCENT, OVERLAY1},
    units::plural,
};

impl Notifications {
    pub(super) fn widget_view(&self, state: &State, w: &WidgetCx<'_>) -> Option<BarWidget> {
        let missed = state.missed.len();
        let color = if missed > 0 && !state.dnd {
            ACCENT
        } else {
            OVERLAY1
        };
        let icon = w
            .local::<Cards>()
            .map(|cards| Part::Drawing(cards.dots.drawing(color)));
        let tip = match (missed, state.dnd) {
            (0, false) => "No missed notifications".to_owned(),
            (0, true) => "Do not disturb is on".to_owned(),
            (missed, dnd) => format!(
                "{}{}",
                plural(missed, "missed notification", "missed notifications"),
                if dnd { " · do not disturb" } else { "" }
            ),
        };
        Some(
            w.base()
                .tip(tip)
                .parts(icon)
                .on_click(w.toggle(Self::panel_handle())),
        )
    }
}
