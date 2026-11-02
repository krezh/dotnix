use crate::bar_widget::BarWidget;
use std::any::Any;

use super::{panel, runtime::Molt, state::Updates, widget};
use crate::{
    features::{Feature, Feed, GlyphSpec, PanelCx, Surface, WidgetCx},
    panel::PanelView,
};

impl Feature for Molt {
    const NAME: &'static str = "molt";

    type State = Updates;

    type Event = ();

    fn start(feed: Feed<Updates>) -> Self {
        Molt::start(feed)
    }
}

impl Surface for Molt {
    const GLYPHS: &'static [GlyphSpec] = &[("molt", &["package"])];

    const PANEL: Option<&'static str> = Some("molt");

    type Local = ();

    /// So the panel opens on the latest without waiting for the network.
    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        self.look();
        None
    }

    fn widget(&self, _: &str, state: &Updates, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        widget::widget(state, w)
    }

    fn panel<'a>(&self, state: &Updates, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
    }
}
