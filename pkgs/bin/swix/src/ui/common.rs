use std::cell::Cell;
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::build::Target;
use crate::state::UiState;

pub(crate) fn clear(root: &gtk::Box) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
}

pub(crate) fn fit_window(
    window: &gtk::ApplicationWindow,
    preferred: (i32, i32),
    minimum: (i32, i32),
) {
    let available = gtk::prelude::WidgetExt::display(window)
        .monitors()
        .item(0)
        .and_downcast::<gdk::Monitor>()
        .map(|monitor| monitor.geometry())
        .map(|geometry| (geometry.width() - 32, geometry.height() - 32))
        .unwrap_or(preferred);
    window.set_default_size(preferred.0.min(available.0), preferred.1.min(available.1));
    window.set_size_request(minimum.0.min(available.0), minimum.1.min(available.1));
}

pub(crate) fn animations_enabled() -> bool {
    gtk::Settings::default()
        .is_none_or(|settings| settings.property::<bool>("gtk-enable-animations"))
}

fn icon_button(
    text: &str,
    button_class: &str,
    label_class: &str,
    icon_name: &str,
    icon_size: i32,
    tooltip: &str,
) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class(button_class);
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(icon_size);
    content.append(&icon);
    content.append(&label(text, &[label_class], 0.0));
    button.set_child(Some(&content));
    button
}

pub(crate) fn back_button(text: &str, tooltip: &str) -> gtk::Button {
    icon_button(
        text,
        "report-back-button",
        "report-back-label",
        "go-previous-symbolic",
        14,
        tooltip,
    )
}

pub(crate) fn simple_header(text: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let heading = title(text);
    heading.set_hexpand(true);
    header.append(&heading);
    header
}

pub(crate) fn wide_header(
    center: &impl IsA<gtk::Widget>,
    trailing: &impl IsA<gtk::Widget>,
) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    header.add_css_class("report-header");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let title_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    title_slot.set_width_request(170);
    title_slot.set_hexpand(true);
    title_slot.append(&title("Swix"));
    row.append(&title_slot);
    row.append(center);
    trailing.set_width_request(170);
    trailing.set_hexpand(true);
    row.append(trailing);
    header.append(&row);
    header
}

pub(crate) fn navigation_footer(
    back: &gtk::Button,
    keybinds: bool,
    hints: &[(&str, &str)],
) -> gtk::Box {
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    footer.add_css_class("report-footer");
    footer.append(back);
    if keybinds {
        let hints = key_hints(hints);
        hints.set_hexpand(true);
        hints.set_halign(gtk::Align::End);
        hints.set_valign(gtk::Align::Center);
        footer.append(&hints);
    }
    footer
}

pub(crate) fn action_close_button(
    window: &gtk::ApplicationWindow,
    state: Rc<UiState>,
    button_class: &str,
    label_class: &str,
) -> gtk::Button {
    let tooltip = if state.appearance.keybinds {
        "Close Swix (Esc)"
    } else {
        "Close Swix"
    };
    let button = icon_button(
        "Close",
        button_class,
        label_class,
        "window-close-symbolic",
        13,
        tooltip,
    );
    let window = window.clone();
    button.connect_clicked(move |_| {
        if state.request_close() {
            window.close();
        }
    });
    button
}

pub(crate) fn title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("title");
    label.set_xalign(0.0);
    label
}

pub(crate) fn label(text: &str, classes: &[&str], xalign: f32) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(xalign);
    label.set_halign(gtk::Align::Fill);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    for class in classes {
        label.add_css_class(class);
    }
    label
}

pub(crate) fn key_hints(hints: &[(&str, &str)]) -> gtk::FlowBox {
    let container = gtk::FlowBox::new();
    container.add_css_class("key-hints");
    container.set_halign(gtk::Align::Center);
    container.set_column_spacing(14);
    container.set_row_spacing(8);
    container.set_selection_mode(gtk::SelectionMode::None);
    for (key, description) in hints {
        let hint = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        hint.add_css_class("key-hint");
        hint.set_valign(gtk::Align::Center);
        let keycap = label(key, &["keycap"], 0.5);
        keycap.set_yalign(0.5);
        keycap.set_valign(gtk::Align::Center);
        hint.append(&keycap);
        hint.append(&label(description, &["key-hint-label"], 0.0));
        container.insert(&hint, -1);
    }
    container
}
pub(crate) fn animate_scroll_to(
    scroller: &gtk::ScrolledWindow,
    adjustment: &gtk::Adjustment,
    target: f64,
    animation: Rc<Cell<u64>>,
) {
    let lower = adjustment.lower();
    let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
    let start = adjustment.value();
    let target = target.clamp(lower, upper);
    let distance = (target - start).abs();
    let duration = 0.28 + (distance / 6000.0).min(1.0) * 0.18;
    let animation_id = animation.get().wrapping_add(1);
    animation.set(animation_id);
    let adjustment = adjustment.clone();
    let started = Cell::new(None);
    scroller.add_tick_callback(move |_, frame_clock| {
        if animation.get() != animation_id {
            return glib::ControlFlow::Break;
        }
        let now = frame_clock.frame_time();
        let started = started.get().unwrap_or_else(|| {
            started.set(Some(now));
            now
        });
        let progress = ((now - started) as f64 / 1_000_000.0 / duration).min(1.0);
        let eased = 0.5 - 0.5 * (std::f64::consts::PI * progress).cos();
        adjustment.set_value(start + (target - start) * eased);
        if progress >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
pub(crate) fn target_subtitle(target: Target, flake: &str) -> gtk::Box {
    let subtitle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    subtitle.add_css_class("report-subtitle");
    subtitle.append(&label("╰── ", &["metric-branch", "subtitle-branch"], 0.0));
    let target = match target {
        Target::HomeManager => "Home Manager ",
        Target::NixOs => "NixOS ",
    };
    subtitle.append(&label(target, &["subtitle", "subtitle-kind"], 0.0));
    subtitle.append(&label(
        &format!("#{flake}"),
        &["subtitle", "flake-tag"],
        0.0,
    ));
    subtitle.append(&label(" flake", &["subtitle", "subtitle-suffix"], 0.0));
    subtitle
}
pub(crate) fn signed_size(bytes: i64) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let value = bytes.unsigned_abs() as f64;
    let (value, unit) = if value >= 1024.0 * 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0 * 1024.0), "GiB")
    } else if value >= 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0), "MiB")
    } else if value >= 1024.0 {
        (value / 1024.0, "KiB")
    } else {
        (value, "B")
    };
    format!("{sign}{value:.1} {unit}")
}
pub(crate) fn size(bytes: i64) -> String {
    signed_size(bytes).trim_start_matches('+').to_owned()
}
