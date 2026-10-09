use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::inputs::{self, FlakeInput, RevisionRecord};
use crate::state::{Operation, UiState};
use crate::ui::common::{action_close_button, centered_icon, clear, fit_window, label, title};
use crate::ui::home::back_to_home_button;
use crate::ui::timeline::EnergyOverlay;

struct InputsView {
    directory: PathBuf,
    window: gtk::ApplicationWindow,
    inputs: RefCell<Vec<FlakeInput>>,
    selected: RefCell<Option<String>>,
    revisions: RefCell<Vec<RevisionRecord>>,
    revision_index: Cell<usize>,
    history_generation: Cell<u64>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    count: gtk::Label,
    source_kind: gtk::Label,
    detail_name: gtk::Label,
    detail_source: gtk::Label,
    detail_revision: gtk::Label,
    detail_date: gtk::Label,
    detail_follows: gtk::Label,
    history_position: gtk::Label,
    history_list: gtk::Box,
    history_empty: gtk::Label,
    revision_entry: gtk::Entry,
    apply_revision: gtk::Button,
    status: gtk::Label,
    spinner: gtk::Spinner,
    update_selected: gtk::Button,
    update_all: gtk::Button,
    older: gtk::Button,
    newer: gtk::Button,
    action_energy: EnergyOverlay,
    state: Rc<UiState>,
}

impl InputsView {
    fn set_inputs(&self, inputs: Vec<FlakeInput>) {
        let previous = self.selected.borrow().clone();
        self.inputs.replace(inputs);
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        for input in self.inputs.borrow().iter() {
            self.list.append(&input_row(input));
        }
        self.count
            .set_text(&format!("{} direct inputs", self.inputs.borrow().len()));
        let selection = previous
            .filter(|name| self.inputs.borrow().iter().any(|input| &input.name == name))
            .or_else(|| self.inputs.borrow().first().map(|input| input.name.clone()));
        if let Some(name) = selection {
            self.select(&name);
        } else {
            self.clear_detail();
        }
        self.list.invalidate_filter();
    }

    fn current_input(&self) -> Option<FlakeInput> {
        let selected = self.selected.borrow();
        self.inputs
            .borrow()
            .iter()
            .find(|input| Some(&input.name) == selected.as_ref())
            .cloned()
    }

    fn select(&self, name: &str) {
        let mut child = self.list.first_child();
        while let Some(widget) = child {
            let next = widget.next_sibling();
            if let Ok(row) = widget.downcast::<gtk::ListBoxRow>()
                && row.widget_name() == name
            {
                self.list.select_row(Some(&row));
                self.show_detail(name);
                return;
            }
            child = next;
        }
    }

    fn show_detail(&self, name: &str) -> bool {
        let inputs = self.inputs.borrow();
        let Some(input) = inputs.iter().find(|input| input.name == name) else {
            return false;
        };
        let changed = self.selected.borrow().as_deref() != Some(name);
        self.selected.replace(Some(name.to_owned()));
        self.detail_name.set_text(&input.name);
        self.detail_source.set_text(&input.source);
        self.detail_source.set_tooltip_text(Some(&input.source));
        self.source_kind.set_text(&input.source_kind.to_uppercase());
        self.detail_revision.set_text(&input.revision);
        self.detail_revision.set_tooltip_text(Some(&input.revision));
        self.detail_date
            .set_text(&input.last_modified.and_then(format_date).map_or_else(
                || "Lock date unavailable".to_owned(),
                |date| format!("Locked {date}"),
            ));
        if let Some(follows) = &input.follows {
            self.detail_follows.set_text(&format!("Follows {follows}"));
            self.detail_follows.set_visible(true);
        } else {
            self.detail_follows.set_visible(false);
        }
        self.revision_entry.set_sensitive(input.flake_ref.is_some());
        self.apply_revision.set_sensitive(
            input.flake_ref.is_some() && !self.revision_entry.text().trim().is_empty(),
        );
        self.update_selected.set_sensitive(true);
        changed
    }

