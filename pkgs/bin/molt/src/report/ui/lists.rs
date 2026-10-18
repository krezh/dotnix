//! The report's change list: packages and configuration files, with status tabs and a filter.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::app::state::{Operation, UiState};
use crate::report::config_diff::{ConfigChange, ConfigContent};
use crate::report::{Change, ChangeStatus};
use crate::ui::theme;
use crate::ui::{
    FadeScroll, MorphLabel, SegmentedControl, fade_in, field, glide, label, signed_size,
};

use super::ReportPosition;
use super::side::revealed_text;

/// Section and tab order.
const SECTION_ORDER: [ChangeStatus; 5] = [
    ChangeStatus::Upgraded,
    ChangeStatus::Downgraded,
    ChangeStatus::Added,
    ChangeStatus::Removed,
    ChangeStatus::Changed,
];
/// Statuses whose tab is shown only when it has entries.
const OPTIONAL_SECTIONS: [ChangeStatus; 1] = [ChangeStatus::Downgraded];

/// The list tab and filter text; `counts` holds the matching rows per `ChangeStatus`.
#[derive(Default)]
struct ListFilter {
    status: Cell<Option<ChangeStatus>>,
    query: RefCell<String>,
    counts: Cell<[usize; 5]>,
}

impl ListFilter {
    fn matches(&self, status: ChangeStatus, lowercase_name: &str) -> bool {
        self.status.get().is_none_or(|tab| tab == status)
            && lowercase_name.contains(self.query.borrow().as_str())
    }
}

/// A row of the change list: a package from dix, or a generated configuration file.
pub(super) enum Entry {
    Package(Change),
    File(ConfigChange),
}

impl Entry {
    fn status(&self) -> ChangeStatus {
        match self {
            Self::Package(change) => change.status,
            Self::File(file) => file.status,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Package(change) => &change.name,
            Self::File(file) => &file.path,
        }
    }

    /// Whether the entry has a detail screen: release notes, or the file's diff.
    fn can_open(&self) -> bool {
        match self {
            Self::Package(change) => has_changelog(change),
            Self::File(_) => true,
        }
    }

    fn row(&self, index: usize, open: &OpenRow) -> gtk::ListBoxRow {
        match self {
            Self::Package(change) => package_row(change, index, open),
            Self::File(file) => config_row(file, index, open),
        }
    }
}

/// Opens the detail screen of the entry at an index, if it has one.
type OpenRow = Rc<dyn Fn(usize)>;

/// The change list's status tabs and filter (`controls`) and its rows (`body`).
pub(super) struct ChangeList {
    pub(super) controls: gtk::Box,
    pub(super) body: gtk::Box,
    pub(super) adjustment: Option<gtk::Adjustment>,
    pub(super) snapshot: Rc<dyn Fn() -> ReportPosition>,
}

