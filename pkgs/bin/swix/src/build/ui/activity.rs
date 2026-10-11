mod glide;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use gtk::prelude::*;

use crate::ui::theme;
use crate::ui::{EnergyOverlay, FadeScroll, MorphLabel, SegmentedControl, Tween, field, label};
use glide::GlideBox;

/// Length of a row's state change: marker crossfade, detail and bar reveals.
const ROW_TRANSITION_MS: u32 = 200;

/// Catppuccin Mocha sapphire for transfer bars, surface1 for their track.
const BAR_FILL: theme::Color = theme::SAPPHIRE;
const BAR_TRACK: theme::Color = theme::SURFACE1;

/// How a row presents its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowLook {
    /// Raised, two lines, energy beam: work happening now.
    Active,
    /// Lightly raised with a progress bar: a transfer.
    Transfer,
    /// Dim dot: waiting, or never touched.
    Idle,
    Done,
    Failed,
}

impl RowLook {
    const fn class(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Transfer => "transfer",
            Self::Idle => "idle",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RowContent {
    pub(crate) group: usize,
    pub(crate) look: RowLook,
    pub(crate) title: String,
    /// Store-hash prefix telling apart rows whose titles would otherwise be identical.
    pub(crate) tag: String,
    /// Secondary text beside the title on flat rows of lists built with a source column.
    pub(crate) column: String,
    /// Monospace second line of active rows.
    pub(crate) detail: String,
    pub(crate) value: String,
    /// Transfer progress, drawn as a bar on transfer rows.
    pub(crate) fraction: f64,
}

pub(crate) struct GroupSpec {
    pub(crate) name: &'static str,
    /// `tone-*` class colouring the section label and row markers.
    pub(crate) tone: theme::Color,
    /// Rows entering this group go to its top rather than its bottom.
    pub(crate) newest_first: bool,
    /// The group's tab shows only while the group has rows.
    pub(crate) optional: bool,
}

struct Group {
    section: gtk::Box,
    header: MorphLabel,
    rows: GlideBox,
}

/// A filterable list of keyed rows grouped into sections, with a tab per group.
///
/// Rows are created once per key and updated in place; a row changing group moves its widget,
/// so progress ticks never rebuild the list. Rows and sections glide to their new places.
pub(crate) struct GroupedList {
    pub(crate) root: gtk::Box,
    pub(crate) scroll: gtk::ScrolledWindow,
    eyebrow: MorphLabel,
    eyebrow_text: &'static str,
    specs: &'static [GroupSpec],
    tabs: SegmentedControl,
    content: GlideBox,
    groups: Vec<Group>,
    empty: gtk::Label,
    empty_text: &'static str,
    has_column: bool,
    rows: RefCell<HashMap<String, ListRow>>,
    query: RefCell<String>,
}

impl GroupedList {
    pub(crate) fn new(
        eyebrow_text: &'static str,
        specs: &'static [GroupSpec],
        empty_text: &'static str,
        has_column: bool,
    ) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("phase-panel");
        root.add_css_class("build-list-panel");
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        toolbar.add_css_class("build-list-toolbar");
        let eyebrow = MorphLabel::new(eyebrow_text, &["panel-eyebrow"], 0.0);
        eyebrow.set_ellipsize(gtk::pango::EllipsizeMode::None);
        eyebrow.root.set_valign(gtk::Align::Center);
        toolbar.append(&eyebrow.root);
        let names = std::iter::once("All")
            .chain(specs.iter().map(|spec| spec.name))
            .collect::<Vec<_>>();
        let tabs = SegmentedControl::new(&names);
        tabs.root.set_valign(gtk::Align::Center);
        tabs.root.set_hexpand(true);
        tabs.root.set_halign(gtk::Align::Start);
        toolbar.append(&tabs.root);
        let search = gtk::SearchEntry::new();
        search.add_css_class("build-filter");
        field(&search);
        search.set_placeholder_text(Some("Filter"));
        search.set_width_request(150);
        search.set_valign(gtk::Align::Center);
        toolbar.append(&search);
        root.append(&toolbar);
        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("build-list-rule");
        root.append(&rule);

        let content = GlideBox::new(0);
        content.add_css_class("build-list");
        let groups = specs
            .iter()
            .map(|spec| {
                let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
                section.add_css_class("build-section");
                section.add_css_class(theme::tone(spec.tone));
                section.set_visible(false);
                let header = MorphLabel::new(spec.name, &["phase-section-label"], 0.0);
                section.append(&header.root);
                let rows = GlideBox::new(4);
                section.append(&rows);
                content.append(&section);
                Group {
                    section,
                    header,
                    rows,
                }
            })
            .collect();
        let empty = label(empty_text, &["build-list-empty"], 0.0);
        empty.set_wrap(true);
        content.append(&empty);
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&content));
        root.append(&FadeScroll::new(&scroll));

