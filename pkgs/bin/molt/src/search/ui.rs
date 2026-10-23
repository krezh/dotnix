use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool, mpsc},
    time::Duration,
};

use gtk::{gdk, glib, prelude::*};
use serde_json::Value;

use super::local::{self, LocalValue};
use super::model::{PAGE_SIZE, RESULT_WINDOW, SearchResultItem, SearchResults, SearchScope};
use crate::cache;
use crate::config::Config;
use crate::report::ui::changelog::{append_markdown_block, markdown_blocks};
use crate::ui::{
    FadeScroll, MorphLabel, SegmentedControl, action_close_button, clear, field, fit_screen,
    key_hints, label, replace_screen, theme, title,
};
use crate::{app::state::UiState, build::ui::failure::copy_button, home::ui::back_to_home_button};

const SCOPE_LABELS: [&str; 4] = ["All", "Packages", "NixOS", "Home Manager"];
const LAST_SEARCH: &str = "search-last";
const COPIED_FOR: Duration = Duration::from_millis(1500);

enum LocalState {
    Running,
    Done(Result<LocalValue, String>),
}

pub(super) struct SearchView {
    pub entry: gtk::SearchEntry,
    pub scopes: SegmentedControl,
    pub previous: gtk::Button,
    pub next: gtk::Button,
    pub retry: gtk::Button,
    /// Set when the view is left; cancels the search worker and local evaluations.
    pub lifetime: Arc<AtomicBool>,
    /// Set while the view changes the scope itself, so the toggle does not search again.
    pub syncing: Cell<bool>,
    list: gtk::ListBox,
    detail: gtk::Box,
    detail_scroll: gtk::ScrolledWindow,
    list_scroll: gtk::ScrolledWindow,
    status: MorphLabel,
    spinner: gtk::Spinner,
    notice: gtk::Label,
    results: RefCell<Option<Rc<SearchResults>>>,
    query: RefCell<String>,
    config: Option<Config>,
    /// Local evaluations by result, kept while the view lives so revisits show them.
    local: RefCell<HashMap<String, LocalState>>,
    /// The inspector's local section and the result it belongs to.
    local_slot: RefCell<Option<(String, gtk::Box)>>,
}

impl SearchView {
    fn new(
        window: &gtk::ApplicationWindow,
        root: &gtk::Box,
        state: &Rc<UiState>,
        config: Option<Config>,
    ) -> Rc<Self> {
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        header.add_css_class("search-header");
        let heading = gtk::Box::new(gtk::Orientation::Vertical, 2);
        heading.set_hexpand(true);
        heading.append(&title("Search"));
        heading.append(&label(
            "nixos-unstable · search.nixos.org",
            &["search-catalog"],
            0.0,
        ));
        heading.append(&label(
            "Catalog results may differ from your locked inputs; check a result locally to see yours",
            &["search-muted"],
            0.0,
        ));
        header.append(&heading);
        root.append(&header);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some(
            "Search packages and options · p: o: hm: to pick a category",
        ));
        entry.set_hexpand(true);
        entry.add_css_class("search-entry");
        field(&entry);
        root.append(&entry);

