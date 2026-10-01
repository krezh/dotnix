use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;

use crate::report::{Change, ChangeStatus, Report};
use crate::state::{Operation, UiState};
use crate::ui::common::{
    action_close_button, animate_scroll_to, clear, fit_window, key_hints, label, signed_size, size,
    target_subtitle, wide_header,
};
use crate::ui::home::back_to_home_button;
use crate::ui::release_notes::show_changelog;
use crate::ui::switch::{
    SwitchView, connect_switch, current_hostname, requires_host_switch_confirmation,
    switch_animation,
};
use crate::ui::timeline::BuildTimeline;

#[derive(Clone)]
struct ReportNavigation {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
    report: Report,
}

pub(crate) fn show_report(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    report: Report,
) {
    state.clear_actions();
    if report.changes.is_empty() {
        fit_window(window, (640, 260), (360, 200));
    } else {
        fit_window(window, (880, 720), (360, 420));
    }
    clear(root);
    root.remove_css_class("home-root");
    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.set_complete();
    let right_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let header = wide_header(&timeline.root, &right_slot);
    header.append(&report_subtitle(&report));
    root.append(&header);
    let requires_confirmation = requires_host_switch_confirmation(
        report.target,
        &report.flake,
        current_hostname().as_deref(),
    );
    let switch = gtk::Button::new();
    switch.add_css_class("switch-button");
    if requires_confirmation {
        switch.add_css_class("switch-danger");
        switch.set_tooltip_text(Some(
            "This configuration targets another host; click twice to activate it",
        ));
    } else {
        switch.set_tooltip_text(Some("Activate this configuration"));
    }
    let switch_label = label("Switch", &["switch-button-label"], 0.5);
    switch.set_child(Some(&switch_label));
    let target = report.target.name();
    let switch_activity = switch_animation(target, &report.flake, &state.appearance.sans_font);
    let summary = gtk::Box::new(gtk::Orientation::Vertical, 0);
    summary.add_css_class("summary");
    let switch_confirmation = connect_switch(
        &SwitchView {
            button: switch.clone(),
            label: switch_label.clone(),
            activity: switch_activity.clone(),
            summary: summary.clone(),
        },
        report.clone(),
        Rc::clone(&state),
        requires_confirmation,
    );
    state.switch_confirmation.replace(Some(switch_confirmation));
    root.append(&switch_activity.root);

    let counts = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let mut section_jumps = HashMap::new();
    for status in ChangeStatus::ALL {
        let count = report
            .changes
            .iter()
            .filter(|change| change.status == status)
            .count();
        if count > 0 {
            let class = status.class();
            let jump = label(&format!("{count} {class}"), &["summary-count", class], 0.0);
            jump.add_css_class("summary-jump");
            jump.set_tooltip_text(Some(&format!("Jump to {class}")));
            jump.set_cursor_from_name(Some("pointer"));
            counts.append(&jump);
            section_jumps.insert(status, jump);
        }
    }
    if report.changes.is_empty() {
        summary.set_vexpand(true);
        summary.set_valign(gtk::Align::Center);
        summary.append(&label("No changes to apply", &["no-changes"], 0.5));
    } else {
        summary.append(&counts);
    }
    if let Some(metrics) = report_metrics(&report) {
        summary.append(&metrics);
    }
    root.append(&summary);

    if !report.changes.is_empty() {
        let navigation = ReportNavigation {
            window: window.clone(),
            root: root.clone(),
            state: Rc::clone(&state),
            report: report.clone(),
        };
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("updates-list");
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let adjustment = scroller.vadjustment();
        let scroll_animation = Rc::new(Cell::new(0));
        for status in ChangeStatus::ALL {
            let changes = report
                .changes
                .iter()
                .filter(|change| change.status == status)
                .collect::<Vec<_>>();
            if !changes.is_empty() {
                let section = change_section(&navigation, status, &changes);
                if let Some(jump) = section_jumps.remove(&status) {
                    let target = section.clone();
                    let jump_scroller = scroller.clone();
                    let adjustment = adjustment.clone();
                    let scroll_animation = Rc::clone(&scroll_animation);
                    let click = gtk::GestureClick::new();
                    click.connect_released(move |_, _, _, _| {
                        animate_scroll_to(
                            &jump_scroller,
                            &adjustment,
                            f64::from(target.allocation().y()),
                            Rc::clone(&scroll_animation),
                        );
                    });
                    jump.add_controller(click);
                }
                list.append(&section);
            }
        }
        scroller.set_child(Some(&list));
        state.set_scroll_adjustment(adjustment);
        root.append(&scroller);
    }
    let hints_widget = if state.appearance.keybinds {
        let mut hints = if report.changes.is_empty() {
            Vec::new()
        } else {
            vec![("↑", "Scroll Up"), ("↓", "Scroll Down")]
        };
        hints.extend([("S", "Switch"), ("←", "Back"), ("Esc", "Close")]);
        Some(key_hints(&hints))
    } else {
        None
    };
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    footer.add_css_class("report-footer");
    footer.set_valign(gtk::Align::End);

    let back = back_to_home_button(window, root, &state);
    let left_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    left_actions.set_valign(gtk::Align::Center);
    left_actions.append(&back);
    left_actions.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));
    footer.append(&left_actions);

    if let Some(hints) = hints_widget {
        hints.set_hexpand(true);
        hints.set_halign(gtk::Align::Center);
        hints.set_valign(gtk::Align::Center);
        footer.append(&hints);
    } else {
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        footer.append(&spacer);
    }

    switch.set_valign(gtk::Align::Center);
    footer.append(&switch);

    root.append(&footer);
}

