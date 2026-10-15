use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{gdk, glib};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::backend::{App, Session};
use crate::config::Options;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Cancelled,
    Complete,
}

#[derive(Clone, PartialEq, Eq)]
enum Stage {
    Preparing,
    Closing,
    Forcing,
    Failed(String),
    Cancelling,
}

enum Command {
    Cancel,
    Force,
}

enum Event {
    Apps(Arc<[App]>),
    Stage(Stage),
}

struct View {
    monitor: gdk::Monitor,
    window: gtk::Window,
    spinner: gtk::Spinner,
    status: gtk::Label,
    count: gtk::Label,
    elapsed: gtk::Label,
    completion: gtk::Label,
    progress: gtk::ProgressBar,
    list: gtk::Box,
    error: gtk::Box,
    error_text: gtk::Label,
    cancel: gtk::Button,
    force: gtk::Button,
}

struct Interface {
    options: Options,
    commands: mpsc::Sender<Command>,
    cancelled: Arc<AtomicBool>,
    apps: RefCell<Arc<[App]>>,
    stage: RefCell<Stage>,
    views: RefCell<Vec<View>>,
    total: Cell<usize>,
    started: Instant,
    elapsed: Cell<u64>,
    animations: bool,
}

pub fn run(options: Options, session: Session) -> Result<(Outcome, Session), String> {
    gtk::init()
        .map_err(|error| format!("Cannot initialize the session-exit interface: {error}"))?;
    if !gtk4_layer_shell::is_supported() {
        return Err("Dusk requires a Wayland compositor with layer-shell support.".into());
    }
    let display = gdk::Display::default().ok_or("No Wayland display is available.")?;
    let monitors = display.monitors();
    if monitors.n_items() == 0 {
        return Err("No connected monitors are available; session exit was not started.".into());
    }

    let provider = gtk::CssProvider::new();
    provider.load_from_data(&format!(
        ":root {{ --dusk-card: rgba(30, 30, 46, {}); --dusk-panel: rgba(24, 24, 37, {}); }}\n{}",
        options.appearance.opacity,
        options.appearance.panel_opacity,
        include_str!("style.css"),
    ));
    // Keep the theme imported by gtk.css from making the surface opaque.
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );

    let main_loop = glib::MainLoop::new(None, false);
    let (commands, command_rx) = mpsc::channel();
    let (events, event_rx) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let apps: Arc<[App]> = Arc::from(session.apps());
    let interface = Rc::new(Interface {
        total: Cell::new(apps.len()),
        options,
        commands,
        cancelled: Arc::clone(&cancelled),
        apps: RefCell::new(Arc::clone(&apps)),
        stage: RefCell::new(Stage::Preparing),
        views: RefCell::new(Vec::new()),
        started: Instant::now(),
        elapsed: Cell::new(0),
        animations: gtk::Settings::default()
            .is_none_or(|settings| settings.property::<bool>("gtk-enable-animations")),
    });
    interface.sync_monitors(&monitors);
    let weak = Rc::downgrade(&interface);
    let monitor_signal = monitors.connect_items_changed(move |monitors, _, _, _| {
        if let Some(interface) = weak.upgrade() {
            interface.sync_monitors(monitors);
        }
    });
    let weak = Rc::downgrade(&interface);
    let display_signal = display.connect_closed(move |_, _| {
        if let Some(interface) = weak.upgrade() {
            interface.cancel();
        }
    });

    let worker = match thread::Builder::new()
        .name("dusk-session".into())
        .spawn(move || run_session(session, apps, command_rx, events, cancelled))
    {
        Ok(worker) => worker,
        Err(error) => {
            monitors.disconnect(monitor_signal);
            display.disconnect(display_signal);
            interface.destroy_windows();
            gtk::style_context_remove_provider_for_display(&display, &provider);
            return Err(format!("Cannot start the session-exit worker: {error}"));
        }
    };
    let worker = Rc::new(RefCell::new(Some(worker)));
    let timer_worker = Rc::clone(&worker);
    let timer_interface = Rc::clone(&interface);
    let timer_loop = main_loop.clone();
    let timer = glib::timeout_add_local(Duration::from_millis(50), move || {
        for event in event_rx.try_iter() {
            match event {
                Event::Apps(apps) => {
                    timer_interface
                        .total
                        .set(timer_interface.total.get().max(apps.len()));
                    timer_interface.apps.replace(apps);
                    for view in timer_interface.views.borrow().iter() {
                        timer_interface.render_apps(view);
                    }
                }
                Event::Stage(stage) => {
                    if !timer_interface.cancelled.load(Ordering::Acquire) {
                        timer_interface.set_stage(stage);
                    }
                }
            }
        }
        let elapsed = timer_interface.started.elapsed().as_secs();
        if timer_interface.elapsed.replace(elapsed) != elapsed {
            for view in timer_interface.views.borrow().iter() {
                update_elapsed(&view.elapsed, elapsed);
            }
        }
        if timer_worker
            .borrow()
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            timer_loop.quit();
        }
        glib::ControlFlow::Continue
    });

    main_loop.run();
    timer.remove();
    monitors.disconnect(monitor_signal);
    display.disconnect(display_signal);
    interface.destroy_windows();
    display.flush();
    gtk::style_context_remove_provider_for_display(&display, &provider);
    let worker = worker
        .borrow_mut()
        .take()
        .ok_or("Session-exit worker is unavailable.")?;
    let (outcome, session) = worker.join().map_err(|_| {
        "Session-exit worker stopped unexpectedly; the session was not exited.".to_owned()
    })?;
    let outcome = if interface.cancelled.load(Ordering::Acquire) {
        Outcome::Cancelled
    } else {
        outcome
    };
    Ok((outcome, session))
}

