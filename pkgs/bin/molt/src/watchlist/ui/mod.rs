//! Watchlist screen: two-column promotion tracker modeled directly on Molt's inputs architecture.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use super::db::{delete_pr, list_prs, open_db, save_pr};
use super::github::{fetch_pr, parse_pr_ref};
use super::{PrState, StepState, WatchedPr};
use crate::app::state::UiState;
use crate::config::Config;
use crate::home::ui::back_to_home_button;
use crate::ui::{
    FadeScroll, MorphLabel, action_close_button, centered_icon, clear, field, fit_screen, label,
    replace_screen, theme, title,
};

const HISTORY_MESSAGE: &str = "message";
const HISTORY_TIMELINE: &str = "timeline";

struct WatchlistView {
    _root: gtk::Box,
    _window: gtk::ApplicationWindow,
    prs: RefCell<Vec<WatchedPr>>,
    selected_id: RefCell<Option<i64>>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    count: MorphLabel,
    detail_title: MorphLabel,
    detail_num: gtk::Label,
    detail_repo: gtk::Label,
    detail_state: gtk::Label,
    _status_caption: gtk::Label,
    status_commit_btn: gtk::Button,
    status_commit_label: gtk::Label,
    status_meta: MorphLabel,
    _history_title: gtk::Label,
    history_summary: MorphLabel,
    history_stack: gtk::Stack,
    _history_scroll: gtk::ScrolledWindow,
    history_list: gtk::Box,
    history_empty: MorphLabel,
    open_button: gtk::Button,
    untrack_button: gtk::Button,
    status_label: MorphLabel,
    track_entry: gtk::Entry,
    track_button: gtk::Button,
    refresh_all_button: gtk::Button,
    _state: Rc<UiState>,
}

impl WatchlistView {
    fn set_prs(&self, prs: Vec<WatchedPr>) {
        let previous = *self.selected_id.borrow();
        let total = prs.len();
        let ready_count = prs.iter().filter(|p| p.is_ready()).count();
        let summary_text = if total == 0 {
            "No pull requests tracked yet".to_owned()
        } else if ready_count > 0 {
            format!("{total} direct pull requests · {ready_count} ready in nixos-unstable")
        } else {
            format!("{total} direct pull requests · waiting on branch promotion")
        };
        self.count.set_text(&summary_text);

        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        if prs.is_empty() {
            let empty_row = gtk::ListBoxRow::new();
            empty_row.set_selectable(false);
            let empty_label = label(
                "No pull requests tracked yet",
                &["watchlist-empty-text"],
                0.5,
            );
            empty_row.set_child(Some(&empty_label));
            self.list.append(&empty_row);
        } else {
            for pr in &prs {
                let row = watchlist_row(pr);
                self.list.append(&row);
            }
        }

        self.prs.replace(prs);

        let selection = previous
            .filter(|id| self.prs.borrow().iter().any(|p| p.id == *id))
            .or_else(|| self.prs.borrow().first().map(|p| p.id));

        if let Some(id) = selection {
            self.select_id(id);
        } else {
            self.clear_detail();
        }
        self.list.invalidate_filter();
    }

    fn select_id(&self, id: i64) {
        if let Some(row) =
            list_rows(&self.list).find(|row| row.widget_name().as_str() == id.to_string())
        {
            self.list.select_row(Some(&row));
        }
        self.show_detail_for_id(id);
    }