        let scope_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        scope_row.add_css_class("search-scope-row");
        let scopes = SegmentedControl::new(&SCOPE_LABELS);
        scope_row.append(&scopes.root);
        root.append(&scope_row);
        let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);
        status_row.append(&spinner);
        let status = MorphLabel::new(
            "Search packages and configuration options",
            &["search-muted"],
            0.0,
        );
        status.root.set_hexpand(true);
        status_row.append(&status.root);
        let retry = button("Retry");
        retry.set_visible(false);
        status_row.append(&retry);
        root.append(&status_row);

        let content = gtk::Grid::new();
        content.set_column_homogeneous(true);
        content.set_column_spacing(14);
        content.set_vexpand(true);
        content.add_css_class("search-content");
        let browser = gtk::Box::new(gtk::Orientation::Vertical, 8);
        browser.add_css_class("phase-panel");
        browser.add_css_class("search-browser");
        let list = gtk::ListBox::new();
        list.add_css_class("see-through");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        let list_scroll = scroller(&list);
        state.set_scroll_adjustment(list_scroll.vadjustment());
        browser.append(&FadeScroll::new(&list_scroll));
        let notice = wrapped("", &["search-muted"], false);
        notice.set_visible(false);
        browser.append(&notice);
        content.attach(&browser, 0, 0, 3, 1);
        let detail = gtk::Box::new(gtk::Orientation::Vertical, 12);
        detail.add_css_class("search-detail");
        let detail_scroll = scroller(&detail);
        let inspector = gtk::Box::new(gtk::Orientation::Vertical, 0);
        inspector.add_css_class("phase-panel");
        inspector.append(&FadeScroll::new(&detail_scroll));
        content.attach(&inspector, 3, 0, 4, 1);
        root.append(&content);

        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        footer.add_css_class("report-footer");
        footer.append(&back_to_home_button(window, root, state));
        footer.append(&action_close_button(
            window,
            Rc::clone(state),
            "report-close-button",
            "report-close-label",
        ));
        if state.appearance.keybinds {
            let hints = key_hints(&[
                ("↑↓", "Select"),
                ("Enter", "Copy snippet"),
                ("PgUp/PgDn", "Page"),
                ("Esc", "Close"),
            ]);
            hints.set_hexpand(true);
            hints.set_halign(gtk::Align::Center);
            hints.set_valign(gtk::Align::Center);
            footer.append(&hints);
        } else {
            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);
            footer.append(&spacer);
        }
        let previous = button("Previous");
        let next = button("Next");
        footer.append(&previous);
        footer.append(&next);
        root.append(&footer);

        let view = Rc::new(Self {
            entry,
            scopes,
            previous,
            next,
            retry,
            lifetime: Arc::new(AtomicBool::new(false)),
            syncing: Cell::new(false),
            list,
            detail,
            detail_scroll,
            list_scroll,
            status,
            spinner,
            notice,
            results: RefCell::new(None),
            query: RefCell::new(String::new()),
            config,
            local: RefCell::new(HashMap::new()),
            local_slot: RefCell::new(None),
        });
        let weak = Rc::downgrade(&view);
        view.list.connect_row_selected(move |_, row| {
            if let Some(view) = weak.upgrade() {
                clear(&view.detail);
                view.local_slot.take();
                view.detail_scroll.vadjustment().set_value(0.0);
                let item = row.and_then(|row| {
                    view.results
                        .borrow()
                        .as_ref()?
                        .items
                        .get(row.index() as usize)
                        .cloned()
                });
                if let Some(item) = item {
                    view.show_detail(&item);
                }
            }
        });
        let weak = Rc::downgrade(&view);
        view.entry.connect_activate(move |_| {
            if let Some(view) = weak.upgrade() {
                view.copy_snippet();
            }
        });
        // The window's shortcuts leave Page Up and Page Down to a focused text field.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&view);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(view) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let button = match key {
                gdk::Key::Page_Up => &view.previous,
                gdk::Key::Page_Down => &view.next,
                _ => return glib::Propagation::Proceed,
            };
            if button.is_sensitive() {
                button.emit_clicked();
            }
            glib::Propagation::Stop
        });
        view.entry.add_controller(keys);
        let weak = Rc::downgrade(&view);
        state.set_list_step(move |delta| weak.upgrade().is_some_and(|view| view.step(delta)));
        view.reset(false);
        view
    }

    pub(super) fn scope(&self) -> SearchScope {
        self.scopes
            .options
            .iter()
            .position(gtk::prelude::ToggleButtonExt::is_active)
            .map_or(SearchScope::All, |index| SearchScope::ALL[index])
    }

    pub(super) fn select_scope(&self, scope: SearchScope) {
        self.syncing.set(true);
        self.scopes.options[scope.index()].set_active(true);
        self.syncing.set(false);
    }

    /// Shows each category's total next to its name once the catalog has reported it.
    pub(super) fn set_counts(&self, counts: [Option<usize>; 4]) {
        for (index, (name, count)) in SCOPE_LABELS.iter().zip(counts).enumerate() {
            let text = count.map_or_else(
                || (*name).to_owned(),
                |count| format!("{name} · {}", compact_count(count)),
            );
            self.scopes.set_label(index, &text);
        }
    }

    pub(super) fn reset(&self, loading: bool) {
        self.results.take();
        self.local_slot.take();
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        clear(&self.detail);
        self.detail.append(&wrapped(
            "Select a result to inspect",
            &["search-muted"],
            false,
        ));
        self.previous.set_sensitive(false);
        self.next.set_sensitive(false);
        self.retry.set_visible(false);
        self.notice.set_visible(false);
        self.spinner.set_visible(loading);
        self.spinner.set_spinning(loading);
        self.status.set_text(if loading {
            "Searching…"
        } else {
            "Search packages and configuration options"
        });
    }

    pub(super) fn failure(&self, error: &str) {
        self.reset(false);
        self.status.set_text("Search failed");
        self.notice.set_text(error);
        self.notice.add_css_class("error");
        self.notice.set_visible(true);
        self.retry.set_visible(true);
    }

    pub(super) fn present(&self, query: &str, offset: usize, results: Rc<SearchResults>) {
        self.reset(false);
        self.notice.remove_css_class("error");
        if results.total == 0 {
            self.status.set_text(&format!("No results for ‘{query}’"));
            return;
        }
        self.status.set_text(&format!(
            "Showing {}–{} of {} results · {} ms",
            offset + 1,
            offset + results.items.len(),
            results.total,
            results.elapsed.as_millis()
        ));
        self.previous.set_sensitive(offset > 0);
        self.next
            .set_sensitive(offset + PAGE_SIZE < results.total.min(RESULT_WINDOW));
        if results.total > RESULT_WINDOW && offset + PAGE_SIZE >= RESULT_WINDOW {
            self.notice
                .set_text("Refine your query to see more than 10,000 results");
            self.notice.set_visible(true);
        }
        self.query.replace(query.to_owned());
        self.results.replace(Some(Rc::clone(&results)));
        for item in &results.items {
            self.list.append(&result_row(item, query));
        }
        self.list_scroll.vadjustment().set_value(0.0);
        self.list.select_row(self.list.row_at_index(0).as_ref());
    }

    /// Moves the selection by `delta` rows and keeps it in view; false when there is no list.
    fn step(&self, delta: i32) -> bool {
        let count = self
            .results
            .borrow()
            .as_ref()
            .map_or(0, |results| results.items.len());
        if count == 0 {
            return false;
        }
        let current = self.list.selected_row().map_or(-1, |row| row.index());
        let index = (current + delta).clamp(0, count as i32 - 1);
        let Some(row) = self.list.row_at_index(index) else {
            return false;
        };
        self.list.select_row(Some(&row));
        if let Some(bounds) = row.compute_bounds(&self.list) {
            let top = f64::from(bounds.y());
            self.list_scroll
                .vadjustment()
                .clamp_page(top, top + f64::from(bounds.height()));
        }
        true
    }

    fn selected_item(&self) -> Option<SearchResultItem> {
        let row = self.list.selected_row()?;
        self.results
            .borrow()
            .as_ref()?
            .items
            .get(row.index() as usize)
            .cloned()
    }

    fn copy_snippet(&self) {
        let Some(item) = self.selected_item() else {
            return;
        };
        let snippet = item.snippet();
        self.entry.clipboard().set_text(&snippet);
        let previous = self.status.text().to_string();
        let first_line = snippet.lines().next().unwrap_or_default();
        let copied = format!("Copied {first_line}");
        self.status.set_text(&copied);
        let status = self.status.downgrade();
        glib::timeout_add_local_once(COPIED_FOR, move || {
            if let Some(status) = status.upgrade()
                && status.text() == copied
            {
                status.set_text(&previous);
            }
        });
    }

    fn show_detail(self: &Rc<Self>, item: &SearchResultItem) {
        let container = &self.detail;
        let (name, category, _, tone) = identity(item);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let badge = label(category, &["chip", "tinted", theme::tone(tone)], 0.5);
        badge.set_hexpand(true);
        badge.set_halign(gtk::Align::Start);
        header.append(&badge);
        let text = name.to_owned();
        let copy = copy_button(
            if matches!(item, SearchResultItem::Package(_)) {
                "Copy attribute"
            } else {
                "Copy option"
            },
            move || text.clone(),
        );
        let snippet = item.snippet();
        let copy_snippet = copy_button("Copy snippet", move || snippet.clone());
        copy_snippet.set_tooltip_text(Some(&item.snippet()));
        for copy in [&copy, &copy_snippet] {
            copy.remove_css_class("build-failure-button");
            copy.add_css_class("search-button");
            header.append(copy);
        }
        container.append(&header);
        container.append(&wrapped(name, &["search-detail-name"], true));
        match item {
            SearchResultItem::Package(hit) => {
                text_section(container, "VERSION", hit.version.as_deref());
                description_section(container, hit.description.as_deref());
                if !hit.licenses.is_empty() {
                    text_section(container, "LICENSES", Some(&hit.licenses.join(", ")));
                }
                if !hit.programs.is_empty() {
                    text_section(
                        container,
                        "PROVIDED PROGRAMS",
                        Some(&hit.programs.join(", ")),
                    );
                }
                text_section(container, "DECLARATION", hit.position.as_deref());
            }
            SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
                description_section(container, hit.description.as_deref());
                text_section(container, "TYPE", hit.option_type.as_deref());
                value_section(container, "DEFAULT", &hit.default_value);
                value_section(container, "EXAMPLE", &hit.example);
                text_section(container, "SOURCE", hit.source.as_deref());
            }
        }
        let links = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        container.append(&links);
        let revision = self
            .results
            .borrow()
            .as_ref()
            .and_then(|results| results.revision.clone());
        if let Some(url) = item.source_url(revision.as_deref()) {
            link_button(&links, container, "View source", url);
        }
        if let SearchResultItem::Package(hit) = item
            && let Some(url) = hit.homepage_url()
        {
            link_button(&links, container, "Open homepage", url.to_owned());
        }
        if links.first_child().is_none() {
            container.remove(&links);
        }
        if self.config.is_some() {
            let slot = section(container, "IN YOUR FLAKE");
            let key = local_key(item);
            self.local_slot.replace(Some((key.clone(), slot)));
            self.render_local(&key, item);
        }
    }

    /// Fills the inspector's local section for `item` if it is still the one shown.
    fn render_local(self: &Rc<Self>, key: &str, item: &SearchResultItem) {
        let Some(slot) = self
            .local_slot
            .borrow()
            .as_ref()
            .filter(|(shown, _)| shown == key)
            .map(|(_, slot)| slot.clone())
        else {
            return;
        };
        while let Some(child) = slot.first_child().and_then(|caption| caption.next_sibling()) {
            slot.remove(&child);
        }
        let check = |caption| {
            let check = button(caption);
            check.set_halign(gtk::Align::Start);
            let weak = Rc::downgrade(self);
            let item = item.clone();
            check.connect_clicked(move |_| {
                if let Some(view) = weak.upgrade() {
                    view.check_locally(item.clone());
                }
            });
            check
        };
        match self.local.borrow().get(key) {
            None => {
                slot.append(&wrapped(
                    "Evaluate this against your flake's locked nixpkgs and configuration",
                    &["search-muted"],
                    false,
                ));
                slot.append(&check("Check locally"));
            }
            Some(LocalState::Running) => {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let spinner = gtk::Spinner::new();
                spinner.start();
                row.append(&spinner);
                row.append(&wrapped("Evaluating with nix eval…", &["search-muted"], false));
                slot.append(&row);
            }
            Some(LocalState::Done(Ok(value))) => {
                let summary = match (item, value.set) {
                    (SearchResultItem::Package(_), _) => "Version in your nixpkgs",
                    (_, Some(true)) => "Set in your configuration",
                    (_, Some(false)) => "Not set; the module default applies",
                    (_, None) => "Value in your configuration",
                };
                slot.append(&wrapped(summary, &["search-text"], false));
                slot.append(&wrapped(&value.value, &["search-value"], true));
                slot.append(&wrapped(&value.attribute, &["search-muted"], true));
                slot.append(&check("Check again"));
            }
            Some(LocalState::Done(Err(error))) => {
                slot.append(&wrapped(error, &["error"], true));
                slot.append(&check("Retry"));
            }
        }
    }

    fn check_locally(self: &Rc<Self>, item: SearchResultItem) {
        let Some(config) = self.config.clone() else {
            return;
        };
        let key = local_key(&item);
        if matches!(self.local.borrow().get(&key), Some(LocalState::Running)) {
            return;
        }
        self.local.borrow_mut().insert(key.clone(), LocalState::Running);
        self.render_local(&key, &item);
        let (sender, receiver) = mpsc::channel();
        let cancellation = Arc::clone(&self.lifetime);
        let worker_item = item.clone();
        std::thread::spawn(move || {
            let _ = sender.send(local::evaluate(&config, &worker_item, &cancellation));
        });
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let Some(view) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err("The evaluation stopped without a result".into())
                }
            };
            view.local
                .borrow_mut()
                .insert(key.clone(), LocalState::Done(result));
            view.render_local(&key, &item);
            glib::ControlFlow::Break
        });
    }
}

