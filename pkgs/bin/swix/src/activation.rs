use std::io::Read;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::build::{Target, active_profile};
use crate::nix::run;
use crate::report::Report;
use swix::protocol::HelperRequest;
const SOCKET_PATH: &str = "/run/swix.sock";
const HOME_MANAGER_ACTIVATE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SERVICE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(36 * 60);
const MAX_SERVICE_RESPONSE_BYTES: usize = 64 * 1024;

pub(crate) fn activate(report: &Report, cancellation: &AtomicBool) -> Result<(), String> {
    match report.target {
        Target::HomeManager => {
            ensure_profile_unchanged(report)?;
            run(
                &mut Command::new(report.output.join("activate")),
                "Home Manager activation",
                cancellation,
                HOME_MANAGER_ACTIVATE_TIMEOUT,
                1024 * 1024,
            )?;
        }
        Target::NixOs => service_request(
            &HelperRequest::Activate {
                baseline: &report.baseline,
                output: &report.output,
            },
            cancellation,
        )?,
    }
    Ok(())
}

pub(crate) fn clean_system(
    nix: bool,
    journals: bool,
    cancellation: &AtomicBool,
) -> Result<(), String> {
    service_request(&HelperRequest::Cleanup { nix, journals }, cancellation)
}

fn service_request(request: &HelperRequest<'_>, cancellation: &AtomicBool) -> Result<(), String> {
    let mut socket = UnixStream::connect(SOCKET_PATH).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "Swix service socket is unavailable. Start swix.socket or activate the Swix NixOS module."
                .to_owned()
        } else {
            format!("failed to connect to the Swix service: {error}")
        }
    })?;
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
    let mut buffer = [0; 1024];
    loop {
        if cancellation.load(Ordering::Relaxed) {
            return Err(
                "operation cancelled; the privileged action may still be running".to_owned(),
            );
        }
        if started.elapsed() >= SERVICE_RESPONSE_TIMEOUT {
            return Err("Swix service timed out; the operation may still be running".to_owned());
        }
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if response.len() + read > MAX_SERVICE_RESPONSE_BYTES {
                    return Err("Swix service response exceeded 64 KiB".to_owned());
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
    parse_service_response(&response)
}

fn ensure_profile_unchanged(report: &Report) -> Result<(), String> {
    let current = active_profile(report.target)?;
    if current == report.baseline {
        Ok(())
    } else {
        Err(format!(
            "active profile changed from {} to {}; rebuild before switching",
            report.baseline.display(),
            current.display()
        ))
    }
}

pub(crate) fn parse_service_response(response: &[u8]) -> Result<(), String> {
    let response = std::str::from_utf8(response)
        .map_err(|_| "Swix activation service returned invalid UTF-8".to_owned())?;
    match response {
        "OK" | "OK\n" => Ok(()),
        "" => Err("Swix activation service closed without a response".to_owned()),
        _ => {
            let detail = response
                .strip_prefix("ERROR\n")
                .ok_or_else(|| "Swix activation service returned an invalid response".to_owned())?
                .trim();
            if detail.is_empty() {
                Err("Swix activation service returned an empty error".to_owned())
            } else {
                Err(detail.to_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_service_responses() {
        assert_eq!(parse_service_response(b"OK\n"), Ok(()));
        assert_eq!(
            parse_service_response(b""),
            Err("Swix activation service closed without a response".to_owned())
        );
        assert!(parse_service_response(b"unexpected").is_err());
        assert!(parse_service_response(b"ERROR\n").is_err());
    }
}
