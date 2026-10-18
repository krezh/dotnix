use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub struct SingleInstanceLock {
    file: std::fs::File,
}

#[derive(Debug)]
pub enum LockError {
    AlreadyRunning,
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => f.write_str("another instance of slumber is already running"),
            Self::Io { path, source } => {
                write!(f, "cannot acquire lock {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyRunning => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl SingleInstanceLock {
    pub fn acquire() -> Result<Self, LockError> {
        let path = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("slumber.lock");
        Self::acquire_path(&path)
    }

    pub fn acquire_path(path: &Path) -> Result<Self, LockError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|source| LockError::Io {
                path: path.into(),
                source,
            })?;

        let fd = file.as_raw_fd();
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            let source = io::Error::last_os_error();
            return Err(if source.kind() == io::ErrorKind::WouldBlock {
                LockError::AlreadyRunning
            } else {
                LockError::Io {
                    path: path.into(),
                    source,
                }
            });
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
