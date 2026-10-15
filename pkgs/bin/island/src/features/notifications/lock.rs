//! Follows whether the session is locked, from logind, so notification contents can hide meanwhile.

use zbus::{
    MatchRule,
    blocking::{Connection, MessageIterator},
    message::Type,
};

use tracing::{debug, info};

use super::{Event, State};
use crate::{dbus::uncached, features::Feed, supervisor::Ended};

const LOGIN: &str = "org.freedesktop.login1";
const SESSION: &str = "/org/freedesktop/login1/session/auto";
const INTERFACE: &str = "org.freedesktop.login1.Session";

fn locked(connection: &Connection) -> Option<bool> {
    uncached(connection, LOGIN, SESSION, INTERFACE)
        .ok()?
        .get_property::<bool>("LockedHint")
        .ok()
}

pub fn follow(feed: &Feed<State, Event>) -> Ended {
    let connection = match Connection::system() {
        Ok(connection) => connection,
        Err(error) => return Ended::Failed(error.to_string()),
    };
    let mut last = locked(&connection).unwrap_or(false);
    debug!(locked = last, "watching the session lock");
    if !feed.send(Event::Locked(last)) {
        return Ended::Unwanted;
    }
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(LOGIN)
        .and_then(|rule| rule.interface("org.freedesktop.DBus.Properties"))
        .and_then(|rule| rule.member("PropertiesChanged"))
        .map(|rule| rule.build());
    let iterator = match rule.and_then(|rule| MessageIterator::for_match_rule(rule, &connection, None)) {
        Ok(iterator) => iterator,
        Err(error) => return Ended::Failed(error.to_string()),
    };
    for _ in iterator.flatten() {
        let current = locked(&connection).unwrap_or(last);
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
