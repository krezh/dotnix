//! Fetches every 15 minutes or when asked; in between, checks every minute against what was last
//! fetched, without the network, so the counts follow local changes such as a pull.

use std::{
    env, fs,
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{AnyElement, FontWeight, MouseButton, prelude::*, px, rgb};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::{
    bar::{GLYPH, ICON_GAP},
    components::{card, icon, tooltip::Tip},
    island::{Mode, Panel},
    panel::{Block, Button, Header, IconButton, Line, PanelView, Route, Stop, StopState, on},
    run,
    supervisor::Ended,
    theme::{ACCENT, CRITICAL, GREEN, SUBTEXT1},
    ui::SMALL_SIZE,
};

const INTERVAL: Duration = Duration::from_secs(15 * 60);
const TICK: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(120);
const CHECK: &str = "Refresh";
const CHECKING: &str = "Refreshing…";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub incoming: u32,
    pub local: u32,
    #[serde(default)]
    pub vcs: String,
    #[serde(default)]
    pub upstream: String,
    /// Newest incoming commits first; may be fewer than `incoming`.
    #[serde(default)]
    pub commits: Vec<Commit>,
    /// The pull requests molt watches; absent before molt had a watchlist.
    #[serde(default)]
    pub watchlist: Option<Watchlist>,
}

