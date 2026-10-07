mod inspector;
mod row;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::cleanup::{
    self, CleanupEvent, CleanupGroup, CleanupKind, DiskUsage, ScanEvent, format_size,
};
use crate::state::{Operation, UiState};
use crate::ui::common::{action_close_button, clear, fit_window, label, title};
use crate::ui::home::back_to_home_button;
use inspector::CleanupInspector;
use row::CleanupCard;

struct CleanupView {
    cards: HashMap<CleanupKind, CleanupCard>,
    groups: RefCell<HashMap<CleanupKind, CleanupGroup>>,
    available: gtk::Label,
    storage_detail: gtk::Label,
    storage_bar: gtk::ProgressBar,
    selected: gtk::Label,
    inspector: CleanupInspector,
    focused: Cell<CleanupKind>,
    status: gtk::Label,
    spinner: gtk::Spinner,
    action: gtk::Button,
    action_label: gtk::Label,
    back: gtk::Button,
    scanning: Cell<bool>,
    armed: Cell<bool>,
    complete: Cell<bool>,
    failures: Cell<usize>,
}

impl CleanupView {
    fn update_group(&self, group: CleanupGroup) {
        if let Some(card) = self.cards.get(&group.kind) {
            card.set_group(&group);
        }
        let kind = group.kind;
        self.groups.borrow_mut().insert(kind, group);
        if self.focused.get() == kind {
            self.show_details(kind);
        }
        self.selection_changed();
    }

    fn selection_changed(&self) {
        self.armed.set(false);
        self.action.remove_css_class("cleanup-confirming");
        let groups = self.groups.borrow();
        let mut bytes = 0_u64;
        let mut count = 0_usize;
        for (kind, card) in &self.cards {
            if card.is_selected()
                && let Some(group) = groups.get(kind)
            {
                bytes = bytes.saturating_add(group.reclaimable);
                count += 1;
            }
        }
        self.selected.set_text(&format!(
            "{} selected · {count} categor{}",
            format_size(bytes),
            if count == 1 { "y" } else { "ies" }
        ));
        self.action.set_sensitive(!self.scanning.get() && count > 0);
        self.action_label.set_text(if self.scanning.get() {
            "Scanning"
        } else {
            "Clean selected"
        });
    }

    fn show_details(&self, kind: CleanupKind) {
        self.focused.set(kind);
        if let Some(group) = self.groups.borrow().get(&kind) {
            self.inspector.set_group(group);
        } else {
            self.inspector.set_loading(kind);
        }
    }

    fn selected_groups(&self) -> Vec<CleanupGroup> {
        let groups = self.groups.borrow();
        self.cards
            .iter()
            .filter(|(_, card)| card.is_selected())
            .filter_map(|(kind, _)| groups.get(kind).cloned())
            .collect()
    }

    fn set_disk(&self, disk: Option<DiskUsage>) {
        let Some(disk) = disk else {
            self.available.set_text("Storage unavailable");
            self.storage_detail
                .set_text("Could not read filesystem usage");
            self.storage_bar.set_fraction(0.0);
            return;
        };
        self.available
            .set_text(&format!("{} available", format_size(disk.available)));
        self.storage_detail.set_text(&format!(
            "{} used of {} across cleanup volumes",
            format_size(disk.used),
            format_size(disk.total)
        ));
        let fraction = if disk.total == 0 {
            0.0
        } else {
            disk.used as f64 / disk.total as f64
        };
        self.storage_bar.set_fraction(fraction.clamp(0.0, 1.0));
    }
}

