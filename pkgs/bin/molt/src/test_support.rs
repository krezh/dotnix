//! Fixtures shared by the test suites of both binaries.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes an executable `/bin/sh` script through a child `sh`, so the calling process never holds
/// a write descriptor to it. Otherwise a parallel test that forks in that window inherits the
/// descriptor, and executing the script fails with ETXTBSY ("Text file busy").
pub fn write_executable_script(path: &Path, body: &str) {
    let mut writer = Command::new("sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to start the script writer");
    writer
        .stdin
        .take()
        .expect("script writer stdin")
        .write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
        .expect("failed to write the script");
    assert!(
        writer
            .wait()
            .expect("script writer did not finish")
            .success(),
        "failed to write {}",
        path.display()
    );
}
