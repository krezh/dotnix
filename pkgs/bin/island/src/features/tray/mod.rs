mod icons;
mod menu;
mod raster;
mod runtime;
mod sni;
mod widget;

use gpui::AnyElement;

use super::{Feature, Feed, IslandInfo, WidgetCx};
use sni::TrayItem;

pub use icons::Icons;
pub use runtime::Tray;

impl Feature for Tray {
    const NAME: &'static str = "tray";

    type State = Vec<TrayItem>;
    type Local = Icons;
    type Event = ();

    fn start(feed: Feed<Vec<TrayItem>>) -> Self {
        Self::start(feed)
    }

    fn sync_local(items: &Vec<TrayItem>, icons: &mut Icons, _: &IslandInfo<'_>, animate: bool) {
        icons::sync(items, icons, animate);
    }

    fn widget(&self, _: &str, _: &Vec<TrayItem>, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        self.widget_view(w)
    }
}
