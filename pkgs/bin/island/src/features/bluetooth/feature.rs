//! How island sees Bluetooth: the bar widget, the Bluetooth tab and its pairing card, and
//! `island ctl bluetooth`.

use super::{
    Bluetooth, glyphs, overview, panel, runtime::Request, state::State, view::View, widget,
};
use crate::bar_widget::BarWidget;
use crate::{
    control::{self, Command, Group, Report, Row, SWITCH, Shown, Takes},
    features::{Battery, Dashboard, Feature, Feed, GlyphSpec, PanelCx, Surface, WidgetCx},
    panel::PanelView,
};

impl Feature for Bluetooth {
    const NAME: &'static str = "bluetooth";

    const CONTROLS: &'static [Command] = &[Command {
        name: "bluetooth",
        about: "Turn Bluetooth on or off",
        group: Group::System,
        takes: Takes::Forms {
            forms: SWITCH,
            bare: Some("Show whether it is on and what is connected"),
        },
        examples: &["island ctl bluetooth off"],
    }];

    type State = State;

    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        Bluetooth::start(feed)
    }

    fn batteries(&self, state: &State) -> Vec<Battery> {
        overview::batteries(state)
    }

    fn command(&self, state: &State, args: &[&str]) -> Report {
        if args.len() == 1 {
            return Report::shows(self.status(state));
        }
        let Ok(switch) = control::switch(&args[1..]) else {
            return Report::default();
        };
        let on = switch.apply(state.powered);
        self.send(Request::Power(Some(on)));
        Report::did(Row::new("Bluetooth", Shown::Switch(on)))
    }

    fn status(&self, state: &State) -> Vec<Row> {
        if !state.available {
            return vec![Row::new("Bluetooth", Shown::Text("no adapter".to_owned()))];
        }
        let connected: Vec<String> = state
            .connected()
            .map(|device| device.name.clone())
            .collect();
        let row = Row::new("Bluetooth", Shown::Switch(state.powered));
        vec![if connected.is_empty() {
            row
        } else {
            row.detail(connected.join(", "))
        }]
    }
}

impl Surface for Bluetooth {
    const PANEL: Option<&'static str> = Some("bluetooth");

    const DASHBOARD_TAB: Option<(&'static str, u8)> = Some(("Bluetooth", 4));

    const DASHBOARD_ORDER: u8 = 20;

    const GLYPHS: &'static [GlyphSpec] = glyphs::GLYPHS;

    type Local = View;

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        Some(widget::widget(state, w))
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
    }

    fn dashboard(&self, state: &State, p: &PanelCx<'_>) -> Dashboard {
        Dashboard {
            switches: overview::switches(state, p),
            ..Dashboard::default()
        }
    }
}
