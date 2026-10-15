//! The PulseAudio-protocol (pipewire-pulse) side of the volume feature: device and level state, and
//! the requests that change them.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    time::Instant,
    io::{ErrorKind, Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    rc::Rc,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use libpulse_binding::{
    callbacks::ListResult,
    context::{Context, FlagSet, State, introspect::Introspector, subscribe::InterestMaskSet},
    mainloop::{
        api::Mainloop as _,
        events::io::FlagSet as IoFlags,
        standard::{IterateResult, Mainloop},
    },
    volume::{ChannelVolumes, Volume},
};

use tracing::{debug, error, info, trace, warn};

use crate::{
    features::Feed,
    supervisor::{Ended, Reconnect},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Output,
    Input,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Device {
    pub name: String,
    pub description: String,
    volume: f32,
    muted: bool,
    channels: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Channel {
    pub volume: f32,
    pub muted: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioState {
    /// Whether the sound server is reachable; while it is not, there are no devices.
    pub connected: bool,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
    pub default_sink: String,
    pub default_source: String,
}

impl AudioState {
    pub fn devices(&self, target: Target) -> &[Device] {
        match target {
            Target::Output => &self.sinks,
            Target::Input => &self.sources,
        }
    }

    pub fn default_device(&self, target: Target) -> Option<&Device> {
        let name = match target {
            Target::Output => &self.default_sink,
            Target::Input => &self.default_source,
        };
        self.devices(target).iter().find(|device| &device.name == name)
    }

    fn default_device_mut(&mut self, target: Target) -> Option<&mut Device> {
        let (devices, name) = match target {
            Target::Output => (&mut self.sinks, &self.default_sink),
            Target::Input => (&mut self.sources, &self.default_source),
        };
        devices.iter_mut().find(|device| &device.name == name)
    }

    pub fn channel(&self, target: Target) -> Channel {
        self.default_device(target).map_or_else(Channel::default, |device| Channel {
            volume: device.volume,
            muted: device.muted,
        })
    }
}

pub enum Request {
    SetVolume(Target, f32),
    Step(Target, f32),
    ToggleMute(Target),
    SetDefault(Target, String),
}

#[derive(Clone)]
pub struct Mixer {
    sender: Sender<Request>,
    wake: Option<Arc<UnixStream>>,
}

impl Mixer {
    pub fn send(&self, request: Request) {
        if self.sender.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            let _ = (&**wake).write(&[1]);
        }
    }
}

fn drain(stream: &UnixStream) {
    let mut buffer = [0u8; 64];
    loop {
        match (&*stream).read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

fn level(volume: &ChannelVolumes) -> f32 {
    volume.avg().0 as f32 / Volume::NORMAL.0 as f32
}

fn volumes(channels: u8, value: f32) -> ChannelVolumes {
    let mut volume = ChannelVolumes::default();
    let raw = (value.clamp(0.0, 1.0) * Volume::NORMAL.0 as f32).round() as u32;
    volume.set(channels.max(1), Volume(raw));
    volume
}

type Pending = (Rc<RefCell<Query>>, Rc<Cell<u8>>);

#[derive(Default)]
struct Query {
    sinks: Vec<Device>,
    sources: Vec<Device>,
    defaults: Option<(String, String)>,
}

fn stepped(current: f32, delta: f32) -> f32 {
    if delta == 0.0 {
        return current.clamp(0.0, 1.0);
    }
    const GRID: f32 = 0.05;
    let notches = delta / GRID;
    let size = if (notches - notches.round()).abs() < 1e-3 { GRID } else { delta.abs() };
    let position = current / size;
    let grid = if delta > 0.0 { (position + 1e-3).floor() } else { (position - 1e-3).ceil() };
    let moved = grid + (delta / size).round();
    ((moved * size).clamp(0.0, 1.0) * 1000.0).round() / 1000.0
}

const INTENT_HOLD: Duration = Duration::from_millis(600);

#[derive(Default)]
struct Intent {
    volume: Option<f32>,
    muted: Option<bool>,
    at: Option<Instant>,
}

type Intents = HashMap<String, Intent>;

fn remember<'a>(intents: &'a mut Intents, device: &Device) -> &'a mut Intent {
    let intent = intents.entry(device.name.clone()).or_default();
    intent.at = Some(Instant::now());
    intent
}

fn honour(intents: &mut Intents, state: &mut AudioState) {
    intents.retain(|_, intent| intent.at.is_some_and(|at| at.elapsed() < INTENT_HOLD));
    for device in state.sinks.iter_mut().chain(state.sources.iter_mut()) {
        if let Some(intent) = intents.get(&device.name) {
            if let Some(volume) = intent.volume {
                device.volume = volume;
            }
            if let Some(muted) = intent.muted {
                device.muted = muted;
            }
        }
    }
}

fn set_volume(
    introspect: &mut Introspector,
    state: &mut AudioState,
    intents: &mut Intents,
    target: Target,
    value: f32,
) {
    let Some(device) = state.default_device_mut(target) else { return };
    device.volume = value.clamp(0.0, 1.0);
    remember(intents, device).volume = Some(device.volume);
    let volume = volumes(device.channels, device.volume);
    match target {
        Target::Output => drop(introspect.set_sink_volume_by_name(&device.name, &volume, None)),
        Target::Input => drop(introspect.set_source_volume_by_name(&device.name, &volume, None)),
    }
}

fn apply(context: &mut Context, state: &mut AudioState, intents: &mut Intents, request: Request) {
    let mut introspect = context.introspect();
    match request {
        Request::SetVolume(target, value) => set_volume(&mut introspect, state, intents, target, value),
        Request::Step(target, delta) => {
            let Some(current) = state.default_device(target).map(|device| device.volume) else { return };
            set_volume(&mut introspect, state, intents, target, stepped(current, delta));
        }
        Request::ToggleMute(target) => {
            let Some(device) = state.default_device_mut(target) else { return };
            device.muted = !device.muted;
            remember(intents, device).muted = Some(device.muted);
            match target {
                Target::Output => drop(introspect.set_sink_mute_by_name(&device.name, device.muted, None)),
                Target::Input => drop(introspect.set_source_mute_by_name(&device.name, device.muted, None)),
            }
        }
        Request::SetDefault(target, name) => match target {
            Target::Output => drop(context.set_default_sink(&name, |_| {})),
            Target::Input => drop(context.set_default_source(&name, |_| {})),
        },
    }
}

fn run(requests: &Receiver<Request>, wake: &UnixStream, feed: &Feed<AudioState>) -> Option<()> {
    let mut mainloop = Mainloop::new()?;
    let waker = wake.try_clone().ok()?;
    let _wake_event = mainloop.new_io_event(
        wake.as_raw_fd(),
        IoFlags::INPUT,
        Box::new(move |_, _, _| drain(&waker)),
    )?;
    let mut context = Context::new(&mainloop, "island")?;
    context.connect(None, FlagSet::NOFAIL, None).ok()?;
    loop {
        match mainloop.iterate(true) {
            IterateResult::Success(_) => {}
            IterateResult::Quit(_) | IterateResult::Err(_) => return None,
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => {
                debug!(error = ?context.errno(), "cannot connect to the sound server");
                return None;
            }
            _ => {}
        }
    }
    info!(server = context.get_server().as_deref().unwrap_or("unknown"), "connected to the sound server");

    let dirty = Rc::new(Cell::new(true));
    let subscribed = dirty.clone();
    context.set_subscribe_callback(Some(Box::new(move |_, _, _| subscribed.set(true))));
    let _ = context.subscribe(InterestMaskSet::SINK | InterestMaskSet::SOURCE | InterestMaskSet::SERVER, |_| {});

    let mut state = AudioState::default();
    let mut intents = Intents::new();
    let mut pending: Option<Pending> = None;
    loop {
        if !matches!(context.get_state(), State::Ready) {
            warn!(error = ?context.errno(), "lost the sound server, reconnecting");
            return None;
        }
        let mut changed = false;
        while let Ok(request) = requests.try_recv() {
            apply(&mut context, &mut state, &mut intents, request);
            changed = true;
        }
        if changed && !feed.publish(state.clone()) {
            return Some(());
        }
        if pending.is_none() && dirty.replace(false) {
            let query = Rc::new(RefCell::new(Query::default()));
            let remaining = Rc::new(Cell::new(3u8));
            let introspect = context.introspect();
            {
                let (query, remaining) = (query.clone(), remaining.clone());
                introspect.get_sink_info_list(move |result| match result {
                    ListResult::Item(info) => query.borrow_mut().sinks.push(Device {
                        name: info.name.as_deref().unwrap_or_default().to_owned(),
                        description: info.description.as_deref().unwrap_or_default().to_owned(),
                        volume: level(&info.volume),
                        muted: info.mute,
                        channels: info.volume.len(),
                    }),
                    ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
                });
            }
            {
                let (query, remaining) = (query.clone(), remaining.clone());
                introspect.get_source_info_list(move |result| match result {
                    ListResult::Item(info) if info.monitor_of_sink.is_none() => query.borrow_mut().sources.push(Device {
                        name: info.name.as_deref().unwrap_or_default().to_owned(),
                        description: info.description.as_deref().unwrap_or_default().to_owned(),
                        volume: level(&info.volume),
                        muted: info.mute,
                        channels: info.volume.len(),
                    }),
                    ListResult::Item(_) => {}
                    ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
                });
            }
            {
                let (query, remaining) = (query.clone(), remaining.clone());
                introspect.get_server_info(move |info| {
                    query.borrow_mut().defaults = Some((
                        info.default_sink_name.as_deref().unwrap_or_default().to_owned(),
                        info.default_source_name.as_deref().unwrap_or_default().to_owned(),
                    ));
                    remaining.set(remaining.get() - 1);
                });
            }
            pending = Some((query, remaining));
        }
        if let Some((query, remaining)) = &pending
            && remaining.get() == 0
        {
            let query = std::mem::take(&mut *query.borrow_mut());
            let (default_sink, default_source) = query.defaults.unwrap_or_default();
            let mut next = AudioState {
                connected: true,
                sinks: query.sinks,
                sources: query.sources,
                default_sink,
                default_source,
            };
            honour(&mut intents, &mut next);
            pending = None;
            if next != state {
                if next.default_sink != state.default_sink {
                    info!(device = next.default_sink, "default output changed");
                }
                if next.default_source != state.default_source {
                    info!(device = next.default_source, "default input changed");
                }
                trace!(outputs = next.sinks.len(), inputs = next.sources.len(), "audio state changed");
                state = next;
                if !feed.publish(state.clone()) {
                    return Some(());
                }
            }
        }
        if let IterateResult::Quit(_) | IterateResult::Err(_) = mainloop.iterate(true) {
            return None;
        }
    }
}

/// Starts the supervised sound-server worker, publishing to `feed`, and returns its mixer.
pub fn start(feed: Feed<AudioState>) -> Mixer {
    let (sender, requests) = mpsc::channel();
    let wake = match UnixStream::pair() {
        Ok((wake, waker)) => {
            let _ = wake.set_nonblocking(true);
            let _ = waker.set_nonblocking(true);
            let mut pace = Reconnect::default();
            feed.supervise(move |feed| loop {
                let started = Instant::now();
                if run(&requests, &waker, feed).is_some() || !feed.publish(AudioState::default()) {
                    return Ended::Unwanted;
                }
                pace.wait(started);
            });
            Some(Arc::new(wake))
        }
        Err(error) => {
            error!("audio failed to start: {error}");
            None
        }
    };
    Mixer { sender, wake }
}

#[cfg(test)]
#[path = "pulse_test.rs"]
mod tests;

