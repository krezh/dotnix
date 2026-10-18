use std::{cell::Cell, path::PathBuf, sync::OnceLock};

use gpui::{AnyElement, prelude::*};

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    control::{self, Switch},
    icons,
    panel::{IconButton, Rising},
    theme::ACCENT,
};

// Papirus's 16 px caffeine-cup-full, split into the cup and its three wisps so the steam can rise
// out of the cup on its own.
const CUP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M4 6c-2.2091 0-4 1.79-4 4s1.7909 4 4 4c.2425 0 .4843-.02.7227-.07C5.462 15.48 6.958 16 10 16c6 0 6-2 6-10Zm0 2h.0078c.0116 1.6.0522 2.94.1953 3.99-.0674 0-.1352.01-.2031.01-1.1046 0-2-.9-2-2s.8954-2 2-2Z"/></svg>"#;
const WISPS: [&str; 3] = [
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M7.1934 0c-1.0982 1.38-1.4274 1.57-1.0313 2.45.3591.8 1.2869.7.8203 2.55.7664-.9 1.3753-1.54.7735-2.46-.6122-.94-.9475-1.01-.5625-2.54Z"/></svg>"#,
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path transform="translate(3 0)" d="M7.1934 0c-1.0982 1.38-1.4274 1.57-1.0313 2.45.3591.8 1.2869.7.8203 2.55.7664-.9 1.3753-1.54.7735-2.46-.6122-.94-.9475-1.01-.5625-2.54Z"/></svg>"#,
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path transform="translate(6 0)" d="M7.1934 0c-1.0982 1.38-1.4274 1.57-1.0313 2.45.3591.8 1.2869.7.8203 2.55.7664-.9 1.3753-1.54.7735-2.46-.6122-.94-.9475-1.01-.5625-2.54Z"/></svg>"#,
];
/// The cup's rim, which the steam rises out of.
const RIM: f32 = 6.0 / 16.0;

fn cup() -> &'static (PathBuf, Rising) {
    static CUP_PARTS: OnceLock<(PathBuf, Rising)> = OnceLock::new();
    CUP_PARTS.get_or_init(|| {
        let parts = WISPS
            .iter()
            .enumerate()
            .map(|(index, wisp)| icons::bundle(&format!("caffeine-wisp-{index}"), wisp))
            .collect();
        (
            icons::bundle("caffeine-cup", CUP),
            Rising { parts, from: RIM },
        )
    })
}

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
    const GLYPHS: &'static [GlyphSpec] = &[(
        "caffeine",
        &["caffeine-cup-full-symbolic", "caffeine-cup-full"],
    )];
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
                .when_some(w.glyph("caffeine"), |badge, path| {
                    badge.child(icon::glyph(path, GLYPH, ACCENT))
                })
                .on_click(w.call(Caffeine::toggle))
                .into_any_element(),
        )
    }

    fn system_button(&self, on: &bool, p: &PanelCx<'_>) -> Option<IconButton> {
        let (glyph, steam) = cup();
        Some(IconButton {
            id: "toggle-caffeine".into(),
            glyph: Some(glyph.clone()),
            rising: Some(steam.clone()),
            label: if *on {
                "Caffeine is on"
            } else {
                "Caffeine is off"
            }
            .into(),
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
