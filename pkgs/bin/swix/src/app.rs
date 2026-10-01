use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::build::Target;
use crate::config::{Appearance, load_config, set_host_override, set_keybinds_override};
use crate::state::{Operation, UiState};
use crate::ui::build_screen::start_build;
use crate::ui::home::show_home;

const APP_ID: &str = "io.github.krezh.Swix";
const CSS: &str = include_str!("style.css");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Back,
    Close,
    FocusNext,
    Home(char),
    ScrollDown,
    ScrollEnd,
    ScrollHome,
    ScrollPageDown,
    ScrollPageUp,
    ScrollUp,
    Switch,
}

impl UiState {
    fn is_home(&self) -> bool {
        !self.home_buttons.borrow().is_empty()
    }

    fn focus_home(&self, delta: isize) -> bool {
        let button = {
            let buttons = self
                .home_buttons
                .borrow()
                .iter()
                .filter(|button| button.is_visible() && button.is_sensitive())
                .cloned()
                .collect::<Vec<_>>();
            if buttons.is_empty() {
                return false;
            }
            let current = buttons
                .iter()
                .position(|button| button.has_focus())
                .unwrap_or(0);
            let next = current
                .saturating_add_signed(delta)
                .min(buttons.len().saturating_sub(1));
            buttons[next].clone()
        };
        button.grab_focus();
        true
    }

    fn scroll(&self, action: KeyAction) -> bool {
        let Some(adjustment) = self.scroll.borrow().clone() else {
            return false;
        };
        scroll_adjustment(&adjustment, action)
    }
}

fn scroll_adjustment(adjustment: &gtk::Adjustment, action: KeyAction) -> bool {
    let lower = adjustment.lower();
    let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
    let step = adjustment.step_increment().max(72.0);
    let page = (adjustment.page_size() * 0.85).max(step);
    let value = match action {
        KeyAction::ScrollUp => adjustment.value() - step,
        KeyAction::ScrollDown => adjustment.value() + step,
        KeyAction::ScrollPageUp => adjustment.value() - page,
        KeyAction::ScrollPageDown => adjustment.value() + page,
        KeyAction::ScrollHome => lower,
        KeyAction::ScrollEnd => upper,
        _ => return false,
    };
    adjustment.set_value(value.clamp(lower, upper));
    true
}

#[derive(Clone)]
struct UiController {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
}

impl UiController {
    fn toggle_close(&self) -> bool {
        if self.state.request_close() {
            self.window.close();
            true
        } else {
            false
        }
    }

    fn start_nixos(&self) -> Result<(), String> {
        if self.state.close_pending.get() {
            return Err("application is closing".to_owned());
        }
        match self.state.operation.get() {
            Operation::Switching => {
                return Err("cannot change the build target while activation is running".to_owned());
            }
            Operation::Updating => {
                return Err("cannot start a build while the repository is updating".to_owned());
            }
            Operation::Building => {
                self.state.cancel();
            }
            Operation::Idle => {}
        }
        let config = load_config()?;
        start_build(
            &self.window,
            &self.root,
            Rc::clone(&self.state),
            config,
            Target::NixOs,
        );
        self.window.present();
        Ok(())
    }
}

