use zbus::{Connection, Proxy, block_on};

use super::report::Report;
use crate::cli::{Line, Mistake, not_running};

pub const SERVICE: &str = "io.github.krezh.Island";
pub const PATH: &str = "/io/github/krezh/Island";
pub const INTERFACE: &str = "io.github.krezh.Island.Control";

/// The answer `Run` sends back.
pub fn encode(answer: &Result<Report, String>) -> String {
    serde_json::to_string(answer).unwrap_or_default()
}

/// What island answered. One started before answers were sent back says nothing on success
/// and its mistake as text otherwise.
fn decode(reply: String) -> Result<Report, Mistake> {
    if reply.is_empty() {
        return Ok(Report::default());
    }
    match serde_json::from_str::<Result<Report, String>>(&reply) {
        Ok(answer) => answer.map_err(|error| Mistake::new(Line::new().plain(error))),
        Err(_) => Err(Mistake::new(Line::new().plain(reply))),
    }
}

/// Hands checked `args` to the running island, and gives back its answer.
pub fn send(args: &[String]) -> Result<Report, Mistake> {
    let reply: String = block_on(async {
        let connection = Connection::session()
            .await
            .map_err(|error| Mistake::new(Line::new().plain(format!("No session bus: {error}"))))?;
        let proxy = Proxy::new(&connection, SERVICE, PATH, INTERFACE)
            .await
            .map_err(|_| not_running())?;
        proxy.call("Run", &(args,)).await.map_err(|_| not_running())
    })?;
    decode(reply)
}
