//! The whole run, as one full-screen session: the build renders its progress
//! here, and when it finishes the same screen becomes the changelog browser.
//!
//! Activation deliberately happens *outside* this module, after the terminal is
//! restored, so sudo can prompt and `switch-to-configuration` can stream.

use std::{
    path::PathBuf,
    process::Command,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use ansi_to_tui::IntoText as _;
use eyre::{Result, bail};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    changelog,
    diff::{self, Change, ChangeKind},
    progress::{BuildStream, Monitor, Plan, fit, human_bytes},
};

/// How long the loop waits on input before redrawing, and so the interval every
/// animation is sampled at. Nothing can appear to move faster than this.
const TICK: Duration = Duration::from_millis(80);

/// The full braille rotation. A subset of it does not read as a spinner: the
/// dots travel partway round, then jump back to the start.
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// One glyph per redraw. The loop is idling on `event::poll` whenever a spinner
/// is on screen, so its ticks are evenly spaced and the rotation comes out
/// smooth without having to be paced against the clock.
fn spinner_glyph(frame: usize) -> char {
    SPINNER[frame % SPINNER.len()]
}

/// How many in-flight items the live block shows. It is a fixed height, so the
/// finished rows above it never shift as Nix's parallelism varies.
const MAX_LIVE_ROWS: usize = 8;

/// Column widths for a browser row, chosen from the terminal width.
///
/// These are hard limits, not minimums: store paths yield names like
/// `vscode-extension-anthropic-claude-code` and versions like
/// `0-unstable-2026-09-01`, which would otherwise shove every column to their
/// right out of alignment. A wider terminal spends its room on the version,
/// since truncating `0-unstable-2026-09-01` to 16 loses the only part that
/// distinguishes it.
fn columns(width: usize) -> (usize, usize) {
    match width {
        0..=99 => (26, 16),
        100..=129 => (30, 21),
        _ => (34, 26),
    }
}

/// What the user decided while reviewing the changes.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Continue,
    Abort,
}

/// Everything the session needs to build, diff and browse.
pub struct Rebuild {
    pub label: String,
    /// The flake attribute, used to plan the transaction with `--dry-run`.
    pub attr: String,
    pub extra: Vec<String>,
    pub command: Command,
    pub old_profile: PathBuf,
    pub out_link: PathBuf,
    /// False for `--no-changelog`: build, then leave without browsing.
    pub browse: bool,
}

/// Turns an ANSI-styled string into a ratatui line, so the bars and colours
/// built in `progress` survive into the widget tree.
fn ansi_line(text: &str) -> Line<'static> {
    text.as_bytes()
        .into_text()
        .ok()
        .and_then(|parsed| parsed.lines.into_iter().next())
        .unwrap_or_else(|| Line::raw(text.to_owned()))
}

fn dim(text: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(Color::DarkGray),
    ))
}

// ---------------------------------------------------------------- browsing --

enum Content {
    Idle,
    Loading,
    Ready(Vec<Line<'static>>),
    Failed(String),
}

struct Row {
    change: Change,
    expanded: bool,
    content: Content,
}

type FetchResult = (usize, Result<String, String>);

struct Browser {
    rows: Vec<Row>,
    summary: String,
    selected: usize,
    scroll: usize,
    width: usize,
    tx: Sender<FetchResult>,
    rx: Receiver<FetchResult>,
}

impl Browser {
    fn new(changes: Vec<Change>, summary: String) -> Self {
        let (tx, rx) = channel();
        Self {
            rows: changes
                .into_iter()
                .map(|change| Row {
                    change,
                    expanded: false,
                    content: Content::Idle,
                })
                .collect(),
            summary,
            selected: 0,
            scroll: 0,
            width: 80,
            tx,
            rx,
        }
    }

    /// Kicks off a changelog lookup for a row, unless one already ran.
    fn request(&mut self, index: usize) {
        let Some(row) = self.rows.get_mut(index) else {
            return;
        };
        if !matches!(row.content, Content::Idle) {
            return;
        }

        // A package that is going away has no release notes worth fetching.
        if row.change.kind == ChangeKind::Removed {
            row.content = Content::Failed("package removed".to_owned());
            return;
        }

        row.content = Content::Loading;
        let tx = self.tx.clone();
        let (name, old, new) = (
            row.change.name.clone(),
            row.change.old.clone(),
            row.change.new.clone(),
        );

        std::thread::spawn(move || {
            let result =
                changelog::markdown_for(&name, &old, &new).map_err(|error| format!("{error:#}"));
            let _ = tx.send((index, result));
        });
    }

