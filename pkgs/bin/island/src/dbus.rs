//! island's own D-Bus name, which `island ctl` talks to, and helpers for talking to other services.

use futures::channel::mpsc::UnboundedSender;
use zbus::{
    Connection, ObjectPath, Proxy, block_on, connection,
    fdo::{RequestNameFlags, RequestNameReply},
    interface,
    names::BusName,
    proxy::{Builder, CacheProperties},
};

use tracing::{debug, info};

use crate::{
    control::{self, Control},
    logging,
};

const INSTANCE: &str = "io.github.krezh.Island";

/// A proxy for one read or call. zbus normally fetches every property of the object up front to
/// cache them; island never uses that cache, so it would only cost a round trip, and a warning
/// when the other side has just gone away.
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

/// Asks for `name` without queueing; false when another process has it.
pub async fn claim(connection: &Connection, name: &'static str) -> Result<bool, String> {
    match connection.request_name_with_flags(name, RequestNameFlags::DO_NOT_QUEUE).await {
        Ok(reply) => Ok(reply == RequestNameReply::PrimaryOwner),
        Err(zbus::Error::NameTaken) => Ok(false),
        Err(error) => Err(format!("cannot request {name}: {error}")),
    }
}

struct Controller {
    controls: UnboundedSender<Control>,
}

#[interface(name = "io.github.krezh.Island.Control")]
impl Controller {
    fn run(&self, args: Vec<String>) -> String {
        debug!(?args, "control command");
        match control::parse(&args) {
            Ok(Control::Log(spec)) => logging::set(&spec).err().unwrap_or_default(),
            Ok(command) => {
                let _ = self.controls.unbounded_send(command);
                String::new()
            }
            Err(error) => error,
        }
    }
}

/// Claims island's name, so only one island runs, and serves `island ctl` on it, handing its
/// commands to `controls`. The connection must be kept for as long as island runs.
pub fn serve(controls: UnboundedSender<Control>) -> Result<Connection, String> {
    let connection = block_on(async {
        let connection = connection::Builder::session()
            .serve_at(control::PATH, Controller { controls })
            .build()
            .await
            .map_err(|error| format!("cannot connect to the session bus: {error}"))?;
        claim(&connection, INSTANCE).await.map(|claimed| claimed.then_some(connection))
    })?
    .ok_or("another island is already running")?;
    info!(name = INSTANCE, "claimed the instance name");
    Ok(connection)
}
