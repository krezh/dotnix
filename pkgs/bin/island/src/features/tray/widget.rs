use super::{
    Tray,
    icons::{ICON, Icons},
    sni::Command,
};
use crate::{
    bar_widget::{BarWidget, Frame, Part, Pointer},
    features::WidgetCx,
    panel::Ui,
    theme::PEACH,
};

/// The line under an icon that wants attention.
const LINE_SHORT: f32 = 6.0;
const LINE_LONG: f32 = 16.0;
const LINE_H: f32 = 3.0;
const LINE_LIFT: f32 = 3.0;

impl Tray {
    /// Where an app that wants attention is in its breath, from 0 to 1 and back, easing at both
    /// ends.
    fn breath(&self) -> f32 {
        const CYCLE: f32 = 1.8;
        let phase = self.epoch.elapsed().as_secs_f32() / CYCLE * std::f32::consts::TAU;
        0.5 - 0.5 * phase.cos()
    }

    pub(super) fn widget_view(&self, w: &WidgetCx<'_>) -> Option<BarWidget> {
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
            let mut widget = w
                .base()
                .width((ICON + 2.0 * half) * presence)
                .padding(half * presence)
                .clip()
                .opacity(presence);
            if !item.title.is_empty() {
                widget = widget.tip(item.title.clone());
            }
            widget = match &item.icon {
                Some(picture) => widget.part(Part::Picture(picture.clone(), ICON)),
                None => widget.small().part(Part::plain(
                    item.title
                        .chars()
                        .next()
                        .map(String::from)
                        .unwrap_or_default(),
                )),
            };
            // A short line under the icon that slowly stretches and settles.
            if item.attention {
                widget = widget.part(Part::Underline {
                    width: LINE_SHORT + (LINE_LONG - LINE_SHORT) * breath,
                    height: LINE_H,
                    lift: LINE_LIFT,
                    opacity: 0.55 + 0.45 * breath,
                    color: PEACH,
                });
            }
            if opens_menu {
                widget = widget.holds_press();
            }
            let on_right = if has_menu {
                w.on_island(move |tray: &Tray, pointer: Pointer, ui: &mut dyn Ui| {
                    ui.stop_propagation();
                    tray.open_menu(menu_key.clone(), pointer.x, ui);
                })
            } else {
                w.on_island(move |tray: &Tray, pointer: Pointer, ui: &mut dyn Ui| {
                    let (x, y) = ui.below_bar(pointer.x).unwrap_or_default();
                    tray.send(Command::ContextMenu(menu_key.clone(), x, y));
                })
            };
            Part::Widget(
                widget
                    .on_click(
                        w.on_island(move |tray: &Tray, pointer: Pointer, ui: &mut dyn Ui| {
                            if opens_menu {
                                tray.open_menu(click_key.clone(), pointer.x, ui);
                            } else {
                                tray.send(Command::Activate(click_key.clone()));
                            }
                        }),
                    )
                    .on_right(on_right)
                    .on_middle(w.call(move |tray: &Tray| {
                        tray.send(Command::Secondary(middle_key.clone()));
                    })),
            )
        });
        Some(BarWidget::new(Frame::Row).parts(slots))
    }
}
