use std::{
    collections::HashMap,
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

use tracing::{error, warn};

use super::state::Previews;
use crate::{
    apps::{self, App},
    capture::{Capturer, News},
    compositor,
    features::Feed,
    picture::Picture,
    share,
    supervisor::Ended,
    sync::lock,
    wake::{self, Waker, Woken},
};

pub(super) enum Request {
    /// A switcher shows, or stops showing, so the pictures are kept live, or no longer.
    Watch(bool),
    /// The windows a switcher has in view, the only ones kept live.
    InView(Vec<String>),
    Focus(String),
    Close(String),
}

/// Alt+Tab: the windows, most recently used first, to switch between with Alt held. Its worker
/// captures them, finds their apps' names and icons, and focuses and closes them.
#[derive(Clone)]
pub struct Switcher {
    requests: Sender<Request>,
    /// The worker waits on Wayland, so requests must wake it.
    wake: Option<Waker>,
    /// The windows last said to be in view, so the worker is told only of a change.
    in_view: Arc<Mutex<Vec<String>>>,
}

/// Held while a switcher shows, keeping the pictures live.
pub(super) struct Watch(Switcher);

impl Drop for Watch {
    fn drop(&mut self) {
        self.0.request(Request::Watch(false));
    }
}

impl Switcher {
    fn request(&self, request: Request) {
        if self.requests.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            wake.wake();
        }
    }

    pub(super) fn watch(&self) -> Watch {
        self.request(Request::Watch(true));
        Watch(self.clone())
    }

    pub(super) fn focus(&self, address: &str) {
        self.request(Request::Focus(address.to_owned()));
    }

    pub(super) fn close(&self, address: &str) {
        self.request(Request::Close(address.to_owned()));
    }

    /// Keeps live only the windows of these identifiers.
    pub(super) fn in_view(&self, ids: Vec<String>) {
        let mut last = lock(&self.in_view);
        if *last != ids {
            last.clone_from(&ids);
            drop(last);
            self.request(Request::InView(ids));
        }
    }
}

pub(super) fn start(feed: Feed<Previews>) -> Switcher {
    let (requests, incoming) = mpsc::channel();
    let wake = match wake::pair() {
        Ok((waker, woken)) => {
            feed.supervise(move |feed| run(feed, &incoming, &woken));
            let windows = waker.clone();
            compositor::on_change(move || windows.wake());
            let offered = waker.clone();
            share::on_offer(move || offered.wake());
            Some(waker)
        }
        Err(error) => {
            error!("cannot start the switcher worker: {error}");
            None
        }
    };
    Switcher {
        requests,
        wake,
        in_view: Arc::default(),
    }
}

fn serve(request: Request, watchers: &mut usize, capturer: &mut Capturer) {
    match request {
        Request::Watch(true) => *watchers += 1,
        Request::Watch(false) => *watchers = watchers.saturating_sub(1),
        Request::InView(ids) => capturer.want(Some(&ids)),
        Request::Focus(address) => {
            if let Some(backend) = compositor::backend() {
                backend.focus_window(&address);
            }
        }
        Request::Close(address) => {
            if let Some(backend) = compositor::backend()
                && let Err(error) = backend.close_window(&address)
            {
                warn!("{error}");
            }
        }
    }
}

/// Sleeps until the compositor or a request has something for it.
fn run(feed: &Feed<Previews>, incoming: &Receiver<Request>, wake: &Woken) -> Ended {
    let mut capturer = match Capturer::connect() {
        Ok(capturer) => capturer,
        Err(error) => return Ended::Failed(format!("cannot capture windows: {error}")),
    };
    let mut found: Arc<HashMap<String, App>> = Arc::default();
    let mut icons: Arc<HashMap<PathBuf, Picture>> = Arc::default();
    let mut previews = Previews::default();
    let mut watchers = 0;
    loop {
        for request in incoming.try_iter() {
            serve(request, &mut watchers, &mut capturer);
        }
        capturer.watch(watchers > 0);
        let windows = compositor::windows();
        let moved = !Arc::ptr_eq(&windows, &previews.windows);
        previews.windows = windows;
        match capturer.turn() {
            Err(error) => return Ended::Failed(error),
            Ok(News::Changed) => {
                for app in capturer.apps() {
                    if found.contains_key(app) {
                        continue;
                    }
                    let app_found = apps::app(app);
                    if let Some(path) = &app_found.icon
                        && !icons.contains_key(path)
                        && let Some(icon) = Picture::read(path)
                    {
                        Arc::make_mut(&mut icons).insert(path.clone(), icon);
                    }
                    Arc::make_mut(&mut found).insert(app.to_owned(), app_found);
                }
                previews.shots = Arc::new(capturer.shots().clone());
                previews.apps = found.clone();
                previews.icons = icons.clone();
                if !feed.publish(previews.clone()) {
                    return Ended::Unwanted;
                }
            }
            Ok(_) if moved => {
                if !feed.publish(previews.clone()) {
                    return Ended::Unwanted;
                }
            }
            Ok(News::Moved) => {
                feed.send(());
            }
            Ok(News::Nothing) => {}
        }
        match capturer.wait(wake.as_raw_fd()) {
            Err(error) => return Ended::Failed(error),
            Ok(true) => wake.drain(),
            Ok(false) => {}
        }
    }
}
