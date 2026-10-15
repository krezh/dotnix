//! Caffeine: keeps the screen awake while on. A badge in the bar while on (clicking it turns caffeine
//! off), a System-panel toggle, and `island ctl caffeine on|off|toggle`.

use std::cell::Cell;

use gpui::{AnyElement, div, prelude::*, px, rgb};

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    control::{self, Switch},
    panel::IconButton,
    theme::ACCENT,
};

pub struct Caffeine {
    feed: Feed<bool>,
    on: Cell<bool>,
}

impl Caffeine {
    fn set(&self, switch: Switch) {
        let on = switch.apply(self.on.get());
        if on != self.on.replace(on) {
            tracing::info!(on, "caffeine changed");
            self.feed.publish(on);
        }
    }

    fn toggle(&self) {
        self.set(Switch::Toggle);
    }
}

impl Feature for Caffeine {
    const NAME: &'static str = "caffeine";
    const GLYPHS: &'static [GlyphSpec] = &[("caffeine", &["caffeine-cup-full-symbolic", "caffeine-cup-full"])];
    const USAGE: &'static str = "  caffeine on|off|toggle                keep the screen awake";

    type State = bool;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<bool>) -> Self {
        Self {
            feed,
            on: Cell::new(false),
        }
    }

    fn present(_: &str, on: &bool) -> bool {
        *on
    }

    fn inhibits_idle(on: &bool) -> bool {
        *on
    }

    fn widget(&self, _: &str, _: &bool, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(
            w.base("bar-caffeine")
                .tip(w.tips(), "bar-caffeine", "Caffeine is on")
                .map(|badge| match w.glyph("caffeine") {
                    Some(path) => badge.child(icon::glyph(path, GLYPH, ACCENT)),
                    None => badge.child(div().size(px(8.0)).rounded_full().bg(rgb(ACCENT))),
                })
                .on_click(w.call(Caffeine::toggle))
                .into_any_element(),
        )
    }

    fn system_button(&self, on: &bool, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(IconButton {
            id: "toggle-caffeine".into(),
            glyph: p.glyph("caffeine"),
            label: if *on { "Caffeine is on" } else { "Caffeine is off" }.into(),
            active: *on,
            badge: None,
            on_click: p.call(Caffeine::toggle),
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        control::switch(&args[1..]).map(drop)
    }

    fn command(&self, _: &bool, args: &[&str]) {
        if let Ok(switch) = control::switch(&args[1..]) {
            self.set(switch);
        }
    }
}
