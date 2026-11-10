//! Whether the session is locked, as logind's `LockedHint` tells it, and hearing it unlock.

use std::thread;

use futures::StreamExt;
use tracing::{debug, warn};
use zbus::{Connection, MatchRule, MessageStream, block_on, message::Type};

use crate::dbus::uncached;

const LOGIN: &str = "org.freedesktop.login1";
const SESSION: &str = "/org/freedesktop/login1/session/auto";
const INTERFACE: &str = "org.freedesktop.login1.Session";

pub async fn locked(connection: &Connection) -> Option<bool> {
    uncached(connection, LOGIN, SESSION, INTERFACE)
        .await
        .ok()?
        .get_property::<bool>("LockedHint")
        .await
        .ok()
}

/// logind's property changes, after each of which the lock may have changed.
pub async fn changes(connection: &Connection) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(LOGIN)
        .interface("org.freedesktop.DBus.Properties")
        .member("PropertiesChanged")
        .build()?;
    MessageStream::for_match_rule(rule, connection, None).await
}

async fn follow(unlocked: &mut impl FnMut()) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let mut stream = changes(&connection).await?;
    let mut last = locked(&connection).await.unwrap_or(false);
    while let Some(message) = stream.next().await {
        if message.is_err() {
            continue;
        }
        let current = locked(&connection).await.unwrap_or(last);
        if last && !current {
            debug!("the session unlocked");
            unlocked();
        }
        last = current;
    }
    Ok(())
}

pub fn listen_unlocked(mut unlocked: impl FnMut() + Send + 'static) {
    let spawned = thread::Builder::new()
        .name("island-unlock".into())
        .spawn(move || {
            if let Err(error) = block_on(follow(&mut unlocked)) {
                warn!("cannot hear the session unlock: {error}");
            }
        });
    if let Err(error) = spawned {
        warn!("cannot listen for the session unlocking: {error}");
    }
}
