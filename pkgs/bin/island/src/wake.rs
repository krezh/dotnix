//! Waking a worker that sleeps on file descriptors: whatever asks it for something holds a
//! [`Waker`], and the worker sleeps on its [`Woken`] end beside its own, such as a Wayland
//! connection in [`wait`].

use std::{
    io::{self, ErrorKind, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, RawFd},
        unix::net::UnixStream,
    },
    sync::Arc,
    time::Instant,
};

use wayland_backend::client::{ReadEventsGuard, WaylandError};

#[derive(Clone)]
pub struct Waker(Arc<UnixStream>);

impl Waker {
    pub fn wake(&self) {
        let _ = (&*self.0).write(&[1]);
    }
}

pub struct Woken(UnixStream);

impl Woken {
    /// Takes in every wake so far, so the next sleep lasts until another.
    pub fn drain(&self) {
        let mut buffer = [0; 64];
        loop {
            match (&self.0).read(&mut buffer) {
                Ok(read) if read > 0 => {}
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                _ => break,
            }
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

impl AsRawFd for Woken {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

pub fn pair() -> io::Result<(Waker, Woken)> {
    let (reader, writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    writer.set_nonblocking(true)?;
    Ok((Waker(Arc::new(writer)), Woken(reader)))
}

/// Sleeps until the compositor sends something, `wake` can be read or `until` comes, then reads
/// what the compositor sent. Whether `wake` can be read; it is left for its owner to drain.
pub fn wait(read: ReadEventsGuard, wake: RawFd, until: Option<Instant>) -> Result<bool, String> {
    let timeout = until.map_or(-1, |until| {
        until
            .saturating_duration_since(Instant::now())
            .as_nanos()
            .div_ceil(1_000_000)
            .min(i32::MAX as u128) as i32
    });
    let mut fds = [
        libc::pollfd {
            fd: read.connection_fd().as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: wake,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    // Both descriptors live past the call.
    if unsafe { libc::poll(fds.as_mut_ptr(), 2, timeout) } < 0 {
        let error = io::Error::last_os_error();
        return match error.kind() {
            ErrorKind::Interrupted => Ok(false),
            _ => Err(error.to_string()),
        };
    }
    if fds[0].revents & libc::POLLIN != 0 {
        match read.read() {
            Ok(_) => {}
            Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.to_string()),
        }
    } else if fds[0].revents & (libc::POLLHUP | libc::POLLERR) != 0 {
        return Err("the compositor closed the connection".to_owned());
    }
    Ok(fds[1].revents & libc::POLLIN != 0)
}