    fn show_detail_for_id(&self, id: i64) {
        let prs = self.prs.borrow();
        let Some(pr) = prs.iter().find(|p| p.id == id) else {
            self.clear_detail();
            return;
        };
        self.selected_id.replace(Some(id));

        self.detail_title.set_text(&pr.title);
        self.detail_num.set_text(&format!("#{}", pr.pr_number));
        self.detail_repo
            .set_text(&format!("{}/{}", pr.owner, pr.repo));

        let (state_text, state_tone) = match pr.pr_state {
            PrState::Merged => ("Merged", theme::GREEN),
            PrState::Open => ("Open", theme::MAUVE),
            PrState::Closed => ("Closed", theme::PEACH),
        };
        for tone in [theme::GREEN, theme::MAUVE, theme::PEACH].map(theme::tone) {
            self.detail_state.remove_css_class(tone);
        }
        self.detail_state.add_css_class(theme::tone(state_tone));
        self.detail_state.set_text(state_text);

        // Status commit button & meta
        if let (PrState::Merged, Some(sha)) = (pr.pr_state, pr.merge_commit.as_deref()) {
            let short_sha = &sha[..sha.len().min(12)];
            self.status_commit_label.set_text(short_sha);
            self.status_commit_btn.set_visible(true);
            self.status_commit_btn
                .set_tooltip_text(Some(&format!("Open commit {short_sha} on GitHub")));

            let date_str = pr.merged_at.as_deref().map_or_else(
                || "· Merged into master".to_owned(),
                |date| format!("· Merged on {}", &date[..date.len().min(10)]),
            );
            self.status_meta.set_text(&date_str);
        } else {
            self.status_commit_btn.set_visible(false);
            let meta_desc = match pr.pr_state {
                PrState::Open => "Open for review upstream",
                PrState::Closed => "Closed without merge",
                PrState::Merged => "Merged into master",
            };
            self.status_meta.set_text(meta_desc);
        }

        self.open_button.set_sensitive(true);
        self.untrack_button.set_sensitive(true);

        // Timeline steps
        clear(&self.history_list);

        let (pr_desc, pr_reached) = match pr.pr_state {
            PrState::Merged => ("Merged upstream into trunk", true),
            PrState::Open => ("Under review / open", false),
            PrState::Closed => ("Closed without merge", false),
        };
        append_timeline_step(
            &self.history_list,
            &format!("Pull Request #{}", pr.pr_number),
            pr_desc,
            if pr_reached {
                StepState::Reached
            } else if pr.pr_state == PrState::Open {
                StepState::Open
            } else {
                StepState::Pending { ahead_by: 0 }
            },
            Some(pr.html_url.clone()),
        );

        for channel in &pr.channels {
            let (desc, target_url) = if let Some(sha) = &pr.merge_commit {
                let short_sha = &sha[..sha.len().min(7)];
                match channel.state {
                    StepState::Reached => (
                        format!("Landed in branch at {short_sha}"),
                        Some(format!(
                            "https://github.com/{}/{}/commit/{sha}",
                            pr.owner, pr.repo
                        )),
                    ),
                    StepState::Pending { ahead_by } if ahead_by > 0 => {
                        let unit = if ahead_by == 1 { "commit" } else { "commits" };
                        (
                            format!("Waiting on Hydra channel advancement (+{ahead_by} {unit})"),
                            Some(format!(
                                "https://github.com/{}/{}/compare/{}...{sha}",
                                pr.owner, pr.repo, channel.name
                            )),
                        )
                    }
                    StepState::Pending { .. } => (
                        "Waiting on branch advancement".to_owned(),
                        Some(format!(
                            "https://github.com/{}/{}/compare/{}...{sha}",
                            pr.owner, pr.repo, channel.name
                        )),
                    ),
                    StepState::Open => ("Pending PR merge".to_owned(), Some(pr.html_url.clone())),
                }
            } else {
                let desc = match channel.state {
                    StepState::Open => "Pending PR merge",
                    _ => "Waiting on branch advancement",
                };
                (desc.to_owned(), Some(pr.html_url.clone()))
            };

            append_timeline_step(
                &self.history_list,
                &channel.name,
                &desc,
                channel.state.clone(),
                target_url,
            );
        }

        let reached_count = pr
            .channels
            .iter()
            .filter(|c| c.state == StepState::Reached)
            .count();
        let total_channels = pr.channels.len();
        self.history_summary.set_text(&format!(
            "{reached_count} reached · {} pending",
            total_channels.saturating_sub(reached_count)
        ));

        self.history_stack.set_visible_child_name(HISTORY_TIMELINE);
    }

