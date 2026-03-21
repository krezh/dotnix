//! Video recording management using wl-screenrec

use anyhow::{Context, Result};
use std::fs;
use std::process::Command;
use std::time::Duration;

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
            // Send SIGINT (2) to gracefully stop recording
            let _ = Command::new("kill")
                .args(["-2", &pid.to_string()])
                .output();
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
        let mut pids = Vec::new();

        let proc_dir = fs::read_dir("/proc")?;
        for entry in proc_dir.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Check if directory name is numeric (PID)
            if let Ok(pid) = name.parse::<u32>() {
                let comm_path = format!("/proc/{}/comm", pid);
                if let Ok(comm) = fs::read_to_string(comm_path) {
                    if comm.trim() == "wl-screenrec" {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    fn read_cmdline(&self, pid: u32) -> Result<String> {
        let path = format!("/proc/{}/cmdline", pid);
        fs::read_to_string(path).context("Failed to read cmdline")
    }
}
