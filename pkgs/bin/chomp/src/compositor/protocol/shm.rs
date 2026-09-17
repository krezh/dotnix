//! Shared memory operations for Wayland buffer management

use anyhow::Result;
use std::os::fd::OwnedFd;

/// Creates a shared memory file descriptor with the specified size.
///
/// Uses memfd_create with sealing to prevent resizing. The descriptor owns the
/// capture memory, so dropping it releases the buffer.
pub(super) fn create_shm_fd(size: usize) -> Result<OwnedFd> {
    use nix::fcntl::{FcntlArg, SealFlag};
    use nix::sys::memfd::{MFdFlags, memfd_create};
    use nix::unistd::ftruncate;

    let name = c"chomp-capture";
    let fd = memfd_create(name, MFdFlags::MFD_CLOEXEC | MFdFlags::MFD_ALLOW_SEALING)?;

    ftruncate(&fd, size as i64)?;

    nix::fcntl::fcntl(
        &fd,
        FcntlArg::F_ADD_SEALS(
            SealFlag::F_SEAL_SHRINK | SealFlag::F_SEAL_GROW | SealFlag::F_SEAL_SEAL,
        ),
    )?;

    Ok(fd)
}

/// Reads data from a shared memory file descriptor into a buffer.
pub(super) fn read_shm_buffer(fd: &OwnedFd, size: usize) -> Result<Vec<u8>> {
    use nix::unistd::{Whence, lseek, read};

    let mut buffer = vec![0u8; size];

    lseek(fd, 0, Whence::SeekSet)?;

    let mut total_read = 0;
    while total_read < size {
        match read(fd, &mut buffer[total_read..]) {
            Ok(0) => break, // EOF
            Ok(n) => total_read += n,
            Err(e) => return Err(anyhow::anyhow!("Failed to read from shm fd: {}", e)),
        }
    }

    if total_read != size {
        anyhow::bail!(
            "Incomplete read: got {} bytes, expected {}",
            total_read,
            size
        );
    }

    Ok(buffer)
}
