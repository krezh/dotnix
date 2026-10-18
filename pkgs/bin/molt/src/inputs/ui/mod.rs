//! The inputs screen: the view state, its layout, and the wiring between list, timeline, and updates.

mod history;
mod list;
mod operations;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

use gtk::glib;
use gtk::prelude::*;

use crate::app::state::{Operation, UiState};
use crate::build::Target;
use crate::build::ui::{AfterBuild, start_build};
use crate::config::Config;
use crate::home::ui::back_to_home_button;
use crate::inputs::{self, FlakeInput, RevisionRecord};
use crate::ui::EnergyOverlay;
use crate::ui::theme;
use crate::ui::{
    FadeScroll, MorphLabel, SegmentedControl, action_close_button, centered_icon, duration_text,
    fade_in, field, fit_screen, label, replace_screen, title,
};

use history::start_history;
use list::{
    compare_inputs, input_matches, input_row, input_row_content, list_rows, lock_age,
    resort_animated, stop_resort,
};
use operations::{
    InputOperationResult, set_transaction_actions, start_latest_update, start_manual_revision,
    start_operation,
};

/// Locks older than this are flagged in the list as worth updating.
const STALE_AFTER: u64 = 60 * 86_400;

struct InputsView {
    directory: PathBuf,
    config: Config,
    root: gtk::Box,
    window: gtk::ApplicationWindow,
    inputs: RefCell<Vec<FlakeInput>>,
    selected: RefCell<Option<String>>,
    revisions: RefCell<Vec<RevisionRecord>>,
    history_generation: Cell<u64>,
    /// Per-row stylesheet and current vertical offset used to glide rows after a re-sort.
    row_offsets: RefCell<HashMap<String, (gtk::CssProvider, f64)>>,
    resort_animation: Cell<u64>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    count: MorphLabel,
    sort_oldest: gtk::ToggleButton,
    detail_name: MorphLabel,
    detail_kind: MorphLabel,
    detail_kind_revealer: gtk::Revealer,
    detail_follows: MorphLabel,
    detail_follows_revealer: gtk::Revealer,
    detail_source: MorphLabel,
    detail_revision: MorphLabel,
    lock_age: MorphLabel,
    lock_upstream: MorphLabel,
    /// Crossfades between `history_empty`'s message and the timeline in `history_list`.
    history_stack: gtk::Stack,
    history_scroll: gtk::ScrolledWindow,
    history_list: gtk::Box,
    history_empty: MorphLabel,
    history_status: MorphLabel,
    history_spinner_revealer: gtk::Revealer,
    revision_toggle: gtk::Button,
    revision_toggle_label: MorphLabel,
    revision_revealer: gtk::Revealer,
    revision_entry: gtk::Entry,
    apply_revision: gtk::Button,
    status: MorphLabel,
    status_revealer: gtk::Revealer,
    spinner_revealer: gtk::Revealer,
    update_selected: gtk::Button,
    update_all: gtk::Button,
    /// Undo and the build buttons, shown once Molt has changed flake.lock.
    transaction_actions: gtk::Revealer,
    undo: gtk::Button,
    build_home: gtk::Button,
    build_nixos: gtk::Button,
    action_energy: EnergyOverlay,
    state: Rc<UiState>,
}

impl InputsView {
    /// Applies freshly loaded inputs in place: rows are matched by name, so selection and scroll
    /// survive, changed rows fade their new content in, and rows glide to their new order.
    fn set_inputs(self: &Rc<Self>, inputs: Vec<FlakeInput>) {
        let previous = self.selected.borrow().clone();
        let now = SystemTime::now();
        let mut rows: HashMap<String, gtk::ListBoxRow> = list_rows(&self.list)
            .map(|row| (row.widget_name().to_string(), row))
            .collect();
        let refreshing = !rows.is_empty();
        let mut membership_changed = false;
        for input in &inputs {
            if let Some(row) = rows.remove(&input.name) {
                let content = input_row_content(input, now);
                row.set_child(Some(&content));
                if !self.inputs.borrow().contains(input) {
                    fade_in(&content);
                }
            } else {
                let row = input_row(input, now);
                self.list.append(&row);
                if refreshing {
                    fade_in(&row);
                }
                membership_changed = true;
            }
        }
        for (name, row) in rows {
            self.list.remove(&row);
            self.row_offsets.borrow_mut().remove(&name);
            membership_changed = true;
        }
        self.inputs.replace(inputs);
        // The glide only knows the heights of rows already laid out, so added rows just fade in.
        if membership_changed {
            stop_resort(self);
            self.list.invalidate_sort();
        } else {
            resort_animated(self);
        }
        let inputs = self.inputs.borrow();
        let stale = inputs
            .iter()
            .filter(|input| lock_age(input, now).is_some_and(|age| age >= STALE_AFTER))
            .count();
        self.count.set_text(&if stale == 0 {
            format!("{} direct inputs", inputs.len())
        } else {
            format!(
                "{} direct inputs · {stale} locked over {} days ago",
                inputs.len(),
                STALE_AFTER / 86_400
            )
        });
        drop(inputs);
        let selection = previous
            .filter(|name| self.inputs.borrow().iter().any(|input| &input.name == name))
            .or_else(|| self.first_visible_input());
        if let Some(name) = selection {
            self.select(&name);
        } else {
            self.clear_detail();
        }
        self.list.invalidate_filter();
    }

