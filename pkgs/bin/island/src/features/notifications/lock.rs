use futures::StreamExt;
use zbus::{Connection, block_on};

use tracing::{debug, info};

use super::{Event, State};
use crate::{
    features::Feed,
    session_lock::{changes, locked},
    supervisor::Ended,
};

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
    let mut stream = match changes(&connection).await {
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
