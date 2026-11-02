use std::time::Duration;

use futures::{
    StreamExt,
    channel::{mpsc::UnboundedSender, oneshot},
};
use zbus::{
    Connection, MatchRule, Message, MessageStream, ObjectPath, Proxy, block_on, connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    names::BusName,
    proxy::{Builder, CacheProperties},
};

use tracing::{debug, info};

use crate::{
    control::{self, Asked, Control, Report, Row},
    logging, profile, supervisor,
};

const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// For workers calling other apps: zbus waits forever for an app that never answers.
pub async fn session_with_timeout() -> zbus::Result<Connection> {
    connection::Builder::session()
        .method_timeout(CALL_TIMEOUT)
        .build()
        .await
}

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

/// Hands `each` every signal `rule` matches, on a thread of its own, until it answers false.
pub fn follow(
    name: &str,
    connection: &Connection,
    rule: MatchRule<'static>,
    mut each: impl FnMut(&Message) -> bool + Send + 'static,
) -> zbus::Result<()> {
    let mut stream = block_on(MessageStream::for_match_rule(rule, connection, None))?;
    supervisor::spawn(name, move || {
        block_on(async {
            while let Some(message) = stream.next().await {
                if let Ok(message) = message
                    && !each(&message)
                {
                    return;
                }
            }
        });
    });
    Ok(())
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

/// [`claim`] for what only the real island may own, which a preview leaves alone.
pub async fn claim_shared(connection: &Connection, name: &'static str) -> Result<bool, String> {
    if profile::preview() {
        return Ok(false);
    }
    claim(connection, name).await
}

/// Whether anything holds one of the names.
pub fn held(names: &[&str]) -> bool {
    block_on(async {
        let connection = Connection::session().await?;
        let bus = DBusProxy::new(&connection).await?;
        for name in names {
            if bus.name_has_owner(BusName::try_from(*name)?).await? {
                return Ok(true);
            }
        }
        Ok::<_, zbus::Error>(false)
    })
    .unwrap_or(false)
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
    let instance = profile::instance();
    let connection = block_on(async {
        let connection = connection::Builder::session()
            .serve_at(control::PATH, Controller { controls })
            .build()
            .await
            .map_err(|error| format!("cannot connect to the session bus: {error}"))?;
        claim(&connection, instance)
            .await
            .map(|claimed| claimed.then_some(connection))
    })?
    .ok_or("another island is already running")?;
    info!(name = instance, "claimed the instance name");
    Ok(connection)
}
