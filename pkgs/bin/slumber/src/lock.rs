use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub struct SingleInstanceLock {
    file: std::fs::File,
}

impl SingleInstanceLock {
    pub fn acquire() -> Result<Self, &'static str> {
        let path = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("slumber.lock");
        Self::acquire_path(&path)
    }

    pub fn acquire_path(path: &Path) -> Result<Self, &'static str> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| "Failed to open lock file")?;

        let fd = file.as_raw_fd();
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            return Err("Another instance of slumber is already running");
        }

        Ok(Self { file })
    }
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();
        unsafe {
            libc::flock(fd, libc::LOCK_UN);
        }
    }
}
