//! Molt's stylesheets: each domain keeps its own next to its screens; they load here in cascade
//! order (shared blocks, screens, tone utilities).

use gtk::gdk;

use crate::config::{Appearance, load_config};

const CSS: &str = concat!(
    include_str!("../ui/style.css"),
    include_str!("../home/style.css"),
    include_str!("../inputs/style.css"),
    include_str!("../cleanup/style.css"),
    include_str!("../build/style.css"),
    include_str!("../activation/style.css"),
    include_str!("../report/style.css"),
    include_str!("../watchlist/style.css"),
    include_str!("../search/style.css"),
    include_str!("../ui/tones.css"),
);

const CONTROLS_CSS: &str = concat!(
    include_str!("../ui/controls.css"),
    include_str!("../home/controls.css"),
    include_str!("../cleanup/controls.css"),
    include_str!("../inputs/controls.css"),
    include_str!("../build/controls.css"),
    include_str!("../report/controls.css"),
    include_str!("../search/controls.css"),
);

pub(super) fn load_css() {
    let provider = gtk::CssProvider::new();
    let appearance = load_config().map_or_else(
        |_| Appearance::default(),
        |config| Appearance {
            sans_font: config.sans_font,
            mono_font: config.mono_font,
            rounding: config.rounding,
            keybinds: config.keybinds,
        },
    );
    let sans_font = appearance
        .sans_font
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let mono_font = appearance
        .mono_font
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    // Stylesheets read colours, fonts, and the window radius from these custom properties.
    let variables = format!(
        "{}:root {{\n  --sans-font: \"{sans_font}\", sans-serif;\n  --mono-font: \"{mono_font}\", monospace;\n  --window-radius: {}px;\n}}\n",
        crate::ui::theme::css_variables(),
        appearance.rounding,
    );
    #[allow(deprecated)]
    provider.load_from_data(&format!("{variables}{CSS}"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        // The user's gtk.css imports the GTK theme at USER priority, which outranks the
        // application provider for buttons, checks, and entries; these rules must sit above it.
        let controls = gtk::CssProvider::new();
        #[allow(deprecated)]
        controls.load_from_data(&format!("{variables}{CONTROLS_CSS}"));
        gtk::style_context_add_provider_for_display(
            &display,
            &controls,
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
    }
}
