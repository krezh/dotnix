use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::build::derivation_log;
use crate::app::state::UiState;
use crate::build::ui::progress::{activity_name, card};
use crate::ui::{FadeScroll, MorphLabel, centered_icon, label};

/// Lines of a build log shown at once; longer logs show their end, which is where builds fail.
const LOG_LINES: usize = 5_000;
/// How long a copy button reads "Copied" before its label returns.
const COPIED_FOR: Duration = Duration::from_millis(1_500);

/// The error without the `nix build failed:` line Swix puts above nix's own message.
pub(crate) fn error_summary(error: &str) -> &str {
    match error.split_once('\n') {
        Some((first, rest)) if first.ends_with(" failed:") && !rest.trim().is_empty() => {
            rest.trim()
        }
        _ => error.trim(),
    }
}

/// The last `max_lines` lines of `log`, and how many lines came before them.
fn log_tail(log: &str, max_lines: usize) -> (&str, usize) {
    let log = log.trim_end();
    let total = log.lines().count();
    if total <= max_lines {
        return (log, 0);
    }
    let skipped = total - max_lines;
    let start = log
        .match_indices('\n')
        .nth(skipped - 1)
        .map_or(0, |(index, _)| index + 1);
    (&log[start..], skipped)
}

/// A button that copies `text()` to the clipboard and briefly confirms it.
fn copy_button(caption: &'static str, text: impl Fn() -> String + 'static) -> gtk::Button {
    let caption_label = MorphLabel::new(caption, &[], 0.5);
    let button = gtk::Button::new();
    button.set_child(Some(&caption_label.root));
    button.add_css_class("build-failure-button");
    button.add_css_class("surface-button");
    button.connect_clicked(move |button| {
        button.clipboard().set_text(&text());
        caption_label.set_text("Copied");
        let caption_label = caption_label.downgrade();
        glib::timeout_add_local_once(COPIED_FOR, move || {
            if let Some(caption_label) = caption_label.upgrade() {
                caption_label.set_text(caption);
            }
        });
    });
    button
}

/// A wrapping, selectable line of log status above or in place of the log.
fn status_label(text: &str) -> gtk::Label {
    let status = label(text, &["build-log-status"], 0.0);
    status.set_wrap(true);
    status.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    status.set_ellipsize(gtk::pango::EllipsizeMode::None);
    status.set_selectable(true);
    status.set_valign(gtk::Align::Start);
    status
}

/// Replaces the side column's first card when evaluation or the build fails: nix's error, with
/// actions to copy it and, when a failed derivation is known, to open its build log.
pub(crate) struct ErrorCard {
    pub(crate) root: gtk::Box,
    pub(crate) view_log: gtk::Button,
}

impl ErrorCard {
    pub(crate) fn new(error: &str, derivation: Option<&str>) -> Self {
        let (root, header, _) = card("ERROR");
        root.add_css_class("build-error-card");
        root.set_vexpand(true);
        if let Some(derivation) = derivation {
            let name = label(&activity_name(derivation), &["build-card-caption"], 1.0);
            name.set_tooltip_text(Some(derivation));
            header.append(&name);
        }
        let message = error_summary(error).to_owned();
        let text = gtk::Label::new(Some(&message));
        text.add_css_class("build-error-text");
        text.set_selectable(true);
        text.set_wrap(true);
        text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        text.set_xalign(0.0);
        text.set_yalign(0.0);
        text.set_valign(gtk::Align::Start);
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&text));
        let faded = FadeScroll::new(&scroll);
        faded.add_css_class("build-error-scroll");
        root.append(&faded);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.add_css_class("build-error-actions");
        actions.append(&copy_button("Copy error", move || message.clone()));
        let view_log = gtk::Button::with_label("View log");
        view_log.add_css_class("build-failure-button");
        view_log.add_css_class("surface-button");
        view_log.set_visible(derivation.is_some());
        actions.append(&view_log);
        root.append(&actions);
        Self { root, view_log }
    }
}

/// The build log of a failed derivation, shown in place of the activity list.
pub(crate) struct LogView {
    pub(crate) root: gtk::Box,
    pub(crate) back: gtk::Button,
    pub(crate) scroll: gtk::ScrolledWindow,
    derivation: String,
    /// Crossfades between fetching, the error, and the log.
    body: gtk::Stack,
    fetching: gtk::Label,
    error: gtk::Label,
    content: gtk::Box,
    /// Above the log when it is cut to its last lines.
    note: gtk::Label,
    view: gtk::TextView,
    copy: gtk::Button,
    log: Rc<RefCell<Option<String>>>,
    loading: RefCell<Option<Arc<AtomicBool>>>,
}

