use gpui::{
    AnyElement, ClickEvent, FontWeight, MouseButton, MouseDownEvent, ObjectFit, Pixels, div,
    prelude::*, px, relative, rgb,
};

use super::{
    Tray,
    icons::{ICON, Icons},
    sni::Command,
};
use crate::{
    components::{icon, tooltip::Tip},
    features::WidgetCx,
    theme::PEACH,
    ui::SMALL_SIZE,
};

/// The line under an icon that wants attention.
const LINE_SHORT: f32 = 6.0;
const LINE_LONG: f32 = 16.0;
const LINE_H: f32 = 3.0;
const LINE_LIFT: f32 = 3.0;

fn anchor(event: &ClickEvent) -> Pixels {
    match event {
        ClickEvent::Keyboard(event) => event.bounds.center().x,
        event => event.position().x,
    }
}

impl Tray {
    /// Where an app that wants attention is in its breath, from 0 to 1 and back, easing at both
    /// ends.
    fn breath(&self) -> f32 {
        const CYCLE: f32 = 1.8;
        let phase = self.epoch.elapsed().as_secs_f32() / CYCLE * std::f32::consts::TAU;
        0.5 - 0.5 * phase.cos()
    }

    pub(super) fn widget_view(&self, w: &WidgetCx<'_>) -> Option<AnyElement> {
        let icons = w.local::<Icons>()?;
        if icons.0.iter().next().is_none() {
            return None;
        }
        let half = w.padding();
        let breath = self.breath();
        let slots = icons.0.iter().map(|slot| {
            let item = &slot.item;
            let presence = slot.presence.value().clamp(0.0, 1.0);
            let key = item.key.clone();
            let (click_key, menu_key, middle_key) = (key.clone(), key.clone(), key.clone());
            let opens_menu = item.item_is_menu && item.has_menu;
            let has_menu = item.has_menu;
            w.base(format!("tray-{key}"))
                .relative()
                .w(px((ICON + 2.0 * half) * presence))
                .px(px(half * presence))
                .overflow_hidden()
                .opacity(presence)
                .when(!item.title.is_empty(), |slot| {
                    slot.tip(w.tips(), format!("tray-{key}"), item.title.clone())
                })
                .when_some(item.icon.as_ref(), |slot, picture| {
                    slot.child(icon::picture(picture, ICON, ObjectFit::Contain))
                })
                .when(item.icon.is_none(), |slot| {
                    slot.text_size(px(SMALL_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(
                            item.title
                                .chars()
                                .next()
                                .map(String::from)
                                .unwrap_or_default(),
                        )
                })
                // A short line under the icon that slowly stretches and settles.
                .when(item.attention, |slot| {
                    let width = LINE_SHORT + (LINE_LONG - LINE_SHORT) * breath;
                    slot.child(
                        div()
                            .absolute()
                            .bottom(px(LINE_LIFT))
                            .left(relative(0.5))
                            .ml(px(-width / 2.0))
                            .w(px(width))
                            .h(px(LINE_H))
                            .rounded_full()
                            .opacity(0.55 + 0.45 * breath)
                            .bg(rgb(PEACH)),
                    )
                })
                .when(opens_menu, |slot| {
                    slot.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                })
                .on_click(
                    w.on_island(move |tray: &Tray, event: &ClickEvent, island, cx| {
                        if opens_menu {
                            tray.open_menu(click_key.clone(), anchor(event), island, cx);
                        } else {
                            tray.send(Command::Activate(click_key.clone()));
                        }
                    }),
                )
                .map(|slot| {
                    if has_menu {
                        slot.on_mouse_down(
                            MouseButton::Right,
                            w.on_island(move |tray: &Tray, event: &MouseDownEvent, island, cx| {
                                cx.stop_propagation();
                                tray.open_menu(menu_key.clone(), event.position.x, island, cx)
                            }),
                        )
                    } else {
                        slot.on_mouse_down(
                            MouseButton::Right,
                            w.on_island(move |tray: &Tray, event: &MouseDownEvent, island, _| {
                                let (x, y) = island.below_bar(event.position.x).unwrap_or_default();
                                tray.send(Command::ContextMenu(menu_key.clone(), x, y));
                            }),
                        )
                    }
                })
                .on_mouse_down(
                    MouseButton::Middle,
                    w.call(move |tray: &Tray| tray.send(Command::Secondary(middle_key.clone()))),
                )
        });
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .children(slots)
                .into_any_element(),
        )
    }
}