fn run_session(
    mut session: Session,
    mut previous: Arc<[App]>,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
    cancelled: Arc<AtomicBool>,
) -> (Outcome, Session) {
    if cancelled.load(Ordering::Acquire) {
        return (Outcome::Cancelled, session);
    }
    let _ = events.send(Event::Stage(Stage::Closing));
    let mut failed = match session.begin() {
        Ok(()) => false,
        Err(error) => {
            let _ = events.send(Event::Stage(Stage::Failed(error)));
            true
        }
    };
    loop {
        if cancelled.load(Ordering::Acquire) {
            return (Outcome::Cancelled, session);
        }
        if session.apps() != previous.as_ref() {
            previous = Arc::from(session.apps());
            let _ = events.send(Event::Apps(Arc::clone(&previous)));
        }
        if !failed && session.apps().is_empty() {
            return (Outcome::Complete, session);
        }
        let result = match commands.recv_timeout(Duration::from_millis(250)) {
            Ok(Command::Cancel) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return (Outcome::Cancelled, session);
            }
            Ok(Command::Force) if !failed => {
                if cancelled.load(Ordering::Acquire) {
                    return (Outcome::Cancelled, session);
                }
                let _ = events.send(Event::Stage(Stage::Forcing));
                match session.force() {
                    Ok(()) => {
                        let outcome = if cancelled.load(Ordering::Acquire) {
                            Outcome::Cancelled
                        } else {
                            Outcome::Complete
                        };
                        return (outcome, session);
                    }
                    Err(error) => Err(error),
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) if !failed => {
                if cancelled.load(Ordering::Acquire) {
                    return (Outcome::Cancelled, session);
                }
                session.poll()
            }
            _ => continue,
        };
        if let Err(error) = result {
            // After an IPC failure, only cancellation is safe; never infer completion.
            failed = true;
            let _ = events.send(Event::Stage(Stage::Failed(error)));
        }
    }
}

