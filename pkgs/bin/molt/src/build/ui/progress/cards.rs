//! The build screen's summary cards: evaluation, progress, downloads, and warnings.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use crate::ui::theme;
use crate::ui::{
    FadeScroll, MorphLabel, Tween, animations_enabled, compact_size, duration_text, fade_in,
    label,
};

use super::evaluation::{InputRow, InputState};
use super::format::clock;
use super::work::{Throughput, WorkSummary, remaining_estimate};

/// Warnings shown before "Show all".
const WARNING_PREVIEW: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum MeterFill {
    Indeterminate,
    Fraction { done: f64, pending: f64 },
}

/// An 8px bar: blue fill with an optional lighter in-flight segment that eases between readings,
/// or a shimmer when the end is unknown.
struct Meter {
    area: gtk::DrawingArea,
    indeterminate: Rc<Cell<bool>>,
    done: Tween,
    pending: Tween,
    shimmer: Rc<Cell<f64>>,
    animating: Rc<Cell<bool>>,
}

impl Meter {
    fn new() -> Self {
        let area = gtk::DrawingArea::new();
        area.add_css_class("build-meter");
        area.set_content_height(8);
        area.set_hexpand(true);
        let indeterminate = Rc::new(Cell::new(false));
        // The draw func reads copies of the tweened values: the tweens hold the area, so a draw
        // func holding them would keep it alive.
        let (done, done_drawn) = Self::tween(&area);
        let (pending, pending_drawn) = Self::tween(&area);
        let shimmer = Rc::new(Cell::new(0.0));
        let draw_indeterminate = Rc::clone(&indeterminate);
        let draw_shimmer = Rc::clone(&shimmer);
        area.set_draw_func(move |_, context, width, height| {
            let (width, height) = (f64::from(width), f64::from(height));
            let radius = height / 2.0;
            theme::set_source_rgb(context, theme::SURFACE0);
            theme::rounded_rectangle(context, 0.0, 0.0, width, height, radius);
            let _ = context.fill_preserve();
            context.clip();
            if draw_indeterminate.get() {
                let band = width * 0.24;
                let x = (width + band) * draw_shimmer.get() - band;
                let gradient = gtk::cairo::LinearGradient::new(x, 0.0, x + band, 0.0);
                let (r, g, b) = theme::ACCENT.rgb();
                gradient.add_color_stop_rgba(0.0, r, g, b, 0.0);
                gradient.add_color_stop_rgba(0.5, r, g, b, 0.6);
                gradient.add_color_stop_rgba(1.0, r, g, b, 0.0);
                let _ = context.set_source(&gradient);
                context.rectangle(x, 0.0, band, height);
                let _ = context.fill();
                return;
            }
            let done_width = width * done_drawn.get().clamp(0.0, 1.0);
            let pending_width =
                (width * pending_drawn.get().clamp(0.0, 1.0)).min(width - done_width);
            if pending_width > 0.0 {
                theme::set_source_rgba(context, theme::ACCENT, 0.35);
                context.rectangle(done_width, 0.0, pending_width, height);
                let _ = context.fill();
            }
            if done_width > 0.0 {
                theme::set_source_rgb(context, theme::ACCENT);
                theme::rounded_rectangle(context, 0.0, 0.0, done_width, height, radius);
                let _ = context.fill();
            }
        });
        Self {
            area,
            indeterminate,
            done,
            pending,
            shimmer,
            animating: Rc::new(Cell::new(false)),
        }
    }

    /// A tween on `area` starting at 0, and the cell its every frame is copied into.
    fn tween(area: &gtk::DrawingArea) -> (Tween, Rc<Cell<f64>>) {
        let tween = Tween::new(area, 0.0);
        let drawn = Rc::new(Cell::new(0.0));
        let frame = Rc::clone(&drawn);
        tween.on_frame(move |value| frame.set(value));
        (tween, drawn)
    }

