//! Shared memory operations for Wayland buffer management

use anyhow::Result;

/// Creates a shared memory file descriptor with the specified size.
///
/// Uses memfd_create with sealing to prevent resizing.
pub(super) fn create_shm_fd(size: usize) -> Result<i32> {
    use nix::fcntl::{FcntlArg, SealFlag};
    use nix::sys::memfd::{memfd_create, MFdFlags};
    use nix::unistd::ftruncate;
    use std::os::fd::IntoRawFd;

    let name = c"chomp-capture";
    let fd = memfd_create(
        name,
        MFdFlags::MFD_CLOEXEC | MFdFlags::MFD_ALLOW_SEALING,
    )?;

    ftruncate(&fd, size as i64)?;

    nix::fcntl::fcntl(
        &fd,
        FcntlArg::F_ADD_SEALS(
            SealFlag::F_SEAL_SHRINK | SealFlag::F_SEAL_GROW | SealFlag::F_SEAL_SEAL,
        ),
    )?;

    // Use into_raw_fd() to transfer ownership and prevent auto-close
    Ok(fd.into_raw_fd())
}

/// Reads data from a shared memory file descriptor into a buffer.
pub(super) fn read_shm_buffer(fd: i32, size: usize) -> Result<Vec<u8>> {
    use nix::unistd::{lseek, read, Whence};
    use std::os::fd::BorrowedFd;

    let mut buffer = vec![0u8; size];
    // SAFETY: The file descriptor `fd` is valid and owned by the caller.
    // BorrowedFd does not take ownership and the fd remains valid for the duration of this function.
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };

    lseek(borrowed_fd, 0, Whence::SeekSet)?;

    let mut total_read = 0;
    while total_read < size {
        match read(borrowed_fd, &mut buffer[total_read..]) {
            Ok(0) => break, // EOF
            Ok(n) => total_read += n,
            Err(e) => return Err(anyhow::anyhow!("Failed to read from shm fd: {}", e)),
        }
    }

    if total_read != size {
        anyhow::bail!("Incomplete read: got {} bytes, expected {}", total_read, size);
    }

    Ok(buffer)
}
