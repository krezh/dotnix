//! Tray: apps' StatusNotifierItem icons in the bar. Each island animates its own icons in and out,
//! icons that need attention pulse, and an item's menu opens as the island's Menu panel.

mod sni;

use std::{rc::Rc, time::Instant};

use futures::channel::oneshot;
use gpui::{AnyElement, Context, FontWeight, MouseButton, MouseDownEvent, SpringConfig, div, prelude::*, px, rgb};
use tracing::{debug, error, warn};

use super::{Feature, Feed, IslandInfo, Local, WidgetCx};
use crate::{
    bar,
    components::{icon, tooltip::Tip},
    island::Island,
    presence::Presence,
    theme::ACCENT,
    ui::SMALL_SIZE,
};
use sni::{Client, Command, TrayItem};

const ICON: f32 = 18.0;
const MOTION: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

/// The width one icon takes in the bar.
fn step(spacing: f32) -> f32 {
    ICON + 2.0 * bar::half(spacing)
}

/// An island's tray icons, each fading and sliding in or out on its own.
pub struct Icons(Presence<TrayItem>);

impl Default for Icons {
    fn default() -> Self {
        Self(Presence::new(MOTION))
    }
}

impl Local for Icons {
    fn advance(&mut self, dt: f32, now: Instant) {
        self.0.advance(dt, now);
    }

    /// Icons that need attention pulse, so they keep frames coming.
    fn is_settled(&self) -> bool {
        self.0.is_settled() && !self.0.iter().any(|slot| slot.item.attention)
    }

    fn pending_width(&self, spacing: f32) -> f32 {
        self.0.pending_width(step(spacing))
    }
}

#[derive(Clone)]
pub struct Tray {
    client: Option<Client>,
    epoch: Instant,
}

impl Tray {
    fn send(&self, command: Command) {
        if let Some(client) = &self.client {
            client.send(command);
        }
    }

    /// Asks the item for its menu and shows it on `island` once it arrives.
    fn open_menu(&self, key: String, title: String, cx: &mut Context<Island>) {
        debug!(item = key, "opening the tray menu");
        let (reply, response) = oneshot::channel();
        self.send(Command::Menu(key.clone(), reply));
        let tray = self.clone();
        cx.spawn(async move |island, cx| {
            let root = match response.await {
                Ok(Some(root)) => root,
                Ok(None) => return,
                Err(_) => {
                    warn!(item = key, "the tray worker dropped the menu request");
                    return;
                }
            };
            if root.children.is_empty() {
                debug!(item = key, "the tray menu is empty");
                return;
            }
            debug!(item = key, entries = root.children.len(), "showing the tray menu");
            let pick = Rc::new(move |id, _: &mut Island, _: &mut Context<Island>| tray.send(Command::Click(key.clone(), id)));
            let _ = island.update(cx, |island, cx| island.show_menu(title, root, pick, cx));
        })
        .detach();
    }

    /// The breathing opacity of the attention dot.
    fn pulse(&self) -> f32 {
        0.55 + 0.45 * (self.epoch.elapsed().as_secs_f32() * 3.0).sin()
    }
}

impl Feature for Tray {
    const NAME: &'static str = "tray";

    type State = Vec<TrayItem>;
    type Local = Icons;
    type Event = ();

    fn start(feed: Feed<Vec<TrayItem>>) -> Self {
        let client = sni::start(feed).inspect_err(|error| error!("{error}")).ok();
        Self {
            client,
            epoch: Instant::now(),
        }
    }

    fn sync_local(items: &Vec<TrayItem>, icons: &mut Icons, _: &IslandInfo<'_>, animate: bool) {
        let keyed: Vec<(String, TrayItem)> = items.iter().map(|item| (item.key.clone(), item.clone())).collect();
        icons.0.sync(&keyed, animate);
    }

    fn widget(&self, _: &str, _: &Vec<TrayItem>, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let icons = w.local::<Icons>()?;
        if icons.0.iter().next().is_none() {
            return None;
        }
        let half = w.padding();
        let pulse = self.pulse();
        let slots = icons.0.iter().map(|slot| {
            let item = &slot.item;
            let presence = slot.presence.value().clamp(0.0, 1.0);
            let key = item.key.clone();
            let (click_key, menu_key, middle_key) = (key.clone(), key.clone(), key.clone());
            let (click_title, menu_title) = (item.title.clone(), item.title.clone());
            let opens_menu = item.item_is_menu && item.has_menu;
            let has_menu = item.has_menu;
            w.base(format!("tray-{key}"))
                .relative()
                .w(px((ICON + 2.0 * half) * presence))
                .px(px(half * presence))
                .overflow_hidden()
                .opacity(presence)
                .when(!item.title.is_empty(), |slot| slot.tip(w.tips(), format!("tray-{key}"), item.title.clone()))
                .when_some(item.icon.as_ref(), |slot, picture| slot.child(icon::tray(picture, ICON)))
                .when(item.icon.is_none(), |slot| {
                    slot.text_size(px(SMALL_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(item.title.chars().next().map(String::from).unwrap_or_default())
                })
                .when(item.attention, |slot| {
                    slot.child(
                        div()
                            .absolute()
                            .bottom(px(2.0))
                            .size(px(5.0))
                            .rounded_full()
                            .opacity(pulse)
                            .bg(rgb(ACCENT)),
                    )
                })
                .on_click(w.on_island(move |tray: &Tray, _, _, cx| {
                    if opens_menu {
                        tray.open_menu(click_key.clone(), click_title.clone(), cx);
                    } else {
                        tray.send(Command::Activate(click_key.clone()));
                    }
                }))
                .map(|slot| {
                    if has_menu {
                        slot.on_mouse_down(
                            MouseButton::Right,
                            w.on_island(move |tray: &Tray, _, _, cx| tray.open_menu(menu_key.clone(), menu_title.clone(), cx)),
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
        Some(div().flex_none().flex().items_center().children(slots).into_any_element())
    }
}
