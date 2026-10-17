//! A spectrum of whatever is playing, captured from the default output's monitor through the
//! PulseAudio protocol (served by pipewire-pulse) and analysed in-process: no external tools.

use std::{
    cell::Cell,
    f32::consts::PI,
    rc::Rc,
    time::Instant,
};

use libpulse_binding::{
    context::{Context, FlagSet as ContextFlags, State, subscribe::InterestMaskSet},
    def::BufferAttr,
    mainloop::standard::{IterateResult, Mainloop},
    sample::{Format, Spec},
    stream::{FlagSet as StreamFlags, PeekResult, State as StreamState, Stream},
};

use tracing::{debug, error};

use crate::{
    features::Feed,
    supervisor::{self, Reconnect},
};

pub const BARS: usize = 10;
const RATE: u32 = 48_000;
/// Samples per analysis window; 2048 at 48 kHz resolves ~23 Hz, enough to separate the low bands.
const WINDOW: usize = 2048;
/// New samples between analyses: 30 spectra per second.
const HOP: usize = RATE as usize / 30;
const LOW_HZ: f32 = 50.0;
const HIGH_HZ: f32 = 12_000.0;
/// Spectra per second, which cava's smoothing constants are corrected for.
const FRAMERATE: f64 = RATE as f64 / HOP as f64;
/// cava's default smoothing strength (0.77): higher is smoother and slower.
const NOISE_REDUCTION: f64 = 0.77;

/// Silence is sent once rather than every frame.
pub fn start<S: Send + 'static>(feed: Feed<S>) {
    let mut pace = Reconnect::default();
    let spawned = supervisor::supervise("visualizer", move || loop {
        let started = Instant::now();
        match capture(&feed) {
            Ended::Unwanted => return supervisor::Ended::Unwanted,
            Ended::Moved => debug!("default output changed, capturing it instead"),
            Ended::Stopped => {
                debug!("capture stopped, reconnecting");
                if !feed.publish_live(vec![0.0; BARS]) {
                    return supervisor::Ended::Unwanted;
                }
                pace.wait(started);
            }
        }
    });
    if let Err(error) = spawned {
        error!("visualizer failed to start: {error}");
    }
}

enum Ended {
    Unwanted,
    /// The default output changed; capture again right away.
    Moved,
    /// The connection failed or dropped; retry after a delay.
    Stopped,
}

fn iterate(mainloop: &mut Mainloop) -> bool {
    matches!(mainloop.iterate(true), IterateResult::Success(_))
}

fn capture<S: Send + 'static>(feed: &Feed<S>) -> Ended {
    let Some(mut mainloop) = Mainloop::new() else {
        return Ended::Stopped;
    };
    let Some(mut context) = Context::new(&mainloop, "island visualizer") else {
        return Ended::Stopped;
    };
    if context.connect(None, ContextFlags::NOFAIL, None).is_err() {
        return Ended::Stopped;
    }
    loop {
        if !iterate(&mut mainloop) {
            return Ended::Stopped;
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => return Ended::Stopped,
            _ => {}
        }
    }

    let moved = Rc::new(Cell::new(false));
    let flag = moved.clone();
    context.set_subscribe_callback(Some(Box::new(move |_, _, _| flag.set(true))));
    context.subscribe(InterestMaskSet::SERVER, |_| {});

    let spec = Spec {
        format: Format::F32le,
        channels: 1,
        rate: RATE,
    };
    let Some(mut stream) = Stream::new(&mut context, "spectrum", &spec, None) else {
        return Ended::Stopped;
    };
    let attr = BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: (HOP * size_of::<f32>()) as u32,
    };
    if stream.connect_record(Some("@DEFAULT_MONITOR@"), Some(&attr), StreamFlags::ADJUST_LATENCY).is_err() {
        return Ended::Stopped;
    }
    loop {
        if !iterate(&mut mainloop) {
            return Ended::Stopped;
        }
        match stream.get_state() {
            StreamState::Ready => break,
            StreamState::Failed | StreamState::Terminated => return Ended::Stopped,
            _ => {}
        }
    }
    debug!("capturing the default output");

    let mut analyser = Analyser::new();
    loop {
        if !iterate(&mut mainloop) || !matches!(context.get_state(), State::Ready) {
            return Ended::Stopped;
        }
        if moved.replace(false) {
            let _ = stream.disconnect();
            return Ended::Moved;
        }
        loop {
            match stream.peek() {
                Ok(PeekResult::Data(bytes)) => {
                    for sample in bytes.chunks_exact(4) {
                        if let Some(levels) = analyser.push(f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
                            && !feed.publish_live(levels)
                        {
                            let _ = stream.disconnect();
                            return Ended::Unwanted;
                        }
                    }
                    let _ = stream.discard();
                }
                Ok(PeekResult::Hole(_)) => {
                    let _ = stream.discard();
                }
                Ok(PeekResult::Empty) => break,
                Err(_) => return Ended::Stopped,
            }
        }
    }
}

