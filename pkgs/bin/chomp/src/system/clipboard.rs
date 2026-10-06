//! Clipboard operations using wl-copy
//!
//! Uses wl-copy binary which properly daemonizes to keep serving clipboard data
//! after the process exits. The wl-clipboard-rs library spawns threads which die
//! when the process exits, causing clipboard data loss.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Copies text to the clipboard.
pub fn copy_text(wl_copy: &str, text: &str) -> Result<()> {
    run_wl_copy(wl_copy, &[], Stdio::piped(), Some(text.as_bytes()))
}

/// Copies an image file to the clipboard.
pub fn copy_image(wl_copy: &str, file_path: &Path) -> Result<()> {
    let file = std::fs::File::open(file_path).context("Failed to open image file")?;

    run_wl_copy(wl_copy, &["-t", "image/png"], Stdio::from(file), None)
}

/// Runs `wl-copy` with `args`, writing `input` to it when its stdin is a pipe,
/// and waits for it to take ownership of the selection.
fn run_wl_copy(wl_copy: &str, args: &[&str], stdin: Stdio, input: Option<&[u8]>) -> Result<()> {
    let mut child = Command::new(wl_copy)
        .args(args)
        .stdin(stdin)
        .spawn()
        .context("Failed to spawn wl-copy")?;

    if let Some(bytes) = input {
        child
            .stdin
            .take()
            .context("Failed to get stdin")?
            .write_all(bytes)
            .context("Failed to write to wl-copy")?;
    }

    let status = child.wait().context("wl-copy failed")?;
    anyhow::ensure!(status.success(), "wl-copy exited with status {}", status);
    Ok(())
}

#[cfg(test)]
#[path = "clipboard_test.rs"]
mod tests;
