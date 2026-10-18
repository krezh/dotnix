use std::time::Instant;

use tracing::error;

use super::sni::{self, Client, Command, TrayItem};
use crate::features::Feed;

#[derive(Clone)]
pub struct Tray {
    client: Option<Client>,
    pub(super) epoch: Instant,
}

impl Tray {
    pub(super) fn send(&self, command: Command) {
        if let Some(client) = &self.client {
            client.send(command);
        }
    }

    pub(super) fn start(feed: Feed<Vec<TrayItem>>) -> Self {
        let client = sni::start(feed).inspect_err(|error| error!("{error}")).ok();
        Self {
            client,
            epoch: Instant::now(),
        }
    }
}