struct Analyser {
    ring: Vec<f32>,
    at: usize,
    since: usize,
    heard: bool,
    hann: Vec<f32>,
    bands: Vec<(usize, usize)>,
    eq: Vec<f64>,
    sens: f64,
    sens_init: bool,
    previous: [f64; BARS],
    peak: [f64; BARS],
    fall: [f64; BARS],
    memory: [f64; BARS],
    silent: bool,
    fft: Fft,
}

impl Analyser {
    fn new() -> Self {
        let hann = (0..WINDOW)
            .map(|index| 0.5 - 0.5 * (2.0 * PI * index as f32 / (WINDOW - 1) as f32).cos())
            .collect();
        // Log-spaced band edges, each at least one FFT bin wide.
        let bin_hz = RATE as f32 / WINDOW as f32;
        let edge = |index: usize| LOW_HZ * (HIGH_HZ / LOW_HZ).powf(index as f32 / BARS as f32);
        let mut bands = Vec::with_capacity(BARS);
        let mut floor = (LOW_HZ / bin_hz).floor() as usize;
        for index in 0..BARS {
            let low = floor.max((edge(index) / bin_hz).round() as usize);
            let high = ((edge(index + 1) / bin_hz).round() as usize).max(low + 1);
            bands.push((low, high));
            floor = high;
        }
        // cava's EQ: normalise the huge FFT sums, boost treble by its upper edge^0.85, and average
        // over the band's width. cava feeds 16-bit sample values (±32768) where these are ±1.0
        // floats, so the same constants apply once scaled back up.
        const SIXTEEN_BIT: f64 = 32_768.0;
        let eq = bands
            .iter()
            .map(|&(low, high)| {
                let upper_hz = f64::from(high as f32 * bin_hz);
                SIXTEEN_BIT * upper_hz.powf(0.85) / 2f64.powi(28) / (WINDOW as f64).log2() / (high - low) as f64
            })
            .collect();
        Self {
            ring: vec![0.0; WINDOW],
            at: 0,
            since: 0,
            heard: false,
            hann,
            bands,
            eq,
            sens: 1.0,
            sens_init: true,
            previous: [0.0; BARS],
            peak: [0.0; BARS],
            fall: [0.0; BARS],
            memory: [0.0; BARS],
            silent: false,
            fft: Fft::new(WINDOW),
        }
    }

    /// Adds one sample; every `HOP` samples returns new levels, unless still silent.
    fn push(&mut self, sample: f32) -> Option<Vec<f32>> {
        let sample = if sample.is_finite() { sample } else { 0.0 };
        self.heard |= sample != 0.0;
        self.ring[self.at] = sample;
        self.at = (self.at + 1) % WINDOW;
        self.since += 1;
        if self.since < HOP {
            return None;
        }
        self.since = 0;
        let silent_input = !std::mem::take(&mut self.heard);
        let levels = self.analyse(silent_input);
        let quiet = levels.iter().all(|level| *level <= 0.0);
        if quiet && self.silent {
            return None;
        }
        self.silent = quiet;
        Some(levels)
    }

