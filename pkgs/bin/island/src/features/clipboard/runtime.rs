use std::{
    io::Write,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
};

use tracing::error;

use super::{settings::Settings, state::State, watch};
use crate::{
    features::{Feature, Feed},
    island::Panel,
};

pub(super) enum Request {
    Select { id: i64, paste: bool },
    Delete(i64),
    Clear,
}

#[derive(Clone)]
pub struct Clipboard {
    requests: Sender<Request>,
    /// The worker waits on Wayland, so requests must wake it.
    wake: Option<Arc<UnixStream>>,
}

impl Clipboard {
    pub(super) fn request(&self, request: Request) {
        if self.requests.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            let _ = (&**wake).write(&[1]);
        }
    }

    pub(super) fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

pub(super) fn start(feed: Feed<State>) -> Clipboard {
    let settings: Settings = feed.settings();
    let (requests, incoming) = mpsc::channel();
    let wake = match UnixStream::pair()
        .and_then(|(reader, writer)| reader.set_nonblocking(true).map(|()| (reader, writer)))
    {
        Ok((reader, writer)) => {
            feed.supervise(move |feed| watch::run(feed, &incoming, &reader, &settings));
            Some(Arc::new(writer))
        }
        Err(error) => {
            error!("cannot start the clipboard worker: {error}");
            None
        }
    };
    Clipboard { requests, wake }
}