    fn set(&self, fill: MeterFill) {
        let MeterFill::Fraction { done, pending } = fill else {
            self.start_shimmer();
            return;
        };
        if self.indeterminate.replace(false) {
            // Coming out of the shimmer there is no fill on screen to ease from.
            self.done.jump(done);
            self.pending.jump(pending);
        } else {
            self.done.set(done);
            self.pending.set(pending);
        }
    }

    fn start_shimmer(&self) {
        if self.indeterminate.replace(true) {
            return;
        }
        self.area.queue_draw();
        if self.animating.get() {
            return;
        }
        if !animations_enabled() {
            self.shimmer.set(0.5);
            return;
        }
        self.animating.set(true);
        let indeterminate = Rc::clone(&self.indeterminate);
        let shimmer = Rc::clone(&self.shimmer);
        let animating = Rc::clone(&self.animating);
        self.area.add_tick_callback(move |area, frame_clock| {
            if !indeterminate.get() {
                animating.set(false);
                return glib::ControlFlow::Break;
            }
            let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
            shimmer.set((seconds / 1.6).fract());
            area.queue_draw();
            glib::ControlFlow::Continue
        });
    }
}

fn set_text(label: &gtk::Label, text: &str) {
    if label.text() != text {
        label.set_text(text);
    }
}

pub(crate) fn card(eyebrow: &str) -> (gtk::Box, gtk::Box, MorphLabel) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("phase-panel");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("build-card-header");
    let heading = MorphLabel::new(eyebrow, &["panel-eyebrow"], 0.0);
    heading.root.set_hexpand(true);
    header.append(&heading.root);
    root.append(&header);
    (root, header, heading)
}

fn figure_row(figure: &gtk::Label, note: &gtk::Label) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    figure.set_ellipsize(gtk::pango::EllipsizeMode::None);
    figure.set_valign(gtk::Align::Baseline);
    note.set_valign(gtk::Align::Baseline);
    row.append(figure);
    row.append(note);
    row
}

pub(crate) struct EvaluationCard {
    pub(crate) root: gtk::Box,
    elapsed: gtk::Label,
    estimate: gtk::Label,
    meter: Meter,
    note: gtk::Label,
    inputs: gtk::Label,
}

impl EvaluationCard {
    pub(crate) fn new() -> Self {
        let (root, _, _) = card("EVALUATION");
        let elapsed = label("0:00", &["build-card-figure"], 0.0);
        let estimate = label("", &["build-card-figure-note"], 0.0);
        root.append(&figure_row(&elapsed, &estimate));
        let meter = Meter::new();
        root.append(&meter.area);
        let note = label("", &["build-card-note"], 0.0);
        root.append(&note);
        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("build-card-rule");
        root.append(&rule);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.add_css_class("build-card-line");
        let caption = label("Inputs", &["build-card-caption"], 0.0);
        caption.set_hexpand(true);
        line.append(&caption);
        let inputs = label("", &["build-card-value"], 1.0);
        line.append(&inputs);
        root.append(&line);
        Self {
            root,
            elapsed,
            estimate,
            meter,
            note,
            inputs,
        }
    }

    pub(crate) fn update(&self, elapsed: Duration, estimate: Option<Duration>, rows: &[InputRow]) {
        set_text(&self.elapsed, &clock(elapsed));
        match estimate {
            Some(estimate) if elapsed < estimate => {
                set_text(&self.estimate, &format!("of about {}", clock(estimate)));
                set_text(&self.note, "Estimate from the last evaluation");
                self.meter.set(MeterFill::Fraction {
                    done: elapsed.as_secs_f64() / estimate.as_secs_f64(),
                    pending: 0.0,
                });
            }
            Some(estimate) => {
                set_text(&self.estimate, &format!("of about {}", clock(estimate)));
                set_text(&self.note, "Taking longer than the last evaluation");
                self.meter.set(MeterFill::Indeterminate);
            }
            None => {
                set_text(&self.estimate, "");
                set_text(&self.note, "No earlier evaluation to estimate from");
                self.meter.set(MeterFill::Indeterminate);
            }
        }
        let cached = rows
            .iter()
            .filter(|row| row.state == InputState::Cached)
            .count();
        set_text(
            &self.inputs,
            &format!("{} fetched · {cached} cached", rows.len() - cached),
        );
    }
}

