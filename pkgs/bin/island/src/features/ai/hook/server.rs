use std::sync::mpsc::{Receiver, Sender};

use futures::channel::oneshot;
use tracing::{debug, info, warn};
use zbus::{block_on, connection, interface};

use super::input::{Hook, parse};
use crate::{dbus, supervisor::Ended};

pub const NAME: &str = "io.github.krezh.Island.Ai";
pub const PATH: &str = "/io/github/krezh/Island/Ai";
pub const INTERFACE: &str = "io.github.krezh.Island.Ai";

/// A hook call on its way to the feature's worker.
pub enum Call {
    Tell(Hook),
    /// Waits for what to print.
    Ask(Hook, oneshot::Sender<String>),
}

struct Server {
    calls: Sender<Call>,
}

#[interface(name = "io.github.krezh.Island.Ai")]
impl Server {
    /// What the hook prints: a decision as JSON, or nothing.
    async fn hook(&self, event: String, input: String) -> String {
        let hook = match parse(&event, &input) {
            Ok(hook) => hook,
            Err(error) => {
                warn!(event, "cannot read a Claude Code hook: {error}");
                return String::new();
            }
        };
        debug!(event, session = hook.from().session, "Claude Code hook");
        if !hook.blocks() {
            let _ = self.calls.send(Call::Tell(hook));
            return String::new();
        }
        let (reply, answer) = oneshot::channel();
        if self.calls.send(Call::Ask(hook, reply)).is_err() {
            return String::new();
        }
        answer.await.unwrap_or_default()
    }
}

/// Serves until `stop` closes, which it does once the feature is gone.
pub fn serve(calls: Sender<Call>, stop: &Receiver<()>) -> Ended {
    let built = block_on(
        connection::Builder::session()
            .serve_at(PATH, Server { calls })
            .build(),
    );
    let connection = match built {
        Ok(connection) => connection,
        Err(error) => return Ended::Failed(format!("cannot connect to the session bus: {error}")),
    };
    match block_on(dbus::claim(&connection, NAME)) {
        Ok(true) => info!(name = NAME, "answering Claude Code hooks"),
        Ok(false) => warn!(
            name = NAME,
            "another island answers Claude Code hooks; sessions are off here"
        ),
        Err(error) => return Ended::Failed(error),
    }
    while stop.recv().is_ok() {}
    drop(connection);
    Ended::Unwanted
}
