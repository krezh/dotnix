use std::cell::Cell;
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::app::state::UiState;

pub(crate) fn clear(root: &gtk::Box) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
}

/// Sizes the popup for the current screen. While Molt is minimized the size is kept for when it
/// is restored.
pub(crate) fn fit_window(
    window: &gtk::ApplicationWindow,
    state: &UiState,
    preferred: (i32, i32),
    minimum: (i32, i32),
) {
    match state.mini.get() {
        Some(mini) => mini.fit(preferred, minimum),
        None => set_window_size(window, preferred, minimum),
    }
}

/// Resizes the popup now, within the first monitor.
pub(crate) fn set_window_size(
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

/// Sizes the popup for every full screen (inputs, cleanup, build, report, release notes, errors).
/// One shared size keeps the layer surface from jumping when moving between them; only home is
/// smaller.
pub(crate) fn fit_screen(window: &gtk::ApplicationWindow, state: &UiState) {
    fit_window(window, state, (1240, 780), (1000, 640));
}

pub(crate) fn animations_enabled() -> bool {
    gtk::Settings::default()
        .is_none_or(|settings| settings.property::<bool>("gtk-enable-animations"))
}

#[derive(Clone)]
pub(crate) struct MorphLabel {
    pub(crate) root: gtk::Stack,
    labels: [gtk::Label; 2],
    visible: Rc<Cell<usize>>,
}

impl MorphLabel {
    pub(crate) fn new(text: &str, classes: &[&str], xalign: f32) -> Self {
        let root = gtk::Stack::new();
        root.set_transition_type(gtk::StackTransitionType::Crossfade);
        root.set_transition_duration(180);
        root.set_interpolate_size(true);
        root.set_hhomogeneous(false);
        root.set_vhomogeneous(false);
        root.set_halign(gtk::Align::Fill);
        for class in classes {
            root.add_css_class(class);
        }

        let first = label(text, &[], xalign);
        let second = label("", &[], xalign);
        root.add_child(&first);
        root.add_child(&second);
        root.set_visible_child(&first);

        Self {
            root,
            labels: [first, second],
            visible: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn set_text(&self, text: &str) {
        let current = self.visible.get();
        if self.labels[current].text().as_str() == text {
            return;
        }
        // Mid-crossfade, retarget the incoming label rather than reversing the fade.
        if self.root.is_transition_running() {
            self.labels[current].set_text(text);
            return;
        }
        let next = 1 - current;
        self.labels[next].set_text(text);
        self.root.set_visible_child(&self.labels[next]);
        self.visible.set(next);
    }

    pub(crate) fn set_text_immediate(&self, text: &str) {
        self.labels[self.visible.get()].set_text(text);
    }

    pub(crate) fn text(&self) -> glib::GString {
        self.labels[self.visible.get()].text()
    }

    pub(crate) fn add_css_class(&self, class: &str) {
        self.root.add_css_class(class);
    }

    pub(crate) fn remove_css_class(&self, class: &str) {
        self.root.remove_css_class(class);
    }

    pub(crate) fn set_selectable(&self, selectable: bool) {
        for label in &self.labels {
            label.set_selectable(selectable);
        }
    }

    pub(crate) fn set_ellipsize(&self, mode: gtk::pango::EllipsizeMode) {
        for label in &self.labels {
            label.set_ellipsize(mode);
        }
    }

    pub(crate) fn downgrade(&self) -> WeakMorphLabel {
        WeakMorphLabel {
            root: self.root.downgrade(),
            labels: [self.labels[0].downgrade(), self.labels[1].downgrade()],
            visible: Rc::clone(&self.visible),
        }
    }
}

#[derive(Clone)]
pub(crate) struct WeakMorphLabel {
    root: glib::WeakRef<gtk::Stack>,
    labels: [glib::WeakRef<gtk::Label>; 2],
    visible: Rc<Cell<usize>>,
}

impl WeakMorphLabel {
    pub(crate) fn upgrade(&self) -> Option<MorphLabel> {
        Some(MorphLabel {
            root: self.root.upgrade()?,
            labels: [self.labels[0].upgrade()?, self.labels[1].upgrade()?],
            visible: Rc::clone(&self.visible),
        })
    }
}

#[derive(Clone)]
pub(crate) struct MorphIcon {
    pub(crate) root: gtk::Stack,
    icons: [gtk::Image; 2],
    visible: Rc<Cell<usize>>,
}

impl MorphIcon {
    pub(crate) fn new(icon_name: &str, pixel_size: i32, classes: &[&str]) -> Self {
        let root = gtk::Stack::new();
        root.set_transition_type(gtk::StackTransitionType::Crossfade);
        root.set_transition_duration(180);
        root.set_interpolate_size(true);
        for class in classes {
            root.add_css_class(class);
        }
        let first = gtk::Image::from_icon_name(icon_name);
        let second = gtk::Image::new();
        for icon in [&first, &second] {
            icon.set_pixel_size(pixel_size);
        }
        root.add_child(&first);
        root.add_child(&second);
        root.set_visible_child(&first);
        Self {
            root,
            icons: [first, second],
            visible: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn set_icon_name(&self, icon_name: &str) {
        let current = self.visible.get();
        let next = 1 - current;
        self.icons[next].set_icon_name(Some(icon_name));
        self.root.set_visible_child(&self.icons[next]);
        self.visible.set(next);
    }

    pub(crate) fn add_css_class(&self, class: &str) {
        self.root.add_css_class(class);
    }

    pub(crate) fn remove_css_class(&self, class: &str) {
        self.root.remove_css_class(class);
    }
}

pub(crate) fn centered_icon(icon_name: &str, icon_size: i32, slot_size: i32) -> gtk::CenterBox {
    let slot = gtk::CenterBox::new();
    slot.set_size_request(slot_size, slot_size);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(icon_size);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    slot.set_center_widget(Some(&icon));
    slot
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
    button.add_css_class("surface-button");
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    content.set_halign(gtk::Align::Center);
    content.append(&centered_icon(icon_name, icon_size, icon_size));
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

pub(crate) fn action_close_button(
    window: &gtk::ApplicationWindow,
    state: Rc<UiState>,
    button_class: &str,
    label_class: &str,
) -> gtk::Button {
    let tooltip = if state.appearance.keybinds {
        "Close Molt (Esc)"
    } else {
        "Close Molt"
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

/// Shrinks Molt to its minimized bar while work runs.
pub(crate) fn minimize_button(state: &Rc<UiState>) -> gtk::Button {
    let button = icon_button(
        "Minimize",
        "report-back-button",
        "report-back-label",
        "window-minimize-symbolic",
        13,
        "Shrink Molt to a bar until you open it again",
    );
    let state = Rc::downgrade(state);
    button.connect_clicked(move |_| {
        if let Some(mini) = state.upgrade().as_ref().and_then(|state| state.mini.get()) {
            mini.minimize();
        }
    });
    button
}

/// Styles a text field. Its focus border shows only once the user clicks or types into it, not
/// when a screen hands it focus on entry, and goes again when focus leaves.
pub(crate) fn field(widget: &impl IsA<gtk::Widget>) {
    widget.add_css_class("field");
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    click.connect_pressed(|gesture, _, _, _| engage(gesture));
    widget.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(|controller, _, _, _| {
        engage(controller);
        glib::Propagation::Proceed
    });
    widget.add_controller(keys);
    let focus = gtk::EventControllerFocus::new();
    focus.connect_leave(|controller| {
        if let Some(widget) = controller.widget() {
            widget.remove_css_class("engaged");
        }
    });
    widget.add_controller(focus);
}

fn engage(controller: &impl IsA<gtk::EventController>) {
    if let Some(widget) = controller.widget() {
        widget.add_css_class("engaged");
    }
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

/// Fixed two-to-one split between a main panel and a side column; homogeneous columns keep the
/// split put however wide either side's content wants to be.
pub(crate) fn phase_grid(main: &impl IsA<gtk::Widget>, side: &impl IsA<gtk::Widget>) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_homogeneous(true);
    grid.set_column_spacing(16);
    grid.set_vexpand(true);
    main.set_hexpand(true);
    side.set_hexpand(true);
    grid.attach(main, 0, 0, 2, 1);
    grid.attach(side, 2, 0, 1, 1);
    grid
}
