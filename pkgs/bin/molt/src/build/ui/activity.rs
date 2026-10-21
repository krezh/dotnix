use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::ui::theme;
use crate::ui::{EnergyOverlay, FadeScroll, MorphLabel, SegmentedControl, Tween, field, label};

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

impl RowContent {
    /// What a row widget holds while no item is bound to it.
    fn unbound() -> Self {
        Self {
            group: usize::MAX,
            look: RowLook::Idle,
            title: String::new(),
            tag: String::new(),
            column: String::new(),
            detail: String::new(),
            value: String::new(),
            fraction: -1.0,
        }
    }

    fn search_text(&self) -> String {
        format!("{}\n{}", self.title, self.column).to_lowercase()
    }
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

mod imp {
    use super::*;

    #[derive(Default)]
    pub(super) struct RowItem {
        pub(super) content: RefCell<Option<RowContent>>,
        pub(super) search_text: RefCell<String>,
        /// When the row entered its group, ordering it within the group.
        pub(super) entered: Cell<u64>,
        /// The row widget showing this item while one is bound to it.
        pub(super) bound: RefCell<Weak<RefCell<ListRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RowItem {
        const NAME: &'static str = "MoltRowItem";
        type Type = super::RowItem;
    }

    impl ObjectImpl for RowItem {}
}

glib::wrapper! {
    /// One keyed row's content in the list model; widgets exist only for rows on screen.
    struct RowItem(ObjectSubclass<imp::RowItem>);
}

impl RowItem {
    fn new(content: RowContent, entered: u64) -> Self {
        let item = glib::Object::new::<Self>();
        let imp = item.imp();
        imp.search_text.replace(content.search_text());
        imp.content.replace(Some(content));
        imp.entered.set(entered);
        item
    }

    fn group(&self) -> usize {
        self.imp()
            .content
            .borrow()
            .as_ref()
            .map_or(0, |content| content.group)
    }

    fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.imp().search_text.borrow().contains(query)
    }

    /// Updates content that keeps the row in place, redrawing its widget if it has one.
    fn update(&self, content: RowContent) {
        let imp = self.imp();
        if let Some(row) = imp.bound.borrow().upgrade() {
            row.borrow_mut().apply(content.clone());
        }
        imp.content.replace(Some(content));
    }
}

fn row_item(object: &glib::Object) -> &RowItem {
    object
        .downcast_ref()
        .expect("grouped lists hold only row items")
}

/// A row's item and its index in the store, which only ever grows.
struct Entry {
    item: RowItem,
    index: usize,
}

/// A section header and the group it currently heads.
struct SectionHeader {
    root: gtk::Box,
    label: MorphLabel,
    group: Cell<usize>,
}

impl SectionHeader {
    fn show(&self, specs: &[GroupSpec], group: usize, shown: &[usize]) {
        let previous = self.group.replace(group);
        if previous != group {
            if let Some(spec) = specs.get(previous) {
                self.root.remove_css_class(theme::tone(spec.tone));
            }
            self.root.add_css_class(theme::tone(specs[group].tone));
        }
        self.label.set_text(&format!(
            "{} · {}",
            specs[group].name,
            shown.get(group).copied().unwrap_or_default()
        ));
    }
}

/// A filterable list of keyed rows grouped into sections, with a tab per group.
///
/// Rows live in a list model and only the ones on screen get widgets, so a build planning
/// thousands of paths costs no more to draw than one planning a dozen.
pub(crate) struct GroupedList {
    pub(crate) root: gtk::Box,
    pub(crate) scroll: gtk::ScrolledWindow,
    eyebrow: MorphLabel,
    eyebrow_text: &'static str,
    specs: &'static [GroupSpec],
    tabs: SegmentedControl,
    body: gtk::Stack,
    list_page: gtk::Widget,
    empty: gtk::Label,
    empty_text: &'static str,
    store: gio::ListStore,
    filter: gtk::CustomFilter,
    entries: RefCell<HashMap<String, Entry>>,
    /// Rows created since the last refresh, added to the store in one batch.
    pending: RefCell<Vec<RowItem>>,
    /// Bumped as rows enter a group.
    entered: Cell<u64>,
    /// Whether the list was scrolled to the top before the current batch of changes.
    top_before: Cell<Option<bool>>,
    /// Whether a scroll back to the top is waiting for the next layout.
    returning: Rc<Cell<bool>>,
    tab: Rc<Cell<usize>>,
    query: Rc<RefCell<String>>,
    headers: Rc<RefCell<HashMap<gtk::ListHeader, SectionHeader>>>,
    /// Rows matching the filter per group, as the section headers count them.
    shown: Rc<RefCell<Vec<usize>>>,
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