    fn clear_detail(&self) {
        self.selected_id.replace(None);
        self.detail_title.set_text("No pull request selected");
        self.detail_num.set_text("");
        self.detail_repo.set_text("");
        self.detail_state.set_text("");
        self.status_commit_btn.set_visible(false);
        self.status_meta.set_text("");
        self.history_summary.set_text("");
        self.open_button.set_sensitive(false);
        self.untrack_button.set_sensitive(false);
        clear(&self.history_list);
        self.history_empty.set_text(
            "Select a pull request from the list, or track a new PR using the input above.",
        );
        self.history_stack.set_visible_child_name(HISTORY_MESSAGE);
    }

    fn current_pr(&self) -> Option<WatchedPr> {
        let selected = (*self.selected_id.borrow())?;
        self.prs.borrow().iter().find(|p| p.id == selected).cloned()
    }
}

pub(crate) fn show_watchlist(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    _config: Config,
) {
    state.clear_actions();
    replace_screen(root);
    root.remove_css_class("home-root");
    fit_screen(window, &state);

    let initial_prs = open_db()
        .ok()
        .flatten()
        .and_then(|conn| list_prs(&conn).ok())
        .unwrap_or_default();

    // ── Header ─────────────────────────────────────────────────────────────────
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("inputs-header");
    header.add_css_class("watchlist-header");

    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.set_hexpand(true);
    heading.append(&title("Watchlist"));
    let count = MorphLabel::new("", &["inputs-count"], 0.0);
    heading.append(&count.root);
    header.append(&heading);

    let track_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    track_bar.add_css_class("watchlist-track-bar");
    track_bar.set_valign(gtk::Align::End);

    let track_entry = gtk::Entry::new();
    track_entry.add_css_class("inputs-revision-entry");
    track_entry.add_css_class("watchlist-track-entry");
    track_entry.set_placeholder_text(Some("Paste PR URL or #number…"));
    track_entry.set_width_chars(30);
    field(&track_entry);
    track_bar.append(&track_entry);

    let track_button = action_button("Track", "list-add-symbolic", true);
    track_button.set_valign(gtk::Align::Center);
    track_bar.append(&track_button);

    header.append(&track_bar);
    root.append(&header);

    // ── Content (2-column grid: 2 cols browser, 5 cols detail) ────────────────
    let content = gtk::Grid::new();
    content.set_column_homogeneous(true);
    content.set_column_spacing(14);
    content.set_vexpand(true);

    // Left Column: Browser
    let browser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browser.add_css_class("inputs-browser");
    browser.add_css_class("watchlist-browser");
    browser.add_css_class("phase-panel");
    browser.set_hexpand(true);

    let search = gtk::SearchEntry::new();
    search.add_css_class("inputs-search");
    search.add_css_class("watchlist-search");
    field(&search);
    search.set_placeholder_text(Some("Filter pull requests"));
    browser.append(&search);

    let list = gtk::ListBox::new();
    list.add_css_class("inputs-list");
    list.add_css_class("see-through");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);

    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.add_css_class("see-through");
    list_scroll.set_vexpand(true);
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_child(Some(&list));
    state.set_scroll_adjustment(list_scroll.vadjustment());
    browser.append(&FadeScroll::new(&list_scroll));
    content.attach(&browser, 0, 0, 2, 1);

    // Right Column: Detail
    let details = gtk::Box::new(gtk::Orientation::Vertical, 12);
    details.add_css_class("inputs-detail");
    details.add_css_class("watchlist-detail");
    details.add_css_class("phase-panel");
    details.set_hexpand(true);
    details.set_vexpand(true);

    // Identity block: metadata & actions on top row, full-width title below
    let identity = gtk::Box::new(gtk::Orientation::Vertical, 8);
    identity.set_valign(gtk::Align::Start);

    let top_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    top_row.set_valign(gtk::Align::Center);

    let chips_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    chips_row.set_valign(gtk::Align::Center);
    chips_row.set_hexpand(true);

    let detail_num = label("", &["chip", "tinted", theme::tone(theme::VERSION)], 0.5);
    detail_num.set_valign(gtk::Align::Center);
    chips_row.append(&detail_num);

    let detail_repo = label("", &["chip", "tinted", theme::tone(theme::TEAL)], 0.5);
    detail_repo.set_valign(gtk::Align::Center);
    chips_row.append(&detail_repo);

    let detail_state = label("", &["chip", "tinted"], 0.5);
    detail_state.set_valign(gtk::Align::Center);
    chips_row.append(&detail_state);
    top_row.append(&chips_row);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_valign(gtk::Align::Center);
    actions.set_hexpand(false);

    let open_button = action_button("Open on GitHub", "web-browser-symbolic", false);
    actions.append(&open_button);

    let untrack_button = action_button("Untrack", "user-trash-symbolic", false);
    actions.append(&untrack_button);

    top_row.append(&actions);
    identity.append(&top_row);

    let detail_title = MorphLabel::new("", &["inputs-detail-name", "watchlist-detail-title"], 0.0);
    detail_title.set_ellipsize(gtk::pango::EllipsizeMode::None);
    identity.append(&detail_title.root);

    details.append(&identity);

    // Status line
    let status_line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_line.add_css_class("inputs-lock-line");

    let status_caption = label("STATUS", &["panel-eyebrow"], 0.0);
    status_caption.set_ellipsize(gtk::pango::EllipsizeMode::None);
    status_line.append(&status_caption);

    let status_commit_btn = gtk::Button::new();
    status_commit_btn.add_css_class("see-through");
    status_commit_btn.set_cursor_from_name(Some("pointer"));
    let status_commit_label = label("", &["inputs-revision"], 0.0);
    status_commit_btn.set_child(Some(&status_commit_label));
    status_line.append(&status_commit_btn);

    let status_meta = MorphLabel::new("", &["inputs-lock-meta", "watchlist-detail-meta"], 0.0);
    status_line.append(&status_meta.root);
    details.append(&status_line);

    // History panel (Branches)
    let history_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    history_panel.set_vexpand(true);

    let history_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let history_title = label("BRANCHES", &["panel-eyebrow"], 0.0);
    history_title.set_ellipsize(gtk::pango::EllipsizeMode::None);
    history_title.set_hexpand(true);
    history_header.append(&history_title);

    let history_summary = MorphLabel::new("", &["inputs-history-status"], 1.0);
    history_header.append(&history_summary.root);
    history_panel.append(&history_header);

    let history_empty = MorphLabel::new(
        "Select a pull request from the list",
        &["inputs-history-empty"],
        0.0,
    );
    history_empty.root.set_valign(gtk::Align::Start);

    let history_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let history_scroll = gtk::ScrolledWindow::new();
    history_scroll.add_css_class("see-through");
    history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    history_scroll.set_min_content_height(140);
    history_scroll.set_vexpand(true);
    history_scroll.set_child(Some(&history_list));

    let history_stack = gtk::Stack::new();
    history_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    history_stack.set_transition_duration(200);
    history_stack.set_vexpand(true);
    history_stack.add_named(&history_empty.root, Some(HISTORY_MESSAGE));
    history_stack.add_named(&FadeScroll::new(&history_scroll), Some(HISTORY_TIMELINE));
    history_panel.append(&history_stack);

    details.append(&history_panel);
    content.attach(&details, 2, 0, 5, 1);
    root.append(&content);

    // ── Footer ─────────────────────────────────────────────────────────────────
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("report-footer");

    let back = back_to_home_button(window, root, &state);
    footer.append(&back);
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));

    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    footer.append(&footer_spacer);

    let status_label = MorphLabel::new("", &["inputs-status"], 1.0);
    footer.append(&status_label.root);

    let refresh_all_button = action_button("Refresh all", "view-refresh-symbolic", false);
    footer.append(&refresh_all_button);
    root.append(&footer);

    // ── Component View Construction ────────────────────────────────────────────
    let view = Rc::new(WatchlistView {
        _root: root.clone(),
        _window: window.clone(),
        prs: RefCell::new(Vec::new()),
        selected_id: RefCell::new(None),
        list,
        search,
        count,
        detail_title,
        detail_num,
        detail_repo,
        detail_state,
        _status_caption: status_caption,
        status_commit_btn,
        status_commit_label,
        status_meta,
        _history_title: history_title,
        history_summary,
        history_stack,
        _history_scroll: history_scroll,
        history_list,
        history_empty,
        open_button,
        untrack_button,
        status_label,
        track_entry,
        track_button,
        refresh_all_button,
        _state: state,
    });

    let view_owner = RefCell::new(Some(Rc::clone(&view)));
    header.connect_unrealize(move |_| {
        view_owner.borrow_mut().take();
    });

    view.set_prs(initial_prs);
    connect_watchlist_signals(&view);
    view.search.grab_focus();
}

