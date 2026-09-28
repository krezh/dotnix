use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use gtk::prelude::*;

use crate::nix::NixBuildProgress;
use crate::{ActivityOrder, GraphNodeState, GraphNodeStatus, activity_name, label};

struct ActivityRow {
    root: gtk::Box,
    state: Cell<GraphNodeState>,
    state_widget: RefCell<gtk::Widget>,
    detail: gtk::Label,
    download: gtk::ProgressBar,
}

impl ActivityRow {
    fn new(name: &str, status: &GraphNodeStatus) -> Self {
        let class = status.state.class();
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        root.add_css_class("graph-node");
        root.add_css_class("build-activity-row");
        root.add_css_class(class);
        root.set_size_request(-1, 38);
        let state_widget = state_widget(class);
        state_widget.set_valign(gtk::Align::Start);
        root.append(&state_widget);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 3);
        content.set_hexpand(true);
        let name = label(name, &["graph-node-name"], 0.0);
        name.set_hexpand(true);
        content.append(&name);
        let detail = label("", &["graph-node-detail"], 0.0);
        content.append(&detail);
        let download = gtk::ProgressBar::new();
        download.add_css_class("derivation-download-progress");
        download.set_hexpand(true);
        content.append(&download);
        root.append(&content);
        let row = Self {
            root,
            state: Cell::new(status.state),
            state_widget: RefCell::new(state_widget),
            detail,
            download,
        };
        row.update(status);
        row
    }

    fn update(&self, status: &GraphNodeStatus) {
        let previous = self.state.replace(status.state);
        if previous != status.state {
            self.root.remove_css_class(previous.class());
            self.root.add_css_class(status.state.class());
            let widget = state_widget(status.state.class());
            widget.set_valign(gtk::Align::Start);
            let previous_widget = self.state_widget.replace(widget.clone());
            self.root.remove(&previous_widget);
            self.root.prepend(&widget);
        }
        self.detail.set_visible(status.detail.is_some());
        self.detail
            .set_text(status.detail.as_deref().unwrap_or_default());
        self.download.set_visible(status.download.is_some());
        if let Some((done, expected)) = status.download {
            if expected > 0 {
                self.download
                    .set_fraction((done as f64 / expected as f64).min(1.0));
            } else {
                self.download.pulse();
            }
        }
        self.root.set_visible(true);
    }
}

pub(crate) struct ActivityRows {
    rows: HashMap<String, ActivityRow>,
    order: ActivityOrder,
    empty: gtk::Label,
}

impl ActivityRows {
    pub(crate) fn new(container: &gtk::Box) -> Self {
        let empty = label("", &["build-activity-empty"], 0.0);
        container.append(&empty);
        Self {
            rows: HashMap::new(),
            order: ActivityOrder::default(),
            empty,
        }
    }

    pub(crate) fn update(
        &mut self,
        container: &gtk::Box,
        states: &HashMap<String, GraphNodeStatus>,
        progress: &NixBuildProgress,
    ) {
        for row in self.rows.values() {
            row.root.set_visible(false);
        }
        let mut activity = states
            .iter()
            .filter(|(_, status)| {
                matches!(
                    status.state,
                    GraphNodeState::Building | GraphNodeState::Downloading | GraphNodeState::Failed
                )
            })
            .collect::<Vec<_>>();
        activity.sort_by(|(left_path, _), (right_path, _)| {
            activity_name(left_path)
                .cmp(&activity_name(right_path))
                .then_with(|| left_path.cmp(right_path))
        });
        self.order
            .observe(activity.iter().map(|(path, _)| path.as_str()));
        activity.sort_by_key(|(path, _)| self.order.position(path));
        self.empty.set_visible(activity.is_empty());
        if activity.is_empty() {
            if progress.planned.is_empty() {
                self.empty.set_text("Preparing build work");
            } else {
                self.empty.set_text(&format!(
                    "Waiting to start {} queued builds",
                    progress.planned.len()
                ));
            }
        }
        let mut prev: Option<gtk::Widget> = None;
        for (path, status) in activity {
            let row_widget = if let Some(row) = self.rows.get(path) {
                row.update(status);
                row.root.clone()
            } else {
                let row = ActivityRow::new(&activity_name(path), status);
                let widget = row.root.clone();
                container.append(&widget);
                self.rows.insert(path.clone(), row);
                widget
            };
            container.reorder_child_after(&row_widget, prev.as_ref());
            prev = Some(row_widget.upcast());
        }
    }
}

fn state_widget(class: &str) -> gtk::Widget {
    if matches!(class, "building" | "downloading") {
        let spinner = gtk::Spinner::new();
        spinner.add_css_class("graph-state");
        spinner.add_css_class(class);
        spinner.set_size_request(18, 18);
        spinner.start();
        spinner.upcast()
    } else {
        let icon = match class {
            "complete" => "object-select-symbolic",
            "failed" => "dialog-warning-symbolic",
            "planned" => "media-record-symbolic",
            _ => "go-next-symbolic",
        };
        let image = gtk::Image::from_icon_name(icon);
        image.add_css_class("graph-state");
        image.add_css_class(class);
        image.set_pixel_size(17);
        image.upcast()
    }
}
