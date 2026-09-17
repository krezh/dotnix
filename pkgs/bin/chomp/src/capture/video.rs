//! Video recording management using wl-screenrec
//!
//! Recording state is tracked in a runtime state file containing the recorder
//! PID and output path, so only recordings started by chomp are detected and stopped.

use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::cli::Settings;

const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// Bits per pixel the derived bitrate aims for, which holds up in motion.
const TARGET_BITS_PER_PIXEL: f64 = 0.35;
const MIN_BITRATE_MB: u64 = 5;
const MAX_BITRATE_MB: u64 = 25;

/// Used when the recorded size is unknown; the recorder's own default.
const DEFAULT_BITRATE: &str = "5 MB";

#[derive(Serialize, Deserialize)]
struct RecordingState {
    pid: u32,
    output_file: String,
}

/// Returns the output path of the recording chomp has running, if any.
///
/// Removes a stale state file whose process is gone.
pub fn recording(settings: &Settings) -> Option<String> {
    match load_state() {
        Some(state) if pid_is_recorder(&settings.wl_screenrec, state.pid) => {
            Some(state.output_file)
        }
        Some(_) => {
            let _ = fs::remove_file(state_file());
            None
        }
        None => None,
    }
}

/// Stops the active recording by sending SIGINT to its recorder process and
/// waiting for it to exit.
///
/// Returns the output file path.
pub fn stop_recording(settings: &Settings) -> Result<String> {
    let recorder = &settings.wl_screenrec;
    let state = load_state().filter(|s| pid_is_recorder(recorder, s.pid));

    let Some(state) = state else {
        let _ = fs::remove_file(state_file());
        anyhow::bail!("No recording active");
    };

    kill(Pid::from_raw(state.pid as i32), Signal::SIGINT)
        .context("Failed to signal the recorder")?;

    let deadline = Instant::now() + STOP_TIMEOUT;
    while pid_is_recorder(recorder, state.pid) {
        if Instant::now() >= deadline {
            log::warn!(
                "{} (pid {}) did not exit within {:?}; recording may be incomplete",
                recorder,
                state.pid,
                STOP_TIMEOUT
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = fs::remove_file(state_file());

    Ok(state.output_file)
}

/// Starts the recorder with the given parameters and records its state.
///
/// `size` is the recorded area in pixels, used to pick a bitrate when none is
/// configured.
pub fn start_recording(
    settings: &Settings,
    geometry: Option<&str>,
    monitor: Option<&str>,
    size: Option<(u32, u32)>,
    output_file: &str,
) -> Result<()> {
    let max_fps = format!("--max-fps={}", settings.video_max_fps);
    let encode_resolution = format!("--encode-resolution={}", settings.video_encode_resolution);
    let codec = format!("--codec={}", settings.video_codec);
    let bitrate = format!("--bitrate={}", resolve_bitrate(settings, size));

    let mut args = vec!["--low-power=off", max_fps.as_str(), &bitrate];

    // Left empty, the recorder encodes at the output's own resolution.
    if !settings.video_encode_resolution.is_empty() {
        args.push(&encode_resolution);
    }

    if !settings.video_codec.is_empty() && settings.video_codec != "auto" {
        args.push(&codec);
    }

    if let Some(g) = geometry {
        args.extend(["-g", g]);
    }

    if let Some(m) = monitor {
        args.extend(["-o", m]);
    }

    args.extend(["-f", output_file]);

    let child = Command::new(&settings.wl_screenrec)
        .args(&args)
        .spawn()
        .with_context(|| format!("Failed to start {}", settings.wl_screenrec))?;

    let state = RecordingState {
        pid: child.id(),
        output_file: output_file.to_string(),
    };

    if let Err(e) = fs::write(state_file(), serde_json::to_string(&state)?) {
        let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGINT);
        return Err(e).context("Failed to write recording state file");
    }

    Ok(())
}

/// Returns the bitrate to record at, in the recorder's own byte-per-second units.
///
/// The recorder's own default is a fixed 5 MB/s, which is sized for 1080p: at
/// 1440p or 4K the same bits are spread over two to four times the pixels, and
/// anything with motion in it breaks up into blocks. Scaling with the pixel rate
/// keeps quality steady across resolutions, and lands on that same 5 MB/s at
/// 1080p60.
fn resolve_bitrate(settings: &Settings, size: Option<(u32, u32)>) -> String {
    if !settings.video_bitrate.is_empty() {
        return settings.video_bitrate.clone();
    }

    // What is encoded, which is the encoder resolution when one is set.
    let encoded = parse_resolution(&settings.video_encode_resolution).or(size);

    let Some((width, height)) = encoded else {
        return DEFAULT_BITRATE.to_string();
    };

    let pixels_per_second = width as f64 * height as f64 * settings.video_max_fps as f64;
    let bytes_per_second = pixels_per_second * TARGET_BITS_PER_PIXEL / 8.0;
    let megabytes = (bytes_per_second / 1_000_000.0).round() as u64;

    format!("{} MB", megabytes.clamp(MIN_BITRATE_MB, MAX_BITRATE_MB))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(bitrate: &str, encode_resolution: &str, max_fps: u32) -> Settings {
        use clap::Parser;

        let mut settings = crate::cli::Args::parse_from(["chomp"]).resolve(Default::default());
        settings.video_bitrate = bitrate.to_string();
        settings.video_encode_resolution = encode_resolution.to_string();
        settings.video_max_fps = max_fps;
        settings
    }

    #[test]
    fn keeps_a_configured_bitrate() {
        let settings = settings("15 MB", "", 60);

        assert_eq!(resolve_bitrate(&settings, Some((2560, 1440))), "15 MB");
    }

    #[test]
    fn scales_the_bitrate_with_the_recorded_area() {
        let settings = settings("", "", 60);

        // 1080p60 lands on the recorder's own default, 1440p60 above it.
        assert_eq!(resolve_bitrate(&settings, Some((1920, 1080))), "5 MB");
        assert_eq!(resolve_bitrate(&settings, Some((2560, 1440))), "10 MB");
        assert_eq!(resolve_bitrate(&settings, Some((3840, 2160))), "22 MB");
    }

    #[test]
    fn sizes_the_bitrate_to_the_encoder_resolution() {
        let settings = settings("", "1920x1080", 60);

        // Downscaled to 1080p, so a 1440p region still encodes at 1080p's rate.
        assert_eq!(resolve_bitrate(&settings, Some((2560, 1440))), "5 MB");
    }

    #[test]
    fn falls_back_without_a_known_area() {
        let settings = settings("", "", 60);

        assert_eq!(resolve_bitrate(&settings, None), DEFAULT_BITRATE);
    }
}

/// Parses a "WIDTHxHEIGHT" resolution.
fn parse_resolution(resolution: &str) -> Option<(u32, u32)> {
    let (width, height) = resolution.split_once('x')?;

    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

fn state_file() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join("chomp-recording.json")
}

fn load_state() -> Option<RecordingState> {
    let content = fs::read_to_string(state_file()).ok()?;
    serde_json::from_str(&content).ok()
}

/// Returns true if the PID is a live process running the configured recorder.
///
/// The recording is stopped by a second chomp process, which is not the
/// recorder's parent, so there is no child to wait on — only a PID, which the
/// kernel may have reused for something else by then.
fn pid_is_recorder(recorder: &str, pid: u32) -> bool {
    let name = Path::new(recorder).file_name();

    fs::read_to_string(format!("/proc/{}/cmdline", pid))
        .ok()
        .and_then(|cmdline| {
            cmdline
                .split('\0')
                .next()
                .map(|arg0| Path::new(arg0).file_name() == name)
        })
        .unwrap_or(false)
}