pub(crate) fn show_search(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Option<Config>,
) {
    state.clear_actions();
    replace_screen(root);
    root.remove_css_class("home-root");
    fit_screen(window, &state);
    let view = SearchView::new(window, root, &state, config);
    view.entry.grab_focus();
    let controller = super::controller::Controller::connect(Rc::clone(&view), &state);
    if let Some(owner) = root.last_child() {
        owner.connect_destroy(move |_| controller.stop());
    }
    if let Some((scope, query)) = cache::read(LAST_SEARCH).as_deref().and_then(parse_last_search)
    {
        view.scopes.options[scope.index()].set_active(true);
        view.entry.set_text(query);
        view.entry.select_region(0, -1);
    }
}

/// Remembers the search to restore when Search is next opened.
pub(super) fn save_last_search(scope: SearchScope, query: &str) {
    cache::write(LAST_SEARCH, &format!("{}\n{}\n", scope.index(), query.trim()));
}

fn parse_last_search(text: &str) -> Option<(SearchScope, &str)> {
    let (scope, query) = text.split_once('\n')?;
    let scope = *SearchScope::ALL.get(scope.parse::<usize>().ok()?)?;
    let query = query.trim();
    (!query.is_empty()).then_some((scope, query))
}

fn local_key(item: &SearchResultItem) -> String {
    format!("{}:{}", identity(item).1, item.name())
}