    /// The input shown first under the current sort order.
    fn first_visible_input(&self) -> Option<String> {
        let inputs = self.inputs.borrow();
        let oldest_first = self.sort_oldest.is_active();
        inputs
            .iter()
            .min_by(|left, right| compare_inputs(left, right, oldest_first))
            .map(|input| input.name.clone())
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
        if let Some(row) = list_rows(&self.list).find(|row| row.widget_name() == name) {
            self.list.select_row(Some(&row));
            self.show_detail(name);
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
        reveal_text(
            &self.detail_kind_revealer,
            &self.detail_kind,
            Some(input.source_kind.as_str()).filter(|kind| !kind.is_empty()),
        );
        let follows = input
            .follows
            .as_ref()
            .map(|follows| format!("follows {follows}"));
        reveal_text(
            &self.detail_follows_revealer,
            &self.detail_follows,
            follows.as_deref(),
        );
        self.detail_source.set_text(&input.source);
        self.detail_source
            .root
            .set_tooltip_text(Some(&input.source));
        self.detail_revision
            .set_text(&short_revision(&input.revision));
        self.detail_revision
            .root
            .set_tooltip_text(Some(&input.revision));
        let now = SystemTime::now();
        self.lock_age.set_text(&lock_age(input, now).map_or_else(
            || "· date unavailable".to_owned(),
            |age| format!("· {} old", duration_text(age)),
        ));
        self.lock_age
            .root
            .set_tooltip_text(input.last_modified.and_then(format_date).as_deref());
        if changed {
            self.lock_upstream.set_text("");
            self.revision_revealer.set_reveal_child(false);
            self.revision_toggle_label.set_text("Pin…");
            self.revision_entry.set_text("");
        }
        self.revision_toggle
            .set_sensitive(input.flake_ref.is_some());
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
        reveal_text(&self.detail_kind_revealer, &self.detail_kind, None);
        reveal_text(&self.detail_follows_revealer, &self.detail_follows, None);
        self.detail_source.set_text("");
        self.detail_revision.set_text("");
        self.lock_age.set_text("");
        self.lock_upstream.set_text("");
        self.update_selected.set_sensitive(false);
        self.revision_toggle.set_sensitive(false);
        self.revision_revealer.set_reveal_child(false);
        self.revision_entry.set_sensitive(false);
        self.apply_revision.set_sensitive(false);
        self.revisions.borrow_mut().clear();
        self.show_history_message("Select an input");
        self.history_status.set_text("");
        self.history_spinner_revealer.set_reveal_child(false);
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
        self.spinner_revealer.set_reveal_child(busy);
        if busy {
            self.action_energy.start();
            self.show_status(message);
        } else {
            self.action_energy.stop();
            self.status_revealer.set_reveal_child(false);
        }
    }

    fn show_status(&self, text: &str) {
        reveal_text(&self.status_revealer, &self.status, Some(text));
    }

    /// Shows `message` in place of the timeline. The timeline's rows stay until the next render
    /// so the crossfade away from them still shows them.
    fn show_history_message(&self, message: &str) {
        if self.history_stack.visible_child_name().as_deref() == Some(HISTORY_MESSAGE) {
            self.history_empty.set_text(message);
        } else {
            self.history_empty.set_text_immediate(message);
            self.history_stack.set_visible_child_name(HISTORY_MESSAGE);
        }
    }

    fn show_error(&self, error: &str) {
        self.status.add_css_class("error");
        self.show_status(error.lines().next().unwrap_or(error));
        self.status.root.set_tooltip_text(Some(error));
    }
}

/// `history_stack` pages.
const HISTORY_MESSAGE: &str = "message";
const HISTORY_TIMELINE: &str = "timeline";

fn revealer(
    child: &impl IsA<gtk::Widget>,
    transition: gtk::RevealerTransitionType,
) -> gtk::Revealer {
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(transition);
    revealer.set_transition_duration(200);
    revealer.set_child(Some(child));
    revealer
}

/// Reveals `text` in `label`, morphing from the old text while it is still on screen; `None`
/// hides the revealer and lets the old text leave with it.
fn reveal_text(revealer: &gtk::Revealer, label: &MorphLabel, text: Option<&str>) {
    match text {
        Some(text) if revealer.is_child_revealed() => label.set_text(text),
        Some(text) => label.set_text_immediate(text),
        None => {}
    }
    revealer.set_reveal_child(text.is_some());
}

/// A revealer that runs `spinner` only while any of it is on screen.
fn spinner_revealer(
    spinner: &gtk::Spinner,
    transition: gtk::RevealerTransitionType,
) -> gtk::Revealer {
    let revealer = revealer(spinner, transition);
    let starting = spinner.clone();
    revealer.connect_reveal_child_notify(move |revealer| {
        if revealer.reveals_child() {
            starting.start();
        }
    });
    let stopping = spinner.clone();
    revealer.connect_child_revealed_notify(move |revealer| {
        if !revealer.is_child_revealed() {
            stopping.stop();
        }
    });
    revealer
}

/// A chip that slides in beside the input's name. It carries its own spacing so a hidden chip
/// leaves no gap.
fn chip_revealer(chip: &MorphLabel) -> gtk::Revealer {
    chip.root.set_margin_start(8);
    let revealer = revealer(&chip.root, gtk::RevealerTransitionType::SlideRight);
    revealer.set_valign(gtk::Align::Center);
    revealer
}

pub(crate) fn show_inputs(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Config,
) {
    let directory = config.flake_dir.clone();
    if state.operation.get() != Operation::Idle {
        return;
    }
    state.clear_actions();
    replace_screen(root);
    root.remove_css_class("home-root");
    fit_screen(window, &state);

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
    let count = MorphLabel::new("", &["inputs-count"], 0.0);
    heading.append(&count.root);
    header.append(&heading);
    let sort = SegmentedControl::new(&["Name", "Oldest first"]);
    sort.root.add_css_class("inputs-sort");
    sort.root.add_css_class("phase-panel");
    sort.root.set_valign(gtk::Align::End);
    let sort_oldest = sort.options[1].clone();
    header.append(&sort.root);
    root.append(&header);

    // Homogeneous columns keep the list/detail split fixed while the detail text changes.
    let content = gtk::Grid::new();
    content.set_column_homogeneous(true);
    content.set_column_spacing(14);
    content.set_vexpand(true);

    let browser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browser.add_css_class("inputs-browser");
    browser.add_css_class("phase-panel");
    browser.set_hexpand(true);
    let search = gtk::SearchEntry::new();
    search.add_css_class("inputs-search");
    field(&search);
    search.set_placeholder_text(Some("Filter inputs"));
    browser.append(&search);
    let list = gtk::ListBox::new();
    list.add_css_class("inputs-list");
    list.add_css_class("see-through");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.add_css_class("see-through");
    list_scroll.set_vexpand(true);
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_child(Some(&list));
    state.set_scroll_adjustment(list_scroll.vadjustment());
    browser.append(&FadeScroll::new(&list_scroll));
    content.attach(&browser, 0, 0, 2, 1);

    let details = gtk::Box::new(gtk::Orientation::Vertical, 12);
    details.add_css_class("inputs-detail");
    details.add_css_class("phase-panel");
    details.set_hexpand(true);
    details.set_vexpand(true);

    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let identity_text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    identity_text.set_hexpand(true);
    let name_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let detail_name = MorphLabel::new("", &["inputs-detail-name"], 0.0);
    name_row.append(&detail_name.root);
    let detail_kind = MorphLabel::new("", &["chip", "tinted", theme::tone(theme::TEAL)], 0.5);
    let detail_kind_revealer = chip_revealer(&detail_kind);
    name_row.append(&detail_kind_revealer);
    let detail_follows = MorphLabel::new("", &["chip"], 0.5);
    let detail_follows_revealer = chip_revealer(&detail_follows);
    name_row.append(&detail_follows_revealer);
    identity_text.append(&name_row);
    let detail_source = MorphLabel::new("", &["inputs-detail-source"], 0.0);
    identity_text.append(&detail_source.root);
    identity.append(&identity_text);
    let revision_toggle = gtk::Button::new();
    let revision_toggle_label = MorphLabel::new("Pin…", &[], 0.5);
    revision_toggle.set_child(Some(&revision_toggle_label.root));
    revision_toggle.add_css_class("inputs-secondary-button");
    revision_toggle.add_css_class("surface-button");
    revision_toggle.set_tooltip_text(Some("Lock this input to a specific commit, tag, or branch"));
    revision_toggle.set_valign(gtk::Align::Start);
    identity.append(&revision_toggle);
    let update_selected = action_button(
        "Update to latest",
        "software-update-available-symbolic",
        true,
    );
    let action_energy = EnergyOverlay::new(&update_selected, 6.0);
    action_energy.root.set_valign(gtk::Align::Start);
    identity.append(&action_energy.root);
    details.append(&identity);

    let lock_line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    lock_line.add_css_class("inputs-lock-line");
    let locked_caption = label("LOCKED", &["panel-eyebrow"], 0.0);
    locked_caption.set_ellipsize(gtk::pango::EllipsizeMode::None);
    lock_line.append(&locked_caption);
    let detail_revision = MorphLabel::new("", &["inputs-revision"], 0.0);
    detail_revision.set_selectable(true);
    lock_line.append(&detail_revision.root);
    let lock_age = MorphLabel::new("", &["inputs-lock-meta"], 0.0);
    lock_line.append(&lock_age.root);
    let lock_upstream = MorphLabel::new("", &["inputs-lock-meta"], 0.0);
    lock_upstream.root.set_hexpand(true);
    lock_line.append(&lock_upstream.root);
    details.append(&lock_line);

    let direct = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    direct.add_css_class("inputs-direct");
    let revision_entry = gtk::Entry::new();
    revision_entry.add_css_class("inputs-revision-entry");
    field(&revision_entry);
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
    history_panel.set_vexpand(true);
    let history_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let history_title = label("HISTORY", &["panel-eyebrow"], 0.0);
    history_title.set_hexpand(true);
    history_header.append(&history_title);
    let history_spinner = gtk::Spinner::new();
    history_spinner.set_size_request(16, 16);
    let history_spinner_revealer =
        spinner_revealer(&history_spinner, gtk::RevealerTransitionType::Crossfade);
    history_header.append(&history_spinner_revealer);
    let history_status = MorphLabel::new("", &["inputs-history-status"], 1.0);
    history_header.append(&history_status.root);
    history_panel.append(&history_header);
    let history_empty = MorphLabel::new("Fetching remote commits…", &["inputs-history-empty"], 0.0);
    history_empty.root.set_valign(gtk::Align::Start);
    let history_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let history_scroll = gtk::ScrolledWindow::new();
    history_scroll.add_css_class("see-through");
    history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    history_scroll.set_min_content_height(140);
    history_scroll.set_vexpand(true);
    history_scroll.set_child(Some(&history_list));
    let history_stack = gtk::Stack::new();
    history_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    history_stack.set_transition_duration(200);
    history_stack.set_vexpand(true);
    history_stack.add_named(&history_empty.root, Some(HISTORY_MESSAGE));
    history_stack.add_named(&FadeScroll::new(&history_scroll), Some(HISTORY_TIMELINE));
    history_panel.append(&history_stack);
    details.append(&history_panel);
    content.attach(&details, 2, 0, 5, 1);
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
    let status = MorphLabel::new("", &["inputs-status"], 1.0);
    let status_revealer = revealer(&status.root, gtk::RevealerTransitionType::Crossfade);
    status_revealer.set_hexpand(true);
    footer.append(&status_revealer);
    // The sliding controls carry their own trailing spacing, so collapsed ones leave no gap.
    let trailing = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let spinner = gtk::Spinner::new();
    spinner.set_size_request(18, 18);
    spinner.set_margin_end(8);
    let spinner_revealer = spinner_revealer(&spinner, gtk::RevealerTransitionType::SlideLeft);
    trailing.append(&spinner_revealer);
    let transaction = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    transaction.set_margin_end(8);
    let undo = action_button("Undo update", "edit-undo-symbolic", false);
    transaction.append(&undo);
    let build_home = action_button("Build Home Manager", "user-home-symbolic", false);
    build_home.set_visible(config.home_flake.is_some());
    transaction.append(&build_home);
    let build_nixos = action_button("Build NixOS", "drive-harddisk-symbolic", true);
    transaction.append(&build_nixos);
    let transaction_actions = revealer(&transaction, gtk::RevealerTransitionType::SlideLeft);
    trailing.append(&transaction_actions);
    let update_all = action_button("Update all", "view-refresh-symbolic", false);
    trailing.append(&update_all);
    footer.append(&trailing);
    root.append(&footer);

    let view = Rc::new(InputsView {
        directory,
        config,
        root: root.clone(),
        window: window.clone(),
        inputs: RefCell::new(Vec::new()),
        selected: RefCell::new(None),
        revisions: RefCell::new(Vec::new()),
        history_generation: Cell::new(0),
        row_offsets: RefCell::new(HashMap::new()),
        resort_animation: Cell::new(0),
        list,
        search,
        count,
        sort_oldest,
        detail_name,
        detail_kind,
        detail_kind_revealer,
        detail_follows,
        detail_follows_revealer,
        detail_source,
        detail_revision,
        lock_age,
        lock_upstream,
        history_stack,
        history_scroll,
        history_list,
        history_empty,
        history_status,
        history_spinner_revealer,
        revision_entry,
        revision_toggle,
        revision_toggle_label,
        revision_revealer,
        apply_revision,
        status,
        status_revealer,
        spinner_revealer,
        update_selected,
        update_all,
        transaction_actions,
        undo,
        build_home,
        build_nixos,
        action_energy,
        state,
    });
    let view_owner = RefCell::new(Some(Rc::clone(&view)));
    header.connect_unrealize(move |_| {
        view_owner.borrow_mut().take();
    });
    view.set_inputs(inputs);
    connect_view(window, &view);
    set_transaction_actions(&view, view.state.input_undo.borrow().is_some());
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
    view.list.set_sort_func(move |left, right| {
        let Some(view) = weak.upgrade() else {
            return gtk::Ordering::Equal;
        };
        let inputs = view.inputs.borrow();
        let find = |row: &gtk::ListBoxRow| {
            let name = row.widget_name();
            inputs.iter().find(|input| input.name == name)
        };
        match (find(left), find(right)) {
            (Some(left), Some(right)) => {
                compare_inputs(left, right, view.sort_oldest.is_active()).into()
            }
            _ => gtk::Ordering::Equal,
        }
    });
    let weak = Rc::downgrade(view);
    view.sort_oldest.connect_toggled(move |_| {
        if let Some(view) = weak.upgrade() {
            resort_animated(&view);
        }
    });
    let weak = Rc::downgrade(view);
    view.search.connect_search_changed(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let hidden: Vec<gtk::ListBoxRow> = list_rows(&view.list)
            .filter(|row| !row.is_child_visible())
            .collect();
        view.list.invalidate_filter();
        for row in hidden.iter().filter(|row| row.is_child_visible()) {
            fade_in(row);
        }
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
    view.revision_toggle.connect_clicked(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let reveal = !view.revision_revealer.reveals_child();
        view.revision_revealer.set_reveal_child(reveal);
        view.revision_toggle_label
            .set_text(if reveal { "Cancel" } else { "Pin…" });
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

    let weak = Rc::downgrade(view);
    view.undo.connect_clicked(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let update = view.state.input_undo.borrow().clone();
        if let Some(update) = update {
            let window = view.window.clone();
            start_operation(
                &window,
                view,
                "Restoring flake.lock to before Molt's changes…".to_owned(),
                move |_, _, _| inputs::restore(&update).map(|()| InputOperationResult::Restored),
            );
        }
    });
    let weak = Rc::downgrade(view);
    view.build_nixos.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            start_build(
                &view.window,
                &view.root,
                Rc::clone(&view.state),
                view.config.clone(),
                Target::NixOs,
                AfterBuild::Review,
            );
        }
    });
    let weak = Rc::downgrade(view);
    view.build_home.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            start_build(
                &view.window,
                &view.root,
                Rc::clone(&view.state),
                view.config.clone(),
                Target::HomeManager,
                AfterBuild::Review,
            );
        }
    });
}

fn action_button(text: &str, icon_name: &str, primary: bool) -> gtk::Button {
    let button = gtk::Button::new();
    if primary {
        button.add_css_class("inputs-primary-button");
        button.add_css_class("accent-button");
    } else {
        button.add_css_class("inputs-secondary-button");
        button.add_css_class("surface-button");
    }
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    content.set_halign(gtk::Align::Center);
    content.append(&centered_icon(icon_name, 14, 14));
    content.append(&label(text, &["inputs-button-label"], 0.5));
    button.set_child(Some(&content));
    button
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
