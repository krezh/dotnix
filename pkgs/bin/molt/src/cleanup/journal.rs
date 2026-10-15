use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::model::{CleanupGroup, CleanupItem, CleanupKind};

/// Persistent and volatile journald storage.
const JOURNAL_ROOTS: [&str; 2] = ["/var/log/journal", "/run/log/journal"];

pub(super) fn scan() -> CleanupGroup {
    let mut usage = JournalUsage::default();
    for root in JOURNAL_ROOTS {
        if let Err(error) = usage.add_root(Path::new(root)) {
            return CleanupGroup::unavailable(
                CleanupKind::Journals,
                format!("could not read {root}: {error}"),
            );
        }
    }
    CleanupGroup {
        kind: CleanupKind::Journals,
        available: true,
        reclaimable: usage.bytes,
        items: vec![CleanupItem {
            label: "Archived and active journals".to_owned(),
            detail: format!("{} journal files", usage.files),
            bytes: Some(usage.bytes),
            path: None,
        }],
        note: Some("The active journal is rotated before archived journals are removed".to_owned()),
        volumes: Vec::new(),
    }
}

/// What `journalctl --disk-usage` reports: the allocated size of every journal file, which
/// journald keeps one directory below each root (per machine ID and per namespace).
#[derive(Default)]
struct JournalUsage {
    bytes: u64,
    files: usize,
}

impl JournalUsage {
    fn add_root(&mut self, root: &Path) -> std::io::Result<()> {
        let directories = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        for directory in directories {
            let directory = directory?;
            if !directory.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(directory.path())? {
                let file = file?;
                if !is_journal_file(&file.file_name().to_string_lossy()) {
                    continue;
                }
                let metadata = file.metadata()?;
                if metadata.is_file() {
                    self.bytes = self.bytes.saturating_add(metadata.blocks().saturating_mul(512));
                    self.files += 1;
                }
            }
        }
        Ok(())
    }
}

/// Active (`.journal`) and dirty archived (`.journal~`) files; journald ignores anything else.
fn is_journal_file(name: &str) -> bool {
    name.ends_with(".journal") || name.ends_with(".journal~")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn sums_journal_files_one_level_below_the_root() {
        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "molt-journal-test-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let machine = root.join("0973a0b630bb47618257da19580125ad");
        fs::create_dir_all(&machine).unwrap();
        let journals = [
            machine.join("system.journal"),
            machine.join("system@0006-0001.journal~"),
            machine.join("user-1000@abc-0001-0002.journal"),
        ];
        for journal in &journals {
            fs::write(journal, vec![1_u8; 8192]).unwrap();
        }
        fs::write(machine.join("notes.txt"), vec![1_u8; 8192]).unwrap();
        fs::write(root.join("stray.journal"), vec![1_u8; 8192]).unwrap();
        let expected = journals
            .iter()
            .map(|journal| fs::metadata(journal).unwrap().blocks() * 512)
            .sum::<u64>();

        let mut usage = JournalUsage::default();
        usage.add_root(&root).unwrap();
        usage.add_root(&root.join("missing")).unwrap();

        assert_eq!(usage.files, 3);
        assert_eq!(usage.bytes, expected);
        fs::remove_dir_all(&root).unwrap();
    }
}
