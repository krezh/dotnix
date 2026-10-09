use std::cell::Cell;

use gtk::prelude::*;

use crate::cleanup::{CleanupGroup, CleanupKind, format_size};
use crate::ui::common::{MorphLabel, label};
use crate::ui::timeline::EnergyOverlay;

pub(super) struct CleanupCard {
    pub(super) root: gtk::Button,
    detail: MorphLabel,
    amount: MorphLabel,
    energy: EnergyOverlay,
    selected: Cell<bool>,
}

impl CleanupCard {
    pub(super) fn new(kind: CleanupKind) -> Self {
        let root = gtk::Button::new();
        root.add_css_class("cleanup-card");
        root.set_sensitive(false);

        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        content.set_valign(gtk::Align::Center);
        let selection = gtk::Image::from_icon_name("object-select-symbolic");
        selection.add_css_class("cleanup-card-selection");
        selection.set_pixel_size(14);
        content.append(&selection);
        let icon = gtk::Image::from_icon_name(kind.icon());
        icon.add_css_class("cleanup-card-icon");
        icon.set_pixel_size(24);
        icon.set_size_request(34, 34);
        content.append(&icon);

        let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
        copy.set_hexpand(true);
        copy.append(&label(kind.title(), &["cleanup-card-title"], 0.0));
        let detail = MorphLabel::new("Scanning…", &["cleanup-card-detail"], 0.0);
        copy.append(&detail.root);
        content.append(&copy);

        let amount = MorphLabel::new("…", &["cleanup-card-amount"], 1.0);
        amount.root.set_valign(gtk::Align::Center);
        content.append(&amount.root);
        root.set_child(Some(&content));
        let energy = EnergyOverlay::new(&root, 9.0);

        Self {
            root,
            detail,
            amount,
            energy,
            selected: Cell::new(false),
        }
    }

    pub(super) fn widget(&self) -> &gtk::Overlay {
        &self.energy.root
    }

    pub(super) fn set_group(&self, group: &CleanupGroup) {
        self.root.remove_css_class("success");
        self.root.remove_css_class("error");
        self.amount.remove_css_class("success");
        self.amount.remove_css_class("error");
        let selectable = group.available && (!group.items.is_empty() || group.reclaimable > 0);
        self.root.set_sensitive(selectable);
        self.set_selected(selectable);
        if group.available {
            self.amount.set_text(&format_size(group.reclaimable));
            self.detail.set_text(&format!(
                "{} item{} · {}",
                group.items.len(),
                if group.items.len() == 1 { "" } else { "s" },
                group.kind.description()
            ));
        } else {
            self.amount.set_text("Unavailable");
            self.detail
                .set_text(group.note.as_deref().unwrap_or("Cleaner unavailable"));
        }
    }

    pub(super) fn is_selected(&self) -> bool {
        self.selected.get()
    }

    pub(super) fn toggle_selected(&self) {
        self.set_selected(!self.selected.get());
    }

    fn set_selected(&self, selected: bool) {
        self.selected.set(selected);
        if selected {
            self.root.add_css_class("selected");
        } else {
            self.root.remove_css_class("selected");
        }
    }

    pub(super) fn set_started(&self) {
        self.root.set_sensitive(false);
        self.detail.set_text("Cleaning…");
        self.amount.set_text("Working");
        self.energy.start();
    }

    pub(super) fn set_finished(&self, result: &Result<u64, String>) {
        self.energy.stop();
        match result {
            Ok(bytes) => {
                self.root.add_css_class("success");
                self.amount.add_css_class("success");
                self.amount
                    .set_text(&format!("{} cleaned", format_size(*bytes)));
                self.detail.set_text("Cleanup complete");
            }
            Err(error) => {
                self.root.add_css_class("error");
                self.amount.add_css_class("error");
                self.amount.set_text("Failed");
                self.detail.set_text(error.lines().next().unwrap_or(error));
            }
        }
    }
}
