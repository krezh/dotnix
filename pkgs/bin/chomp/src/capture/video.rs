//! Video recording management using wl-screenrec

use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::fs;
use std::process::Command;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub struct VideoRecorder;

impl VideoRecorder {
    pub fn new() -> Self {
        Self
    }

    /// Checks if wl-screenrec is currently recording.
    ///
    /// Returns (is_recording, output_file_path).
    pub fn is_recording(&self) -> Result<(bool, Option<String>)> {
        let pids = self.find_wl_screenrec_pids()?;

        if pids.is_empty() {
            return Ok((false, None));
        }

        // Find output file from first process cmdline
        for pid in &pids {
            if let Ok(cmdline) = self.read_cmdline(*pid) {
                let args: Vec<&str> = cmdline.split('\0').filter(|s| !s.is_empty()).collect();
                for i in 0..args.len() {
                    if args[i] == "-f" && i + 1 < args.len() {
                        return Ok((true, Some(args[i + 1].to_string())));
                    }
                }
            }
        }

        Ok((true, None))
    }

    /// Stops active recording by sending SIGINT to wl-screenrec.
    ///
    /// Returns the output file path if found.
    pub fn stop_recording(&self) -> Result<Option<String>> {
        let (is_recording, output_file) = self.is_recording()?;

        if !is_recording {
            anyhow::bail!("No recording active");
        }

        let pids = self.find_wl_screenrec_pids()?;
        for pid in pids {
            // Send SIGINT to gracefully stop recording
            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
        }

        // Wait for process to finish writing
        std::thread::sleep(Duration::from_millis(500));

        Ok(output_file)
    }

    /// Starts wl-screenrec with the given parameters.
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

        Command::new("wl-screenrec")
            .args(&args)
            .spawn()
            .context("Failed to start wl-screenrec")?;

        Ok(())
    }

    fn find_wl_screenrec_pids(&self) -> Result<Vec<u32>> {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing())
        );
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let pids = sys
            .processes()
            .iter()
            .filter(|(_, process)| {
                process.name().to_string_lossy() == "wl-screenrec"
            })
            .map(|(pid, _)| pid.as_u32())
            .collect();

        Ok(pids)
    }

    fn read_cmdline(&self, pid: u32) -> Result<String> {
        let path = format!("/proc/{}/cmdline", pid);
        fs::read_to_string(path).context("Failed to read cmdline")
    }
}
