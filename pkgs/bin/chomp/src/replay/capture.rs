use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use wayland_client::Connection;

use super::hardware::HardwareEncoder;
use super::ipc::{ReplayStatus, ServiceState};
use crate::cli::Settings;
use crate::compositor::backend::hyprland::TaggedWindow;
use crate::compositor::{self, protocol::hyprland_toplevel::HyprlandToplevelCapture};

pub struct SavedReplay {
    pub path: PathBuf,
    pub duration: Duration,
}

struct BufferedPacket {
    packet: ffmpeg::Packet,
    timestamp_micros: i64,
}

struct PacketRing {
    packets: VecDeque<BufferedPacket>,
    duration_micros: i64,
    bytes: u64,
    max_bytes: u64,
    waiting_for_keyframe: bool,
}

impl PacketRing {
    fn new(duration: Duration) -> Self {
        Self {
            packets: VecDeque::new(),
            duration_micros: duration.as_micros() as i64,
            bytes: 0,
            max_bytes: u64::MAX,
            waiting_for_keyframe: false,
        }
    }

    fn push(&mut self, packet: ffmpeg::Packet, timestamp_micros: i64) {
        if self.waiting_for_keyframe {
            if !packet.is_key() {
                return;
            }
            self.waiting_for_keyframe = false;
        }
        self.bytes += packet.size() as u64;
        self.packets.push_back(BufferedPacket {
            packet,
            timestamp_micros,
        });
        let cutoff = timestamp_micros - self.duration_micros;
        let retain = self
            .packets
            .iter()
            .enumerate()
            .filter(|(_, packet)| packet.packet.is_key() && packet.timestamp_micros <= cutoff)
            .map(|(index, _)| index)
            .next_back();
        if let Some(retain) = retain {
            self.drain_before(retain);
        }
        while self.bytes > self.max_bytes {
            let Some(retain) = self
                .packets
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, packet)| packet.packet.is_key())
                .map(|(index, _)| index)
            else {
                self.clear();
                self.waiting_for_keyframe = true;
                break;
            };
            self.drain_before(retain);
        }
    }
    fn set_duration(&mut self, duration: Duration) {
        self.duration_micros = duration.as_micros() as i64;
        if let Some(last) = self.packets.back() {
            let cutoff = last.timestamp_micros - self.duration_micros;
            let retain = self
                .packets
                .iter()
                .enumerate()
                .filter(|(_, packet)| packet.packet.is_key() && packet.timestamp_micros <= cutoff)
                .map(|(index, _)| index)
                .next_back();
            if let Some(retain) = retain {
                self.drain_before(retain);
            }
        }
    }

    fn packets_for_duration(&self, duration: Option<Duration>) -> (Vec<ffmpeg::Packet>, Duration) {
        if self.packets.is_empty() {
            return (Vec::new(), Duration::ZERO);
        }
        let last = self.packets.back().unwrap();

        let start_index = match duration {
            Some(requested) => {
                let requested_micros = requested.as_micros() as i64;
                let cutoff = last.timestamp_micros.saturating_sub(requested_micros);
                self.packets
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.packet.is_key() && p.timestamp_micros <= cutoff)
                    .map(|(idx, _)| idx)
                    .next_back()
                    .unwrap_or(0)
            }
            None => 0,
        };

        let packets = self
            .packets
            .range(start_index..)
            .map(|p| p.packet.clone())
            .collect();
        let start_ts = self.packets[start_index].timestamp_micros;
        let actual_duration =
            Duration::from_micros(last.timestamp_micros.saturating_sub(start_ts) as u64);
        (packets, actual_duration)
    }

    fn set_max_bytes(&mut self, max_bytes: u64) {
        self.max_bytes = max_bytes;
    }
    fn drain_before(&mut self, retain: usize) {
        for _ in 0..retain {
            if let Some(packet) = self.packets.pop_front() {
                self.bytes -= packet.packet.size() as u64;
            }
        }
    }

    fn clear(&mut self) {
        self.packets.clear();
        self.bytes = 0;
    }

    fn buffered_duration(&self) -> Duration {
        let Some(first) = self.packets.front() else {
            return Duration::ZERO;
        };
        let Some(last) = self.packets.back() else {
            return Duration::ZERO;
        };
        Duration::from_micros(last.timestamp_micros.saturating_sub(first.timestamp_micros) as u64)
    }
}

struct SharedCapture {
    status: ReplayStatus,
    ring: PacketRing,
    parameters: Option<ffmpeg::codec::Parameters>,
    time_base: ffmpeg::Rational,
}

