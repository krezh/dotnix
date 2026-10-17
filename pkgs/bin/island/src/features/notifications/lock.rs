//! Follows whether the session is locked, from logind, so notification contents can hide meanwhile.

use futures::StreamExt;
use zbus::{Connection, MatchRule, MessageStream, block_on, message::Type};

use tracing::{debug, info};

use super::{Event, State};
use crate::{dbus::uncached, features::Feed, supervisor::Ended};

const LOGIN: &str = "org.freedesktop.login1";
const SESSION: &str = "/org/freedesktop/login1/session/auto";
const INTERFACE: &str = "org.freedesktop.login1.Session";

async fn locked(connection: &Connection) -> Option<bool> {
    uncached(connection, LOGIN, SESSION, INTERFACE)
        .await
        .ok()?
        .get_property::<bool>("LockedHint")
        .await
        .ok()
}

pub fn follow(feed: &Feed<State, Event>) -> Ended {
    block_on(watch(feed))
}

async fn watch(feed: &Feed<State, Event>) -> Ended {
    let connection = match Connection::system().await {
        Ok(connection) => connection,
        Err(error) => return Ended::Failed(error.to_string()),
    };
    let mut last = locked(&connection).await.unwrap_or(false);
    debug!(locked = last, "watching the session lock");
    if !feed.send(Event::Locked(last)) {
        return Ended::Unwanted;
    }
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(LOGIN)
        .interface("org.freedesktop.DBus.Properties")
        .member("PropertiesChanged")
        .build();
    let mut stream = match async { MessageStream::for_match_rule(rule?, &connection, None).await }.await {
        Ok(stream) => stream,
        Err(error) => return Ended::Failed(error.to_string()),
    };
    while let Some(message) = stream.next().await {
        if message.is_err() {
            continue;
        }
        let current = locked(&connection).await.unwrap_or(last);
        if current != last {
            info!(locked = current, "session lock changed");
            last = current;
            if !feed.send(Event::Locked(current)) {
                return Ended::Unwanted;
            }
        }
    }
    Ended::Failed("the system bus closed".to_owned())
}
