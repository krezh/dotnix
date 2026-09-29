use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;

use crate::theme;
use crate::ui::build_progress::{GraphNodeState, GraphNodeStatus, activity_name};
use crate::ui::common::animations_enabled;

const ROW_HEIGHT: f64 = 26.0;
const CONTENT_PADDING: f64 = 8.0;
const MAX_ACTIVITY_SLOTS: usize = 8;

pub(crate) struct BuildActivityView {
    pub(crate) widget: gtk::DrawingArea,
    model: Rc<RefCell<ActivityModel>>,
    phase: Rc<Cell<f64>>,
    animating: Rc<Cell<bool>>,
}

impl BuildActivityView {
    pub(crate) fn new(font: &str) -> Self {
        let widget = gtk::DrawingArea::new();
        widget.add_css_class("build-activity");
        widget.set_content_height(350);
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        let model = Rc::new(RefCell::new(ActivityModel::default()));
        let phase = Rc::new(Cell::new(0.0));
        let animating = Rc::new(Cell::new(false));
        let draw_model = Rc::clone(&model);
        let draw_phase = Rc::clone(&phase);
        let draw_font = font.to_owned();
        widget.set_draw_func(move |_, context, width, height| {
            draw_activity(
                context,
                width,
                height,
                &draw_model.borrow(),
                draw_phase.get(),
                &draw_font,
            );
        });
        if animations_enabled() {
            let tick_phase = Rc::clone(&phase);
            let tick_animating = Rc::clone(&animating);
            widget.add_tick_callback(move |widget, clock| {
                if tick_animating.get() {
                    tick_phase.set(clock.frame_time() as f64 / 1_000_000.0);
                    widget.queue_draw();
                }
                gtk::glib::ControlFlow::Continue
            });
        }
        Self {
            widget,
            model,
            phase,
            animating,
        }
    }

    pub(crate) fn update(&self, states: &HashMap<String, GraphNodeStatus>) {
        let mut model = self.model.borrow_mut();
        if !model.update(states) {
            return;
        }
        drop(model);
        self.sync_animation();
        self.widget.queue_draw();
    }

    fn sync_animation(&self) {
        let active = self.model.borrow().entries.iter().any(|entry| {
            matches!(
                entry.status.state,
                GraphNodeState::Building | GraphNodeState::Downloading
            )
        });
        self.animating.set(active);
        if !active {
            self.phase.set(0.0);
        }
    }
}

#[derive(Default)]
struct ActivityModel {
    entries: Vec<ActivityEntry>,
    indices: HashMap<String, usize>,
    slots: Vec<usize>,
    replacement_cursor: usize,
    queued: usize,
    rows: Vec<ActivityRow>,
}

impl ActivityModel {
    fn update(&mut self, states: &HashMap<String, GraphNodeStatus>) -> bool {
        let mut changed = false;
        for entry in &mut self.entries {
            if matches!(
                entry.status.state,
                GraphNodeState::Building | GraphNodeState::Downloading
            ) && !states.contains_key(&entry.path)
            {
                entry.status = GraphNodeStatus {
                    state: GraphNodeState::Complete,
                    detail: Some("complete".to_owned()),
                };
                changed = true;
            }
        }
        let mut paths = states.keys().collect::<Vec<_>>();
        paths.sort_by(|left, right| {
            activity_name(left)
                .cmp(&activity_name(right))
                .then_with(|| left.cmp(right))
        });
        for path in paths {
            let status = states[path].clone();
            let index = if let Some(index) = self.indices.get(path).copied() {
                if self.entries[index].status != status {
                    self.entries[index].status = status;
                    changed = true;
                }
                index
            } else {
                let index = self.entries.len();
                self.indices.insert(path.clone(), index);
                self.entries.push(ActivityEntry {
                    path: path.clone(),
                    name: activity_name(path),
                    status,
                    shown: false,
                });
                changed = true;
                index
            };
            if self.entries[index].status.state != GraphNodeState::Planned
                && !self.entries[index].shown
            {
                self.assign_slot(index);
                self.entries[index].shown = true;
                changed = true;
            }
        }
        let queued = self
            .entries
            .iter()
            .filter(|entry| entry.status.state == GraphNodeState::Planned)
            .count();
        if self.queued != queued {
            self.queued = queued;
            changed = true;
        }
        if changed {
            self.rebuild_rows();
        }
        changed
    }