    fn clear_detail(&self) {
        self.selected.replace(None);
        self.detail_name.set_text("No input selected");
        self.detail_source.set_text("");
        self.source_kind.set_text("");
        self.detail_revision.set_text("");
        self.detail_date.set_text("");
        self.detail_follows.set_visible(false);
        self.update_selected.set_sensitive(false);
        self.revision_entry.set_sensitive(false);
        self.apply_revision.set_sensitive(false);
        self.revisions.borrow_mut().clear();
        self.render_history_state();
    }

    fn set_busy(&self, busy: bool, message: &str) {
        self.search.set_sensitive(!busy);
        self.list.set_sensitive(!busy);
        self.update_selected
            .set_sensitive(!busy && self.selected.borrow().is_some());
        self.update_all
            .set_sensitive(!busy && !self.inputs.borrow().is_empty());
        self.revision_entry
            .set_sensitive(!busy && self.current_input().is_some_and(|i| i.flake_ref.is_some()));
        self.apply_revision.set_sensitive(
            !busy
                && self.revision_entry.is_sensitive()
                && !self.revision_entry.text().trim().is_empty(),
        );
        self.status.remove_css_class("error");
        if busy {
            self.spinner.start();
            self.spinner.set_visible(true);
            self.action_energy.start();
            self.status.set_text(message);
        } else {
            self.spinner.stop();
            self.spinner.set_visible(false);
            self.action_energy.stop();
            self.refresh_history_buttons();
        }
    }

    fn refresh_history_buttons(&self) {
        let index = self.revision_index.get();
        let count = self.revisions.borrow().len();
        self.newer.set_sensitive(index > 0);
        self.older.set_sensitive(index + 1 < count);
        self.history_position.set_text(
            if count == 0 {
                "No repository history".to_owned()
            } else {
                format!("Revision {} of {count}", index + 1)
            }
            .as_str(),
        );
    }

    fn render_history_state(&self) {
        while let Some(child) = self.history_list.first_child() {
            self.history_list.remove(&child);
        }
        let empty = self.revisions.borrow().is_empty();
        self.history_empty.set_visible(empty);
        self.refresh_history_buttons();
    }

    fn show_error(&self, error: &str) {
        self.status.add_css_class("error");
        self.status.set_text(error.lines().next().unwrap_or(error));
        self.status.set_tooltip_text(Some(error));
    }
}