    fn drain(&mut self) {
        while let Ok((index, result)) = self.rx.try_recv() {
            let width = self.width;
            if let Some(row) = self.rows.get_mut(index) {
                row.content = match result {
                    Ok(markdown) => Content::Ready(render_markdown(&markdown, width)),
                    Err(error) => Content::Failed(error),
                };
            }
        }
    }

    fn toggle(&mut self) {
        let index = self.selected;
        if let Some(row) = self.rows.get_mut(index) {
            row.expanded = !row.expanded;
            if row.expanded {
                self.request(index);
            }
        }
    }

    fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    /// Flattens the rows into display lines, remembering where each row starts
    /// so the viewport can be kept on the selection.
    fn lines(&self) -> (Vec<Line<'static>>, Vec<usize>) {
        let mut lines = Vec::new();
        let mut offsets = Vec::with_capacity(self.rows.len());

        if self.rows.is_empty() {
            lines.push(dim("  Nothing changed in this closure."));
            return (lines, offsets);
        }

        for (index, row) in self.rows.iter().enumerate() {
            offsets.push(lines.len());
            lines.push(header_line(row, index == self.selected, self.width));

            if !row.expanded {
                continue;
            }

            match &row.content {
                Content::Idle | Content::Loading => lines.push(indented(Span::styled(
                    "loading changelog…",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ))),
                Content::Failed(error) => lines.push(indented(Span::styled(
                    format!("no changelog: {error}"),
                    Style::default().fg(Color::DarkGray),
                ))),
                Content::Ready(body) => {
                    for line in body {
                        let mut spans = vec![Span::raw("    ")];
                        spans.extend(line.spans.iter().cloned());
                        lines.push(Line::from(spans));
                    }
                }
            }
            lines.push(Line::raw(""));
        }

        (lines, offsets)
    }

    /// Scrolls the minimum amount needed to keep the selected row on screen.
    fn clamp_scroll(&mut self, offsets: &[usize], total: usize, height: usize) {
        let Some(&start) = offsets.get(self.selected) else {
            return;
        };

        if start < self.scroll {
            self.scroll = start;
        } else if start >= self.scroll + height {
            self.scroll = start.saturating_sub(height.saturating_sub(1));
        }
        self.scroll = self.scroll.min(total.saturating_sub(height));
    }

    fn render(&mut self, frame: &mut Frame, body: Rect, footer: Rect, hint: &str) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Changed packages ({}) ", self.rows.len()));
        let inner = block.inner(body);

        self.width = inner.width as usize;
        let (lines, offsets) = self.lines();
        let height = inner.height as usize;
        self.clamp_scroll(&offsets, lines.len(), height);

        let visible: Vec<Line> = lines.into_iter().skip(self.scroll).take(height).collect();
        frame.render_widget(Paragraph::new(Text::from(visible)).block(block), body);
        frame.render_widget(
            Paragraph::new(dim(format!("  {}   ·   {hint}", self.summary))),
            footer,
        );
    }

    /// Handles a keypress shared by both browsing entry points.
    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::PageUp => self.move_by(-10),
            KeyCode::PageDown => self.move_by(10),
            KeyCode::Home | KeyCode::Char('g') => self.selected = 0,
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = self.rows.len().saturating_sub(1);
            }
            KeyCode::Enter | KeyCode::Char(' ' | 'l') | KeyCode::Right => self.toggle(),
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(row) = self.rows.get_mut(self.selected) {
                    row.expanded = false;
                }
            }
            _ => {}
        }
    }
}

fn size_span(delta: i64) -> Span<'static> {
    if delta == 0 {
        return Span::raw("");
    }
    let (sign, colour) = if delta < 0 {
        ("-", Color::Magenta)
    } else {
        ("+", Color::Cyan)
    };
    Span::styled(
        format!("  {sign}{}", human_bytes(delta.unsigned_abs())),
        Style::default().fg(colour),
    )
}

