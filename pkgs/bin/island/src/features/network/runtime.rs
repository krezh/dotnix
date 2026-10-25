use std::{
    collections::VecDeque,
    net::Ipv4Addr,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use chrono::{Days, Local};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use serde::Deserialize;
use tracing::warn;

use super::{probe, state::State, worker};
use crate::features::Feed;

/// How often what the charts show is measured, while the panel is open.
const SAMPLE_EVERY: Duration = Duration::from_secs(1);
const PROBE_EVERY: Duration = Duration::from_secs(2);
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
/// Pings to the internet that the loss is counted over.
const LOSS_WINDOW: usize = 20;
/// Every so many probes, a DNS lookup too.
const LOOKUP_EVERY: u64 = 5;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    /// An address on the internet to time pings to.
    pub latency_target: String,
    /// A name whose lookup is timed.
    pub dns_probe: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            latency_target: "1.1.1.1".to_owned(),
            dns_probe: "example.com".to_owned(),
        }
    }
}

#[derive(Debug)]
pub(super) enum Request {
    /// On, off, or the other way when `None`.
    Wifi(Option<bool>),
    Scan,
    /// A saved network, or an open one.
    Connect(String),
    Join(String, String),
    Disconnect,
    Forget(String),
    Vpn(String, Option<bool>),
    /// The panel opened.
    Watch,
    /// Time to measure what the charts show, while the panel is open.
    Sample,
    /// A new day began, so the day's data used is written down.
    Midnight,
    /// Something may have moved on without a signal saying so, such as a join that timed out.
    Refresh,
}

/// What the probes last measured; `round` moves on with each.
#[derive(Debug, Default)]
pub(super) struct Probes {
    pub round: u64,
    pub gateway: Option<f32>,
    pub internet: Option<f32>,
    pub answered: VecDeque<bool>,
    pub lookup: Option<f32>,
    pub lookups: u64,
    /// Where to send them: the gateway and the DNS server, as the worker last saw them.
    pub gateway_address: Option<Ipv4Addr>,
    pub dns_address: Option<Ipv4Addr>,
}

/// How many islands have the panel open, and a way to wake whatever sleeps until one does.
#[derive(Default)]
pub(super) struct Watched {
    watchers: AtomicUsize,
    lock: Mutex<()>,
    opened: Condvar,
}

impl Watched {
    pub fn watching(&self) -> bool {
        self.watchers.load(Ordering::SeqCst) > 0
    }

    /// Sleeps until the panel is open, without waking until it is.
    fn wait(&self) {
        let guard = self.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(
            self.opened
                .wait_while(guard, |_| !self.watching())
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    }
}

#[derive(Clone)]
pub struct Network {
    requests: UnboundedSender<Request>,
    watched: Arc<Watched>,
}

pub(super) struct Watch(Arc<Watched>);

impl Drop for Watch {
    fn drop(&mut self) {
        self.0.watchers.fetch_sub(1, Ordering::SeqCst);
    }
}

fn spawn(name: &str, run: impl FnOnce() + Send + 'static) {
    if let Err(error) = thread::Builder::new().name(name.into()).spawn(run) {
        warn!("cannot start {name}: {error}");
    }
}

/// Asks for a sample each second while the panel is open, and sleeps while it is not.
fn sampler(watched: &Watched, requests: &UnboundedSender<Request>) {
    loop {
        watched.wait();
        while watched.watching() {
            if requests.unbounded_send(Request::Sample).is_err() {
                return;
            }
            thread::sleep(SAMPLE_EVERY);
        }
    }
}

/// Wakes the worker as each day begins.
fn midnight(requests: &UnboundedSender<Request>) {
    loop {
        let now = Local::now();
        let next = now
            .date_naive()
            .checked_add_days(Days::new(1))
            .and_then(|day| day.and_hms_opt(0, 0, 1))
            .and_then(|at| at.and_local_timezone(Local).earliest());
        let wait = next.map_or(Duration::from_secs(3600), |next| {
            (next - now).to_std().unwrap_or(Duration::from_secs(60))
        });
        thread::sleep(wait);
        if requests.unbounded_send(Request::Midnight).is_err() {
            return;
        }
    }
}

fn probe_loop(watched: &Watched, probes: &Mutex<Probes>, settings: &Settings) {
    let target: Option<Ipv4Addr> = settings.latency_target.parse().ok();
    if target.is_none() {
        warn!(
            target = settings.latency_target,
            "latency_target is not an IPv4 address; pinging only the gateway"
        );
    }
    let mut sequence: u16 = 0;
    loop {
        watched.wait();
        while watched.watching() {
            let (gateway_address, dns_address, round) = {
                let probes = probes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                (probes.gateway_address, probes.dns_address, probes.round)
            };
            sequence = sequence.wrapping_add(1);
            let gateway =
                gateway_address.and_then(|address| probe::ping(address, sequence, PROBE_TIMEOUT));
            sequence = sequence.wrapping_add(1);
            let internet = target.and_then(|address| probe::ping(address, sequence, PROBE_TIMEOUT));
            let lookup = (round % LOOKUP_EVERY == 0).then(|| {
                dns_address
                    .and_then(|server| probe::lookup(server, &settings.dns_probe, PROBE_TIMEOUT))
            });
            {
                let mut probes = probes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                probes.round += 1;
                probes.gateway = gateway;
                probes.internet = internet;
                if target.is_some() {
                    probes.answered.push_back(internet.is_some());
                    while probes.answered.len() > LOSS_WINDOW {
                        probes.answered.pop_front();
                    }
                }
                if let Some(lookup) = lookup {
                    probes.lookup = lookup;
                    probes.lookups += 1;
                }
            }
            thread::sleep(PROBE_EVERY);
        }
    }
}

impl Network {
    pub(super) fn start(feed: Feed<State>) -> Self {
        let settings: Settings = feed.settings();
        let (requests, received) = mpsc::unbounded();
        let received: Arc<Mutex<UnboundedReceiver<Request>>> = Arc::new(Mutex::new(received));
        let watched = Arc::new(Watched::default());
        let probes = Arc::new(Mutex::new(Probes::default()));
        {
            let (watched, probes, settings) = (watched.clone(), probes.clone(), settings.clone());
            spawn("island-network-probe", move || probe_loop(&watched, &probes, &settings));
        }
        {
            let (watched, requests) = (watched.clone(), requests.clone());
            spawn("island-network-sample", move || sampler(&watched, &requests));
        }
        {
            let requests = requests.clone();
            spawn("island-network-midnight", move || midnight(&requests));
        }
        let shared = watched.clone();
        let sender = requests.clone();
        feed.supervise(move |feed| {
            worker::run(feed, &received, &sender, &shared, &probes, &settings)
        });
        Self { requests, watched }
    }

    pub(super) fn send(&self, request: Request) {
        let _ = self.requests.unbounded_send(request);
    }

    /// The panel is in view: the worker measures what it shows until it is not.
    pub(super) fn watch(&self) -> Watch {
        if self.watched.watchers.fetch_add(1, Ordering::SeqCst) == 0 {
            let _guard = self
                .watched
                .lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            self.watched.opened.notify_all();
            self.send(Request::Watch);
        }
        Watch(self.watched.clone())
    }
}
