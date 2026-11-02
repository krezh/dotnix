use std::cell::Cell;

use super::{Dashboard, Feature, Feed, GlyphSpec, PanelCx, Surface, WidgetCx};
use crate::{
    bar_widget::BarWidget,
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

    type Event = ();

    fn start(feed: Feed<bool>) -> Self {
        Self {
            feed,
            on: Cell::new(false),
        }
    }

    fn inhibits_idle(on: &bool) -> bool {
        *on
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

impl Surface for Caffeine {
    const DASHBOARD_ORDER: u8 = 60;

    const GLYPHS: &'static [GlyphSpec] = &[("caffeine", &["coffee"])];

    type Local = ();

    fn present(_: &str, on: &bool) -> bool {
        *on
    }

    fn widget(&self, _: &str, _: &bool, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        Some(
            w.base("bar-caffeine")
                .tip("Caffeine is on")
                .glyph(w.glyph("caffeine"), ACCENT)
                .on_click(w.call(Caffeine::toggle)),
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
}