fn compact_count(count: usize) -> String {
    if count < 1000 {
        count.to_string()
    } else {
        format!("{:.1}k", count as f64 / 1000.0)
    }
}

fn button(caption: &str) -> gtk::Button {
    let button = gtk::Button::with_label(caption);
    button.add_css_class("surface-button");
    button.add_css_class("search-button");
    button
}

/// Adds a button to `row` that opens `url`, with its error shown at the end of `container`.
fn link_button(row: &gtk::Box, container: &gtk::Box, caption: &str, url: String) {
    let open = button(caption);
    open.set_tooltip_text(Some(&url));
    let error = wrapped("", &["error"], false);
    error.set_visible(false);
    let message = error.clone();
    let caption = caption.to_owned();
    open.connect_clicked(move |_| {
        match gio::AppInfo::launch_default_for_uri(&url, None::<&gio::AppLaunchContext>) {
            Ok(()) => message.set_visible(false),
            Err(error) => {
                message.set_text(&format!("Could not {}: {error}", caption.to_lowercase()));
                message.set_visible(true);
            }
        }
    });
    row.append(&open);
    container.append(&error);
}

fn scroller(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.add_css_class("see-through");
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_child(Some(child));
    scroll
}

fn wrapped(text: &str, classes: &[&str], selectable: bool) -> gtk::Label {
    let value = label(text, classes, 0.0);
    value.set_ellipsize(gtk::pango::EllipsizeMode::None);
    value.set_wrap(true);
    value.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    value.set_selectable(selectable);
    value
}