        let tab = Rc::new(Cell::new(0));
        let query = Rc::new(RefCell::new(String::new()));
        let store = gio::ListStore::new::<RowItem>();
        // Rows keep the order they entered their group in, newest first where the group says.
        let sorter = gtk::CustomSorter::new(move |a, b| {
            let key = |item: &RowItem| {
                let group = item.group();
                let entered = item.imp().entered.get() as i64;
                let newest_first = specs.get(group).is_some_and(|spec| spec.newest_first);
                (group, if newest_first { -entered } else { entered })
            };
            key(row_item(a)).cmp(&key(row_item(b))).into()
        });
        let sections =
            gtk::CustomSorter::new(|a, b| row_item(a).group().cmp(&row_item(b).group()).into());
        let sorted = gtk::SortListModel::new(Some(store.clone()), Some(sorter));
        sorted.set_section_sorter(Some(&sections));
        let filter = {
            let (tab, query) = (Rc::clone(&tab), Rc::clone(&query));
            gtk::CustomFilter::new(move |object| {
                let item = row_item(object);
                let tab = tab.get();
                (tab == 0 || tab == item.group() + 1) && item.matches(&query.borrow())
            })
        };
        let filtered = gtk::FilterListModel::new(Some(sorted), Some(filter.clone()));

        let rows = Rc::new(RefCell::new(
            HashMap::<gtk::ListItem, Rc<RefCell<ListRow>>>::new(),
        ));
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup({
            let rows = Rc::clone(&rows);
            move |_, object| {
                let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                list_item.set_activatable(false);
                list_item.set_selectable(false);
                list_item.set_focusable(false);
                let row = ListRow::new(specs, has_column);
                list_item.set_child(Some(&row.energy.root));
                rows.borrow_mut()
                    .insert(list_item.clone(), Rc::new(RefCell::new(row)));
            }
        });
        factory.connect_bind({
            let rows = Rc::clone(&rows);
            move |_, object| {
                let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let (Some(item), Some(row)) = (
                    list_item.item().and_downcast::<RowItem>(),
                    rows.borrow().get(list_item).cloned(),
                ) else {
                    return;
                };
                if let Some(content) = item.imp().content.borrow().clone() {
                    row.borrow_mut().show(content);
                }
                item.imp().bound.replace(Rc::downgrade(&row));
            }
        });
        factory.connect_unbind({
            let rows = Rc::clone(&rows);
            move |_, object| {
                let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let Some(row) = rows.borrow().get(list_item).cloned() else {
                    return;
                };
                if let Some(item) = list_item.item().and_downcast::<RowItem>() {
                    let bound = item.imp().bound.borrow().upgrade();
                    if bound.is_some_and(|bound| Rc::ptr_eq(&bound, &row)) {
                        item.imp().bound.replace(Weak::new());
                    }
                }
                row.borrow_mut().clear();
            }
        });
        factory.connect_teardown(move |_, object| {
            if let Some(list_item) = object.downcast_ref::<gtk::ListItem>() {
                rows.borrow_mut().remove(list_item);
            }
        });

        let headers = Rc::new(RefCell::new(
            HashMap::<gtk::ListHeader, SectionHeader>::new(),
        ));
        let shown = Rc::new(RefCell::new(vec![0; specs.len()]));
        let header_factory = gtk::SignalListItemFactory::new();
        header_factory.connect_setup({
            let headers = Rc::clone(&headers);
            move |_, object| {
                let Some(list_header) = object.downcast_ref::<gtk::ListHeader>() else {
                    return;
                };
                let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
                root.add_css_class("build-section");
                let label = MorphLabel::new("", &["phase-section-label"], 0.0);
                root.append(&label.root);
                list_header.set_child(Some(&root));
                headers.borrow_mut().insert(
                    list_header.clone(),
                    SectionHeader {
                        root,
                        label,
                        group: Cell::new(usize::MAX),
                    },
                );
            }
        });
        header_factory.connect_bind({
            let (headers, shown) = (Rc::clone(&headers), Rc::clone(&shown));
            move |_, object| {
                let Some(list_header) = object.downcast_ref::<gtk::ListHeader>() else {
                    return;
                };
                let Some(item) = list_header.item().and_downcast::<RowItem>() else {
                    return;
                };
                if let Some(header) = headers.borrow().get(list_header) {
                    header.show(specs, item.group(), &shown.borrow());
                }
            }
        });
        header_factory.connect_teardown({
            let headers = Rc::clone(&headers);
            move |_, object| {
                if let Some(list_header) = object.downcast_ref::<gtk::ListHeader>() {
                    headers.borrow_mut().remove(list_header);
                }
            }
        });