pub(crate) fn show_inputs(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    directory: PathBuf,
) {
    if state.operation.get() != Operation::Idle {
        return;
    }
    state.clear_actions();
    clear(root);
    root.remove_css_class("home-root");
    fit_window(window, (1240, 760), (900, 620));

    let (_, inputs) = match inputs::load(&directory) {
        Ok(result) => result,
        Err(error) => {
            show_load_error(window, root, state, &error);
            return;
        }
    };

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("inputs-header");
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.set_hexpand(true);
    heading.append(&title("Inputs"));
    let count = label("", &["inputs-count"], 0.0);
    heading.append(&count);
    header.append(&heading);
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    spinner.set_size_request(18, 18);
    header.append(&spinner);
    let status = label("Ready", &["inputs-status"], 1.0);
    status.set_width_chars(26);
    header.append(&status);
    root.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    content.add_css_class("inputs-content");
    content.set_vexpand(true);

    let browser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browser.add_css_class("inputs-browser");
    browser.set_width_request(380);
    let search = gtk::SearchEntry::new();
    search.add_css_class("inputs-search");
    search.set_placeholder_text(Some("Filter by input, source, or revision"));
    browser.append(&search);
    let list = gtk::ListBox::new();
    list.add_css_class("inputs-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.add_css_class("inputs-scroll");
    list_scroll.set_vexpand(true);
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_child(Some(&list));
    state.set_scroll_adjustment(list_scroll.vadjustment());
    browser.append(&list_scroll);
    content.append(&browser);

    let details = gtk::Box::new(gtk::Orientation::Vertical, 12);
    details.add_css_class("inputs-detail");
    details.set_hexpand(true);
    details.set_vexpand(true);

    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    identity.add_css_class("inputs-identity");
    let source_emblem = centered_icon("folder-remote-symbolic", 26, 46);
    source_emblem.add_css_class("inputs-source-emblem");
    source_emblem.set_halign(gtk::Align::Center);
    source_emblem.set_valign(gtk::Align::Center);
    identity.append(&source_emblem);
    let identity_text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    identity_text.set_hexpand(true);
    identity_text.append(&label("SELECTED INPUT", &["inputs-eyebrow"], 0.0));
    let detail_name = label("", &["inputs-detail-name"], 0.0);
    identity_text.append(&detail_name);
    let detail_source = label("", &["inputs-detail-source"], 0.0);
    identity_text.append(&detail_source);
    identity.append(&identity_text);
    let source_kind = label("", &["inputs-source-kind"], 0.5);
    source_kind.set_valign(gtk::Align::Start);
    identity.append(&source_kind);
    details.append(&identity);

    let revision_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    revision_card.add_css_class("inputs-revision-card");
    let revision_heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let revision_title = label("LOCKED REVISION", &["inputs-revision-label"], 0.0);
    revision_title.set_hexpand(true);
    revision_heading.append(&revision_title);
    let detail_follows = label("", &["inputs-follow-badge"], 0.5);
    revision_heading.append(&detail_follows);
    revision_card.append(&revision_heading);
    let detail_revision = label("", &["inputs-revision"], 0.0);
    detail_revision.set_selectable(true);
    revision_card.append(&detail_revision);
    let detail_date = label("", &["inputs-detail-meta"], 0.0);
    revision_card.append(&detail_date);
    details.append(&revision_card);

    let history_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    history_panel.add_css_class("inputs-history-panel");
    history_panel.set_vexpand(true);
    let history_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let history_heading = label("REVISION TIMELINE", &["inputs-eyebrow"], 0.0);
    history_heading.set_hexpand(true);
    history_header.append(&history_heading);
    let history_position = label("Loading history…", &["inputs-history-position"], 1.0);
    history_header.append(&history_position);
    history_panel.append(&history_header);
    let history_list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let history_empty = label(
        "Select an input with revision history",
        &["inputs-history-empty"],
        0.0,
    );
    history_list.append(&history_empty);
    let history_scroll = gtk::ScrolledWindow::new();
    history_scroll.add_css_class("inputs-history-scroll");
    history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    history_scroll.set_min_content_height(140);
    history_scroll.set_vexpand(true);
    history_scroll.set_child(Some(&history_list));
    history_panel.append(&history_scroll);
    let history_controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let newer = history_button("Newer", "go-previous-symbolic");
    let older = history_button("Older", "go-next-symbolic");
    history_controls.append(&newer);
    history_controls.append(&older);
    history_panel.append(&history_controls);
    details.append(&history_panel);

    let direct = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    direct.add_css_class("inputs-direct");
    let direct_copy = gtk::Box::new(gtk::Orientation::Vertical, 1);
    direct_copy.set_hexpand(true);
    direct_copy.append(&label("JUMP TO REVISION", &["inputs-eyebrow"], 0.0));
    direct_copy.append(&label(
        "Commit, tag, or branch resolved by Nix",
        &["inputs-detail-meta"],
        0.0,
    ));
    direct.append(&direct_copy);
    let revision_entry = gtk::Entry::new();
    revision_entry.add_css_class("inputs-revision-entry");
    revision_entry.set_placeholder_text(Some("revision or tag"));
    revision_entry.set_width_chars(18);
    direct.append(&revision_entry);
    let apply_revision = action_button("Apply", "go-jump-symbolic", false);
    apply_revision.set_sensitive(false);
    direct.append(&apply_revision);
    details.append(&direct);
    content.append(&details);
    root.append(&content);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("report-footer");
    let back = back_to_home_button(window, root, &state);
    footer.append(&back);
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));
    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    footer.append(&footer_spacer);
    let update_all = action_button("Update all", "view-refresh-symbolic", false);
    footer.append(&update_all);
    let update_selected = action_button(
        "Update selected",
        "software-update-available-symbolic",
        true,
    );
    let action_energy = EnergyOverlay::new(&update_selected, 6.0);
    footer.append(&action_energy.root);
    root.append(&footer);

    let view = Rc::new(InputsView {
        directory,
        window: window.clone(),
        inputs: RefCell::new(Vec::new()),
        selected: RefCell::new(None),
        revisions: RefCell::new(Vec::new()),
        revision_index: Cell::new(0),
        history_generation: Cell::new(0),
        list,
        search,
        count,
        source_kind,
        detail_name,
        detail_source,
        detail_revision,
        detail_date,
        detail_follows,
        history_position,
        history_list,
        history_empty,
        revision_entry,
        apply_revision,
        status,
        spinner,
        update_selected,
        update_all,
        older,
        newer,
        action_energy,
        state,
    });
    let view_owner = RefCell::new(Some(Rc::clone(&view)));
    header.connect_unrealize(move |_| {
        view_owner.borrow_mut().take();
    });
    view.set_inputs(inputs);
    connect_view(window, &view);
    if view.selected.borrow().is_some() {
        start_history(Rc::clone(&view));
    }
    view.search.grab_focus();
}

