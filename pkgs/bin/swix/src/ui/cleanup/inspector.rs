use gtk::prelude::*;

use crate::cleanup::{CleanupGroup, CleanupItem, CleanupKind, format_size};
use crate::ui::common::label;

const KNOWN_ITEM_LIMIT: usize = 18;
const DEFERRED_ITEM_LIMIT: usize = 6;

pub(super) struct CleanupInspector {
    pub(super) root: gtk::Box,
    title: gtk::Label,
    summary: gtk::Label,
    note: gtk::Label,
    items: gtk::Grid,
}

impl CleanupInspector {
    pub(super) fn new() -> (Self, gtk::Adjustment) {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("cleanup-insight");
        root.set_width_request(660);
        root.set_vexpand(true);

        root.append(&label("REMOVAL PLAN", &["cleanup-insight-eyebrow"], 0.0));
        let title = label("Nix store & generations", &["cleanup-insight-title"], 0.0);
        root.append(&title);
        let summary = label(
            "Scanning this cleanup job…",
            &["cleanup-insight-summary"],
            0.0,
        );
        root.append(&summary);
        let note = label("", &["cleanup-insight-note"], 0.0);
        note.set_wrap(true);
        note.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        note.set_visible(false);
        root.append(&note);

        let items = gtk::Grid::new();
        items.set_column_homogeneous(true);
        items.set_column_spacing(8);
        items.set_row_spacing(6);
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("cleanup-insight-scroll");
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&items));
        let adjustment = scroller.vadjustment();
        root.append(&scroller);

        (
            Self {
                root,
                title,
                summary,
                note,
                items,
            },
            adjustment,
        )
    }

    pub(super) fn set_loading(&self, kind: CleanupKind) {
        self.title.set_text(kind.title());
        self.summary.set_text("Scanning this cleanup job…");
        self.note.set_visible(false);
        clear_items(&self.items);
    }

    pub(super) fn set_group(&self, group: &CleanupGroup) {
        self.title.set_text(group.kind.title());
        if group.available {
            self.summary.set_text(&format!(
                "{} item{} · {} estimated",
                group.items.len(),
                if group.items.len() == 1 { "" } else { "s" },
                format_size(group.reclaimable),
            ));
        } else {
            self.summary.set_text("Cleanup job unavailable");
        }
        if let Some(note) = &group.note {
            self.note.set_text(note);
            self.note.set_visible(true);
        } else {
            self.note.set_visible(false);
        }

        clear_items(&self.items);
        if group.items.is_empty() {
            let text = if group.available {
                "Nothing is currently eligible for removal"
            } else {
                "The scan did not return a removal plan"
            };
            self.items
                .attach(&label(text, &["cleanup-insight-empty"], 0.0), 0, 0, 2, 1);
            return;
        }

        let largest = group
            .items
            .iter()
            .filter_map(|item| item.bytes)
            .max()
            .unwrap_or(0);
        let deferred = group
            .items
            .iter()
            .filter(|item| item.bytes.is_none())
            .take(DEFERRED_ITEM_LIMIT);
        let known = group
            .items
            .iter()
            .filter(|item| item.bytes.is_some())
            .take(KNOWN_ITEM_LIMIT);
        let mut displayed = 0;
        for item in deferred.chain(known) {
            self.items.attach(
                &item_row(item, largest),
                (displayed % 2) as i32,
                (displayed / 2) as i32,
                1,
                1,
            );
            displayed += 1;
        }

        let remaining = group.items.len().saturating_sub(displayed);
        if remaining > 0 {
            self.items.attach(
                &label(
                    &format!("+ {remaining} more items included in this job"),
                    &["cleanup-insight-more"],
                    0.0,
                ),
                0,
                displayed.div_ceil(2) as i32,
                2,
                1,
            );
        }
    }
}

fn clear_items(items: &gtk::Grid) {
    while let Some(child) = items.first_child() {
        items.remove(&child);
    }
}

fn item_row(item: &CleanupItem, largest: u64) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 3);
    row.add_css_class("cleanup-insight-row");
    row.set_hexpand(true);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let name = label(&item.label, &["cleanup-insight-item-title"], 0.0);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    name.set_tooltip_text(Some(&item.label));
    name.set_hexpand(true);
    heading.append(&name);
    let amount = item
        .bytes
        .map_or_else(|| "On cleanup".to_owned(), format_size);
    heading.append(&label(&amount, &["cleanup-insight-item-size"], 1.0));
    row.append(&heading);

    let detail = label(&item.detail, &["cleanup-insight-item-detail"], 0.0);
    detail.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    detail.set_tooltip_text(Some(&item.detail));
    row.append(&detail);

    if let Some(bytes) = item.bytes
        && bytes > 0
        && largest > 0
    {
        let bar = gtk::ProgressBar::new();
        bar.add_css_class("cleanup-insight-bar");
        bar.set_fraction((bytes as f64 / largest as f64).clamp(0.0, 1.0));
        row.append(&bar);
    }
    row
}