        let view = gtk::ListView::new(Some(gtk::NoSelection::new(Some(filtered))), Some(factory));
        view.set_header_factory(Some(&header_factory));
        view.add_css_class("build-list");
        view.set_can_focus(false);
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&view));
        let list_page = FadeScroll::new(&scroll).upcast::<gtk::Widget>();
        let empty = label(empty_text, &["build-list-empty"], 0.0);
        empty.set_wrap(true);
        empty.set_valign(gtk::Align::Start);
        let body = gtk::Stack::new();
        body.set_vexpand(true);
        body.add_child(&list_page);
        body.add_child(&empty);
        body.set_visible_child(&empty);
        root.append(&body);

        let list = Rc::new(Self {
            root,
            scroll,
            eyebrow,
            eyebrow_text,
            specs,
            tabs,
            body,
            list_page,
            empty,
            empty_text,
            store,
            filter,
            entries: RefCell::new(HashMap::new()),
            pending: RefCell::new(Vec::new()),
            entered: Cell::new(0),
            top_before: Cell::new(None),
            returning: Rc::new(Cell::new(false)),
            tab,
            query,
            headers,
            shown,
        });
        for (index, option) in list.tabs.options.iter().enumerate() {
            let list = Rc::downgrade(&list);
            option.connect_toggled(move |option| {
                if option.is_active() {
                    with_list(&list, |list| {
                        list.tab.set(index);
                        list.filter.changed(gtk::FilterChange::Different);
                        list.refresh();
                    });
                }
            });
        }
        let weak = Rc::downgrade(&list);
        search.connect_search_changed(move |search| {
            with_list(&weak, |list| {
                list.query.replace(search.text().trim().to_lowercase());
                list.filter.changed(gtk::FilterChange::Different);
                list.refresh();
            });
        });
        list
    }

    /// Creates or updates the row for `key`; call [`Self::refresh`] after a batch.
    pub(crate) fn upsert(&self, key: &str, content: RowContent) {
        let mut entries = self.entries.borrow_mut();
        if let Some(entry) = entries.get_mut(key) {
            let in_place = {
                let current = entry.item.imp().content.borrow();
                let Some(current) = current.as_ref() else {
                    return;
                };
                if *current == content {
                    return;
                }
                current.group == content.group
                    && current.title == content.title
                    && current.column == content.column
            };
            if in_place {
                entry.item.update(content);
            } else {
                // A new item, not the same one moved: the list keeps its scroll position
                // instead of following the row to its new group.
                let item = self.new_item(content);
                self.note_top();
                self.replace(entry.index, &item);
                entry.item = item;
            }
            return;
        }
        self.note_top();
        let item = self.new_item(content);
        let mut pending = self.pending.borrow_mut();
        let index = self.store.n_items() as usize + pending.len();
        pending.push(item.clone());
        entries.insert(key.to_owned(), Entry { item, index });
    }

    fn new_item(&self, content: RowContent) -> RowItem {
        let entered = self.entered.get() + 1;
        self.entered.set(entered);
        RowItem::new(content, entered)
    }

    fn replace(&self, index: usize, item: &RowItem) {
        let stored = self.store.n_items() as usize;
        if index < stored {
            self.store
                .splice(index as u32, 1, std::slice::from_ref(item));
        } else {
            self.pending.borrow_mut()[index - stored] = item.clone();
        }
    }

    /// Records, before the first change of a batch, whether the list sits at its top.
    fn note_top(&self) {
        if self.top_before.get().is_none() {
            let adjustment = self.scroll.vadjustment();
            self.top_before
                .set(Some(adjustment.value() <= adjustment.lower() + 1.0));
        }
    }

    /// Scrolls back to the top once the changed rows are laid out. The list view keeps its
    /// top row in place, so rows inserted above it would otherwise push the view down.
    fn keep_top(&self) {
        if self.returning.replace(true) {
            return;
        }
        let adjustment = self.scroll.vadjustment();
        let returning = Rc::clone(&self.returning);
        let laid_out = Cell::new(false);
        self.scroll.add_tick_callback(move |_, _| {
            // Tick callbacks run before layout: the first frame lays the new rows out.
            if !laid_out.replace(true) {
                return glib::ControlFlow::Continue;
            }
            adjustment.set_value(adjustment.lower());
            returning.set(false);
            glib::ControlFlow::Break
        });
    }

    /// Adds new rows, recounts groups, and applies the active tab and filter to the tabs.
    pub(crate) fn refresh(&self) {
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        if !pending.is_empty() {
            self.note_top();
            self.store.extend_from_slice(&pending);
        }
        if self.top_before.take() == Some(true) {
            self.keep_top();
        }
        let entries = self.entries.borrow();
        let query = self.query.borrow();
        let mut totals = vec![0_usize; self.specs.len()];
        let mut shown = vec![0_usize; self.specs.len()];
        for entry in entries.values() {
            let group = entry.item.group();
            totals[group] += 1;
            if entry.item.matches(&query) {
                shown[group] += 1;
            }
        }
        let tab = self.tab.get();
        let mut any = false;
        for (index, spec) in self.specs.iter().enumerate() {
            any |= shown[index] > 0 && (tab == 0 || tab == index + 1);
            self.tabs
                .set_label(index + 1, &format!("{} {}", spec.name, totals[index]));
            self.tabs
                .set_option_visible(index + 1, !spec.optional || totals[index] > 0);
        }
        for header in self.headers.borrow().values() {
            let group = header.group.get();
            if group < self.specs.len() {
                header.show(self.specs, group, &shown);
            }
        }
        *self.shown.borrow_mut() = shown;
        self.tabs.set_label(0, &format!("All {}", entries.len()));
        self.eyebrow
            .set_text(&format!("{} · {}", self.eyebrow_text, entries.len()));
        if any {
            self.body.set_visible_child(&self.list_page);
        } else {
            set_text(
                &self.empty,
                if query.is_empty() {
                    self.empty_text
                } else {
                    "Nothing matches the filter"
                },
            );
            self.body.set_visible_child(&self.empty);
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
    specs: &'static [GroupSpec],
    has_column: bool,
    content: RowContent,
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
    fn new(specs: &'static [GroupSpec], has_column: bool) -> Self {
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
            specs,
            has_column,
            content: RowContent::unbound(),
        }
    }

    /// Shows a newly bound item's content at once, without the state-change transitions.
    fn show(&mut self, content: RowContent) {
        self.set_transitions(0);
        self.fraction.jump(content.fraction);
        self.apply(content);
        self.set_transitions(ROW_TRANSITION_MS);
    }

    fn set_transitions(&self, duration: u32) {
        self.marker.set_transition_duration(duration);
        for slot in [&self.column_slot, &self.detail_slot, &self.bar_slot] {
            slot.set_transition_duration(duration);
        }
    }

    /// Lets go of the item the row showed, so the widget can be reused for another. The text
    /// stays in `content`, matching the labels, so the next item only rewrites what differs.
    fn clear(&mut self) {
        if self.content.look == RowLook::Active {
            self.energy.stop();
        }
        if let Some(spec) = self.specs.get(self.content.group) {
            self.energy.root.remove_css_class(theme::tone(spec.tone));
        }
        self.card.remove_css_class(self.content.look.class());
        self.content.group = usize::MAX;
        self.content.look = RowLook::Idle;
    }

    fn apply(&mut self, content: RowContent) {
        let previous = &self.content;
        // Before the look: a bar about to be revealed is still unmapped, so its first reading
        // lands at once instead of sliding up from the last transfer's.
        if previous.fraction != content.fraction {
            self.fraction.set(content.fraction);
        }
        // The group's tone colours the row's dot.
        if previous.group != content.group {
            if let Some(spec) = self.specs.get(previous.group) {
                self.energy.root.remove_css_class(theme::tone(spec.tone));
            }
            if let Some(spec) = self.specs.get(content.group) {
                self.energy.root.add_css_class(theme::tone(spec.tone));
            }
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
}
