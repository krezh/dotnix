mod items;
mod plan;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::cleanup::{
    self, CleanupEvent, CleanupGroup, CleanupKind, DiskUsage, ScanEvent, Volume,
};
use crate::config::CleanupPolicy;
use crate::app::state::{Operation, UiState};
use crate::cleanup::CleanupEstimate;
use crate::ui::theme;
use crate::ui::{
    MorphLabel, Tween, action_close_button, fade_in, fit_screen, format_size, label,
    replace_screen, title,
};
use crate::home::ui::back_to_home_button;
use crate::ui::EnergyOverlay;
use items::CleanupItems;
use plan::CleanupPlan;

/// A category's name as the cleanup screen shows it.
fn category_title(kind: CleanupKind) -> &'static str {
    match kind {
        CleanupKind::Nix => "Nix store",
        _ => kind.title(),
    }
}

struct CleanupView {
    groups: RefCell<HashMap<CleanupKind, CleanupGroup>>,
    available: MorphLabel,
    /// The volume rows, or a placeholder until they are known or when none can be read.
    storage: gtk::Stack,
    storage_placeholder: gtk::Label,
    /// One row per filesystem cleanup frees space on.
    volumes: gtk::Grid,
    volume_rows: RefCell<Vec<VolumeRow>>,
    /// Each selected category and the bytes it would free, in display order.
    reclaim: RefCell<Vec<(CleanupKind, u64)>>,
    /// Bytes the selection would free, counting up or down to each new total.
    selected: Tween,
    selected_detail: MorphLabel,
    plan: CleanupPlan,
    items: CleanupItems,
    /// The category whose items the left column lists.
    open: Cell<CleanupKind>,
    status: MorphLabel,
    spinner: gtk::Spinner,
    spinner_reveal: gtk::Revealer,
    action: gtk::Button,
    action_label: MorphLabel,
    action_energy: EnergyOverlay,
    back: gtk::Button,
    scanning: Cell<bool>,
    armed: Cell<bool>,
    complete: Cell<bool>,
    failures: Cell<usize>,
    policy: RefCell<CleanupPolicy>,
    /// Generation retention the shown Nix group was scanned with.
    nix_generations: Cell<Option<usize>>,
    /// Generation retention of the latest Nix scan started.
    nix_target: Cell<usize>,
    /// Identifies the latest scan; results of superseded scans are dropped.
    scan_serial: Cell<u64>,
    /// A Nix rescan for changed retention is running.
    rescanning: Cell<bool>,
    rescan_timer: RefCell<Option<glib::SourceId>>,
}

impl CleanupView {
    fn update_group(&self, group: CleanupGroup) {
        self.plan.set_group(&group);
        if self.open.get() == group.kind {
            self.items.set_group(&group);
        }
        self.groups.borrow_mut().insert(group.kind, group);
        self.selection_changed();
    }

    fn selection_changed(&self) {
        self.armed.set(false);
        self.action.remove_css_class("cleanup-confirming");
        let groups = self.groups.borrow();
        let mut plan = Vec::new();
        for kind in CleanupKind::ALL {
            if self.plan.is_selected(kind)
                && let Some(group) = groups.get(&kind)
            {
                plan.push((kind, group.reclaimable));
            }
        }
        drop(groups);
        let bytes = plan
            .iter()
            .fold(0_u64, |total, (_, bytes)| total.saturating_add(*bytes));
        let count = plan.len();
        let size = format_size(bytes);
        let categories = if count == 1 { "category" } else { "categories" };
        self.selected.set(bytes as f64);
        self.selected_detail
            .set_text(&format!("to reclaim · {count} {categories}"));
        self.plan.set_plan(&plan);
        self.reclaim.replace(plan);
        self.refresh_storage();
        let policy = self.policy();
        let current = self.plan_is_current();
        let busy = self.scanning.get() || self.rescanning.get();
        self.action
            .set_sensitive(!busy && count > 0 && policy.is_ok() && current);
        self.action_label.set_text(&if busy || (policy.is_ok() && !current) {
            "Scanning".to_owned()
        } else if count == 0 {
            "Nothing selected".to_owned()
        } else {
            format!("Clean {size}")
        });
        if !self.scanning.get() {
            self.status.set_text(match &policy {
                Err(error) => error,
                Ok(_) if !current || self.rescanning.get() => {
                    "Updating the Nix plan for the new retention"
                }
                Ok(_) => "Review every category before cleaning",
            });
        }
    }

