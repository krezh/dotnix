//! How island sees the tray: its items, each island's icons, and the widget.

use super::{
    icons::{self, Icons},
    runtime::Tray,
    sni::TrayItem,
};
use crate::bar_widget::BarWidget;
use crate::features::{Feature, Feed, IslandInfo, Surface, WidgetCx};

impl Feature for Tray {
    const NAME: &'static str = "tray";

    type State = Vec<TrayItem>;

    type Event = ();

    fn start(feed: Feed<Vec<TrayItem>>) -> Self {
        Self::start(feed)
    }
}

impl Surface for Tray {
    type Local = Icons;

    fn sync_local(items: &Vec<TrayItem>, icons: &mut Icons, _: &IslandInfo<'_>, animate: bool) {
        icons::sync(items, icons, animate);
    }

    fn widget(&self, _: &str, _: &Vec<TrayItem>, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        self.widget_view(w)
    }
}
