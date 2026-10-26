use futures::channel::{mpsc::UnboundedSender, oneshot};
use zbus::{
    Connection, ObjectPath, Proxy, block_on, connection,
    fdo::{RequestNameFlags, RequestNameReply},
    interface,
    names::BusName,
    proxy::{Builder, CacheProperties},
};

use tracing::{debug, info};

use crate::{
    control::{self, Asked, Control, Report, Row},
    logging,
};

const INSTANCE: &str = "io.github.krezh.Island";

/// For one read or call. zbus normally fetches every property up front to cache them; island never
/// uses that cache, so it would only cost a round trip, and a warning when the other side has just
/// gone away.
pub async fn uncached<'a>(
    connection: &Connection,
    destination: impl TryInto<BusName<'a>, Error: Into<zbus::Error>>,
    path: impl TryInto<ObjectPath<'a>, Error: Into<zbus::Error>>,
    interface: &'a str,
) -> zbus::Result<Proxy<'a>> {
    Builder::<Proxy>::new(connection)
        .destination(destination)
        .path(path)
        .interface(interface)
        .cache_properties(CacheProperties::No)
        .build()
        .await
}

/// Without queueing; false when another process has the name.
pub async fn claim(connection: &Connection, name: &'static str) -> Result<bool, String> {
    match connection
        .request_name_with_flags(name, RequestNameFlags::DO_NOT_QUEUE)
        .await
    {
        Ok(reply) => Ok(reply == RequestNameReply::PrimaryOwner),
        Err(zbus::Error::NameTaken) => Ok(false),
        Err(error) => Err(format!("cannot request {name}: {error}")),
    }
}

struct Controller {
    controls: UnboundedSender<Asked>,
}

#[interface(name = "io.github.krezh.Island.Control")]
impl Controller {
    /// Waits for island's answer, so the caller can show what changed.
    async fn run(&self, args: Vec<String>) -> String {
        debug!(?args, "control command");
        let answer = match control::parse(&args) {
            Ok(Control::Log(spec)) => logging::set(&spec)
                .map(|()| Report::did(Row::said(format!("Logging {spec} until island restarts")))),
            Ok(control) => {
                let (answer, answered) = oneshot::channel();
                match self.controls.unbounded_send(Asked { control, answer }) {
                    Ok(()) => answered
                        .await
                        .unwrap_or_else(|_| Err("island stopped before it answered".to_owned())),
                    Err(_) => Err("island is shutting down".to_owned()),
                }
            }
            Err(mistake) => Err(mistake.plain()),
        };
        control::encode(&answer)
    }
}

/// Claiming the name means only one island runs. Keep the connection for as long as island runs.
pub fn serve(controls: UnboundedSender<Asked>) -> Result<Connection, String> {
    let connection = block_on(async {
        let connection = connection::Builder::session()
            .serve_at(control::PATH, Controller { controls })
            .build()
            .await
            .map_err(|error| format!("cannot connect to the session bus: {error}"))?;
        claim(&connection, INSTANCE)
            .await
            .map(|claimed| claimed.then_some(connection))
    })?
    .ok_or("another island is already running")?;
    info!(name = INSTANCE, "claimed the instance name");
    Ok(connection)
}
