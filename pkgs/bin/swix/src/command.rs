use std::collections::VecDeque;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub const CANCELLED: &str = "operation cancelled";

const TERMINATION_GRACE: Duration = Duration::from_millis(500);
const SIGTERM: i32 = 15;
const SIGKILL: i32 = 9;

#[derive(Clone, Copy)]
pub struct OutputLimits {
    pub stdout: usize,
    pub stderr: usize,
}

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

pub fn run(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    limits: OutputLimits,
    mut stderr_line: impl FnMut(&str),
    error_detail: impl FnOnce(&[u8]) -> String,
) -> Result<Output, String> {
    command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to run {name}: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or("failed to capture command stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("failed to capture command stderr")?;
    let stdout_limit = limits.stdout;
    let stdout_reader = thread::spawn(move || read_bounded(&mut stdout, stdout_limit));
    let (line_sender, line_receiver) = mpsc::channel();
    let stderr_limit = limits.stderr;
    let stderr_reader = thread::spawn(move || read_stderr(stderr, line_sender, stderr_limit));

    let started = Instant::now();
    let status = loop {
        while let Ok(line) = line_receiver.try_recv() {
            stderr_line(&line);
        }
        if cancellation.load(Ordering::Relaxed) {
            terminate(&mut child);
            join_discard(stdout_reader, stderr_reader);
            return Err(CANCELLED.to_owned());
        }
        if started.elapsed() >= timeout {
            terminate(&mut child);
            join_discard(stdout_reader, stderr_reader);
            return Err(format!(
                "{name} timed out after {} seconds",
                timeout.as_secs()
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                terminate(&mut child);
                join_discard(stdout_reader, stderr_reader);
                return Err(format!("failed while waiting for {name}: {error}"));
            }
        }
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| format!("failed to join {name} stdout reader"))?
        .map_err(|error| format!("failed to read {name} stdout: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| format!("failed to join {name} stderr reader"))?
        .map_err(|error| format!("failed to read {name} stderr: {error}"))?;
    while let Ok(line) = line_receiver.try_recv() {
        stderr_line(&line);
    }
    let BoundedOutput {
        bytes: stdout,
        exceeded,
    } = stdout;
    let output = Output {
        status,
        stdout,
        stderr,
    };
    if !output.status.success() {
        Err(format!("{name} failed:\n{}", error_detail(&output.stderr)))
    } else if exceeded {
        Err(format!(
            "{name} stdout exceeded the {}-byte limit",
            limits.stdout
        ))
    } else {
        Ok(output)
    }
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn read_bounded(reader: &mut impl Read, limit: usize) -> std::io::Result<BoundedOutput> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0; 8192];
    let mut exceeded = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        exceeded |= read > remaining;
    }
    Ok(BoundedOutput { bytes, exceeded })
}

fn read_stderr(
    mut reader: impl Read,
    line_sender: mpsc::Sender<String>,
    limit: usize,
) -> std::io::Result<Vec<u8>> {
    let mut tail = VecDeque::with_capacity(limit.min(64 * 1024));
    let mut line = Vec::with_capacity(limit.min(8192));
    let line_limit = limit.min(1024 * 1024);
    let mut buffer = [0; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        push_tail(&mut tail, &buffer[..read], limit);
        let mut start = 0;
        while start < read {
            let end = buffer[start..read]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(read, |position| start + position);
            let remaining = line_limit.saturating_sub(line.len());
            line.extend_from_slice(&buffer[start..end.min(start + remaining)]);
            if end == read {
                break;
            }
            let text = String::from_utf8_lossy(&line).trim_end().to_owned();
            let _ = line_sender.send(text);
            line.clear();
            start = end + 1;
        }
    }
    if !line.is_empty() {
        let text = String::from_utf8_lossy(&line).trim_end().to_owned();
        let _ = line_sender.send(text);
    }
    Ok(tail.into_iter().collect())
}

fn push_tail(tail: &mut VecDeque<u8>, bytes: &[u8], limit: usize) {
    if bytes.len() >= limit {
        tail.clear();
        tail.extend(bytes[bytes.len() - limit..].iter().copied());
        return;
    }
    let excess = tail.len().saturating_add(bytes.len()).saturating_sub(limit);
    tail.drain(..excess);
    tail.extend(bytes.iter().copied());
}

fn terminate(child: &mut Child) {
    let Ok(process_group) = i32::try_from(child.id()) else {
        let _ = child.kill();
        let _ = child.wait();
        return;
    };

    signal_group(process_group, SIGTERM);
    let deadline = Instant::now() + TERMINATION_GRACE;
    let mut reaped = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                reaped = true;
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => break,
        }
    }
    signal_group(process_group, SIGKILL);
    if !reaped {
        let _ = child.wait();
    }
}

fn signal_group(process_group: i32, signal: i32) {
    // Negative PIDs address the process group created by `process_group(0)`.
    unsafe {
        kill(-process_group, signal);
    }
}

fn join_discard(
    stdout: thread::JoinHandle<std::io::Result<BoundedOutput>>,
    stderr: thread::JoinHandle<std::io::Result<Vec<u8>>>,
) {
    let _ = stdout.join();
    let _ = stderr.join();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_output_and_stderr_lines() {
        let cancellation = AtomicBool::new(false);
        let mut lines = Vec::new();
        let output = run(
            Command::new("sh").args(["-c", "printf output; printf 'one\\ntwo\\n' >&2"]),
            "fixture",
            &cancellation,
            Duration::from_secs(2),
            OutputLimits {
                stdout: 1024,
                stderr: 1024,
            },
            |line| lines.push(line.to_owned()),
            |stderr| String::from_utf8_lossy(stderr).trim().to_owned(),
        )
        .unwrap();
        assert_eq!(output.stdout, b"output");
        assert_eq!(lines, ["one", "two"]);
    }

    #[test]
    fn cancellation_terminates_descendants() {
        let cancellation = AtomicBool::new(true);
        let started = Instant::now();
        let error = run(
            Command::new("sh").args(["-c", "sleep 30 & wait"]),
            "fixture",
            &cancellation,
            Duration::from_secs(2),
            OutputLimits {
                stdout: 1024,
                stderr: 1024,
            },
            |_| {},
            |_| String::new(),
        )
        .unwrap_err();
        assert_eq!(error, CANCELLED);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn bounds_stdout_and_keeps_the_stderr_tail() {
        let cancellation = AtomicBool::new(false);
        let error = run(
            Command::new("sh").args(["-c", "printf 12345; printf 'discard-this-tail' >&2; exit 1"]),
            "fixture",
            &cancellation,
            Duration::from_secs(2),
            OutputLimits {
                stdout: 4,
                stderr: 4,
            },
            |_| {},
            |stderr| String::from_utf8_lossy(stderr).to_string(),
        )
        .unwrap_err();
        assert!(error.ends_with("tail"));

        let error = run(
            Command::new("sh").args(["-c", "printf 12345"]),
            "fixture",
            &cancellation,
            Duration::from_secs(2),
            OutputLimits {
                stdout: 4,
                stderr: 4,
            },
            |_| {},
            |_| String::new(),
        )
        .unwrap_err();
        assert!(error.contains("stdout exceeded the 4-byte limit"));
    }
}