pub(crate) struct ProgressCard {
    pub(crate) root: gtk::Box,
    count: gtk::Label,
    meter: Meter,
    elapsed: gtk::Label,
    remaining: gtk::Label,
    note: gtk::Label,
}

impl ProgressCard {
    pub(crate) fn new() -> Self {
        let (root, _, _) = card("PROGRESS");
        let count = label("0 / 0", &["build-card-figure"], 0.0);
        count.set_ellipsize(gtk::pango::EllipsizeMode::None);
        root.append(&count);
        let meter = Meter::new();
        root.append(&meter.area);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.add_css_class("build-card-line");
        let elapsed = label("", &["build-card-caption"], 0.0);
        elapsed.set_hexpand(true);
        line.append(&elapsed);
        let remaining = label("", &["build-card-caption"], 1.0);
        line.append(&remaining);
        root.append(&line);
        let note = label("", &["build-card-note"], 0.0);
        root.append(&note);
        Self {
            root,
            count,
            meter,
            elapsed,
            remaining,
            note,
        }
    }

    /// `status` replaces the estimate note once building is over and comparison runs.
    pub(crate) fn update(&self, summary: &WorkSummary, elapsed: Duration, status: Option<&str>) {
        let finished = summary.done + summary.failed;
        set_text(&self.count, &format!("{finished} / {}", summary.total));
        let total = summary.total.max(1) as f64;
        self.meter.set(MeterFill::Fraction {
            done: finished as f64 / total,
            pending: summary.in_flight as f64 / total,
        });
        set_text(&self.elapsed, &format!("{} elapsed", clock(elapsed)));
        let estimate = status
            .is_none()
            .then(|| remaining_estimate(summary))
            .flatten();
        let remaining = match estimate {
            Some(left) if left < Duration::from_secs(60) => "under a minute left".to_owned(),
            Some(left) => format!("about {} left", duration_text(left.as_secs())),
            None => String::new(),
        };
        set_text(&self.remaining, &remaining);
        let note = match (status, estimate) {
            (Some(status), _) => status,
            (None, Some(_)) => "Estimate from finished build times",
            (None, None) => "",
        };
        set_text(&self.note, note);
        self.note.set_visible(!note.is_empty());
    }
}

pub(crate) struct DownloadsCard {
    pub(crate) root: gtk::Box,
    done: gtk::Label,
    total: gtk::Label,
    rate: gtk::Label,
    sparkline: gtk::DrawingArea,
    history: Rc<RefCell<Vec<f64>>>,
    throughput: RefCell<Throughput>,
}