fn connect_watchlist_signals(view: &Rc<WatchlistView>) {
    // List selection
    let weak = Rc::downgrade(view);
    view.list.connect_row_selected(move |_, row| {
        if let (Some(view), Some(row)) = (weak.upgrade(), row)
            && let Ok(id) = row.widget_name().as_str().parse::<i64>()
        {
            view.show_detail_for_id(id);
        }
    });

    // Search filter
    let weak = Rc::downgrade(view);
    view.list.set_filter_func(move |row| {
        let Some(view) = weak.upgrade() else {
            return false;
        };
        let Ok(id) = row.widget_name().as_str().parse::<i64>() else {
            return false;
        };
        let query = view.search.text().to_lowercase();
        if query.is_empty() {
            return true;
        }
        view.prs.borrow().iter().any(|pr| {
            pr.id == id
                && (pr.title.to_lowercase().contains(&query)
                    || pr.pr_number.to_string().contains(&query)
                    || pr.repo.to_lowercase().contains(&query))
        })
    });

    let weak = Rc::downgrade(view);
    view.search.connect_search_changed(move |_| {
        if let Some(view) = weak.upgrade() {
            view.list.invalidate_filter();
        }
    });

    // Open on GitHub
    let weak = Rc::downgrade(view);
    view.open_button.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade()
            && let Some(pr) = view.current_pr()
        {
            open_uri(&pr.html_url);
        }
    });

    // Status commit click
    let weak = Rc::downgrade(view);
    view.status_commit_btn.connect_clicked(move |_| {
        if let Some(view) = weak.upgrade()
            && let Some(pr) = view.current_pr()
            && let Some(sha) = pr.merge_commit.as_deref()
        {
            let commit_url = format!("https://github.com/{}/{}/commit/{sha}", pr.owner, pr.repo);
            open_uri(&commit_url);
        }
    });

    // Untrack
    let weak = Rc::downgrade(view);
    view.untrack_button.connect_clicked(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let Some(pr) = view.current_pr() else {
            return;
        };
        if let Ok(Some(conn)) = open_db() {
            let _ = delete_pr(&conn, pr.id);
            if let Ok(updated) = list_prs(&conn) {
                view.set_prs(updated);
            }
        }
    });

    // Track button click / Enter press
    let on_track = {
        let weak = Rc::downgrade(view);
        move || {
            let Some(view) = weak.upgrade() else {
                return;
            };
            let text = view.track_entry.text().trim().to_string();
            if text.is_empty() {
                return;
            }
            view.status_label.set_text("");
            view.track_button.set_sensitive(false);

            let (owner, repo, pr_number) = match parse_pr_ref(&text) {
                Ok(parsed) => parsed,
                Err(err) => {
                    view.status_label.add_css_class("error");
                    view.status_label.set_text(&err);
                    view.track_button.set_sensitive(true);
                    return;
                }
            };

            let cancellation = Arc::new(AtomicBool::new(false));
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = fetch_pr(&owner, &repo, pr_number, None, &cancellation);
                let _ = sender.send(result);
            });

            let weak = Rc::downgrade(&view);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                let Some(view) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                match receiver.try_recv() {
                    Ok(Ok(watched_pr)) => {
                        view.track_button.set_sensitive(true);
                        view.track_entry.set_text("");
                        view.status_label.remove_css_class("error");
                        view.status_label.set_text("");
                        let new_id = if let Ok(Some(conn)) = open_db() {
                            let id = save_pr(&conn, &watched_pr).unwrap_or(0);
                            if let Ok(updated) = list_prs(&conn) {
                                view.set_prs(updated);
                            }
                            id
                        } else {
                            0
                        };
                        view.select_id(new_id);
                        glib::ControlFlow::Break
                    }
                    Ok(Err(err)) => {
                        view.track_button.set_sensitive(true);
                        view.status_label.add_css_class("error");
                        view.status_label.set_text(&err);
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        view.track_button.set_sensitive(true);
                        view.status_label.add_css_class("error");
                        view.status_label.set_text("GitHub request failed");
                        glib::ControlFlow::Break
                    }
                }
            });
        }
    };

    let on_track_rc = Rc::new(on_track);
    let on_track_cb = Rc::clone(&on_track_rc);
    view.track_button.connect_clicked(move |_| on_track_cb());
    view.track_entry.connect_activate(move |_| on_track_rc());

    // Refresh all
    let weak = Rc::downgrade(view);
    view.refresh_all_button.connect_clicked(move |_| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        view.refresh_all_button.set_sensitive(false);
        view.status_label.remove_css_class("error");
        view.status_label.set_text("Refreshing pull requests…");

        let current_prs = view.prs.borrow().clone();
        let cancellation = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();

        std::thread::spawn(move || {
            let mut updated_list = Vec::new();
            for pr in current_prs {
                if let Ok(refreshed) = fetch_pr(
                    &pr.owner,
                    &pr.repo,
                    pr.pr_number,
                    Some(pr.id),
                    &cancellation,
                ) {
                    updated_list.push(refreshed);
                } else {
                    updated_list.push(pr);
                }
            }
            let _ = sender.send(updated_list);
        });

        let weak = Rc::downgrade(&view);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let Some(view) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            match receiver.try_recv() {
                Ok(updated_list) => {
                    view.refresh_all_button.set_sensitive(true);
                    view.status_label.set_text("");
                    if let Ok(Some(conn)) = open_db() {
                        for pr in &updated_list {
                            let _ = save_pr(&conn, pr);
                        }
                        if let Ok(reloaded) = list_prs(&conn) {
                            view.set_prs(reloaded);
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    view.refresh_all_button.set_sensitive(true);
                    view.status_label.set_text("");
                    glib::ControlFlow::Break
                }
            }
        });
    });
}

