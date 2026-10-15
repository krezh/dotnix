use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;
use nix_changelog::{Changelog, Release, Session};
use molt::command::CANCELLED;

use crate::report::{Change, Report};
use crate::app::state::UiState;
use crate::report::ui::changelog::render as render_changelog;
use crate::ui::{
    FadeScroll, MorphLabel, Slide, Tween, action_close_button, back_button, fit_screen, glide,
    key_hints, label, slide_screen,
};
use crate::report::ui::{ReportPosition, show_report};
use split_layout::SplitLayout;

const CHANGELOG_TIMEOUT: Duration = Duration::from_secs(2 * 60);
/// How far above the viewport top a release may start and still count as the one being read.
const RELEASE_LEAD: f64 = 24.0;
/// The gap between the notes and the jump list: `phase_grid`'s column spacing.
const COLUMN_GAP: i32 = 16;

pub(crate) fn show_changelog(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    change: &Change,
    state: Rc<UiState>,
    report: Report,
    position: ReportPosition,
) {
    state.clear_actions();
    slide_screen(root, Slide::Forward);
    fit_screen(window, &state);
    let flake_dir = report.flake_dir.clone();
    let name = change.name.clone();
    let version_spec = if change.old.is_empty() {
        change.new.clone()
    } else {
        format!("{}..{}", change.old, change.new)
    };
    let cache_key = format!("{}\0{name}\0{version_spec}", flake_dir.display());
    let restore_scroll = state
        .changelog_scroll_positions
        .borrow()
        .get(&cache_key)
        .copied();

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
    let back_cache_key = cache_key.clone();
    back.connect_clicked(move |_| {
        let adjustment = back_state.scroll.borrow().clone();
        if let Some(adjustment) = adjustment {
            back_state
                .changelog_scroll_positions
                .borrow_mut()
                .insert(back_cache_key.clone(), adjustment.value());
        }
        show_report(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            report.clone(),
            Some(position.clone()),
        );
    });
    state.set_back_button(&back);

    let view = NotesView::new();
    state.set_scroll_adjustment(view.scroller.vadjustment());
    // Cached notes go in before the view is on screen, so they show without a loading fade.
    let cached = state.changelog_cache.borrow().get(&cache_key).cloned();
    if let Some(changelog) = &cached {
        view.show(changelog);
    }
    root.append(&view.body);

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

    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    back.grab_focus();

    if cached.is_some() {
        restore_scroll_position(&view.scroller, restore_scroll);
        return;
    }
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let started = Instant::now();
        let result = nix_changelog::releases(
            &Session {
                flake_dir: &flake_dir,
                cancelled: &|| {
                    worker_cancellation.load(Ordering::Relaxed)
                        || started.elapsed() > CHANGELOG_TIMEOUT
                },
                progress: &|_| {},
            },
            &name,
            &version_spec,
        )
        .map_err(|error| {
            if worker_cancellation.load(Ordering::Relaxed) {
                CANCELLED.to_owned()
            } else if started.elapsed() > CHANGELOG_TIMEOUT {
                format!(
                    "nix-changelog timed out after {} seconds",
                    CHANGELOG_TIMEOUT.as_secs()
                )
            } else {
                format!("{error:#}")
            }
        });
        let _ = sender.send(result);
    });
    let cache_state = Rc::clone(&state);
    let callback_cancellation = Arc::clone(&cancellation);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if callback_cancellation.load(Ordering::Relaxed) {
            return glib::ControlFlow::Break;
        }
        match receiver.try_recv() {
            Ok(Ok(changelog)) => {
                view.show(&changelog);
                restore_scroll_position(&view.scroller, restore_scroll);
                cache_state
                    .changelog_cache
                    .borrow_mut()
                    .insert(cache_key.clone(), changelog);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                view.show_error(&error);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                view.show_error("changelog worker stopped without a result");
                glib::ControlFlow::Break
            }
        }
    });
}

