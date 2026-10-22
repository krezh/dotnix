//! Every feature, listed once; what island asks of all of them goes through this list.

use std::time::Duration;

use gpui::App;

use super::{
    Feature, Features, GlyphSpec, ai, audio, brightness, caffeine, clipboard, clock,
    feature::{answers, widgets_of},
    headset,
    instance::start_one,
    launcher, media, molt, notifications, session, tray, workspaces, zowie,
};

macro_rules! features {
    ($($feature:ty),* $(,)?) => {
        pub const NAMES: &[&str] = &[$(<$feature as Feature>::NAME),*];

        pub fn widget_names() -> Vec<&'static str> {
            let mut names = Vec::new();
            $(names.extend(widgets_of::<$feature>());)*
            names
        }

        pub fn owner(widget: &str) -> Option<&'static str> {
            $(
                if widgets_of::<$feature>().any(|name| name == widget) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        pub fn panel_refresh(feature: &str) -> Option<Duration> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL_REFRESH;
                }
            )*
            None
        }

        pub fn check_settings(section: &str, settings: &toml::Value) -> Result<(), String> {
            $(
                if <$feature as Feature>::NAME == section {
                    return <$feature as Feature>::check_settings(settings).map_err(|error| format!("[features.{section}]: {error}"));
                }
            )*
            Err(format!("unknown section [features.{section}]"))
        }

        pub fn glyphs() -> Vec<GlyphSpec> {
            let mut all = Vec::new();
            $(all.extend_from_slice(<$feature as Feature>::GLYPHS);)*
            all
        }

        pub fn panel_named(panel: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::PANEL == Some(panel) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        pub fn keyboard(feature: &str) -> bool {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::KEYBOARD;
                }
            )*
            false
        }

        /// The views the panel named `panel` can be opened on.
        pub fn views(panel: &str) -> &'static [&'static str] {
            $(
                if <$feature as Feature>::PANEL == Some(panel) {
                    return <$feature as Feature>::VIEWS;
                }
            )*
            &[]
        }

        pub fn panel_of(feature: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL;
                }
            )*
            None
        }

        /// Feature, title and place among the tabs.
        pub fn tabs() -> Vec<(&'static str, &'static str, u8)> {
            let mut tabs = Vec::new();
            $(
                if let Some((title, order)) = <$feature as Feature>::TAB {
                    tabs.push((<$feature as Feature>::NAME, title, order));
                }
            )*
            tabs
        }

        /// `None` when no feature answers to the first word.
        pub fn check_command(args: &[&str]) -> Option<Result<&'static str, String>> {
            let word = args.first()?;
            $(
                if answers::<$feature>(word) {
                    return Some(<$feature as Feature>::check(args).map(|()| <$feature as Feature>::NAME));
                }
            )*
            None
        }

        /// The client of the feature whose `CLIENT` is `word`.
        pub fn client(word: &str) -> Option<fn(&[String]) -> i32> {
            $(
                if !<$feature as Feature>::CLIENT.is_empty() && <$feature as Feature>::CLIENT == word {
                    return Some(<$feature as Feature>::client);
                }
            )*
            None
        }

        pub fn usage() -> String {
            [$(<$feature as Feature>::USAGE),*]
                .into_iter()
                .filter(|usage| !usage.is_empty())
                .map(|usage| format!("\n{usage}"))
                .collect()
        }

        /// Features with a panel or commands start even without a wanted widget, since they can be
        /// reached without one; the others never run.
        pub fn start(wanted: impl Fn(&'static str) -> bool, glow: bool, settings: &toml::Table, cx: &mut App) -> Features {
            let mut started = Vec::new();
            $(
                if <$feature as Feature>::PANEL.is_some()
                    || !<$feature as Feature>::USAGE.is_empty()
                    || widgets_of::<$feature>().any(&wanted)
                {
                    let live = <$feature as Feature>::LIVE.iter().any(|widget| wanted(widget))
                        || (glow && !<$feature as Feature>::LIVE.is_empty());
                    let section = settings.get(<$feature as Feature>::NAME).cloned();
                    started.push(start_one::<$feature>(live, section, cx));
                }
            )*
            Features(started)
        }
    };
}

features![
    clock::Clock,
    notifications::Notifications,
    workspaces::Workspaces,
    tray::Tray,
    caffeine::Caffeine,
    molt::Molt,
    ai::Ai,
    headset::Headset,
    zowie::Zowie,
    audio::Audio,
    brightness::Brightness,
    media::Media,
    session::Session,
    clipboard::Clipboard,
    launcher::Launcher,
];
