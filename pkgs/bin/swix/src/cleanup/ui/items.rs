use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};

use crate::app::state::UiState;
use crate::cleanup::{CleanupGroup, CleanupItem, CleanupKind};
use crate::ui::{FadeScroll, MorphLabel, field, format_size, label, theme};

use super::category_title;

/// Width of the per-item size bar; the column header reserves the same space.
const BAR_WIDTH: i32 = 80;
/// Characters reserved for the size column, wide enough for "On cleanup".
const SIZE_CHARS: i32 = 10;

/// An item with its lowercased label and detail, so filtering does not allocate per keystroke.
struct Entry {
    item: CleanupItem,
    haystack: String,
}

/// One list buffer: a category's items, or the reason it has none. Two of them let a replaced
/// list crossfade out over its successor.
struct Page {
    root: gtk::Stack,
    store: gio::ListStore,
    largest: Rc<Cell<u64>>,
    scroll: gtk::ScrolledWindow,
    empty: gtk::Label,
}

impl Page {
    fn new(filter: &gtk::CustomFilter) -> Self {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));

        let largest = Rc::new(Cell::new(0));
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, object| {
            if let Some(list_item) = object.downcast_ref::<gtk::ListItem>() {
                list_item.set_activatable(false);
                list_item.set_child(Some(&item_row()));
            }
        });
        let bind_largest = Rc::clone(&largest);
        factory.connect_bind(move |_, object| {
            let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let (Some(entry), Some(root)) = (
                list_item.item().and_downcast::<glib::BoxedAnyObject>(),
                list_item.child().and_downcast::<gtk::Box>(),
            ) else {
                return;
            };
            ItemRow::from_root(&root).bind(&entry.borrow::<Entry>().item, bind_largest.get());
        });
        let list = gtk::ListView::new(Some(gtk::NoSelection::new(Some(filtered.clone()))), Some(factory));
        list.add_css_class("cleanup-items-list");
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("cleanup-items-scroll");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&list));

        let empty = label("", &["cleanup-items-empty"], 0.5);
        empty.set_ellipsize(gtk::pango::EllipsizeMode::None);
        empty.set_wrap(true);
        empty.set_valign(gtk::Align::Start);
        let root = gtk::Stack::new();
        root.set_transition_type(gtk::StackTransitionType::Crossfade);
        root.set_transition_duration(150);
        root.set_vexpand(true);
        root.add_named(&FadeScroll::new(&scroll), Some("list"));
        root.add_named(&empty, Some("empty"));

        // An empty store keeps the reason replace_items gave; a non-empty one means the filter hid it all.
        // Weak: the page owns the list model that emits this.
        let (body, empty_label) = (root.downgrade(), empty.downgrade());
        filtered.connect_items_changed(move |filtered, _, _, _| {
            let (Some(body), Some(empty_label)) = (body.upgrade(), empty_label.upgrade()) else {
                return;
            };
            if filtered.n_items() > 0 {
                body.set_visible_child_name("list");
            } else {
                if filtered.model().is_some_and(|store| store.n_items() > 0) {
                    empty_label.set_text("No items match this filter");
                }
                body.set_visible_child_name("empty");
            }
        });

        Self {
            root,
            store,
            largest,
            scroll,
            empty,
        }
    }
}

/// One category's complete removal list, virtualised so stores with tens of thousands of paths
/// stay responsive.
pub(super) struct CleanupItems {
    pub(super) root: gtk::Box,
    title: MorphLabel,
    summary: MorphLabel,
    note: gtk::Label,
    note_reveal: gtk::Revealer,
    search: gtk::SearchEntry,
    query: Rc<RefCell<String>>,
    body: gtk::Stack,
    pages: Rc<[Page; 2]>,
    /// Index into `pages` of the page on show.
    shown: Cell<usize>,
    state: Rc<UiState>,
}

