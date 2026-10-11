use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;

use crate::cleanup::{CleanupGroup, CleanupKind};
use crate::config::CleanupPolicy;
use crate::ui::{EnergyOverlay, MorphLabel, field, format_size, label};

use super::category_title;

/// One category: a button that opens its item list, its size or cleanup status, and whether it
/// is included in the cleanup.
struct PlanRow {
    root: gtk::Box,
    open: gtk::Button,
    size: MorphLabel,
    /// The checkbox, swapped for a result badge once the category has been cleaned.
    slot: gtk::Stack,
    selection: gtk::CheckButton,
    /// Whether the scan found something this category can remove.
    selectable: Cell<bool>,
    /// Animates the row while its category is being cleaned.
    energy: EnergyOverlay,
}

/// The side panel: every category with its size, status, and selection, then the retention
/// policy.
pub(super) struct CleanupPlan {
    pub(super) root: gtk::Box,
    title: MorphLabel,
    rows: HashMap<CleanupKind, PlanRow>,
    retention: gtk::Revealer,
    generations_row: gtk::Revealer,
    generations: gtk::SpinButton,
    journals_row: gtk::Revealer,
    journals: gtk::Entry,
}

impl CleanupPlan {
    pub(super) fn new(policy: &CleanupPolicy) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("cleanup-plan-panel");
        root.add_css_class("phase-panel");
        root.set_width_request(400);
        root.set_vexpand(true);
        root.append(&label("REMOVAL PLAN", &["panel-eyebrow"], 0.0));
        let title = MorphLabel::new("Nothing selected", &["cleanup-plan-title"], 0.0);
        root.append(&title.root);

        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        list.add_css_class("cleanup-plan");
        let mut rows = HashMap::new();
        for kind in CleanupKind::ALL {
            let row = plan_row(kind);
            list.append(&row.energy.root);
            rows.insert(kind, row);
        }
        root.append(&list);

        // Rows space themselves with a top margin, so a collapsed row leaves no gap behind.
        let rows_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let generations = gtk::SpinButton::with_range(0.0, 100.0, 1.0);
        generations.add_css_class("cleanup-retention-input");
        field(&generations);
        generations.set_digits(0);
        generations.set_numeric(false);
        generations.set_width_chars(3);
        generations.set_max_width_chars(3);
        generations.set_valign(gtk::Align::Center);
        generations.set_value(policy.keep_generations as f64);
        generations.set_text(&policy.keep_generations.to_string());
        generations.set_tooltip_text(Some("Old generations to keep per Nix profile: 0–100"));
        let generations_row = retention_row(
            "Keep Nix generations",
            "0 keeps only the current one · fewer rollbacks",
            "Removing old generations reduces rollback history. Keeping 0 removes non-current generations; the current generation remains.",
            &generations,
        );
        rows_box.append(&generations_row);

        let journals = gtk::Entry::new();
        journals.add_css_class("cleanup-retention-input");
        field(&journals);
        journals.set_width_chars(7);
        journals.set_max_width_chars(8);
        journals.set_valign(gtk::Align::Center);
        journals.set_text(&policy.journal_max_age);
        journals.set_placeholder_text(Some("0"));
        journals.set_tooltip_text(Some("Journal history to keep: 0, 7d, 12h, or 30min"));
        let journals_row = retention_row(
            "Keep journal history",
            "0 removes all archived history · or 7d, 12h, 30min",
            "Keeping 0 removes archived journal history. Active journals are rotated before archived entries are removed.",
            &journals,
        );
        rows_box.append(&journals_row);
        let retention = slide_down(&rows_box);
        root.append(&retention);