        let list = Rc::new(Self {
            root,
            scroll,
            eyebrow,
            eyebrow_text,
            specs,
            tabs,
            content,
            groups,
            empty,
            empty_text,
            has_column,
            rows: RefCell::new(HashMap::new()),
            query: RefCell::new(String::new()),
        });
        for option in &list.tabs.options {
            let list = Rc::downgrade(&list);
            option.connect_toggled(move |option| {
                if option.is_active() {
                    with_list(&list, |list| list.refresh());
                }
            });
        }
        let weak = Rc::downgrade(&list);
        search.connect_search_changed(move |search| {
            with_list(&weak, |list| {
                list.query.replace(search.text().trim().to_lowercase());
                let query = list.query.borrow();
                for row in list.rows.borrow().values() {
                    row.energy.root.set_visible(row.matches(&query));
                }
                drop(query);
                list.refresh();
            });
        });
        list
    }

    /// Creates or updates the row for `key`; call [`Self::refresh`] after a batch.
    pub(crate) fn upsert(&self, key: &str, content: RowContent) {
        let mut rows = self.rows.borrow_mut();
        if let Some(row) = rows.get_mut(key) {
            if row.content == content {
                return;
            }
            if row.content.group != content.group {
                let widget = &row.energy.root;
                let from = &self.groups[row.content.group].rows;
                let origin = from.drawn_y(widget, &self.content);
                from.remove(widget);
                self.place(widget, content.group);
                if let Some(origin) = origin {
                    self.groups[content.group]
                        .rows
                        .arrive(widget, &self.content, origin);
                }
            }
            row.apply(content);
            row.energy.root.set_visible(row.matches(&self.query.borrow()));
            return;
        }
        let mut row = ListRow::new(self.has_column);
        self.place(&row.energy.root, content.group);
        row.apply(content);
        row.energy.root.set_visible(row.matches(&self.query.borrow()));
        rows.insert(key.to_owned(), row);
    }

    fn place(&self, widget: &gtk::Overlay, group: usize) {
        let rows = &self.groups[group].rows;
        if self.specs[group].newest_first {
            rows.prepend(widget);
        } else {
            rows.append(widget);
        }
    }

    /// Recounts groups and applies the active tab and filter to sections.
    pub(crate) fn refresh(&self) {
        let rows = self.rows.borrow();
        let mut totals = vec![0_usize; self.groups.len()];
        let mut shown = vec![0_usize; self.groups.len()];
        for row in rows.values() {
            totals[row.content.group] += 1;
            // `get_visible` is the row's own flag; `is_visible` also requires visible ancestors,
            // which would keep a hidden section's rows from ever counting and the section hidden.
            if row.energy.root.get_visible() {
                shown[row.content.group] += 1;
            }
        }
        let tab = self
            .tabs
            .options
            .iter()
            .position(|option| option.is_active())
            .unwrap_or(0);
        let mut any = false;
        for (index, group) in self.groups.iter().enumerate() {
            let visible = shown[index] > 0 && (tab == 0 || tab == index + 1);
            any |= visible;
            group.section.set_visible(visible);
            group
                .header
                .set_text(&format!("{} · {}", self.specs[index].name, shown[index]));
            self.tabs.set_label(
                index + 1,
                &format!("{} {}", self.specs[index].name, totals[index]),
            );
            self.tabs.set_option_visible(
                index + 1,
                !self.specs[index].optional || totals[index] > 0,
            );
        }
        self.tabs.set_label(0, &format!("All {}", rows.len()));
        self.eyebrow
            .set_text(&format!("{} · {}", self.eyebrow_text, rows.len()));
        self.empty.set_visible(!any);
        if !any {
            set_text(
                &self.empty,
                if self.query.borrow().is_empty() {
                    self.empty_text
                } else {
                    "Nothing matches the filter"
                },
            );
        }
    }
}