impl CleanupItems {
    pub(super) fn new(state: &Rc<UiState>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("cleanup-items");
        root.set_hexpand(true);
        root.set_vexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = MorphLabel::new("", &["cleanup-items-title", "toned"], 0.0);
        title.root.set_hexpand(true);
        header.append(&title.root);
        let summary = MorphLabel::new("", &["cleanup-section-detail"], 1.0);
        header.append(&summary.root);
        root.append(&header);

        let note = label("", &["cleanup-items-note", "notice", theme::tone(theme::YELLOW)], 0.0);
        note.set_ellipsize(gtk::pango::EllipsizeMode::None);
        note.set_wrap(true);
        note.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        // The note brings its own gap to the filter, so a hidden note leaves none behind.
        note.set_margin_bottom(10);
        let note_reveal = gtk::Revealer::new();
        note_reveal.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        note_reveal.set_transition_duration(200);
        note_reveal.set_child(Some(&note));

        let search = gtk::SearchEntry::new();
        search.add_css_class("cleanup-items-filter");
        field(&search);
        search.set_placeholder_text(Some("Filter items"));
        let filter_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
        filter_area.append(&note_reveal);
        filter_area.append(&search);
        root.append(&filter_area);

        let columns = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        columns.add_css_class("cleanup-items-columns");
        columns.append(&column_label("ITEM", "cleanup-items-column"));
        columns.append(&column_label("DETAIL", "cleanup-items-column"));
        let bar_space = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar_space.set_size_request(BAR_WIDTH, -1);
        columns.append(&bar_space);
        let size = label("SIZE", &["cleanup-items-column"], 1.0);
        size.set_width_chars(SIZE_CHARS);
        columns.append(&size);
        root.append(&columns);

        let query = Rc::new(RefCell::new(String::new()));
        let filter_query = Rc::clone(&query);
        let filter = gtk::CustomFilter::new(move |object| {
            let query = filter_query.borrow();
            query.is_empty()
                || object
                    .downcast_ref::<glib::BoxedAnyObject>()
                    .is_some_and(|entry| entry.borrow::<Entry>().haystack.contains(query.as_str()))
        });
        let pages = Rc::new([Page::new(&filter), Page::new(&filter)]);
        let body = gtk::Stack::new();
        body.set_transition_type(gtk::StackTransitionType::Crossfade);
        body.set_transition_duration(200);
        body.set_vexpand(true);
        for page in pages.iter() {
            body.add_child(&page.root);
        }
        root.append(&body);
        // The outgoing page is drawn throughout the crossfade, so it is emptied once that ends.
        let weak_pages = Rc::downgrade(&pages);
        body.connect_transition_running_notify(move |body| {
            if let Some(pages) = weak_pages.upgrade()
                && !body.is_transition_running()
            {
                release_hidden(body, &pages);
            }
        });

        let search_query = Rc::clone(&query);
        search.connect_search_changed(move |search| {
            let text = search.text().trim().to_lowercase();
            if *search_query.borrow() != text {
                search_query.replace(text);
                filter.changed(gtk::FilterChange::Different);
            }
        });

        Self {
            root,
            title,
            summary,
            note,
            note_reveal,
            search,
            query,
            body,
            pages,
            shown: Cell::new(0),
            state: Rc::clone(state),
        }
    }

    pub(super) fn set_loading(&self, kind: CleanupKind) {
        self.set_category(kind);
        self.summary.set_text("Scanning…");
        self.note_reveal.set_reveal_child(false);
        self.replace_items(kind, Vec::new(), 0, "Scanning this cleanup job…");
    }

    pub(super) fn set_group(&self, group: &CleanupGroup) {
        self.set_category(group.kind);
        self.summary.set_text(&if group.available {
            format!(
                "{} item{} · {} estimated",
                group.items.len(),
                if group.items.len() == 1 { "" } else { "s" },
                format_size(group.reclaimable),
            )
        } else {
            "Cleanup job unavailable".to_owned()
        });
        // A hiding note keeps its text until it has slid away.
        if let Some(note) = &group.note {
            self.note.set_text(note);
        }
        self.note_reveal.set_reveal_child(group.note.is_some());

        let largest = group.items.iter().filter_map(|item| item.bytes).max().unwrap_or(0);
        let mut items: Vec<&CleanupItem> = group.items.iter().collect();
        // Items sized only at cleanup time (Nix generations) lead, then the largest first.
        items.sort_by_key(|item| Reverse(item.bytes.unwrap_or(u64::MAX)));
        let entries = items
            .into_iter()
            .map(|item| {
                glib::BoxedAnyObject::new(Entry {
                    haystack: format!("{}\n{}", item.label, item.detail).to_lowercase(),
                    item: item.clone(),
                })
            })
            .collect();
        self.replace_items(
            group.kind,
            entries,
            largest,
            if group.available {
                "Nothing is currently eligible for removal"
            } else {
                "The scan did not return a removal plan"
            },
        );
    }

