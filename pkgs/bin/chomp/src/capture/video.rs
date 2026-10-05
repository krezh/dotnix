//! Video recording management using wl-screenrec

use anyhow::{Context, Result};
use nix::fcntl::{Flock, FlockArg};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::cli::{RecordingOptions, Settings};

const STOP_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_GRACE: Duration = Duration::from_millis(200);
const TARGET_BITS_PER_PIXEL: f64 = 0.35;
const MIN_BITRATE_MB: u64 = 5;
const MAX_BITRATE_MB: u64 = 25;
const DEFAULT_BITRATE: &str = "5 MB";

#[derive(Serialize, Deserialize)]
struct RecordingState {
    pid: u32,
    start_time: u64,
    executable: PathBuf,
    output_file: PathBuf,
}

pub fn recording(_settings: &Settings) -> Result<Option<PathBuf>> {
    let _lock = lock_state()?;
    let Some(state) = load_state()? else {
        return Ok(None);
    };

    if process_matches(&state) {
        Ok(Some(state.output_file))
    } else {
        remove_state()?;
        Ok(None)
    }
}

pub fn stop_recording(_settings: &Settings) -> Result<PathBuf> {
    let _lock = lock_state()?;
    let Some(state) = load_state()? else {
        anyhow::bail!("No recording active");
    };

    if !process_matches(&state) {
        remove_state()?;
        anyhow::bail!("No recording active");
    }

    kill(Pid::from_raw(state.pid as i32), Signal::SIGINT)
        .context("Failed to signal the recorder")?;

    let deadline = Instant::now() + STOP_TIMEOUT;
    while process_matches(&state) {
        anyhow::ensure!(
            Instant::now() < deadline,
            "Recorder pid {} did not stop within {:?}; recording state was retained",
            state.pid,
            STOP_TIMEOUT
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    remove_state()?;
    anyhow::ensure!(
        state
            .output_file
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 0),
        "Recorder stopped without producing {}",
        state.output_file.display()
    );
    Ok(state.output_file)
}

pub fn start_recording(
    settings: &Settings,
    geometry: Option<&str>,
    monitor: Option<&str>,
    size: Option<(u32, u32)>,
    output_file: &Path,
) -> Result<()> {
    let _lock = lock_state()?;
    if let Some(state) = load_state()? {
        if process_matches(&state) {
            anyhow::bail!(
                "A recording is already active at {}",
                state.output_file.display()
            );
        }
        remove_state()?;
    }

    let options = &settings.recording;
    let max_fps = format!("--max-fps={}", options.max_fps);
    let encode_resolution = format!("--encode-resolution={}", options.encode_resolution);
    let codec = format!("--codec={}", options.codec);
    let bitrate = format!("--bitrate={}", resolve_bitrate(options, size));

    let mut args = vec!["--low-power=off", max_fps.as_str(), bitrate.as_str()];
    if !options.encode_resolution.is_empty() {
        args.push(encode_resolution.as_str());
    }
    if !options.codec.is_empty() && options.codec != "auto" {
        args.push(codec.as_str());
    }
    if let Some(value) = geometry {
        args.extend(["-g", value]);
    }
    if let Some(value) = monitor {
        args.extend(["-o", value]);
    }
    args.push("-f");

    let mut child = Command::new(&settings.wl_screenrec)
        .args(&args)
        .arg(output_file)
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to start {}", settings.wl_screenrec))?;

    let deadline = Instant::now() + STARTUP_GRACE;
    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .context("Failed to inspect recorder startup")?
        {
            anyhow::bail!(
                "{} exited during startup with {}",
                settings.wl_screenrec,
                status
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let pid = child.id();
    let state = RecordingState {
        pid,
        start_time: process_start_time(pid).context("Recorder disappeared during startup")?,
        executable: fs::read_link(format!("/proc/{pid}/exe"))
            .context("Failed to identify recorder executable")?,
        output_file: output_file.to_path_buf(),
    };

    if let Err(error) = write_state(&state) {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
        return Err(error);
    }

    Ok(())
}

fn resolve_bitrate(options: &RecordingOptions, size: Option<(u32, u32)>) -> String {
    if !options.bitrate.is_empty() {
        return options.bitrate.clone();
    }

    let encoded = parse_resolution(&options.encode_resolution).or(size);
    let Some((width, height)) = encoded else {
        return DEFAULT_BITRATE.to_string();
    };

    let pixels_per_second = width as f64 * height as f64 * options.max_fps as f64;
    let bytes_per_second = pixels_per_second * TARGET_BITS_PER_PIXEL / 8.0;
    let megabytes = (bytes_per_second / 1_000_000.0).round() as u64;
    format!("{} MB", megabytes.clamp(MIN_BITRATE_MB, MAX_BITRATE_MB))
}

fn parse_resolution(resolution: &str) -> Option<(u32, u32)> {
    let (width, height) = resolution.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

fn runtime_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Some(uid) = std::env::var_os("UID") {
        let dir = PathBuf::from("/run/user").join(uid);
        if dir.is_dir() {
            return Ok(dir);
        }
    }
    anyhow::bail!("XDG_RUNTIME_DIR is not set and no private user runtime directory exists")
}

fn state_file() -> Result<PathBuf> {
    Ok(runtime_dir()?.join("chomp-recording.json"))
}

fn lock_state() -> Result<Flock<fs::File>> {
    let path = runtime_dir()?.join("chomp-recording.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(&path)
        .with_context(|| format!("Failed to open recording lock {}", path.display()))?;
    Flock::lock(file, FlockArg::LockExclusive)
        .map_err(|(_, error)| error)
        .context("Failed to lock recording state")
}

fn load_state() -> Result<Option<RecordingState>> {
    let path = state_file()?;
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse recording state {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Failed to read {}", path.display())),
    }
}

fn write_state(state: &RecordingState) -> Result<()> {
    let path = state_file()?;
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec(state).context("Failed to encode recording state")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(&temp)
        .with_context(|| format!("Failed to create {}", temp.display()))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(error).context("Failed to persist recording state");
    }
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(error).context("Failed to publish recording state");
    }
    Ok(())
}

fn remove_state() -> Result<()> {
    let path = state_file()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Failed to remove {}", path.display())),
    }
}

fn process_matches(state: &RecordingState) -> bool {
    process_start_time(state.pid) == Some(state.start_time)
        && fs::read_link(format!("/proc/{}/exe", state.pid))
            .is_ok_and(|path| path == state.executable)
}

fn process_start_time(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields = stat
        .rsplit_once(") ")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    fields.get(19)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(bitrate: &str, encode_resolution: &str, max_fps: u32) -> RecordingOptions {
        RecordingOptions {
            bitrate: bitrate.to_string(),
            encode_resolution: encode_resolution.to_string(),
            max_fps,
            codec: "auto".to_string(),
        }
    }

    #[test]
    fn keeps_a_configured_bitrate() {
        assert_eq!(
            resolve_bitrate(&options("15 MB", "", 60), Some((2560, 1440))),
            "15 MB"
        );
    }

    #[test]
    fn scales_the_bitrate_with_the_recorded_area() {
        let options = options("", "", 60);
        assert_eq!(resolve_bitrate(&options, Some((1920, 1080))), "5 MB");
        assert_eq!(resolve_bitrate(&options, Some((2560, 1440))), "10 MB");
        assert_eq!(resolve_bitrate(&options, Some((3840, 2160))), "22 MB");
    }

    #[test]
    fn sizes_the_bitrate_to_the_encoder_resolution() {
        let options = options("", "1920x1080", 60);
        assert_eq!(resolve_bitrate(&options, Some((2560, 1440))), "5 MB");
    }

    #[test]
    fn falls_back_without_a_known_area() {
        assert_eq!(resolve_bitrate(&options("", "", 60), None), DEFAULT_BITRATE);
    }

    #[test]
    fn reads_the_current_process_identity() {
        let pid = std::process::id();
        assert!(process_start_time(pid).is_some());
    }

    #[test]
    fn rejects_a_recorder_that_exits_during_startup() {
        use clap::Parser;
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let recorder = directory.path().join("wl-screenrec");
        fs::write(&recorder, "#!/bin/sh\nexit 23\n").unwrap();
        fs::set_permissions(&recorder, fs::Permissions::from_mode(0o700)).unwrap();

        let previous = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::set_var("XDG_RUNTIME_DIR", directory.path());
        let mut settings = crate::cli::Args::parse_from(["chomp"]).resolve(Default::default());
        settings.wl_screenrec = recorder.to_string_lossy().into_owned();
        let result = start_recording(
            &settings,
            None,
            None,
            Some((1920, 1080)),
            &directory.path().join("recording.mp4"),
        );
        match previous {
            Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }

        assert!(result.is_err());
    }
}