    fn assign_slot(&mut self, entry: usize) {
        if self.slots.len() < MAX_ACTIVITY_SLOTS {
            self.slots.push(entry);
            return;
        }
        for offset in 0..self.slots.len() {
            let position = (self.replacement_cursor + offset) % self.slots.len();
            if self.entries[self.slots[position]].status.state == GraphNodeState::Complete {
                self.slots[position] = entry;
                self.replacement_cursor = (position + 1) % self.slots.len();
                return;
            }
        }
        self.slots.push(entry);
    }

    fn rebuild_rows(&mut self) {
        self.rows = self
            .slots
            .iter()
            .map(|index| ActivityRow::entry(&self.entries[*index]))
            .collect();
    }
}

struct ActivityEntry {
    path: String,
    name: String,
    status: GraphNodeStatus,
    shown: bool,
}

struct ActivityRow {
    name: String,
    detail: String,
    state: GraphNodeState,
}

impl ActivityRow {
    fn entry(entry: &ActivityEntry) -> Self {
        Self {
            name: compact_name(&entry.name, 48),
            detail: entry
                .status
                .detail
                .clone()
                .filter(|detail| !detail.is_empty())
                .unwrap_or_else(|| state_label(entry.status.state).to_owned()),
            state: entry.status.state,
        }
    }
}

fn draw_activity(
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    model: &ActivityModel,
    phase: f64,
    font: &str,
) {
    let width = f64::from(width);
    let height = f64::from(height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    if model.rows.is_empty() {
        let message = if model.queued > 0 {
            format!("Waiting to start {} queued items…", model.queued)
        } else {
            "Waiting for build activity…".to_owned()
        };
        draw_centered_message(context, width, height, &message, font);
        return;
    }
    let top = CONTENT_PADDING;
    let detail_x = detail_column(width);
    for (index, row) in model.rows.iter().enumerate() {
        let y = top + index as f64 * ROW_HEIGHT;
        let center_y = y + ROW_HEIGHT / 2.0;
        if index > 0 {
            theme::set_source_rgba(context, theme::SURFACE1, 0.25);
            context.set_line_width(1.0);
            context.move_to(12.0, y + 0.5);
            context.line_to(width - 12.0, y + 0.5);
            let _ = context.stroke();
        }
        if matches!(
            row.state,
            GraphNodeState::Building | GraphNodeState::Downloading
        ) {
            let pulse = (phase * 2.2).sin() * 0.5 + 0.5;
            theme::set_source_rgba(context, theme::SAPPHIRE, 0.03 + pulse * 0.04);
            theme::rounded_rectangle(context, 7.0, y + 1.0, width - 14.0, ROW_HEIGHT - 2.0, 4.0);
            let _ = context.fill();
        }
        let color = state_color(Some(row.state));
        context.arc(18.0, center_y, 4.0, 0.0, std::f64::consts::TAU);
        theme::set_source_rgba(context, color, 0.95);
        let _ = context.fill();

        context.save().ok();
        context.rectangle(30.0, y, (detail_x - 38.0).max(20.0), ROW_HEIGHT);
        context.clip();
        context.select_font_face(
            font,
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Normal,
        );
        context.set_font_size(11.5);
        theme::set_source_rgba(context, theme::TEXT, 0.97);
        context.move_to(30.0, y + 18.0);
        let _ = context.show_text(&row.name);
        context.restore().ok();

        context.save().ok();
        context.rectangle(detail_x, y, (width - detail_x - 10.0).max(10.0), ROW_HEIGHT);
        context.clip();
        context.set_font_size(10.5);
        theme::set_source_rgba(context, color, 0.92);
        context.move_to(detail_x, y + 18.0);
        let _ = context.show_text(&row.detail);
        context.restore().ok();
    }
}

fn draw_centered_message(
    context: &gtk::cairo::Context,
    width: f64,
    height: f64,
    message: &str,
    font: &str,
) {
    context.select_font_face(
        font,
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Normal,
    );
    context.set_font_size(12.0);
    let text_width = context
        .text_extents(message)
        .map_or(0.0, |value| value.width());
    theme::set_source_rgba(context, theme::OVERLAY0, 0.9);
    context.move_to(((width - text_width) / 2.0).max(12.0), height / 2.0);
    let _ = context.show_text(message);
}

fn compact_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        return name.to_owned();
    }
    let mut compact = name
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    compact.push('…');
    compact
}

