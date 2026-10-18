//! Level meters, as pavucontrol draws them: peak-detecting record streams, for which the server
//! sends only the highest sample of each slice. They run only while the panel shows them.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use libpulse_binding::{
    context::Context,
    def::BufferAttr,
    sample::{Format, Spec},
    stream::{FlagSet as StreamFlags, PeekResult, Stream},
};
use tracing::debug;

use super::state::{AudioState, Target};

/// What a level meter listens to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Probe {
    Output,
    Input,
    App(u32),
}

/// Both 0 to 1 on a decibel scale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Peak {
    pub level: f32,
    /// The recent highest level, falling slowly.
    pub hold: f32,
}

/// Shared between the worker, which measures, and the panel, which reads.
#[derive(Default)]
pub struct Levels {
    peaks: HashMap<Probe, Peak>,
    /// When the panel last read them; meters run only while it keeps doing so.
    seen: Option<Instant>,
}

impl Levels {
    /// Keeps the meters running for a while longer.
    pub fn touch(&mut self) {
        self.seen = Some(Instant::now());
    }

    pub fn read(&mut self) -> HashMap<Probe, Peak> {
        self.touch();
        self.peaks.clone()
    }

    fn watched(&self) -> bool {
        self.seen.is_some_and(|seen| seen.elapsed() < WATCH_FOR)
    }
}

/// Peaks arrive this many times a second.
const RATE: u32 = 30;
/// The quietest a meter shows, below full scale.
const FLOOR_DB: f32 = 60.0;
/// How far a level falls per peak once the sound drops.
const FALL: f32 = 0.05;
const HOLD_FALL: f32 = 0.01;
/// The panel reads far more often than this while it is open.
const WATCH_FOR: Duration = Duration::from_secs(1);

pub fn loudness(peak: f32) -> f32 {
    if peak <= 0.0 || !peak.is_finite() {
        return 0.0;
    }
    ((20.0 * peak.log10() + FLOOR_DB) / FLOOR_DB).clamp(0.0, 1.0)
}

/// A source to listen to and, for an app, its stream on the sink it is played to.
type Want = (Probe, String, Option<u32>);

fn wanted(state: &AudioState) -> Vec<Want> {
    let mut wanted = Vec::new();
    if let Some(sink) = state.default_device(Target::Output) {
        wanted.push((Probe::Output, sink.monitor.clone(), None));
    }
    if let Some(source) = state.default_device(Target::Input) {
        wanted.push((Probe::Input, source.name.clone(), None));
    }
    for app in &state.playing {
        if let Some(sink) = state.sink(app.device) {
            wanted.push((Probe::App(app.index), sink.monitor.clone(), Some(app.index)));
        }
    }
    wanted
}

fn open(context: &mut Context, source: &str, app: Option<u32>) -> Option<Stream> {
    let spec = Spec {
        format: Format::F32le,
        channels: 1,
        rate: RATE,
    };
    let mut stream = Stream::new(context, "peak", &spec, None)?;
    if let Some(index) = app {
        stream.set_monitor_stream(index).ok()?;
    }
    let attr = BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: size_of::<f32>() as u32,
    };
    let flags = StreamFlags::PEAK_DETECT | StreamFlags::ADJUST_LATENCY | StreamFlags::DONT_MOVE;
    stream.connect_record(Some(source), Some(&attr), flags).ok()?;
    Some(stream)
}

/// The loudest peak that arrived since last time, if any did.
fn loudest(stream: &mut Stream) -> Option<f32> {
    let mut loudest: Option<f32> = None;
    loop {
        match stream.peek() {
            Ok(PeekResult::Data(bytes)) => {
                for sample in bytes.chunks_exact(4) {
                    let peak = f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]).abs();
                    loudest = Some(loudest.map_or(peak, |loudest| loudest.max(peak)));
                }
                let _ = stream.discard();
            }
            Ok(PeekResult::Hole(_)) => {
                let _ = stream.discard();
            }
            Ok(PeekResult::Empty) | Err(_) => break,
        }
    }
    loudest
}

#[derive(Default)]
pub struct Meters {
    open: Vec<(Want, Stream)>,
}

impl Meters {
    /// Opens what the panel shows and closes the rest, or closes all once it stops reading.
    pub fn update(&mut self, context: &mut Context, state: &AudioState, levels: &Mutex<Levels>) {
        if levels.lock().is_ok_and(|levels| levels.watched()) {
            self.sync(context, wanted(state));
        } else if !self.open.is_empty() {
            debug!("level meters stopped");
            self.open.clear();
        }
        if let Ok(mut levels) = levels.lock() {
            self.measure(&mut levels);
        }
    }

    fn sync(&mut self, context: &mut Context, wanted: Vec<Want>) {
        self.open.retain(|(want, _)| wanted.contains(want));
        for want in wanted {
            if self.open.iter().any(|(open, _)| *open == want) {
                continue;
            }
            match open(context, &want.1, want.2) {
                Some(stream) => self.open.push((want, stream)),
                None => debug!(probe = ?want.0, source = want.1, "cannot start a level meter"),
            }
        }
    }

    fn measure(&mut self, levels: &mut Levels) {
        levels.peaks.retain(|probe, _| self.open.iter().any(|((open, _, _), _)| open == probe));
        for ((probe, _, _), stream) in &mut self.open {
            let Some(peak) = loudest(stream) else { continue };
            let entry = levels.peaks.entry(*probe).or_default();
            entry.level = loudness(peak).max(entry.level - FALL);
            entry.hold = entry.level.max(entry.hold - HOLD_FALL);
        }
    }
}

#[cfg(test)]
#[path = "meters_test.rs"]
mod tests;
