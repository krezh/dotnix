mod paste;
mod store;
mod watch;

use std::{
    io::Write,
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{AnyElement, Keystroke, SharedString, prelude::*};
use serde::Deserialize;
use tracing::error;

use super::{Feature, Feed, GlyphSpec, Local, Outcome, PanelCx, WidgetCx, parse_settings};
use crate::{
    bar::GLYPH,
    components::{
        icon,
        list::{self, ROWS},
        tooltip::Tip,
    },
    island::{Mode, Panel},
    panel::{AppRow, Block, Header, IconButton, LauncherView, PanelView, Preview, on},
    theme::TEXT,
};

/// Twice the preview pane, for sharp scaling.
const PREVIEW_W: u32 = 860;
const PREVIEW_H: u32 = 720;
/// So "5 min ago" keeps up.
const REFRESH: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    max_entries: usize,
    max_total_mb: u64,
    max_item_mb: u64,
    images: bool,
    /// Put the newest entry back when the clipboard empties, such as when the app that copied
    /// something quits.
    persist: bool,
    /// Empty only puts a picked entry on the clipboard.
    paste: String,
    terminal_paste: String,
    /// Window classes.
    terminals: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_entries: 200,
            max_total_mb: 256,
            max_item_mb: 32,
            images: true,
            persist: true,
            paste: "ctrl+v".to_owned(),
            terminal_paste: "ctrl+shift+v".to_owned(),
            terminals: [
                "com.mitchellh.ghostty",
                "kitty",
                "foot",
                "footclient",
                "Alacritty",
                "org.wezfurlong.wezterm",
                "org.gnome.Ptyxis",
                "org.gnome.Console",
                "org.kde.konsole",
                "xterm",
            ]
            .map(str::to_owned)
            .to_vec(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Image,
    Files,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Image => "image",
            Kind::Files => "files",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        [Kind::Text, Kind::Image, Kind::Files].into_iter().find(|kind| kind.word() == word)
    }
}

/// Without its data.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: i64,
    pub kind: Kind,
    pub mime: String,
    /// The start of the text, or the files' paths one per line.
    pub text: String,
    /// In the whole text.
    pub chars: i64,
    pub size: i64,
    pub dimensions: Option<(u32, u32)>,
    /// Unix seconds it was last copied or picked.
    pub used: i64,
    pub preview: Option<PathBuf>,
}

impl Entry {
    fn title(&self) -> String {
        match self.kind {
            Kind::Text => self.text.split_whitespace().collect::<Vec<_>>().join(" "),
            Kind::Image => format!("Picture ({})", self.mime.trim_start_matches("image/").to_uppercase()),
            Kind::Files => self
                .text
                .lines()
                .map(|path| path.rsplit('/').next().unwrap_or(path))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    fn detail(&self) -> String {
        let what = match self.kind {
            Kind::Text => plural(self.chars, "character", "characters"),
            Kind::Image => match self.dimensions {
                Some((width, height)) => format!("{width}×{height} · {}", bytes(self.size)),
                None => bytes(self.size),
            },
            Kind::Files => plural(self.text.lines().count() as i64, "file", "files"),
        };
        format!("{what} · {}", ago(self.used))
    }

    fn preview(&self) -> Preview {
        match (&self.preview, self.kind) {
            (Some(picture), Kind::Image | Kind::Files) => Preview::Picture(picture.clone()),
            (_, Kind::Text | Kind::Files) => Preview::Text(self.text.clone().into()),
            (None, Kind::Image) => Preview::Empty,
        }
    }

    fn about(&self) -> String {
        match self.kind {
            Kind::Text => format!(
                "{} · {} · {}",
                plural(self.chars, "character", "characters"),
                plural(self.text.lines().count().max(1) as i64, "line", "lines"),
                ago(self.used)
            ),
            Kind::Image => format!("{} · {}", self.mime, self.detail()),
            Kind::Files => self.detail(),
        }
    }

    fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.text.to_lowercase().contains(query) || self.title().to_lowercase().contains(query)
    }
}

fn plural(count: i64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn bytes(size: i64) -> String {
    match size {
        size if size < 1024 => format!("{size} B"),
        size if size < 1024 * 1024 => format!("{:.0} kB", size as f64 / 1024.0),
        size => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
    }
}

/// "just now", "4 min ago", "2 h ago", "3 d ago".
fn ago(at: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX));
    match now.saturating_sub(at) {
        seconds if seconds < 60 => "just now".to_owned(),
        seconds if seconds < 3600 => format!("{} min ago", seconds / 60),
        seconds if seconds < 86_400 => format!("{} h ago", seconds / 3600),
        seconds => format!("{} d ago", seconds / 86_400),
    }
}

