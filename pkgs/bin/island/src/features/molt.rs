//! Molt updates: `molt check` for the flake every 15 minutes, a badge in the bar while changes are
//! incoming or the check failed, an Updates panel listing the commits, and a System-panel button.

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
    panel::{Block, Button, CommitRow, Header, IconButton, PanelView, on},
    run,
    supervisor::Ended,
    theme::{ACCENT, CRITICAL, SUBTEXT1},
    ui::SMALL_SIZE,
};

const INTERVAL: Duration = Duration::from_secs(15 * 60);
const TICK: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(120);

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
    pub error: Option<String>,
    /// Unix seconds of the last check attempt.
    pub checked_at: Option<u64>,
}

impl Updates {
    fn incoming(&self) -> u32 {
        self.report.as_ref().map_or(0, |report| report.incoming)
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

fn check() -> Result<Report, String> {
    let output = run::output("molt", &["check", "--json"], TIMEOUT)?;
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

fn work(feed: &Feed<Updates>, refresh: &mpsc::Receiver<()>) -> Ended {
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
        if forced || now().saturating_sub(cache.attempted_at) >= INTERVAL.as_secs() {
            state.checking = true;
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
            cache.attempted_at = now();
            state.checked_at = Some(cache.attempted_at);
            debug!(forced, "checking for updates");
            match check() {
                Ok(report) => {
                    info!(incoming = report.incoming, "update check finished");
                    cache.report = Some(report.clone());
                    state.report = Some(report);
                    state.error = None;
                }
                Err(error) => {
                    warn!("update check failed: {error}");
                    state.error = Some(error);
                }
            }
            state.checking = false;
            save(&cache);
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
        }
        forced = match refresh.recv_timeout(TICK) {
            Ok(()) => true,
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        };
    }
}

pub struct Molt {
    refresh: Sender<()>,
}

impl Molt {
    /// Checks for updates now instead of at the next interval.
    fn refresh(&self) {
        let _ = self.refresh.send(());
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

impl Feature for Molt {
    const NAME: &'static str = "molt";
    const GLYPHS: &'static [GlyphSpec] = &[("molt", &["io.github.krezh.Molt-symbolic", "io.github.krezh.Molt"])];
    const PANEL: Option<&'static str> = Some("molt");

    type State = Updates;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Updates>) -> Self {
        let (refresh, requests) = mpsc::channel();
        feed.supervise(move |feed| work(feed, &requests));
        Self { refresh }
    }

    fn present(_: &str, state: &Updates) -> bool {
        state.error.is_some() || state.incoming() > 0
    }

    fn widget(&self, _: &str, state: &Updates, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let (text, color) = match (&state.error, &state.report) {
            (Some(_), _) => ("!".to_owned(), CRITICAL),
            (None, Some(report)) => (report.incoming.to_string(), ACCENT),
            (None, None) => (String::new(), SUBTEXT1),
        };
        let tip = match (&state.error, &state.report) {
            (Some(error), _) => format!("molt check failed: {error}"),
            (None, Some(report)) => plural(report.incoming, "incoming change", "incoming changes"),
            (None, None) => "Molt".to_owned(),
        };
        Some(
            w.base("bar-molt")
                .gap(px(ICON_GAP))
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(color))
                .tip(w.tips(), "bar-molt", tip)
                .when_some(w.glyph("molt"), |badge, path| badge.child(icon::glyph(path, GLYPH, color)))
                .child(w.count(text))
                .on_click(w.toggle(Self::panel_handle()))
                .on_mouse_down(MouseButton::Right, w.call(Molt::refresh))
                .into_any_element(),
        )
    }

    /// Updates: what `molt check` last found for the flake, with a re-check and a way into Molt.
    fn panel<'a>(&self, state: &Updates, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let buttons = vec![
            Button {
                id: "molt-check".into(),
                label: if state.checking { "Checking…" } else { "Check now" }.into(),
                active: state.checking,
                toggle: false,
                on_click: p.call(Molt::refresh),
            },
            Button {
                id: "molt-open".into(),
                label: "Open Molt".into(),
                active: false,
                toggle: false,
                on_click: on(|island, cx| {
                    open();
                    island.set_mode(Mode::Collapsed, cx);
                }),
            },
        ];
        let card = |block: Block<'static>| Block::Card(None, card::Size::Large, vec![block]);
        let block = match (&state.error, &state.report) {
            (Some(error), _) => card(Block::Note(format!("The last check failed: {error}").into())),
            (None, None) if state.checking => card(Block::Note("Checking for updates…".into())),
            (None, None) => card(Block::Note("Not checked yet".into())),
            (None, Some(report)) if report.commits.is_empty() => card(Block::Note(
                if report.incoming == 0 { "Up to date" } else { "Check again to list the commits" }.into(),
            )),
            (None, Some(report)) => card(Block::Commits(
                report
                    .commits
                    .iter()
                    .map(|commit| CommitRow {
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
        Some(PanelView {
            header: Header::Title(title.into(), buttons),
            blocks: vec![block],
        })
    }

    fn system_button(&self, state: &Updates, p: &PanelCx<'_>) -> Option<IconButton> {
        let incoming = state.incoming();
        let label = match (&state.error, state.checking) {
            (_, true) => "Checking for updates…".to_owned(),
            (Some(_), false) => "The update check failed".to_owned(),
            (None, false) if incoming > 0 => plural(incoming, "incoming change", "incoming changes"),
            (None, false) => "Up to date".to_owned(),
        };
        Some(IconButton {
            id: "open-molt".into(),
            glyph: p.glyph("molt"),
            label: label.into(),
            active: incoming > 0,
            badge: (incoming > 0).then(|| incoming.to_string().into()),
            on_click: on(|island, cx| island.set_mode(Mode::Open(Self::panel_handle()), cx)),
        })
    }
}
