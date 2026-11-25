use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(super) struct OperationCancellation {
    pub(super) flag: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
    marker: PathBuf,
}

impl OperationCancellation {
    pub(super) fn start(request_id: &str, directory: &Path) -> Result<Self, String> {
        let marker = marker_path(request_id, directory)?;
        fs::create_dir_all(directory).map_err(|error| {
            format!(
                "failed to create cancellation directory {}: {error}",
                directory.display()
            )
        })?;
        let flag = Arc::new(AtomicBool::new(marker.exists()));
        let stop = Arc::new(AtomicBool::new(false));
        let watcher_flag = Arc::clone(&flag);
        let watcher_stop = Arc::clone(&stop);
        let watcher_marker = marker.clone();
        let watcher = thread::spawn(move || {
            while !watcher_stop.load(Ordering::Relaxed) {
                if watcher_marker.exists() {
                    watcher_flag.store(true, Ordering::Relaxed);
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
        });
        Ok(Self {
            flag,
            stop,
            watcher: Some(watcher),
            marker,
        })
    }
}

impl Drop for OperationCancellation {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
        let _ = fs::remove_file(&self.marker);
    }
}

pub(super) fn cancel(request_id: &str, directory: &Path) -> Result<(), String> {
    let marker = marker_path(request_id, directory)?;
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "failed to create cancellation directory {}: {error}",
            directory.display()
        )
    })?;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&marker)
        .map_err(|error| format!("failed to signal cancellation: {error}"))?;
    Ok(())
}

fn marker_path(request_id: &str, directory: &Path) -> Result<PathBuf, String> {
    if request_id.is_empty()
        || request_id.len() > 96
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
    {
        return Err("invalid service request ID".to_owned());
    }
    Ok(directory.join(request_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    static ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn cancellation_reaches_active_operation() {
        let root = std::env::temp_dir().join(format!(
            "molt-cancel-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        let operation = OperationCancellation::start("abc-123", &root).unwrap();
        cancel("abc-123", &root).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !operation.flag.load(Ordering::Relaxed) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(operation.flag.load(Ordering::Relaxed));
        drop(operation);
        assert!(!root.join("abc-123").exists());
        let _ = fs::remove_dir_all(root);
    }
}