fn report_subtitle(report: &Report) -> gtk::Box {
    target_subtitle(report.target, &report.flake)
}

fn report_metrics(report: &Report) -> Option<gtk::Box> {
    let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    metrics.add_css_class("metrics");
    metrics.append(&label("╰── ", &["metric-branch"], 0.0));
    let mut count = 0;
    if let Some((old, new, added, removed)) = report.paths {
        let text = if added == 0 && removed == 0 {
            format!("{new}")
        } else {
            format!("{old} -> {new} (+{added}, -{removed})")
        };
        metrics.append(&metric("PATHS:", &text, count));
        count += 1;
    }
    if let Some((old, new)) = report.sizes {
        let difference = new - old;
        if difference == 0 {
            metrics.append(&metric("SIZE:", &size(new), count));
            count += 1;
        } else {
            let text = format!("{} -> {}", size(old), size(new));
            metrics.append(&metric("SIZE:", &text, count));
            count += 1;
            let difference_metric = metric("DIFF:", &signed_size(difference), count);
            difference_metric.add_css_class(match difference.cmp(&0) {
                std::cmp::Ordering::Greater => "increase",
                std::cmp::Ordering::Less => "decrease",
                std::cmp::Ordering::Equal => "unchanged",
            });
            metrics.append(&difference_metric);
            count += 1;
        }
    }
    (count > 0).then_some(metrics)
}

fn metric(name: &str, value: &str, index: usize) -> gtk::Box {
    let metric = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    metric.add_css_class("metric");
    if index > 0 {
        metric.set_margin_start(18);
    }
    metric.append(&label(name, &["metric-label"], 0.0));
    metric.append(&label(value, &["metric-value"], 0.0));
    metric
}

fn change_section(
    navigation: &ReportNavigation,
    status: ChangeStatus,
    changes: &[&Change],
) -> gtk::Box {
    let class = status.class();
    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.add_css_class("section");
    let title = label(status.label(), &["section-title", class], 0.0);
    title.set_halign(gtk::Align::Start);
    section.append(&title);
    let grid = gtk::Grid::new();
    grid.add_css_class("changes-grid");
    grid.set_column_spacing(12);
    grid.set_row_spacing(4);
    let paired = status.is_paired();
    let headers: &[&str] = if paired {
        &["Name", "Old Version", "New Version", "Size"]
    } else {
        &["Name", "Version", "Size"]
    };
    for (column, heading) in headers.iter().enumerate() {
        let heading = label(heading, &["header-cell"], 0.0);
        heading.set_max_width_chars(if column == 0 { 44 } else { 28 });
        grid.attach(&heading, column as i32, 0, 1, 1);
    }
    for (index, change) in changes.iter().enumerate() {
        attach_change(navigation, &grid, change, paired, index as i32 + 1);
    }
    section.append(&grid);
    section
}