impl DownloadsCard {
    pub(crate) fn new() -> Self {
        let (root, _, _) = card("DOWNLOADS");
        let done = label("0 B", &["build-card-figure", "small"], 0.0);
        let total = label("", &["build-card-figure-note"], 0.0);
        let figures = figure_row(&done, &total);
        let rate = label("", &["build-card-rate"], 1.0);
        rate.set_hexpand(true);
        rate.set_ellipsize(gtk::pango::EllipsizeMode::None);
        rate.set_valign(gtk::Align::Baseline);
        figures.append(&rate);
        root.append(&figures);
        let history = Rc::new(RefCell::new(Vec::new()));
        let sparkline = gtk::DrawingArea::new();
        sparkline.add_css_class("build-sparkline");
        sparkline.set_content_height(36);
        sparkline.set_hexpand(true);
        let draw_history = Rc::clone(&history);
        sparkline.set_draw_func(move |_, context, width, height| {
            let (width, height) = (f64::from(width), f64::from(height));
            theme::set_source_rgb(context, theme::SURFACE0);
            context.set_line_width(1.0);
            context.move_to(0.0, height - 0.5);
            context.line_to(width, height - 0.5);
            let _ = context.stroke();
            let history = draw_history.borrow();
            if history.len() < 2 {
                return;
            }
            let peak = history.iter().copied().fold(1.0_f64, f64::max);
            let step = width / (Throughput::POINTS - 1) as f64;
            let start = width - step * (history.len() - 1) as f64;
            context.set_line_width(1.5);
            context.set_line_cap(gtk::cairo::LineCap::Round);
            context.set_line_join(gtk::cairo::LineJoin::Round);
            for (index, rate) in history.iter().enumerate() {
                let x = start + step * index as f64;
                let y = (height - 3.0) - (height - 6.0) * (rate / peak);
                if index == 0 {
                    context.move_to(x, y);
                } else {
                    context.line_to(x, y);
                }
            }
            theme::set_source_rgb(context, theme::SAPPHIRE);
            let _ = context.stroke();
        });
        root.append(&sparkline);
        Self {
            root,
            done,
            total,
            rate,
            sparkline,
            history,
            throughput: RefCell::new(Throughput::default()),
        }
    }

    pub(crate) fn update(&self, now: Instant, (done, expected): (u64, u64)) {
        let mut throughput = self.throughput.borrow_mut();
        let rate = throughput.sample(now, done);
        set_text(&self.done, &compact_size(done as f64));
        set_text(
            &self.total,
            &if expected > 0 {
                format!("of {}", compact_size(expected as f64))
            } else {
                "nothing to download".to_owned()
            },
        );
        set_text(
            &self.rate,
            &if expected > done {
                format!("{}/s", compact_size(rate))
            } else {
                String::new()
            },
        );
        if throughput.history.len() != self.history.borrow().len()
            || throughput.history.back() != self.history.borrow().last()
        {
            self.history
                .replace(throughput.history.iter().copied().collect());
            self.sparkline.queue_draw();
        }
    }
}

/// A revealer around `child` that starts `revealed`.
fn revealer(
    child: &impl IsA<gtk::Widget>,
    transition: gtk::RevealerTransitionType,
    revealed: bool,
) -> gtk::Revealer {
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(transition);
    revealer.set_transition_duration(if animations_enabled() { 200 } else { 0 });
    revealer.set_reveal_child(revealed);
    revealer.set_child(Some(child));
    revealer
}

pub(crate) struct WarningsCard {
    pub(crate) root: gtk::Box,
    heading: MorphLabel,
    toggle: gtk::Revealer,
    toggle_label: MorphLabel,
    preview: gtk::Box,
    rest: gtk::Box,
    reveal: gtk::Revealer,
    /// "Warnings appear here…" until the first one does.
    empty: gtk::Revealer,
    more: gtk::Revealer,
    more_label: MorphLabel,
    labels: RefCell<Vec<gtk::Label>>,
    warnings: RefCell<HashSet<String>>,
    scroll: gtk::ScrolledWindow,
}