fn state_color(state: Option<GraphNodeState>) -> (f64, f64, f64) {
    match state {
        Some(GraphNodeState::Planned) => theme::LAVENDER,
        Some(GraphNodeState::Building) => theme::YELLOW,
        Some(GraphNodeState::Downloading) => theme::SAPPHIRE,
        Some(GraphNodeState::Complete) => theme::GREEN,
        Some(GraphNodeState::Failed) => theme::RED,
        None => theme::SURFACE1,
    }
}

fn state_label(state: GraphNodeState) -> &'static str {
    match state {
        GraphNodeState::Planned => "queued",
        GraphNodeState::Building => "building",
        GraphNodeState::Downloading => "downloading",
        GraphNodeState::Complete => "complete",
        GraphNodeState::Failed => "failed",
    }
}

fn detail_column(width: f64) -> f64 {
    (width * 0.66).clamp(260.0, (width - 130.0).max(260.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(state: GraphNodeState, detail: Option<&str>) -> GraphNodeStatus {
        GraphNodeStatus {
            state,
            detail: detail.map(str::to_owned),
        }
    }

    #[test]
    fn planned_work_does_not_fill_activity_slots() {
        let mut model = ActivityModel::default();
        let states = (0..12)
            .map(|index| {
                (
                    format!("/nix/store/hash-item-{index}.drv"),
                    status(GraphNodeState::Planned, None),
                )
            })
            .collect();
        assert!(model.update(&states));
        assert_eq!(model.entries.len(), 12);
        assert!(model.rows.is_empty());
        assert_eq!(model.queued, 12);
    }

    #[test]
    fn completed_work_keeps_its_slot() {
        let mut model = ActivityModel::default();
        let path = "/nix/store/hash-swix.drv".to_owned();
        model.update(&HashMap::from([(
            path,
            status(GraphNodeState::Building, Some("compiling phase 12")),
        )]));
        assert_eq!(model.rows[0].name, "swix");
        assert_eq!(model.rows[0].detail, "compiling phase 12");

        model.update(&HashMap::new());
        assert_eq!(model.rows[0].name, "swix");
        assert_eq!(model.rows[0].detail, "complete");
    }

    #[test]
    fn new_work_reuses_a_completed_slot_without_moving_other_rows() {
        let mut model = ActivityModel::default();
        let active = (0..MAX_ACTIVITY_SLOTS)
            .map(|index| {
                (
                    format!("/nix/store/hash-item-{index}.drv"),
                    status(GraphNodeState::Building, None),
                )
            })
            .collect::<HashMap<_, _>>();
        model.update(&active);
        let original = model
            .rows
            .iter()
            .map(|row| row.name.clone())
            .collect::<Vec<_>>();
        let complete = active
            .keys()
            .map(|path| (path.clone(), status(GraphNodeState::Complete, None)))
            .chain([(
                "/nix/store/hash-replacement.drv".to_owned(),
                status(GraphNodeState::Building, None),
            )])
            .collect();
        model.update(&complete);

        assert_eq!(model.rows.len(), MAX_ACTIVITY_SLOTS);
        assert_eq!(model.rows[0].name, "replacement");
        assert_eq!(
            model.rows[1..]
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            original[1..].iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unchanged_updates_do_not_rebuild_the_view() {
        let mut model = ActivityModel::default();
        let states = HashMap::from([(
            "/nix/store/hash-swix.drv".to_owned(),
            status(GraphNodeState::Building, None),
        )]);
        assert!(model.update(&states));
        assert!(!model.update(&states));
    }

    #[test]
    fn detail_column_is_bounded_for_narrow_views() {
        assert_eq!(detail_column(300.0), 260.0);
        assert!(detail_column(593.0) < 593.0);
        assert!(detail_column(900.0) < 900.0);
    }
}