/// `firefox release notes` over `143.0.1 → 144.0 · NixOS #thor`, without the build timeline.
fn header(change: &Change, report: &Report) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 2);
    root.add_css_class("phase-header");
    root.append(&label(
        &format!("{} release notes", change.name),
        &["phase-title"],
        0.0,
    ));
    let subtitle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let versions = label("", &["phase-subtitle", "changelog-versions"], 0.0);
    versions.set_markup(&version_range(change));
    subtitle.append(&versions);
    subtitle.append(&label(
        &format!(" · {} #{}", report.target.name(), report.flake),
        &["phase-subtitle"],
        0.0,
    ));
    root.append(&subtitle);
    root
}

/// Pango markup for `1.0 → 2.0` with the target version in the version colour, or just the new
/// version for an added package.
fn version_range(change: &Change) -> String {
    let version = |text: &str| {
        format!(
            "<span foreground=\"{}\">{}</span>",
            crate::ui::theme::VERSION,
            glib::markup_escape_text(text)
        )
    };
    match (change.old.is_empty(), change.new.is_empty()) {
        (false, false) => format!(
            "{} → {}",
            glib::markup_escape_text(&change.old),
            version(&change.new)
        ),
        (true, _) => version(&change.new),
        (false, true) => glib::markup_escape_text(&change.old).to_string(),
    }
}

/// The main notes panel, joined by a release jump list once a multi-release changelog arrives.
struct NotesView {
    body: gtk::Box,
    /// Slides the jump list in beside the panel, `0..=1`.
    split: Tween,
    eyebrow: MorphLabel,
    source: MorphLabel,
    /// Crossfades from the loading notice to `notes`.
    pages: gtk::Stack,
    notes: FadeScroll,
    content: gtk::Box,
    scroller: gtk::ScrolledWindow,
}

impl NotesView {
    fn new() -> Self {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.add_css_class("phase-panel");
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        bar.add_css_class("changelog-bar");
        let eyebrow = MorphLabel::new("RELEASE NOTES", &["panel-eyebrow"], 0.0);
        eyebrow.root.set_hexpand(true);
        bar.append(&eyebrow.root);
        let source = MorphLabel::new("", &["changelog-source"], 1.0);
        source
            .root
            .set_tooltip_text(Some("Package and version nix-changelog resolved"));
        bar.append(&source.root);
        panel.append(&bar);

        let loading = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        loading.add_css_class("changelog-content");
        loading.set_valign(gtk::Align::Start);
        let spinner = gtk::Spinner::new();
        spinner.set_size_request(16, 16);
        spinner.start();
        loading.append(&spinner);
        loading.append(&label(
            "Looking up release notes…",
            &["changelog-loading-label"],
            0.0,
        ));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.add_css_class("changelog-content");
        content.set_vexpand(true);
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("see-through");
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&content));
        let notes = FadeScroll::new(&scroller);

        let pages = gtk::Stack::new();
        pages.set_transition_type(gtk::StackTransitionType::Crossfade);
        pages.set_transition_duration(220);
        pages.set_vexpand(true);
        pages.add_child(&loading);
        pages.add_child(&notes);
        panel.append(&pages);

        let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        body.set_vexpand(true);
        body.set_overflow(gtk::Overflow::Hidden);
        let layout = SplitLayout::default();
        body.set_layout_manager(Some(layout.clone()));
        let split = Tween::new(&body, 0.0);
        split.on_frame(move |value| layout.set_split(value));
        body.append(&panel);
        Self {
            body,
            split,
            eyebrow,
            source,
            pages,
            notes,
            content,
            scroller,
        }
    }

    fn show(&self, changelog: &Changelog) {
        let sections = render_changelog(&self.content, changelog);
        self.pages.set_visible_child(&self.notes);
        self.source
            .set_text(&format!("{} {}", changelog.pname, changelog.version));
        if !changelog.releases.is_empty() {
            self.eyebrow
                .set_text(&format!("RELEASE NOTES · {}", changelog.releases.len()));
        }
        if sections.len() > 1 {
            self.body
                .append(&self.jump_list(&changelog.releases, sections));
            self.split.set(1.0);
        }
    }

    fn show_error(&self, error: &str) {
        let notice = gtk::Box::new(gtk::Orientation::Vertical, 4);
        notice.add_css_class("changelog-notice");
        notice.add_css_class("notice");
        notice.add_css_class(crate::ui::theme::tone(crate::ui::theme::YELLOW));
        notice.set_valign(gtk::Align::Start);
        notice.append(&label(
            "Release notes unavailable",
            &["changelog-notice-title"],
            0.0,
        ));
        let message = label(error, &["changelog-notice-text"], 0.0);
        message.set_ellipsize(gtk::pango::EllipsizeMode::None);
        message.set_wrap(true);
        message.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        message.set_selectable(true);
        notice.append(&message);
        self.content.append(&notice);
        self.pages.set_visible_child(&self.notes);
    }

    /// Side card listing every release; a click glides the notes to that release and scrolling
    /// the notes keeps the release being read selected.
    fn jump_list(&self, releases: &[Release], sections: Vec<gtk::Box>) -> gtk::Box {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("phase-panel");
        card.append(&label("RELEASES", &["panel-eyebrow", "changelog-bar"], 0.0));
        let list = gtk::ListBox::new();
        list.add_css_class("changelog-release-list");
        list.add_css_class("see-through");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        for release in releases {
            let tag = label(&release.tag, &["changelog-release-tag"], 0.0);
            tag.set_tooltip_text(Some(&release.tag));
            let row = gtk::ListBoxRow::new();
            row.add_css_class("changelog-release-row");
            row.add_css_class("list-row");
            row.set_child(Some(&tag));
            list.append(&row);
        }
        list.select_row(list.row_at_index(0).as_ref());
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("see-through");
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&list));
        card.append(&FadeScroll::new(&scroller));

        // The glide a row click started; selection follows the notes only while none runs.
        let gliding = Rc::new(RefCell::new(Rc::new(Cell::new(false))));
        let content = self.content.clone();
        let offsets = Rc::new(move || {
            sections
                .iter()
                .map(|section| {
                    section
                        .compute_bounds(&content)
                        .map_or(0.0, |bounds| f64::from(bounds.y()))
                })
                .collect::<Vec<_>>()
        });

        let notes = self.scroller.clone();
        let jump_offsets = Rc::clone(&offsets);
        let jump_gliding = Rc::clone(&gliding);
        list.connect_row_activated(move |_, row| {
            let target = usize::try_from(row.index())
                .ok()
                .and_then(|index| jump_offsets().get(index).copied());
            if let Some(target) = target {
                jump_gliding.replace(glide(&notes, &notes.vadjustment(), target));
            }
        });

        let adjustment = self.scroller.vadjustment();
        adjustment.connect_value_changed(move |adjustment| {
            if gliding.borrow().get() {
                return;
            }
            let index = active_release(&offsets(), adjustment.value());
            let row = list.row_at_index(index as i32);
            if row.is_some() && list.selected_row() != row {
                list.select_row(row.as_ref());
            }
        });
        card
    }
}

