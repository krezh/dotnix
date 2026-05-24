//! Screenshot annotation using satty

use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Pipes PNG image data to satty over stdin, delegating copy to wl-copy and writing
/// the annotated result to output_path. Exits after the copy/save action. Blocks until satty exits.
pub fn annotate(satty_path: &str, png_data: &[u8], output_path: &str) -> Result<()> {
    let mut child = Command::new(satty_path)
        .arg("--filename")
        .arg("-")
        .arg("--output-filename")
        .arg(output_path)
        .arg("--copy-command")
        .arg("wl-copy --type image/png")
        .arg("--fullscreen")
        .arg("--early-exit")
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn satty ({})", satty_path))?;

    child
        .stdin
        .take()
        .context("Failed to open satty stdin")?
        .write_all(png_data)
        .context("Failed to write image to satty stdin")?;

    let status = child.wait().context("satty failed")?;
    if !status.success() {
        anyhow::bail!("satty exited with status: {}", status);
    }

    Ok(())
}