fn header_line(row: &Row, selected: bool, width: usize) -> Line<'static> {
    let (name_col, version_col) = columns(width);
    let marker = if row.expanded { "▾" } else { "▸" };
    let (marker_style, name_style) = if selected {
        let highlight = Style::default().fg(Color::Black).bg(Color::Cyan);
        (highlight, highlight.add_modifier(Modifier::BOLD))
    } else {
        (
            Style::default().fg(Color::DarkGray),
            Style::default().add_modifier(Modifier::BOLD),
        )
    };

    // Every branch below occupies exactly VERSION_COLS, so the size column
    // lines up whichever kind the row is. An install or a removal has only one
    // version, so the arrow would point at nothing.
    let versions = match row.change.kind {
        ChangeKind::Upgraded => vec![
            Span::styled(
                format!(" {}", fit(&row.change.old, version_col)),
                Style::default().fg(Color::Red),
            ),
            Span::styled(" → ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                fit(&row.change.new, version_col),
                Style::default().fg(Color::Green),
            ),
        ],
        ChangeKind::Added => vec![
            Span::styled(
                format!("{:>width$} ", "added", width = version_col + 3),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                fit(&row.change.new, version_col),
                Style::default().fg(Color::Green),
            ),
        ],
        ChangeKind::Removed => vec![
            Span::styled(
                format!("{:>width$} ", "removed", width = version_col + 3),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                fit(&row.change.old, version_col),
                Style::default().fg(Color::Red),
            ),
        ],
    };

    let mut spans = vec![
        Span::styled(format!(" {marker} "), marker_style),
        Span::styled(fit(&row.change.name, name_col), name_style),
    ];
    spans.extend(versions);
    spans.push(size_span(row.change.size_delta));
    Line::from(spans)
}

fn indented(span: Span<'static>) -> Line<'static> {
    Line::from(vec![Span::raw("    "), span])
}

/// Renders markdown to ANSI with glamour, then reinterprets those escapes as
/// ratatui spans so the styling survives into the widget tree.
fn render_markdown(markdown: &str, width: usize) -> Vec<Line<'static>> {
    let wrap = width.saturating_sub(6).max(20);
    let ansi = glamour::Renderer::new()
        .with_style(glamour::Style::Dark)
        .with_word_wrap(wrap)
        .render(markdown);

    ansi.into_text().map_or_else(
        |_| markdown.lines().map(|l| Line::raw(l.to_owned())).collect(),
        |text: Text<'static>| text.lines,
    )
}

// ----------------------------------------------------------------- session --

enum Phase {
    /// `--dry-run` is working out the transaction on a worker thread. This can
    /// take a while on a large config, which is exactly why it happens inside
    /// the session rather than before it.
    Planning(Receiver<Result<Plan, String>>),
    Building,
    /// The build is done; dix is comparing the closures on a worker thread.
    Diffing(Receiver<Result<(Vec<Change>, String), String>>),
    Browsing(Browser),
    Failed(Vec<String>),
}

struct Session {
    label: String,
    monitor: Monitor,
    /// Held until the plan arrives, since the build starts only after it.
    command: Option<Command>,
    build: Option<BuildStream>,
    phase: Phase,
    toplevel: Option<PathBuf>,
    out_link: PathBuf,
    old_profile: PathBuf,
    browse: bool,
    frame: usize,
}

impl Session {
    fn spinner(&self) -> char {
        spinner_glyph(self.frame)
    }

    /// Starts the build once the plan is known.
    fn poll_plan(&mut self) -> Result<()> {
        let Phase::Planning(rx) = &self.phase else {
            return Ok(());
        };
        let Ok(result) = rx.try_recv() else {
            return Ok(());
        };

        match result {
            Ok(plan) => {
                self.monitor.set_plan(plan);
                let mut command = self.command.take().expect("command is taken once");
                self.build = Some(BuildStream::start(&mut command)?);
                self.phase = Phase::Building;
            }
            Err(error) => self.phase = Phase::Failed(vec![error]),
        }
        Ok(())
    }