/// - `empty`: shown instead of the list when there are no entries.
/// - `footer`: appended below the list, or below the empty notice.
/// - `open`: navigates to an entry's detail screen.
pub(super) fn change_list(
    mut entries: Vec<Entry>,
    empty: &str,
    footer: Option<gtk::Widget>,
    open: Rc<dyn Fn(&Entry)>,
    state: &Rc<UiState>,
    restore: Option<&ReportPosition>,
) -> ChangeList {
    let count = entries.len();
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    controls.set_hexpand(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.set_vexpand(true);
    if entries.is_empty() {
        let empty = label(empty, &["report-empty"], 0.5);
        empty.set_ellipsize(gtk::pango::EllipsizeMode::None);
        empty.set_wrap(true);
        empty.set_justify(gtk::Justification::Center);
        empty.set_vexpand(true);
        empty.set_valign(gtk::Align::Center);
        body.append(&empty);
        if let Some(footer) = &footer {
            body.append(footer);
        }
        return ChangeList {
            controls,
            body,
            adjustment: None,
            snapshot: Rc::new(ReportPosition::default),
        };
    }

    entries.sort_by_key(|entry| section_rank(&SECTION_ORDER, entry.status()));
    let entries = Rc::new(entries);
    let names = Rc::new(
        entries
            .iter()
            .map(|entry| entry.name().to_lowercase())
            .collect::<Vec<_>>(),
    );
    let totals = status_counts(entries.iter().map(Entry::status));
    let filter = Rc::new(ListFilter {
        counts: Cell::new(totals),
        ..ListFilter::default()
    });

    let tab_statuses = std::iter::once(None)
        .chain(
            SECTION_ORDER
                .iter()
                .copied()
                .filter(|status| {
                    !OPTIONAL_SECTIONS.contains(status) || totals[*status as usize] > 0
                })
                .map(Some),
        )
        .collect::<Vec<_>>();
    let tab_labels = tab_statuses
        .iter()
        .map(|status| match status {
            None => format!("All {count}"),
            Some(status) => format!("{} {}", status.label(), totals[*status as usize]),
        })
        .collect::<Vec<_>>();
    let tabs = SegmentedControl::new(&tab_labels.iter().map(String::as_str).collect::<Vec<_>>());
    tabs.root.set_valign(gtk::Align::Center);
    controls.append(&tabs.root);
    let search = gtk::SearchEntry::new();
    search.add_css_class("report-filter");
    field(&search);
    search.set_placeholder_text(Some("Filter packages and files"));
    search.set_width_chars(12);
    search.set_hexpand(true);
    search.set_halign(gtk::Align::End);
    search.set_valign(gtk::Align::Center);
    controls.append(&search);

    body.append(&columns_header());
    let list = gtk::ListBox::new();
    list.add_css_class("see-through");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.set_activate_on_single_click(false);
    let placeholder = label("Nothing matches the filter", &["report-empty"], 0.5);
    list.set_placeholder(Some(&placeholder));
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("see-through");
    scroller.set_vexpand(true);
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_child(Some(&list));
    let adjustment = scroller.vadjustment();
    body.append(&FadeScroll::new(&scroller));
    if let Some(footer) = &footer {
        body.append(footer);
    }

    let open: OpenRow = {
        let state = Rc::clone(state);
        let entries = Rc::clone(&entries);
        Rc::new(move |index: usize| {
            if let Some(entry) = entries.get(index)
                && state.operation.get() == Operation::Idle
                && entry.can_open()
            {
                open(entry);
            }
        })
    };
    for (index, entry) in entries.iter().enumerate() {
        list.append(&entry.row(index, &open));
    }
    list.connect_row_activated(move |_, row| {
        if let Ok(index) = usize::try_from(row.index()) {
            open(index);
        }
    });

    let filter_entries = Rc::clone(&entries);
    let filter_names = Rc::clone(&names);
    let row_filter = Rc::clone(&filter);
    list.set_filter_func(move |row| {
        usize::try_from(row.index())
            .ok()
            .and_then(|index| filter_entries.get(index).zip(filter_names.get(index)))
            .is_some_and(|(entry, name)| row_filter.matches(entry.status(), name))
    });
    // One header per section, in `SECTION_ORDER`, kept across filtering so its count morphs.
    let headers =
        Rc::new(SECTION_ORDER.map(|status| section_header(status, totals[status as usize])));
    // The row each header was last given to.
    let header_rows: RefCell<[Option<gtk::ListBoxRow>; 5]> = RefCell::default();
    let header_entries = Rc::clone(&entries);
    let header_widgets = Rc::clone(&headers);
    list.set_header_func(move |row, before| {
        let status_of = |row: &gtk::ListBoxRow| {
            usize::try_from(row.index())
                .ok()
                .and_then(|index| header_entries.get(index))
                .map(Entry::status)
        };
        let status = status_of(row);
        match status {
            Some(status) if before.and_then(status_of) != Some(status) => {
                let rank = section_rank(&SECTION_ORDER, status);
                let header = header_widgets[rank].root.upcast_ref::<gtk::Widget>();
                // GTK unparents a header when it reaches the hidden row still holding it, which
                // would pull it off its new row; take it off the old row first.
                if let Some(previous) = header_rows.borrow_mut()[rank].replace(row.clone())
                    && previous != *row
                    && previous.header().as_ref() == Some(header)
                {
                    previous.set_header(None::<&gtk::Widget>);
                }
                row.set_header(Some(header));
            }
            _ => row.set_header(None::<&gtk::Widget>),
        }
    });

    let refresh = {
        let list = list.clone();
        let filter = Rc::clone(&filter);
        let shown = RefCell::new(vec![true; entries.len()]);
        move || {
            let mut counts = [0; 5];
            let mut appearing = Vec::new();
            let mut shown = shown.borrow_mut();
            for (index, (entry, name)) in entries.iter().zip(names.iter()).enumerate() {
                let matches = filter.matches(entry.status(), name);
                if matches {
                    counts[entry.status() as usize] += 1;
                    if !shown[index] {
                        appearing.push(index);
                    }
                }
                shown[index] = matches;
            }
            let before = filter.counts.replace(counts);
            for (header, status) in headers.iter().zip(SECTION_ORDER) {
                header.set_text(&section_title(status, counts[status as usize]));
            }
            list.invalidate_filter();
            list.invalidate_headers();
            // What the filter brings back fades in; what it hides goes at once, so typing stays
            // immediate.
            for index in appearing {
                if let Some(row) = list.row_at_index(index as i32) {
                    fade_in(&row);
                }
            }
            for (header, status) in headers.iter().zip(SECTION_ORDER) {
                if before[status as usize] == 0 && counts[status as usize] > 0 {
                    fade_in(&header.root);
                }
            }
            if before.iter().sum::<usize>() > 0 && counts.iter().sum::<usize>() == 0 {
                fade_in(&placeholder);
            }
        }
    };
    let refresh = Rc::new(refresh);
    for (option, status) in tabs.options.iter().zip(tab_statuses.iter().copied()) {
        let filter = Rc::clone(&filter);
        let refresh = Rc::clone(&refresh);
        let (scroller, adjustment) = (scroller.clone(), adjustment.clone());
        option.connect_toggled(move |option| {
            if option.is_active() {
                filter.status.set(status);
                refresh();
                glide(&scroller, &adjustment, 0.0);
            }
        });
    }
    let search_filter = Rc::clone(&filter);
    let search_refresh = Rc::clone(&refresh);
    search.connect_search_changed(move |search| {
        let query = search.text().to_lowercase();
        if *search_filter.query.borrow() != query {
            *search_filter.query.borrow_mut() = query;
            search_refresh();
        }
    });

    if let Some(position) = restore {
        if let Some(index) = tab_statuses
            .iter()
            .position(|status| *status == position.tab)
        {
            tabs.options[index].set_active(true);
        }
        search.set_text(&position.filter);
        *filter.query.borrow_mut() = position.filter.to_lowercase();
        refresh();
        // Straight there: the screen is sliding back in, so a scroll glide would be a second motion.
        let adjustment = adjustment.clone();
        let scroll = position.scroll;
        glib::idle_add_local_once(move || adjustment.set_value(scroll));
    }

    let snapshot = {
        let search = search.downgrade();
        let adjustment = adjustment.downgrade();
        move || ReportPosition {
            tab: filter.status.get(),
            filter: search
                .upgrade()
                .map(|search| search.text().to_string())
                .unwrap_or_default(),
            scroll: adjustment
                .upgrade()
                .map_or(0.0, |adjustment| adjustment.value()),
        }
    };
    ChangeList {
        controls,
        body,
        adjustment: Some(adjustment),
        snapshot: Rc::new(snapshot),
    }
}

/// Name, version, and size columns at fixed 2 : 2 : 1 proportions, shared by header and rows. A
/// file's path spans name and version; its line counts take the size column.
fn columns() -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_homogeneous(true);
    grid.set_column_spacing(12);
    grid
}

