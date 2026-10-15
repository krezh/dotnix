//! The input list: rows, filtering, sort order, and the slide after a re-sort.

use std::cell::Cell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::time::SystemTime;

use gtk::glib;
use gtk::prelude::*;

use crate::inputs::FlakeInput;
use crate::ui::theme;
use crate::ui::{animations_enabled, duration_text, label, seconds_since, short_age};

use super::{InputsView, STALE_AFTER};

/// Re-sorts the list, then slides each row from where it was drawn to its new slot.
///
/// The new layout is computed up front from the sort order and the rows' heights, so the very
/// first frame after re-sorting already draws every row at its old position: no jump, no flash.
pub(super) fn resort_animated(view: &Rc<InputsView>) {
    let rows: Vec<gtk::ListBoxRow> = list_rows(&view.list)
        .filter(|row| row.is_child_visible())
        .collect();
    // (row, drawn y including any offset still applied mid-slide, height)
    let measured: Vec<(gtk::ListBoxRow, f64, f64)> = if animations_enabled() {
        rows.into_iter()
            .filter_map(|row| {
                let bounds = row.compute_bounds(&view.list)?;
                Some((row, f64::from(bounds.y()), f64::from(bounds.height())))
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut moves = Vec::new();
    if !measured.is_empty() {
        let offsets = view.row_offsets.borrow();
        let offset_of = |row: &gtk::ListBoxRow| {
            offsets
                .get(row.widget_name().as_str())
                .map_or(0.0, |entry| entry.1)
        };
        // Rows stack without gaps, so the topmost layout slot plus heights gives every new slot.
        let top = measured
            .iter()
            .map(|(row, drawn, _)| drawn - offset_of(row))
            .fold(f64::INFINITY, f64::min);
        drop(offsets);
        let inputs = view.inputs.borrow();
        let oldest_first = view.sort_oldest.is_active();
        let find = |row: &gtk::ListBoxRow| {
            let name = row.widget_name();
            inputs.iter().find(|input| input.name == name)
        };
        let mut order: Vec<&(gtk::ListBoxRow, f64, f64)> = measured.iter().collect();
        order.sort_by(|left, right| match (find(&left.0), find(&right.0)) {
            (Some(left), Some(right)) => compare_inputs(left, right, oldest_first),
            _ => Ordering::Equal,
        });
        let mut slot = top;
        for (row, drawn, height) in order {
            moves.push((row.clone(), drawn - slot));
            slot += height;
        }
    }
    view.list.invalidate_sort();
    let animation_id = view.resort_animation.get().wrapping_add(1);
    view.resort_animation.set(animation_id);
    // Rows already in their slot stay put, without the opaque mid-slide background.
    for (row, offset) in &mut moves {
        if offset.abs() < 0.5 {
            *offset = 0.0;
        }
        set_row_offset(view, row, *offset);
    }
    moves.retain(|(_, offset)| *offset != 0.0);
    if moves.is_empty() {
        view.list.remove_css_class("resorting");
        return;
    }
    // Opaque row backgrounds while rows pass each other, so text never overlaps mid-slide.
    view.list.add_css_class("resorting");
    const DURATION: f64 = 0.32;
    let weak = Rc::downgrade(view);
    let started = Cell::new(None);
    view.list.add_tick_callback(move |_, frame_clock| {
        let Some(view) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if view.resort_animation.get() != animation_id {
            return glib::ControlFlow::Break;
        }
        let now = frame_clock.frame_time();
        let started = started.get().unwrap_or_else(|| {
            started.set(Some(now));
            now
        });
        let progress = ((now - started) as f64 / 1_000_000.0 / DURATION).min(1.0);
        // Ease-out cubic on the remaining distance.
        let remaining = (1.0 - progress).powi(3);
        for (row, offset) in &moves {
            set_row_offset(&view, row, offset * remaining);
        }
        if progress >= 1.0 {
            view.list.remove_css_class("resorting");
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

/// Stops any slide in progress and puts every row straight into its slot.
pub(super) fn stop_resort(view: &InputsView) {
    view.resort_animation
        .set(view.resort_animation.get().wrapping_add(1));
    view.list.remove_css_class("resorting");
    for (provider, offset) in view.row_offsets.borrow_mut().values_mut() {
        if *offset != 0.0 {
            *offset = 0.0;
            #[allow(deprecated)]
            provider.load_from_data("");
        }
    }
}

pub(super) fn list_rows(list: &gtk::ListBox) -> impl Iterator<Item = gtk::ListBoxRow> {
    std::iter::successors(list.first_child(), |child| child.next_sibling())
        .filter_map(|child| child.downcast().ok())
}

fn set_row_offset(view: &InputsView, row: &gtk::ListBoxRow, offset: f64) {
    let mut offsets = view.row_offsets.borrow_mut();
    let entry = offsets
        .entry(row.widget_name().to_string())
        .or_insert_with(|| {
            let provider = gtk::CssProvider::new();
            // A widget-scoped provider styles only this row, so `*` matches nothing else.
            #[allow(deprecated)]
            row.style_context()
                .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_USER + 2);
            (provider, 0.0)
        });
    if (entry.1 - offset).abs() < 0.01 {
        return;
    }
    entry.1 = offset;
    let css = if offset.abs() < 0.01 {
        String::new()
    } else {
        format!("* {{ transform: translateY({offset:.2}px); }}")
    };
    #[allow(deprecated)]
    entry.0.load_from_data(&css);
}

pub(super) fn input_row(input: &FlakeInput, now: SystemTime) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("input-row");
    row.add_css_class("list-row");
    row.set_widget_name(&input.name);
    row.set_child(Some(&input_row_content(input, now)));
    row
}

pub(super) fn input_row_content(input: &FlakeInput, now: SystemTime) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 1);
    copy.set_hexpand(true);
    copy.append(&label(&input.name, &["input-row-name"], 0.0));
    let source = label(&input.source, &["input-row-source"], 0.0);
    source.set_tooltip_text(Some(&input.source));
    copy.append(&source);
    content.append(&copy);
    if let Some(age) = lock_age(input, now) {
        let pill = label(&short_age(age), &["input-row-age"], 0.5);
        pill.set_valign(gtk::Align::Center);
        pill.set_tooltip_text(Some(&format!("Locked {} ago", duration_text(age))));
        if age >= STALE_AFTER {
            pill.add_css_class("tinted");
            pill.add_css_class(theme::tone(theme::YELLOW));
        }
        content.append(&pill);
    }
    content
}

/// Inputs listed first, in this order, regardless of the sort mode.
const PINNED: [&str; 2] = ["nixpkgs", "home-manager"];

/// Pinned inputs first; then name order, or oldest lock first with undated inputs last and ties
/// broken by name.
pub(super) fn compare_inputs(
    left: &FlakeInput,
    right: &FlakeInput,
    oldest_first: bool,
) -> Ordering {
    let pin_rank = |input: &FlakeInput| {
        PINNED
            .iter()
            .position(|name| *name == input.name)
            .unwrap_or(PINNED.len())
    };
    let by_pin = pin_rank(left).cmp(&pin_rank(right));
    if by_pin.is_ne() {
        return by_pin;
    }
    let by_name = || left.name.cmp(&right.name);
    if !oldest_first {
        return by_name();
    }
    match (left.last_modified, right.last_modified) {
        (Some(left_time), Some(right_time)) => left_time.cmp(&right_time).then_with(by_name),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => by_name(),
    }
}

pub(super) fn lock_age(input: &FlakeInput, now: SystemTime) -> Option<u64> {
    input
        .last_modified
        .map(|timestamp| seconds_since(now, timestamp))
}

pub(super) fn input_matches(input: &FlakeInput, query: &str) -> bool {
    query.is_empty()
        || input.name.to_lowercase().contains(query)
        || input.source.to_lowercase().contains(query)
        || input.revision.to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, last_modified: Option<i64>) -> FlakeInput {
        FlakeInput {
            name: name.to_owned(),
            source: String::new(),
            source_kind: String::new(),
            revision: String::new(),
            last_modified,
            follows: None,
            flake_ref: None,
            remote_url: None,
            remote_ref: None,
        }
    }

    #[test]
    fn pinned_inputs_lead_in_both_sort_orders() {
        // home-manager is the oldest and nixpkgs undated: pinning must beat both rules.
        let mut inputs = [
            input("agenix", Some(5)),
            input("home-manager", Some(1)),
            input("zen", None),
            input("nixpkgs", None),
            input("hardware", Some(2)),
        ];
        for (oldest_first, rest) in [
            (false, ["agenix", "hardware", "zen"]),
            (true, ["hardware", "agenix", "zen"]),
        ] {
            inputs.sort_by(|left, right| compare_inputs(left, right, oldest_first));
            let names = inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>();
            assert_eq!(names[..2], ["nixpkgs", "home-manager"]);
            assert_eq!(names[2..], rest);
        }
    }
}
