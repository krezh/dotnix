mod activation;
mod build;
mod changelog;
mod config;
mod nix;
mod report;
pub(crate) mod theme;
mod ui;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use build::{BuildEvent, BuildPhase, BuildUpdate, Target, build_report};
use changelog::{ChangelogOutput, parse_changelog};
use config::{Appearance, Config, load_config, set_host_override};
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use nix::{NixBuildProgress, run};
use report::{Change, ChangeStatus, Report};
use swix::command;
use ui::build_progress::ActivityRows;
use ui::changelog::render as render_changelog;
#[cfg(test)]
use ui::changelog::{MarkdownBlock, MarkdownBlockKind, markdown_blocks};
use ui::switch::{
    SwitchConfirmation, SwitchView, connect_switch, current_hostname,
    requires_host_switch_confirmation, switch_animation,
};

#[cfg(test)]
use activation::parse_service_response;
#[cfg(test)]
use build::GcRoot;
#[cfg(test)]
use config::parse_config;
#[cfg(test)]
use nix::{NixProgressTracker, nix_error_message, strip_ansi};
#[cfg(test)]
use report::{ReportMetadata, compact_dix_version, compact_versions, parse_report};
#[cfg(test)]
use std::{env, path::PathBuf};
#[cfg(test)]
use ui::switch::switch_confirmation_allows_activation;

const APP_ID: &str = "io.github.krezh.Swix";
const CSS: &str = include_str!("style.css");
const CHANGELOG_TIMEOUT: Duration = Duration::from_secs(2 * 60);

#[derive(Clone)]
struct ReportNavigation {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
    report: Report,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Operation {
    #[default]
    Idle,
    Building,
    Switching,
}

struct UiState {
    chooser_buttons: RefCell<Vec<gtk::Button>>,
    scroll: RefCell<Option<gtk::Adjustment>>,
    switch_confirmation: RefCell<Option<SwitchConfirmation>>,
    back_button: RefCell<Option<gtk::Button>>,
    operation: Cell<Operation>,
    generation: Cell<u64>,
    cancellation: RefCell<Option<Arc<AtomicBool>>>,
    view_cancellation: RefCell<Option<Arc<AtomicBool>>>,
    changelog_cache: RefCell<HashMap<String, ChangelogOutput>>,
    appearance: Appearance,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chooser_buttons: RefCell::new(Vec::new()),
            scroll: RefCell::new(None),
            switch_confirmation: RefCell::new(None),
            back_button: RefCell::new(None),
            operation: Cell::new(Operation::Idle),
            generation: Cell::new(0),
            cancellation: RefCell::new(None),
            view_cancellation: RefCell::new(None),
            changelog_cache: RefCell::new(HashMap::new()),
            appearance: Appearance::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Back,
    Close,
    FocusNext,
    HomeManager,
    NixOs,
    ScrollDown,
    ScrollEnd,
    ScrollHome,
    ScrollPageDown,
    ScrollPageUp,
    ScrollUp,
    Switch,
}

impl UiState {
    fn clear_actions(&self) {
        let cancellation = self.view_cancellation.borrow().clone();
        if let Some(cancellation) = cancellation {
            cancellation.store(true, Ordering::Relaxed);
        }
        self.chooser_buttons.borrow_mut().clear();
        self.scroll.replace(None);
        let confirmation = self.switch_confirmation.borrow_mut().take();
        if let Some(confirmation) = confirmation {
            confirmation.reset();
        }
        self.back_button.replace(None);
        self.view_cancellation.replace(None);
    }

    fn begin(&self, operation: Operation) -> Option<(u64, Arc<AtomicBool>)> {
        if self.operation.get() != Operation::Idle {
            return None;
        }
        let generation = self.generation.get().wrapping_add(1);
        let cancellation = Arc::new(AtomicBool::new(false));
        self.generation.set(generation);
        self.operation.set(operation);
        self.cancellation.replace(Some(Arc::clone(&cancellation)));
        Some((generation, cancellation))
    }

    fn finish(&self, generation: u64) -> bool {
        if self.generation.get() != generation {
            return false;
        }
        self.operation.set(Operation::Idle);
        self.cancellation.replace(None);
        true
    }

