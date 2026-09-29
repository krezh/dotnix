use std::rc::Rc;

use gtk::prelude::*;

use crate::state::UiState;
use crate::ui::chooser::back_to_chooser_button;
use crate::ui::common::{clear, fit_window, navigation_footer, simple_header};

pub(crate) fn show_error(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    message: &str,
) {
    state.clear_actions();
    fit_window(window, (1040, 760), (360, 420));
    clear(root);
    root.append(&simple_header("Update failed"));
    let back = back_to_chooser_button(window, root, &state);
    let error = gtk::Label::new(Some(message));
    error.add_css_class("error");
    error.add_css_class("build-error");
    error.set_selectable(true);
    error.set_wrap(true);
    error.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    error.set_xalign(0.0);
    error.set_yalign(0.0);
    let scroll = gtk::ScrolledWindow::new();
    scroll.add_css_class("build-error-scroll");
    scroll.set_vexpand(true);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&error));
    state.set_scroll_adjustment(scroll.vadjustment());
    root.append(&scroll);
    root.append(&navigation_footer(
        &back,
        state.appearance.keybinds,
        &[("←", "Back"), ("Esc", "Close")],
    ));
    back.grab_focus();
}