    /// Advances the build, moving on once Nix closes its log.
    fn poll_build(&mut self, width: usize) -> Result<Option<Outcome>> {
        let Some(build) = self.build.as_mut() else {
            return Ok(None);
        };
        if build.pump(&mut self.monitor, width) {
            return Ok(None);
        }

        let status = build.wait()?;
        if !status.success() {
            self.phase = Phase::Failed(self.monitor.build_log().to_vec());
            return Ok(None);
        }

        let toplevel = std::fs::canonicalize(&self.out_link)?;
        self.toplevel = Some(toplevel.clone());

        if !self.browse {
            return Ok(Some(Outcome::Continue));
        }

        // dix walks the whole closure, which takes a beat; keep the UI alive.
        let (tx, rx) = channel();
        let old = self.old_profile.clone();
        std::thread::spawn(move || {
            let _ = tx.send(diff::changes(&old, &toplevel).map_err(|error| format!("{error:#}")));
        });
        self.phase = Phase::Diffing(rx);
        Ok(None)
    }

    fn poll_diff(&mut self) -> Option<Outcome> {
        let Phase::Diffing(rx) = &self.phase else {
            return None;
        };
        let Ok(result) = rx.try_recv() else {
            return None;
        };

        match result {
            // Always land on the review screen, even with nothing to show:
            // dropping straight out of the session at the end of a long build
            // reads as a crash.
            Ok((changes, summary)) => {
                self.phase = Phase::Browsing(Browser::new(changes, summary));
                None
            }
            Err(error) => {
                self.phase = Phase::Failed(vec![error]);
                None
            }
        }
    }
}

/// Builds the build-phase body: the transaction summary, the finished rows,
/// then whatever is in flight, directly beneath them.
///
/// The result is always exactly `height` lines, and the log is capped so room
/// for a full live block is always reserved. That keeps the finished rows still
/// as Nix's parallelism varies — they are anchored to the top and the live block
/// grows downwards into space already set aside, rather than the two being held
/// apart by a run of padding.
fn body_lines(monitor: &Monitor, width: usize, height: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = monitor.summary().into_iter().map(dim).collect();
    lines.push(Line::raw(""));

    // On a short terminal the live block shrinks rather than crowding out the
    // log entirely.
    let live_rows = MAX_LIVE_ROWS.min(height.saturating_sub(lines.len() + 2));
    let (active, hidden) = monitor.active_rows(width, live_rows);

    let log = monitor.log();
    let room = height.saturating_sub(lines.len() + live_rows + 1);
    let start = log.len().saturating_sub(room);
    lines.extend(log[start..].iter().map(|l| ansi_line(l)));

    lines.extend(active.iter().map(|l| ansi_line(l)));
    if hidden > 0 {
        lines.push(dim(format!("    … and {hidden} more")));
    }

    lines.resize(height, Line::raw(""));
    lines
}

fn draw(frame: &mut Frame, session: &mut Session) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());

    let spinner = session.spinner();
    let label = session.label.clone();
    let diffing = matches!(session.phase, Phase::Diffing(_));
    let planning = matches!(session.phase, Phase::Planning(_));

    match &mut session.phase {
        Phase::Browsing(browser) => {
            browser.render(
                frame,
                body,
                footer,
                "↑↓ move  ↵ expand  c continue  q abort",
            );
        }

        Phase::Failed(log) => {
            let block = Block::default().borders(Borders::ALL).title(" Failed ");
            let inner = block.inner(body);
            let height = inner.height as usize;
            let start = log.len().saturating_sub(height);
            let lines: Vec<Line> = log[start..].iter().map(|l| ansi_line(l)).collect();

            frame.render_widget(Paragraph::new(Text::from(lines)).block(block), body);
            frame.render_widget(
                Paragraph::new(dim("  the build failed — press any key to exit")),
                footer,
            );
        }

        Phase::Planning(_) | Phase::Building | Phase::Diffing(_) => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" norn · {label} "));
            let inner = block.inner(body);
            let width = inner.width as usize;
            let height = inner.height as usize;

            let lines = body_lines(&session.monitor, width, height);
            frame.render_widget(Paragraph::new(Text::from(lines)).block(block), body);

            let status = if planning {
                dim(format!(" {spinner} resolving what needs building…"))
            } else if diffing {
                dim(format!(" {spinner} comparing closures…"))
            } else {
                session.monitor.total_row(width).map_or_else(
                    || dim(" building…"),
                    // The footer sits outside the bordered block, so it needs a
                    // column of padding to line its bar up with the rows above.
                    |row| ansi_line(&format!(" {row}")),
                )
            };
            frame.render_widget(Paragraph::new(status), footer);
        }
    }
}

