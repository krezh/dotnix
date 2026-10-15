//! The report's side column: closure sizes and the services a switch would touch.

use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::build::Target;
use crate::report::{Report, ServiceGroup, service_impact};
use crate::ui::theme;
use crate::ui::{
    FadeScroll, MorphLabel, Tween, animations_enabled, byte_unit, fade_in, label, signed_size,
};

use super::eyebrow;
use super::lists::size_classes;

/// Delay between successive service groups fading in.
const GROUP_STAGGER: Duration = Duration::from_millis(45);

/// A total that counts up from zero: its label and the text at a `0..=1` progress.
type Counter = (gtk::Label, Box<dyn Fn(f64) -> String>);

/// `entrance`: the report is appearing for the first time rather than being returned to, so
/// its totals count up and its service groups fade in.
pub(super) fn side_column(report: &Report, entrance: bool) -> gtk::Box {
    let side = gtk::Box::new(gtk::Orientation::Vertical, 14);
    side.append(&closure_card(report, entrance));
    if report.target == Target::NixOs {
        side.append(&services_card(report, entrance));
    }
    side
}

fn closure_card(report: &Report, entrance: bool) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("phase-panel");
    card.append(&eyebrow("CLOSURE"));
    let mut counters: Vec<Counter> = Vec::new();
    if let Some((old, new)) = report.sizes {
        let delta = label(
            &signed_size(new - old),
            &size_classes("report-closure-delta", new - old),
            0.0,
        );
        card.append(&delta);
        counters.push((
            delta,
            Box::new(move |progress| signed_size(scaled(new - old, progress))),
        ));
        card.append(&label(
            &size_range(old, new),
            &["report-closure-range"],
            0.0,
        ));
    }
    let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
    details.add_css_class("report-closure-details");
    if let Some((_, new, added, removed)) = report.paths {
        let paths = move |progress| {
            format!(
                "{}  +{} −{}",
                grouped(scaled(new, progress)),
                scaled(added, progress),
                scaled(removed, progress)
            )
        };
        let (row, value) = closure_row("Paths", &paths(1.0), true);
        details.append(&row);
        counters.push((value, Box::new(paths)));
    }
    let packages = report.changes.len() as i64;
    let changed = move |count: i64| move |progress| format!("{} changed", scaled(count, progress));
    let (row, value) = closure_row("Packages", &changed(packages)(1.0), false);
    details.append(&row);
    counters.push((value, Box::new(changed(packages))));
    match &report.config {
        Ok(config) => {
            let files = config.changes.len() as i64;
            let (row, value) = closure_row("Config files", &changed(files)(1.0), false);
            details.append(&row);
            counters.push((value, Box::new(changed(files))));
        }
        Err(_) => details.append(&closure_row("Config files", "unavailable", false).0),
    }
    card.append(&details);
    if entrance {
        count_up(&card, counters);
    }
    card
}

/// Counts every total up from zero once `card` is on screen; it starts from zero so the first
/// frame never shows the totals early.
fn count_up(card: &gtk::Box, counters: Vec<Counter>) {
    let progress = Tween::new(card, 0.0);
    progress.on_frame(move |progress| {
        for (label, text) in &counters {
            label.set_text(&text(progress));
        }
    });
    progress.jump(0.0);
    glib::idle_add_local_once(move || progress.set(1.0));
}

/// `value` at `progress` of the way up from zero.
fn scaled(value: i64, progress: f64) -> i64 {
    (value as f64 * progress).round() as i64
}

fn closure_row(name: &str, value: &str, monospace: bool) -> (gtk::Box, gtk::Label) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let name = label(name, &["report-closure-key"], 0.0);
    name.set_hexpand(true);
    row.append(&name);
    let value = label(value, &["report-closure-value"], 1.0);
    if monospace {
        value.add_css_class("monospace");
    }
    row.append(&value);
    (row, value)
}

/// `28.70 → 28.73 GiB`: both sizes in the larger one's unit.
fn size_range(old: i64, new: i64) -> String {
    let largest = old.unsigned_abs().max(new.unsigned_abs()) as f64;
    let (scale, unit) = byte_unit(largest, 2);
    format!(
        "{:.2} → {:.2} {unit}",
        old as f64 / scale,
        new as f64 / scale
    )
}