fn watchlist_row(pr: &WatchedPr) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("input-row");
    row.add_css_class("watchlist-row");
    row.add_css_class("list-row");
    row.set_widget_name(&pr.id.to_string());

    let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);

    let title_text = label(&pr.title, &["input-row-name", "watchlist-row-title"], 0.0);
    title_text.set_tooltip_text(Some(&pr.title));
    copy.append(&title_text);

    let sub_text = format!("#{} · {}/{}", pr.pr_number, pr.owner, pr.repo);
    let sub = label(&sub_text, &["input-row-source", "watchlist-row-sub"], 0.0);
    sub.set_ellipsize(gtk::pango::EllipsizeMode::None);
    copy.append(&sub);
    row_content.append(&copy);

    let (badge_text, badge_classes): (String, Vec<&str>) = if pr.is_ready() {
        (
            "Ready".to_owned(),
            vec!["chip", "tinted", theme::tone(theme::GREEN)],
        )
    } else if pr.pr_state == PrState::Merged {
        (
            "Waiting".to_owned(),
            vec!["chip", "tinted", theme::tone(theme::YELLOW)],
        )
    } else if pr.pr_state == PrState::Open {
        (
            "Open".to_owned(),
            vec!["chip", "tinted", theme::tone(theme::MAUVE)],
        )
    } else {
        ("Closed".to_owned(), vec!["chip"])
    };
    let badge = label(&badge_text, &badge_classes, 0.5);
    badge.set_valign(gtk::Align::Center);
    row_content.append(&badge);

    row.set_child(Some(&row_content));
    row
}

