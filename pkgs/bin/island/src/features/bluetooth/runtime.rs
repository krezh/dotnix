use std::sync::{Arc, Mutex};

use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::{state::State, worker};
use crate::features::Feed;

#[derive(Clone, Debug)]
pub(super) enum Request {
    /// On, off, or the other way when `None`.
    Power(Option<bool>),
    Scan,
    /// Ends the scan started as `generation`, unless a newer one started since.
    StopScan(u64),
    Visible,
    Connect(String),
    Disconnect(String),
    Pair(String),
    Trust(String, bool),
    Forget(String),
    /// Yes or no to the pairing code asked as `id`.
    Answer(u64, bool),
    /// Something slow on its own thread finished with the device, such as connecting.
    Done(String),
    /// The agent was asked something.
    Refresh,
}

#[derive(Clone)]
pub struct Bluetooth {
    requests: UnboundedSender<Request>,
}

impl Bluetooth {
    pub(super) fn start(feed: Feed<State>) -> Self {
        let (requests, received) = mpsc::unbounded();
        let received: Arc<Mutex<UnboundedReceiver<Request>>> = Arc::new(Mutex::new(received));
        let sender = requests.clone();
        feed.supervise(move |feed| worker::run(feed, &received, &sender));
        Self { requests }
    }

    pub(super) fn send(&self, request: Request) {
        let _ = self.requests.unbounded_send(request);
    }
}
