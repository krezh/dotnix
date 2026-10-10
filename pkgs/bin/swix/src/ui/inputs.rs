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

#[derive(Clone, Copy)]
enum RevisionAction {
    Update,
    Rollback,
    Select,
}

struct InputsView {
    directory: PathBuf,
    window: gtk::ApplicationWindow,
    inputs: RefCell<Vec<FlakeInput>>,
    selected: RefCell<Option<String>>,
    revisions: RefCell<Vec<RevisionRecord>>,
    history_generation: Cell<u64>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    count: gtk::Label,
    detail_name: gtk::Label,
    detail_source: gtk::Label,
    detail_revision: gtk::Label,
    detail_date: gtk::Label,
    detail_follows: gtk::Label,
    history_list: gtk::Box,
    history_empty: gtk::Label,
    history_source: gtk::Label,
    history_status: gtk::Label,
    history_spinner: gtk::Spinner,
    revision_toggle: gtk::Button,
    revision_revealer: gtk::Revealer,
    revision_entry: gtk::Entry,
    apply_revision: gtk::Button,
    status: gtk::Label,
    spinner: gtk::Spinner,
    update_selected: gtk::Button,
    update_all: gtk::Button,
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
        self.history_source
            .set_text(&format!("Recent commits from {}", input.source));
        self.detail_revision
            .set_text(&short_revision(&input.revision));
        self.detail_revision.set_tooltip_text(Some(&input.revision));
        self.detail_date.set_text(
            &input
                .last_modified
                .and_then(format_date)
                .map_or_else(|| "Date unavailable".to_owned(), |date| date),
        );
        if let Some(follows) = &input.follows {
            self.detail_follows.set_text(&format!("Follows {follows}"));
            self.detail_follows.set_visible(true);
        } else {
            self.detail_follows.set_visible(false);
        }
        self.revision_toggle
            .set_sensitive(input.flake_ref.is_some());
        if changed {
            self.revision_revealer.set_reveal_child(false);
            self.revision_toggle.set_label("Set commit or tag…");
            self.revision_entry.set_text("");
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
        self.detail_revision.set_text("");
        self.detail_date.set_text("");
        self.detail_follows.set_visible(false);
        self.update_selected.set_sensitive(false);
        self.revision_toggle.set_sensitive(false);
        self.revision_revealer.set_reveal_child(false);
        self.revision_entry.set_sensitive(false);
        self.apply_revision.set_sensitive(false);
        self.revisions.borrow_mut().clear();
        self.clear_history("Select an input");
        self.history_source.set_text("");
        self.history_status.set_text("");
        self.history_spinner.stop();
        self.history_spinner.set_visible(false);
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
            self.status.set_visible(true);
            self.status.set_text(message);
        } else {
            self.spinner.stop();
            self.spinner.set_visible(false);
            self.action_energy.stop();
            self.status.set_visible(false);
        }
    }

    fn clear_history(&self, message: &str) {
        while let Some(child) = self.history_list.first_child() {
            self.history_list.remove(&child);
        }
        self.history_empty.set_text(message);
        self.history_empty.set_visible(true);
    }

    fn show_error(&self, error: &str) {
        self.status.add_css_class("error");
        self.status.set_text(error.lines().next().unwrap_or(error));
        self.status.set_visible(true);
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
    root.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    content.add_css_class("inputs-content");
    content.set_vexpand(true);

    let browser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browser.add_css_class("inputs-browser");
    browser.set_width_request(340);
    let search = gtk::SearchEntry::new();
    search.add_css_class("inputs-search");
    search.set_placeholder_text(Some("Filter inputs"));
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

    let details = gtk::Box::new(gtk::Orientation::Vertical, 14);
    details.add_css_class("inputs-detail");
    details.set_hexpand(true);
    details.set_vexpand(true);

    let update_selected = action_button(
        "Update to latest",
        "software-update-available-symbolic",
        true,
    );
    let action_energy = EnergyOverlay::new(&update_selected, 6.0);
    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    identity.add_css_class("inputs-identity");
    let identity_text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    identity_text.set_hexpand(true);
    let detail_name = label("", &["inputs-detail-name"], 0.0);
    identity_text.append(&detail_name);
    let detail_source = label("", &["inputs-detail-source"], 0.0);
    identity_text.append(&detail_source);
    identity.append(&identity_text);
    identity.append(&action_energy.root);
    details.append(&identity);

    let current_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    current_card.add_css_class("inputs-current-card");
    let current_heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let current_title = label("CURRENTLY LOCKED", &["inputs-eyebrow"], 0.0);
    current_title.set_hexpand(true);
    current_heading.append(&current_title);
    current_heading.append(&label("CURRENT", &["inputs-current-badge"], 0.5));
    current_card.append(&current_heading);
    let lock_meta = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    lock_meta.add_css_class("inputs-lock-meta");
    let detail_revision = label("", &["inputs-revision"], 0.0);
    detail_revision.set_selectable(true);
    lock_meta.append(&detail_revision);
    let detail_date = label("", &["inputs-detail-meta"], 0.0);
    detail_date.set_hexpand(true);
    lock_meta.append(&detail_date);
    let detail_follows = label("", &["inputs-follow-badge"], 0.5);
    lock_meta.append(&detail_follows);
    let revision_toggle = gtk::Button::with_label("Set commit or tag…");
    revision_toggle.add_css_class("inputs-link-button");
    lock_meta.append(&revision_toggle);
    current_card.append(&lock_meta);
    details.append(&current_card);

    let direct = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    direct.add_css_class("inputs-direct");
    let revision_entry = gtk::Entry::new();
    revision_entry.add_css_class("inputs-revision-entry");
    revision_entry.set_placeholder_text(Some("Commit, tag, or branch"));
    revision_entry.set_hexpand(true);
    direct.append(&revision_entry);
    let apply_revision = action_button("Apply", "go-jump-symbolic", false);
    apply_revision.set_sensitive(false);
    direct.append(&apply_revision);
    let revision_revealer = gtk::Revealer::new();
    revision_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    revision_revealer.set_transition_duration(140);
    revision_revealer.set_child(Some(&direct));
    details.append(&revision_revealer);

    let history_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    history_panel.add_css_class("inputs-history-panel");
    history_panel.set_vexpand(true);
    let history_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let history_copy = gtk::Box::new(gtk::Orientation::Vertical, 1);
    history_copy.set_hexpand(true);
    history_copy.append(&label("REMOTE REVISIONS", &["inputs-eyebrow"], 0.0));
    let history_source = label("", &["inputs-history-source"], 0.0);
    history_copy.append(&history_source);
    history_header.append(&history_copy);
    let history_spinner = gtk::Spinner::new();
    history_spinner.set_size_request(16, 16);
    history_header.append(&history_spinner);
    let history_status = label("", &["inputs-history-status"], 1.0);
    history_header.append(&history_status);
    history_panel.append(&history_header);
    let history_list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let history_empty = label("Fetching remote commits…", &["inputs-history-empty"], 0.0);
    history_list.append(&history_empty);
    let history_scroll = gtk::ScrolledWindow::new();
    history_scroll.add_css_class("inputs-history-scroll");
    history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    history_scroll.set_min_content_height(140);
    history_scroll.set_vexpand(true);
    history_scroll.set_child(Some(&history_list));
    history_panel.append(&history_scroll);
    details.append(&history_panel);
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
    let status = label("", &["inputs-status"], 1.0);
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.set_hexpand(true);
    status.set_visible(false);
    footer.append(&status);
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    spinner.set_size_request(18, 18);
    footer.append(&spinner);
    let update_all = action_button("Update all", "view-refresh-symbolic", false);
    footer.append(&update_all);
    root.append(&footer);

    let view = Rc::new(InputsView {
        directory,
        window: window.clone(),
        inputs: RefCell::new(Vec::new()),
        selected: RefCell::new(None),
        revisions: RefCell::new(Vec::new()),
        history_generation: Cell::new(0),
        list,
        search,
        count,
        detail_name,
        detail_source,
        detail_revision,
        detail_date,
        detail_follows,
        history_list,
        history_empty,
        history_source,
        history_status,
        history_spinner,
        revision_entry,
        revision_toggle,
        revision_revealer,
        apply_revision,
        status,
        spinner,
        update_selected,
        update_all,
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
    view.revision_toggle.connect_clicked(move |button| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let reveal = !view.revision_revealer.reveals_child();
        view.revision_revealer.set_reveal_child(reveal);
        button.set_label(if reveal {
            "Cancel"
        } else {
            "Set commit or tag…"
        });
        if reveal {
            view.revision_entry.grab_focus();
        } else {
            view.revision_entry.set_text("");
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
    view.clear_history("Fetching remote commits…");
    view.history_status.set_text("Loading…");
    view.history_status.remove_css_class("warning");
    view.history_spinner.set_visible(true);
    view.history_spinner.start();
    view.status.remove_css_class("error");
    view.status.set_visible(false);
    view.status.set_tooltip_text(None);
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
                    view.revisions.replace(records);
                    render_history(&view);
                    view.history_spinner.stop();
                    view.history_spinner.set_visible(false);
                }
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                if view.history_generation.get() == generation {
                    view.show_error(&error);
                    view.history_spinner.stop();
                    view.history_spinner.set_visible(false);
                    view.history_status.set_text("Unavailable");
                    view.clear_history("Remote revisions unavailable");
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if view.history_generation.get() == generation {
                    view.show_error("Remote revision fetch stopped without a result");
                    view.history_spinner.stop();
                    view.history_spinner.set_visible(false);
                    view.history_status.set_text("Unavailable");
                    view.clear_history("Remote revisions unavailable");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_history(view: &Rc<InputsView>) {
    view.clear_history("No other remote revisions found");
    view.history_status.remove_css_class("warning");
    let current_revision = view.current_input().map(|input| input.revision);
    let records = view.revisions.borrow();
    let current_position = current_revision
        .as_deref()
        .and_then(|revision| history_position(&records, revision));

    let available_count = if let Some((current_index, newer, older)) = current_position {
        if newer > 0 {
            append_history_section(&view.history_list, "NEWER THAN CURRENT");
            for position in 0..current_index {
                append_revision_row(view, &records[position], position, RevisionAction::Update);
            }
        }
        if older > 0 {
            append_history_section(&view.history_list, "ROLLBACK POINTS");
            for position in current_index + 1..records.len() {
                append_revision_row(view, &records[position], position, RevisionAction::Rollback);
            }
        }
        view.history_status
            .set_text(&format!("{newer} newer · {older} older"));
        newer + older
    } else {
        if !records.is_empty() {
            append_history_section(&view.history_list, "RECENT REMOTE COMMITS");
        }
        for (position, record) in records.iter().enumerate() {
            append_revision_row(view, record, position, RevisionAction::Select);
        }
        view.history_status.add_css_class("warning");
        view.history_status
            .set_text("Current commit not in recent history");
        records.len()
    };
    view.history_empty.set_visible(available_count == 0);
}

fn append_history_section(container: &gtk::Box, text: &str) {
    container.append(&label(text, &["inputs-history-section"], 0.0));
}

fn append_revision_row(
    view: &Rc<InputsView>,
    record: &RevisionRecord,
    position: usize,
    action: RevisionAction,
) {
    let row = revision_row(record, action);
    let weak = Rc::downgrade(view);
    let window = view.window.clone();
    row.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            apply_history_revision(&window, view, position);
        }
    });
    view.history_list.append(&row);
}

fn history_position(records: &[RevisionRecord], current: &str) -> Option<(usize, usize, usize)> {
    let index = records
        .iter()
        .position(|record| record.revision == current)?;
    Some((index, index, records.len().saturating_sub(index + 1)))
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
        "Setting {input_name} to {}…",
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
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 1);
    copy.append(&label(&input.name, &["input-row-name"], 0.0));
    let source = label(&input.source, &["input-row-source"], 0.0);
    source.set_tooltip_text(Some(&input.source));
    copy.append(&source);
    row.set_child(Some(&copy));
    row
}

fn revision_row(record: &RevisionRecord, action: RevisionAction) -> gtk::Button {
    let row = gtk::Button::new();
    row.add_css_class("inputs-history-row");
    let (action_text, action_class, tooltip) = match action {
        RevisionAction::Update => (
            "UPDATE",
            "inputs-update-badge",
            "Move this input forward to the selected remote revision",
        ),
        RevisionAction::Rollback => (
            "ROLLBACK",
            "inputs-rollback-badge",
            "Roll this input back to the selected remote revision",
        ),
        RevisionAction::Select => (
            "SELECT",
            "inputs-use-badge",
            "Lock this input to the selected remote revision",
        ),
    };
    row.set_tooltip_text(Some(tooltip));
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
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
    content.append(&label(action_text, &[action_class], 0.5));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn record(revision: &str) -> RevisionRecord {
        RevisionRecord {
            revision: revision.to_owned(),
            flake_ref: format!("github:owner/repo/{revision}"),
            date: String::new(),
            summary: String::new(),
        }
    }

    #[test]
    fn positions_remote_revisions_relative_to_the_current_lock() {
        let records = ["newer", "current", "older-1", "older-2"].map(record);
        assert_eq!(history_position(&records, "current"), Some((1, 1, 2)));
        assert_eq!(history_position(&records, "missing"), None);
    }
}