fn attach_change(
    navigation: &ReportNavigation,
    grid: &gtk::Grid,
    change: &Change,
    paired: bool,
    row: i32,
) {
    let class = change.status.class();
    if !has_changelog_versions(change) {
        let name = label(&change.name, &["cell", "name-cell"], 0.0);
        name.set_hexpand(true);
        name.set_max_width_chars(44);
        name.set_tooltip_text(Some(&change.name));
        grid.attach(&name, 0, row, 1, 1);
    } else {
        let name = gtk::Button::new();
        name.add_css_class("package-button");
        name.set_hexpand(true);
        name.set_halign(gtk::Align::Fill);
        name.set_tooltip_text(Some(&change.name));
        let name_text = label(&change.name, &["name-cell"], 0.0);
        name_text.set_hexpand(true);
        name_text.set_max_width_chars(44);
        name.set_child(Some(&name_text));
        let navigation = navigation.clone();
        let changelog_change = change.clone();
        name.connect_clicked(move |_| {
            if navigation.state.operation.get() == Operation::Idle {
                show_changelog(
                    &navigation.window,
                    &navigation.root,
                    &changelog_change,
                    Rc::clone(&navigation.state),
                    navigation.report.clone(),
                );
            }
        });
        grid.attach(&name, 0, row, 1, 1);
    }
    if paired {
        grid.attach(
            &version_label(
                &change.old,
                &["cell", "version-cell", "old-version-cell"],
                true,
            ),
            1,
            row,
            1,
            1,
        );
        grid.attach(
            &version_label(
                &change.new,
                &["cell", "version-cell", "new-version-cell"],
                true,
            ),
            2,
            row,
            1,
            1,
        );
        grid.attach(&change_size_label(change, class), 3, row, 1, 1);
    } else {
        let version = if change.new.is_empty() {
            &change.old
        } else {
            &change.new
        };
        grid.attach(
            &version_label(version, &["cell", "version-cell"], false),
            1,
            row,
            1,
            1,
        );
        grid.attach(&change_size_label(change, class), 2, row, 1, 1);
    }
}

fn version_label(text: &str, classes: &[&str], paired: bool) -> gtk::Label {
    let value = label(text, classes, 0.0);
    value.set_ellipsize(gtk::pango::EllipsizeMode::End);
    value.set_single_line_mode(true);
    value.set_max_width_chars(if paired { 28 } else { 36 });
    value.set_hexpand(true);
    value.set_tooltip_text(Some(text));
    value
}

fn change_size_label(change: &Change, class: &str) -> gtk::Label {
    let direction = match change.size.cmp(&0) {
        std::cmp::Ordering::Greater => "increase",
        std::cmp::Ordering::Less => "decrease",
        std::cmp::Ordering::Equal => "unchanged",
    };
    let value = label(
        &signed_size(change.size),
        &["cell", "size-cell", class, direction],
        1.0,
    );
    value.set_max_width_chars(12);
    value
}

pub(crate) fn has_changelog_versions(change: &Change) -> bool {
    !change.new.is_empty() && change.old != "..." && change.new != "..."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enables_changelogs_only_for_known_target_versions() {
        let change = |old: &str, new: &str| Change {
            status: ChangeStatus::Upgraded,
            name: "demo".to_owned(),
            old: old.to_owned(),
            new: new.to_owned(),
            size: 0,
        };
        assert!(has_changelog_versions(&change("1.0", "2.0")));
        assert!(has_changelog_versions(&change("", "2.0")));
        assert!(!has_changelog_versions(&change("1.0", "...")));
        assert!(!has_changelog_versions(&change("...", "2.0")));
        assert!(!has_changelog_versions(&change("1.0", "")));
    }
}
