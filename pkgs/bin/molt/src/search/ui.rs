use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;
use serde_json::Value;

use super::model::{PAGE_SIZE, RESULT_WINDOW, SearchResultItem, SearchResults};
use crate::report::ui::changelog::{append_markdown_block, markdown_blocks};
use crate::ui::{
    FadeScroll, MorphLabel, SegmentedControl, action_close_button, clear, field, fit_screen, label,
    replace_screen, theme, title,
};
use crate::{app::state::UiState, build::ui::failure::copy_button, home::ui::back_to_home_button};

pub(super) struct SearchView {
    pub entry: gtk::SearchEntry,
    pub scopes: SegmentedControl,
    pub previous: gtk::Button,
    pub next: gtk::Button,
    pub retry: gtk::Button,
    list: gtk::ListBox,
    detail: gtk::Box,
    detail_scroll: gtk::ScrolledWindow,
    list_scroll: gtk::ScrolledWindow,
    status: MorphLabel,
    spinner: gtk::Spinner,
    notice: gtk::Label,
    results: RefCell<Option<Rc<SearchResults>>>,
}

impl SearchView {
    fn new(window: &gtk::ApplicationWindow, root: &gtk::Box, state: &Rc<UiState>) -> Rc<Self> {
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
            "Catalog results may differ from your locked inputs",
            &["search-muted"],
            0.0,
        ));
        header.append(&heading);
        root.append(&header);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Search packages and configuration options…"));
        entry.set_hexpand(true);
        entry.add_css_class("search-entry");
        field(&entry);
        root.append(&entry);

        let scope_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        scope_row.add_css_class("search-scope-row");
        let scopes = SegmentedControl::new(&["All", "Packages", "NixOS", "Home Manager"]);
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
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        footer.append(&spacer);
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
            list,
            detail,
            detail_scroll,
            list_scroll,
            status,
            spinner,
            notice,
            results: RefCell::new(None),
        });
        let weak = Rc::downgrade(&view);
        view.list.connect_row_selected(move |_, row| {
            if let Some(view) = weak.upgrade() {
                clear(&view.detail);
                view.detail_scroll.vadjustment().set_value(0.0);
                let results = view.results.borrow();
                if let Some(item) =
                    row.and_then(|row| results.as_ref()?.items.get(row.index() as usize))
                {
                    show_detail(&view.detail, item);
                }
            }
        });
        view.reset(false);
        view
    }

    pub(super) fn reset(&self, loading: bool) {
        self.results.take();
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
        self.results.replace(Some(Rc::clone(&results)));
        for item in &results.items {
            self.list.append(&result_row(item));
        }
        self.list_scroll.vadjustment().set_value(0.0);
        self.list.select_row(self.list.row_at_index(0).as_ref());
    }
}

pub(crate) fn show_search(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<UiState>) {
    state.clear_actions();
    replace_screen(root);
    root.remove_css_class("home-root");
    fit_screen(window, &state);
    let view = SearchView::new(window, root, &state);
    view.entry.grab_focus();
    let controller = super::controller::Controller::connect(view, &state);
    if let Some(owner) = root.last_child() {
        owner.connect_destroy(move |_| controller.stop());
    }
}

fn button(caption: &str) -> gtk::Button {
    let button = gtk::Button::with_label(caption);
    button.add_css_class("surface-button");
    button.add_css_class("search-button");
    button
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

fn result_row(item: &SearchResultItem) -> gtk::ListBoxRow {
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
    text.append(&wrapped(name, &["search-name"], false));
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

fn show_detail(container: &gtk::Box, item: &SearchResultItem) {
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
    copy.remove_css_class("build-failure-button");
    copy.add_css_class("search-button");
    header.append(&copy);
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
            if let Some(url) = hit.homepage_url() {
                let open = button("Open homepage");
                open.set_halign(gtk::Align::Start);
                let url = url.to_owned();
                let error = wrapped("", &["error"], false);
                error.set_visible(false);
                let message = error.clone();
                open.connect_clicked(move |_| {
                    match gio::AppInfo::launch_default_for_uri(&url, None::<&gio::AppLaunchContext>)
                    {
                        Ok(()) => message.set_visible(false),
                        Err(error) => {
                            message.set_text(&format!("Could not open homepage: {error}"));
                            message.set_visible(true);
                        }
                    }
                });
                container.append(&open);
                container.append(&error);
            }
        }
        SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
            description_section(container, hit.description.as_deref());
            text_section(container, "TYPE", hit.option_type.as_deref());
            value_section(container, "DEFAULT", &hit.default_value);
            value_section(container, "EXAMPLE", &hit.example);
            text_section(container, "SOURCE", hit.source.as_deref());
        }
    }
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