impl WarningsCard {
    pub(crate) fn new() -> Rc<Self> {
        let (root, header, heading) = card("WARNINGS · 0");
        root.add_css_class("build-warnings-card");
        root.set_vexpand(true);
        let toggle_label = MorphLabel::new("Show all", &[], 0.5);
        let button = gtk::Button::new();
        button.add_css_class("build-link-button");
        button.set_child(Some(&toggle_label.root));
        let toggle = revealer(&button, gtk::RevealerTransitionType::Crossfade, false);
        header.append(&toggle);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let preview = gtk::Box::new(gtk::Orientation::Vertical, 6);
        list.append(&preview);
        let rest = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let reveal = revealer(&rest, gtk::RevealerTransitionType::SlideDown, false);
        list.append(&reveal);
        let placeholder = label(
            "Warnings appear here as nix reports them",
            &["build-card-note"],
            0.0,
        );
        placeholder.set_wrap(true);
        let empty = revealer(&placeholder, gtk::RevealerTransitionType::SlideDown, true);
        list.append(&empty);
        let more_label = MorphLabel::new("", &["build-card-note"], 0.0);
        let more = revealer(&more_label.root, gtk::RevealerTransitionType::SlideDown, false);
        list.append(&more);
        let scroll = gtk::ScrolledWindow::new();
        scroll.add_css_class("see-through");
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&list));
        root.append(&FadeScroll::new(&scroll));
        let card = Rc::new(Self {
            root,
            heading,
            toggle,
            toggle_label,
            preview,
            rest,
            reveal,
            empty,
            more,
            more_label,
            labels: RefCell::new(Vec::new()),
            warnings: RefCell::new(HashSet::new()),
            scroll,
        });
        let weak = Rc::downgrade(&card);
        button.connect_clicked(move |_| {
            if let Some(card) = weak.upgrade() {
                card.reveal.set_reveal_child(!card.reveal.reveals_child());
                card.sync();
            }
        });
        card
    }

    /// A copy with the same warnings for the failure column. It stops taking the column's spare
    /// height, so the error card above it can, and grows with its content up to a cap instead.
    pub(crate) fn compact_copy(&self) -> Rc<Self> {
        let copy = Self::new();
        copy.root.set_vexpand(false);
        copy.scroll.set_vexpand(false);
        copy.scroll.set_propagate_natural_height(true);
        copy.scroll.set_max_content_height(220);
        copy.reveal.set_reveal_child(self.reveal.reveals_child());
        for line in self.labels.borrow().iter() {
            copy.push(&line.text());
        }
        copy.sync();
        copy
    }

    /// Adds warnings not seen before; nix repeats some across evaluation and build. Pass only
    /// warnings not handed in before, so each update costs only what it adds.
    pub(crate) fn extend(&self, warnings: &[String]) {
        let mut added = false;
        for warning in warnings {
            let text = warning
                .strip_prefix("warning:")
                .map_or(warning.as_str(), str::trim_start);
            if let Some(row) = self.push(text) {
                fade_in(&row);
                added = true;
            }
        }
        if added {
            self.sync();
        }
    }

    /// Adds a row for `text` unless it is already shown, and returns it.
    fn push(&self, text: &str) -> Option<gtk::Box> {
        if !self.warnings.borrow_mut().insert(text.to_owned()) {
            return None;
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        row.add_css_class("build-warning");
        row.add_css_class("notice");
        row.add_css_class(crate::ui::theme::tone(crate::ui::theme::YELLOW));
        let line = label(text, &["build-warning-text"], 0.0);
        line.set_hexpand(true);
        line.set_tooltip_text(Some(text));
        row.append(&line);
        if self.warnings.borrow().len() <= WARNING_PREVIEW {
            self.preview.append(&row);
        } else {
            self.rest.append(&row);
        }
        self.labels.borrow_mut().push(line);
        Some(row)
    }

    fn sync(&self) {
        let count = self.warnings.borrow().len();
        let expanded = self.reveal.reveals_child();
        self.heading.set_text(&format!("WARNINGS · {count}"));
        self.toggle.set_reveal_child(count > WARNING_PREVIEW);
        self.toggle_label
            .set_text(if expanded { "Show less" } else { "Show all" });
        for line in self.labels.borrow().iter() {
            line.set_wrap(expanded);
            line.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            line.set_ellipsize(if expanded {
                gtk::pango::EllipsizeMode::None
            } else {
                gtk::pango::EllipsizeMode::End
            });
        }
        self.empty.set_reveal_child(count == 0);
        let hidden = count.saturating_sub(WARNING_PREVIEW);
        let more = hidden > 0 && !expanded;
        // Text only changes while shown, so a hiding count keeps its last value as it slides out.
        if more {
            self.more_label.set_text(&format!("+ {hidden} more"));
        }
        self.more.set_reveal_child(more);
    }
}