/// `3,238`: an integer with thousands separators.
fn grouped(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        text.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

fn services_card(report: &Report, entrance: bool) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("phase-panel");
    card.set_vexpand(true);
    card.append(&eyebrow("SERVICES"));
    let lines = match &report.operational_impact {
        Ok(lines) => lines,
        Err(error) => {
            let notice = wrapped_label(
                &format!("Impact analysis unavailable: {error}"),
                "report-services-notice",
            );
            notice.add_css_class("notice");
            notice.add_css_class(theme::tone(theme::YELLOW));
            notice.set_selectable(true);
            card.append(&notice);
            return card;
        }
    };
    let groups = service_impact(lines);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("report-service-groups");
    if groups.is_empty() {
        let message = if lines.is_empty() {
            "Dry activation reports no service changes"
        } else {
            "No units would change"
        };
        body.append(&wrapped_label(message, "report-services-empty"));
    }
    let mut sections = Vec::with_capacity(groups.len());
    for group in &groups {
        let section = service_group(group);
        body.append(&section);
        sections.push(section);
    }
    if entrance {
        stagger_in(sections);
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("see-through");
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&body));
    card.append(&FadeScroll::new(&scroller));
    if !lines.is_empty() {
        card.append(&revealed_text(
            lines,
            "Full dry-activate output ›",
            "Hide dry-activate output ‹",
        ));
    }
    card
}

/// Fades `sections` in one after another, each group's chips together.
fn stagger_in(sections: Vec<gtk::Box>) {
    if !animations_enabled() {
        return;
    }
    for (index, section) in sections.into_iter().enumerate() {
        section.set_opacity(0.0);
        // Later groups share the last slot so long lists don't trail in.
        let delay = GROUP_STAGGER * index.min(8) as u32;
        let section = section.downgrade();
        glib::timeout_add_local_once(delay, move || {
            let Some(section) = section.upgrade() else {
                return;
            };
            if section.is_mapped() {
                fade_in(&section);
            } else {
                section.set_opacity(1.0);
            }
        });
    }
}

fn wrapped_label(text: &str, class: &str) -> gtk::Label {
    let label = label(text, &[class], 0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label
}

fn service_group(group: &ServiceGroup) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
    section.add_css_class("report-service-group");
    section.append(&label(
        &format!("{} · {}", group.action.label(), group.units.len()),
        &["report-service-action", "toned", group.action.tone()],
        0.0,
    ));
    let chips = gtk::FlowBox::new();
    chips.add_css_class("report-service-units");
    chips.set_selection_mode(gtk::SelectionMode::None);
    chips.set_homogeneous(false);
    chips.set_column_spacing(6);
    chips.set_row_spacing(6);
    chips.set_max_children_per_line(32);
    chips.set_valign(gtk::Align::Start);
    for unit in &group.units {
        let chip = label(
            unit.display_name(),
            &["service-chip", "chip", "tinted", group.action.tone()],
            0.5,
        );
        chip.set_max_width_chars(24);
        if unit.user {
            chip.add_css_class("user");
            chip.set_tooltip_text(Some(&format!("{} (user unit)", unit.name)));
        } else {
            chip.set_tooltip_text(Some(&unit.name));
        }
        chips.insert(&chip, -1);
    }
    section.append(&chips);
    section
}

/// `lines` in a selectable block that `show` reveals above the toggle and `hide` collapses.
pub(super) fn revealed_text(lines: &[String], show: &'static str, hide: &'static str) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let text = wrapped_label(&lines.join("\n"), "report-raw-output");
    text.set_selectable(true);
    text.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_min_content_height(180);
    scroller.set_child(Some(&text));
    let faded = FadeScroll::new(&scroller);
    faded.add_css_class("report-raw-scroll");
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::SlideUp);
    revealer.set_transition_duration(180);
    revealer.set_child(Some(&faded));
    container.append(&revealer);
    let toggle_label = MorphLabel::new(show, &[], 0.0);
    let toggle = gtk::Button::new();
    toggle.set_child(Some(&toggle_label.root));
    toggle.add_css_class("report-link");
    toggle.add_css_class("text-link");
    toggle.set_halign(gtk::Align::Start);
    toggle.connect_clicked(move |_| {
        let reveal = !revealer.reveals_child();
        revealer.set_reveal_child(reveal);
        toggle_label.set_text(if reveal { hide } else { show });
    });
    container.append(&toggle);
    container
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(3238), "3,238");
        assert_eq!(grouped(-1_234_567), "-1,234,567");
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            size_range(gib * 2870 / 100, gib * 2873 / 100),
            "28.70 → 28.73 GiB"
        );
        assert_eq!(size_range(512, 2048), "0.50 → 2.00 KiB");
    }
}