struct CaptureWorker {
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<SharedCapture>>,
    thread: Option<JoinHandle<()>>,
}

pub struct ReplayController {
    settings: Settings,
    worker: Option<CaptureWorker>,
}

impl ReplayController {
    pub fn new(settings: Settings) -> Result<Self> {
        Ok(Self {
            settings,
            worker: None,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        let worker_finished = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.thread.as_ref().is_some_and(JoinHandle::is_finished));
        if worker_finished {
            self.stop()?;
        }
        if self.worker.is_some() {
            return Ok(());
        }
        anyhow::ensure!(
            self.settings
                .replay
                .hyprland_tag
                .as_ref()
                .is_some_and(|tag| !tag.trim().is_empty()),
            "capture.replay.hyprland_tag is required to start instant replay"
        );

        let duration = Duration::from_secs(self.settings.replay.duration_seconds as u64);
        let shared = Arc::new(Mutex::new(SharedCapture {
            status: ReplayStatus {
                state: ServiceState::WaitingForTarget,
                target: None,
                target_app: None,
                target_title: None,
                duration_seconds: self.settings.replay.duration_seconds,
                buffered_millis: 0,
                buffered_bytes: 0,
                message: None,
            },
            ring: PacketRing::new(duration),
            parameters: None,
            time_base: ffmpeg::Rational(1, 1_000_000),
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_shared = shared.clone();
        let thread_stop = stop.clone();
        let settings = self.settings.clone();
        let handle = thread::Builder::new()
            .name("chomp-replay-capture".to_string())
            .spawn(move || {
                if let Err(error) = capture_loop(&settings, &thread_shared, &thread_stop) {
                    let mut shared = thread_shared
                        .lock()
                        .unwrap_or_else(|lock| lock.into_inner());
                    shared.status.state = ServiceState::Failed;
                    shared.status.message = Some(format!("{error:#}"));
                }
            })
            .context("Failed to start replay capture thread")?;
        self.worker = Some(CaptureWorker {
            stop,
            shared,
            thread: Some(handle),
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        let Some(mut worker) = self.worker.take() else {
            return Ok(());
        };
        worker.stop.store(true, Ordering::Release);
        if let Some(handle) = worker.thread.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Replay capture thread panicked"))?;
        }
        Ok(())
    }

    pub fn status(&self) -> ReplayStatus {
        let Some(worker) = &self.worker else {
            return ReplayStatus {
                state: ServiceState::Suspended,
                target: None,
                target_app: None,
                target_title: None,
                duration_seconds: self.settings.replay.duration_seconds,
                buffered_millis: 0,
                buffered_bytes: 0,
                message: None,
            };
        };
        worker
            .shared
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .status
            .clone()
    }

    pub fn configure(&mut self, duration_seconds: u32) -> Result<ReplayStatus> {
        anyhow::ensure!(
            (5..=600).contains(&duration_seconds),
            "capture.replay.duration_seconds must be between 5 and 600"
        );
        let duration = Duration::from_secs(duration_seconds as u64);
        self.settings.replay.duration_seconds = duration_seconds;
        if let Some(worker) = &self.worker {
            let mut shared = worker
                .shared
                .lock()
                .map_err(|_| anyhow::anyhow!("Replay buffer lock is poisoned"))?;
            shared.ring.set_duration(duration);
            shared.status.duration_seconds = duration_seconds;
            update_status(&mut shared);
            Ok(shared.status.clone())
        } else {
            Ok(self.status())
        }
    }

    pub fn save(
        &self,
        output: Option<PathBuf>,
        duration_seconds: Option<u32>,
    ) -> Result<SavedReplay> {
        let worker = self
            .worker
            .as_ref()
            .context("Instant replay is not running")?;
        let (packets, parameters, time_base, duration) = {
            let shared = worker
                .shared
                .lock()
                .map_err(|_| anyhow::anyhow!("Replay buffer lock is poisoned"))?;
            let parameters = shared
                .parameters
                .clone()
                .context("Replay encoder is waiting for a tagged game")?;
            anyhow::ensure!(!shared.ring.packets.is_empty(), "Replay buffer is empty");
            let requested = duration_seconds.map(|s| Duration::from_secs(s as u64));
            let (packets, duration) = shared.ring.packets_for_duration(requested);
            anyhow::ensure!(
                !packets.is_empty(),
                "No replay packets available for requested duration"
            );
            (packets, parameters, shared.time_base, duration)
        };
        let output = match output {
            Some(path) => path,
            None => crate::unique_output_path(&self.settings.save_path, "mp4")?,
        };
        mux_replay(&output, packets, parameters, time_base)?;
        Ok(SavedReplay {
            path: output,
            duration,
        })
    }
}

impl Drop for ReplayController {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn capture_loop(
    settings: &Settings,
    shared: &Arc<Mutex<SharedCapture>>,
    stop: &AtomicBool,
) -> Result<()> {
    let connection = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let mut capture = HyprlandToplevelCapture::new(&connection)?;
    let frame_interval = Duration::from_secs_f64(1.0 / settings.recording.max_fps as f64);
    let mut encoder: Option<HardwareEncoder> = None;
    let mut current_address = None;
    let mut target_missing_since = None;

    while !stop.load(Ordering::Acquire) {
        let loop_started = Instant::now();
        let tag = settings
            .replay
            .hyprland_tag
            .as_deref()
            .context("capture.replay.hyprland_tag is required")?;
        let windows = compositor::backend::hyprland::get_tagged_windows(tag)?;
        let selected = select_target(&windows, current_address);

        let Some(window) = selected else {
            let missing_since = target_missing_since.get_or_insert_with(Instant::now);
            let retaining = current_address.is_some()
                && missing_since.elapsed()
                    < Duration::from_secs(settings.replay.retain_after_exit_seconds as u64);
            let mut state = shared.lock().unwrap_or_else(|lock| lock.into_inner());
            state.status.state = if retaining {
                ServiceState::RetainingAfterExit
            } else {
                state.ring.clear();
                state.parameters = None;
                state.status.target = None;
                state.status.target_app = None;
                state.status.target_title = None;
                state.status.message = None;
                encoder = None;
                current_address = None;
                ServiceState::WaitingForTarget
            };
            update_status(&mut state);
            drop(state);
            thread::sleep(Duration::from_millis(250));
            continue;
        };

        target_missing_since = None;
        let target_changed = current_address != Some(window.address);
        if target_changed {
            let (width, height, fourcc) = match capture.probe_hardware(window.capture_handle) {
                Ok(probe) => probe,
                Err(error) => {
                    let mut state = shared.lock().unwrap_or_else(|lock| lock.into_inner());
                    state.status.state = ServiceState::WaitingForTarget;
                    state.status.target = Some(format!(
                        "{} — {} (0x{:x})",
                        window.class, window.title, window.address
                    ));
                    state.status.target_app = Some(window.class.clone());
                    state.status.target_title = Some(window.title.clone());
                    state.status.message = Some(format!("Waiting for capture target: {error:#}"));
                    drop(state);
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
            };
            let capture_format = drm_pixel_format(fourcc)?;
            let bitrate = resolve_bitrate_bytes(
                &settings.recording.bitrate,
                width,
                height,
                settings.recording.max_fps,
            )?;
            let next = HardwareEncoder::new(
                width,
                height,
                settings.recording.max_fps,
                bitrate * 8,
                capture_format,
                std::path::Path::new(&settings.replay.dri_device),
            )?;
            let mut state = shared.lock().unwrap_or_else(|lock| lock.into_inner());
            state.ring.clear();
            state.ring.set_max_bytes(
                bitrate as u64 * u64::from(settings.replay.duration_seconds) * 3 / 2,
            );
            state.parameters = Some(next.parameters());
            state.time_base = next.time_base();
            state.status.target = Some(format!(
                "{} — {} (0x{:x})",
                window.class, window.title, window.address
            ));
            state.status.target_app = Some(window.class.clone());
            state.status.target_title = Some(window.title.clone());
            state.status.message = None;
            encoder = Some(next);
            current_address = Some(window.address);
        }

        let captured = {
            let encoder = encoder
                .as_mut()
                .context("Replay encoder was not initialized")?;
            let hardware_frame = encoder.allocate_capture_frame()?;
            match capture.capture_hardware(window.capture_handle, &hardware_frame) {
                Ok((timestamp, y_inverted)) => Some((
                    encoder.encode(hardware_frame, timestamp, y_inverted)?,
                    timestamp,
                )),
                Err(error) if error.to_string().starts_with("Toplevel dimensions changed") => None,
                Err(error) => {
                    let mut state = shared.lock().unwrap_or_else(|lock| lock.into_inner());
                    state.status.state = ServiceState::WaitingForTarget;
                    state.status.message = Some(format!("Waiting for capture target: {error:#}"));
                    None
                }
            }
        };
        let Some((packets, fallback_timestamp)) = captured else {
            encoder = None;
            current_address = None;
            continue;
        };
        let mut state = shared.lock().unwrap_or_else(|lock| lock.into_inner());
        for packet in packets {
            let timestamp = packet.pts().unwrap_or(fallback_timestamp);
            state.ring.push(packet, timestamp);
        }
        state.status.state = ServiceState::Buffering;
        update_status(&mut state);
        drop(state);

        if let Some(remaining) = frame_interval.checked_sub(loop_started.elapsed()) {
            thread::sleep(remaining);
        }
    }
    Ok(())
}

fn select_target(windows: &[TaggedWindow], current_address: Option<u64>) -> Option<&TaggedWindow> {
    windows
        .iter()
        .filter(|window| window.focus_history_id >= 0)
        .min_by_key(|window| {
            (
                window.focus_history_id,
                Some(window.address) != current_address,
            )
        })
        .or_else(|| {
            windows
                .iter()
                .find(|window| Some(window.address) == current_address)
        })
        .or_else(|| windows.first())
}

fn update_status(shared: &mut SharedCapture) {
    shared.status.buffered_millis = shared.ring.buffered_duration().as_millis() as u64;
    shared.status.buffered_bytes = shared.ring.bytes;
}

fn drm_pixel_format(fourcc: u32) -> Result<ffmpeg::format::Pixel> {
    use drm::buffer::DrmFourcc;

    match DrmFourcc::try_from(fourcc)
        .map_err(|_| anyhow::anyhow!("Unknown DRM format {fourcc:#x}"))?
    {
        DrmFourcc::Xrgb8888 => Ok(ffmpeg::format::Pixel::BGRZ),
        DrmFourcc::Argb8888 => Ok(ffmpeg::format::Pixel::BGRA),
        format => anyhow::bail!("Unsupported DMA-BUF capture format {format:?}"),
    }
}
fn resolve_bitrate_bytes(value: &str, width: u32, height: u32, fps: u32) -> Result<usize> {
    if !value.trim().is_empty() {
        let mut parts = value.split_whitespace();
        let amount: f64 = parts
            .next()
            .context("Missing bitrate value")?
            .parse()
            .context("Invalid bitrate value")?;
        let unit = parts.next().unwrap_or("B");
        let multiplier = match unit.to_ascii_lowercase().as_str() {
            "b" => 1.0,
            "kb" => 1_000.0,
            "mb" => 1_000_000.0,
            other => anyhow::bail!("Unsupported bitrate unit {other}"),
        };
        return Ok((amount * multiplier) as usize);
    }
    let bytes = width as f64 * height as f64 * fps as f64 * 0.35 / 8.0;
    Ok((bytes / 1_000_000.0).round().clamp(5.0, 25.0) as usize * 1_000_000)
}

fn mux_replay(
    output: &PathBuf,
    packets: Vec<ffmpeg::Packet>,
    parameters: ffmpeg::codec::Parameters,
    time_base: ffmpeg::Rational,
) -> Result<()> {
    anyhow::ensure!(
        output
            .extension()
            .is_some_and(|extension| extension == "mp4"),
        "Replay output must use the .mp4 extension"
    );
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let temporary = output.with_extension(format!("mp4.part-{}", std::process::id()));
    let result = (|| -> Result<()> {
        let mut context =
            ffmpeg::format::output_as(&temporary, "mp4").context("Failed to create replay MP4")?;
        let mut stream = context
            .add_stream(ffmpeg::encoder::find(parameters.id()))
            .context("Failed to add replay video stream")?;
        stream.set_parameters(parameters);
        stream.set_time_base(time_base);
        let stream_index = stream.index();
        context
            .write_header()
            .context("Failed to write MP4 header")?;
        let output_time_base = context
            .stream(stream_index)
            .context("Replay stream disappeared")?
            .time_base();
        let first_timestamp = packets
            .iter()
            .filter_map(|packet| packet.dts().or_else(|| packet.pts()))
            .min()
            .context("Replay contains no timestamps")?;
        for mut packet in packets {
            packet.set_pts(packet.pts().map(|pts| pts - first_timestamp));
            packet.set_dts(packet.dts().map(|dts| dts - first_timestamp));
            packet.set_stream(stream_index);
            packet.rescale_ts(time_base, output_time_base);
            packet.set_position(-1);
            packet
                .write_interleaved(&mut context)
                .context("Failed to write replay packet")?;
        }
        context
            .write_trailer()
            .context("Failed to finish replay MP4")?;
        OpenOptions::new().read(true).open(&temporary)?.sync_all()?;
        fs::rename(&temporary, output)
            .with_context(|| format!("Failed to publish {}", output.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
#[path = "capture_test.rs"]
mod tests;
