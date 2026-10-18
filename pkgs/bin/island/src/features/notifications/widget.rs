use gpui::{AnyElement, prelude::*};

use super::{Cards, Notifications, State};
use crate::{
    components::tooltip::Tip,
    features::{Feature, WidgetCx},
    island::Panel,
    theme::{ACCENT, OVERLAY1},
};

impl Notifications {
    pub(super) fn widget_view(&self, state: &State, w: &WidgetCx<'_>) -> Option<AnyElement> {
        let missed = state.missed.len();
        let color = if missed > 0 && !state.dnd {
            ACCENT
        } else {
            OVERLAY1
        };
        let icon = w.local::<Cards>().map(|cards| cards.dots.element(color));
        let tip = match (missed, state.dnd) {
            (0, false) => "No missed notifications".to_owned(),
            (0, true) => "Do not disturb is on".to_owned(),
            (missed, dnd) => format!(
                "{missed} missed notification{}{}",
                if missed == 1 { "" } else { "s" },
                if dnd { " · do not disturb" } else { "" }
            ),
        };
        Some(
            w.base("bar-notifications")
                .tip(w.tips(), "bar-notifications", tip)
                .children(icon)
                .on_click(w.toggle(Panel::Feature(Self::NAME)))
                .into_any_element(),
        )
    }
}