    /// Whether the shown Nix plan was scanned with the retention entered, or Nix is left out.
    fn plan_is_current(&self) -> bool {
        !self.plan.is_selected(CleanupKind::Nix)
            || self.plan.keep_generations() == self.nix_generations.get()
    }

    fn open_category(&self, kind: CleanupKind) {
        self.open.set(kind);
        self.plan.set_open(kind);
        if let Some(group) = self.groups.borrow().get(&kind) {
            self.items.set_group(group);
        } else {
            self.items.set_loading(kind);
        }
    }

    fn selected_groups(&self) -> Vec<CleanupGroup> {
        let groups = self.groups.borrow();
        CleanupKind::ALL
            .into_iter()
            .filter(|kind| self.plan.is_selected(*kind))
            .filter_map(|kind| groups.get(&kind).cloned())
            .collect()
    }

    /// Shows `volumes`, updating rows already shown for the same mount in place so their bars
    /// ease to the new usage.
    fn set_volumes(&self, volumes: Vec<Volume>) {
        let mut rows: HashMap<PathBuf, VolumeRow> = self
            .volume_rows
            .take()
            .into_iter()
            .map(|row| (row.mount.clone(), row))
            .collect();
        rows.retain(|mount, row| {
            let kept = volumes.iter().any(|volume| volume.mount == *mount);
            if !kept {
                row.detach(&self.volumes);
            }
            kept
        });
        if volumes.is_empty() {
            self.available.set_text("Storage unavailable");
            self.storage_placeholder
                .set_text("Could not read filesystem usage");
            self.storage.set_visible_child_name("placeholder");
            return;
        }
        self.available.set_text(&format!(
            "{} available",
            format_size(available(&volumes))
        ));
        let shown = self.storage.visible_child_name().as_deref() == Some("volumes");
        let rows = volumes
            .into_iter()
            .enumerate()
            .map(|(index, volume)| {
                let row = match rows.remove(&volume.mount) {
                    Some(row) => {
                        row.update(volume);
                        row
                    }
                    None => VolumeRow::new(volume),
                };
                if row.attach(&self.volumes, index as i32) && shown {
                    for widget in row.widgets() {
                        fade_in(&widget);
                    }
                }
                row
            })
            .collect();
        self.volume_rows.replace(rows);
        self.storage.set_visible_child_name("volumes");
        self.refresh_storage();
    }

    fn refresh_storage(&self) {
        let plan = self.reclaim.borrow();
        let groups = self.groups.borrow();
        for row in self.volume_rows.borrow().iter() {
            // Categories spread over several filesystems count only their share on this one.
            let here = plan
                .iter()
                .map(|(kind, bytes)| {
                    let share = groups
                        .get(kind)
                        .filter(|group| !group.volumes.is_empty())
                        .map(|group| {
                            group
                                .volumes
                                .iter()
                                .find(|(device, _)| *device == row.device)
                                .map_or(0, |(_, bytes)| *bytes)
                        });
                    (*kind, share.unwrap_or(*bytes))
                })
                .collect::<Vec<_>>();
            row.show_plan(&here);
        }
    }

    fn policy(&self) -> Result<CleanupPolicy, String> {
        let policy = CleanupPolicy {
            keep_generations: if self.plan.is_selected(CleanupKind::Nix) {
                self.plan.keep_generations().ok_or_else(|| {
                    "Nix generations to keep must be a whole number from 0 to 100".to_owned()
                })?
            } else {
                0
            },
            journal_max_age: if self.plan.is_selected(CleanupKind::Journals) {
                self.plan.journal_max_age()
            } else {
                "0".to_owned()
            },
        };
        policy.validate()?;
        Ok(policy)
    }

}