fn identity(item: &SearchResultItem) -> (&str, &'static str, &'static str, theme::Color) {
    match item {
        SearchResultItem::Package(hit) => (
            &hit.attr_name,
            "Package",
            "package-x-generic-symbolic",
            theme::GREEN,
        ),
        SearchResultItem::NixOsOption(hit) => (
            &hit.name,
            "NixOS option",
            "preferences-system-symbolic",
            theme::ACCENT,
        ),
        SearchResultItem::HomeManagerOption(hit) => (
            &hit.name,
            "Home Manager option",
            "user-home-symbolic",
            theme::PEACH,
        ),
    }
}

/// Pango markup for `name` with every occurrence of a query word underlined, ignoring ASCII case.
fn highlighted(name: &str, query: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let mut marked = vec![false; name.len()];
    for word in query
        .split(|character: char| character.is_whitespace() || character == '.')
        .filter(|word| !word.is_empty())
    {
        let word = word.to_ascii_lowercase();
        let mut from = 0;
        while let Some(found) = lower[from..].find(&word) {
            let start = from + found;
            marked[start..start + word.len()].fill(true);
            from = start + word.len();
        }
    }
    let mut markup = String::with_capacity(name.len() + 32);
    let mut open = false;
    for (index, character) in name.char_indices() {
        if marked[index] != open {
            markup.push_str(if open { "</span>" } else { "<span underline=\"single\">" });
            open = marked[index];
        }
        markup.push_str(&glib::markup_escape_text(character.encode_utf8(&mut [0; 4])));
    }
    if open {
        markup.push_str("</span>");
    }
    markup
}