    /// Crossfades to `kind`'s new list, from the top with the filter cleared; `empty` explains an
    /// empty list. The list fills the hidden page, so neither the swap nor the scroll reset shows.
    fn replace_items(
        &self,
        kind: CleanupKind,
        entries: Vec<glib::BoxedAnyObject>,
        largest: u64,
        empty: &str,
    ) {
        self.query.borrow_mut().clear();
        self.search.set_text("");
        let next = 1 - self.shown.get();
        let page = &self.pages[next];
        set_tone(&page.root, kind);
        page.empty.set_text(empty);
        page.largest.set(largest);
        page.store.splice(0, page.store.n_items(), &entries);
        page.scroll.vadjustment().set_value(0.0);
        self.shown.set(next);
        self.body.set_visible_child(&page.root);
        self.state.set_scroll_adjustment(page.scroll.vadjustment());
        if !self.body.is_transition_running() {
            release_hidden(&self.body, &self.pages);
        }
    }

    /// Titles the page with `kind`'s name in its tone.
    fn set_category(&self, kind: CleanupKind) {
        self.title.set_text(category_title(kind));
        set_tone(&self.title.root, kind);
    }
}

/// Empties every page but the one on show.
fn release_hidden(body: &gtk::Stack, pages: &[Page; 2]) {
    let shown = body.visible_child();
    for page in pages {
        if shown.as_ref() != Some(page.root.upcast_ref::<gtk::Widget>()) {
            page.store.remove_all();
        }
    }
}

/// Tints `widget` and its descendants, such as size bars, in `kind`'s tone.
fn set_tone(widget: &impl IsA<gtk::Widget>, kind: CleanupKind) {
    for other in CleanupKind::ALL {
        widget.remove_css_class(other.tone());
    }
    widget.add_css_class(kind.tone());
}

/// Builds an empty list row: name, detail, size bar, and size columns.
fn item_row() -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    root.add_css_class("cleanup-item-row");
    let detail = column_label("", "cleanup-item-detail");
    detail.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let bar = gtk::ProgressBar::new();
    bar.add_css_class("cleanup-item-bar");
    bar.set_size_request(BAR_WIDTH, -1);
    bar.set_valign(gtk::Align::Center);
    let size = label("", &["cleanup-item-size"], 1.0);
    size.set_width_chars(SIZE_CHARS);
    root.append(&column_label("", "cleanup-item-name"));
    root.append(&detail);
    root.append(&bar);
    root.append(&size);
    root
}

/// The widgets of one `item_row`, recovered from its root box on every bind.
struct ItemRow {
    name: gtk::Label,
    detail: gtk::Label,
    bar: gtk::ProgressBar,
    size: gtk::Label,
}

impl ItemRow {
    fn from_root(root: &gtk::Box) -> Self {
        let child = |widget: Option<gtk::Widget>| widget.expect("cleanup item row child");
        let name = child(root.first_child());
        let detail = child(name.next_sibling());
        let bar = child(detail.next_sibling());
        let size = child(bar.next_sibling());
        Self {
            name: name.downcast().expect("item name label"),
            detail: detail.downcast().expect("item detail label"),
            bar: bar.downcast().expect("item size bar"),
            size: size.downcast().expect("item size label"),
        }
    }

    fn bind(&self, item: &CleanupItem, largest: u64) {
        self.name.set_text(&item.label);
        self.name.set_tooltip_text(Some(&item.label));
        self.detail.set_text(&item.detail);
        self.detail.set_tooltip_text(Some(&item.detail));
        match item.bytes {
            Some(bytes) => {
                self.size.set_text(&format_size(bytes));
                self.size.remove_css_class("pending");
            }
            None => {
                self.size.set_text("On cleanup");
                self.size.add_css_class("pending");
            }
        }
        // Hidden by opacity, not visibility, so every row keeps the same column positions.
        let fraction = item
            .bytes
            .filter(|_| largest > 0)
            .map_or(0.0, |bytes| (bytes as f64 / largest as f64).clamp(0.0, 1.0));
        self.bar.set_fraction(fraction);
        self.bar.set_opacity(if fraction > 0.0 { 1.0 } else { 0.0 });
    }
}

/// A label that shares its row's spare width equally with the other expanding columns.
fn column_label(text: &str, class: &str) -> gtk::Label {
    let column = label(text, &[class], 0.0);
    column.set_hexpand(true);
    column.set_width_chars(1);
    column.set_max_width_chars(1);
    column
}