pub fn files(uris: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(uris)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|uri| uri.strip_prefix("file://").map(|path| path.strip_prefix("localhost").unwrap_or(path)))
        .map(|path| path.and_then(unescape).map(PathBuf::from))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn unescape(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let high = (rest.next()? as char).to_digit(16)?;
            let low = (rest.next()? as char).to_digit(16)?;
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

#[derive(Clone, Default)]
pub struct State {
    entries: Arc<Vec<Entry>>,
    error: Option<String>,
    /// Counts every change, so comparing two states does not compare every entry.
    version: u64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl State {
    fn matching(&self, query: &str) -> Vec<&Entry> {
        let query = query.to_lowercase();
        self.entries.iter().filter(|entry| entry.matches(&query)).collect()
    }
}

pub enum Request {
    Select { id: i64, paste: bool },
    Delete(i64),
    Clear,
}

#[derive(Default)]
pub struct Picker {
    query: String,
    selected: usize,
    top: usize,
}

impl Picker {
    fn select(&mut self, position: usize, count: usize) {
        self.selected = position.min(count.saturating_sub(1));
        self.top = list::reveal(self.selected, self.top, count);
    }

    fn step(&mut self, step: i32, count: usize) {
        self.select(list::stepped(self.selected.min(count.saturating_sub(1)), step, count), count);
    }

    fn search(&mut self, change: impl FnOnce(&mut String)) {
        change(&mut self.query);
        self.selected = 0;
        self.top = 0;
    }
}

impl Local for Picker {
    fn panel_closed(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone)]
pub struct Clipboard {
    requests: Sender<Request>,
    /// The worker waits on Wayland, so it needs waking to read its requests.
    wake: Option<Arc<UnixStream>>,
}

impl Clipboard {
    fn request(&self, request: Request) {
        if self.requests.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            let _ = (&**wake).write(&[1]);
        }
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

impl Feature for Clipboard {
    const NAME: &'static str = "clipboard";
    const GLYPHS: &'static [GlyphSpec] = &[
        ("clipboard", &["edit-paste-symbolic", "edit-paste"]),
        ("clipboard-text", &["text-x-generic"]),
        ("clipboard-image", &["image-x-generic"]),
        ("clipboard-files", &["folder", "inode-directory"]),
    ];
    const PANEL: Option<&'static str> = Some("clipboard");
    const PANEL_REFRESH: Option<Duration> = Some(REFRESH);
    const USAGE: &'static str = "  clipboard clear                       forget the clipboard history";
    const KEYBOARD: bool = true;

    type State = State;
    type Local = Picker;
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        let settings: Settings = feed.settings();
        let (requests, incoming) = mpsc::channel();
        let wake = match UnixStream::pair().and_then(|(reader, writer)| reader.set_nonblocking(true).map(|()| (reader, writer))) {
            Ok((reader, writer)) => {
                feed.supervise(move |feed| watch::run(feed, &incoming, &reader, &settings));
                Some(Arc::new(writer))
            }
            Err(error) => {
                error!("cannot start the clipboard worker: {error}");
                None
            }
        };
        Self { requests, wake }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        let settings: Settings = parse_settings(settings)?;
        paste::Shortcut::parse(&settings.paste).map_err(|error| format!("paste: {error}"))?;
        paste::Shortcut::parse(&settings.terminal_paste).map_err(|error| format!("terminal_paste: {error}"))?;
        Ok(())
    }

    fn widget(&self, _: &str, _: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(
            w.base("bar-clipboard")
                .tip(w.tips(), "bar-clipboard", "Clipboard")
                .when_some(w.glyph("clipboard"), |button, path| button.child(icon::glyph(path, GLYPH, TEXT)))
                .on_click(w.toggle(Self::panel_handle()))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let picker = p.local::<Picker>()?;
        let matching = state.matching(&picker.query);
        let count = matching.len();
        let selected = picker.selected.min(count.saturating_sub(1));
        let top = list::reveal(selected, picker.top, count);
        let rows = matching
            .iter()
            .enumerate()
            .skip(top)
            .take(ROWS)
            .map(|(position, entry)| {
                let id = entry.id;
                let glyph = match entry.kind {
                    Kind::Text => "clipboard-text",
                    Kind::Image => "clipboard-image",
                    Kind::Files => "clipboard-files",
                };
                AppRow {
                    id: format!("clip-{id}").into(),
                    name: entry.title().into(),
                    detail: entry.detail().into(),
                    preview: entry.preview.is_some(),
                    icon: entry.preview.clone().or_else(|| p.glyph(glyph)),
                    selected: position == selected,
                    on_click: p.on_island::<Self>(move |clipboard, island, cx| {
                        clipboard.request(Request::Select { id, paste: true });
                        island.set_mode(Mode::Collapsed, cx);
                    }),
                    on_hover: p.on_local::<Self>(move |_, picker| picker.select(position, count)),
                }
            })
            .collect();
        let preview = match matching.get(selected) {
            Some(entry) => (entry.preview(), entry.about().into()),
            None => (Preview::Empty, SharedString::default()),
        };
        Some(PanelView {
            header: Header::None,
            blocks: vec![Block::Launcher(LauncherView {
                query: picker.query.clone().into(),
                placeholder: "Search the clipboard".into(),
                empty: state.error.clone().unwrap_or_else(|| "Nothing copied yet".to_owned()).into(),
                rows,
                on_step: p.on_local_with::<Self, i32>(move |_, picker, step| picker.step(step, count)),
                preview: Some(preview),
            })],
        })
    }

    fn key(&self, state: &State, picker: &mut Picker, keystroke: &Keystroke) -> Outcome {
        let matching = state.matching(&picker.query);
        let count = matching.len();
        let selected = matching.get(picker.selected.min(count.saturating_sub(1)));
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                if let Some(entry) = selected {
                    self.request(Request::Select {
                        id: entry.id,
                        paste: !keystroke.modifiers.shift,
                    });
                }
                return Outcome::Close;
            }
            "up" => picker.step(-1, count),
            "down" | "tab" => picker.step(1, count),
            "pageup" => picker.step(-(ROWS as i32), count),
            "pagedown" => picker.step(ROWS as i32, count),
            "delete" => {
                if let Some(entry) = selected {
                    self.request(Request::Delete(entry.id));
                }
            }
            "backspace" => picker.search(|query| {
                query.pop();
            }),
            "u" if keystroke.modifiers.control => picker.search(String::clear),
            _ => {
                if let Some(text) = list::typed(keystroke) {
                    picker.search(|query| query.push_str(text));
                }
            }
        }
        Outcome::Open
    }

    fn system_button(&self, _: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(IconButton {
            id: "open-clipboard".into(),
            glyph: p.glyph("clipboard"),
            label: "Clipboard".into(),
            active: false,
            badge: None,
            on_click: on(|island, cx| island.set_mode(Mode::Open(Self::panel_handle()), cx)),
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        match args {
            ["clipboard", "clear"] => Ok(()),
            _ => Err("expected clipboard clear".to_owned()),
        }
    }

    fn command(&self, _: &State, args: &[&str]) {
        if args == ["clipboard", "clear"] {
            self.request(Request::Clear);
        }
    }
}

#[cfg(test)]
#[path = "clipboard_test.rs"]
mod tests;