        Self {
            root,
            title,
            rows,
            retention,
            generations_row,
            generations,
            journals_row,
            journals,
        }
    }

    /// The generation retention entered, or `None` when it is not a whole number.
    pub(super) fn keep_generations(&self) -> Option<usize> {
        self.generations.text().trim().parse().ok()
    }

    pub(super) fn journal_max_age(&self) -> String {
        self.journals.text().trim().to_owned()
    }

    pub(super) fn connect_policy_changed<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        let callback = Rc::new(callback);
        let generations_callback = Rc::clone(&callback);
        self.generations
            .connect_changed(move |_| generations_callback());
        self.journals.connect_changed(move |_| callback());
    }

    /// Calls `callback` with the category whose row was clicked.
    pub(super) fn connect_open<F>(&self, callback: F)
    where
        F: Fn(CleanupKind) + 'static,
    {
        let callback = Rc::new(callback);
        for (kind, row) in &self.rows {
            let kind = *kind;
            let callback = Rc::clone(&callback);
            row.open.connect_clicked(move |_| callback(kind));
        }
    }

    pub(super) fn is_selected(&self, kind: CleanupKind) -> bool {
        self.rows
            .get(&kind)
            .is_some_and(|row| row.selection.is_active())
    }

    /// Includes `kind` in the cleanup if it has anything to remove and `selected` is set.
    pub(super) fn set_selected(&self, kind: CleanupKind, selected: bool) {
        if let Some(row) = self.rows.get(&kind) {
            row.selection.set_active(selected && row.selectable.get());
        }
    }

    /// Calls `callback` whenever a category is included in or left out of the cleanup.
    pub(super) fn connect_selection_changed<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        let callback = Rc::new(callback);
        for row in self.rows.values() {
            let callback = Rc::clone(&callback);
            row.selection.connect_toggled(move |_| callback());
        }
    }

    /// Locks or unlocks what to clean; item lists stay browsable either way.
    pub(super) fn set_sensitive(&self, sensitive: bool) {
        for row in self.rows.values() {
            row.selection.set_sensitive(sensitive && row.selectable.get());
        }
        self.generations.set_sensitive(sensitive);
        self.journals.set_sensitive(sensitive);
    }

    /// Marks the row of the category whose items are open.
    pub(super) fn set_open(&self, open: CleanupKind) {
        for (kind, row) in &self.rows {
            if open == *kind {
                row.root.add_css_class("open");
            } else {
                row.root.remove_css_class("open");
            }
        }
    }

    /// Shows a scanned category's size and selects it if it has anything to remove.
    pub(super) fn set_group(&self, group: &CleanupGroup) {
        let Some(row) = self.rows.get(&group.kind) else {
            return;
        };
        row.set_outcome(None);
        let empty = group.items.is_empty() && group.reclaimable == 0;
        let selectable = group.available && !empty;
        row.selectable.set(selectable);
        row.selection.set_sensitive(selectable);
        row.selection
            .set_active(selectable && group.kind.selected_by_default());
        if group.available {
            row.size.set_text(&format_size(group.reclaimable));
            row.size.root.set_tooltip_text(None);
        } else {
            row.size.set_text("Unavailable");
            row.size
                .root
                .set_tooltip_text(Some(group.note.as_deref().unwrap_or("Cleaner unavailable")));
        }
    }

    pub(super) fn set_started(&self, kind: CleanupKind) {
        if let Some(row) = self.rows.get(&kind) {
            row.size.set_text("Cleaning…");
            row.size.root.set_tooltip_text(None);
            row.energy.start();
        }
    }

    pub(super) fn set_finished(&self, kind: CleanupKind, result: &Result<u64, String>) {
        let Some(row) = self.rows.get(&kind) else {
            return;
        };
        row.energy.stop();
        match result {
            Ok(bytes) => {
                row.set_outcome(Some("success"));
                row.size.set_text(&format!("{} cleaned", format_size(*bytes)));
                row.size.root.set_tooltip_text(None);
            }
            Err(error) => {
                row.set_outcome(Some("error"));
                row.size.set_text("Failed");
                row.size.root.set_tooltip_text(Some(error));
            }
        }
    }

    /// Totals the selected categories and shows the retention settings that apply to them.
    pub(super) fn set_plan(&self, plan: &[(CleanupKind, u64)]) {
        let total = plan
            .iter()
            .fold(0_u64, |total, (_, bytes)| total.saturating_add(*bytes));
        let count = plan.len();
        self.title.set_text(&match count {
            0 => "Nothing selected".to_owned(),
            1 => format!("{} in 1 category", format_size(total)),
            _ => format!("{} across {count} categories", format_size(total)),
        });
        let selected = |kind| plan.iter().any(|(planned, _)| *planned == kind);
        let nix = selected(CleanupKind::Nix);
        let journals = selected(CleanupKind::Journals);
        let open = nix || journals;
        // Rows change in place only inside an open section; one that is opening shows its rows at
        // once, and one that is closing keeps them while it slides shut.
        if open {
            let animate = self.retention.reveals_child();
            reveal(&self.generations_row, nix, animate);
            reveal(&self.journals_row, journals, animate);
        }
        self.retention.set_reveal_child(open);
    }
}