impl Report {
    fn ready(&self) -> usize {
        self.watchlist
            .as_ref()
            .map_or(0, |watchlist| watchlist.ready)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchlist {
    pub total: usize,
    pub ready: usize,
    #[serde(default)]
    pub prs: Vec<WatchedPr>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchedPr {
    pub pr_number: u64,
    pub repo: String,
    pub title: String,
    /// Reached the branch the system follows.
    pub ready: bool,
    /// `open`, `merged` or `closed`.
    pub state: String,
    #[serde(default)]
    pub merge_commit: Option<String>,
    #[serde(default)]
    pub branches: Vec<Branch>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub reached: bool,
    /// `reached`, `pending` or `open`.
    pub status: String,
    /// Commits the branch still has to take in before it has the pull request.
    #[serde(default)]
    pub ahead_by: Option<usize>,
}

impl WatchedPr {
    fn row(&self) -> Route {
        let state = match self.state.as_str() {
            "merged" => "Merged",
            "open" => "Open",
            "closed" => "Closed",
            other => other,
        };
        let commit = self
            .merge_commit
            .as_deref()
            .map(|sha| format!(" · {}", &sha[..sha.len().min(7)]))
            .unwrap_or_default();
        Route {
            title: format!("#{} {}", self.pr_number, self.title).into(),
            detail: format!("{} · {state}{commit}", self.repo).into(),
            done: self.ready.then(|| "Ready".into()),
            stops: self
                .branches
                .iter()
                .map(|branch| Stop {
                    label: branch.name.clone().into(),
                    state: match (branch.reached, branch.status.as_str()) {
                        (true, _) => StopState::Reached,
                        (false, "pending") => StopState::Pending(
                            branch
                                .ahead_by
                                .filter(|ahead| *ahead > 0)
                                .map(|ahead| format!("+{ahead}").into()),
                        ),
                        _ => StopState::NotYet,
                    },
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    pub summary: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Updates {
    pub report: Option<Report>,
    pub checking: bool,
    /// The check under way was asked for, by Check now or a right-click, rather than due; only
    /// then does the energy border run round the button and the widget.
    pub refreshing: bool,
    pub error: Option<String>,
    /// Unix seconds of the last check attempt.
    pub checked_at: Option<u64>,
}

impl Updates {
    fn incoming(&self) -> u32 {
        self.report.as_ref().map_or(0, |report| report.incoming)
    }

    fn ready(&self) -> usize {
        self.report.as_ref().map_or(0, Report::ready)
    }

    fn summary(&self) -> String {
        let incoming = self.incoming();
        let ready = self.ready();
        let ready = (ready > 0).then(|| {
            format!(
                "{} ready",
                plural(
                    u32::try_from(ready).unwrap_or(u32::MAX),
                    "watched PR",
                    "watched PRs"
                )
            )
        });
        match (incoming, ready) {
            (0, None) => "Up to date".to_owned(),
            (0, Some(ready)) => ready,
            (incoming, None) => plural(incoming, "incoming change", "incoming changes"),
            (incoming, Some(ready)) => format!(
                "{} · {ready}",
                plural(incoming, "incoming change", "incoming changes")
            ),
        }
    }
}

/// "just now", "4 min ago", "2 h ago", "3 d ago".
fn ago(at: u64) -> String {
    match now().saturating_sub(at) {
        _ if at == 0 => "unknown time".to_owned(),
        seconds if seconds < 60 => "just now".to_owned(),
        seconds if seconds < 3600 => format!("{} min ago", seconds / 60),
        seconds if seconds < 86_400 => format!("{} h ago", seconds / 3600),
        seconds => format!("{} d ago", seconds / 86_400),
    }
}

fn plural(count: u32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

#[derive(Default, Serialize, Deserialize)]
struct Cache {
    attempted_at: u64,
    report: Option<Report>,
}

fn cache_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("island").join("molt.json"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn load() -> Cache {
    cache_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(cache: &Cache) {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(raw) = serde_json::to_string(cache) {
        let _ = fs::write(path, raw);
    }
}

/// Only with `fetch` does it touch the network, fetching the upstream and refreshing the watchlist
/// from GitHub.
fn check(fetch: bool) -> Result<Report, String> {
    let args: &[&str] = if fetch {
        &["check", "--json"]
    } else {
        &["check", "--json", "--no-fetch"]
    };
    let output = run::output("molt", args, TIMEOUT)?;
    if !output.success {
        return Err(if output.stderr.is_empty() {
            "molt check failed".to_owned()
        } else {
            output.stderr
        });
    }
    serde_json::from_str(&output.stdout).map_err(|_| "invalid JSON from molt check".to_owned())
}

fn open() {
    crate::launch::spawn(crate::launch::Launch {
        id: "io.github.krezh.Molt".to_owned(),
        name: "Molt".to_owned(),
        argv: vec!["molt".to_owned()],
        directory: None,
    });
}

enum Request {
    Fetch,
    /// Without the network.
    Look,
}

fn work(feed: &Feed<Updates>, requests: &mpsc::Receiver<Request>) -> Ended {
    let mut cache = load();
    let mut state = Updates {
        report: cache.report.clone(),
        checked_at: (cache.attempted_at > 0).then_some(cache.attempted_at),
        ..Updates::default()
    };
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }
    let mut forced = false;
    loop {
        let fetch = forced || now().saturating_sub(cache.attempted_at) >= INTERVAL.as_secs();
        if fetch {
            state.checking = true;
            state.refreshing = forced;
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
            cache.attempted_at = now();
            state.checked_at = Some(cache.attempted_at);
        }
        debug!(forced, fetch, "checking for updates");
        let checked = check(fetch);
        let changed = match checked {
            Ok(report) => {
                if fetch {
                    info!(incoming = report.incoming, "update check finished");
                }
                let changed = state.report.as_ref() != Some(&report) || state.error.is_some();
                cache.report = Some(report.clone());
                state.report = Some(report);
                state.error = None;
                changed
            }
            Err(error) => {
                warn!(fetch, "update check failed: {error}");
                let changed = state.error.as_ref() != Some(&error);
                state.error = Some(error);
                changed
            }
        };
        if fetch || changed {
            state.checking = false;
            state.refreshing = false;
            save(&cache);
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
        }
        forced = match requests.recv_timeout(TICK) {
            Ok(Request::Fetch) => true,
            Ok(Request::Look) | Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        };
    }
}

pub struct Molt {
    requests: Sender<Request>,
}

impl Molt {
    fn refresh(&self) {
        let _ = self.requests.send(Request::Fetch);
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

impl Feature for Molt {
    const NAME: &'static str = "molt";
    const GLYPHS: &'static [GlyphSpec] = &[(
        "molt",
        &["io.github.krezh.Molt-symbolic", "io.github.krezh.Molt"],
    )];
    const PANEL: Option<&'static str> = Some("molt");

    type State = Updates;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Updates>) -> Self {
        let (requests, incoming) = mpsc::channel();
        feed.supervise(move |feed| work(feed, &incoming));
        Self { requests }
    }

    /// So the panel opens on the latest without waiting for the network.
    fn panel_opened(&self) {
        let _ = self.requests.send(Request::Look);
    }

    /// With no incoming changes, the ready watched pull requests show in green, since an update now
    /// brings their fixes.
    fn widget(&self, _: &str, state: &Updates, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let (text, color) = match (&state.error, state.incoming(), state.ready()) {
            (Some(_), _, _) => ("!".to_owned(), CRITICAL),
            (None, 0, ready) if ready > 0 => (ready.to_string(), GREEN),
            (None, incoming, _) if state.report.is_some() => (incoming.to_string(), ACCENT),
            _ => (String::new(), SUBTEXT1),
        };
        let tip = match (&state.error, &state.report) {
            (Some(error), _) => format!("molt check failed: {error}"),
            (None, Some(_)) => state.summary(),
            (None, None) => "Molt".to_owned(),
        };
        Some(
            w.base("bar-molt")
                .gap(px(ICON_GAP))
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(color))
                .tip(w.tips(), "bar-molt", tip)
                .when_some(w.glyph("molt"), |badge, path| {
                    badge.child(icon::glyph(path, GLYPH, color))
                })
                .child(w.count(text))
                .child(w.energy(state.refreshing))
                .on_click(w.toggle(Self::panel_handle()))
                .on_mouse_down(MouseButton::Right, w.call(Molt::refresh))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, state: &Updates, p: &PanelCx<'a>) -> Option<PanelView> {
        let buttons = vec![
            Button {
                id: "molt-check".into(),
                label: if state.checking { CHECKING } else { CHECK }.into(),
                active: state.checking,
                toggle: false,
                fits: vec![CHECK.into(), CHECKING.into()],
                busy: state.refreshing,
                enabled: true,
                on_click: p.call(Molt::refresh),
            },
            Button {
                id: "molt-open".into(),
                label: "Open Molt".into(),
                active: false,
                toggle: false,
                fits: Vec::new(),
                busy: false,
                enabled: true,
                on_click: on(|island, cx| {
                    open();
                    island.set_mode(Mode::Collapsed, cx);
                }),
            },
        ];
        let card = |block: Block| Block::Card(None, card::Size::Large, vec![block]);
        let block = match (&state.error, &state.report) {
            (Some(error), _) => card(Block::note(format!("The last check failed: {error}"))),
            (None, None) if state.checking => card(Block::note("Checking for updates…")),
            (None, None) => card(Block::note("Not checked yet")),
            (None, Some(report)) if report.commits.is_empty() => {
                card(Block::note(if report.incoming == 0 {
                    "Up to date"
                } else {
                    "Check again to list the commits"
                }))
            }
            (None, Some(report)) => card(Block::Lines(
                report
                    .commits
                    .iter()
                    .map(|commit| Line {
                        summary: commit.summary.clone().into(),
                        detail: format!(
                            "{} · {} · {}",
                            commit.author,
                            ago(u64::try_from(commit.time).unwrap_or(0)),
                            commit.id
                        )
                        .into(),
                    })
                    .collect(),
            )),
        };
        let title = match state.incoming() {
            0 => "Updates".to_owned(),
            incoming => plural(incoming, "incoming change", "incoming changes"),
        };
        let mut blocks = vec![block];
        if let Some(watchlist) = state
            .report
            .as_ref()
            .and_then(|report| report.watchlist.as_ref())
            .filter(|watchlist| !watchlist.prs.is_empty())
        {
            let title = match watchlist.ready {
                0 => "Watchlist".to_owned(),
                ready => format!("Watchlist · {ready} ready"),
            };
            blocks.push(Block::Card(
                Some(title.into()),
                card::Size::Large,
                vec![Block::Routes(
                    watchlist.prs.iter().map(WatchedPr::row).collect(),
                )],
            ));
        }
        Some(PanelView {
            header: Header::Title(title.into(), buttons),
            blocks,
        })
    }

    fn system_button(&self, state: &Updates, p: &PanelCx<'_>) -> Option<IconButton> {
        let incoming = state.incoming();
        let ready = state.ready();
        let label = match (&state.error, state.checking) {
            (_, true) => "Checking for updates…".to_owned(),
            (Some(_), false) => "The update check failed".to_owned(),
            (None, false) => state.summary(),
        };
        let badge = match (incoming, ready) {
            (0, 0) => None,
            (0, ready) => Some(ready.to_string().into()),
            (incoming, _) => Some(incoming.to_string().into()),
        };
        Some(IconButton {
            id: "open-molt".into(),
            glyph: p.glyph("molt"),
            rising: None,
            label: label.into(),
            active: incoming > 0 || ready > 0,
            badge,
            on_click: on(|island, cx| island.set_mode(Mode::Open(Self::panel_handle()), cx)),
        })
    }
}

#[cfg(test)]
#[path = "molt_test.rs"]
mod tests;