    /// cava's pipeline (cavacore.c): summed band magnitudes with a treble-boosting EQ, auto
    /// sensitivity, gravity falloff and integral smoothing, with its frame-rate corrections.
    fn analyse(&mut self, silent_input: bool) -> Vec<f32> {
        let (re, im) = self.fft.buffers();
        for (index, (re, im)) in re.iter_mut().zip(im.iter_mut()).enumerate() {
            *re = self.ring[(self.at + index) % WINDOW] * self.hann[index];
            *im = 0.0;
        }
        self.fft.run();
        let (re, im) = self.fft.buffers();
        let mut out: Vec<f64> = self
            .bands
            .iter()
            .zip(&self.eq)
            .map(|(&(low, high), eq)| {
                let high = high.min(WINDOW / 2);
                let sum: f64 = (low..high).map(|bin| f64::from(re[bin]).hypot(f64::from(im[bin]))).sum();
                sum * eq * self.sens
            })
            .collect();

        let framerate_mod = 66.0 / FRAMERATE;
        let gravity_mod = framerate_mod.powf(2.5) * 2.0 / NOISE_REDUCTION;
        let integral_mod = framerate_mod.powf(0.1);
        let mut overshoot = false;
        for (index, value) in out.iter_mut().enumerate() {
            // Gravity: a falling bar keeps its last peak and drops with growing speed.
            if *value < self.previous[index] {
                *value = (self.peak[index] * (1.0 - self.fall[index] * self.fall[index] * gravity_mod)).max(0.0);
                self.fall[index] += 0.028;
            } else {
                self.peak[index] = *value;
                self.fall[index] = 0.0;
            }
            self.previous[index] = *value;
            // Integral: blend with the previous output so bars glide instead of jitter.
            *value += self.memory[index] * NOISE_REDUCTION / integral_mod;
            self.memory[index] = *value;
            if *value > 1.0 {
                overshoot = true;
                *value = 1.0;
            }
        }
        // Auto sensitivity: back off on overshoot, creep up otherwise, quickly at first.
        if overshoot {
            self.sens *= 1.0 - 0.02 * framerate_mod;
            self.sens_init = false;
        } else if !silent_input {
            self.sens *= 1.0 + 0.001 * framerate_mod;
            if self.sens_init {
                self.sens *= 1.0 + 0.1 * framerate_mod;
            }
        }
        out.into_iter()
            .map(|value| if value < 0.005 { 0.0 } else { value as f32 })
            .collect()
    }
}

/// In-place radix-2 FFT of a fixed power-of-two size, with precomputed twiddles and bit reversal.
struct Fft {
    re: Vec<f32>,
    im: Vec<f32>,
    cos: Vec<f32>,
    sin: Vec<f32>,
    reverse: Vec<usize>,
}

impl Fft {
    fn new(size: usize) -> Self {
        assert!(size.is_power_of_two());
        let bits = size.trailing_zeros();
        Self {
            re: vec![0.0; size],
            im: vec![0.0; size],
            cos: (0..size / 2).map(|k| (-2.0 * PI * k as f32 / size as f32).cos()).collect(),
            sin: (0..size / 2).map(|k| (-2.0 * PI * k as f32 / size as f32).sin()).collect(),
            reverse: (0..size).map(|index| index.reverse_bits() >> (usize::BITS - bits)).collect(),
        }
    }

    fn buffers(&mut self) -> (&mut [f32], &mut [f32]) {
        (&mut self.re, &mut self.im)
    }

    fn run(&mut self) {
        let size = self.re.len();
        for index in 0..size {
            let swap = self.reverse[index];
            if swap > index {
                self.re.swap(index, swap);
                self.im.swap(index, swap);
            }
        }
        let mut span = 2;
        while span <= size {
            let half = span / 2;
            let stride = size / span;
            for start in (0..size).step_by(span) {
                for k in 0..half {
                    let (c, s) = (self.cos[k * stride], self.sin[k * stride]);
                    let (a, b) = (start + k, start + k + half);
                    let (br, bi) = (self.re[b] * c - self.im[b] * s, self.re[b] * s + self.im[b] * c);
                    self.re[b] = self.re[a] - br;
                    self.im[b] = self.im[a] - bi;
                    self.re[a] += br;
                    self.im[a] += bi;
                }
            }
            span *= 2;
        }
    }
}

#[cfg(test)]
#[path = "spectrum_test.rs"]
mod tests;
