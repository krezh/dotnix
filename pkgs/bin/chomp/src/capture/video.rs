//! Video recording management using wl-screenrec
//!
//! Recording state is tracked in a runtime state file containing the wl-screenrec
//! PID and output path, so only recordings started by chomp are detected and stopped.

use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const STOP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize)]
struct RecordingState {
    pid: u32,
    output_file: String,
}

pub struct VideoRecorder;

impl VideoRecorder {
    pub fn new() -> Self {
        Self
    }

    /// Checks if a chomp-started wl-screenrec recording is active.
    ///
    /// Returns (is_recording, output_file_path). Removes stale state files
    /// whose process no longer exists.
    pub fn is_recording(&self) -> Result<(bool, Option<String>)> {
        match Self::load_state() {
            Some(state) if Self::pid_is_wl_screenrec(state.pid) => {
                Ok((true, Some(state.output_file)))
            }
            Some(_) => {
                let _ = fs::remove_file(Self::state_file());
                Ok((false, None))
            }
            None => Ok((false, None)),
        }
    }

    /// Stops the active recording by sending SIGINT to its wl-screenrec process
    /// and waiting for it to exit.
    ///
    /// Returns the output file path.
    pub fn stop_recording(&self) -> Result<Option<String>> {
        let state = Self::load_state().filter(|s| Self::pid_is_wl_screenrec(s.pid));
        let Some(state) = state else {
            let _ = fs::remove_file(Self::state_file());
            anyhow::bail!("No recording active");
        };

        kill(Pid::from_raw(state.pid as i32), Signal::SIGINT)
            .context("Failed to signal wl-screenrec")?;

        let deadline = Instant::now() + STOP_TIMEOUT;
        while Self::pid_is_wl_screenrec(state.pid) {
            if Instant::now() >= deadline {
                log::warn!(
                    "wl-screenrec (pid {}) did not exit within {:?}; recording may be incomplete",
                    state.pid,
                    STOP_TIMEOUT
                );
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let _ = fs::remove_file(Self::state_file());
        Ok(Some(state.output_file))
    }

    /// Starts wl-screenrec with the given parameters and records its state.
    pub fn start_recording(
        &self,
        geometry: Option<&str>,
        monitor: Option<&str>,
        output_file: &str,
    ) -> Result<()> {
        let mut args = vec![
            "--low-power=off",
            "--max-fps=60",
            "--encode-resolution=1920x1080",
        ];

        if let Some(g) = geometry {
            args.extend(["-g", g]);
        }

        if let Some(m) = monitor {
            args.extend(["-o", m]);
        }

        args.extend(["-f", output_file]);

        let child = Command::new("wl-screenrec")
            .args(&args)
            .spawn()
            .context("Failed to start wl-screenrec")?;

        let state = RecordingState {
            pid: child.id(),
            output_file: output_file.to_string(),
        };
        if let Err(e) = fs::write(Self::state_file(), serde_json::to_string(&state)?) {
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
        let content = fs::read_to_string(Self::state_file()).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Returns true if the PID is a live process whose executable is wl-screenrec.
    fn pid_is_wl_screenrec(pid: u32) -> bool {
        fs::read_to_string(format!("/proc/{}/cmdline", pid))
            .ok()
            .and_then(|cmdline| {
                cmdline.split('\0').next().map(|arg0| {
                    Path::new(arg0)
                        .file_name()
                        .is_some_and(|name| name == "wl-screenrec")
                })
            })
            .unwrap_or(false)
    }
}