/// Handles a keypress. `Some` ends the session.
fn on_key(session: &mut Session, code: KeyCode, modifiers: KeyModifiers) -> Option<Outcome> {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Outcome::Abort);
    }

    match &mut session.phase {
        Phase::Failed(_) => Some(Outcome::Abort),
        Phase::Planning(_) | Phase::Building | Phase::Diffing(_) => None,
        Phase::Browsing(browser) => match code {
            KeyCode::Char('c') => Some(Outcome::Continue),
            // Esc goes with `q`: backing out of the review is a decision not to
            // activate, not a way to wave the build through.
            KeyCode::Char('q') | KeyCode::Esc => Some(Outcome::Abort),
            other => {
                browser.on_key(other);
                None
            }
        },
    }
}

fn event_loop(terminal: &mut DefaultTerminal, session: &mut Session) -> Result<Outcome> {
    loop {
        session.frame = session.frame.wrapping_add(1);
        session.monitor.tick();

        let width = terminal.size().map_or(100, |area| area.width as usize);

        match &session.phase {
            Phase::Planning(_) => session.poll_plan()?,
            Phase::Building => {
                if let Some(outcome) = session.poll_build(width)? {
                    return Ok(outcome);
                }
            }
            Phase::Diffing(_) => {
                if let Some(outcome) = session.poll_diff() {
                    return Ok(outcome);
                }
            }
            Phase::Browsing(_) => {
                if let Phase::Browsing(browser) = &mut session.phase {
                    browser.drain();
                }
            }
            Phase::Failed(_) => {}
        }

        terminal.draw(|frame| draw(frame, session))?;

        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && let Some(outcome) = on_key(session, key.code, key.modifiers)
        {
            return Ok(outcome);
        }
    }
}

/// Runs the build and, unless suppressed, the changelog browser — all in one
/// full-screen session. Returns the built closure and what the user decided.
pub fn run(spec: Rebuild) -> Result<(PathBuf, Outcome)> {
    let Rebuild {
        label,
        attr,
        extra,
        command,
        old_profile,
        out_link,
        browse,
    } = spec;

    // Planning happens on a worker so the session can draw from the first
    // frame; on a large config `--dry-run` is many seconds of evaluation.
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::nix::dry_run(&attr, &extra).map_err(|error| format!("{error:#}")));
    });

    let mut session = Session {
        label,
        monitor: Monitor::new(Plan::default()),
        command: Some(command),
        build: None,
        phase: Phase::Planning(rx),
        toplevel: None,
        out_link,
        old_profile,
        browse,
        frame: 0,
    };

    let mut terminal = ratatui::init();
    // Restore unconditionally: an error mid-loop must not leave the user
    // staring at a raw-mode alternate screen.
    let result = event_loop(&mut terminal, &mut session);
    ratatui::restore();

    let outcome = result?;

    // A build that never produced a closure has nothing to report but why.
    let Some(toplevel) = session.toplevel else {
        if let Phase::Failed(log) = &session.phase
            && !log.is_empty()
        {
            eprintln!("Last {} lines of the build log:", log.len());
            for line in log {
                eprintln!("  {line}");
            }
        }
        bail!("`nix build` failed");
    };

    eprintln!("{}", session.monitor.outcome());
    Ok((toplevel, outcome))
}

/// Browses an already-computed diff, without a build in front of it. Used by
/// `norn diff`.
pub fn browse_only(changes: Vec<Change>, summary: String) -> Result<Outcome> {
    let mut browser = Browser::new(changes, summary);
    let mut terminal = ratatui::init();
    let outcome = browse_loop(&mut terminal, &mut browser);
    ratatui::restore();
    outcome
}

