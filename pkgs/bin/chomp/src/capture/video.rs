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
pub fn start_recording(
    settings: &Settings,
    geometry: Option<&str>,
    monitor: Option<&str>,
    output_file: &str,
) -> Result<()> {
    let max_fps = format!("--max-fps={}", settings.video_max_fps);
    let encode_resolution = format!("--encode-resolution={}", settings.video_encode_resolution);

    let mut args = vec!["--low-power=off", max_fps.as_str()];

    // Left empty, the recorder encodes at the output's own resolution.
    if !settings.video_encode_resolution.is_empty() {
        args.push(&encode_resolution);
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