fn columns_header() -> gtk::Grid {
    let grid = columns();
    grid.add_css_class("report-columns");
    grid.attach(&label("NAME", &["report-column-label"], 0.0), 0, 0, 2, 1);
    grid.attach(&label("VERSION", &["report-column-label"], 0.0), 2, 0, 2, 1);
    grid.attach(&label("CHANGE", &["report-column-label"], 1.0), 4, 0, 1, 1);
    grid
}

fn package_row(change: &Change, index: usize, open: &OpenRow) -> gtk::ListBoxRow {
    let grid = columns();
    let name = label(&change.name, &["report-package-name"], 0.0);
    name.set_tooltip_text(Some(&change.name));
    grid.attach(&name, 0, 0, 2, 1);

    let version = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let (markup, plain) = version_text(change);
    let text = label("", &["report-version"], 0.0);
    text.set_markup(&markup);
    text.set_hexpand(true);
    text.set_tooltip_text(Some(&plain));
    version.append(&text);
    if has_changelog(change) {
        version.append(&detail_link(
            "Changelog ›",
            &format!("Release notes for {}", change.name),
            index,
            open,
        ));
    }
    grid.attach(&version, 2, 0, 2, 1);
    grid.attach(&size_label(change.size), 4, 0, 1, 1);

    let row = gtk::ListBoxRow::new();
    row.add_css_class("report-row");
    row.add_css_class("list-row");
    row.set_child(Some(&grid));
    row
}

