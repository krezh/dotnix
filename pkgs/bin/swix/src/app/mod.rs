pub(crate) mod mini;
pub(crate) mod state;
mod cli;
mod keys;
mod style;

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::app::state::{Operation, UiState};
use crate::build::Target;
use crate::build::ui::{AfterBuild, start_build};
use crate::config::{Appearance, load_config, set_host_override, set_keybinds_override};
use crate::home::ui::show_home;
use cli::parse_command_line;
use keys::{KeyAction, key_action};
use style::load_css;

const APP_ID: &str = "io.github.krezh.Swix";

#[derive(Clone)]
struct UiController {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
}

impl UiController {
    fn toggle_close(&self) -> bool {
        // Running swix again while minimized brings the window back rather than closing it.
        if let Some(mini) = self.state.mini.get()
            && mini.is_minimized()
        {
            mini.restore();
            return false;
        }
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
            Operation::Updating | Operation::Cleaning => {
                return Err("cannot start a build while maintenance is running".to_owned());
            }
            Operation::Building => {
                self.state.cancel();
            }
            Operation::Idle => {}
        }
        let config = load_config()?;
        if let Some(mini) = self.state.mini.get() {
            mini.restore();
        }
        start_build(
            &self.window,
            &self.root,
            Rc::clone(&self.state),
            config,
            Target::NixOs,
            AfterBuild::Review,
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
                    // A cold start has no window yet; activation builds the UI and controller
                    // synchronously, so the build starts on the fresh window.
                    if app.windows().is_empty() {
                        app.activate();
                    }
                    let controller = command_controller.borrow().clone();
                    let started = controller.map_or_else(
                        || Err("application window is unavailable".to_owned()),
                        |controller| controller.start_nixos(),
                    );
                    match started {
                        Ok(()) => 0.into(),
                        Err(error) => {
                            command_line.printerr_literal(&format!("swix: {error}\n"));
                            1.into()
                        }
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

fn build_ui(app: &gtk::Application) -> UiController {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Swix")
        .decorated(false)
        .default_width(720)
        .default_height(260)
        .build();
    let config = load_config();
    let appearance = config.as_ref().map_or_else(
        |_| Appearance::default(),
        |config| Appearance {
            sans_font: config.sans_font.clone(),
            mono_font: config.mono_font.clone(),
            rounding: config.rounding,
            keybinds: config.keybinds,
        },
    );
    window.init_layer_shell();
    window.set_namespace(Some("swix"));
    window.set_layer(Layer::Top);
    // Compositors focus an on-demand layer surface as soon as it maps, so Swix opens without the
    // keyboard and only takes it while it needs it (see `claim_keyboard`).
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 16);
    window.set_exclusive_zone(0);

    window.add_css_class("update-popup");
    // `root` is the current screen. The frame holds it, and during a screen change also the
    // previous screen, moving or fading the two. The window's root (see `mini`) draws the panel
    // behind the frame and morphs it into the minimized bar.
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("screen-root");
    let frame = crate::ui::SlideBin::new(&root);

    let state = Rc::new(UiState {
        appearance,
        ..UiState::default()
    });
    let mini = mini::Mini::new(&window, &frame, &state.appearance);
    let _ = state.mini.set(mini);
    match config {
        Ok(config) if std::env::var_os("SWIX_SMOKE_CLEANUP").is_some() => {
            crate::cleanup::ui::show_cleanup(&window, &root, Rc::clone(&state), config.cleanup);
        }
        Ok(config) if std::env::var_os("SWIX_SMOKE_INPUTS").is_some() => {
            crate::inputs::ui::show_inputs(&window, &root, Rc::clone(&state), config);
        }
        config => show_home(&window, &root, Rc::clone(&state), config),
    }
    let key_window = window.clone();
    let key_state = Rc::clone(&state);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(action) = key_action(key, modifiers) else {
            return glib::Propagation::Proceed;
        };
        // The controller runs in the capture phase, so a focused text field would otherwise lose
        // letters ("s" switching) and cursor keys to the shortcuts. Escape and list scrolling
        // still work from inside a filter.
        if gtk::prelude::GtkWindowExt::focus(&key_window).is_some_and(|widget| widget.is::<gtk::Text>())
            && !matches!(
                action,
                KeyAction::Close | KeyAction::ScrollUp | KeyAction::ScrollDown
            )
        {
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
                let confirmation = key_state.switch_confirmations.borrow().first().cloned();
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
                if key_state.scroll(&key_window, KeyAction::ScrollUp) {
                    true
                } else if key_state.is_home() {
                    key_state.focus_home(-1)
                } else {
                    false
                }
            }
            KeyAction::ScrollDown => {
                if key_state.scroll(&key_window, KeyAction::ScrollDown) {
                    true
                } else if key_state.is_home() {
                    key_state.focus_home(1)
                } else {
                    false
                }
            }
            scroll => key_state.scroll(&key_window, scroll),
        };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    let release_state = Rc::clone(&state);
    keys.connect_key_released(move |_, key, _, modifiers| {
        let confirmation = release_state.switch_confirmations.borrow().first().cloned();
        if key_action(key, modifiers) == Some(KeyAction::Switch)
            && let Some(confirmation) = confirmation
        {
            confirmation.key_release();
        }
    });
    claim_keyboard(&window, &state);
    let active_state = Rc::clone(&state);
    window.connect_is_active_notify(move |window| {
        if window.is_active() {
            return;
        }
        window.set_keyboard_mode(KeyboardMode::None);
        let confirmations = active_state.switch_confirmations.borrow().clone();
        for confirmation in confirmations {
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
    if state.appearance.keybinds {
        window.add_controller(keys);
    }
    window.present();
    UiController {
        window,
        root,
        state,
    }
}

/// Gives Swix the keyboard only while it needs it. With shortcuts off that is while a text field
/// is focused: a click into one takes the keyboard exclusively (the click itself has already been
/// routed, so on-demand would only focus on the next one), and focusing anything else, the pointer
/// leaving, or another window activating hands it back. With shortcuts on, the panel takes the
/// keyboard on demand while the pointer is over it.
fn claim_keyboard(window: &gtk::ApplicationWindow, state: &Rc<UiState>) {
    let minimized = {
        let state = Rc::clone(state);
        move || state.mini.get().is_some_and(mini::Mini::is_minimized)
    };
    let motion = gtk::EventControllerMotion::new();
    if state.appearance.keybinds {
        let enter_window = window.downgrade();
        let enter_minimized = minimized.clone();
        motion.connect_enter(move |_, _, _| {
            if let Some(window) = enter_window.upgrade()
                && !enter_minimized()
            {
                window.set_keyboard_mode(KeyboardMode::OnDemand);
            }
        });
    }
    let leave_window = window.downgrade();
    motion.connect_leave(move |_| {
        if let Some(window) = leave_window.upgrade() {
            window.set_keyboard_mode(KeyboardMode::None);
        }
    });
    window.add_controller(motion);
    if state.appearance.keybinds {
        return;
    }

    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let click_window = window.downgrade();
    click.connect_pressed(move |_, _, x, y| {
        let Some(window) = click_window.upgrade() else {
            return;
        };
        let on_text = window.pick(x, y, gtk::PickFlags::DEFAULT).is_some_and(|widget| {
            widget.is::<gtk::Text>() || widget.ancestor(gtk::Text::static_type()).is_some()
        });
        window.set_keyboard_mode(if on_text && !minimized() {
            KeyboardMode::Exclusive
        } else {
            KeyboardMode::None
        });
    });
    window.add_controller(click);
    window.connect_focus_widget_notify(|window| {
        if !gtk::prelude::GtkWindowExt::focus(window).is_some_and(|widget| widget.is::<gtk::Text>()) {
            window.set_keyboard_mode(KeyboardMode::None);
        }
    });
}

