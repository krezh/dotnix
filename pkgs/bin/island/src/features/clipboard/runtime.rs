use std::sync::mpsc::{self, Sender};

use tracing::error;

use super::{settings::Settings, state::State, watch};
use crate::{
    features::Feed,
    wake::{self, Waker},
};

#[derive(Clone)]
pub(super) enum Request {
    Select {
        id: i64,
        paste: bool,
    },
    Delete(i64),
    Pin {
        id: i64,
        pinned: bool,
    },
    /// Keeps what is pinned.
    Clear,
}

#[derive(Clone)]
pub struct Clipboard {
    requests: Sender<Request>,
    /// The worker waits on Wayland, so requests must wake it.
    wake: Option<Waker>,
}

impl Clipboard {
    pub(super) fn request(&self, request: Request) {
        if self.requests.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            wake.wake();
        }
    }
}

pub(super) fn start(feed: Feed<State>) -> Clipboard {
    let settings: Settings = feed.settings();
    let (requests, incoming) = mpsc::channel();
    let wake = match wake::pair() {
        Ok((waker, woken)) => {
            feed.supervise(move |feed| watch::run(feed, &incoming, &woken, &settings));
            Some(waker)
        }
        Err(error) => {
            error!("cannot start the clipboard worker: {error}");
            None
        }
    };
    Clipboard { requests, wake }
}