fn connect_view(window: &gtk::ApplicationWindow, view: &Rc<InputsView>) {
    let weak = Rc::downgrade(view);
    view.list.connect_row_selected(move |_, row| {
        if let (Some(view), Some(row)) = (weak.upgrade(), row)
            && view.show_detail(row.widget_name().as_str())
        {
            start_history(view);
        }
    });

    let weak = Rc::downgrade(view);
    view.list.set_filter_func(move |row| {
        let Some(view) = weak.upgrade() else {
            return false;
        };
        let query = view.search.text().to_lowercase();
        let name = row.widget_name();
        view.inputs
            .borrow()
            .iter()
            .any(|input| input.name == name && input_matches(input, &query))
    });
    let weak = Rc::downgrade(view);
    view.search.connect_search_changed(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        view.list.invalidate_filter();
        let query = view.search.text().to_lowercase();
        let current_matches = view.selected.borrow().as_ref().is_some_and(|name| {
            view.inputs
                .borrow()
                .iter()
                .any(|input| &input.name == name && input_matches(input, &query))
        });
        if current_matches {
            return;
        }
        let first = view
            .inputs
            .borrow()
            .iter()
            .find(|input| input_matches(input, &query))
            .map(|input| input.name.clone());
        if let Some(name) = first {
            view.select(&name);
            start_history(view);
        } else {
            view.list.unselect_all();
            view.clear_detail();
        }
    });

    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.update_selected.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            let input = view.selected.borrow().clone();
            start_latest_update(&callback_window, view, input);
        }
    });
    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.update_all.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            start_latest_update(&callback_window, view, None);
        }
    });

    let weak = Rc::downgrade(view);
    view.revision_entry.connect_changed(move |entry| {
        if let Some(view) = weak.upgrade() {
            view.apply_revision.set_sensitive(
                view.state.operation.get() == Operation::Idle
                    && view
                        .current_input()
                        .is_some_and(|input| input.flake_ref.is_some())
                    && !entry.text().trim().is_empty(),
            );
        }
    });
    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.apply_revision.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            start_manual_revision(&callback_window, view);
        }
    });
    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.revision_entry.connect_activate(move |_| {
        if let Some(view) = weak.upgrade() {
            start_manual_revision(&callback_window, view);
        }
    });

    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.older.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            let index = view.revision_index.get() + 1;
            apply_history_revision(&callback_window, view, index);
        }
    });
    let callback_window = window.clone();
    let weak = Rc::downgrade(view);
    view.newer.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            let index = view.revision_index.get().saturating_sub(1);
            apply_history_revision(&callback_window, view, index);
        }
    });
}