pub(crate) fn show_cleanup(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    policy: CleanupPolicy,
) {
    if state.operation.get() != Operation::Idle {
        return;
    }
    state.clear_actions();
    replace_screen(root);
    root.remove_css_class("home-root");
    fit_screen(window, &state);

    let header = cleanup_header();
    let status = MorphLabel::new("Inspecting every available cleaner", &["cleanup-status"], 1.0);
    status.root.set_hexpand(true);
    let spinner = gtk::Spinner::new();
    spinner.start();
    let spinner_reveal = gtk::Revealer::new();
    spinner_reveal.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    spinner_reveal.set_transition_duration(200);
    spinner_reveal.set_reveal_child(true);
    spinner_reveal.set_child(Some(&spinner));
    // A stopped spinner draws nothing, so it keeps spinning until it has faded out.
    spinner_reveal.connect_child_revealed_notify(|revealer| {
        if !revealer.is_child_revealed()
            && let Some(spinner) = revealer.child().and_downcast::<gtk::Spinner>()
        {
            spinner.stop();
        }
    });
    let scan_state = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    scan_state.set_halign(gtk::Align::End);
    scan_state.append(&spinner_reveal);
    scan_state.append(&status.root);
    header.append(&scan_state);
    root.append(&header);

    let storage = storage_card();
    root.append(&storage.root);

    // Homogeneous columns pin the split at 3:2; a box would size the panel to its content and
    // slide it sideways whenever a category's size text changes width.
    let content = gtk::Grid::new();
    content.set_column_homogeneous(true);
    content.set_column_spacing(18);
    content.set_vexpand(true);
    let items = CleanupItems::new(&state);
    items.root.set_size_request(480, -1);
    items.set_loading(CleanupKind::Nix);
    content.attach(&items.root, 0, 0, 3, 1);
    let plan = CleanupPlan::new(&policy);
    plan.root.set_hexpand(true);
    plan.set_open(CleanupKind::Nix);
    content.attach(&plan.root, 3, 0, 2, 1);
    root.append(&content);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    footer.add_css_class("report-footer");
    footer.add_css_class("cleanup-footer");
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
    action.add_css_class("accent-button");
    action.set_sensitive(false);
    let action_label = MorphLabel::new("Scanning", &["cleanup-button-label"], 0.5);
    action.set_child(Some(&action_label.root));
    let action_energy = EnergyOverlay::new(&action, 6.0);
    action_energy.start();
    footer.append(&action_energy.root);
    root.append(&footer);

    let view = Rc::new(CleanupView {
        groups: RefCell::new(HashMap::new()),
        available: storage.available,
        storage: storage.stack,
        storage_placeholder: storage.placeholder,
        volumes: storage.volumes,
        volume_rows: RefCell::new(Vec::new()),
        reclaim: RefCell::new(Vec::new()),
        selected: storage.selected,
        selected_detail: storage.selection_detail,
        plan,
        items,
        open: Cell::new(CleanupKind::Nix),
        status,
        spinner,
        spinner_reveal,
        action,
        action_label,
        action_energy,
        back,
        scanning: Cell::new(true),
        armed: Cell::new(false),
        complete: Cell::new(false),
        failures: Cell::new(0),
        nix_generations: Cell::new(None),
        nix_target: Cell::new(policy.keep_generations),
        scan_serial: Cell::new(0),
        rescanning: Cell::new(false),
        rescan_timer: RefCell::new(None),
        policy: RefCell::new(policy),
    });
    // Widget callbacks keep weak references, so the page owns the controller until it is removed.
    let view_owner = RefCell::new(Some(Rc::clone(&view)));
    header.connect_unrealize(move |_| {
        view_owner.borrow_mut().take();
    });
    let weak_view = Rc::downgrade(&view);
    view.plan.connect_open(move |kind| {
        if let Some(view) = weak_view.upgrade() {
            view.open_category(kind);
        }
    });
    let weak_view = Rc::downgrade(&view);
    view.plan.connect_selection_changed(move || {
        if let Some(view) = weak_view.upgrade() {
            view.selection_changed();
        }
    });
    let weak_view = Rc::downgrade(&view);
    let policy_state = Rc::clone(&state);
    view.plan.connect_policy_changed(move || {
        if let Some(view) = weak_view.upgrade() {
            view.selection_changed();
            schedule_nix_rescan(&view, &policy_state);
        }
    });

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
            crate::home::ui::show_home(
                &action_window,
                &action_root,
                Rc::clone(&action_state),
                crate::config::load_config(),
            );
            return;
        }
        if view.complete.get() {
            show_cleanup(
                &action_window,
                &action_root,
                Rc::clone(&action_state),
                view.policy.borrow().clone(),
            );
            return;
        }
        if !view.armed.replace(true) {
            view.action.add_css_class("cleanup-confirming");
            view.action_label.set_text("Confirm cleanup");
            view.status.set_text(&cleanup_confirmation(&view));
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
    let scan_policy = view.policy.borrow().clone();
    let keep_generations = scan_policy.keep_generations;
    thread::spawn(move || cleanup::scan(&CleanupKind::ALL, scan_policy, cancellation, sender));
    watch_scan(&view, state, receiver, keep_generations);
}

