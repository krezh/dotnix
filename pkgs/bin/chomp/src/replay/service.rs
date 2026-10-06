use anyhow::{Context, Result};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

use super::capture::ReplayController;
use super::ipc::{self, ReplayCommand, ReplayReply};
use crate::cli::Settings;

pub fn run(settings: Settings) -> Result<()> {
    let path = ipc::socket_path()?;
    let directory = path
        .parent()
        .context("Replay socket has no parent directory")?;
    fs::create_dir_all(directory)
        .with_context(|| format!("Failed to create {}", directory.display()))?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;

    if path.exists() {
        match UnixStream::connect(&path) {
            Ok(_) => anyhow::bail!("Another replay service is already running"),
            Err(_) => fs::remove_file(&path)
                .with_context(|| format!("Failed to remove stale {}", path.display()))?,
        }
    }

    let listener =
        UnixListener::bind(&path).with_context(|| format!("Failed to bind {}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    let enabled = settings.replay.enabled;
    let mut controller = ReplayController::new(settings)?;
    if enabled {
        controller.start()?;
    }
    log::info!("Replay service listening at {}", path.display());

    let result = serve(&listener, &mut controller);
    let _ = fs::remove_file(&path);
    result
}

fn serve(listener: &UnixListener, controller: &mut ReplayController) -> Result<()> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .context("Failed to accept replay client")?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .context("Failed to configure replay client timeout")?;
        let result = handle_client(&mut stream, controller).map_err(|error| error.to_string());
        if let Err(error) = ipc::write_reply(&mut stream, result) {
            log::warn!("Failed to answer replay client: {}", error);
        }
    }
}

fn handle_client(
    stream: &mut UnixStream,
    controller: &mut ReplayController,
) -> Result<ReplayReply> {
    let credentials =
        getsockopt(stream, PeerCredentials).context("Failed to inspect replay client")?;
    let current_uid = unsafe { libc::geteuid() };
    anyhow::ensure!(
        credentials.uid() == current_uid,
        "Replay client belongs to another user"
    );

    match ipc::read_request(stream)? {
        ReplayCommand::Start => {
            controller.start()?;
            Ok(ReplayReply::Status(controller.status()))
        }
        ReplayCommand::Save { output } => {
            let saved = controller.save(output)?;
            Ok(ReplayReply::Saved {
                output: saved.path,
                duration_millis: saved.duration.as_millis() as u64,
            })
        }
        ReplayCommand::Stop => {
            controller.stop()?;
            Ok(ReplayReply::Stopped)
        }
        ReplayCommand::Status => Ok(ReplayReply::Status(controller.status())),
    }
}
