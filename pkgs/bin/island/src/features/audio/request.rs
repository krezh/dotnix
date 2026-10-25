//! What the bar, the panel and commands ask of the sound server, and how each is carried out.

use std::{
    collections::HashMap,
    io::Write,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex, mpsc::Sender},
    time::{Duration, Instant},
};

use libpulse_binding::{
    context::{Context, introspect::Introspector},
    volume::{ChannelVolumes, Volume},
};

use super::{
    meters::{Levels, Peak, Probe},
    state::{AudioState, Target},
};

pub enum Request {
    SetVolume(Target, f32),
    Step(Target, f32),
    ToggleMute(Target),
    SetDefault(Target, String),
    SetAppVolume(u32, f32),
    ToggleAppMute(u32),
    MoveApp(u32, String),
    /// Only wakes the worker, so it starts measuring.
    Watch,
}

/// The way in to the worker.
#[derive(Clone)]
pub struct Mixer {
    pub(super) sender: Sender<Request>,
    pub(super) wake: Option<Arc<UnixStream>>,
    pub(super) levels: Arc<Mutex<Levels>>,
}

impl Mixer {
    pub fn send(&self, request: Request) {
        if self.sender.send(request).is_ok()
            && let Some(wake) = &self.wake
        {
            let _ = (&**wake).write(&[1]);
        }
    }

    /// Starts the meters, which stop once every returned [`Watch`] is dropped.
    pub fn watch(&self) -> Watch {
        if self.levels.lock().is_ok_and(|mut levels| levels.watch()) {
            self.send(Request::Watch);
        }
        Watch(self.clone())
    }

    pub fn peaks(&self) -> HashMap<Probe, Peak> {
        self.levels
            .lock()
            .map(|levels| levels.read())
            .unwrap_or_default()
    }
}

pub struct Watch(Mixer);

impl Drop for Watch {
    fn drop(&mut self) {
        if self
            .0
            .levels
            .lock()
            .is_ok_and(|mut levels| levels.unwatch())
        {
            self.0.send(Request::Watch);
        }
    }
}

pub fn volumes(channels: u8, value: f32) -> ChannelVolumes {
    let mut volume = ChannelVolumes::default();
    let raw = (value.clamp(0.0, 1.0) * Volume::NORMAL.0 as f32).round() as u32;
    volume.set(channels.max(1), Volume(raw));
    volume
}

pub fn stepped(current: f32, delta: f32) -> f32 {
    if delta == 0.0 {
        return current.clamp(0.0, 1.0);
    }
    const GRID: f32 = 0.05;
    let notches = delta / GRID;
    let size = if (notches - notches.round()).abs() < 1e-3 {
        GRID
    } else {
        delta.abs()
    };
    let position = current / size;
    let grid = if delta > 0.0 {
        (position + 1e-3).floor()
    } else {
        (position - 1e-3).ceil()
    };
    let moved = grid + (delta / size).round();
    ((moved * size).clamp(0.0, 1.0) * 1000.0).round() / 1000.0
}

const INTENT_HOLD: Duration = Duration::from_millis(600);

/// A change just asked for, which wins over the server's older word on it for a moment.
#[derive(Default)]
struct Intent {
    volume: Option<f32>,
    muted: Option<bool>,
    at: Option<Instant>,
}

/// Devices by name, apps by `#index`.
#[derive(Default)]
pub struct Intents(HashMap<String, Intent>);

fn app_key(index: u32) -> String {
    format!("#{index}")
}

impl Intents {
    fn remember(&mut self, key: String) -> &mut Intent {
        let intent = self.0.entry(key).or_default();
        intent.at = Some(Instant::now());
        intent
    }

    /// Puts what was just asked for over a fresh state from the server.
    pub fn honour(&mut self, state: &mut AudioState) {
        self.0
            .retain(|_, intent| intent.at.is_some_and(|at| at.elapsed() < INTENT_HOLD));
        let devices = state
            .sinks
            .iter_mut()
            .chain(state.sources.iter_mut())
            .map(|device| (device.name.clone(), &mut device.volume, &mut device.muted));
        let apps = state
            .playing
            .iter_mut()
            .map(|app| (app_key(app.index), &mut app.volume, &mut app.muted));
        for (key, volume, muted) in devices.chain(apps) {
            if let Some(intent) = self.0.get(&key) {
                if let Some(wanted) = intent.volume {
                    *volume = wanted;
                }
                if let Some(wanted) = intent.muted {
                    *muted = wanted;
                }
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
    let Some(device) = state.default_device_mut(target) else {
        return;
    };
    device.volume = value.clamp(0.0, 1.0);
    intents.remember(device.name.clone()).volume = Some(device.volume);
    let volume = volumes(device.channels, device.volume);
    match target {
        Target::Output => drop(introspect.set_sink_volume_by_name(&device.name, &volume, None)),
        Target::Input => drop(introspect.set_source_volume_by_name(&device.name, &volume, None)),
    }
}

/// Sends the request on and changes `state` to match at once, ahead of the server's answer.
pub fn apply(
    context: &mut Context,
    state: &mut AudioState,
    intents: &mut Intents,
    request: Request,
) {
    let mut introspect = context.introspect();
    match request {
        Request::SetVolume(target, value) => {
            set_volume(&mut introspect, state, intents, target, value)
        }
        Request::Step(target, delta) => {
            let Some(current) = state.default_device(target).map(|device| device.volume) else {
                return;
            };
            set_volume(
                &mut introspect,
                state,
                intents,
                target,
                stepped(current, delta),
            );
        }
        Request::ToggleMute(target) => {
            let Some(device) = state.default_device_mut(target) else {
                return;
            };
            device.muted = !device.muted;
            intents.remember(device.name.clone()).muted = Some(device.muted);
            match target {
                Target::Output => {
                    drop(introspect.set_sink_mute_by_name(&device.name, device.muted, None))
                }
                Target::Input => {
                    drop(introspect.set_source_mute_by_name(&device.name, device.muted, None))
                }
            }
        }
        Request::SetDefault(target, name) => match target {
            Target::Output => drop(context.set_default_sink(&name, |_| {})),
            Target::Input => drop(context.set_default_source(&name, |_| {})),
        },
        Request::SetAppVolume(index, value) => {
            let Some(app) = state.playing.iter_mut().find(|app| app.index == index) else {
                return;
            };
            app.volume = value.clamp(0.0, 1.0);
            intents.remember(app_key(index)).volume = Some(app.volume);
            drop(introspect.set_sink_input_volume(index, &volumes(app.channels, app.volume), None));
        }
        Request::ToggleAppMute(index) => {
            let Some(app) = state.playing.iter_mut().find(|app| app.index == index) else {
                return;
            };
            app.muted = !app.muted;
            intents.remember(app_key(index)).muted = Some(app.muted);
            drop(introspect.set_sink_input_mute(index, app.muted, None));
        }
        Request::MoveApp(index, sink) => {
            let Some(to) = state
                .sinks
                .iter()
                .find(|device| device.name == sink)
                .map(|device| device.index)
            else {
                return;
            };
            if let Some(app) = state.playing.iter_mut().find(|app| app.index == index) {
                app.device = to;
            }
            drop(introspect.move_sink_input_by_name(index, &sink, None));
        }
        Request::Watch => {}
    }
}

#[cfg(test)]
#[path = "request_test.rs"]
mod tests;
