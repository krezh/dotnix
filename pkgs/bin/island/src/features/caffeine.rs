use std::cell::Cell;

use gpui::{AnyElement, prelude::*};

use super::{Dashboard, Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    control::{self, Command, Group, Report, Row, SWITCH, Shown, Switch, Takes},
    panel::ActionTile,
    theme::{ACCENT, PEACH},
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
    const DASHBOARD_ORDER: u8 = 60;
    const GLYPHS: &'static [GlyphSpec] = &[("caffeine", &["coffee"])];
    const CONTROLS: &'static [Command] = &[Command {
        name: "caffeine",
        about: "Keep the screen awake",
        group: Group::System,
        takes: Takes::Forms {
            forms: SWITCH,
            bare: Some("Show whether it is on"),
        },
        examples: &["island ctl caffeine on"],
    }];

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

    fn dashboard(&self, on: &bool, p: &PanelCx<'_>) -> Dashboard {
        Dashboard {
            switches: vec![ActionTile {
                id: "switch-caffeine".into(),
                glyph: p.glyph("caffeine"),
                label: "Keep awake".into(),
                state: Some(if *on { "On" } else { "Off" }.into()),
                color: PEACH,
                armed: *on,
                on_click: p.call(Caffeine::toggle),
            }],
            ..Dashboard::default()
        }
    }

    fn command(&self, _: &bool, args: &[&str]) -> Report {
        let row = |on| Row::new("Caffeine", Shown::Switch(on));
        if args.len() == 1 {
            return Report::shows(vec![row(self.on.get())]);
        }
        let Ok(switch) = control::switch(&args[1..]) else {
            return Report::default();
        };
        self.set(switch);
        Report::did(row(self.on.get()))
    }

    fn status(&self, on: &bool) -> Vec<Row> {
        vec![Row::new("Caffeine", Shown::Switch(*on))]
    }
}