pub(crate) fn run() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let controller = Rc::new(RefCell::new(None::<UiController>));
    app.connect_startup(|_| load_css());
    let command_controller = Rc::clone(&controller);
    app.connect_command_line(move |app, command_line| {
        match parse_command_line(command_line.arguments()) {
            Ok(command) => {
                if let Some(host) = command.host.clone() {
                    set_host_override(Some(host));
                }
                if let Some(keybinds) = command.keybinds {
                    set_keybinds_override(Some(keybinds));
                }
                if command.host.is_some() {
                    if app.windows().is_empty() {
                        app.activate();
                        0.into()
                    } else if let Some(controller) = command_controller.borrow().clone() {
                        match controller.start_nixos() {
                            Ok(()) => 0.into(),
                            Err(error) => {
                                command_line.printerr_literal(&format!("swix: {error}\n"));
                                1.into()
                            }
                        }
                    } else {
                        command_line.printerr_literal("swix: application window is unavailable\n");
                        1.into()
                    }
                } else {
                    if app.windows().is_empty() {
                        app.activate();
                    } else if let Some(controller) = command_controller.borrow().clone() {
                        controller.toggle_close();
                    }
                    0.into()
                }
            }
            Err(error) => {
                command_line.printerr_literal(&format!("swix: {error}\n"));
                2.into()
            }
        }
    });
    let activate_controller = Rc::clone(&controller);
    app.connect_activate(move |app| {
        if let Some(window) = app
            .windows()
            .into_iter()
            .find_map(|window| window.downcast::<gtk::ApplicationWindow>().ok())
        {
            if let Some(controller) = activate_controller.borrow().clone() {
                controller.toggle_close();
            } else {
                window.close();
            }
        } else {
            activate_controller.replace(Some(build_ui(app)));
        }
    });
    app.run()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CommandLine {
    host: Option<String>,
    keybinds: Option<bool>,
}

