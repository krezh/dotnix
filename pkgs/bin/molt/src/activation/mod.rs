pub(crate) mod ui;

use std::io::Read;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use molt::command;
use molt::protocol::{ActivationMode, HelperRequest, HelperResponse};

use crate::build::{Target, profile_target};
use crate::config::CleanupPolicy;
use crate::nix::{run, strip_ansi};
use crate::report::Report;

const SOCKET_PATH: &str = "/run/molt.sock";
const HOME_MANAGER_ACTIVATE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SERVICE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(36 * 60);
const MAX_SERVICE_RESPONSE_BYTES: usize = 128 * 1024;
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn activate(
    report: &Report,
    mode: ActivationMode,
    cancellation: &AtomicBool,
) -> Result<(), String> {
    match report.target {
        Target::HomeManager => {
            if mode == ActivationMode::Boot {
                return Err("Home Manager has no boot activation".to_owned());
            }
            ensure_profile_unchanged(report)?;
            run(
                &mut Command::new(report.output.join("activate")),
                "Home Manager activation",
                cancellation,
                HOME_MANAGER_ACTIVATE_TIMEOUT,
                1024 * 1024,
            )?;
        }
        Target::NixOs => {
            let request_id = request_id();
            let (baseline, output) = (&report.profile, &report.output);
            let request = match mode {
                ActivationMode::Switch => HelperRequest::Activate {
                    request_id: &request_id,
                    baseline,
                    output,
                },
                ActivationMode::Boot => HelperRequest::ActivateOnBoot {
                    request_id: &request_id,
                    baseline,
                    output,
                },
            };
            service_request(&request, &request_id, cancellation)?;
        }
    }
    Ok(())
}

pub(crate) fn analyze_nixos(
    output: &std::path::Path,
    cancellation: &AtomicBool,
) -> Result<Vec<String>, String> {
    let request_id = request_id();
    let output = service_request(
        &HelperRequest::Analyze {
            request_id: &request_id,
            output,
        },
        &request_id,
        cancellation,
    )?
    .unwrap_or_default();
    let mut lines = strip_ansi(&output)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.dedup();
    Ok(lines)
}

pub(crate) fn clean_system(
    nix: bool,
    journals: bool,
    policy: &CleanupPolicy,
    cancellation: &AtomicBool,
) -> Result<(), String> {
    let request_id = request_id();
    service_request(
        &HelperRequest::Cleanup {
            request_id: &request_id,
            nix,
            journals,
            keep_generations: policy.keep_generations,
            journal_max_age: &policy.journal_max_age,
        },
        &request_id,
        cancellation,
    )
    .map(|_| ())
}

fn service_request(
    request: &HelperRequest<'_>,
    request_id: &str,
    cancellation: &AtomicBool,
) -> Result<Option<String>, String> {
    let mut socket = connect()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|error| format!("failed to set the service read timeout: {error}"))?;
    socket
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| format!("failed to set the service write timeout: {error}"))?;
    serde_json::to_writer(&mut socket, request)
        .map_err(|error| format!("failed to send service request: {error}"))?;
    socket
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("failed to finish service request: {error}"))?;
    let mut response = Vec::new();
    let started = Instant::now();
    let mut cancel_sent = false;
    let mut buffer = [0; 1024];
    loop {
        if cancellation.load(Ordering::Relaxed) && !cancel_sent {
            send_cancel(request_id)?;
            cancel_sent = true;
        }
        if started.elapsed() >= SERVICE_RESPONSE_TIMEOUT {
            return Err("Molt service timed out".to_owned());
        }
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if response.len() + read > MAX_SERVICE_RESPONSE_BYTES {
                    return Err("Molt service response exceeded 128 KiB".to_owned());
                }
                response.extend_from_slice(&buffer[..read]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(format!("failed to read service response: {error}")),
        }
    }
    if cancel_sent {
        return Err(command::CANCELLED.to_owned());
    }
    parse_service_response(&response)
}

fn connect() -> Result<UnixStream, String> {
    UnixStream::connect(SOCKET_PATH).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "Molt service socket is unavailable. Start molt.socket or activate the Molt NixOS module."
                .to_owned()
        } else {
            format!("failed to connect to the Molt service: {error}")
        }
    })
}

fn send_cancel(request_id: &str) -> Result<(), String> {
    let mut socket = connect()?;
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| format!("failed to set cancellation read timeout: {error}"))?;
    serde_json::to_writer(&mut socket, &HelperRequest::Cancel { request_id })
        .map_err(|error| format!("failed to send cancellation request: {error}"))?;
    socket
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("failed to finish cancellation request: {error}"))?;
    let mut response = Vec::new();
    socket
        .take((MAX_SERVICE_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read cancellation response: {error}"))?;
    parse_service_response(&response).map(|_| ())
}

fn request_id() -> String {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}-{timestamp:x}-{sequence:x}", std::process::id())
}

fn ensure_profile_unchanged(report: &Report) -> Result<(), String> {
    let current = profile_target(report.target)?;
    if current == report.profile {
        Ok(())
    } else {
        Err(format!(
            "active profile changed from {} to {}; rebuild before switching",
            report.profile.display(),
            current.display()
        ))
    }
}

pub(crate) fn parse_service_response(response: &[u8]) -> Result<Option<String>, String> {
    if response.is_empty() {
        return Err("Molt activation service closed without a response".to_owned());
    }
    match serde_json::from_slice(response)
        .map_err(|error| format!("Molt activation service returned invalid JSON: {error}"))?
    {
        HelperResponse::Ok { output } => Ok(output),
        HelperResponse::Error { message } if message.trim().is_empty() => {
            Err("Molt activation service returned an empty error".to_owned())
        }
        HelperResponse::Error { message } => Err(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_service_responses() {
        assert_eq!(
            parse_service_response(br#"{"status":"ok"}"#),
            Ok(None)
        );
        assert_eq!(
            parse_service_response(br#"{"status":"ok","output":"impact"}"#),
            Ok(Some("impact".to_owned()))
        );
        assert_eq!(
            parse_service_response(b""),
            Err("Molt activation service closed without a response".to_owned())
        );
        assert!(parse_service_response(b"unexpected").is_err());
        assert!(parse_service_response(br#"{"status":"error","message":""}"#).is_err());
    }
}
