//! The kernel's device events, heard over netlink the way udev hears them: a backlight's level
//! changing, wherever the change came from, and monitors being plugged in or out.

use std::{
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

/// The kernel's own multicast group, as opposed to udev's rebroadcast.
const KERNEL: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Uevent {
    Backlight,
    Hotplug,
}

/// A message is `action@devpath` and then `KEY=value` fields, each ending in a NUL.
pub(super) fn parse(message: &[u8]) -> Option<Uevent> {
    let (mut action, mut subsystem, mut hotplug) = (None, None, false);
    for field in message.split(|&byte| byte == 0) {
        if let Some(value) = field.strip_prefix(b"ACTION=") {
            action = Some(value);
        } else if let Some(value) = field.strip_prefix(b"SUBSYSTEM=") {
            subsystem = Some(value);
        } else if field == b"HOTPLUG=1" {
            hotplug = true;
        }
    }
    match (action?, subsystem?) {
        (b"change", b"backlight") => Some(Uevent::Backlight),
        (b"change", b"drm") if hotplug => Some(Uevent::Hotplug),
        _ => None,
    }
}

pub(super) struct Socket(OwnedFd);

impl Socket {
    pub(super) fn open() -> io::Result<Self> {
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                libc::NETLINK_KOBJECT_UEVENT,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut address: libc::sockaddr_nl = unsafe { mem::zeroed() };
        address.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        address.nl_groups = KERNEL;
        let bound = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&raw const address).cast(),
                mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };
        if bound < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(fd))
    }

    /// Waits for the next event island cares about.
    pub(super) fn next(&self) -> io::Result<Uevent> {
        let mut buffer = [0u8; 8192];
        loop {
            let read = unsafe {
                libc::recv(
                    self.0.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    0,
                )
            };
            if read < 0 {
                let error = io::Error::last_os_error();
                match error.kind() {
                    io::ErrorKind::Interrupted => continue,
                    // Some events were dropped; later ones still arrive.
                    _ if error.raw_os_error() == Some(libc::ENOBUFS) => continue,
                    _ => return Err(error),
                }
            }
            if let Some(event) = parse(&buffer[..read as usize]) {
                return Ok(event);
            }
        }
    }
}

#[cfg(test)]
#[path = "uevent_test.rs"]
mod tests;