fn append_timeline_step(
    container: &gtk::Box,
    name: &str,
    description: &str,
    state: StepState,
    target_url: Option<String>,
) {
    let button = gtk::Button::new();
    button.add_css_class("inputs-history-row");
    button.add_css_class("watchlist-timeline-row");
    button.set_halign(gtk::Align::Fill);

    if let Some(url) = &target_url {
        button.set_tooltip_text(Some(&format!("Open on GitHub ({url})")));
        button.set_cursor_from_name(Some("pointer"));
        let url = url.clone();
        button.connect_clicked(move |_| {
            open_uri(&url);
        });
    } else {
        button.set_can_focus(false);
    }

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);

    let rail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    rail.add_css_class("inputs-timeline-rail");
    rail.add_css_class("watchlist-timeline-rail");

    let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dot.add_css_class("inputs-timeline-dot");
    dot.add_css_class("watchlist-timeline-dot");

    match state {
        StepState::Reached => dot.add_css_class("reached"),
        StepState::Pending { .. } => dot.add_css_class("pending"),
        StepState::Open => dot.add_css_class("open"),
    }
    dot.set_halign(gtk::Align::Center);
    dot.set_valign(gtk::Align::Start);
    rail.append(&dot);
    row.append(&rail);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    copy.set_margin_top(8);
    copy.set_margin_bottom(8);

    let name_label = label(name, &["watchlist-timeline-name"], 0.0);
    name_label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    copy.append(&name_label);

    let desc_label = label(
        description,
        &["inputs-history-summary", "watchlist-timeline-desc"],
        0.0,
    );
    desc_label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    copy.append(&desc_label);
    row.append(&copy);

    let (pill_text, pill_classes): (String, Vec<&str>) = match state {
        StepState::Reached => (
            "✓ In branch".to_owned(),
            vec!["chip", "tinted", theme::tone(theme::GREEN)],
        ),
        StepState::Pending { ahead_by } if ahead_by > 0 => {
            let unit = if ahead_by == 1 { "commit" } else { "commits" };
            (
                format!("⏳ Waiting (+{ahead_by} {unit})"),
                vec!["chip", "tinted", theme::tone(theme::YELLOW)],
            )
        }
        StepState::Pending { .. } => (
            "⏳ Waiting".to_owned(),
            vec!["chip", "tinted", theme::tone(theme::YELLOW)],
        ),
        StepState::Open => ("⋯ Pending".to_owned(), vec!["chip"]),
    };
    let pill = label(&pill_text, &pill_classes, 0.5);
    pill.set_valign(gtk::Align::Center);
    row.append(&pill);

    button.set_child(Some(&row));
    container.append(&button);
}

fn open_uri(uri: &str) {
    if gio::AppInfo::launch_default_for_uri(uri, None::<&gio::AppLaunchContext>).is_err() {
        let _ = std::process::Command::new("xdg-open").arg(uri).spawn();
    }
}

fn action_button(text: &str, icon_name: &str, primary: bool) -> gtk::Button {
    let button = gtk::Button::new();
    if primary {
        button.add_css_class("inputs-primary-button");
        button.add_css_class("accent-button");
    } else {
        button.add_css_class("inputs-secondary-button");
        button.add_css_class("surface-button");
    }
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    content.set_halign(gtk::Align::Center);
    content.append(&centered_icon(icon_name, 14, 14));
    let btn_label = label(text, &["inputs-button-label"], 0.5);
    btn_label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    content.append(&btn_label);
    button.set_child(Some(&content));
    button
}

fn list_rows(list: &gtk::ListBox) -> impl Iterator<Item = gtk::ListBoxRow> + '_ {
    let mut current = list.first_child();
    std::iter::from_fn(move || {
        while let Some(child) = current.take() {
            current = child.next_sibling();
            if let Ok(row) = child.downcast::<gtk::ListBoxRow>() {
                return Some(row);
            }
        }
        None
    })
}
