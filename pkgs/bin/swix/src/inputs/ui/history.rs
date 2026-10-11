//! The revision timeline: fetching remote commits and laying them out around the locked one.

use std::cmp::Ordering;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use gtk::glib;
use gtk::prelude::*;

use crate::inputs::{self, RevisionRecord};
use crate::ui::theme;
use crate::ui::{clear, duration_text, label, seconds_since};

use super::operations::apply_history_revision;
use super::{HISTORY_TIMELINE, InputsView, format_date, short_revision};

/// Where a remote commit sits relative to the locked one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TimelineKind {
    Newer,
    Current,
    Older,
    /// The locked commit is not in the fetched history, so order relative to it is unknown.
    Unplaced,
}

pub(super) fn start_history(view: Rc<InputsView>) {
    let Some(input) = view.current_input() else {
        return;
    };
    view.state.cancel_view();
    let cancellation = Arc::new(AtomicBool::new(false));
    view.state.set_view_cancellation(&cancellation);
    let generation = view.history_generation.get().wrapping_add(1);
    view.history_generation.set(generation);
    view.revisions.borrow_mut().clear();
    view.show_history_message("Fetching remote commits…");
    view.history_status.set_text("Loading…");
    view.history_status.remove_css_class("warning");
    view.lock_upstream.set_text("");
    view.lock_upstream.remove_css_class("newer");
    view.history_spinner_revealer.set_reveal_child(true);
    view.status.remove_css_class("error");
    view.status_revealer.set_reveal_child(false);
    view.status.root.set_tooltip_text(None);
    let directory = view.directory.clone();
    let input_name = input.name;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = inputs::history(&directory, &input_name, &cancellation);
        let _ = sender.send(result);
    });
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(records)) => {
                if view.history_generation.get() == generation {
                    view.revisions.replace(records);
                    render_history(&view);
                    view.history_spinner_revealer.set_reveal_child(false);
                }
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                if view.history_generation.get() == generation {
                    view.show_error(&error);
                    view.history_spinner_revealer.set_reveal_child(false);
                    view.history_status.set_text("Unavailable");
                    view.show_history_message("Remote revisions unavailable");
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if view.history_generation.get() == generation {
                    view.show_error("Remote revision fetch stopped without a result");
                    view.history_spinner_revealer.set_reveal_child(false);
                    view.history_status.set_text("Unavailable");
                    view.show_history_message("Remote revisions unavailable");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

/// Lays the fetched commits out as one timeline: newer commits, the locked one, then older ones.
/// The timeline page is hidden behind the loading message here, so it is rebuilt off screen and
/// then crossfaded in.
fn render_history(view: &Rc<InputsView>) {
    clear(&view.history_list);
    view.history_scroll.vadjustment().set_value(0.0);
    view.history_status.remove_css_class("warning");
    view.lock_upstream.remove_css_class("newer");
    let current_revision = view.current_input().map(|input| input.revision);
    let records = view.revisions.borrow();
    let current_position = current_revision
        .as_deref()
        .and_then(|revision| history_position(&records, revision));
    let now = SystemTime::now();

    if let Some((current_index, newer, older)) = current_position {
        for (position, record) in records.iter().enumerate() {
            let kind = match position.cmp(&current_index) {
                Ordering::Less => TimelineKind::Newer,
                Ordering::Equal => TimelineKind::Current,
                Ordering::Greater => TimelineKind::Older,
            };
            append_timeline_row(view, record, position, kind, now);
        }
        view.history_status
            .set_text(&format!("{newer} newer · {older} older"));
        if newer > 0 {
            view.lock_upstream.add_css_class("newer");
            view.lock_upstream.set_text(&format!(
                "· {newer} newer commit{} upstream",
                if newer == 1 { "" } else { "s" }
            ));
        } else {
            view.lock_upstream.set_text("· up to date");
        }
    } else {
        if !records.is_empty() {
            let notice = label(
                &format!(
                    "The locked commit is not among the latest {} remote commits",
                    records.len()
                ),
                &["inputs-history-notice"],
                0.0,
            );
            view.history_list.append(&notice);
        }
        for (position, record) in records.iter().enumerate() {
            append_timeline_row(view, record, position, TimelineKind::Unplaced, now);
        }
        view.history_status.add_css_class("warning");
        view.history_status.set_text("Locked commit not in recent history");
        view.lock_upstream.set_text("· not in recent history");
    }
    if records.is_empty() {
        view.show_history_message("No other remote revisions found");
    } else {
        view.history_stack
            .set_visible_child_name(HISTORY_TIMELINE);
    }
}

fn append_timeline_row(
    view: &Rc<InputsView>,
    record: &RevisionRecord,
    position: usize,
    kind: TimelineKind,
    now: SystemTime,
) {
    let content = timeline_content(record, kind, now);
    if kind == TimelineKind::Current {
        content.add_css_class("inputs-history-row");
        content.add_css_class("current");
        view.history_list.append(&content);
        return;
    }
    let row = gtk::Button::new();
    row.add_css_class("inputs-history-row");
    row.set_tooltip_text(Some(match kind {
        TimelineKind::Newer => "Move this input forward to this commit",
        TimelineKind::Older => "Roll this input back to this commit",
        TimelineKind::Unplaced | TimelineKind::Current => "Lock this input to this commit",
    }));
    row.set_child(Some(&content));
    let weak = Rc::downgrade(view);
    let window = view.window.clone();
    row.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade() {
            apply_history_revision(&window, view, position);
        }
    });
    view.history_list.append(&row);
}

fn history_position(records: &[RevisionRecord], current: &str) -> Option<(usize, usize, usize)> {
    let index = records
        .iter()
        .position(|record| record.revision == current)?;
    Some((index, index, records.len().saturating_sub(index + 1)))
}

fn timeline_content(record: &RevisionRecord, kind: TimelineKind, now: SystemTime) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let kind_class = match kind {
        TimelineKind::Newer => "newer",
        TimelineKind::Current => "current",
        TimelineKind::Older => "older",
        TimelineKind::Unplaced => "unplaced",
    };
    // The rail spans the full row height so consecutive rows join into one line; the row's
    // vertical padding therefore lives on the text, not on the row. The dot marks this commit.
    let rail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    rail.add_css_class("inputs-timeline-rail");
    let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dot.add_css_class("inputs-timeline-dot");
    dot.add_css_class(kind_class);
    dot.set_halign(gtk::Align::Center);
    dot.set_valign(gtk::Align::Start);
    rail.append(&dot);
    content.append(&rail);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    copy.set_margin_top(8);
    copy.set_margin_bottom(8);
    copy.append(&label(
        &short_revision(&record.revision),
        &["inputs-history-revision"],
        0.0,
    ));
    let summary = label(&record.summary, &["inputs-history-summary"], 0.0);
    summary.set_tooltip_text(Some(&record.summary));
    copy.append(&summary);
    content.append(&copy);

    let date = label(&relative_date(&record.date, now), &["inputs-history-date"], 1.0);
    date.set_valign(gtk::Align::Start);
    date.set_margin_top(9);
    date.set_tooltip_text(Some(&history_date(&record.date)));
    content.append(&date);

    let action = label(
        match kind {
            TimelineKind::Newer => "Update here",
            TimelineKind::Current => "Locked",
            TimelineKind::Older => "Roll back",
            TimelineKind::Unplaced => "Lock here",
        },
        &["inputs-row-action", kind_class],
        0.5,
    );
    // The newer and the locked commit's actions are tinted; rolling back stays neutral.
    match kind {
        TimelineKind::Newer => action.add_css_class(theme::tone(theme::BLUE)),
        TimelineKind::Current => action.add_css_class(theme::tone(theme::LAVENDER)),
        TimelineKind::Older | TimelineKind::Unplaced => {}
    }
    if matches!(kind, TimelineKind::Newer | TimelineKind::Current) {
        action.add_css_class("tinted");
    }
    action.set_valign(gtk::Align::Center);
    action.set_width_chars(11);
    content.append(&action);
    content
}

/// `7 days ago` from the ISO 8601 commit date Git reports; anything unparseable is shown as-is.
fn relative_date(value: &str, now: SystemTime) -> String {
    glib::DateTime::from_iso8601(value, None)
        .map(|date| format!("{} ago", duration_text(seconds_since(now, date.to_unix()))))
        .unwrap_or_else(|_| history_date(value))
}

fn history_date(value: &str) -> String {
    if let Ok(timestamp) = value.parse::<i64>() {
        return format_date(timestamp).unwrap_or_else(|| value.to_owned());
    }
    value.get(..10).unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(revision: &str) -> RevisionRecord {
        RevisionRecord {
            revision: revision.to_owned(),
            flake_ref: format!("github:owner/repo/{revision}"),
            date: String::new(),
            summary: String::new(),
        }
    }

    #[test]
    fn positions_remote_revisions_relative_to_the_current_lock() {
        let records = ["newer", "current", "older-1", "older-2"].map(record);
        assert_eq!(history_position(&records, "current"), Some((1, 1, 2)));
        assert_eq!(history_position(&records, "missing"), None);
    }
}
