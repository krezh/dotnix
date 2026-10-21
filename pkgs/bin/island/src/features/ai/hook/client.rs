use std::{
    io::{Read, Write},
    time::Duration,
};

use zbus::{block_on, connection, fdo::DBusProxy, names::BusName};

use super::server::{INTERFACE, NAME, PATH};
use crate::dbus;

/// Longer than any hook may run, so Claude Code's own timeout is the one that ends the wait.
const WAIT: Duration = Duration::from_secs(60 * 60);

/// Passes one Claude Code hook to the running island and prints its answer. It always exits 0, at
/// once when island is not running, so Claude Code carries on as if there were no hook.
pub fn run(args: &[String]) -> i32 {
    let Some(event) = args.first() else {
        eprintln!("usage: island ai-hook EVENT");
        return 0;
    };
    // island's own chat is not a session to follow.
    if std::env::var_os(crate::features::ai::converse::CHAT_ENV).is_some() {
        return 0;
    }
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }
    let reply: Result<String, zbus::Error> = block_on(async {
        let connection = connection::Builder::session()
            .method_timeout(WAIT)
            .build()
            .await?;
        let name = BusName::try_from(NAME)?;
        if !DBusProxy::new(&connection).await?.name_has_owner(name).await? {
            return Ok(String::new());
        }
        let proxy = dbus::uncached(&connection, NAME, PATH, INTERFACE).await?;
        proxy.call("Hook", &(event.as_str(), input.as_str())).await
    });
    if let Ok(reply) = reply
        && !reply.is_empty()
    {
        let mut out = std::io::stdout();
        let _ = out.write_all(reply.as_bytes());
        let _ = out.flush();
    }
    0
}