fn with_list(list: &Weak<GroupedList>, action: impl FnOnce(&GroupedList)) {
    if let Some(list) = list.upgrade() {
        action(&list);
    }
}

fn set_text(label: &gtk::Label, text: &str) {
    if label.text() != text {
        label.set_text(text);
    }
}

struct ListRow {
    energy: EnergyOverlay,
    card: gtk::Box,
    /// Crossfades between the dot (live or waiting work) and the done/failed marks.
    marker: gtk::Stack,
    head: gtk::Box,
    title: gtk::Label,
    tag: gtk::Label,
    column: gtk::Label,
    column_slot: gtk::Revealer,
    detail: gtk::Label,
    detail_slot: gtk::Revealer,
    bar_slot: gtk::Revealer,
    fraction: Tween,
    value: gtk::Label,
    has_column: bool,
    content: RowContent,
    search_text: String,
}

fn row_revealer(
    child: &impl IsA<gtk::Widget>,
    transition: gtk::RevealerTransitionType,
) -> gtk::Revealer {
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(transition);
    revealer.set_transition_duration(ROW_TRANSITION_MS);
    revealer.set_child(Some(child));
    revealer
}

impl ListRow {
    fn new(has_column: bool) -> Self {
        let card = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        card.add_css_class("build-row");
        card.set_hexpand(true);
        let marker = gtk::Stack::new();
        marker.add_css_class("build-row-marker");
        marker.set_transition_type(gtk::StackTransitionType::Crossfade);
        marker.set_transition_duration(ROW_TRANSITION_MS);
        marker.set_valign(gtk::Align::Center);
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("build-row-dot");
        dot.set_halign(gtk::Align::Center);
        dot.set_valign(gtk::Align::Center);
        marker.add_named(&dot, Some("dot"));
        for (name, text) in [("done", "✓"), ("failed", "✕")] {
            let mark = label(text, &["build-row-mark"], 0.5);
            mark.set_ellipsize(gtk::pango::EllipsizeMode::None);
            marker.add_named(&mark, Some(name));
        }
        card.append(&marker);

        let text = gtk::Box::new(gtk::Orientation::Vertical, 0);
        text.set_hexpand(true);
        text.set_valign(gtk::Align::Center);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        // The tag follows the title directly; the head box, not the title, takes spare width.
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = label("", &["build-row-title"], 0.0);
        head.append(&title);
        let tag = label("", &["build-row-tag"], 0.0);
        tag.set_ellipsize(gtk::pango::EllipsizeMode::None);
        tag.set_visible(false);
        head.append(&tag);
        line.append(&head);
        let column = label("", &["build-row-column"], 0.0);
        let column_slot = row_revealer(&column, gtk::RevealerTransitionType::SlideRight);
        column_slot.set_hexpand(true);
        line.append(&column_slot);
        let bar = gtk::DrawingArea::new();
        bar.set_content_width(150);
        bar.set_content_height(4);
        // The bar draws each tween frame; the tween itself stays off the draw func, which would
        // otherwise keep the bar alive through its own reference.
        let fraction = Tween::new(&bar, 0.0);
        let drawn = Rc::new(Cell::new(0.0_f64));
        let frame = Rc::clone(&drawn);
        fraction.on_frame(move |value| frame.set(value));
        bar.set_draw_func(move |_, context, width, height| {
            let (width, height) = (f64::from(width), f64::from(height));
            theme::set_source_rgb(context, BAR_TRACK);
            theme::rounded_rectangle(context, 0.0, 0.0, width, height, height / 2.0);
            let _ = context.fill();
            let filled = width * drawn.get().clamp(0.0, 1.0);
            if filled > 0.0 {
                theme::set_source_rgb(context, BAR_FILL);
                theme::rounded_rectangle(context, 0.0, 0.0, filled, height, height / 2.0);
                let _ = context.fill();
            }
        });
        let bar_slot = row_revealer(&bar, gtk::RevealerTransitionType::SlideLeft);
        bar_slot.set_valign(gtk::Align::Center);
        line.append(&bar_slot);
        text.append(&line);
        let detail = label("", &["build-row-detail"], 0.0);
        let detail_slot = row_revealer(&detail, gtk::RevealerTransitionType::SlideDown);
        text.append(&detail_slot);
        card.append(&text);
        let value = label("", &["build-row-value"], 1.0);
        value.set_ellipsize(gtk::pango::EllipsizeMode::None);
        value.set_valign(gtk::Align::Center);
        card.append(&value);
        let energy = EnergyOverlay::new(&card, 7.0);
        Self {
            energy,
            card,
            marker,
            head,
            title,
            tag,
            column,
            column_slot,
            detail,
            detail_slot,
            bar_slot,
            fraction,
            value,
            has_column,
            content: RowContent {
                group: usize::MAX,
                look: RowLook::Idle,
                title: String::new(),
                tag: String::new(),
                column: String::new(),
                detail: String::new(),
                value: String::new(),
                fraction: -1.0,
            },
            search_text: String::new(),
        }
    }

