use std::{
    collections::{HashMap, HashSet},
    env,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Mutex, PoisonError,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use futures::StreamExt;
use zbus::{
    Connection, MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Variant,
    block_on, fdo::DBusProxy, message::Type,
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
    art_url: Option<String>,
}

impl Track {
    pub fn position_now(&self) -> f64 {
        let elapsed = if self.playing {
            self.read_at.elapsed().as_secs_f64()
        } else {
            0.0
        };
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

fn text(metadata: &HashMap<String, OwnedVariant>, key: &str) -> String {
    metadata
        .get(key)
        .and_then(|value| match &**value {
            Variant::Str(text) => Some(text.to_string()),
            Variant::Array(items) => {
                let joined: Vec<String> = items
                    .iter()
                    .filter_map(|item| match item {
                        Variant::Str(text) => Some(text.to_string()),
                        _ => None,
                    })
                    .collect();
                Some(joined.join(", "))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn micros(metadata: &HashMap<String, OwnedVariant>, key: &str) -> Option<f64> {
    metadata.get(key).and_then(|value| match &**value {
        Variant::I64(micros) => Some(*micros as f64 / 1e6),
        Variant::U64(micros) => Some(*micros as f64 / 1e6),
        _ => None,
    })
}

/// An `http` one not cached yet is downloaded in the background, at most once per run, and
/// `changed` hears when it is in.
fn art(url: &str, changed: &Sender<Event>) -> Option<PathBuf> {
    static STARTED: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

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
    let fresh = STARTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_default()
        .insert(path.clone());
    if fresh {
        let (url, changed) = (url.to_owned(), changed.clone());
        let _ = thread::Builder::new()
            .name("island-media-art".into())
            .spawn(move || {
                if download(&url, &dir, &path).is_some() {
                    let _ = changed.send(Event::Changed);
                } else {
                    debug!(url, "cannot download the album art");
                }
            });
    }
    None
}

fn download(url: &str, dir: &Path, path: &Path) -> Option<()> {
    std::fs::create_dir_all(dir).ok()?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let response = agent.get(url).call().ok()?;
    let bytes = response
        .into_body()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_vec()
        .ok()?;
    let partial = path.with_extension("part");
    std::fs::write(&partial, bytes).ok()?;
    std::fs::rename(&partial, path).ok()
}

async fn read(connection: &Connection, player: &str) -> Option<Track> {
    let proxy = uncached(connection, player.to_owned(), PATH, PLAYER)
        .await
        .ok()?;
    let status: String = proxy.get_property("PlaybackStatus").await.ok()?;
    let metadata: HashMap<String, OwnedVariant> =
        proxy.get_property("Metadata").await.unwrap_or_default();
    let position = proxy
        .get_property::<i64>("Position")
        .await
        .map_or(0.0, |micros| micros as f64 / 1e6);
    let identity = match uncached(
        connection,
        player.to_owned(),
        PATH,
        "org.mpris.MediaPlayer2",
    )
    .await
    {
        Ok(root) => root.get_property::<String>("Identity").await.ok(),
        Err(_) => None,
    }
    .unwrap_or_else(|| player.trim_start_matches(PREFIX).to_owned());
    let track_id = metadata
        .get("mpris:trackid")
        .and_then(|value| match &**value {
            Variant::ObjectPath(path) => Some(path.to_string()),
            Variant::Str(path) => Some(path.to_string()),
            _ => None,
        });
    Some(Track {
        player: player.to_owned(),
        identity,
        title: text(&metadata, "xesam:title"),
        artist: text(&metadata, "xesam:artist"),
        album: text(&metadata, "xesam:album"),
        art: None,
        art_url: Some(text(&metadata, "mpris:artUrl")).filter(|url| !url.is_empty()),
        length: micros(&metadata, "mpris:length").filter(|length| *length > 0.0),
        position,
        playing: status == "Playing",
        read_at: Instant::now(),
        track_id,
    })
}

fn current(
    connection: &Connection,
    preferred: &Option<String>,
    changed: &Sender<Event>,
) -> Option<Track> {
    let names = block_on(async { DBusProxy::new(connection).await?.list_names().await }).ok()?;
    let tracks: Vec<Track> = names
        .iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with(PREFIX))
        .filter_map(|name| block_on(read(connection, &name)))
        .filter(|track| !track.title.is_empty())
        .collect();
    tracks
        .iter()
        .find(|track| track.playing)
        .or_else(|| {
            tracks
                .iter()
                .find(|track| Some(&track.player) == preferred.as_ref())
        })
        .or_else(|| tracks.first())
        .cloned()
        .map(|track| Track {
            art: track.art_url.as_deref().and_then(|url| art(url, changed)),
            ..track
        })
}

fn control(connection: &Connection, track: &Track, control: Control) {
    let Ok(proxy) = block_on(uncached(connection, track.player.clone(), PATH, PLAYER)) else {
        return;
    };
    debug!(player = track.identity, ?control, "media control");
    let sent = match control {
        Control::PlayPause => block_on(proxy.call_method("PlayPause", &())),
        Control::Next => block_on(proxy.call_method("Next", &())),
        Control::Previous => block_on(proxy.call_method("Previous", &())),
        Control::Seek(seconds) => {
            let Some(id) = track
                .track_id
                .as_deref()
                .and_then(|id| ObjectPath::try_from(id).ok())
            else {
                return;
            };
            block_on(proxy.call_method(
                "SetPosition",
                &(OwnedObjectPath::from(id), (seconds * 1e6) as i64),
            ))
        }
    };
    if let Err(error) = sent {
        warn!(
            player = track.identity,
            ?control,
            "media control failed: {error}"
        );
    }
}

fn listen(connection: &Connection, events: Sender<Event>) {
    let rules = [
        MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .path(PATH)
            .build(),
        MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(PLAYER)
            .member("Seeked")
            .path(PATH)
            .build(),
        MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus")
            .member("NameOwnerChanged")
            .arg0ns("org.mpris.MediaPlayer2")
            .build(),
    ];
    for rule in rules.into_iter().flatten() {
        let Ok(mut stream) = block_on(MessageStream::for_match_rule(rule, connection, None)) else {
            continue;
        };
        let events = events.clone();
        let _ = thread::Builder::new()
            .name("island-media-signals".into())
            .spawn(move || {
                block_on(async {
                    while let Some(message) = stream.next().await {
                        if message.is_ok() && events.send(Event::Changed).is_err() {
                            return;
                        }
                    }
                });
            });
    }
}

fn worker(
    connection: &Connection,
    events: &Receiver<Event>,
    changed: &Sender<Event>,
    feed: &Feed<Option<Track>>,
) -> Ended {
    let mut preferred: Option<String> = None;
    let mut last = current(connection, &preferred, changed);
    if let Some(track) = &last {
        preferred = Some(track.player.clone());
    }
    if !feed.publish(last.clone()) {
        return Ended::Unwanted;
    }
    let publish_current = |last: &mut Option<Track>, preferred: &mut Option<String>| {
        let current = current(connection, preferred, changed);
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
                    info!(
                        player = new.identity,
                        bus = new.player,
                        "following a media player"
                    );
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
        match events.recv() {
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
            Err(_) => return Ended::Unwanted,
        }
        if !publish_current(&mut last, &mut preferred) {
            return Ended::Unwanted;
        }
    }
}

/// `None` without a session bus.
pub fn start(feed: Feed<Option<Track>>) -> Option<Player> {
    let connection = block_on(Connection::session()).ok()?;
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
    feed.supervise(move |feed| worker(&connection, &receiver, &events, feed));
    Some(Player { sender: controls })
}