    fn cancel(&self) -> bool {
        if self.operation.get() == Operation::Switching {
            return false;
        }
        let cancellation = self.cancellation.borrow().clone();
        if let Some(cancellation) = cancellation {
            cancellation.store(true, Ordering::Relaxed);
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.operation.set(Operation::Idle);
        self.cancellation.replace(None);
        let cancellation = self.view_cancellation.borrow().clone();
        if let Some(cancellation) = cancellation {
            cancellation.store(true, Ordering::Relaxed);
        }
        self.view_cancellation.replace(None);
        true
    }

    fn focus_chooser(&self, delta: isize) -> bool {
        let button = {
            let buttons = self.chooser_buttons.borrow();
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

fn animate_scroll_to(
    scroller: &gtk::ScrolledWindow,
    adjustment: &gtk::Adjustment,
    target: f64,
    animation: Rc<Cell<u64>>,
) {
    let lower = adjustment.lower();
    let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
    let start = adjustment.value();
    let target = target.clamp(lower, upper);
    let distance = (target - start).abs();
    let duration = 0.28 + (distance / 6000.0).min(1.0) * 0.18;
    let animation_id = animation.get().wrapping_add(1);
    animation.set(animation_id);
    let adjustment = adjustment.clone();
    let started = Cell::new(None);
    scroller.add_tick_callback(move |_, frame_clock| {
        if animation.get() != animation_id {
            return glib::ControlFlow::Break;
        }
        let now = frame_clock.frame_time();
        let started = started.get().unwrap_or_else(|| {
            started.set(Some(now));
            now
        });
        let progress = ((now - started) as f64 / 1_000_000.0 / duration).min(1.0);
        let eased = 0.5 - 0.5 * (std::f64::consts::PI * progress).cos();
        adjustment.set_value(start + (target - start) * eased);
        if progress >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

#[derive(Clone)]
struct UiController {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
}

impl UiController {
    fn start_nixos(&self) -> Result<(), String> {
        if self.state.operation.get() == Operation::Switching {
            return Err("cannot change the build target while activation is running".to_owned());
        }
        if self.state.operation.get() == Operation::Building {
            self.state.cancel();
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

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let controller = Rc::new(RefCell::new(None::<UiController>));
    app.connect_startup(|_| load_css());
    let command_controller = Rc::clone(&controller);
    app.connect_command_line(move |app, command_line| {
        match parse_command_line(command_line.arguments()) {
            Ok(Some(host)) => {
                set_host_override(Some(host));
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
            }
            Ok(None) => {
                app.activate();
                0.into()
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
            window.present();
        } else {
            activate_controller.replace(Some(build_ui(app)));
        }
    });
    app.run()
}

fn parse_command_line(arguments: Vec<std::ffi::OsString>) -> Result<Option<String>, String> {
    let mut host = None;
    let mut arguments = arguments.into_iter().skip(1);
    while let Some(argument) = arguments.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "command-line arguments must be valid UTF-8".to_owned())?;
        let value = if argument == "--host" {
            arguments
                .next()
                .ok_or("--host requires a NixOS configuration name")?
                .into_string()
                .map_err(|_| "host name must be valid UTF-8".to_owned())?
        } else if let Some(value) = argument.strip_prefix("--host=") {
            value.to_owned()
        } else {
            return Err(format!(
                "unknown option {argument:?}; expected --host <name>"
            ));
        };
        if value.trim().is_empty() || value.starts_with('-') {
            return Err("--host requires a non-empty NixOS configuration name".to_owned());
        }
        if host.replace(value).is_some() {
            return Err("--host may only be specified once".to_owned());
        }
    }
    Ok(host)
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
        "{CSS}\nwindow.update-popup, window.update-popup.background, .updates-root {{ border-radius: {}px; }}\n.updates-root {{ font-family: \"{sans_font}\", sans-serif; }}\n.keycap, .flake-tag, .metric-branch, .metric-value, .version-cell, .size-cell, .switch-error, .build-error, .evaluation-activity {{ font-family: \"{mono_font}\", monospace; }}\n.symbol-icon, .nerd-icon {{ font-family: \"{symbol_font}\", \"{mono_font}\", monospace; }}",
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
    match character {
        'h' | 'H' => Some(KeyAction::Back),
        'j' | 'J' => Some(KeyAction::ScrollDown),
        'k' | 'K' => Some(KeyAction::ScrollUp),
        'm' | 'M' => Some(KeyAction::HomeManager),
        'n' | 'N' => Some(KeyAction::NixOs),
        's' | 'S' => Some(KeyAction::Switch),
        'g' => Some(KeyAction::ScrollHome),
        'G' => Some(KeyAction::ScrollEnd),
        _ => None,
    }
}

fn key_action(key: gdk::Key, modifiers: gdk::ModifierType) -> Option<KeyAction> {
    if modifiers.intersects(
        gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SUPER_MASK,
    ) {
        return None;
    }
    match key {
        gdk::Key::Escape => Some(KeyAction::Close),
        gdk::Key::Left => Some(KeyAction::Back),
        gdk::Key::Right => Some(KeyAction::FocusNext),
        gdk::Key::Up => Some(KeyAction::ScrollUp),
        gdk::Key::Down => Some(KeyAction::ScrollDown),
        gdk::Key::Page_Up => Some(KeyAction::ScrollPageUp),
        gdk::Key::Page_Down => Some(KeyAction::ScrollPageDown),
        gdk::Key::Home => Some(KeyAction::ScrollHome),
        gdk::Key::End => Some(KeyAction::ScrollEnd),
        _ => key.to_unicode().and_then(character_action),
    }
}

fn build_ui(app: &gtk::Application) -> UiController {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Swix Software Updates")
        .decorated(false)
        .default_width(720)
        .default_height(260)
        .build();
    window.init_layer_shell();
    window.set_namespace(Some("swix"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 16);
    window.set_exclusive_zone(0);

    window.add_css_class("update-popup");
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("updates-root");
    window.set_child(Some(&root));

    let appearance = load_config().map_or_else(
        |_| Appearance::default(),
        |config| Appearance {
            sans_font: config.sans_font,
            mono_font: config.mono_font,
            symbol_font: config.symbol_font,
            rounding: config.rounding,
        },
    );
    let state = Rc::new(UiState {
        appearance,
        ..UiState::default()
    });
    let config = load_config();
    match config {
        Ok(config) if config.home_flake.is_none() => {
            start_build(&window, &root, Rc::clone(&state), config, Target::NixOs);
        }
        config => show_chooser(&window, &root, Rc::clone(&state), config),
    }
    let key_window = window.clone();
    let key_root = root.clone();
    let key_state = Rc::clone(&state);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(action) = key_action(key, modifiers) else {
            return glib::Propagation::Proceed;
        };
        match action {
            KeyAction::Close => {
                if key_state.cancel() {
                    key_window.close();
                }
            }
            KeyAction::Back => {
                let button = key_state.back_button.borrow().clone();
                if let Some(button) = button {
                    button.emit_clicked();
                } else if !key_state.focus_chooser(-1) {
                    show_chooser(&key_window, &key_root, Rc::clone(&key_state), load_config());
                }
            }
            KeyAction::FocusNext => {
                key_state.focus_chooser(1);
            }
            KeyAction::HomeManager => {
                if key_state.operation.get() == Operation::Idle
                    && let Ok(config) = load_config()
                    && config.home_flake.is_some()
                {
                    start_build(
                        &key_window,
                        &key_root,
                        Rc::clone(&key_state),
                        config,
                        Target::HomeManager,
                    );
                }
            }
            KeyAction::NixOs => {
                if key_state.operation.get() == Operation::Idle
                    && let Ok(config) = load_config()
                {
                    start_build(
                        &key_window,
                        &key_root,
                        Rc::clone(&key_state),
                        config,
                        Target::NixOs,
                    );
                }
            }
            KeyAction::Switch => {
                let confirmation = key_state.switch_confirmation.borrow().clone();
                if key_state.operation.get() == Operation::Idle
                    && let Some(confirmation) = confirmation
                {
                    confirmation.key_press();
                }
            }
            KeyAction::ScrollUp if !key_state.scroll(KeyAction::ScrollUp) => {
                key_state.focus_chooser(-1);
            }
            KeyAction::ScrollDown if !key_state.scroll(KeyAction::ScrollDown) => {
                key_state.focus_chooser(1);
            }
            scroll if key_state.scroll(scroll) => {}
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    let release_state = Rc::clone(&state);
    keys.connect_key_released(move |_, key, _, modifiers| {
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
        if close_state.operation.get() == Operation::Switching {
            glib::Propagation::Stop
        } else {
            close_state.cancel();
            glib::Propagation::Proceed
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

fn clear(root: &gtk::Box) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
}

fn fit_window(window: &gtk::ApplicationWindow, preferred: (i32, i32), minimum: (i32, i32)) {
    let available = gtk::prelude::WidgetExt::display(window)
        .monitors()
        .item(0)
        .and_downcast::<gdk::Monitor>()
        .map(|monitor| monitor.geometry())
        .map(|geometry| (geometry.width() - 32, geometry.height() - 32))
        .unwrap_or(preferred);
    window.set_default_size(preferred.0.min(available.0), preferred.1.min(available.1));
    window.set_size_request(minimum.0.min(available.0), minimum.1.min(available.1));
}

fn title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("title");
    label.set_xalign(0.0);
    label
}

fn label(text: &str, classes: &[&str], xalign: f32) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(xalign);
    label.set_halign(gtk::Align::Fill);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    for class in classes {
        label.add_css_class(class);
    }
    label
}

#[derive(Clone)]
struct BuildSummary {
    root: gtk::Box,
    map: gtk::DrawingArea,
    map_states: Rc<RefCell<Vec<GraphNodeState>>>,
    count: gtk::Label,
    building: gtk::Label,
    downloading: gtk::Label,
    complete: gtk::Label,
    planned: gtk::Label,
    failed_group: gtk::Box,
    failed: gtk::Label,
}

fn build_summary() -> BuildSummary {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    root.add_css_class("build-summary");
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    heading.append(&label("Build plan", &["build-summary-title"], 0.0));
    let count = label("Preparing derivations", &["build-summary-count"], 1.0);
    count.set_hexpand(true);
    heading.append(&count);
    root.append(&heading);
    let map = gtk::DrawingArea::new();
    map.add_css_class("build-plan-map");
    map.set_content_height(28);
    map.set_hexpand(true);
    let map_states = Rc::new(RefCell::new(Vec::<GraphNodeState>::new()));
    let draw_states = Rc::clone(&map_states);
    map.set_draw_func(move |_, context, width, height| {
        draw_build_plan(context, width, height, &draw_states.borrow());
    });
    root.append(&map);
    let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    metrics.add_css_class("build-summary-metrics");
    let (building_group, building) = summary_metric("system-run-symbolic", "building");
    let (downloading_group, downloading) =
        summary_metric("folder-download-symbolic", "downloading");
    let (complete_group, complete) = summary_metric("object-select-symbolic", "complete");
    let (planned_group, planned) = summary_metric("media-playback-pause-symbolic", "planned");
    let (failed_group, failed) = summary_metric("dialog-warning-symbolic", "failed");
    failed_group.set_visible(false);
    metrics.append(&building_group);
    metrics.append(&downloading_group);
    metrics.append(&complete_group);
    metrics.append(&planned_group);
    metrics.append(&failed_group);
    root.append(&metrics);
    BuildSummary {
        root,
        map,
        map_states,
        count,
        building,
        downloading,
        complete,
        planned,
        failed_group,
        failed,
    }
}
fn summary_metric(icon: &str, class: &str) -> (gtk::Box, gtk::Label) {
    let group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    group.add_css_class("build-summary-metric");
    group.add_css_class(class);
    group.set_width_request(128);
    let icon = gtk::Image::from_icon_name(icon);
    icon.set_pixel_size(16);
    group.append(&icon);
    let value = label("0", &["build-summary-value"], 0.0);
    group.append(&value);
    (group, value)
}

struct BuildTimeline {
    root: gtk::Box,
    steps: Vec<gtk::Label>,
    indicator: gtk::DrawingArea,
    current: Rc<Cell<f64>>,
    target: Rc<Cell<f64>>,
    complete: Rc<Cell<bool>>,
    pulse: Rc<Cell<f64>>,
    animating: Rc<Cell<bool>>,
}

impl BuildTimeline {
    fn new(font: &str) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        root.add_css_class("build-timeline");
        root.set_width_request(440);
        root.set_halign(gtk::Align::Center);
        root.set_valign(gtk::Align::Center);
        root.set_vexpand(false);
        let current = Rc::new(Cell::new(0.0_f64));
        let target = Rc::new(Cell::new(0.0_f64));
        let complete = Rc::new(Cell::new(false));
        let pulse = Rc::new(Cell::new(0.0_f64));
        let indicator = gtk::DrawingArea::new();
        indicator.add_css_class("build-phase-indicator");
        indicator.set_hexpand(true);
        indicator.set_content_width(440);
        indicator.set_content_height(34);
        let draw_position = Rc::clone(&current);
        let draw_target = Rc::clone(&target);
        let draw_complete = Rc::clone(&complete);
        let draw_pulse = Rc::clone(&pulse);
        let draw_font = font.to_owned();
        indicator.set_draw_func(move |_, context, width, _| {
            let width = f64::from(width);
            let w_cap = 116.0_f64;
            let h_cap = 30.0_f64;
            let y_cap = 2.0_f64;
            let center_y = 17.0_f64;
            let w_conn = 46.0_f64;
            let total_caps_width = 3.0 * w_cap + 2.0 * w_conn;
            let offset_x = (width - total_caps_width) / 2.0;
            let position = draw_position.get().clamp(0.0, 2.0);
            let active = draw_target.get().round().clamp(0.0, 2.0) as usize;
            let is_complete = draw_complete.get();
            let phase = draw_pulse.get();

            let cap_names = ["Evaluate", "Build", "Compare"];

            // 1. Draw inter-phase connecting rails
            for i in 0..2 {
                let conn_start = offset_x + (i as f64 + 1.0) * w_cap + i as f64 * w_conn;
                let conn_end = conn_start + w_conn;

                // Base rail track
                context.set_line_cap(gtk::cairo::LineCap::Round);
                context.set_line_width(2.5);
                theme::set_source_rgba(context, theme::SURFACE1, 0.35);
                context.move_to(conn_start, center_y);
                context.line_to(conn_end, center_y);
                let _ = context.stroke();

                // Progress fill through connector
                let conn_progress = (position - i as f64).clamp(0.0, 1.0);
                if conn_progress > 0.0 {
                    let fill_end = conn_start + w_conn * conn_progress;
                    context.set_line_width(3.0);
                    theme::set_source_rgba(context, theme::GREEN, 0.90);
                    context.move_to(conn_start, center_y);
                    context.line_to(fill_end, center_y);
                    let _ = context.stroke();
                }

                // Subtle pulse through active connector during transition
                if !is_complete && active == i + 1 && position < (i + 1) as f64 {
                    let transition_t = (position - i as f64).clamp(0.0, 1.0);
                    let pulse_x = conn_start + w_conn * transition_t;
                    context.arc(pulse_x, center_y, 3.0, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::SAPPHIRE, 0.90);
                    let _ = context.fill();
                }
            }

            // 2. Draw three horizontal phase capsules
            for (i, name) in cap_names.iter().enumerate() {
                let cap_x = offset_x + i as f64 * (w_cap + w_conn);
                let is_capsule_complete =
                    is_complete || (i < active && position >= i as f64 + 0.85);
                let is_capsule_active = !is_complete && (i == active);

                // Select font and calculate text extents
                let font_name = draw_font.clone();
                context.select_font_face(
                    &font_name,
                    gtk::cairo::FontSlant::Normal,
                    gtk::cairo::FontWeight::Bold,
                );
                context.set_font_size(13.0);
                let Ok(ext) = context.text_extents(name) else {
                    continue;
                };

                if is_capsule_complete {
                    // Completed: subtle green background tint + green border + checkmark
                    theme::set_source_rgba(context, theme::GREEN, 0.12);
                    theme::rounded_rectangle(context, cap_x, y_cap, w_cap, h_cap, 15.0);
                    let _ = context.fill_preserve();

                    context.set_line_width(1.5);
                    theme::set_source_rgba(context, theme::GREEN, 0.80);
                    let _ = context.stroke();

                    // Checkmark indicator at left
                    let chk_x = cap_x + 18.0;
                    context.set_line_width(2.0);
                    context.set_line_cap(gtk::cairo::LineCap::Round);
                    context.set_line_join(gtk::cairo::LineJoin::Round);
                    theme::set_source_rgb(context, theme::GREEN);
                    context.move_to(chk_x - 3.2, center_y);
                    context.line_to(chk_x - 0.8, center_y + 2.5);
                    context.line_to(chk_x + 3.6, center_y - 2.8);
                    let _ = context.stroke();

                    // Centered text in remaining width
                    let tx = cap_x + 22.0 + (w_cap - 22.0 - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgb(context, theme::GREEN);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                } else if is_capsule_active {
                    // Active: ENTIRE PILL IS ANIMATED with circulating border beam & interior spotlight
                    let r = 15.0_f64;
                    theme::set_source_rgba(context, theme::BASE, 0.85);
                    theme::rounded_rectangle(context, cap_x, y_cap, w_cap, h_cap, r);
                    let _ = context.fill_preserve();

                    // Base outline
                    context.set_line_width(1.2);
                    theme::set_source_rgba(context, theme::SURFACE1, 0.35);
                    let _ = context.stroke();

                    // 1. Interior ambient spotlight following the beam around the pill
                    let (head_x, head_y) =
                        theme::capsule_point(cap_x, y_cap, w_cap, h_cap, r, phase);
                    let _ = context.save();
                    theme::rounded_rectangle(
                        context,
                        cap_x + 1.0,
                        y_cap + 1.0,
                        w_cap - 2.0,
                        h_cap - 2.0,
                        r - 1.0,
                    );
                    context.clip();

                    let gradient =
                        gtk::cairo::RadialGradient::new(head_x, head_y, 0.0, head_x, head_y, 55.0);
                    let (sr, sg, sb) = theme::SAPPHIRE;
                    let (lr, lg, lb) = theme::LAVENDER;
                    gradient.add_color_stop_rgba(0.0, lr, lg, lb, 0.32);
                    gradient.add_color_stop_rgba(0.40, sr, sg, sb, 0.16);
                    gradient.add_color_stop_rgba(1.0, sr, sg, sb, 0.0);
                    let _ = context.set_source(&gradient);
                    let _ = context.paint();
                    let _ = context.restore();

                    // 2. Circulating luminous border beam orbiting the entire capsule perimeter
                    let beam_len = 0.35_f64;
                    let steps = 64;
                    context.set_line_cap(gtk::cairo::LineCap::Butt);
                    for s in 0..steps {
                        let u0 = phase - beam_len * (s as f64 / steps as f64);
                        let u1 = phase - beam_len * ((s as f64 + 1.25) / steps as f64);
                        let p0 = theme::capsule_point(cap_x, y_cap, w_cap, h_cap, r, u0);
                        let p1 = theme::capsule_point(cap_x, y_cap, w_cap, h_cap, r, u1);

                        let frac = 1.0 - (s as f64 / steps as f64);
                        let alpha = frac * frac * 0.95;
                        let width = 1.4 + 1.8 * frac;

                        context.set_line_width(width);
                        theme::set_source_rgba(context, theme::SAPPHIRE, alpha);
                        context.move_to(p0.0, p0.1);
                        context.line_to(p1.0, p1.1);
                        let _ = context.stroke();
                    }

                    // Soft rounded tip at the tail end
                    let tail_u = phase - beam_len;
                    let (tail_x, tail_y) =
                        theme::capsule_point(cap_x, y_cap, w_cap, h_cap, r, tail_u);
                    context.arc(tail_x, tail_y, 0.7, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::SAPPHIRE, 0.12);
                    let _ = context.fill();

                    // Subtle soft flare at head
                    context.arc(head_x, head_y, 2.0, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::SKY, 0.70);
                    let _ = context.fill();
                    context.arc(head_x, head_y, 0.9, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::TEXT, 0.80);
                    let _ = context.fill();

                    // 3. Crisp centered label text
                    let tx = cap_x + (w_cap - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgba(context, theme::CRUST, 0.85);
                    context.move_to(tx + 1.0, ty + 1.0);
                    let _ = context.show_text(name);
                    theme::set_source_rgb(context, theme::TEXT);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                } else {
                    // Pending: Dim capsule with muted centered label
                    theme::set_source_rgba(context, theme::MANTLE, 0.40);
                    theme::rounded_rectangle(context, cap_x, y_cap, w_cap, h_cap, 15.0);
                    let _ = context.fill_preserve();

                    context.set_line_width(1.0);
                    theme::set_source_rgba(context, theme::SURFACE1, 0.35);
                    let _ = context.stroke();

                    let tx = cap_x + (w_cap - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgb(context, theme::OVERLAY0);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                }
            }
        });
        root.append(&indicator);

        let mut steps = Vec::new();
        for text in ["Evaluate", "Build", "Compare"] {
            let step = label(text, &["build-phase"], 0.5);
            steps.push(step);
        }
        let timeline = Self {
            root,
            steps,
            indicator,
            current,
            target,
            complete,
            pulse,
            animating: Rc::new(Cell::new(false)),
        };
        timeline.set_phase(BuildPhase::Evaluate);
        timeline
    }

    fn start_pulse(&self) {
        let animations_enabled = gtk::Settings::default()
            .is_none_or(|settings| settings.property::<bool>("gtk-enable-animations"));
        if !animations_enabled {
            return;
        }
        let pulse = Rc::clone(&self.pulse);
        self.indicator
            .add_tick_callback(move |indicator, frame_clock| {
                let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
                pulse.set((seconds / 1.4).fract());
                indicator.queue_draw();
                glib::ControlFlow::Continue
            });
    }

    fn set_complete(&self) {
        self.complete.set(true);
        self.current.set(2.0);
        self.target.set(2.0);
        for step in &self.steps {
            step.remove_css_class("active");
            step.add_css_class("complete");
        }
        self.indicator.queue_draw();
    }

    fn set_phase(&self, phase: BuildPhase) {
        let active = phase.index();
        self.complete.set(false);
        for (index, step) in self.steps.iter().enumerate() {
            step.remove_css_class("complete");
            step.remove_css_class("active");
            if index < active {
                step.add_css_class("complete");
            } else if index == active {
                step.add_css_class("active");
            }
        }
        self.target.set(active as f64);
        self.indicator.queue_draw();
        if (self.target.get() - self.current.get()).abs() < 0.001 {
            return;
        }
        let animations_enabled = gtk::Settings::default()
            .is_none_or(|settings| settings.property::<bool>("gtk-enable-animations"));
        if !animations_enabled {
            self.current.set(self.target.get());
            self.indicator.queue_draw();
            return;
        }
        if self.animating.replace(true) {
            return;
        }
        let current = Rc::clone(&self.current);
        let target = Rc::clone(&self.target);
        let animating = Rc::clone(&self.animating);
        let previous_frame = Cell::new(0_i64);
        self.indicator
            .add_tick_callback(move |indicator, frame_clock| {
                let frame_time = frame_clock.frame_time();
                let previous = previous_frame.replace(frame_time);
                if previous != 0 {
                    let elapsed = (frame_time - previous).clamp(0, 50_000) as f64 / 1_000_000.0;
                    current.set(advance_timeline(current.get(), target.get(), elapsed));
                    indicator.queue_draw();
                }
                if (target.get() - current.get()).abs() < 0.001 {
                    current.set(target.get());
                    animating.set(false);
                    indicator.queue_draw();
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
    }
}

fn advance_timeline(current: f64, target: f64, elapsed: f64) -> f64 {
    if (target - current).abs() < 0.001 {
        return target;
    }
    let blend = 1.0 - (-elapsed * 12.0).exp();
    current + (target - current) * blend
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphNodeState {
    Planned,
    Building,
    Downloading,
    Complete,
    Failed,
}

#[derive(Clone, Debug)]
struct GraphNodeStatus {
    state: GraphNodeState,
    detail: Option<String>,
    download: Option<(u64, u64)>,
}

#[derive(Default)]
struct BuildPlan {
    nodes: Vec<(String, GraphNodeState)>,
    indices: HashMap<String, usize>,
}

impl BuildPlan {
    fn update<'a>(
        &mut self,
        states: &HashMap<String, GraphNodeStatus>,
        ordered_paths: impl IntoIterator<Item = &'a str>,
    ) {
        for (path, state) in &mut self.nodes {
            if matches!(
                *state,
                GraphNodeState::Building | GraphNodeState::Downloading
            ) && !states.contains_key(path)
            {
                *state = GraphNodeState::Complete;
            }
        }
        for path in ordered_paths {
            let state = states[path].state;
            if let Some(index) = self.indices.get(path).copied() {
                self.nodes[index].1 = state;
            } else {
                let path = path.to_owned();
                self.indices.insert(path.clone(), self.nodes.len());
                self.nodes.push((path, state));
            }
        }
    }

    fn write_states(&self, states: &mut Vec<GraphNodeState>) {
        states.clear();
        states.extend(self.nodes.iter().map(|(_, state)| *state));
    }

    fn summary(&self) -> (usize, usize, usize, usize, usize) {
        let mut building = 0;
        let mut downloading = 0;
        let mut complete = 0;
        let mut planned = 0;
        let mut failed = 0;
        for (_, state) in &self.nodes {
            match state {
                GraphNodeState::Building => building += 1,
                GraphNodeState::Downloading => downloading += 1,
                GraphNodeState::Complete => complete += 1,
                GraphNodeState::Planned => planned += 1,
                GraphNodeState::Failed => failed += 1,
            }
        }
        (building, downloading, complete, planned, failed)
    }
}

#[derive(Default)]
struct ActivityOrder {
    positions: HashMap<String, usize>,
}

impl ActivityOrder {
    fn observe<'a>(&mut self, paths: impl IntoIterator<Item = &'a str>) {
        for path in paths {
            if !self.positions.contains_key(path) {
                self.positions.insert(path.to_owned(), self.positions.len());
            }
        }
    }

    fn position(&self, path: &str) -> usize {
        self.positions.get(path).copied().unwrap_or(usize::MAX)
    }
}

impl GraphNodeState {
    const fn class(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Building => "building",
            Self::Downloading => "downloading",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BuildPlanGrid {
    columns: usize,
    horizontal_gap: f64,
    vertical_gap: f64,
    cell_width: f64,
    cell_height: f64,
}

fn build_plan_grid(width: f64, height: f64, count: usize) -> BuildPlanGrid {
    const ROWS: usize = 5;
    const CELL: f64 = 9.0;
    const GAP: f64 = 3.0;
    let natural_columns = ((width + GAP) / (CELL + GAP)).floor().max(1.0) as usize;
    let columns = natural_columns.max(count.div_ceil(ROWS));
    let slot_width = width / columns as f64;
    let horizontal_gap = if columns == natural_columns {
        GAP
    } else {
        (slot_width * 0.2).min(1.0)
    };
    BuildPlanGrid {
        columns,
        horizontal_gap,
        vertical_gap: GAP,
        cell_width: (slot_width - horizontal_gap).clamp(0.5, CELL),
        cell_height: ((height - GAP * (ROWS - 1) as f64) / ROWS as f64).clamp(1.0, CELL),
    }
}

fn draw_build_plan(
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    states: &[GraphNodeState],
) {
    if states.is_empty() || width <= 0 || height <= 0 {
        return;
    }
    let width = f64::from(width);
    let height = f64::from(height);
    let grid = build_plan_grid(width, height, states.len());
    for (index, state) in states.iter().enumerate() {
        let row = index / grid.columns;
        let column = index % grid.columns;
        let (color, alpha) = match state {
            GraphNodeState::Planned => (theme::LAVENDER, 0.28),
            GraphNodeState::Building => (theme::YELLOW, 0.90),
            GraphNodeState::Downloading => (theme::SAPPHIRE, 0.90),
            GraphNodeState::Complete => (theme::GREEN, 0.85),
            GraphNodeState::Failed => (theme::RED, 0.95),
        };
        theme::set_source_rgba(context, color, alpha);
        let x = column as f64 * (grid.cell_width + grid.horizontal_gap);
        let y = row as f64 * (grid.cell_height + grid.vertical_gap);
        let radius = (grid.cell_width.min(grid.cell_height) * 0.35).min(2.0);
        theme::rounded_rectangle(context, x, y, grid.cell_width, grid.cell_height, radius);
        let _ = context.fill();
    }
}

fn build_node_states(progress: &NixBuildProgress) -> HashMap<String, GraphNodeStatus> {
    let mut states = progress
        .planned
        .iter()
        .map(|path| {
            (
                path.clone(),
                GraphNodeStatus {
                    state: GraphNodeState::Planned,
                    detail: None,
                    download: None,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    for path in &progress.completed {
        states
            .entry(path.clone())
            .and_modify(|status| status.state = GraphNodeState::Complete)
            .or_insert(GraphNodeStatus {
                state: GraphNodeState::Complete,
                detail: None,
                download: None,
            });
    }
    for path in &progress.failed {
        states
            .entry(path.clone())
            .and_modify(|status| {
                status.state = GraphNodeState::Failed;
                status.detail = Some("Build failed".to_owned());
            })
            .or_insert(GraphNodeStatus {
                state: GraphNodeState::Failed,
                detail: Some("Build failed".to_owned()),
                download: None,
            });
    }
    for item in &progress.items {
        let Some(path) = item.path.as_deref() else {
            continue;
        };
        let state = if item.failed > 0 {
            GraphNodeState::Failed
        } else if item.kind == 101 {
            GraphNodeState::Downloading
        } else {
            GraphNodeState::Building
        };
        let priority = |state| match state {
            GraphNodeState::Failed => 5,
            GraphNodeState::Downloading => 4,
            GraphNodeState::Building => 3,
            GraphNodeState::Complete => 2,
            GraphNodeState::Planned => 1,
        };
        if states
            .get(path)
            .is_some_and(|current| priority(current.state) > priority(state))
        {
            continue;
        }
        let detail = match state {
            GraphNodeState::Downloading if item.expected > 0 => Some(format!(
                "{} of {} downloaded",
                size(item.done as i64),
                size(item.expected as i64)
            )),
            GraphNodeState::Building => item.detail.clone(),
            GraphNodeState::Failed => Some("Build failed".to_owned()),
            _ => None,
        };
        states.insert(
            path.to_owned(),
            GraphNodeStatus {
                state,
                detail,
                download: (state == GraphNodeState::Downloading)
                    .then_some((item.done, item.expected)),
            },
        );
    }
    states
}
fn activity_name(text: &str) -> String {
    let Some(store_name) = text
        .split("/nix/store/")
        .nth(1)
        .and_then(|value| value.split(['\'', ' ']).next())
    else {
        return text.to_owned();
    };
    store_name
        .trim_end_matches(".drv")
        .split_once('-')
        .map_or(store_name, |(_, name)| name)
        .to_owned()
}

fn render_flake_fetches(container: &gtk::Box, inputs: &[String]) {
    clear(container);
    for input in inputs {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("flake-fetch");
        let spinner = gtk::Spinner::new();
        spinner.set_size_request(16, 16);
        spinner.start();
        row.append(&spinner);
        row.append(&label(input, &["flake-fetch-name"], 0.0));
        container.append(&row);
    }
}

fn render_build_progress(
    container: &gtk::Box,
    summary: &BuildSummary,
    progress: &NixBuildProgress,
    build_plan: &mut BuildPlan,
    activity_rows: &mut ActivityRows,
) {
    let states = build_node_states(progress);
    let mut plan = states.iter().collect::<Vec<_>>();
    plan.sort_by(|(left_path, _), (right_path, _)| {
        activity_name(left_path)
            .cmp(&activity_name(right_path))
            .then_with(|| left_path.cmp(right_path))
    });
    build_plan.update(&states, plan.iter().map(|(path, _)| path.as_str()));
    build_plan.write_states(&mut summary.map_states.borrow_mut());
    summary.map.queue_draw();
    activity_rows.update(container, &states, progress);
    let (building, downloading, complete, planned, failed) = build_plan.summary();
    let total = build_plan.nodes.len();
    summary.building.set_text(&format!("{building} building"));
    summary
        .downloading
        .set_text(&format!("{downloading} downloading"));
    summary.complete.set_text(&format!("{complete} complete"));
    summary.planned.set_text(&format!("{planned} queued"));
    summary.failed.set_text(&format!("{failed} failed"));
    summary.failed_group.set_visible(failed > 0);
    summary
        .count
        .set_text(&format!("{complete} of {total} complete"));
}

fn key_hints(hints: &[(&str, &str)]) -> gtk::FlowBox {
    let container = gtk::FlowBox::new();
    container.add_css_class("key-hints");
    container.set_halign(gtk::Align::Center);
    container.set_column_spacing(14);
    container.set_row_spacing(8);
    container.set_selection_mode(gtk::SelectionMode::None);
    for (key, description) in hints {
        let hint = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        hint.add_css_class("key-hint");
        hint.set_valign(gtk::Align::Center);
        let keycap = label(key, &["keycap"], 0.5);
        keycap.set_yalign(0.5);
        keycap.set_valign(gtk::Align::Center);
        hint.append(&keycap);
        hint.append(&label(description, &["key-hint-label"], 0.0));
        container.insert(&hint, -1);
    }
    container
}

fn show_chooser(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Result<Config, String>,
) {
    if !state.cancel() {
        return;
    }
    state.clear_actions();
    fit_window(window, (520, 220), (380, 180));
    clear(root);
    root.add_css_class("chooser-root");
    let Ok(config) = config else {
        let error = gtk::Label::new(config.err().as_deref());
        error.add_css_class("error");
        error.set_wrap(true);
        root.append(&error);
        return;
    };

    let chooser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    chooser.add_css_class("chooser");
    chooser.set_vexpand(true);

    let header_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header_bar.add_css_class("chooser-header");
    let prompt_icon = gtk::Image::from_icon_name("emblem-system-symbolic");
    prompt_icon.add_css_class("chooser-prompt-icon");
    prompt_icon.set_pixel_size(16);
    prompt_icon.set_valign(gtk::Align::Center);
    header_bar.append(&prompt_icon);
    let prompt_label = label("Switch Configuration", &["chooser-prompt-label"], 0.0);
    prompt_label.set_hexpand(true);
    header_bar.append(&prompt_label);
    chooser.append(&header_bar);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("chooser-divider");
    chooser.append(&separator);

    let actions = gtk::Box::new(gtk::Orientation::Vertical, 4);
    actions.add_css_class("chooser-list");
    let mut targets = vec![("NixOS", Target::NixOs, "drive-harddisk-symbolic", "N")];
    if config.home_flake.is_some() {
        targets.insert(
            0,
            (
                "Home Manager",
                Target::HomeManager,
                "user-home-symbolic",
                "M",
            ),
        );
    }
    let mut first_button: Option<gtk::Button> = None;
    for (name, target, icon_name, key_str) in targets {
        let button = gtk::Button::new();
        button.add_css_class("chooser-button");
        button.add_css_class("chooser-row");
        first_button.get_or_insert_with(|| button.clone());

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_valign(gtk::Align::Center);

        let icon = gtk::Image::from_icon_name(icon_name);
        icon.add_css_class("chooser-row-icon");
        icon.set_pixel_size(18);
        row.append(&icon);

        let title = label(name, &["chooser-row-title"], 0.0);
        row.append(&title);

        let detail_text = match target {
            Target::HomeManager => config.home_flake.as_deref().unwrap_or("User environment"),
            Target::NixOs => &config.nixos_flake,
        };
        let detail = label(&format!("#{detail_text}"), &["chooser-row-detail"], 0.0);
        detail.set_hexpand(true);
        row.append(&detail);

        let keycap = label(key_str, &["keycap", "chooser-row-key"], 0.5);
        row.append(&keycap);

        button.set_child(Some(&row));
        let window = window.clone();
        let root = root.clone();
        let callback_state = Rc::clone(&state);
        let config = config.clone();
        button.connect_clicked(move |_| {
            start_build(
                &window,
                &root,
                Rc::clone(&callback_state),
                config.clone(),
                target,
            );
        });
        actions.append(&button);
        state.chooser_buttons.borrow_mut().push(button);
    }
    chooser.append(&actions);
    let hints = [("Esc", "Close")];
    let hints_widget = key_hints(&hints);
    hints_widget.set_margin_top(10);
    chooser.append(&hints_widget);
    root.append(&chooser);
    if let Some(button) = first_button {
        button.grab_focus();
    }
}

fn start_build(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Config,
    target: Target,
) {
    let Some((generation, cancellation)) = state.begin(Operation::Building) else {
        return;
    };
    state.clear_actions();
    fit_window(window, (880, 560), (720, 480));
    clear(root);
    root.remove_css_class("chooser-root");
    let flake = match target {
        Target::HomeManager => config.home_flake.as_deref().unwrap_or("<unset>"),
        Target::NixOs => &config.nixos_flake,
    };
    let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    header.add_css_class("report-header");
    let header_row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let title_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    title_slot.set_width_request(170);
    title_slot.set_hexpand(true);
    title_slot.append(&title("Swix"));
    header_row.append(&title_slot);
    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.start_pulse();
    header_row.append(&timeline.root);
    let action_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    action_slot.set_width_request(170);
    action_slot.set_hexpand(true);
    header_row.append(&action_slot);
    header.append(&header_row);
    header.append(&target_subtitle(target, flake));
    root.append(&header);
    let visualization = gtk::Box::new(gtk::Orientation::Vertical, 12);
    visualization.add_css_class("build-visualization");
    visualization.set_vexpand(true);
    let build_heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let phase_spinner = gtk::Spinner::new();
    phase_spinner.set_size_request(18, 18);
    phase_spinner.start();
    build_heading.append(&phase_spinner);
    let status = gtk::Label::new(Some(match target {
        Target::HomeManager => "Evaluating Home Manager and computing changes...",
        Target::NixOs => "Evaluating NixOS and computing changes...",
    }));
    status.add_css_class("loading-text");
    status.set_xalign(0.0);
    status.set_hexpand(true);
    build_heading.append(&status);
    let elapsed = label("0s", &["graph-node-detail"], 1.0);
    build_heading.append(&elapsed);
    visualization.append(&build_heading);
    let overview = build_summary();
    overview.root.set_visible(false);
    visualization.append(&overview.root);
    let activity_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    activity_content.add_css_class("build-scroll-content");
    let evaluation_heading = label("Evaluation target", &["graph-heading"], 0.0);
    activity_content.append(&evaluation_heading);
    let evaluation_target = match target {
        Target::HomeManager => format!(
            ".#homeConfigurations.{}.activationPackage",
            config.home_flake.as_deref().unwrap_or("<unset>")
        ),
        Target::NixOs => format!(
            ".#nixosConfigurations.{}.config.system.build.toplevel",
            config.nixos_flake
        ),
    };
    let evaluation_activity = label(&evaluation_target, &["evaluation-activity"], 0.0);
    evaluation_activity.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    evaluation_activity.set_wrap(false);
    evaluation_activity.set_lines(1);
    evaluation_activity.set_height_request(24);
    activity_content.append(&evaluation_activity);
    let fetch_heading = label("Flake inputs", &["graph-heading"], 0.0);
    fetch_heading.set_visible(false);
    activity_content.append(&fetch_heading);
    let fetch_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    fetch_list.add_css_class("flake-fetches");
    fetch_list.set_visible(false);
    activity_content.append(&fetch_list);
    let graph_heading = label("In progress", &["graph-heading"], 0.0);
    graph_heading.set_visible(false);
    activity_content.append(&graph_heading);
    let activity_list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    activity_list.add_css_class("build-activities");
    activity_list.set_visible(false);
    activity_content.append(&activity_list);
    let activity_scroll = gtk::ScrolledWindow::new();
    activity_scroll.add_css_class("build-scroll");
    activity_scroll.set_vexpand(true);
    activity_scroll.set_propagate_natural_height(false);
    activity_scroll.set_min_content_height(260);
    activity_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    activity_scroll.set_child(Some(&activity_content));
    state.scroll.replace(Some(activity_scroll.vadjustment()));
    visualization.append(&activity_scroll);
    let latest_progress = Rc::new(RefCell::new(NixBuildProgress::default()));
    root.append(&visualization);

    let cancel = gtk::Button::with_label("Cancel build");
    cancel.add_css_class("cancel-button");
    cancel.set_halign(gtk::Align::Center);
    let cancel_window = window.clone();
    let cancel_root = root.clone();
    let cancel_state = Rc::clone(&state);
    cancel.connect_clicked(move |_| {
        cancel_state.cancel();
        show_chooser(
            &cancel_window,
            &cancel_root,
            Rc::clone(&cancel_state),
            load_config(),
        );
    });
    root.append(&cancel);

    let (sender, receiver) = mpsc::channel();
    let build_started = Instant::now();
    thread::spawn(move || {
        let result = build_report(&config, target, generation, &cancellation, |update| {
            let _ = sender.send(BuildEvent::Update(update));
        });
        let _ = sender.send(BuildEvent::Finished(result));
    });
    let window = window.downgrade();
    let root = root.downgrade();
    let mut activity_rows = ActivityRows::new(&activity_list);
    let mut build_plan = BuildPlan::default();
    let mut current_phase = BuildPhase::Evaluate;
    glib::timeout_add_local(Duration::from_millis(100), move || {
        elapsed.set_text(&format!("{}s", build_started.elapsed().as_secs()));
        let mut phase = None;
        let mut nix_update = None;
        let mut finished = None;
        for event in receiver.try_iter() {
            match event {
                BuildEvent::Update(BuildUpdate::Phase(build_phase, update)) => {
                    phase = Some((build_phase, update));
                }
                BuildEvent::Update(BuildUpdate::Nix(update)) => nix_update = Some(update),
                BuildEvent::Finished(result) => finished = Some(result),
            }
        }
        let render_progress = nix_update.is_some();
        if state.generation.get() == generation {
            if let Some((phase, message)) = phase {
                current_phase = phase;
                timeline.set_phase(phase);
                status.set_text(message);
            }
            if let Some(progress) = nix_update {
                if let Some(activity) = &progress.evaluation_activity {
                    evaluation_activity.set_text(activity);
                }
                let show_fetches =
                    current_phase == BuildPhase::Evaluate && !progress.flake_fetches.is_empty();
                fetch_heading.set_visible(show_fetches);
                fetch_list.set_visible(show_fetches);
                if show_fetches {
                    render_flake_fetches(&fetch_list, &progress.flake_fetches);
                }
                latest_progress.replace(*progress);
            }
            let has_work = {
                let progress = latest_progress.borrow();
                !progress.planned.is_empty() || !progress.items.is_empty()
            };
            if current_phase != BuildPhase::Evaluate {
                evaluation_heading.set_visible(false);
                evaluation_activity.set_visible(false);
                fetch_heading.set_visible(false);
                fetch_list.set_visible(false);
            }
            if current_phase != BuildPhase::Evaluate && has_work {
                phase_spinner.set_visible(false);
                graph_heading.set_visible(true);
                activity_list.set_visible(true);
                overview.root.set_visible(true);
            }
            if render_progress {
                render_build_progress(
                    &activity_list,
                    &overview,
                    &latest_progress.borrow(),
                    &mut build_plan,
                    &mut activity_rows,
                );
            }
        }
        if let Some(result) = finished {
            if state.finish(generation)
                && let (Some(window), Some(root)) = (window.upgrade(), root.upgrade())
            {
                match result {
                    Ok(report) => show_report(&window, &root, Rc::clone(&state), report),
                    Err(error) if error == command::CANCELLED => {
                        show_chooser(&window, &root, Rc::clone(&state), load_config())
                    }
                    Err(error) => show_error(&window, &root, Rc::clone(&state), &error),
                }
            }
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}

fn show_report(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    report: Report,
) {
    state.clear_actions();
    if report.changes.is_empty() {
        fit_window(window, (640, 260), (360, 200));
    } else {
        fit_window(window, (880, 720), (360, 420));
    }
    clear(root);
    root.remove_css_class("chooser-root");
    let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    header.add_css_class("report-header");
    let header_row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let title_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    title_slot.set_width_request(170);
    title_slot.set_hexpand(true);
    title_slot.append(&title("Swix"));
    header_row.append(&title_slot);
    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.set_complete();
    header_row.append(&timeline.root);

    let right_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    right_slot.set_width_request(170);
    right_slot.set_hexpand(true);
    header_row.append(&right_slot);
    header.append(&header_row);
    header.append(&report_subtitle(&report));
    root.append(&header);
    let requires_confirmation = requires_host_switch_confirmation(
        report.target,
        &report.flake,
        current_hostname().as_deref(),
    );
    let switch = gtk::Button::new();
    switch.add_css_class("switch-button");
    if requires_confirmation {
        switch.add_css_class("switch-danger");
        switch.set_tooltip_text(Some(
            "This configuration targets another host; click twice to activate it",
        ));
    } else {
        switch.set_tooltip_text(Some("Activate this configuration"));
    }
    let switch_label = label("Switch", &["switch-button-label"], 0.5);
    switch.set_child(Some(&switch_label));
    let target = match report.target {
        Target::HomeManager => "Home Manager",
        Target::NixOs => "NixOS",
    };
    let switch_activity = switch_animation(target, &report.flake, &state.appearance.sans_font);
    let summary = gtk::Box::new(gtk::Orientation::Vertical, 0);
    summary.add_css_class("summary");
    let switch_confirmation = connect_switch(
        &SwitchView {
            button: switch.clone(),
            label: switch_label.clone(),
            activity: switch_activity.clone(),
            summary: summary.clone(),
        },
        report.clone(),
        Rc::clone(&state),
        requires_confirmation,
    );
    state.switch_confirmation.replace(Some(switch_confirmation));
    root.append(&switch_activity.root);

    let counts = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let mut section_jumps = HashMap::new();
    for status in ChangeStatus::ALL {
        let count = report
            .changes
            .iter()
            .filter(|change| change.status == status)
            .count();
        if count > 0 {
            let class = status.class();
            let jump = label(&format!("{count} {class}"), &["summary-count", class], 0.0);
            jump.add_css_class("summary-jump");
            jump.set_tooltip_text(Some(&format!("Jump to {class}")));
            jump.set_cursor_from_name(Some("pointer"));
            counts.append(&jump);
            section_jumps.insert(status, jump);
        }
    }
    if report.changes.is_empty() {
        summary.set_vexpand(true);
        summary.set_valign(gtk::Align::Center);
        summary.append(&label("No changes to apply", &["no-changes"], 0.5));
    } else {
        summary.append(&counts);
    }
    if let Some(metrics) = report_metrics(&report) {
        summary.append(&metrics);
    }
    root.append(&summary);

    if !report.changes.is_empty() {
        let navigation = ReportNavigation {
            window: window.clone(),
            root: root.clone(),
            state: Rc::clone(&state),
            report: report.clone(),
        };
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("updates-list");
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let adjustment = scroller.vadjustment();
        let scroll_animation = Rc::new(Cell::new(0));
        for status in ChangeStatus::ALL {
            let changes = report
                .changes
                .iter()
                .filter(|change| change.status == status)
                .collect::<Vec<_>>();
            if !changes.is_empty() {
                let section = change_section(&navigation, status, &changes);
                if let Some(jump) = section_jumps.remove(&status) {
                    let target = section.clone();
                    let jump_scroller = scroller.clone();
                    let adjustment = adjustment.clone();
                    let scroll_animation = Rc::clone(&scroll_animation);
                    let click = gtk::GestureClick::new();
                    click.connect_released(move |_, _, _, _| {
                        animate_scroll_to(
                            &jump_scroller,
                            &adjustment,
                            f64::from(target.allocation().y()),
                            Rc::clone(&scroll_animation),
                        );
                    });
                    jump.add_controller(click);
                }
                list.append(&section);
            }
        }
        scroller.set_child(Some(&list));
        state.scroll.replace(Some(adjustment));
        root.append(&scroller);
    }
    let mut hints = if report.changes.is_empty() {
        Vec::new()
    } else {
        vec![("↑ / K", "Scroll Up"), ("↓ / J", "Scroll Down")]
    };
    if report.separate_home_manager {
        hints.push(("M", "Home Manager"));
    }
    hints.extend([
        ("N", "NixOS"),
        ("S", "Switch"),
        ("← / H", "Back"),
        ("Esc", "Close"),
    ]);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    footer.add_css_class("report-footer");
    footer.set_valign(gtk::Align::End);

    let back = gtk::Button::new();
    back.add_css_class("report-back-button");
    back.set_tooltip_text(Some("Back to target chooser (H or ←)"));
    let back_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    back_box.set_valign(gtk::Align::Center);
    let back_icon = gtk::Image::from_icon_name("go-previous-symbolic");
    back_icon.set_pixel_size(14);
    back_box.append(&back_icon);
    let back_label = label("Back", &["report-back-label"], 0.0);
    back_box.append(&back_label);
    back.set_child(Some(&back_box));
    back.set_valign(gtk::Align::Center);
    let back_window = window.clone();
    let back_root = root.clone();
    let back_state = Rc::clone(&state);
    back.connect_clicked(move |_| {
        show_chooser(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            load_config(),
        );
    });
    state.back_button.replace(Some(back.clone()));
    footer.append(&back);

    let hints_widget = key_hints(&hints);
    hints_widget.set_hexpand(true);
    hints_widget.set_halign(gtk::Align::Center);
    hints_widget.set_valign(gtk::Align::Center);
    footer.append(&hints_widget);

    switch.set_valign(gtk::Align::Center);
    footer.append(&switch);

    root.append(&footer);
}

fn report_subtitle(report: &Report) -> gtk::Box {
    target_subtitle(report.target, &report.flake)
}

fn target_subtitle(target: Target, flake: &str) -> gtk::Box {
    let subtitle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    subtitle.add_css_class("report-subtitle");
    subtitle.append(&label("╰── ", &["metric-branch", "subtitle-branch"], 0.0));
    let target = match target {
        Target::HomeManager => "Home Manager ",
        Target::NixOs => "NixOS ",
    };
    subtitle.append(&label(target, &["subtitle", "subtitle-kind"], 0.0));
    subtitle.append(&label(
        &format!("#{flake}"),
        &["subtitle", "flake-tag"],
        0.0,
    ));
    subtitle.append(&label(" flake", &["subtitle", "subtitle-suffix"], 0.0));
    subtitle
}

fn report_metrics(report: &Report) -> Option<gtk::Box> {
    let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    metrics.add_css_class("metrics");
    metrics.append(&label("╰── ", &["metric-branch"], 0.0));
    let mut count = 0;
    if let Some((old, new, added, removed)) = report.paths {
        let text = if added == 0 && removed == 0 {
            format!("{new}")
        } else {
            format!("{old} -> {new} (+{added}, -{removed})")
        };
        metrics.append(&metric("PATHS:", &text, count));
        count += 1;
    }
    if let Some((old, new)) = report.sizes {
        let difference = new - old;
        if difference == 0 {
            metrics.append(&metric("SIZE:", &size(new), count));
            count += 1;
        } else {
            let text = format!("{} -> {}", size(old), size(new));
            metrics.append(&metric("SIZE:", &text, count));
            count += 1;
            let difference_metric = metric("DIFF:", &signed_size(difference), count);
            difference_metric.add_css_class(match difference.cmp(&0) {
                std::cmp::Ordering::Greater => "increase",
                std::cmp::Ordering::Less => "decrease",
                std::cmp::Ordering::Equal => "unchanged",
            });
            metrics.append(&difference_metric);
            count += 1;
        }
    }
    (count > 0).then_some(metrics)
}

fn metric(name: &str, value: &str, index: usize) -> gtk::Box {
    let metric = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    metric.add_css_class("metric");
    if index > 0 {
        metric.set_margin_start(18);
    }
    metric.append(&label(name, &["metric-label"], 0.0));
    metric.append(&label(value, &["metric-value"], 0.0));
    metric
}

fn change_section(
    navigation: &ReportNavigation,
    status: ChangeStatus,
    changes: &[&Change],
) -> gtk::Box {
    let class = status.class();
    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.add_css_class("section");
    let title = label(status.label(), &["section-title", class], 0.0);
    title.set_halign(gtk::Align::Start);
    section.append(&title);
    let grid = gtk::Grid::new();
    grid.add_css_class("changes-grid");
    grid.set_column_spacing(12);
    grid.set_row_spacing(4);
    let paired = status.is_paired();
    let headers: &[&str] = if paired {
        &["Name", "Old Version", "New Version", "Size"]
    } else {
        &["Name", "Version", "Size"]
    };
    for (column, heading) in headers.iter().enumerate() {
        let heading = label(heading, &["header-cell"], 0.0);
        heading.set_max_width_chars(if column == 0 { 44 } else { 28 });
        grid.attach(&heading, column as i32, 0, 1, 1);
    }
    for (index, change) in changes.iter().enumerate() {
        attach_change(navigation, &grid, change, paired, index as i32 + 1);
    }
    section.append(&grid);
    section
}

fn attach_change(
    navigation: &ReportNavigation,
    grid: &gtk::Grid,
    change: &Change,
    paired: bool,
    row: i32,
) {
    let class = change.status.class();
    if !has_changelog_versions(change) {
        let name = label(&change.name, &["cell", "name-cell"], 0.0);
        name.set_hexpand(true);
        name.set_max_width_chars(44);
        name.set_tooltip_text(Some(&change.name));
        grid.attach(&name, 0, row, 1, 1);
    } else {
        let name = gtk::Button::new();
        name.add_css_class("package-button");
        name.set_hexpand(true);
        name.set_halign(gtk::Align::Fill);
        name.set_tooltip_text(Some(&change.name));
        let name_text = label(&change.name, &["name-cell"], 0.0);
        name_text.set_hexpand(true);
        name_text.set_max_width_chars(44);
        name.set_child(Some(&name_text));
        let navigation = navigation.clone();
        let changelog_change = change.clone();
        name.connect_clicked(move |_| {
            if navigation.state.operation.get() == Operation::Idle {
                show_changelog(
                    &navigation.window,
                    &navigation.root,
                    &changelog_change,
                    Rc::clone(&navigation.state),
                    navigation.report.clone(),
                );
            }
        });
        grid.attach(&name, 0, row, 1, 1);
    }
    if paired {
        grid.attach(
            &version_label(
                &change.old,
                &["cell", "version-cell", "old-version-cell"],
                true,
            ),
            1,
            row,
            1,
            1,
        );
        grid.attach(
            &version_label(
                &change.new,
                &["cell", "version-cell", "new-version-cell"],
                true,
            ),
            2,
            row,
            1,
            1,
        );
        grid.attach(&change_size_label(change, class), 3, row, 1, 1);
    } else {
        let version = if change.new.is_empty() {
            &change.old
        } else {
            &change.new
        };
        grid.attach(
            &version_label(version, &["cell", "version-cell"], false),
            1,
            row,
            1,
            1,
        );
        grid.attach(&change_size_label(change, class), 2, row, 1, 1);
    }
}

fn version_label(text: &str, classes: &[&str], paired: bool) -> gtk::Label {
    let value = label(text, classes, 0.0);
    value.set_ellipsize(gtk::pango::EllipsizeMode::End);
    value.set_single_line_mode(true);
    value.set_max_width_chars(if paired { 28 } else { 36 });
    value.set_hexpand(true);
    value.set_tooltip_text(Some(text));
    value
}

fn change_size_label(change: &Change, class: &str) -> gtk::Label {
    let direction = match change.size.cmp(&0) {
        std::cmp::Ordering::Greater => "increase",
        std::cmp::Ordering::Less => "decrease",
        std::cmp::Ordering::Equal => "unchanged",
    };
    let value = label(
        &signed_size(change.size),
        &["cell", "size-cell", class, direction],
        1.0,
    );
    value.set_max_width_chars(12);
    value
}

fn has_changelog_versions(change: &Change) -> bool {
    !change.new.is_empty() && change.old != "..." && change.new != "..."
}

fn signed_size(bytes: i64) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let value = bytes.unsigned_abs() as f64;
    let (value, unit) = if value >= 1024.0 * 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0 * 1024.0), "GiB")
    } else if value >= 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0), "MiB")
    } else if value >= 1024.0 {
        (value / 1024.0, "KiB")
    } else {
        (value, "B")
    };
    format!("{sign}{value:.1} {unit}")
}

fn size(bytes: i64) -> String {
    signed_size(bytes).trim_start_matches('+').to_owned()
}

fn show_changelog(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    change: &Change,
    state: Rc<UiState>,
    report: Report,
) {
    state.clear_actions();
    clear(root);
    fit_window(window, (900, 760), (360, 480));
    let flake_dir = report.flake_dir.clone();

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let heading = title(&format!("{} release notes", change.name));
    heading.set_hexpand(true);
    header.append(&heading);
    let back = gtk::Button::with_label("Back to report");
    back.add_css_class("header-icon-button");
    let back_window = window.clone();
    let back_root = root.clone();
    let back_state = Rc::clone(&state);
    back.connect_clicked(move |_| {
        show_report(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            report.clone(),
        );
    });
    state.back_button.replace(Some(back.clone()));
    header.append(&back);
    root.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.add_css_class("changelog-content");
    let loading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    loading.add_css_class("changelog-loading");
    let spinner = gtk::Spinner::new();
    spinner.start();
    loading.append(&spinner);
    loading.append(&label(
        "Looking up release notes...",
        &["changelog-loading-label"],
        0.0,
    ));
    content.append(&loading);
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("changelog-scroll");
    scroller.set_vexpand(true);
    scroller.set_child(Some(&content));
    state.scroll.replace(Some(scroller.vadjustment()));
    root.append(&scroller);
    root.append(&key_hints(&[
        ("↑ / K", "Scroll Up"),
        ("↓ / J", "Scroll Down"),
        ("PgUp / PgDn", "Page"),
        ("← / H", "Back"),
        ("Esc", "Close"),
    ]));

    let cancellation = Arc::new(AtomicBool::new(false));
    state
        .view_cancellation
        .replace(Some(Arc::clone(&cancellation)));
    back.grab_focus();

    let name = change.name.clone();
    let version_spec = if change.old.is_empty() {
        change.new.clone()
    } else {
        format!("{}..{}", change.old, change.new)
    };
    let cache_key = format!("{}\0{name}\0{version_spec}", flake_dir.display());
    let changelog = state.changelog_cache.borrow().get(&cache_key).cloned();
    if let Some(changelog) = changelog {
        render_changelog(&content, &changelog);
        return;
    }
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let result = run(
            Command::new("nix-changelog")
                .env("NO_COLOR", "1")
                .arg("--json")
                .arg(name)
                .arg(version_spec)
                .current_dir(flake_dir),
            "nix-changelog",
            &worker_cancellation,
            CHANGELOG_TIMEOUT,
            8 * 1024 * 1024,
        )
        .and_then(|output| parse_changelog(&output.stdout));
        let _ = sender.send(result);
    });
    let cache_state = Rc::clone(&state);
    let callback_cancellation = Arc::clone(&cancellation);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if callback_cancellation.load(Ordering::Relaxed) {
            return glib::ControlFlow::Break;
        }
        match receiver.try_recv() {
            Ok(Ok(changelog)) => {
                render_changelog(&content, &changelog);
                cache_state
                    .changelog_cache
                    .borrow_mut()
                    .insert(cache_key.clone(), changelog);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear(&content);
                let message = label(&error, &["error", "changelog-error"], 0.0);
                message.set_wrap(true);
                message.set_selectable(true);
                content.append(&message);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear(&content);
                let message = label(
                    "changelog worker stopped without a result",
                    &["error", "changelog-error"],
                    0.0,
                );
                message.set_wrap(true);
                content.append(&message);
                glib::ControlFlow::Break
            }
        }
    });
}

fn show_error(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<UiState>, message: &str) {
    state.clear_actions();
    fit_window(window, (1040, 760), (360, 420));
    clear(root);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let heading = title("Update failed");
    heading.set_hexpand(true);
    header.append(&heading);
    let back = gtk::Button::with_label("Back");
    let back_window = window.clone();
    let back_root = root.clone();
    let back_state = Rc::clone(&state);
    back.connect_clicked(move |_| {
        show_chooser(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            load_config(),
        );
    });
    header.append(&back);
    root.append(&header);
    let error = gtk::Label::new(Some(message));
    error.add_css_class("error");
    error.add_css_class("build-error");
    error.set_selectable(true);
    error.set_wrap(true);
    error.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    error.set_xalign(0.0);
    error.set_yalign(0.0);
    let scroll = gtk::ScrolledWindow::new();
    scroll.add_css_class("build-error-scroll");
    scroll.set_vexpand(true);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&error));
    state.scroll.replace(Some(scroll.vadjustment()));
    root.append(&scroll);
    root.append(&key_hints(&[("← / H", "Back"), ("Esc", "Close")]));
    back.grab_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gc_root() -> Arc<GcRoot> {
        Arc::new(GcRoot {
            path: env::temp_dir().join(format!("swix-test-gc-root-{}", std::process::id())),
        })
    }

    fn parse_test_report(json: &[u8]) -> Result<Report, String> {
        parse_report(
            ReportMetadata {
                target: Target::NixOs,
                flake: "host".to_owned(),
                flake_dir: PathBuf::from("/flake"),
                separate_home_manager: false,
                baseline: PathBuf::from("/nix/store/old-system"),
                output: PathBuf::from("/nix/store/new-system"),
                gc_root: test_gc_root(),
            },
            json,
        )
    }

    #[test]
    fn strips_only_shared_output_suffixes() {
        assert_eq!(
            compact_versions("1.6.5-bwrap", "1.8.3-bwrap"),
            ("1.6.5".to_owned(), "1.8.3".to_owned())
        );
        assert_eq!(
            compact_versions("1.2.0-rc1", "1.3.0"),
            ("1.2.0-rc1".to_owned(), "1.3.0".to_owned())
        );
        assert_eq!(compact_dix_version("10.3.2_fish-completions"), "10.3.2");
    }

    #[test]
    fn enables_changelogs_only_for_known_target_versions() {
        let change = |old: &str, new: &str| Change {
            status: ChangeStatus::Upgraded,
            name: "demo".to_owned(),
            old: old.to_owned(),
            new: new.to_owned(),
            size: 0,
        };
        assert!(has_changelog_versions(&change("1.0", "2.0")));
        assert!(has_changelog_versions(&change("", "2.0")));
        assert!(!has_changelog_versions(&change("1.0", "...")));
        assert!(!has_changelog_versions(&change("...", "2.0")));
        assert!(!has_changelog_versions(&change("1.0", "")));
    }

    #[test]
    fn parses_release_markdown_into_readable_blocks() {
        assert_eq!(
            markdown_blocks(
                "## @oh-my-pi/pi-agent-core\n\n### Fixed\n\n- Handle **empty** `--auth`\n\n[Details](https://example.test)\n\n---"
            ),
            vec![
                MarkdownBlock {
                    kind: MarkdownBlockKind::Heading(2),
                    markup: "@oh-my-pi/pi-agent-core".to_owned(),
                },
                MarkdownBlock {
                    kind: MarkdownBlockKind::Heading(3),
                    markup: "Fixed".to_owned(),
                },
                MarkdownBlock {
                    kind: MarkdownBlockKind::Item(1),
                    markup: "Handle <b>empty</b> <tt>--auth</tt>".to_owned(),
                },
                MarkdownBlock {
                    kind: MarkdownBlockKind::Paragraph,
                    markup: "<a href=\"https://example.test\">Details</a>".to_owned(),
                },
                MarkdownBlock {
                    kind: MarkdownBlockKind::Rule,
                    markup: String::new(),
                },
            ]
        );
    }

    #[test]
    fn maps_vim_and_action_shortcuts() {
        assert_eq!(character_action('j'), Some(KeyAction::ScrollDown));
        assert_eq!(character_action('k'), Some(KeyAction::ScrollUp));
        assert_eq!(character_action('g'), Some(KeyAction::ScrollHome));
        assert_eq!(character_action('G'), Some(KeyAction::ScrollEnd));
        assert_eq!(character_action('s'), Some(KeyAction::Switch));
        assert_eq!(character_action('x'), None);
    }

    #[test]
    fn only_cross_host_nixos_switches_require_confirmation() {
        assert!(!requires_host_switch_confirmation(
            Target::NixOs,
            "thor",
            Some("thor")
        ));
        assert!(!requires_host_switch_confirmation(
            Target::NixOs,
            "thor",
            Some("thor.example")
        ));
        assert!(requires_host_switch_confirmation(
            Target::NixOs,
            "odin",
            Some("thor")
        ));
        assert!(requires_host_switch_confirmation(
            Target::NixOs,
            "odin",
            None
        ));
        assert!(!requires_host_switch_confirmation(
            Target::HomeManager,
            "alice",
            Some("thor")
        ));
    }

    #[test]
    fn cross_host_switch_requires_two_actions() {
        let confirmed = Cell::new(false);
        assert!(!switch_confirmation_allows_activation(true, &confirmed));
        assert!(confirmed.get());
        assert!(switch_confirmation_allows_activation(true, &confirmed));
        assert!(!confirmed.get());
        assert!(switch_confirmation_allows_activation(false, &confirmed));
    }

    #[test]
    fn switching_cannot_be_cancelled_from_navigation() {
        let state = UiState::default();
        state.operation.set(Operation::Switching);
        assert!(!state.cancel());
        assert_eq!(state.operation.get(), Operation::Switching);
    }

    #[test]
    fn clear_actions_cancels_view_and_clears_state() {
        let state = UiState::default();
        let cancellation = Arc::new(AtomicBool::new(false));
        state.view_cancellation.replace(Some(Arc::clone(&cancellation)));
        state.clear_actions();
        assert!(cancellation.load(Ordering::Relaxed));
        assert!(state.view_cancellation.borrow().is_none());
        assert!(state.back_button.borrow().is_none());
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
        state.back_button.replace(Some(button));

        let button = state.back_button.borrow().clone();
        if let Some(button) = button {
            button.emit_clicked();
        }
        assert!(state.back_button.borrow().is_none());
    }

    #[test]
    fn timeline_animation_retargets_while_moving() {
        let first_step = advance_timeline(0.0, 1.0, 0.05);
        assert!(first_step > 0.0 && first_step < 1.0);

        let retargeted = advance_timeline(first_step, 2.0, 0.05);
        assert!(retargeted > first_step && retargeted < 2.0);

        let settled = (0..100).fold(retargeted, |position, _| {
            advance_timeline(position, 2.0, 0.05)
        });
        assert_eq!(settled, 2.0);
    }

    #[test]
    fn active_work_keeps_first_seen_order() {
        let mut order = ActivityOrder::default();
        order.observe(["bravo", "charlie"]);
        order.observe(["alpha", "bravo", "charlie"]);

        let mut active = ["alpha", "charlie", "bravo"];
        active.sort_by_key(|path| order.position(path));
        assert_eq!(active, ["bravo", "charlie", "alpha"]);
    }

    #[test]
    fn build_plan_keeps_nodes_and_updates_colors_in_place() {
        let status = |state| GraphNodeStatus {
            state,
            detail: None,
            download: None,
        };
        let mut plan = BuildPlan::default();
        let initial = HashMap::from([
            ("bravo".to_owned(), status(GraphNodeState::Building)),
            ("charlie".to_owned(), status(GraphNodeState::Planned)),
        ]);
        plan.update(&initial, ["bravo", "charlie"]);

        let next = HashMap::from([
            ("alpha".to_owned(), status(GraphNodeState::Downloading)),
            ("charlie".to_owned(), status(GraphNodeState::Building)),
        ]);
        plan.update(&next, ["alpha", "charlie"]);

        assert_eq!(
            plan.nodes,
            [
                ("bravo".to_owned(), GraphNodeState::Complete),
                ("charlie".to_owned(), GraphNodeState::Building),
                ("alpha".to_owned(), GraphNodeState::Downloading),
            ]
        );
        assert_eq!(plan.summary(), (1, 1, 1, 0, 0));
    }

    #[test]
    fn build_plan_grid_does_not_reflow_within_capacity() {
        assert_eq!(
            build_plan_grid(900.0, 48.0, 20),
            build_plan_grid(900.0, 48.0, 100)
        );
    }

    #[test]
    fn parses_host_override() {
        let args = |values: &[&str]| values.iter().map(std::ffi::OsString::from).collect();
        assert_eq!(
            parse_command_line(args(&["swix", "--host", "odin"])).unwrap(),
            Some("odin".to_owned())
        );
        assert_eq!(
            parse_command_line(args(&["swix", "--host=thor"])).unwrap(),
            Some("thor".to_owned())
        );
        assert_eq!(parse_command_line(args(&["swix"])).unwrap(), None);
        assert!(parse_command_line(args(&["swix", "--host"])).is_err());
        assert!(parse_command_line(args(&["swix", "--unknown"])).is_err());
    }

    #[test]
    fn tracks_nix_internal_json_progress() {
        let mut tracker = NixProgressTracker::default();
        tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"fetching \u001b[35;1mgit\u001b[0m input '\u001b[35;1mgit+file:///dotnix\u001b[0m'"}"#,
            )
            .unwrap();
        assert_eq!(tracker.snapshot().flake_fetches, ["git+file:///dotnix"]);
        assert!(tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"fetching git input 'git+file:///dotnix'"}"#,
            )
            .is_none());
        tracker
            .update(
                r#"@nix {"action":"start","id":0,"level":4,"text":"evaluating derivation 'git+file:///dotnix#nixosConfigurations.thor.config.system.build.toplevel'","type":0}"#,
            )
            .unwrap();
        let progress = tracker.snapshot();
        assert!(progress.evaluation_active);
        assert_eq!(
            progress.evaluation_activity.as_deref(),
            Some("git+file:///dotnix#nixosConfigurations.thor.config.system.build.toplevel")
        );
        assert!(progress.items.is_empty());
        assert!(
            tracker
                .update(r#"@nix {"action":"stop","id":99}"#)
                .is_none()
        );
        assert!(tracker
            .update(
                r#"@nix {"action":"start","id":99,"level":5,"text":"copying '/nix/store/hash-source/file' to the store","type":0}"#,
            )
            .is_none());
        tracker.update(r#"@nix {"action":"stop","id":0}"#);
        tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"these derivations will be built:\n  /nix/store/hash-demo-1.0.drv"}"#,
            )
            .unwrap();
        assert!(
            tracker
                .snapshot()
                .planned
                .contains("/nix/store/hash-demo-1.0.drv")
        );
        tracker.update(r#"@nix {"action":"start","id":1,"text":"","type":104}"#);
        tracker
            .update(r#"@nix {"action":"result","id":1,"type":105,"fields":[2,5,0,0]}"#)
            .unwrap();
        let progress = tracker.snapshot();
        assert_eq!((progress.builds.done, progress.builds.expected), (2, 5));

        for event in [
            r#"@nix {"action":"start","id":4,"fields":["/nix/store/hash-demo","https://cache.example"],"text":"fetching demo","type":108}"#,
            r#"@nix {"action":"start","id":5,"parent":4,"fields":["/nix/store/hash-demo"],"text":"copying demo","type":100}"#,
            r#"@nix {"action":"start","id":3,"parent":5,"text":"downloading https://cache.example/demo.nar","type":101}"#,
            r#"@nix {"action":"result","id":3,"type":105,"fields":[512,1024,0,0]}"#,
        ] {
            tracker.update(event).unwrap();
        }
        let progress = tracker.snapshot();
        assert_eq!(
            (progress.downloads.done, progress.downloads.expected),
            (512, 1024)
        );
        assert_eq!(
            (progress.items[0].done, progress.items[0].expected),
            (512, 1024)
        );
        assert_eq!(
            progress.items[0].path.as_deref(),
            Some("/nix/store/hash-demo")
        );
        tracker.update(r#"@nix {"action":"stop","id":3}"#);

        tracker
            .update(
                r#"@nix {"action":"start","id":2,"fields":["/nix/store/hash-demo-1.0.drv"],"text":"building '/nix/store/hash-demo-1.0.drv'","type":105}"#,
            )
            .unwrap();
        tracker
            .update(
                r#"@nix {"action":"result","id":2,"type":101,"fields":["\u001b[1m\u001b[92mChecking\u001b[0m \u001b[1mnum-complex\u001b[0m v0.4.6"]}"#,
            )
            .unwrap();
        assert_eq!(
            tracker.snapshot().items[0].detail.as_deref(),
            Some("Checking num-complex v0.4.6")
        );
        tracker.update(r#"@nix {"action":"stop","id":2}"#).unwrap();
        let progress = tracker.snapshot();
        assert!(progress.items.is_empty());
        assert!(progress.completed.contains("/nix/store/hash-demo-1.0.drv"));
    }

    #[test]
    fn extracts_originating_nix_build_error() {
        let stderr = br#"@nix {"action":"msg","level":1,"raw_msg":"warning: deprecated option"}
@nix {"action":"msg","level":0,"raw_msg":"linking '/nix/store/system_fish-completions/uptime.fish' to '/nix/store/.links/content-address' not allowed"}
@nix {"action":"msg","level":0,"raw_msg":"\u001b[31;1merror:\u001b[0m builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"}
@nix {"action":"msg","level":0,"raw_msg":"error: 2 dependencies of derivation '/nix/store/home-manager.drv' failed to build"}
@nix {"action":"msg","level":0,"raw_msg":"error: 1 dependencies of derivation '/nix/store/nixos-system-odin.drv' failed to build"}"#;
        assert_eq!(
            nix_error_message(stderr).as_deref(),
            Some(
                "error: builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"
            )
        );
        assert_eq!(
            strip_ansi("Resolving package\n\u{1b}[1;31m✗\u{1b}[0m failed"),
            "Resolving package\n✗ failed"
        );
    }

    #[test]
    fn parses_a_dix_upgrade() {
        let report = parse_test_report(
            br#"{"diffs":[{"name":"demo","status":"Upgraded","size_delta":10,"versions":[{"kind":"changed","old":{"name":"1.0-bin"},"new":{"name":"2.0-bin"}}]}]}"#,
        )
        .unwrap();
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].old, "1.0");
        assert_eq!(report.changes[0].new, "2.0");
    }

    #[test]
    fn parses_every_supported_dix_status_and_metrics() {
        let report = parse_test_report(
            br#"{
                "diffs": [
                    {"name":"a","status":"Added","size_delta":1,"versions":[{"kind":"added","version":{"name":"1"}}]},
                    {"name":"b","status":"Removed","size_delta":-2,"versions":[{"kind":"removed","version":{"name":"1"}}]},
                    {"name":"c","status":"Upgraded","size_delta":3,"versions":[{"kind":"changed","old":{"name":"1"},"new":{"name":"2"}}]},
                    {"name":"d","status":"Downgraded","size_delta":-4,"versions":[{"kind":"changed","old":{"name":"2"},"new":{"name":"1"}}]},
                    {"name":"e","status":"Changed","size_delta":0,"versions":[]},
                    {"name":"X-Restart-Triggers-dbus-broker","status":"Changed","size_delta":0,"versions":[{"kind":"amount_changed","version":{"name":"1"},"old_amount":1,"new_amount":2}]},
                    {"name":"chomp","status":"Downgraded","size_delta":0,"versions":[{"kind":"removed","version":{"name":"0.1.0"}}],"has_omitted_versions":true},
                    {"name":"graphics-drivers","status":"Downgraded","size_delta":0,"versions":[{"kind":"removed","version":{"name":"570.1"}},{"kind":"amount_changed","version":{"name":"565.2"},"old_amount":1,"new_amount":2}],"has_omitted_versions":false},
                    {"name":"abseil-cpp","status":"Downgraded","size_delta":-7721032,"versions":[{"kind":"removed","version":{"name":"20260107.1-dev"}},{"kind":"amount_changed","version":{"name":"20260107.1"},"old_amount":3,"new_amount":2}],"has_omitted_versions":false},
                    {"name":"dhcpcd","status":"Upgraded","size_delta":-511536,"versions":[{"kind":"added","version":{"name":"10.3.2_fish-completions"}},{"kind":"amount_changed","version":{"name":"10.3.2"},"old_amount":2,"new_amount":1}],"has_omitted_versions":false},
                    {"name":"util-linux","status":"Mixed","size_delta":141704,"versions":[{"kind":"changed","old":{"name":"2.42.3-dev"},"new":{"name":"2.42.3-man"}},{"kind":"removed","version":{"name":"2.42.3"}}],"has_omitted_versions":true}
                ],
                "paths":{"old":10,"new":11,"added":2,"removed":1},
                "size_old":100,
                "size_new":110
            }"#,
        )
        .unwrap();
        assert_eq!(report.changes.len(), 10);
        assert_eq!(report.changes[5].name, "chomp");
        assert_eq!(report.changes[5].old, "0.1.0");
        assert_eq!(report.changes[5].new, "...");
        assert_eq!(report.changes[6].name, "graphics-drivers");
        assert_eq!(report.changes[6].old, "570.1");
        assert_eq!(report.changes[6].new, "565.2");
        assert_eq!(report.changes[7].name, "abseil-cpp");
        assert_eq!(report.changes[7].status, ChangeStatus::Changed);
        assert!(report.changes[7].old.is_empty());
        assert_eq!(report.changes[7].new, "20260107.1");
        assert_eq!(report.changes[8].name, "dhcpcd");
        assert_eq!(report.changes[8].status, ChangeStatus::Changed);
        assert!(report.changes[8].old.is_empty());
        assert_eq!(report.changes[8].new, "10.3.2");
        assert_eq!(report.changes[9].name, "util-linux");
        assert_eq!(report.changes[9].status, ChangeStatus::Changed);
        assert!(report.changes[9].old.is_empty());
        assert_eq!(report.changes[9].new, "2.42.3");
        assert_eq!(report.paths, Some((10, 11, 2, 1)));
        assert_eq!(report.sizes, Some((100, 110)));
    }

    #[test]
    fn rejects_incomplete_or_unknown_dix_data() {
        assert!(parse_test_report(br#"{}"#).is_err());
        assert!(
            parse_test_report(
                br#"{"diffs":[{"name":"demo","status":"Unexpected","size_delta":0,"versions":[]}]}"#
            )
            .is_err()
        );
        assert!(
            parse_test_report(
                br#"{"diffs":[{"name":"demo","status":"Upgraded","size_delta":0,"versions":[]}]}"#
            )
            .is_err()
        );
        assert!(parse_test_report(br#"{"diffs":[],"size_old":1}"#).is_err());
    }

    #[test]
    fn validates_service_responses_and_changelogs() {
        assert_eq!(parse_service_response(b"OK\n"), Ok(()));
        assert_eq!(
            parse_service_response(b""),
            Err("Swix activation service closed without a response".to_owned())
        );
        assert!(parse_service_response(b"unexpected").is_err());
        assert!(parse_service_response(b"ERROR\n").is_err());

        let changelog = parse_changelog(
            br#"{"pname":"demo","version":"2.0","description":null,"releases":[]}"#,
        )
        .unwrap();
        assert_eq!(changelog.pname, "demo");
        assert!(parse_changelog(b"not json").is_err());
    }

    #[test]
    fn parses_toml_escaping_and_validates_values() {
        let config = parse_config(
            r#"
                flake_dir = "/tmp/flake"
                nixos_flake = "host"
                sans_font = "Font \"Quoted\""
                mono_font = "Mono"
                rounding = 12
            "#,
        )
        .unwrap();
        assert_eq!(config.sans_font, "Font \"Quoted\"");
        assert_eq!(config.symbol_font, "Symbols Nerd Font");
        assert_eq!(config.rounding, 12);
        assert!(
            parse_config(
                r#"flake_dir = "/tmp/flake"
                   nixos_flake = ""
                   rounding = 100"#
            )
            .is_err()
        );
    }
}