/// Applies scan results until the scan completes or a newer scan supersedes it.
fn watch_scan(
    view: &Rc<CleanupView>,
    state: Rc<UiState>,
    receiver: mpsc::Receiver<ScanEvent>,
    keep_generations: usize,
) {
    let serial = view.scan_serial.get();
    let weak_view = Rc::downgrade(view);
    glib::timeout_add_local(Duration::from_millis(60), move || {
        let Some(view) = weak_view.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if view.scan_serial.get() != serial {
            return glib::ControlFlow::Break;
        }
        let mut complete = None;
        for event in receiver.try_iter() {
            match event {
                ScanEvent::Group(group) => {
                    let kind = group.kind;
                    // Rescans keep the user's choice; the first scan selects what it finds.
                    let selected = (!view.scanning.get()).then(|| view.plan.is_selected(kind));
                    if kind == CleanupKind::Nix {
                        view.nix_generations.set(Some(keep_generations));
                    }
                    view.update_group(group);
                    if let Some(selected) = selected {
                        view.plan.set_selected(kind, selected);
                    }
                }
                ScanEvent::Complete(volumes) => complete = Some(volumes),
            }
        }
        let Some(volumes) = complete else {
            return glib::ControlFlow::Continue;
        };
        if view.scanning.replace(false) {
            view.set_volumes(volumes);
            let reclaimable = view
                .groups
                .borrow()
                .values()
                .filter(|group| group.available)
                .fold(0_u64, |total, group| total.saturating_add(group.reclaimable));
            CleanupEstimate::save(reclaimable);
        }
        view.rescanning.set(false);
        view.action_energy.stop();
        view.spinner_reveal.set_reveal_child(false);
        view.selection_changed();
        rescan_nix_if_stale(&view, &state);
        glib::ControlFlow::Break
    });
}

/// Rescans the Nix category once the retention entered has stopped changing.
fn schedule_nix_rescan(view: &Rc<CleanupView>, state: &Rc<UiState>) {
    if let Some(timer) = view.rescan_timer.take() {
        timer.remove();
    }
    let weak_view = Rc::downgrade(view);
    let state = Rc::clone(state);
    let timer = glib::timeout_add_local_once(Duration::from_millis(400), move || {
        if let Some(view) = weak_view.upgrade() {
            view.rescan_timer.take();
            rescan_nix_if_stale(&view, &state);
        }
    });
    view.rescan_timer.replace(Some(timer));
}