/// A section's header chip, e.g. `Upgraded · 12`.
fn section_header(status: ChangeStatus, count: usize) -> MorphLabel {
    let header = MorphLabel::new(
        &section_title(status, count),
        &[
            "phase-section-label",
            "report-section-label",
            "chip",
            "tinted",
            status.tone(),
        ],
        0.0,
    );
    header.root.set_halign(gtk::Align::Start);
    header
}

fn section_title(status: ChangeStatus, count: usize) -> String {
    format!("{} · {count}", status.label())
}

fn section_rank(sections: &[ChangeStatus], status: ChangeStatus) -> usize {
    sections
        .iter()
        .position(|candidate| *candidate == status)
        .unwrap_or(sections.len())
}

/// Entries per `ChangeStatus`, indexed by its discriminant.
fn status_counts(statuses: impl Iterator<Item = ChangeStatus>) -> [usize; 5] {
    let mut counts = [0; 5];
    for status in statuses {
        counts[status as usize] += 1;
    }
    counts
}

/// Laid out like `package_row`: path, then a muted `config file` with the diff link, then line
/// counts.
fn config_row(change: &ConfigChange, index: usize, open: &OpenRow) -> gtk::ListBoxRow {
    let grid = columns();
    let path = label(&change.path, &["report-package-name"], 0.0);
    path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    path.set_tooltip_text(Some(&change.path));
    grid.attach(&path, 0, 0, 2, 1);

    let version = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let kind = label("", &["report-version"], 0.0);
    kind.set_markup(&format!(
        "<span foreground=\"{}\">config file</span>",
        theme::OVERLAY1
    ));
    kind.set_hexpand(true);
    version.append(&kind);
    version.append(&detail_link(
        "Diff ›",
        &format!("Changes to {}", change.path),
        index,
        open,
    ));
    grid.attach(&version, 2, 0, 2, 1);
    let lines = label("", &["report-size"], 1.0);
    lines.set_markup(&line_counts(&change.content));
    lines.set_ellipsize(gtk::pango::EllipsizeMode::None);
    grid.attach(&lines, 4, 0, 1, 1);

    let row = gtk::ListBoxRow::new();
    row.add_css_class("report-row");
    row.add_css_class("list-row");
    row.set_child(Some(&grid));
    row
}

/// The row's link to its detail screen, shown on hover.
fn detail_link(text: &str, tooltip: &str, index: usize, open: &OpenRow) -> gtk::Button {
    let link = gtk::Button::with_label(text);
    link.add_css_class("report-changelog");
    link.add_css_class("text-link");
    link.set_focusable(false);
    link.set_valign(gtk::Align::Center);
    link.set_tooltip_text(Some(tooltip));
    let open = Rc::clone(open);
    link.connect_clicked(move |_| open(index));
    link
}

/// Pango markup for the lines column: `+2 −1`, or why there are no line counts.
fn line_counts(content: &ConfigContent) -> String {
    let (added_colour, removed_colour) = (theme::GREEN, theme::RED);
    match content {
        ConfigContent::Binary => "binary".to_owned(),
        ConfigContent::Text {
            added: 0,
            removed: 0,
            ..
        } => "line endings".to_owned(),
        ConfigContent::Text { added, removed, .. } => {
            let mut counts = Vec::with_capacity(2);
            if *added > 0 {
                counts.push(format!(
                    "<span foreground=\"{added_colour}\">+{added}</span>"
                ));
            }
            if *removed > 0 {
                counts.push(format!(
                    "<span foreground=\"{removed_colour}\">−{removed}</span>"
                ));
            }
            counts.join(" ")
        }
    }
}

/// `9 files differ only in store paths`, with the paths behind a toggle.
pub(super) fn store_path_footer(paths: &[String]) -> gtk::Widget {
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    footer.add_css_class("report-store-only");
    let noun = if paths.len() == 1 {
        "file differs"
    } else {
        "files differ"
    };
    footer.append(&label(
        &format!("{} {noun} only in store paths", paths.len()),
        &["report-store-only-summary"],
        0.0,
    ));
    footer.append(&revealed_text(paths, "Show files ›", "Hide files ‹"));
    footer.upcast()
}

/// A rebuild without a version change, e.g. after a dependency changed: dix reports the version
/// once, twice, or not at all.
fn is_rebuild(change: &Change) -> bool {
    change.status == ChangeStatus::Changed && (change.old.is_empty() || change.old == change.new)
}

