//! The warnings nix printed during the build: a pill beside the report's subtitle that swaps the
//! change list for them.

use std::rc::Rc;

use gtk::prelude::*;

use crate::app::state::UiState;
use crate::build::ui::failure::copy_button;
use crate::build::ui::progress::warning_text;
use crate::ui::theme;
use crate::ui::{FadeScroll, centered_icon, label};

/// Adds the warnings pill to `subtitle_row` and returns the stack that holds `changes` and,
/// once the pill is clicked, the warnings in its place.
pub(super) fn warnings_switch(
    subtitle_row: &gtk::Box,
    changes: &gtk::Box,
    warnings: &[String],
    state: &Rc<UiState>,
) -> gtk::Stack {
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(220);
    stack.set_vexpand(true);
    stack.add_child(changes);
    let panel = WarningsPanel::new(warnings);
    stack.add_child(&panel.root);
    stack.set_visible_child(changes);

    let count = warnings.len();
    let pill = gtk::Button::new();
    pill.add_css_class("report-warnings-pill");
    pill.add_css_class(theme::tone(theme::YELLOW));
    pill.set_valign(gtk::Align::Center);
    pill.set_tooltip_text(Some("Show the warnings nix printed during the build"));
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    content.append(&centered_icon("dialog-warning-symbolic", 11, 12));
    content.append(&label(
        &if count == 1 {
            "1 warning".to_owned()
        } else {
            format!("{count} warnings")
        },
        &[],
        0.0,
    ));
    pill.set_child(Some(&content));
    subtitle_row.append(&pill);

    // The keyboard scrolls whichever list is showing.
    let changes_scroll = state.scroll.borrow().clone();
    let show = {
        let (stack, changes, pill, state) = (
            stack.clone(),
            changes.clone(),
            pill.clone(),
            Rc::clone(state),
        );
        let (warnings_root, warnings_scroll) = (panel.root.clone(), panel.scroll.vadjustment());
        Rc::new(move |open: bool| {
            if open {
                stack.set_visible_child(&warnings_root);
                pill.add_css_class("open");
                state.set_scroll_adjustment(warnings_scroll.clone());
            } else {
                stack.set_visible_child(&changes);
                pill.remove_css_class("open");
                state.scroll.replace(changes_scroll.clone());
            }
        })
    };
    let toggle = Rc::clone(&show);
    let (shown, warnings_root) = (stack.clone(), panel.root.upcast::<gtk::Widget>());
    pill.connect_clicked(move |_| toggle(shown.visible_child().as_ref() != Some(&warnings_root)));
    panel.back.connect_clicked(move |_| show(false));
    stack
}

/// The warnings in full, in place of the change list.
struct WarningsPanel {
    root: gtk::Box,
    back: gtk::Button,
    scroll: gtk::ScrolledWindow,
}

impl WarningsPanel {
    fn new(warnings: &[String]) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("phase-panel");
        root.add_css_class("build-list-panel");
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        toolbar.add_css_class("build-list-toolbar");
        let back = gtk::Button::new();
        back.add_css_class("build-link-button");
        back.set_tooltip_text(Some("Back to the changes"));
        back.set_valign(gtk::Align::Center);
        let back_content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        back_content.append(&centered_icon("go-previous-symbolic", 12, 12));
        back_content.append(&label("Changes", &[], 0.0));
        back.set_child(Some(&back_content));
        toolbar.append(&back);
        let eyebrow = label(
            &format!("WARNINGS · {}", warnings.len()),
            &["panel-eyebrow"],
            0.0,
        );
        eyebrow.set_hexpand(true);
        eyebrow.set_valign(gtk::Align::Center);
        toolbar.append(&eyebrow);
        let all = warnings.join("\n\n");
        let copy = copy_button("Copy all", move || all.clone());
        copy.set_valign(gtk::Align::Center);
        toolbar.append(&copy);
        root.append(&toolbar);
        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("build-list-rule");
        root.append(&rule);

        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        list.add_css_class("report-warnings-list");
        for warning in warnings {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            row.add_css_class("report-warning");
            row.add_css_class("notice");
            row.add_css_class(theme::tone(theme::YELLOW));
            let text = label(warning_text(warning), &["report-warning-text"], 0.0);
            text.set_hexpand(true);
            text.set_wrap(true);
            text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            text.set_ellipsize(gtk::pango::EllipsizeMode::None);
            text.set_selectable(true);
            row.append(&text);
            list.append(&row);
        }
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&list));
        root.append(&FadeScroll::new(&scroll));
        Self { root, back, scroll }
    }
}
