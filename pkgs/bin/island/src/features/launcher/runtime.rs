use std::sync::Arc;

use super::{
    desktop, history,
    index::{Index, Refresh},
    settings::Settings,
    state::Apps,
};
use crate::{features::Feed, supervisor::Ended};

#[derive(Clone)]
pub struct Launcher {
    pub(super) settings: Arc<Settings>,
    refresh: Refresh,
}

pub(super) fn start(feed: Feed<Apps>) -> Launcher {
    let settings = Arc::new(feed.settings());
    let (refresh, requests) = Refresh::channel();
    let requested = refresh.clone();
    feed.supervise(move |feed| {
        history::load();
        let mut index = match Index::new(desktop::dirs(), requested.clone()) {
            Ok(index) => index,
            Err(error) => return Ended::Failed(error),
        };
        loop {
            match index.scan() {
                Ok(Some(apps)) => {
                    if !feed.publish(Apps(apps)) {
                        return Ended::Unwanted;
                    }
                }
                Ok(None) => {}
                Err(error) => return Ended::Failed(error),
            }
            if !Refresh::wait(&requests) {
                return Ended::Unwanted;
            }
        }
    });
    Launcher { settings, refresh }
}

impl Launcher {
    pub(super) fn refresh(&self) {
        self.refresh.request();
    }
}