/// Rescans the Nix category when the valid retention entered differs from the latest Nix scan's,
/// so the reviewed plan always matches the policy cleanup runs with. Waits for the first scan,
/// which calls this again when it completes.
fn rescan_nix_if_stale(view: &Rc<CleanupView>, state: &Rc<UiState>) {
    if view.scanning.get() || view.complete.get() || state.operation.get() != Operation::Idle {
        return;
    }
    let Some(keep_generations) = view.plan.keep_generations() else {
        return;
    };
    let policy = CleanupPolicy {
        keep_generations,
        ..view.policy.borrow().clone()
    };
    if policy.validate().is_err() || view.nix_target.get() == keep_generations {
        return;
    }
    view.nix_target.set(keep_generations);
    view.scan_serial.set(view.scan_serial.get().wrapping_add(1));
    view.rescanning.set(true);
    view.spinner_reveal.set_reveal_child(true);
    view.spinner.start();
    view.action_energy.start();
    view.selection_changed();

    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || cleanup::scan(&[CleanupKind::Nix], policy, cancellation, sender));
    watch_scan(view, Rc::clone(state), receiver, keep_generations);
}

/// `Cleaning Nix store` / `Cleaning Nix store and Trash` / `Cleaning Nix store and 3 more`.
fn running_status(running: &[CleanupKind]) -> String {
    match running {
        [] => "Cleanup in progress".to_owned(),
        [only] => format!("Cleaning {}", category_title(*only)),
        [first, second] => format!(
            "Cleaning {} and {}",
            category_title(*first),
            category_title(*second)
        ),
        [first, rest @ ..] => format!("Cleaning {} and {} more", category_title(*first), rest.len()),
    }
}

fn cleanup_confirmation(view: &CleanupView) -> String {
    let selected = view.selected_groups();
    let Ok(policy) = view.policy() else {
        return "Enter a valid cleanup retention policy".to_owned();
    };
    let mut policies = Vec::new();
    if selected.iter().any(|group| group.kind == CleanupKind::Nix) {
        policies.push(if policy.keep_generations == 0 {
            "keep no old Nix profile generations".to_owned()
        } else {
            format!(
                "keep the latest {} generation(s) per Nix profile",
                policy.keep_generations
            )
        });
    }
    if selected
        .iter()
        .any(|group| group.kind == CleanupKind::Journals)
    {
        policies.push(if policy.journal_max_age == "0" {
            "keep no archived journal history".to_owned()
        } else {
            format!(
                "keep journal entries from the last {}",
                policy.journal_max_age
            )
        });
    }
    if selected.iter().any(|group| group.kind == CleanupKind::Docker) {
        policies.push("prune all unused Docker data".to_owned());
    }
    if selected
        .iter()
        .any(|group| group.kind == CleanupKind::GamePrefixes)
    {
        policies.push(
            "trim Proton prefixes of uninstalled games, keeping their users and ProgramData folders"
                .to_owned(),
        );
    }
    if policies.is_empty() {
        "Confirm deletion of every item shown in the selected categories".to_owned()
    } else {
        format!("Confirm cleanup: {}", policies.join("; "))
    }
}