fn start_history(view: Rc<InputsView>) {
    let Some(input) = view.current_input() else {
        return;
    };
    view.state.cancel_view();
    let cancellation = Arc::new(AtomicBool::new(false));
    view.state.set_view_cancellation(&cancellation);
    let generation = view.history_generation.get().wrapping_add(1);
    view.history_generation.set(generation);
    view.revisions.borrow_mut().clear();
    view.render_history_state();
    view.history_position.set_text("Loading history…");
    view.status.remove_css_class("error");
    view.status
        .set_text(&format!("Reading {} history…", input.name));
    let directory = view.directory.clone();
    let input_name = input.name;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = inputs::history(&directory, &input_name, &cancellation);
        let _ = sender.send(result);
    });
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(records)) => {
                if view.history_generation.get() == generation {
                    let current_revision = view.current_input().map(|input| input.revision);
                    let index = current_revision
                        .as_ref()
                        .and_then(|revision| {
                            records
                                .iter()
                                .position(|record| &record.revision == revision)
                        })
                        .unwrap_or(0);
                    view.revisions.replace(records);
                    view.revision_index.set(index);
                    render_history(&view);
                    view.status.set_text("Revision history ready");
                }
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                if view.history_generation.get() == generation {
                    view.show_error(&error);
                    view.render_history_state();
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if view.history_generation.get() == generation {
                    view.show_error("Revision history stopped without a result");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_history(view: &Rc<InputsView>) {
    view.render_history_state();
    let index = view.revision_index.get();
    for (position, record) in view.revisions.borrow().iter().enumerate() {
        let row = revision_row(record, position == index);
        let weak = Rc::downgrade(view);
        let window = view.window.clone();
        row.connect_clicked(move |_| {
            if let Some(view) = weak.upgrade() {
                apply_history_revision(&window, view, position);
            }
        });
        view.history_list.append(&row);
    }
    view.history_empty
        .set_visible(view.revisions.borrow().is_empty());
    view.refresh_history_buttons();
}

fn start_latest_update(
    window: &gtk::ApplicationWindow,
    view: Rc<InputsView>,
    input: Option<String>,
) {
    let message = input.as_ref().map_or_else(
        || "Updating every input…".to_owned(),
        |name| format!("Updating {name}…"),
    );
    start_operation(window, view, message, move |directory, cancellation| {
        inputs::update(directory, input.as_deref(), cancellation)
    });
}

fn start_manual_revision(window: &gtk::ApplicationWindow, view: Rc<InputsView>) {
    let Some(input) = view.current_input() else {
        return;
    };
    let revision = view.revision_entry.text().to_string();
    let flake_ref = match inputs::revision_ref(&input, &revision) {
        Ok(reference) => reference,
        Err(error) => {
            view.show_error(&error);
            return;
        }
    };
    let input_name = input.name;
    let message = format!("Resolving {input_name} at {}…", revision.trim());
    start_operation(window, view, message, move |directory, cancellation| {
        inputs::update_at_revision(directory, &input_name, &flake_ref, cancellation)
    });
}

fn apply_history_revision(window: &gtk::ApplicationWindow, view: Rc<InputsView>, index: usize) {
    let Some(input) = view.current_input() else {
        return;
    };
    let Some(record) = view.revisions.borrow().get(index).cloned() else {
        return;
    };
    let input_name = input.name;
    let message = format!(
        "Restoring {input_name} to {}…",
        short_revision(&record.revision)
    );
    start_operation(window, view, message, move |directory, cancellation| {
        inputs::update_at_revision(directory, &input_name, &record.flake_ref, cancellation)
    });
}

fn start_operation(
    window: &gtk::ApplicationWindow,
    view: Rc<InputsView>,
    message: String,
    operation: impl FnOnce(&std::path::Path, &AtomicBool) -> Result<(), String> + Send + 'static,
) {
    let Some((generation, cancellation)) = view.state.begin(Operation::Updating) else {
        return;
    };
    view.state.cancel_view();
    view.set_busy(true, &message);
    let directory = view.directory.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = operation(&directory, &cancellation);
        let _ = sender.send(result);
    });
    let callback_window = window.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(result) => {
                view.set_busy(false, "");
                if view.state.finish(generation) {
                    if view.state.close_pending.replace(false) {
                        callback_window.close();
                    } else {
                        match result {
                            Ok(()) => match inputs::load(&view.directory) {
                                Ok((_, refreshed)) => {
                                    view.set_inputs(refreshed);
                                    view.revision_entry.set_text("");
                                    view.status.set_tooltip_text(None);
                                    view.status.set_text("Input revision applied");
                                    start_history(Rc::clone(&view));
                                }
                                Err(error) => view.show_error(&error),
                            },
                            Err(error) => view.show_error(&error),
                        }
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                view.set_busy(false, "");
                if view.state.finish(generation) {
                    view.show_error("The input operation stopped without a result");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

fn input_row(input: &FlakeInput) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("input-row");
    row.set_widget_name(&input.name);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    content.set_valign(gtk::Align::Center);
    let icon_container = centered_icon("folder-remote-symbolic", 15, 30);
    icon_container.add_css_class("input-row-icon");
    icon_container.set_halign(gtk::Align::Center);
    icon_container.set_valign(gtk::Align::Center);
    content.append(&icon_container);
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    copy.append(&label(&input.name, &["input-row-name"], 0.0));
    let source = label(&input.source, &["input-row-source"], 0.0);
    source.set_tooltip_text(Some(&input.source));
    copy.append(&source);
    content.append(&copy);
    let metadata = gtk::Box::new(gtk::Orientation::Vertical, 2);
    metadata.set_halign(gtk::Align::End);
    metadata.set_valign(gtk::Align::Center);
    metadata.append(&label(
        &short_revision(&input.revision),
        &["input-row-revision"],
        1.0,
    ));
    metadata.append(&label(&input.source_kind, &["input-row-kind"], 1.0));
    content.append(&metadata);
    row.set_child(Some(&content));
    row
}

fn revision_row(record: &RevisionRecord, active: bool) -> gtk::Button {
    let row = gtk::Button::new();
    row.add_css_class("inputs-history-row");
    if active {
        row.add_css_class("active");
    }
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let rail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    rail.add_css_class("inputs-history-rail");
    content.append(&rail);
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let revision = label(
        &short_revision(&record.revision),
        &["inputs-history-revision"],
        0.0,
    );
    revision.set_hexpand(true);
    heading.append(&revision);
    let date = history_date(&record.date);
    heading.append(&label(&date, &["inputs-history-date"], 1.0));
    copy.append(&heading);
    copy.append(&label(&record.summary, &["inputs-history-summary"], 0.0));
    content.append(&copy);
    if active {
        content.append(&label("CURRENT", &["inputs-current-badge"], 0.5));
    } else if record.commit.is_some() {
        content.append(&label("KNOWN", &["inputs-known-badge"], 0.5));
    }
    row.set_child(Some(&content));
    row
}

fn action_button(text: &str, icon_name: &str, primary: bool) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class(if primary {
        "inputs-primary-button"
    } else {
        "inputs-secondary-button"
    });
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    content.set_halign(gtk::Align::Center);
    content.append(&centered_icon(icon_name, 14, 14));
    content.append(&label(text, &["inputs-button-label"], 0.5));
    button.set_child(Some(&content));
    button
}

fn history_button(text: &str, icon_name: &str) -> gtk::Button {
    let button = action_button(text, icon_name, false);
    button.add_css_class("inputs-history-button");
    button.set_hexpand(true);
    button
}

fn input_matches(input: &FlakeInput, query: &str) -> bool {
    query.is_empty()
        || input.name.to_lowercase().contains(query)
        || input.source.to_lowercase().contains(query)
        || input.revision.to_lowercase().contains(query)
}

fn show_load_error(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    error: &str,
) {
    root.append(&title("Inputs"));
    let message = label(error, &["error", "build-error"], 0.0);
    message.set_wrap(true);
    message.set_ellipsize(gtk::pango::EllipsizeMode::None);
    message.set_vexpand(true);
    root.append(&message);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("report-footer");
    footer.append(&back_to_home_button(window, root, &state));
    footer.append(&action_close_button(
        window,
        state,
        "report-close-button",
        "report-close-label",
    ));
    root.append(&footer);
}

fn short_revision(revision: &str) -> String {
    if revision.len() > 12
        && revision
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        revision[..12].to_owned()
    } else {
        revision.to_owned()
    }
}

fn format_date(timestamp: i64) -> Option<String> {
    let date = glib::DateTime::from_unix_local(timestamp).ok()?;
    date.format("%Y-%m-%d").ok().map(|date| date.to_string())
}

fn history_date(value: &str) -> String {
    if let Ok(timestamp) = value.parse::<i64>() {
        return format_date(timestamp).unwrap_or_else(|| value.to_owned());
    }
    value.get(..10).unwrap_or(value).to_owned()
}
