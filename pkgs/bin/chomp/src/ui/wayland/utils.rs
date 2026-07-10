//! Utility functions for Wayland module

use crate::cli::Settings;
use crate::render::{RenderConfig, Renderer};
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

// Event loop timeout
pub const IDLE_FRAME_TIMEOUT_MS: u64 = 33; // ~30 FPS when idle

/// Acquires an exclusive, non-blocking lock on a per-user runtime lock file.
///
/// Returns the open lock file (which must be kept alive for the process lifetime
/// to retain the lock). If another chomp overlay instance already holds the lock,
/// returns `Err` so the caller can exit rather than stacking another overlay.
pub fn ensure_single_instance() -> Result<std::fs::File> {
    let lock_path = runtime_lock_path();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| format!("Failed to open lock file {}", lock_path.display()))?;

    match nix::fcntl::flock(file.as_raw_fd(), nix::fcntl::FlockArg::LockExclusiveNonblock) {
        Ok(()) => Ok(file),
        Err(e)
            if e == nix::errno::Errno::EWOULDBLOCK || e == nix::errno::Errno::EAGAIN =>
        {
            anyhow::bail!("another chomp instance already holds the overlay lock")
        }
        Err(e) => Err(e).with_context(|| "Failed to acquire instance lock"),
    }
}

/// Resolves the path of the per-user runtime lock file.
fn runtime_lock_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("chomp.lock");
    }
    if let Ok(uid) = std::env::var("UID") {
        return PathBuf::from(format!("/run/user/{}/chomp.lock", uid));
    }
    std::env::temp_dir().join("chomp.lock")
}

/// Creates a renderer with the specified dimensions and styling configuration.
///
/// Returns `None` if renderer creation fails due to invalid color values or other configuration errors.
pub fn create_renderer(width: i32, height: i32, settings: &Settings) -> Option<Renderer> {
    let config = RenderConfig::new(
        &settings.border_color,
        settings.border_thickness,
        settings.border_rounding,
        settings.dim_opacity,
        settings.font_family.clone(),
        settings.font_size,
        settings.font_weight,
    )
    .ok()?;

    Some(Renderer::new(width, height, config))
}
