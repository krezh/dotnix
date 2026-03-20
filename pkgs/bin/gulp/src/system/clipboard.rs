//! Clipboard operations using wl-copy

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

/// Copies text to the Wayland clipboard.
pub fn copy_text(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn wl-copy")?;

    child.stdin.take()
        .context("Failed to get stdin")?
        .write_all(text.as_bytes())?;

    child.wait()?;
    Ok(())
}

/// Copies an image file to the Wayland clipboard.
pub fn copy_image(file_path: &str) -> Result<()> {
    Command::new("wl-copy")
        .arg("-t")
        .arg("image/png")
        .stdin(fs::File::open(file_path)?)
        .spawn()
        .context("Failed to copy image to clipboard")?
        .wait()?;
    Ok(())
}
