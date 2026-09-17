//! Shared memory operations for Wayland buffer management

use anyhow::Result;
use std::os::fd::OwnedFd;

use crate::capture::buffer::MappedPixels;

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

/// Maps a capture's shared memory for reading.
///
/// The compositor has already written the pixels into these pages, so mapping
/// them hands the capture straight over: no second full-screen allocation to
/// zero, and no copy through `read`.
pub(super) fn map_shm_buffer(fd: &OwnedFd, size: usize) -> Result<MappedPixels> {
    MappedPixels::map(fd, size)
}