fn browse_loop(terminal: &mut DefaultTerminal, browser: &mut Browser) -> Result<Outcome> {
    loop {
        browser.drain();

        terminal.draw(|frame| {
            let [body, footer] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
            browser.render(frame, body, footer, "↑↓ move  ↵ expand  q quit");
        })?;

        if !event::poll(TICK)? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Outcome::Abort);
            }
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Outcome::Continue),
            other => browser.on_key(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::display_width;

    fn monitor_with(active: usize, finished: usize) -> Monitor {
        let plan = Plan {
            builds: 64,
            known: true,
            ..Plan::default()
        };
        let mut monitor = Monitor::new(plan);

        for id in 0..(active + finished) {
            monitor.handle_line(
                &format!(
                    r#"@nix {{"action":"start","id":{id},"type":105,"fields":["/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-pkg{id}-1.0.drv","",1,1]}}"#
                ),
                100,
            );
        }
        // Retire the first `finished` of them into the log.
        for id in 0..finished {
            monitor.handle_line(&format!(r#"@nix {{"action":"stop","id":{id}}}"#), 100);
        }
        monitor
    }

    /// The regression that made the display jump: the live block grew with
    /// Nix's parallelism, so every frame had a different height.
    #[test]
    fn body_height_is_independent_of_parallelism() {
        for active in [0, 1, 5, 8, 20, 64] {
            for finished in [0, 3, 50] {
                let monitor = monitor_with(active, finished);
                assert_eq!(
                    body_lines(&monitor, 110, 38).len(),
                    38,
                    "active={active} finished={finished} changed the body height"
                );
            }
        }
    }

    #[test]
    fn body_fits_a_short_terminal() {
        for height in [4, 6, 10, 20] {
            let monitor = monitor_with(12, 20);
            assert_eq!(
                body_lines(&monitor, 110, height).len(),
                height,
                "height {height} overflowed"
            );
        }
    }

    /// Holding a glyph across two redraws reads as a stall, and covering only
    /// part of the rotation makes the dots snap backwards. The spinner used to
    /// do both: four of the ten glyphs, stepped every other frame.
    #[test]
    fn the_spinner_advances_every_redraw_through_a_whole_rotation() {
        let rotation: Vec<char> = (0..SPINNER.len()).map(spinner_glyph).collect();

        assert_eq!(
            rotation
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            SPINNER.len(),
            "a repeated glyph is a frame the eye reads as a stall"
        );
        assert!(
            rotation.windows(2).all(|pair| pair[0] != pair[1]),
            "the spinner must move on every redraw: {rotation:?}"
        );
        assert_eq!(
            spinner_glyph(SPINNER.len()),
            rotation[0],
            "and then come back round"
        );
    }

    fn row_of(name: &str, old: &str, new: &str, kind: ChangeKind) -> Row {
        Row {
            change: Change {
                name: name.to_owned(),
                old: old.to_owned(),
                new: new.to_owned(),
                kind,
                size_delta: 33_000,
            },
            expanded: false,
            content: Content::Idle,
        }
    }

    /// Long store-path names and versions must not shift the columns to their
    /// right; every row has to occupy the same width.
    #[test]
    fn browser_rows_are_column_aligned() {
        let rows = [
            row_of("norn", "1.0", "1.1", ChangeKind::Upgraded),
            row_of(
                "vscode-extension-anthropic-claude-code",
                "2.1.259",
                "2.1.263",
                ChangeKind::Upgraded,
            ),
            row_of(
                "noctalia",
                "5.0.0-beta.10-fish-completions",
                "5.0.1-fish-completions",
                ChangeKind::Upgraded,
            ),
            row_of("some-package", "", "0.1.0", ChangeKind::Added),
            row_of(
                "another-really-long-package-name",
                "1.2.3",
                "",
                ChangeKind::Removed,
            ),
        ];

        for term in [80, 110, 140] {
            let widths: Vec<usize> = rows
                .iter()
                .map(|row| display_width(&header_line(row, false, term).to_string()))
                .collect();

            assert!(
                widths.windows(2).all(|pair| pair[0] == pair[1]),
                "at {term} columns rows differ in width: {widths:?}"
            );
        }
    }

    /// The columns must also fit the terminal they were sized for.
    #[test]
    fn browser_rows_fit_the_terminal() {
        let row = row_of(
            "vscode-extension-anthropic-claude-code",
            "0-unstable-2026-09-01",
            "0-unstable-2026-09-07",
            ChangeKind::Upgraded,
        );
        for term in [80, 110, 140, 200] {
            let rendered = display_width(&header_line(&row, false, term).to_string());
            assert!(
                rendered <= term,
                "row of {rendered} overflowed {term} columns"
            );
        }
    }

    #[test]
    fn overflowing_work_is_reported_not_dropped() {
        let monitor = monitor_with(20, 0);
        let rendered: Vec<String> = body_lines(&monitor, 110, 38)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(
            rendered.iter().any(|line| line.contains("and 12 more")),
            "hidden rows must be counted"
        );
    }
}