pub(crate) fn show_cleanup(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<UiState>) {
    if state.operation.get() != Operation::Idle {
        return;
    }
    state.clear_actions();
    clear(root);
    root.remove_css_class("home-root");
    fit_window(window, (1420, 820), (1100, 660));

    let header = cleanup_header();
    let status = label(
        "Inspecting every available cleaner",
        &["cleanup-status"],
        1.0,
    );
    status.set_hexpand(true);
    let spinner = gtk::Spinner::new();
    spinner.start();
    let scan_state = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    scan_state.set_halign(gtk::Align::End);
    scan_state.append(&spinner);
    scan_state.append(&status);
    header.append(&scan_state);
    root.append(&header);

    let (storage, available, storage_detail, storage_bar, selected) = storage_card();
    root.append(&storage);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.add_css_class("cleanup-content");
    content.set_vexpand(true);
    let grid = gtk::Grid::new();
    grid.add_css_class("cleanup-grid");
    grid.set_column_spacing(10);
    grid.set_row_spacing(10);
    grid.set_column_homogeneous(true);
    grid.set_hexpand(true);
    grid.set_vexpand(true);
    let mut cards = HashMap::new();
    for (index, kind) in CleanupKind::ALL.into_iter().enumerate() {
        let card = CleanupCard::new(kind);
        grid.attach(&card.root, (index % 2) as i32, (index / 2) as i32, 1, 1);
        cards.insert(kind, card);
    }
    content.append(&grid);
    let (inspector, inspector_adjustment) = CleanupInspector::new();
    state.set_scroll_adjustment(inspector_adjustment);
    content.append(&inspector.root);
    root.append(&content);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    footer.add_css_class("report-footer");
    let back = back_to_home_button(window, root, &state);
    footer.append(&back);
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    let action = gtk::Button::new();
    action.add_css_class("cleanup-button");
    action.set_sensitive(false);
    let action_label = label("Scanning", &["cleanup-button-label"], 0.5);
    action.set_child(Some(&action_label));
    footer.append(&action);
    root.append(&footer);

    let view = Rc::new(CleanupView {
        cards,
        groups: RefCell::new(HashMap::new()),
        available,
        storage_detail,
        storage_bar,
        selected,
        inspector,
        focused: Cell::new(CleanupKind::Nix),
        status,
        spinner,
        action,
        action_label,
        back,
        scanning: Cell::new(true),
        armed: Cell::new(false),
        complete: Cell::new(false),
        failures: Cell::new(0),
    });
    // Widget callbacks keep weak references, so the page owns the controller until it is removed.
    let view_owner = RefCell::new(Some(Rc::clone(&view)));
    header.connect_unrealize(move |_| {
        view_owner.borrow_mut().take();
    });
    for (kind, card) in &view.cards {
        let kind = *kind;
        let weak_view = Rc::downgrade(&view);
        card.root.connect_clicked(move |_| {
            if let Some(view) = weak_view.upgrade() {
                view.show_details(kind);
                if let Some(card) = view.cards.get(&kind) {
                    card.toggle_selected();
                }
                view.selection_changed();
            }
        });
        let weak_view = Rc::downgrade(&view);
        let motion = gtk::EventControllerMotion::new();
        motion.connect_enter(move |_, _, _| {
            if let Some(view) = weak_view.upgrade() {
                view.show_details(kind);
            }
        });
        card.root.add_controller(motion);
    }

    let action_window = window.clone();
    let action_root = root.clone();
    let action_state = Rc::clone(&state);
    let weak_view = Rc::downgrade(&view);
    view.action.connect_clicked(move |_| {
        let Some(view) = weak_view.upgrade() else {
            return;
        };
        if action_state.operation.get() == Operation::Cleaning {
            action_state.cancel();
            crate::ui::home::show_home(
                &action_window,
                &action_root,
                Rc::clone(&action_state),
                crate::config::load_config(),
            );
            return;
        }
        if view.complete.get() {
            show_cleanup(&action_window, &action_root, Rc::clone(&action_state));
            return;
        }
        if !view.armed.replace(true) {
            view.action.add_css_class("cleanup-confirming");
            view.action_label.set_text("Confirm cleanup");
            view.status
                .set_text("Click confirm again to clean the selected categories");
            return;
        }
        start_cleanup(
            &action_window,
            &action_root,
            Rc::clone(&action_state),
            Rc::clone(&view),
        );
    });

    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || cleanup::scan(cancellation, sender));
    let weak_view = Rc::downgrade(&view);
    glib::timeout_add_local(Duration::from_millis(60), move || {
        let Some(view) = weak_view.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let mut complete = false;
        for event in receiver.try_iter() {
            match event {
                ScanEvent::Group(group) => view.update_group(group),
                ScanEvent::Complete(disk) => {
                    view.set_disk(disk);
                    complete = true;
                }
            }
        }
        if complete {
            view.scanning.set(false);
            view.spinner.stop();
            view.spinner.set_visible(false);
            view.status
                .set_text("Review every category before cleaning");
            view.selection_changed();
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn start_cleanup(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    view: Rc<CleanupView>,
) {
    let Some((generation, cancellation)) = state.begin(Operation::Cleaning) else {
        return;
    };
    let groups = view.selected_groups();
    view.failures.set(0);
    view.action.remove_css_class("cleanup-confirming");
    view.action_label.set_text("Cancel");
    view.back.set_sensitive(false);
    view.status.set_text("Cleanup in progress");
    for card in view.cards.values() {
        card.root.set_sensitive(false);
    }

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || cleanup::clean(groups, cancellation, sender));
    let window = window.downgrade();
    let root = root.downgrade();
    glib::timeout_add_local(Duration::from_millis(80), move || {
        if state.generation.get() != generation {
            return glib::ControlFlow::Break;
        }
        for event in receiver.try_iter() {
            match event {
                CleanupEvent::Started(kind) => {
                    if let Some(card) = view.cards.get(&kind) {
                        card.set_started();
                    }
                    view.status.set_text(&format!("Cleaning {}", kind.title()));
                }
                CleanupEvent::Finished(kind, result) => {
                    if result.is_err() {
                        view.failures.set(view.failures.get() + 1);
                    }
                    if let Some(card) = view.cards.get(&kind) {
                        card.set_finished(&result);
                    }
                }
                CleanupEvent::Complete {
                    before,
                    after,
                    elapsed,
                } => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    if state.close_pending.replace(false) {
                        if let Some(window) = window.upgrade() {
                            window.close();
                        }
                        return glib::ControlFlow::Break;
                    }
                    view.set_disk(after);
                    view.complete.set(true);
                    view.back.set_sensitive(true);
                    view.action_label.set_text("Scan again");
                    let reclaimed = before.zip(after).map_or(0, |(before, after)| {
                        after.available.saturating_sub(before.available)
                    });
                    if view.failures.get() == 0 {
                        view.status.set_text(&format!(
                            "Freed {} in {:.1}s · cleanup complete",
                            format_size(reclaimed),
                            elapsed.as_secs_f64()
                        ));
                    } else {
                        view.status.set_text(&format!(
                            "Freed {} in {:.1}s · {} categor{} failed",
                            format_size(reclaimed),
                            elapsed.as_secs_f64(),
                            view.failures.get(),
                            if view.failures.get() == 1 { "y" } else { "ies" }
                        ));
                    }
                    if let Some(root) = root.upgrade() {
                        root.queue_draw();
                    }
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

fn cleanup_header() -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("cleanup-header");
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.set_hexpand(true);
    heading.append(&title("Swix"));
    heading.append(&label("System cleanup", &["cleanup-subtitle"], 0.0));
    header.append(&heading);
    header
}

fn storage_card() -> (
    gtk::Box,
    gtk::Label,
    gtk::Label,
    gtk::ProgressBar,
    gtk::Label,
) {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("cleanup-storage");
    let summary = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let available = label("Inspecting storage…", &["cleanup-available"], 0.0);
    available.set_hexpand(true);
    summary.append(&available);
    let selected = label("0 B selected", &["cleanup-selected"], 1.0);
    summary.append(&selected);
    card.append(&summary);
    let storage_bar = gtk::ProgressBar::new();
    storage_bar.add_css_class("cleanup-storage-bar");
    card.append(&storage_bar);
    let storage_detail = label(
        "Reading filesystem capacity",
        &["cleanup-storage-detail"],
        0.0,
    );
    card.append(&storage_detail);
    (card, available, storage_detail, storage_bar, selected)
}
