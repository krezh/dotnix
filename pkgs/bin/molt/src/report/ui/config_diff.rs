use std::rc::Rc;

use gtk::prelude::*;

use crate::app::state::UiState;
use crate::report::Report;
use crate::report::config_diff::{ConfigChange, ConfigContent, DiffLine, DiffLineKind};
use crate::report::ui::{ReportPosition, show_report};
use crate::ui::theme;
use crate::ui::{
    FadeScroll, Slide, action_close_button, back_button, fit_screen, key_hints, label, slide_screen,
};

/// One generated file's changes against the active generation, as a unified diff.
pub(crate) fn show_config_diff(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    change: &ConfigChange,
    state: Rc<UiState>,
    report: Report,
    position: ReportPosition,
) {
    state.clear_actions();
    slide_screen(root, Slide::Forward);
    fit_screen(window, &state);
    root.append(&header(change, &report));

    let back_tooltip = if state.appearance.keybinds {
        "Back to report (←)"
    } else {
        "Back to report"
    };
    let back = back_button("Back to report", back_tooltip);
    let back_window = window.clone();
    let back_root = root.clone();
    let back_state = Rc::clone(&state);
    back.connect_clicked(move |_| {
        show_report(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            report.clone(),
            Some(position.clone()),
        );
    });
    state.set_back_button(&back);

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("phase-panel");
    panel.set_vexpand(true);
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    bar.add_css_class("changelog-bar");
    let eyebrow = label("DIFF", &["panel-eyebrow"], 0.0);
    eyebrow.set_hexpand(true);
    bar.append(&eyebrow);
    bar.append(&label(
        "store hashes shown as …",
        &["changelog-source"],
        1.0,
    ));
    panel.append(&bar);
    let content: gtk::Widget = match &change.content {
        ConfigContent::Text { lines, .. } => diff_view(lines).upcast(),
        ConfigContent::Binary => {
            let notice = label(
                "Binary or oversized file: contents are not shown",
                &["report-empty"],
                0.0,
            );
            notice.set_valign(gtk::Align::Start);
            notice.set_halign(gtk::Align::Start);
            notice.upcast()
        }
    };
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("see-through");
    scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&content));
    state.set_scroll_adjustment(scroller.vadjustment());
    panel.append(&FadeScroll::new(&scroller));
    root.append(&panel);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("report-footer");
    footer.add_css_class("report-actions");
    footer.append(&back);
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));
    if state.appearance.keybinds {
        let hints = key_hints(&[
            ("↑", "Scroll Up"),
            ("↓", "Scroll Down"),
            ("←", "Back"),
            ("Esc", "Close"),
        ]);
        hints.set_hexpand(true);
        hints.set_halign(gtk::Align::Center);
        hints.set_valign(gtk::Align::Center);
        footer.append(&hints);
    }
    root.append(&footer);
    back.grab_focus();
}

/// `config` over `Changed · ~/.config/ghostty/config · NixOS #thor`.
fn header(change: &ConfigChange, report: &Report) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 2);
    root.add_css_class("phase-header");
    let name = change.path.rsplit('/').next().unwrap_or(&change.path);
    root.append(&label(name, &["phase-title"], 0.0));
    let subtitle = label(
        &format!(
            "{} · {} · {} #{}",
            change.status.label(),
            change.path,
            report.target.name(),
            report.flake
        ),
        &["phase-subtitle"],
        0.0,
    );
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    subtitle.set_tooltip_text(Some(&change.path));
    root.append(&subtitle);
    root
}

/// A read-only diff: removals and additions in red and green on a full-width tint, hunk headers
/// in the version colour, context muted.
fn diff_view(lines: &[DiffLine]) -> gtk::TextView {
    let buffer = gtk::TextBuffer::new(None);
    let tags = buffer.tag_table();
    for (name, foreground, background) in [
        ("hunk", theme::VERSION, None),
        ("context", theme::SUBTEXT1, None),
        ("removed", theme::RED, Some(theme::RED.with_alpha(0.1))),
        ("added", theme::GREEN, Some(theme::GREEN.with_alpha(0.1))),
    ] {
        let mut tag = gtk::TextTag::builder()
            .name(name)
            .foreground_rgba(&foreground.with_alpha(1.0));
        if let Some(background) = background {
            tag = tag.paragraph_background_rgba(&background);
        }
        tags.add(&tag.build());
    }
    for (index, line) in lines.iter().enumerate() {
        let (tag, sign) = match line.kind {
            DiffLineKind::Hunk => ("hunk", ""),
            DiffLineKind::Context => ("context", " "),
            DiffLineKind::Removed => ("removed", "-"),
            DiffLineKind::Added => ("added", "+"),
        };
        let newline = if index + 1 < lines.len() { "\n" } else { "" };
        buffer.insert_with_tags_by_name(
            &mut buffer.end_iter(),
            &format!("{sign}{}{newline}", line.text),
            &[tag],
        );
    }
    let view = gtk::TextView::with_buffer(&buffer);
    view.add_css_class("config-diff-text");
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::None);
    view
}