fn parse_command_line(arguments: Vec<std::ffi::OsString>) -> Result<CommandLine, String> {
    let mut host = None;
    let mut keybinds = None;
    let mut arguments = arguments.into_iter().skip(1);
    while let Some(argument) = arguments.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "command-line arguments must be valid UTF-8".to_owned())?;
        if argument == "--host" {
            let value = arguments
                .next()
                .ok_or("--host requires a NixOS configuration name")?
                .into_string()
                .map_err(|_| "host name must be valid UTF-8".to_owned())?;
            if value.trim().is_empty() || value.starts_with('-') {
                return Err("--host requires a non-empty NixOS configuration name".to_owned());
            }
            if host.replace(value).is_some() {
                return Err("--host may only be specified once".to_owned());
            }
        } else if let Some(value) = argument.strip_prefix("--host=") {
            let value = value.to_owned();
            if value.trim().is_empty() || value.starts_with('-') {
                return Err("--host requires a non-empty NixOS configuration name".to_owned());
            }
            if host.replace(value).is_some() {
                return Err("--host may only be specified once".to_owned());
            }
        } else if argument == "--no-keybinds" || argument == "--disable-keybinds" {
            keybinds = Some(false);
        } else if argument == "--keybinds" {
            keybinds = Some(true);
        } else if let Some(value) = argument.strip_prefix("--keybinds=") {
            match value {
                "true" | "1" | "yes" => keybinds = Some(true),
                "false" | "0" | "no" => keybinds = Some(false),
                _ => {
                    return Err(format!(
                        "invalid value {value:?} for --keybinds; expected true or false"
                    ));
                }
            }
        } else {
            return Err(format!(
                "unknown option {argument:?}; expected --host <name>, --no-keybinds"
            ));
        }
    }
    Ok(CommandLine { host, keybinds })
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    let appearance = load_config().map_or_else(
        |_| Appearance::default(),
        |config| Appearance {
            sans_font: config.sans_font,
            mono_font: config.mono_font,
            symbol_font: config.symbol_font,
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
    let symbol_font = appearance
        .symbol_font
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let css = format!(
        "{CSS}\nwindow.update-popup, window.update-popup.background, .updates-root {{ border-radius: {}px; }}\n.updates-root {{ font-family: \"{sans_font}\", sans-serif; }}\n.keycap, .flake-tag, .metric-branch, .metric-value, .version-cell, .size-cell, .switch-error, .build-error, .evaluation-warning-text {{ font-family: \"{mono_font}\", monospace; }}\n.symbol-icon, .nerd-icon {{ font-family: \"{symbol_font}\", \"{mono_font}\", monospace; }}",
        appearance.rounding,
    );
    #[allow(deprecated)]
    provider.load_from_data(&css);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn character_action(character: char) -> Option<KeyAction> {
    match character.to_ascii_lowercase() {
        key @ ('m' | 'n' | 'p' | 'r') => Some(KeyAction::Home(key)),
        's' => Some(KeyAction::Switch),
        _ => None,
    }
}

fn key_action(key: gdk::Key, modifiers: gdk::ModifierType) -> Option<KeyAction> {
    let clean_mods = modifiers
        & (gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::SHIFT_MASK);
    if clean_mods.intersects(
        gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SUPER_MASK,
    ) {
        return None;
    }
    match key {
        gdk::Key::Escape => Some(KeyAction::Close),
        gdk::Key::Left if clean_mods.is_empty() => Some(KeyAction::Back),
        gdk::Key::Right if clean_mods.is_empty() => Some(KeyAction::FocusNext),
        gdk::Key::Up if clean_mods.is_empty() => Some(KeyAction::ScrollUp),
        gdk::Key::Down if clean_mods.is_empty() => Some(KeyAction::ScrollDown),
        gdk::Key::Page_Up if clean_mods.is_empty() => Some(KeyAction::ScrollPageUp),
        gdk::Key::Page_Down if clean_mods.is_empty() => Some(KeyAction::ScrollPageDown),
        gdk::Key::Home if clean_mods.is_empty() => Some(KeyAction::ScrollHome),
        gdk::Key::End if clean_mods.is_empty() => Some(KeyAction::ScrollEnd),
        _ if clean_mods.is_empty() || clean_mods == gdk::ModifierType::SHIFT_MASK => {
            key.to_unicode().and_then(character_action)
        }
        _ => None,
    }
}

fn build_ui(app: &gtk::Application) -> UiController {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Swix")
        .decorated(false)
        .default_width(720)
        .default_height(260)
        .build();
    let appearance = load_config().map_or_else(
        |_| Appearance::default(),
        |config| Appearance {
            sans_font: config.sans_font,
            mono_font: config.mono_font,
            symbol_font: config.symbol_font,
            rounding: config.rounding,
            keybinds: config.keybinds,
        },
    );
    let keyboard_mode = if appearance.keybinds {
        KeyboardMode::OnDemand
    } else {
        KeyboardMode::None
    };
    window.init_layer_shell();
    window.set_namespace(Some("swix"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(keyboard_mode);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 16);
    window.set_exclusive_zone(0);

    window.add_css_class("update-popup");
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("updates-root");
    window.set_child(Some(&root));

    let state = Rc::new(UiState {
        appearance,
        ..UiState::default()
    });
    show_home(&window, &root, Rc::clone(&state), load_config());
    let key_window = window.clone();
    let key_state = Rc::clone(&state);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(action) = key_action(key, modifiers) else {
            return glib::Propagation::Proceed;
        };
        if !key_state.appearance.keybinds && action != KeyAction::Close {
            return glib::Propagation::Proceed;
        }
        let handled = match action {
            KeyAction::Close => {
                if key_state.request_close() {
                    key_window.close();
                }
                true
            }
            KeyAction::Back => {
                let button = key_state.back_button.borrow().clone();
                if let Some(button) = button {
                    button.emit_clicked();
                    true
                } else {
                    key_state.is_home() && key_state.focus_home(-1)
                }
            }
            KeyAction::FocusNext => key_state.is_home() && key_state.focus_home(1),
            KeyAction::Home(key) => {
                key_state.is_home()
                    && key_state.operation.get() == Operation::Idle
                    && key_state.activate_home_action(key)
            }
            KeyAction::Switch => {
                let confirmation = key_state.switch_confirmation.borrow().clone();
                if key_state.operation.get() == Operation::Idle
                    && let Some(confirmation) = confirmation
                {
                    confirmation.key_press();
                    true
                } else {
                    false
                }
            }
            KeyAction::ScrollUp => {
                if key_state.scroll(KeyAction::ScrollUp) {
                    true
                } else if key_state.is_home() {
                    key_state.focus_home(-1)
                } else {
                    false
                }
            }
            KeyAction::ScrollDown => {
                if key_state.scroll(KeyAction::ScrollDown) {
                    true
                } else if key_state.is_home() {
                    key_state.focus_home(1)
                } else {
                    false
                }
            }
            scroll => key_state.scroll(scroll),
        };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    let release_state = Rc::clone(&state);
    keys.connect_key_released(move |_, key, _, modifiers| {
        if !release_state.appearance.keybinds {
            return;
        }
        let confirmation = release_state.switch_confirmation.borrow().clone();
        if key_action(key, modifiers) == Some(KeyAction::Switch)
            && let Some(confirmation) = confirmation
        {
            confirmation.key_release();
        }
    });
    let active_state = Rc::clone(&state);
    window.connect_is_active_notify(move |window| {
        let confirmation = active_state.switch_confirmation.borrow().clone();
        if !window.is_active()
            && let Some(confirmation) = confirmation
        {
            confirmation.reset();
        }
    });
    let close_state = Rc::clone(&state);
    window.connect_close_request(move |_| {
        if close_state.request_close() {
            glib::Propagation::Proceed
        } else {
            glib::Propagation::Stop
        }
    });
    window.add_controller(keys);
    window.present();
    UiController {
        window,
        root,
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_vim_and_action_shortcuts() {
        assert_eq!(character_action('j'), None);
        assert_eq!(character_action('k'), None);
        assert_eq!(character_action('h'), None);
        assert_eq!(character_action('g'), None);
        assert_eq!(character_action('G'), None);
        assert_eq!(character_action('n'), Some(KeyAction::Home('n')));
        assert_eq!(character_action('N'), Some(KeyAction::Home('n')));
        assert_eq!(character_action('m'), Some(KeyAction::Home('m')));
        assert_eq!(character_action('M'), Some(KeyAction::Home('m')));
        assert_eq!(character_action('p'), Some(KeyAction::Home('p')));
        assert_eq!(character_action('r'), Some(KeyAction::Home('r')));
        assert_eq!(character_action('R'), Some(KeyAction::Home('r')));
        assert_eq!(character_action('s'), Some(KeyAction::Switch));
        assert_eq!(character_action('x'), None);
    }

    #[test]
    fn back_button_action_can_clear_actions_without_double_borrow() {
        if gtk::init().is_err() {
            return;
        }
        let state = Rc::new(UiState::default());
        let button = gtk::Button::new();
        let action_state = Rc::clone(&state);
        button.connect_clicked(move |_| {
            action_state.clear_actions();
        });
        state.set_back_button(&button);

        let button = state.back_button.borrow().clone();
        if let Some(button) = button {
            button.emit_clicked();
        }
        assert!(state.back_button.borrow().is_none());
    }

    #[test]
    fn parses_host_override() {
        let args = |values: &[&str]| values.iter().map(std::ffi::OsString::from).collect();
        assert_eq!(
            parse_command_line(args(&["swix", "--host", "odin"])).unwrap(),
            CommandLine {
                host: Some("odin".to_owned()),
                keybinds: None,
            }
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--host=thor"])).unwrap(),
            CommandLine {
                host: Some("thor".to_owned()),
                keybinds: None,
            }
        );
        assert_eq!(
            parse_command_line(args(&["swix"])).unwrap(),
            CommandLine::default()
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--no-keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(false),
            }
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--disable-keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(false),
            }
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(true),
            }
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--host", "odin", "--no-keybinds"])).unwrap(),
            CommandLine {
                host: Some("odin".to_owned()),
                keybinds: Some(false),
            }
        );
        assert!(parse_command_line(args(&["swix", "--host"])).is_err());
        assert!(parse_command_line(args(&["swix", "--unknown"])).is_err());
    }
}