impl PlanRow {
    /// Shows a finished cleanup's result, `success` or `error`, in place of the checkbox; `None`
    /// brings the checkbox back.
    fn set_outcome(&self, outcome: Option<&str>) {
        for class in ["success", "error"] {
            if outcome == Some(class) {
                self.root.add_css_class(class);
            } else {
                self.root.remove_css_class(class);
            }
        }
        self.slot.set_visible_child_name(outcome.unwrap_or("selection"));
    }
}

fn plan_row(kind: CleanupKind) -> PlanRow {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    root.add_css_class("cleanup-plan-row");
    root.add_css_class(kind.tone());

    let open = gtk::Button::new();
    open.add_css_class("cleanup-plan-open");
    open.set_hexpand(true);
    open.set_tooltip_text(Some(&kind.description().replace('\n', " ")));
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let icon = gtk::Image::from_icon_name(kind.icon());
    icon.add_css_class("cleanup-plan-icon");
    icon.add_css_class("tinted");
    icon.set_pixel_size(14);
    icon.set_size_request(28, 28);
    content.append(&icon);
    let name = label(category_title(kind), &["cleanup-plan-name"], 0.0);
    name.set_hexpand(true);
    content.append(&name);
    let size = MorphLabel::new("Scanning…", &["cleanup-plan-size"], 1.0);
    content.append(&size.root);
    open.set_child(Some(&content));
    root.append(&open);

    let selection = gtk::CheckButton::new();
    selection.add_css_class("cleanup-plan-selection");
    selection.set_valign(gtk::Align::Center);
    selection.set_sensitive(false);
    selection.set_tooltip_text(Some("Include in cleanup"));
    let toggled_root = root.downgrade();
    selection.connect_toggled(move |selection| {
        if let Some(root) = toggled_root.upgrade() {
            if selection.is_active() {
                root.add_css_class("selected");
            } else {
                root.remove_css_class("selected");
            }
        }
    });
    // Results take the checkbox's place: it cannot change after cleanup. The stack is as large as
    // its largest child, the themed checkbox, and the badges fill it so they match it exactly.
    let success = gtk::Image::from_icon_name("object-select-symbolic");
    success.set_pixel_size(12);
    success.set_tooltip_text(Some("Cleaned"));
    let error = label("!", &[], 0.5);
    error.set_ellipsize(gtk::pango::EllipsizeMode::None);
    error.set_tooltip_text(Some("Cleanup failed"));
    let slot = gtk::Stack::new();
    slot.set_transition_type(gtk::StackTransitionType::Crossfade);
    slot.set_transition_duration(180);
    slot.set_valign(gtk::Align::Center);
    slot.add_named(&selection, Some("selection"));
    for (badge, name) in [
        (success.upcast_ref::<gtk::Widget>(), "success"),
        (error.upcast_ref(), "error"),
    ] {
        badge.add_css_class("cleanup-plan-outcome");
        badge.add_css_class(name);
        badge.set_halign(gtk::Align::Fill);
        badge.set_valign(gtk::Align::Fill);
        slot.add_named(badge, Some(name));
    }
    root.append(&slot);
    let energy = EnergyOverlay::new(&root, 8.0);

    PlanRow {
        root,
        open,
        size,
        slot,
        selection,
        selectable: Cell::new(false),
        energy,
    }
}

/// A retention setting with its caption and hint, collapsed until its category is selected.
fn retention_row(
    text: &str,
    hint: &str,
    warning: &str,
    input: &impl IsA<gtk::Widget>,
) -> gtk::Revealer {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("cleanup-retention-row");
    row.set_tooltip_text(Some(warning));
    let captions = gtk::Box::new(gtk::Orientation::Vertical, 1);
    captions.set_hexpand(true);
    captions.set_valign(gtk::Align::Center);
    let caption = label(text, &["cleanup-retention-label"], 0.0);
    caption.set_mnemonic_widget(Some(input));
    captions.append(&caption);
    captions.append(&label(hint, &["cleanup-retention-hint"], 0.0));
    row.append(&captions);
    row.append(input);
    slide_down(&row)
}

/// A collapsed revealer that slides `child` down into view.
fn slide_down(child: &impl IsA<gtk::Widget>) -> gtk::Revealer {
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    revealer.set_transition_duration(200);
    revealer.set_child(Some(child));
    revealer
}

/// Shows or hides `revealer`'s child, sliding only when `animate` is set.
fn reveal(revealer: &gtk::Revealer, shown: bool, animate: bool) {
    if animate {
        revealer.set_reveal_child(shown);
        return;
    }
    let transition = revealer.transition_type();
    revealer.set_transition_type(gtk::RevealerTransitionType::None);
    revealer.set_reveal_child(shown);
    revealer.set_transition_type(transition);
}
