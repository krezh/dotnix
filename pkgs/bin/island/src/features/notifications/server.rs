use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, mpsc::Receiver},
    time::{Duration, SystemTime},
};

use gpui::RenderImage;
use tracing::{debug, info, trace, warn};
use zbus::{Connection, OwnedVariant, block_on, connection, interface};

use super::{
    Event, State,
    notification::{Action, Expire, Notification, Signal, next_id},
};
use crate::{activation::Activation, components::icon::Picture, dbus, features::Feed, icons, supervisor::Ended};

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

struct Server {
    feed: Feed<State, Event>,
}

fn urgency(hints: &HashMap<String, OwnedVariant>) -> u8 {
    hints
        .get("urgency")
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(1)
}

fn hint_string(hints: &HashMap<String, OwnedVariant>, key: &str) -> Option<String> {
    hints
        .get(key)
        .and_then(|value| String::try_from(value.clone()).ok())
        .filter(|value| !value.is_empty())
}

fn hint_bool(hints: &HashMap<String, OwnedVariant>, key: &str) -> bool {
    hints.get(key).and_then(|value| bool::try_from(value).ok()).unwrap_or(false)
}

fn icon_reference(reference: &str) -> Option<Picture> {
    let reference = reference.strip_prefix("file://").unwrap_or(reference);
    if reference.is_empty() {
        return None;
    }
    let path = Path::new(reference);
    if path.is_absolute() {
        return path.exists().then(|| Picture::File(path.to_path_buf()));
    }
    icons::lookup(reference).map(Picture::File)
}

type RawImage = (i32, i32, i32, bool, i32, i32, Vec<u8>);

fn pixels((width, height, stride, alpha, bits, channels, data): RawImage) -> Option<Picture> {
    let (width, height, stride) = (usize::try_from(width).ok()?, usize::try_from(height).ok()?, usize::try_from(stride).ok()?);
    let channels = usize::try_from(channels).ok()?;
    if bits != 8 || width == 0 || height == 0 || channels < 3 || (alpha && channels < 4) {
        return None;
    }
    let mut bgra = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let line = data.get(row * stride..row * stride + width * channels)?;
        for pixel in line.chunks_exact(channels) {
            let opacity = if alpha { pixel[3] } else { u8::MAX };
            bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], opacity]);
        }
    }
    let buffer = image::RgbaImage::from_raw(width as u32, height as u32, bgra)?;
    Some(Picture::Pixels(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))))
}

fn image_hint(hints: &HashMap<String, OwnedVariant>) -> Option<Picture> {
    ["image-data", "image_data", "icon_data"]
        .into_iter()
        .find_map(|key| hints.get(key))
        .and_then(|value| RawImage::try_from(value.clone()).ok())
        .and_then(pixels)
        .or_else(|| {
            ["image-path", "image_path"]
                .into_iter()
                .find_map(|key| hint_string(hints, key))
                .and_then(|reference| icon_reference(&reference))
        })
}

fn app_icon(app_icon: &str, app: &str, hints: &HashMap<String, OwnedVariant>) -> Option<Picture> {
    icon_reference(app_icon)
        .or_else(|| hint_string(hints, "desktop-entry").and_then(|entry| icons::lookup(&entry)).map(Picture::File))
        .or_else(|| icons::lookup(&app.to_lowercase().replace(' ', "-")).map(Picture::File))
}

#[interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&'static str> {
        vec!["actions", "action-icons", "body", "icon-static", "persistence"]
    }

    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("island", "krezh", env!("CARGO_PKG_VERSION"), "1.3")
    }

    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedVariant>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id == 0 { next_id() } else { replaces_id };
        let app = if app_name.is_empty() {
            hint_string(&hints, "desktop-entry").unwrap_or_else(|| "Notification".to_owned())
        } else {
            app_name
        };
        let action_icons = hint_bool(&hints, "action-icons");
        let expire = match expire_timeout {
            0 => Expire::Never,
            timeout if timeout > 0 => Expire::After(Duration::from_millis(timeout.unsigned_abs().into())),
            _ => Expire::Default,
        };
        let notification = Notification {
            id,
            icon: self::app_icon(&app_icon, &app, &hints),
            image: image_hint(&hints),
            transient: hint_bool(&hints, "transient"),
            resident: hint_bool(&hints, "resident"),
            app,
            summary,
            body,
            critical: urgency(&hints) >= 2,
            expire,
            actions: actions
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[key, label]| Action {
                    key: key.clone(),
                    label: label.clone(),
                    icon: action_icons.then(|| icons::lookup(label)).flatten(),
                })
                .collect(),
            received: SystemTime::now(),
        };
        debug!(
            id,
            app = notification.app,
            replaces = replaces_id != 0,
            critical = notification.critical,
            "notification received"
        );
        trace!(id, summary = notification.summary, body = notification.body, "notification content");
        self.feed.send(Event::Show(notification));
        id
    }

    fn close_notification(&self, id: u32) {
        debug!(id, "notification closed by its app");
        self.feed.send(Event::Close(id));
    }
}

pub fn serve(feed: &Feed<State, Event>, signals: &Receiver<Signal>) -> Ended {
    let built = block_on(connection::Builder::session().serve_at(PATH, Server { feed: feed.clone() }).build());
    let connection = match built {
        Ok(connection) => connection,
        Err(error) => return Ended::Failed(format!("cannot connect to the session bus: {error}")),
    };
    match block_on(dbus::claim(&connection, NAME)) {
        Ok(true) => info!(name = NAME, "serving notifications"),
        Ok(false) => warn!(name = NAME, "another notification daemon owns the name; notifications are off"),
        Err(error) => return Ended::Failed(error),
    }
    let mut activation = None;
    for signal in signals {
        emit(&connection, &mut activation, signal);
    }
    Ended::Unwanted
}

/// Remakes the connection if it broke.
fn activation_token(activation: &mut Option<Activation>) -> Option<String> {
    if activation.is_none() {
        *activation = Activation::connect();
    }
    let token = activation.as_mut()?.token();
    if token.is_none() {
        *activation = None;
    }
    token
}

fn emit(connection: &Connection, activation: &mut Option<Activation>, signal: Signal) {
    let sent = match signal {
        Signal::Closed(id, reason) => {
            debug!(id, ?reason, "notification closed");
            block_on(connection.emit_signal(None::<&str>, PATH, NAME, "NotificationClosed", &(id, reason as u32)))
        }
        Signal::Action(id, key) => {
            debug!(id, action = key, "notification action invoked");
            if let Some(token) = activation_token(activation)
                && let Err(error) =
                    block_on(connection.emit_signal(None::<&str>, PATH, NAME, "ActivationToken", &(id, token)))
            {
                warn!("cannot emit a notification signal: {error}");
            }
            block_on(connection.emit_signal(None::<&str>, PATH, NAME, "ActionInvoked", &(id, key)))
        }
    };
    if let Err(error) = sent {
        warn!("cannot emit a notification signal: {error}");
    }
}
