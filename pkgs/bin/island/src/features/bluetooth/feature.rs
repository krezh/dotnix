//! How island sees Bluetooth: the bar widget, the Bluetooth tab and its pairing card, and
//! `island ctl bluetooth`.

use gpui::AnyElement;

use super::{Bluetooth, glyphs, panel, runtime::Request, state::State, view::View, widget};
use crate::{
    control::{self, Command, Group, Report, Row, SWITCH, Shown, Takes},
    features::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx},
    panel::PanelView,
};

impl Feature for Bluetooth {
    const NAME: &'static str = "bluetooth";
    const PANEL: Option<&'static str> = Some("bluetooth");
    const TAB: Option<(&'static str, u8)> = Some(("Bluetooth", 6));
    const GLYPHS: &'static [GlyphSpec] = glyphs::GLYPHS;
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
    type Local = View;
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        Bluetooth::start(feed)
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(widget::widget(state, w))
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
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