mod split_layout {
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{graphene, gsk};

    use super::COLUMN_GAP;

    mod imp {
        use std::cell::Cell;

        use super::*;

        #[derive(Default)]
        pub(crate) struct SplitLayout {
            pub(super) split: Cell<f64>,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for SplitLayout {
            const NAME: &'static str = "MoltSplitLayout";
            type Type = super::SplitLayout;
            type ParentType = gtk::LayoutManager;
        }

        impl ObjectImpl for SplitLayout {}

        impl LayoutManagerImpl for SplitLayout {
            fn measure(
                &self,
                widget: &gtk::Widget,
                orientation: gtk::Orientation,
                for_size: i32,
            ) -> (i32, i32, i32, i32) {
                let Some(panel) = widget.first_child() else {
                    return (0, 0, -1, -1);
                };
                let (mut minimum, mut natural, _, _) = panel.measure(orientation, for_size);
                if let Some(side) = panel.next_sibling() {
                    let (side_minimum, side_natural, _, _) = side.measure(orientation, -1);
                    if orientation == gtk::Orientation::Horizontal {
                        let share = |size: i32| {
                            (f64::from(size + COLUMN_GAP) * self.split.get()).round() as i32
                        };
                        minimum += share(side_minimum);
                        natural += share(side_natural);
                    } else {
                        minimum = minimum.max(side_minimum);
                        natural = natural.max(side_natural);
                    }
                }
                (minimum, natural, -1, -1)
            }

            fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
                let Some(panel) = widget.first_child() else {
                    return;
                };
                let side = panel.next_sibling();
                // `phase_grid`'s side column.
                let column = (width - 2 * COLUMN_GAP) / 3;
                let shown = side.as_ref().map_or(0, |_| {
                    (f64::from(column + COLUMN_GAP) * self.split.get()).round() as i32
                });
                let panel_minimum = panel.measure(gtk::Orientation::Horizontal, height).0;
                panel.allocate((width - shown).max(panel_minimum), height, -1, None);
                if let Some(side) = side {
                    let side_minimum = side.measure(gtk::Orientation::Horizontal, height).0;
                    let offset = graphene::Point::new((width - shown + COLUMN_GAP) as f32, 0.0);
                    side.allocate(
                        column.max(side_minimum),
                        height,
                        -1,
                        Some(gsk::Transform::new().translate(&offset)),
                    );
                }
            }
        }
    }

    glib::wrapper! {
        /// Lays the notes panel (first child) across the whole width, handing the jump list
        /// (second child) its share as the split goes to 1; at 1 the two sit in `phase_grid`'s
        /// 2:1 columns. The list slides in beside the notes without reparenting them.
        pub(crate) struct SplitLayout(ObjectSubclass<imp::SplitLayout>)
            @extends gtk::LayoutManager;
    }

    impl Default for SplitLayout {
        fn default() -> Self {
            glib::Object::new()
        }
    }

    impl SplitLayout {
        /// `0..=1`: how far the jump list has slid in.
        pub(crate) fn set_split(&self, split: f64) {
            self.imp().split.set(split);
            self.layout_changed();
        }
    }
}

/// The release being read: the last one starting at or above the viewport top, allowing
/// `RELEASE_LEAD` so a release counts once its tag is in view.
fn active_release(offsets: &[f64], scroll: f64) -> usize {
    offsets
        .iter()
        .rposition(|offset| *offset <= scroll + RELEASE_LEAD)
        .unwrap_or(0)
}

/// Puts the notes back where the reader left them once they are long enough to scroll. No glide:
/// the screen itself is sliding in.
fn restore_scroll_position(scroller: &gtk::ScrolledWindow, position: Option<f64>) {
    let Some(position) = position else {
        return;
    };
    let adjustment = scroller.vadjustment();
    let handler = Rc::new(RefCell::new(None));
    let callback_handler = Rc::clone(&handler);
    let handler_id = adjustment.connect_changed(move |adjustment| {
        if try_restore_scroll_position(adjustment, position)
            && let Some(handler_id) = callback_handler.borrow_mut().take()
        {
            adjustment.disconnect(handler_id);
        }
    });
    handler.replace(Some(handler_id));

    glib::idle_add_local_once(move || {
        if handler.borrow().is_some()
            && try_restore_scroll_position(&adjustment, position)
            && let Some(handler_id) = handler.borrow_mut().take()
        {
            adjustment.disconnect(handler_id);
        }
    });
}

fn try_restore_scroll_position(adjustment: &gtk::Adjustment, position: f64) -> bool {
    let lower = adjustment.lower();
    if adjustment.upper() - adjustment.page_size() <= lower {
        return false;
    }
    adjustment.set_value(position);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ChangeStatus;

    #[test]
    fn tracks_the_release_at_the_viewport_top() {
        let offsets = [0.0, 400.0, 900.0];
        assert_eq!(active_release(&offsets, 0.0), 0);
        assert_eq!(active_release(&offsets, 370.0), 0);
        assert_eq!(active_release(&offsets, 376.0), 1);
        assert_eq!(active_release(&offsets, 899.0), 2);
        assert_eq!(active_release(&[], 120.0), 0);
    }

    #[test]
    fn describes_version_ranges() {
        let change = |old: &str, new: &str| Change {
            status: ChangeStatus::Upgraded,
            name: "demo".to_owned(),
            old: old.to_owned(),
            new: new.to_owned(),
            size: 0,
        };
        assert_eq!(
            version_range(&change("1.0", "2.0")),
            "1.0 → <span foreground=\"#cba6f7\">2.0</span>"
        );
        assert_eq!(
            version_range(&change("", "<2>")),
            "<span foreground=\"#cba6f7\">&lt;2&gt;</span>"
        );
    }
}