    fn apply(&mut self, content: RowContent) {
        let previous = &self.content;
        // Before the look: a bar about to be revealed is still unmapped, so its first reading
        // lands at once instead of sliding up from the last transfer's.
        if previous.fraction != content.fraction {
            self.fraction.set(content.fraction);
        }
        if previous.look != content.look || previous.group == usize::MAX {
            self.card.remove_css_class(previous.look.class());
            self.card.add_css_class(content.look.class());
            let flat = !matches!(content.look, RowLook::Active | RowLook::Transfer);
            let with_column = self.has_column && flat;
            self.column_slot.set_reveal_child(with_column);
            self.head.set_hexpand(!with_column);
            self.head
                .set_width_request(if with_column { 190 } else { -1 });
            self.bar_slot
                .set_reveal_child(content.look == RowLook::Transfer);
            self.detail_slot.set_reveal_child(matches!(
                content.look,
                RowLook::Active | RowLook::Transfer | RowLook::Failed
            ));
            self.marker.set_visible_child_name(match content.look {
                RowLook::Done => "done",
                RowLook::Failed => "failed",
                RowLook::Active | RowLook::Transfer | RowLook::Idle => "dot",
            });
            if content.look == RowLook::Active {
                self.energy.start();
            } else if previous.look == RowLook::Active {
                self.energy.stop();
            }
        }
        if previous.title != content.title || previous.column != content.column {
            set_text(&self.title, &content.title);
            set_text(&self.column, &content.column);
            self.title.set_tooltip_text(Some(&content.title));
            self.search_text = format!("{}\n{}", content.title, content.column).to_lowercase();
        }
        if previous.tag != content.tag {
            set_text(&self.tag, &content.tag);
            self.tag.set_visible(!content.tag.is_empty());
        }
        if previous.detail != content.detail {
            set_text(&self.detail, &content.detail);
        }
        if previous.value != content.value {
            set_text(&self.value, &content.value);
        }
        self.content = content;
    }

    fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.search_text.contains(query)
    }
}