fn start_cleanup(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    view: Rc<CleanupView>,
) {
    let policy = match view.policy() {
        Ok(policy) => policy,
        Err(error) => {
            view.status.set_text(&error);
            return;
        }
    };
    if view.rescanning.get() || !view.plan_is_current() {
        rescan_nix_if_stale(&view, &state);
        view.selection_changed();
        view.status
            .set_text("Retention changed since the scan; review the updated Nix plan first");
        return;
    }
    let Some((generation, cancellation)) = state.begin(Operation::Cleaning) else {
        return;
    };
    let groups = view.selected_groups();
    view.failures.set(0);
    view.action.remove_css_class("cleanup-confirming");
    view.action_label.set_text("Cancel");
    view.action_energy.start();
    view.back.set_sensitive(false);
    view.status.set_text("Cleanup in progress");
    view.plan.set_sensitive(false);

    let (sender, receiver) = mpsc::channel();
    view.policy.replace(policy.clone());
    thread::spawn(move || cleanup::clean(groups, policy, cancellation, sender));
    let window = window.downgrade();
    let root = root.downgrade();
    // Categories clean at the same time; the status names every one still running.
    let running = RefCell::new(Vec::new());
    glib::timeout_add_local(Duration::from_millis(80), move || {
        if state.generation.get() != generation {
            return glib::ControlFlow::Break;
        }
        for event in receiver.try_iter() {
            match event {
                CleanupEvent::Started(kind) => {
                    view.plan.set_started(kind);
                    running.borrow_mut().push(kind);
                    view.status.set_text(&running_status(&running.borrow()));
                }
                CleanupEvent::Finished(kind, result) => {
                    if result.is_err() {
                        view.failures.set(view.failures.get() + 1);
                    }
                    view.plan.set_finished(kind, &result);
                    running.borrow_mut().retain(|other| *other != kind);
                    if !running.borrow().is_empty() {
                        view.status.set_text(&running_status(&running.borrow()));
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
                    view.reclaim.borrow_mut().clear();
                    CleanupEstimate::clear();
                    let reclaimed = available(&after).saturating_sub(available(&before));
                    view.set_volumes(after);
                    view.complete.set(true);
                    view.back.set_sensitive(true);
                    view.action_energy.stop();
                    view.action_label.set_text("Scan again");
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
    heading.append(&title("Molt"));
    heading.append(&label("System cleanup", &["cleanup-subtitle"], 0.0));
    header.append(&heading);
    header
}

/// Free space across `volumes`.
fn available(volumes: &[Volume]) -> u64 {
    volumes
        .iter()
        .fold(0_u64, |total, volume| total.saturating_add(volume.usage.available))
}

struct StorageCard {
    root: gtk::Box,
    available: MorphLabel,
    stack: gtk::Stack,
    placeholder: gtk::Label,
    volumes: gtk::Grid,
    selected: Tween,
    selection_detail: MorphLabel,
}

/// One filesystem in the storage card: its mount, usage bar, and usage before and after cleanup.
struct VolumeRow {
    mount: PathBuf,
    device: u64,
    name: gtk::Label,
    meter: Rc<StorageMeter>,
    used: MorphLabel,
    after: MorphLabel,
    usage: Cell<DiskUsage>,
    kinds: RefCell<Vec<CleanupKind>>,
    /// The grid row it sits in, once attached.
    position: Cell<Option<i32>>,
}

impl VolumeRow {
    fn new(volume: Volume) -> Self {
        let mount = volume.mount.display().to_string();
        let name = label(&mount, &["cleanup-volume-mount"], 0.0);
        name.set_tooltip_text(Some(&mount));
        let meter = StorageMeter::new();
        meter.area.set_valign(gtk::Align::Center);
        let used = MorphLabel::new(&used_text(volume.usage), &["cleanup-storage-detail"], 1.0);
        let after = MorphLabel::new("", &["cleanup-storage-after"], 1.0);
        Self {
            mount: volume.mount,
            device: volume.device,
            name,
            meter,
            used,
            after,
            usage: Cell::new(volume.usage),
            kinds: RefCell::new(volume.kinds),
            position: Cell::new(None),
        }
    }

    /// The row's widgets, in column order.
    fn widgets(&self) -> [gtk::Widget; 4] {
        [
            self.name.clone().upcast(),
            self.meter.area.clone().upcast(),
            self.used.root.clone().upcast(),
            self.after.root.clone().upcast(),
        ]
    }

    /// Places the row in grid row `row`; true when it was not in the grid before.
    fn attach(&self, grid: &gtk::Grid, row: i32) -> bool {
        let previous = self.position.replace(Some(row));
        if previous == Some(row) {
            return false;
        }
        for (column, widget) in self.widgets().iter().enumerate() {
            if previous.is_some() {
                grid.remove(widget);
            }
            grid.attach(widget, column as i32, row, 1, 1);
        }
        previous.is_none()
    }

    fn detach(&self, grid: &gtk::Grid) {
        for widget in self.widgets() {
            grid.remove(&widget);
        }
    }

    /// Takes `volume`'s new usage; the next `show_plan` eases the bar to it.
    fn update(&self, volume: Volume) {
        self.usage.set(volume.usage);
        self.kinds.replace(volume.kinds);
        self.used.set_text(&used_text(volume.usage));
    }

    /// Marks the share of this volume each selected category in `plan` would free.
    fn show_plan(&self, plan: &[(CleanupKind, u64)]) {
        let kinds = self.kinds.borrow();
        let parts: Vec<_> = plan
            .iter()
            .filter(|(kind, bytes)| *bytes > 0 && kinds.contains(kind))
            .collect();
        let selected = parts
            .iter()
            .fold(0_u64, |total, (_, bytes)| total.saturating_add(*bytes));
        let usage = self.usage.get();
        let reclaim = selected.min(usage.used);
        if reclaim > 0 {
            let after = format_size(usage.used - reclaim);
            self.after.set_text(&format!("→ {after}"));
            self.after
                .root
                .set_tooltip_text(Some(&format!("{after} used after cleanup")));
        } else {
            self.after.set_text("");
            self.after.root.set_tooltip_text(None);
        }
        let breakdown = parts
            .iter()
            .map(|(kind, bytes)| format!("{} · {}", category_title(*kind), format_size(*bytes)))
            .collect::<Vec<_>>()
            .join("\n");
        self.meter
            .area
            .set_tooltip_text((!breakdown.is_empty()).then_some(breakdown.as_str()));
        if usage.total == 0 {
            self.meter.set(MeterLevels::default());
            return;
        }
        // Estimates can exceed the used space; every category then shrinks by the same factor.
        let scale = if selected == 0 {
            0.0
        } else {
            reclaim as f64 / selected as f64
        };
        let mut levels = MeterLevels {
            used: usage.used as f64 / usage.total as f64,
            ..MeterLevels::default()
        };
        for (index, kind) in CleanupKind::ALL.iter().enumerate() {
            if let Some((_, bytes)) = parts.iter().find(|(part, _)| part == kind) {
                levels.reclaim[index] = *bytes as f64 * scale / usage.total as f64;
                levels.shown[index] = 1.0;
            }
        }
        self.meter.set(levels);
    }
}

fn used_text(usage: DiskUsage) -> String {
    format!(
        "{} of {} used",
        format_size(usage.used),
        format_size(usage.total)
    )
}

/// What a `StorageMeter` draws, each part a fraction of total capacity.
#[derive(Clone, Copy, Default, PartialEq)]
struct MeterLevels {
    used: f64,
    /// Space each category would free, in `CleanupKind::ALL` order.
    reclaim: [f64; CleanupKind::ALL.len()],
    /// How much of each category's segment is drawn: 1 while selected, 0 otherwise. Easing it
    /// lets even a minimum-width sliver grow in and shrink away.
    shown: [f64; CleanupKind::ALL.len()],
}

impl MeterLevels {
    /// The levels `progress` of the way from `self` to `target`.
    fn toward(&self, target: &Self, progress: f64) -> Self {
        let mix = |from: f64, to: f64| from + (to - from) * progress;
        Self {
            used: mix(self.used, target.used),
            reclaim: std::array::from_fn(|index| mix(self.reclaim[index], target.reclaim[index])),
            shown: std::array::from_fn(|index| mix(self.shown[index], target.shown[index])),
        }
    }
}

/// Disk usage bar: used space, ending in one segment per selected category for the space it
/// would free, each in that category's colour. Every change eases from what is drawn.
struct StorageMeter {
    area: gtk::DrawingArea,
    /// Where the latest change started from, and where it ends.
    from: Cell<MeterLevels>,
    to: Cell<MeterLevels>,
    /// `0..=1` progress from `from` to `to`.
    progress: Tween,
}

impl StorageMeter {
    /// Narrowest a category's segment draws, so a sliver of the disk stays visible.
    const MIN_SEGMENT: f64 = 3.0;

    fn new() -> Rc<Self> {
        let area = gtk::DrawingArea::new();
        area.add_css_class("cleanup-storage-meter");
        area.set_content_height(8);
        area.set_hexpand(true);
        let progress = Tween::new(&area, 1.0);
        let meter = Rc::new(Self {
            area,
            from: Cell::default(),
            to: Cell::default(),
            progress,
        });
        let weak = Rc::downgrade(&meter);
        meter.area.set_draw_func(move |_, context, width, height| {
            let Some(meter) = weak.upgrade() else {
                return;
            };
            let levels = meter.levels();
            let (width, height) = (f64::from(width), f64::from(height));
            let radius = height / 2.0;
            theme::set_source_rgb(context, theme::SURFACE0);
            theme::rounded_rectangle(context, 0.0, 0.0, width, height, radius);
            let _ = context.fill();

            let used_width = width * levels.used.clamp(0.0, 1.0);
            if used_width <= 0.0 {
                return;
            }
            context.save().ok();
            theme::rounded_rectangle(context, 0.0, 0.0, used_width, height, radius);
            context.clip();
            theme::set_source_rgb(context, theme::OVERLAY0);
            context.rectangle(0.0, 0.0, used_width, height);
            let _ = context.fill();
            // Laid out from the end of the used space backwards, so the first category sits leftmost.
            let mut end = used_width;
            for (index, kind) in CleanupKind::ALL.iter().enumerate().rev() {
                let shown = levels.shown[index];
                if shown <= 0.0 {
                    continue;
                }
                if end <= 0.0 {
                    break;
                }
                let segment = (width * levels.reclaim[index])
                    .max(Self::MIN_SEGMENT * shown)
                    .min(end);
                end -= segment;
                theme::set_source_rgb(context, kind.color());
                context.rectangle(end, 0.0, segment, height);
                let _ = context.fill();
            }
            context.restore().ok();
        });
        meter
    }

    /// The levels drawn this frame.
    fn levels(&self) -> MeterLevels {
        self.from.get().toward(&self.to.get(), self.progress.value())
    }

    fn set(&self, levels: MeterLevels) {
        if self.to.get() == levels {
            return;
        }
        self.from.set(self.levels());
        self.to.set(levels);
        self.progress.jump(0.0);
        self.progress.set(1.0);
    }
}

fn storage_card() -> StorageCard {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("cleanup-storage");
    root.add_css_class("phase-panel");
    let overview = gtk::Box::new(gtk::Orientation::Vertical, 10);
    overview.set_hexpand(true);
    let available = MorphLabel::new("Inspecting storage…", &["cleanup-available"], 0.0);
    overview.append(&available.root);
    let volumes = gtk::Grid::new();
    volumes.set_column_spacing(12);
    volumes.set_row_spacing(6);
    let placeholder = label("Reading filesystem capacity", &["cleanup-storage-detail"], 0.0);
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(200);
    stack.set_interpolate_size(true);
    stack.set_vhomogeneous(false);
    stack.add_named(&placeholder, Some("placeholder"));
    stack.add_named(&volumes, Some("volumes"));
    overview.append(&stack);
    root.append(&overview);
    let selection = gtk::Box::new(gtk::Orientation::Vertical, 2);
    selection.add_css_class("cleanup-storage-selection");
    selection.set_valign(gtk::Align::Center);
    let selected_label = label("0 B", &["cleanup-selected"], 0.0);
    selection.append(&selected_label);
    let selected = Tween::new(&selected_label, 0.0);
    selected.on_frame(move |bytes| selected_label.set_text(&format_size(bytes.round() as u64)));
    let selection_detail =
        MorphLabel::new("to reclaim · 0 categories", &["cleanup-storage-detail"], 0.0);
    selection.append(&selection_detail.root);
    root.append(&selection);
    StorageCard {
        root,
        available,
        stack,
        placeholder,
        volumes,
        selected,
        selection_detail,
    }
}
