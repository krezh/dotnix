mod capture;
pub(crate) mod hardware;
pub mod ipc;
mod service;

pub use ipc::{ReplayStatus, ServiceState};

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{ReplayAction, Settings};

/// Queries the replay service status with a short deadline.
///
/// Returns `None` if the service is not running, the socket is absent, or the
/// service is unresponsive. This is intended for interactive UI startup so the
/// overlay never hangs when the service is down.
pub fn query_status() -> Option<ReplayStatus> {
    let path = ipc::socket_path().ok()?;
    if !path.exists() {
        return None;
    }
    match ipc::request_with_timeout(
        ipc::ReplayCommand::Status,
        Some(Duration::from_millis(500)),
        Some(Duration::from_millis(500)),
    ) {
        Ok(ipc::ReplayReply::Status(status)) => Some(status),
        _ => None,
    }
}
pub fn run_service(settings: Settings) -> Result<()> {
    service::run(settings)
}

pub fn control(settings: &Settings, action: ReplayAction) -> Result<Option<PathBuf>> {
    let command = match action {
        ReplayAction::Start => ipc::ReplayCommand::Start,
        ReplayAction::Save => ipc::ReplayCommand::Save {
            output: settings
                .output
                .clone()
                .map(absolute_output_path)
                .transpose()?,
        },
        ReplayAction::Stop => ipc::ReplayCommand::Stop,
        ReplayAction::Status => ipc::ReplayCommand::Status,
    };

    match ipc::request(command)? {
        ipc::ReplayReply::Status(status) => {
            println!("Instant replay: {:?}", status.state);
            if let Some(target) = status.target {
                println!("Target: {target}");
            }
            println!(
                "Buffer: {:.1}s, {:.1} MiB",
                status.buffered_millis as f64 / 1000.0,
                status.buffered_bytes as f64 / 1024.0 / 1024.0
            );
            if let Some(message) = status.message {
                println!("Message: {message}");
            }
            Ok(None)
        }
        ipc::ReplayReply::Saved {
            output,
            duration_millis,
        } => {
            println!(
                "Replay saved to {} ({:.1}s)",
                output.display(),
                duration_millis as f64 / 1000.0
            );
            Ok(Some(output))
        }
        ipc::ReplayReply::Stopped => {
            println!("Instant replay stopped");
            Ok(None)
        }
    }
}

fn absolute_output_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(std::env::current_dir()
        .context("Failed to resolve the current directory")?
        .join(path))
}

#[cfg(test)]
#[path = "replay_test.rs"]
mod tests;
