//! MPRIS players over D-Bus: the track to show, and play/pause, skip and seek.

use std::{
    collections::HashMap,
    env,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

use zbus::{
    MatchRule,
    blocking::{Connection, MessageIterator, fdo::DBusProxy},
    message::Type,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

use tracing::{debug, info, trace, warn};

use crate::{dbus::uncached, features::Feed, supervisor::Ended};

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub player: String,
    pub identity: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art: Option<PathBuf>,
    pub length: Option<f64>,
    pub position: f64,
    pub playing: bool,
    pub read_at: Instant,
    track_id: Option<String>,
}

impl Track {
    pub fn position_now(&self) -> f64 {
        let elapsed = if self.playing { self.read_at.elapsed().as_secs_f64() } else { 0.0 };
        let position = self.position + elapsed;
        self.length.map_or(position, |length| position.min(length))
    }

    fn same(&self, other: &Track) -> bool {
        Track {
            read_at: other.read_at,
            position: other.position,
            ..self.clone()
        } == *other
            && (self.position_now() - other.position_now()).abs() < 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    PlayPause,
    Next,
    Previous,
    Seek(f64),
}

enum Event {
    Changed,
    Control(Control),
}

#[derive(Clone)]
pub struct Player {
    sender: Sender<Control>,
}

impl Player {
    pub fn send(&self, control: Control) {
        let _ = self.sender.send(control);
    }
}

fn text(metadata: &HashMap<String, OwnedValue>, key: &str) -> String {
    metadata
        .get(key)
        .and_then(|value| match &**value {
            Value::Str(text) => Some(text.to_string()),
            Value::Array(items) => {
                let joined: Vec<String> = items
                    .iter()
                    .filter_map(|item| match item {
                        Value::Str(text) => Some(text.to_string()),
                        _ => None,
                    })
                    .collect();
                Some(joined.join(", "))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn micros(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<f64> {
    metadata.get(key).and_then(|value| match &**value {
        Value::I64(micros) => Some(*micros as f64 / 1e6),
        Value::U64(micros) => Some(*micros as f64 / 1e6),
        _ => None,
    })
}

fn art(url: &str) -> Option<PathBuf> {
    if let Some(path) = url.strip_prefix("file://") {
        return Some(PathBuf::from(path));
    }
    if !url.starts_with("http") {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let dir = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?
        .join("island")
        .join("art");
    let path = dir.join(format!("{:016x}", hasher.finish()));
    if path.exists() {
        return Some(path);
    }
    std::fs::create_dir_all(&dir).ok()?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let response = agent.get(url).call().ok()?;
    let bytes = response.into_body().with_config().limit(16 * 1024 * 1024).read_to_vec().ok()?;
    let partial = path.with_extension("part");
    std::fs::write(&partial, bytes).ok()?;
    std::fs::rename(&partial, &path).ok()?;
    Some(path)
}

fn read(connection: &Connection, player: &str) -> Option<Track> {
    let proxy = uncached(connection, player.to_owned(), PATH, PLAYER).ok()?;
    let status: String = proxy.get_property("PlaybackStatus").ok()?;
    let metadata: HashMap<String, OwnedValue> = proxy.get_property("Metadata").unwrap_or_default();
    let position = proxy.get_property::<i64>("Position").map_or(0.0, |micros| micros as f64 / 1e6);
    let identity = uncached(connection, player.to_owned(), PATH, "org.mpris.MediaPlayer2")
        .ok()
        .and_then(|root| root.get_property::<String>("Identity").ok())
        .unwrap_or_else(|| player.trim_start_matches(PREFIX).to_owned());
    let track_id = metadata.get("mpris:trackid").and_then(|value| match &**value {
        Value::ObjectPath(path) => Some(path.to_string()),
        Value::Str(path) => Some(path.to_string()),
        _ => None,
    });
    Some(Track {
        player: player.to_owned(),
        identity,
        title: text(&metadata, "xesam:title"),
        artist: text(&metadata, "xesam:artist"),
        album: text(&metadata, "xesam:album"),
        art: Some(text(&metadata, "mpris:artUrl")).filter(|url| !url.is_empty()).and_then(|url| art(&url)),
        length: micros(&metadata, "mpris:length").filter(|length| *length > 0.0),
        position,
        playing: status == "Playing",
        read_at: Instant::now(),
        track_id,
    })
}

fn current(connection: &Connection, preferred: &Option<String>) -> Option<Track> {
    let names = DBusProxy::new(connection).ok()?.list_names().ok()?;
    let tracks: Vec<Track> = names
        .iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with(PREFIX))
        .filter_map(|name| read(connection, &name))
        .filter(|track| !track.title.is_empty())
        .collect();
    tracks
        .iter()
        .find(|track| track.playing)
        .or_else(|| tracks.iter().find(|track| Some(&track.player) == preferred.as_ref()))
        .or_else(|| tracks.first())
        .cloned()
}

fn control(connection: &Connection, track: &Track, control: Control) {
    let Ok(proxy) = uncached(connection, track.player.clone(), PATH, PLAYER) else {
        return;
    };
    debug!(player = track.identity, ?control, "media control");
    let sent = match control {
        Control::PlayPause => proxy.call_method("PlayPause", &()),
        Control::Next => proxy.call_method("Next", &()),
        Control::Previous => proxy.call_method("Previous", &()),
        Control::Seek(seconds) => {
            let Some(id) = track.track_id.as_deref().and_then(|id| ObjectPath::try_from(id).ok()) else {
                return;
            };
            proxy.call_method("SetPosition", &(OwnedObjectPath::from(id), (seconds * 1e6) as i64))
        }
    };
    if let Err(error) = sent {
        warn!(player = track.identity, ?control, "media control failed: {error}");
    }
}

fn listen(connection: &Connection, events: Sender<Event>) {
    let rules = [
        MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .and_then(|rule| rule.path(PATH))
            .map(|rule| rule.build()),
        MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus")
            .and_then(|rule| rule.member("NameOwnerChanged"))
            .map(|rule| rule.build()),
    ];
    for rule in rules.into_iter().flatten() {
        let Ok(iterator) = MessageIterator::for_match_rule(rule, connection, None) else {
            continue;
        };
        let events = events.clone();
        let _ = thread::Builder::new().name("island-media-signals".into()).spawn(move || {
            for _ in iterator.flatten() {
                if events.send(Event::Changed).is_err() {
                    return;
                }
            }
        });
    }
}

fn worker(connection: &Connection, events: &Receiver<Event>, feed: &Feed<Option<Track>>) -> Ended {
    let mut preferred: Option<String> = None;
    let mut last = current(connection, &mut preferred);
    if let Some(track) = &last {
        preferred = Some(track.player.clone());
    }
    if !feed.publish(last.clone()) {
        return Ended::Unwanted;
    }
    let publish_current =|last: &mut Option<Track>, preferred: &mut Option<String>| {
        let current = current(connection, preferred);
        if let Some(track) = &current {
            *preferred = Some(track.player.clone());
        }
        let unchanged = match (&current, &*last) {
            (Some(new), Some(old)) => old.same(new),
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            match (&current, &*last) {
                (Some(new), old) if old.as_ref().is_none_or(|old| old.player != new.player) => {
                    info!(player = new.identity, bus = new.player, "following a media player");
                }
                (None, Some(old)) => info!(player = old.identity, "media player gone"),
                _ => {}
            }
            trace!(track = ?current, "track changed");
            *last = current.clone();
            return feed.publish(current);
        }
        true
    };
    loop {
        let wait = if last.as_ref().is_some_and(|track| track.playing) {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };
        match events.recv_timeout(wait) {
            Ok(Event::Control(command)) => {
                if let Some(track) = &last {
                    control(connection, track, command);
                }
            }
            Ok(Event::Changed) => loop {
                match events.recv_timeout(Duration::from_millis(50)) {
                    Ok(Event::Changed) => {}
                    Ok(Event::Control(command)) => {
                        if let Some(track) = &last {
                            control(connection, track, command);
                        }
                    }
                    Err(_) => break,
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        }
        if !publish_current(&mut last, &mut preferred) {
            return Ended::Unwanted;
        }
    }
}

/// Starts the supervised player worker, publishing the track to show to `feed`, and returns the
/// handle controls go through; `None` without a session bus.
pub fn start(feed: Feed<Option<Track>>) -> Option<Player> {
    let connection = Connection::session().ok()?;
    let (events, receiver) = mpsc::channel();
    let (controls, control_receiver) = mpsc::channel::<Control>();
    listen(&connection, events.clone());
    let forward = events.clone();
    thread::Builder::new()
        .name("island-media-controls".into())
        .spawn(move || {
            for control in control_receiver {
                if forward.send(Event::Control(control)).is_err() {
                    return;
                }
            }
        })
        .ok()?;
    feed.supervise(move |feed| worker(&connection, &receiver, feed));
    Some(Player { sender: controls })
}
