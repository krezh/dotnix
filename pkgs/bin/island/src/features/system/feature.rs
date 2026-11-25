use std::{any::Any, sync::Arc};

use super::{overview, panel, state::State};
use crate::{
    config,
    features::{
        Dashboard, Feature, Feed, GlyphSpec, IslandInfo, Local, PanelCx, Surface, parse_settings,
    },
    panel::PanelView,
    sys::{self, Samples},
};

/// What the machine is doing: its own view among the dashboard's, and a card on the overview.
pub struct System {
    settings: config::System,
    samples: Samples,
}

/// How many displays the island's compositor reports, for the host tile.
#[derive(Default)]
pub struct Displays(usize);

impl Local for Displays {}

impl Feature for System {
    const NAME: &'static str = "system";

    type State = State;

    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        let settings = feed.settings();
        let mut revision = 0;
        let samples = sys::watch(move |stats, sampled| {
            revision += 1;
            feed.publish(State {
                stats: Arc::new(stats.clone()),
                sampled: Some(sampled),
                revision,
            })
        });
        Self { settings, samples }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        parse_settings::<config::System>(settings).map(drop)
    }
}

impl Surface for System {
    const GLYPHS: &'static [GlyphSpec] = &[("tab-system", &["cpu"])];

    type Local = Displays;

    const PANEL: Option<&'static str> = Some("system");

    const DASHBOARD_TAB: Option<(&'static str, u8)> = Some(("System", 2));

    /// After the Batteries card.
    const DASHBOARD_ORDER: u8 = 200;

    fn sync_local(_: &State, local: &mut Displays, island: &IslandInfo<'_>, _: bool) {
        local.0 = island
            .snapshot
            .map_or(1, |snapshot| snapshot.monitors.len().max(1));
    }

    /// The costly readings are taken only while it shows.
    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        Some(Box::new(self.samples.watch()))
    }

    fn dashboard_opened(&self) -> Option<Box<dyn Any>> {
        Some(Box::new(self.samples.watch()))
    }

    fn dashboard(&self, state: &State, p: &PanelCx<'_>) -> Dashboard {
        Dashboard {
            cards: vec![overview::card(state, p)],
            warnings: overview::warnings(state, p),
            ..Dashboard::default()
        }
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        let displays = p.local::<Displays>().map_or(1, |local| local.0);
        Some(panel::view(state, &self.settings, displays))
    }
}