fn result_row(item: &SearchResultItem, query: &str) -> gtk::ListBoxRow {
    let (name, category, icon, tone) = identity(item);
    let row = gtk::ListBoxRow::new();
    row.add_css_class("list-row");
    row.add_css_class("search-row");
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let image = gtk::Image::from_icon_name(icon);
    image.add_css_class("toned");
    image.add_css_class(theme::tone(tone));
    body.append(&image);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    let title = wrapped("", &["search-name"], false);
    title.set_markup(&highlighted(name, query));
    text.append(&title);
    let metadata = match item {
        SearchResultItem::Package(hit) => hit.version.as_deref().unwrap_or(category).to_owned(),
        SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
            hit.option_type.as_deref().map_or_else(
                || category.to_owned(),
                |kind| format!("{category} · {kind}"),
            )
        }
    };
    text.append(&wrapped(&metadata, &["search-muted"], false));
    body.append(&text);
    row.set_child(Some(&body));
    row
}

fn section(container: &gtk::Box, caption: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
    section.append(&label(caption, &["panel-eyebrow"], 0.0));
    container.append(&section);
    section
}

fn text_section(container: &gtk::Box, caption: &str, text: Option<&str>) {
    if let Some(text) = text.filter(|text| !text.is_empty()) {
        section(container, caption).append(&wrapped(text, &["search-text"], true));
    }
}

fn rendered_html(text: &str) -> Option<&str> {
    text.trim()
        .strip_prefix("<rendered-html>")?
        .strip_suffix("</rendered-html>")
}

fn append_description(container: &gtk::Box, text: &str) {
    for block in markdown_blocks(rendered_html(text).unwrap_or(text)) {
        append_markdown_block(container, block, None);
    }
}

fn description_section(container: &gtk::Box, text: Option<&str>) {
    if let Some(text) = text.filter(|text| !text.is_empty()) {
        append_description(&section(container, "DESCRIPTION"), text);
    }
}

fn value_section(container: &gtk::Box, caption: &str, value: &Value) {
    let section = section(container, caption);
    match value {
        Value::Null => {
            section.append(&wrapped("Not provided", &["search-muted"], true));
        }
        Value::String(text) if rendered_html(text).is_some() => append_description(&section, text),
        Value::String(text) => {
            section.append(&wrapped(text, &["search-value"], true));
        }
        value => {
            section.append(&wrapped(&value.to_string(), &["search-value"], true));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_every_query_word_and_escapes_markup() {
        assert_eq!(
            highlighted("services.nginx.virtualHosts", "nginx VIRT"),
            "services.<span underline=\"single\">nginx</span>.<span underline=\"single\">virt</span>ualHosts"
        );
        assert_eq!(highlighted("a<b>", "b"), "a&lt;<span underline=\"single\">b</span>&gt;");
        assert_eq!(highlighted("ripgrep", ""), "ripgrep");
        assert_eq!(highlighted("aaa", "aa"), "<span underline=\"single\">aa</span>a");
    }

    #[test]
    fn last_search_round_trips_and_ignores_damage() {
        assert_eq!(
            parse_last_search("1\nripgrep\n"),
            Some((SearchScope::Packages, "ripgrep"))
        );
        assert_eq!(parse_last_search("3\n  programs.git \n").unwrap().1, "programs.git");
        for damaged in ["", "9\nx\n", "x\nquery\n", "0\n\n", "0"] {
            assert_eq!(parse_last_search(damaged), None, "{damaged:?}");
        }
        assert_eq!(compact_count(999), "999");
        assert_eq!(compact_count(1946), "1.9k");
    }
}