fn has_changelog_versions(change: &Change) -> bool {
    !change.new.is_empty() && change.old != "..." && change.new != "..."
}

fn has_changelog(change: &Change) -> bool {
    has_changelog_versions(change) && !is_rebuild(change)
}

/// Pango markup for the version column, plus the same text unstyled for the tooltip. Never empty:
/// a row without versions still says why it is listed.
fn version_text(change: &Change) -> (String, String) {
    let (old_colour, muted_colour) = (theme::SUBTEXT0, theme::OVERLAY1);
    let muted = |text: &str| {
        (
            format!("<span foreground=\"{muted_colour}\">{text}</span>"),
            text.to_owned(),
        )
    };
    let old = glib::markup_escape_text(&change.old);
    let new = glib::markup_escape_text(&change.new);
    let version = theme::VERSION;
    if is_rebuild(change) {
        return if change.new.is_empty() {
            muted("rebuilt")
        } else {
            (
                format!("{new} <span foreground=\"{muted_colour}\">· rebuilt</span>"),
                format!("{} · rebuilt", change.new),
            )
        };
    }
    match (change.old.is_empty(), change.new.is_empty()) {
        (false, false) => (
            format!(
                "<span foreground=\"{old_colour}\">{old}</span> <span foreground=\"{muted_colour}\">→</span> <span foreground=\"{version}\">{new}</span>"
            ),
            format!("{} → {}", change.old, change.new),
        ),
        (true, false) => (
            format!("<span foreground=\"{version}\">{new}</span>"),
            change.new.clone(),
        ),
        (false, true) => (
            format!("<span foreground=\"{old_colour}\">{old}</span>"),
            change.old.clone(),
        ),
        (true, true) => muted("no version"),
    }
}

/// Classes for a size change: growth in red, shrinkage in green, no change plain.
pub(super) fn size_classes(base: &'static str, bytes: i64) -> Vec<&'static str> {
    match bytes.cmp(&0) {
        std::cmp::Ordering::Greater => vec![base, "toned", theme::tone(theme::RED)],
        std::cmp::Ordering::Less => vec![base, "toned", theme::tone(theme::GREEN)],
        std::cmp::Ordering::Equal => vec![base],
    }
}

fn size_label(bytes: i64) -> gtk::Label {
    let size = label(
        &signed_size(bytes),
        &size_classes("report-size", bytes),
        1.0,
    );
    size.set_ellipsize(gtk::pango::EllipsizeMode::None);
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(status: ChangeStatus, old: &str, new: &str) -> Change {
        Change {
            status,
            name: "demo".to_owned(),
            old: old.to_owned(),
            new: new.to_owned(),
            size: 0,
        }
    }

    #[test]
    fn enables_changelogs_only_for_known_target_versions() {
        let upgraded = |old, new| change(ChangeStatus::Upgraded, old, new);
        assert!(has_changelog(&upgraded("1.0", "2.0")));
        assert!(has_changelog(&change(ChangeStatus::Added, "", "2.0")));
        assert!(!has_changelog(&upgraded("1.0", "...")));
        assert!(!has_changelog(&upgraded("...", "2.0")));
        assert!(!has_changelog(&upgraded("1.0", "")));
        assert!(!has_changelog(&change(ChangeStatus::Changed, "", "0.1.0")));
        assert!(!has_changelog(&change(
            ChangeStatus::Changed,
            "0.1.0",
            "0.1.0"
        )));
    }

    #[test]
    fn describes_versions() {
        assert_eq!(
            version_text(&change(ChangeStatus::Upgraded, "1.0", "2.0")).1,
            "1.0 → 2.0"
        );
        assert_eq!(
            version_text(&change(ChangeStatus::Changed, "", "0.1.0")).1,
            "0.1.0 · rebuilt"
        );
        assert_eq!(
            version_text(&change(ChangeStatus::Changed, "0.1.0", "0.1.0")).1,
            "0.1.0 · rebuilt"
        );
        assert_eq!(
            version_text(&change(ChangeStatus::Changed, "", "")),
            (
                "<span foreground=\"#7f849c\">rebuilt</span>".to_owned(),
                "rebuilt".to_owned()
            )
        );
        assert_eq!(
            version_text(&change(ChangeStatus::Removed, "<1>", "")).0,
            "<span foreground=\"#a6adc8\">&lt;1&gt;</span>"
        );
        assert_eq!(
            version_text(&change(ChangeStatus::Added, "", "")).1,
            "no version"
        );
    }
}
