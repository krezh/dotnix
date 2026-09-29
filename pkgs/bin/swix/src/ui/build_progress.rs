use std::cell::Cell;
use std::collections::HashMap;

use gtk::glib;
use gtk::prelude::*;

use crate::nix::NixBuildProgress;
use crate::ui::build_activity::BuildActivityView;
use crate::ui::common::{animations_enabled, clear, label, size};

pub(crate) struct BuildSummary {
    pub(crate) root: gtk::Box,
    pub(crate) outline: BuildActivityView,
    pub(crate) count: gtk::Label,
    pub(crate) building: gtk::Label,
    pub(crate) downloading: gtk::Label,
    pub(crate) complete: gtk::Label,
    pub(crate) planned: gtk::Label,
    pub(crate) failed_group: gtk::Box,
    pub(crate) failed: gtk::Label,
}

pub(crate) fn build_summary(font: &str) -> BuildSummary {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("build-summary");
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    heading.append(&label("Build activity", &["build-summary-title"], 0.0));
    let count = label("Waiting for work", &["build-summary-count"], 1.0);
    count.set_hexpand(true);
    heading.append(&count);
    root.append(&heading);
    let outline = BuildActivityView::new(font);
    root.append(&outline.widget);
    let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    metrics.add_css_class("build-summary-metrics");
    let (building_group, building) = summary_metric("system-run-symbolic", "building");
    let (downloading_group, downloading) =
        summary_metric("folder-download-symbolic", "downloading");
    let (complete_group, complete) = summary_metric("object-select-symbolic", "complete");
    let (planned_group, planned) = summary_metric("media-playback-pause-symbolic", "planned");
    let (failed_group, failed) = summary_metric("dialog-warning-symbolic", "failed");
    failed_group.set_visible(false);
    metrics.append(&building_group);
    metrics.append(&downloading_group);
    metrics.append(&complete_group);
    metrics.append(&planned_group);
    metrics.append(&failed_group);
    root.append(&metrics);
    BuildSummary {
        root,
        outline,
        count,
        building,
        downloading,
        complete,
        planned,
        failed_group,
        failed,
    }
}
fn summary_metric(icon: &str, class: &str) -> (gtk::Box, gtk::Label) {
    let group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    group.add_css_class("build-summary-metric");
    group.add_css_class(class);
    group.set_width_request(128);
    let icon = gtk::Image::from_icon_name(icon);
    icon.set_pixel_size(16);
    group.append(&icon);
    let value = label("0", &["build-summary-value"], 0.0);
    group.append(&value);
    (group, value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphNodeState {
    Planned,
    Building,
    Downloading,
    Complete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GraphNodeStatus {
    pub(crate) state: GraphNodeState,
    pub(crate) detail: Option<String>,
}

#[derive(Default)]
pub(crate) struct BuildPlan {
    nodes: Vec<(String, GraphNodeState)>,
    indices: HashMap<String, usize>,
}

impl BuildPlan {
    pub(crate) fn update<'a>(
        &mut self,
        states: &HashMap<String, GraphNodeStatus>,
        ordered_paths: impl IntoIterator<Item = &'a str>,
    ) {
        for (path, state) in &mut self.nodes {
            if matches!(
                *state,
                GraphNodeState::Building | GraphNodeState::Downloading
            ) && !states.contains_key(path)
            {
                *state = GraphNodeState::Complete;
            }
        }
        for path in ordered_paths {
            let state = states[path].state;
            if let Some(index) = self.indices.get(path).copied() {
                self.nodes[index].1 = state;
            } else {
                let path = path.to_owned();
                self.indices.insert(path.clone(), self.nodes.len());
                self.nodes.push((path, state));
            }
        }
    }

    pub(crate) fn summary(&self) -> (usize, usize, usize, usize, usize) {
        let mut building = 0;
        let mut downloading = 0;
        let mut complete = 0;
        let mut planned = 0;
        let mut failed = 0;
        for (_, state) in &self.nodes {
            match state {
                GraphNodeState::Building => building += 1,
                GraphNodeState::Downloading => downloading += 1,
                GraphNodeState::Complete => complete += 1,
                GraphNodeState::Planned => planned += 1,
                GraphNodeState::Failed => failed += 1,
            }
        }
        (building, downloading, complete, planned, failed)
    }
}

pub(crate) fn build_node_states(progress: &NixBuildProgress) -> HashMap<String, GraphNodeStatus> {
    let mut states = progress
        .planned
        .iter()
        .map(|path| {
            (
                path.clone(),
                GraphNodeStatus {
                    state: GraphNodeState::Planned,
                    detail: None,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    for path in &progress.completed {
        states
            .entry(path.clone())
            .and_modify(|status| status.state = GraphNodeState::Complete)
            .or_insert(GraphNodeStatus {
                state: GraphNodeState::Complete,
                detail: None,
            });
    }
    for path in &progress.failed {
        states
            .entry(path.clone())
            .and_modify(|status| {
                status.state = GraphNodeState::Failed;
                status.detail = Some("Build failed".to_owned());
            })
            .or_insert(GraphNodeStatus {
                state: GraphNodeState::Failed,
                detail: Some("Build failed".to_owned()),
            });
    }
    for item in &progress.items {
        let Some(path) = item.path.as_deref() else {
            continue;
        };
        let state = if item.failed > 0 {
            GraphNodeState::Failed
        } else if item.kind == 101 {
            GraphNodeState::Downloading
        } else {
            GraphNodeState::Building
        };
        let priority = |state| match state {
            GraphNodeState::Failed => 5,
            GraphNodeState::Downloading => 4,
            GraphNodeState::Building => 3,
            GraphNodeState::Complete => 2,
            GraphNodeState::Planned => 1,
        };
        if states
            .get(path)
            .is_some_and(|current| priority(current.state) > priority(state))
        {
            continue;
        }
        let detail = match state {
            GraphNodeState::Downloading if item.expected > 0 => Some(format!(
                "{} of {} downloaded",
                size(item.done as i64),
                size(item.expected as i64)
            )),
            GraphNodeState::Building => item.detail.clone(),
            GraphNodeState::Failed => Some("Build failed".to_owned()),
            _ => None,
        };
        states.insert(path.to_owned(), GraphNodeStatus { state, detail });
    }
    states
}
pub(crate) fn activity_name(text: &str) -> String {
    let Some(store_name) = text
        .split("/nix/store/")
        .nth(1)
        .and_then(|value| value.split(['\'', ' ']).next())
    else {
        return text.to_owned();
    };
    store_name
        .trim_end_matches(".drv")
        .split_once('-')
        .map_or(store_name, |(_, name)| name)
        .to_owned()
}

pub(crate) fn render_flake_fetches(container: &gtk::Box, inputs: &[String], active: bool) {
    clear(container);
    for input in inputs {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("flake-fetch");
        let state: gtk::Widget = if active {
            let spinner = gtk::Spinner::new();
            spinner.set_size_request(16, 16);
            spinner.start();
            spinner.upcast()
        } else {
            let icon = gtk::Image::from_icon_name("object-select-symbolic");
            icon.add_css_class("flake-fetch-complete");
            icon.set_pixel_size(16);
            icon.upcast()
        };
        row.append(&state);
        row.append(&label(input, &["flake-fetch-name"], 0.0));
        container.append(&row);
    }
}

fn animate_evaluation_warning(row: &gtk::Box) {
    if !animations_enabled() {
        return;
    }
    row.set_opacity(0.0);
    row.set_margin_start(18);
    let started = Cell::new(None);
    row.add_tick_callback(move |row, frame_clock| {
        let now = frame_clock.frame_time();
        let started = started.get().unwrap_or_else(|| {
            started.set(Some(now));
            now
        });
        let progress = ((now - started) as f64 / 240_000.0).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        row.set_opacity(eased);
        row.set_margin_start(((1.0 - eased) * 18.0).round() as i32);
        if progress >= 1.0 {
            row.set_opacity(1.0);
            row.set_margin_start(0);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

pub(crate) fn append_evaluation_warnings(container: &gtk::Box, warnings: &[String]) {
    for warning in warnings {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("evaluation-warning");
        let icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        icon.add_css_class("evaluation-warning-icon");
        icon.set_pixel_size(16);
        icon.set_valign(gtk::Align::Start);
        row.append(&icon);
        let text = label(warning, &["evaluation-warning-text"], 0.0);
        text.set_hexpand(true);
        text.set_selectable(true);
        text.set_ellipsize(gtk::pango::EllipsizeMode::None);
        text.set_wrap(true);
        text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        row.append(&text);
        container.append(&row);
        animate_evaluation_warning(&row);
    }
}

pub(crate) fn render_build_progress(
    summary: &BuildSummary,
    progress: &NixBuildProgress,
    build_plan: &mut BuildPlan,
) {
    let states = build_node_states(progress);
    let mut plan = states.iter().collect::<Vec<_>>();
    plan.sort_by(|(left_path, _), (right_path, _)| {
        activity_name(left_path)
            .cmp(&activity_name(right_path))
            .then_with(|| left_path.cmp(right_path))
    });
    build_plan.update(&states, plan.iter().map(|(path, _)| path.as_str()));
    summary.outline.update(&states);
    let (building, downloading, complete, planned, failed) = build_plan.summary();
    let total = build_plan.nodes.len();
    summary.building.set_text(&format!("{building} building"));
    summary
        .downloading
        .set_text(&format!("{downloading} downloading"));
    summary.complete.set_text(&format!("{complete} complete"));
    summary.planned.set_text(&format!("{planned} queued"));
    summary.failed.set_text(&format!("{failed} failed"));
    summary.failed_group.set_visible(failed > 0);
    summary
        .count
        .set_text(&format!("{complete} of {total} work items complete"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_plan_keeps_nodes_and_updates_colors_in_place() {
        let status = |state| GraphNodeStatus {
            state,
            detail: None,
        };
        let mut plan = BuildPlan::default();
        let initial = HashMap::from([
            ("bravo".to_owned(), status(GraphNodeState::Building)),
            ("charlie".to_owned(), status(GraphNodeState::Planned)),
        ]);
        plan.update(&initial, ["bravo", "charlie"]);

        let next = HashMap::from([
            ("alpha".to_owned(), status(GraphNodeState::Downloading)),
            ("charlie".to_owned(), status(GraphNodeState::Building)),
        ]);
        plan.update(&next, ["alpha", "charlie"]);

        assert_eq!(
            plan.nodes,
            [
                ("bravo".to_owned(), GraphNodeState::Complete),
                ("charlie".to_owned(), GraphNodeState::Building),
                ("alpha".to_owned(), GraphNodeState::Downloading),
            ]
        );
        assert_eq!(plan.summary(), (1, 1, 1, 0, 0));
    }
}