impl LogView {
    pub(crate) fn new(derivation: &str) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("phase-panel");
        root.add_css_class("build-list-panel");
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        toolbar.add_css_class("build-list-toolbar");
        let back = gtk::Button::new();
        back.add_css_class("build-link-button");
        back.set_tooltip_text(Some("Back to the activity list"));
        back.set_valign(gtk::Align::Center);
        let back_content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        back_content.append(&centered_icon("go-previous-symbolic", 12, 12));
        back_content.append(&label("Activity", &[], 0.0));
        back.set_child(Some(&back_content));
        toolbar.append(&back);
        let eyebrow = label(
            &format!("BUILD LOG · {}", activity_name(derivation)),
            &["panel-eyebrow"],
            0.0,
        );
        eyebrow.set_tooltip_text(Some(derivation));
        eyebrow.set_hexpand(true);
        eyebrow.set_valign(gtk::Align::Center);
        toolbar.append(&eyebrow);
        root.append(&toolbar);
        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("build-list-rule");
        root.append(&rule);

        let fetching = status_label("Fetching the build log from nix…");
        let error = status_label("");
        error.add_css_class("error");
        let note = status_label("");
        note.set_visible(false);
        let view = gtk::TextView::new();
        view.add_css_class("build-log-view");
        view.set_editable(false);
        view.set_cursor_visible(false);
        view.set_monospace(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.add_css_class("build-log-scroll");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&view));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&note);
        content.append(&FadeScroll::new(&scroll));
        let body = gtk::Stack::new();
        body.set_transition_type(gtk::StackTransitionType::Crossfade);
        body.set_transition_duration(220);
        body.set_vexpand(true);
        body.add_child(&fetching);
        body.add_child(&error);
        body.add_child(&content);
        root.append(&body);

        let log = Rc::new(RefCell::new(None::<String>));
        let copy_log = Rc::clone(&log);
        let copy = copy_button("Copy log", move || {
            copy_log.borrow().clone().unwrap_or_default()
        });
        copy.set_valign(gtk::Align::Center);
        copy.set_sensitive(false);
        toolbar.append(&copy);
        Rc::new(Self {
            root,
            back,
            scroll,
            derivation: derivation.to_owned(),
            body,
            fetching,
            error,
            content,
            note,
            view,
            copy,
            log,
            loading: RefCell::new(None),
        })
    }

    /// Fetches the log with `nix log` on a worker thread, once; leaving the screen cancels it.
    pub(crate) fn load(self: &Rc<Self>, state: &UiState) {
        if self.log.borrow().is_some() || self.loading.borrow().is_some() {
            return;
        }
        let cancellation = Arc::new(AtomicBool::new(false));
        state.set_view_cancellation(&cancellation);
        self.loading.replace(Some(Arc::clone(&cancellation)));
        self.body.set_visible_child(&self.fetching);
        let (sender, receiver) = mpsc::channel();
        let derivation = self.derivation.clone();
        thread::spawn(move || {
            let _ = sender.send(derivation_log(&derivation, &cancellation));
        });
        let view = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err("nix log stopped without a result".to_owned())
                }
            };
            show_log(&view, result);
            glib::ControlFlow::Break
        });
    }

    fn show_error(&self, text: &str) {
        self.error.set_text(text);
        self.body.set_visible_child(&self.error);
    }
}

fn show_log(view: &Weak<LogView>, result: Result<String, String>) {
    let Some(view) = view.upgrade() else {
        return;
    };
    view.loading.replace(None);
    let log = match result {
        Ok(log) if !log.trim().is_empty() => log,
        Ok(_) => {
            view.show_error("Nix kept no build log for this derivation");
            return;
        }
        Err(error) => {
            view.show_error(error_summary(&error));
            return;
        }
    };
    let (shown, skipped) = log_tail(&log, LOG_LINES);
    let buffer = view.view.buffer();
    buffer.set_text(shown);
    // Settles at the end before the log fades in, so it never shows its top first.
    let end = buffer.create_mark(None, &buffer.end_iter(), false);
    view.view.scroll_to_mark(&end, 0.0, true, 0.0, 1.0);
    if skipped > 0 {
        view.note.set_text(&format!(
            "Showing the last {LOG_LINES} lines; {skipped} earlier lines are in Copy log"
        ));
    }
    view.note.set_visible(skipped > 0);
    view.body.set_visible_child(&view.content);
    view.log.replace(Some(log));
    view.copy.set_sensitive(true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_summary_drops_swix_command_line() {
        assert_eq!(
            error_summary("nix build failed:\nerror: builder for '/nix/store/a-x.drv' failed"),
            "error: builder for '/nix/store/a-x.drv' failed"
        );
        assert_eq!(
            error_summary("nix eval returned an invalid derivation path: x"),
            "nix eval returned an invalid derivation path: x"
        );
        assert_eq!(error_summary("dix failed:\n"), "dix failed:");
    }

    #[test]
    fn log_tail_keeps_the_end() {
        assert_eq!(log_tail("a\nb\n", 5), ("a\nb", 0));
        assert_eq!(log_tail("a\nb\nc\nd\n", 2), ("c\nd", 2));
        assert_eq!(log_tail("a\nb\nc", 1), ("c", 2));
    }
}
