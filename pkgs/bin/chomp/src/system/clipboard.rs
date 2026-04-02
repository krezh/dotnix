//! Clipboard operations using wl-copy
//!
//! Uses wl-copy binary which properly daemonizes to keep serving clipboard data
//! after the process exits. The wl-clipboard-rs library spawns threads which die
//! when the process exits, causing clipboard data loss.

use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Copies text to the clipboard.
pub fn copy_text(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn wl-copy")?;

    child
        .stdin
        .take()
        .context("Failed to get stdin")?
        .write_all(text.as_bytes())
        .context("Failed to write to wl-copy")?;

    child.wait().context("wl-copy failed")?;
    Ok(())
}

/// Copies an image file to the clipboard.
pub fn copy_image(file_path: &str) -> Result<()> {
    Command::new("wl-copy")
        .arg("-t")
        .arg("image/png")
        .stdin(std::fs::File::open(file_path).context("Failed to open image file")?)
        .spawn()
        .context("Failed to spawn wl-copy")?
        .wait()
        .context("wl-copy failed")?;
    Ok(())
}
