use std::{
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

pub struct Output {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn output(program: &str, args: &[&str], timeout: Duration) -> Result<Output, String> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not launch {program}: {error}"))?;
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    match receiver.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(Output {
            success: output.status.success(),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        }),
        Ok(Err(error)) => Err(format!("{program} failed: {error}")),
        Err(_) => {
            if let Ok(pid) = i32::try_from(pid) {
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
            Err(format!("{program} timed out"))
        }
    }
}
