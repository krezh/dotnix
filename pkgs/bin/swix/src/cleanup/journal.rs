use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use super::model::{CleanupGroup, CleanupItem, CleanupKind, parse_size};
use super::run_command;

const JOURNALCTL: &str = match option_env!("SWIX_JOURNALCTL") {
    Some(path) => path,
    None => "journalctl",
};

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let mut command = Command::new(JOURNALCTL);
    command.args(["--disk-usage", "--quiet"]);
    let output = match run_command(
        &mut command,
        "journal storage scan",
        cancellation,
        Duration::from_secs(30),
        64 * 1024,
    ) {
        Ok(output) => output,
        Err(error) => return CleanupGroup::unavailable(CleanupKind::Journals, error),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let reclaimable = text
        .split("take up ")
        .nth(1)
        .and_then(parse_size)
        .unwrap_or(0);
    CleanupGroup {
        kind: CleanupKind::Journals,
        available: true,
        reclaimable,
        items: vec![CleanupItem {
            label: "Archived and active journals".to_owned(),
            detail: text.trim().to_owned(),
            bytes: Some(reclaimable),
            path: None,
        }],
        note: Some("The active journal is rotated before archived journals are removed".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_journal_disk_usage() {
        let text = "Archived and active journals take up 3.5G in the file system.";
        assert_eq!(
            text.split("take up ").nth(1).and_then(parse_size),
            Some(3_758_096_384)
        );
    }
}