impl Interface {
    fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::AcqRel) {
            return;
        }
        self.set_stage(Stage::Cancelling);
        let _ = self.commands.send(Command::Cancel);
    }

    fn force(&self) {
        if !matches!(*self.stage.borrow(), Stage::Preparing | Stage::Closing) {
            return;
        }
        self.set_stage(Stage::Forcing);
        let _ = self.commands.send(Command::Force);
    }

    fn set_stage(&self, stage: Stage) {
        self.stage.replace(stage);
        for view in self.views.borrow().iter() {
            self.render_stage(view);
        }
    }

    fn sync_monitors(self: &Rc<Self>, monitors: &gtk::gio::ListModel) {
        let current: Vec<gdk::Monitor> = (0..monitors.n_items())
            .filter_map(|index| monitors.item(index)?.downcast().ok())
            .collect();
        let mut views = self.views.borrow_mut();
        views.retain(|view| {
            let keep = current.contains(&view.monitor);
            if !keep {
                view.window.destroy();
            }
            keep
        });
        for monitor in current {
            if !views.iter().any(|view| view.monitor == monitor) {
                let view = self.build_view(monitor);
                self.render_apps(&view);
                self.render_stage(&view);
                update_elapsed(&view.elapsed, self.elapsed.get());
                view.window.present();
                view.cancel.grab_focus();
                views.push(view);
            }
        }
        let empty = views.is_empty();
        drop(views);
        if empty {
            self.cancel();
        }
    }

    fn destroy_windows(&self) {
        for view in self.views.borrow_mut().drain(..) {
            view.window.destroy();
        }
    }

    fn build_view(self: &Rc<Self>, monitor: gdk::Monitor) -> View {
        let geometry = monitor.geometry();
        let window = gtk::Window::builder()
            .title("Dusk")
            .decorated(false)
            .build();
        window.add_css_class("dusk-window");
        if !self.animations {
            window.add_css_class("reduced-motion");
        }
        if geometry.width() < 560 || geometry.height() < 640 {
            window.add_css_class("compact");
        }
        window.init_layer_shell();
        window.set_namespace(Some("dusk"));
        window.set_monitor(Some(&monitor));
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_exclusive_zone(-1);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }
        crate::blur::install(&window);

        let wide = geometry.width() >= 820;
        let card = gtk::Box::new(gtk::Orientation::Vertical, 24);
        card.add_css_class("shutdown-card");
        card.set_halign(gtk::Align::Center);
        card.set_valign(gtk::Align::Center);
        card.set_width_request((geometry.width() - 64).clamp(200, 1040));
        card.set_margin_top(20);
        card.set_margin_bottom(20);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 20);
        let headings = gtk::Box::new(gtk::Orientation::Vertical, 4);
        headings.set_hexpand(true);
        let title = label(&self.options.top_label, "heading");
        title.set_wrap(true);
        title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        title.set_max_width_chars(1);
        headings.append(&title);
        let subtitle = label("Waiting for applications to close.", "subtitle");
        subtitle.set_wrap(true);
        subtitle.set_max_width_chars(1);
        headings.append(&subtitle);
        header.append(&headings);
        let activity = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        activity.add_css_class("activity");
        activity.set_valign(gtk::Align::Center);
        let spinner = gtk::Spinner::new();
        spinner.set_size_request(12, 12);
        activity.append(&spinner);
        let status = label("Preparing…", "status");
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status.set_max_width_chars(if wide { 24 } else { 12 });
        activity.append(&status);
        header.append(&activity);
        card.append(&header);

        let body = gtk::Grid::builder()
            .column_homogeneous(true)
            .column_spacing(14)
            .row_spacing(14)
            .build();
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
        panel.add_css_class("apps-panel");
        panel.set_hexpand(true);
        let panel_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        panel_header.add_css_class("panel-toolbar");
        let caption = label("APPLICATIONS", "eyebrow");
        caption.set_hexpand(true);
        panel_header.append(&caption);
        let count = label("", "app-count");
        panel_header.append(&count);
        panel.append(&panel_header);
        panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let app_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .min_content_height(if wide { 280 } else { 100 })
            .max_content_height((geometry.height() - 300).clamp(280, 420))
            .propagate_natural_height(true)
            .vexpand(true)
            .child(&list)
            .build();
        panel.append(&app_scroll);
        body.attach(&panel, 0, 0, if wide { 2 } else { 1 }, 1);

        let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 14);
        sidebar.set_hexpand(true);
        let summary = gtk::Box::new(gtk::Orientation::Vertical, 12);
        summary.add_css_class("summary-panel");
        summary.append(&label("SESSION", "eyebrow"));
        let time = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let elapsed = label("", "elapsed");
        time.append(&elapsed);
        let time_caption = label("elapsed", "summary-caption");
        time_caption.set_valign(gtk::Align::End);
        time.append(&time_caption);
        summary.append(&time);
        let progress = gtk::ProgressBar::new();
        progress.add_css_class("shutdown-progress");
        progress.update_property(&[gtk::accessible::Property::Label(
            "Application closure progress",
        )]);
        summary.append(&progress);
        let progress_note = label(
            if self.options.dry_run {
                "Preview only; no close requests sent."
            } else {
                "Applications can finish saving before exiting."
            },
            "summary-note",
        );
        progress_note.set_wrap(true);
        progress_note.set_max_width_chars(1);
        summary.append(&progress_note);
        summary.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let totals = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let caption = label("Applications", "summary-caption");
        caption.set_hexpand(true);
        totals.append(&caption);
        let completion = label("", "summary-value");
        totals.append(&completion);
        summary.append(&totals);
        sidebar.append(&summary);

        let warning = gtk::Box::new(gtk::Orientation::Vertical, 14);
        warning.add_css_class("warning-panel");
        warning.set_vexpand(true);
        warning.append(&label("UNSAVED CHANGES", "eyebrow"));
        let warning_text = label(
            "Cancel to return to your desktop and save unfinished work.",
            "warning-text",
        );
        warning_text.set_wrap(true);
        warning_text.set_max_width_chars(1);
        warning.append(&warning_text);
        let force_warning = label(
            "Force quit stops remaining applications immediately. Unsaved changes may be lost.",
            "warning-text",
        );
        force_warning.set_wrap(true);
        force_warning.set_max_width_chars(1);
        warning.append(&force_warning);
        let cancelled_note = label(
            "Already closed applications cannot be restored.",
            "summary-note",
        );
        cancelled_note.set_wrap(true);
        cancelled_note.set_max_width_chars(1);
        warning.append(&cancelled_note);
        sidebar.append(&warning);

        let error = gtk::Box::new(gtk::Orientation::Vertical, 8);
        error.add_css_class("error-panel");
        error.append(&label("SESSION EXIT PAUSED", "error-title"));
        let error_text = label("", "error-detail");
        error_text.set_wrap(true);
        error_text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        error_text.set_max_width_chars(1);
        error_text.set_selectable(true);
        let error_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .max_content_height(96)
            .propagate_natural_height(true)
            .child(&error_text)
            .build();
        error.append(&error_scroll);
        let guidance = label(
            "Your session will stay open. Cancel and check the Hyprland connection.",
            "error-detail",
        );
        guidance.set_wrap(true);
        guidance.set_max_width_chars(1);
        error.append(&guidance);
        error.set_visible(false);
        sidebar.append(&error);
        body.attach(
            &sidebar,
            if wide { 2 } else { 0 },
            if wide { 0 } else { 1 },
            1,
            1,
        );
        let body_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .max_content_height((geometry.height() - 240).clamp(160, 520))
            .propagate_natural_height(true)
            .child(&body)
            .build();
        card.append(&body_scroll);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.add_css_class("footer");
        let footer_text = if self.options.dry_run {
            "Preview only · your session will stay open."
        } else if self.options.no_exit {
            "Closes applications without ending your session."
        } else {
            "Your session ends when all applications close."
        };
        let hint = label(footer_text, "footer-hint");
        hint.set_hexpand(true);
        hint.set_ellipsize(gtk::pango::EllipsizeMode::End);
        hint.set_max_width_chars(1);
        hint.set_visible(geometry.width() >= 620);
        actions.append(&hint);
        let cancel = gtk::Button::with_label("Cancel");
        cancel.add_css_class("cancel-button");
        cancel.set_tooltip_text(Some(
            "Stop session exit and keep the desktop session open (Escape)",
        ));
        actions.append(&cancel);
        let force = gtk::Button::with_label("Force quit");
        force.add_css_class("force-button");
        force.set_tooltip_text(Some(
            "Immediately stop remaining applications. Unsaved changes may be lost.",
        ));
        actions.append(&force);
        card.append(&actions);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&card)
            .build();
        window.set_child(Some(&scroll));

        let weak = Rc::downgrade(self);
        cancel.connect_clicked(move |_| {
            if let Some(interface) = weak.upgrade() {
                interface.cancel();
            }
        });
        let weak = Rc::downgrade(self);
        force.connect_clicked(move |_| {
            if let Some(interface) = weak.upgrade() {
                interface.force();
            }
        });
        let weak = Rc::downgrade(self);
        window.connect_close_request(move |_| {
            if let Some(interface) = weak.upgrade() {
                interface.cancel();
            }
            glib::Propagation::Stop
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                if let Some(interface) = weak.upgrade() {
                    interface.cancel();
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(keys);
        View {
            monitor,
            window,
            spinner,
            status,
            count,
            elapsed,
            completion,
            progress,
            list,
            error,
            error_text,
            cancel,
            force,
        }
    }

    fn render_apps(&self, view: &View) {
        while let Some(child) = view.list.first_child() {
            view.list.remove(&child);
        }
        let apps = self.apps.borrow();
        view.count.set_text(&format!("{} remaining", apps.len()));
        let total = self.total.get();
        view.completion.set_text(&format!(
            "{} / {} closed",
            total.saturating_sub(apps.len()),
            total
        ));
        let fraction = if total == 0 {
            1.0
        } else {
            1.0 - apps.len() as f64 / total as f64
        };
        view.progress.set_fraction(fraction.clamp(0.0, 1.0));
        if apps.is_empty() {
            let empty = label("All clear. No applications are waiting.", "empty-state");
            empty.set_wrap(true);
            view.list.append(&empty);
            return;
        }
        for app in apps.iter() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.add_css_class("app-row");
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("app-dot");
            dot.set_valign(gtk::Align::Center);
            row.append(&dot);
            let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
            text.set_hexpand(true);
            let name = if app.class.is_empty() {
                "Application"
            } else {
                &app.class
            };
            let name = label(name, "app-name");
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name.set_max_width_chars(1);
            text.append(&name);
            let title = if app.title.is_empty() {
                "Waiting for this process to exit"
            } else {
                &app.title
            };
            let title = label(title, "app-title");
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title.set_max_width_chars(1);
            text.append(&title);
            row.append(&text);
            let pid = label(&app.pid.to_string(), "app-pid");
            pid.set_tooltip_text(Some("Process ID"));
            row.append(&pid);
            row.set_tooltip_text(Some(&format!(
                "{}\n{}\nProcess {}",
                app.class, app.title, app.pid
            )));
            view.list.append(&row);
        }
    }

    fn render_stage(&self, view: &View) {
        let stage = self.stage.borrow();
        let active = matches!(*stage, Stage::Preparing | Stage::Closing | Stage::Forcing);
        view.spinner.set_visible(active && self.animations);
        view.spinner.set_spinning(active && self.animations);
        view.force
            .set_sensitive(matches!(*stage, Stage::Preparing | Stage::Closing));
        view.cancel
            .set_sensitive(!matches!(*stage, Stage::Cancelling));
        view.error.set_visible(matches!(*stage, Stage::Failed(_)));
        let text = match &*stage {
            Stage::Preparing => "Preparing…",
            Stage::Closing if self.options.dry_run => "Dry run",
            Stage::Closing => "Closing applications",
            Stage::Forcing => "Force quitting…",
            Stage::Failed(error) => {
                view.error_text.set_text(error);
                view.cancel.grab_focus();
                "Session exit paused"
            }
            Stage::Cancelling => "Cancelling…",
        };
        view.status.set_text(text);
    }
}

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class(class);
    label
}

fn update_elapsed(label: &gtk::Label, seconds: u64) {
    label.set_text(&format!("{}:{:02}", seconds / 60, seconds % 60));
}
