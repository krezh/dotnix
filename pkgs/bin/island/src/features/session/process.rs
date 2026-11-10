//! Processes as `/proc` and pidfds see them, so a process is still the one found even once its
//! number is reused.

use std::{
    collections::HashSet,
    fs, io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    time::{Duration, Instant},
};

/// A process by its number and when it started, which no later one with that number shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Identity {
    pub pid: i32,
    pub started: u64,
}

pub struct Process {
    pub identity: Identity,
    fd: OwnedFd,
}

pub struct Stat {
    pub identity: Identity,
    pub parent: i32,
    pub name: String,
    pub dead: bool,
}

impl Process {
    /// `None` if it has already gone.
    pub fn open(pid: i32) -> Result<Option<Self>, String> {
        let Some(before) = stat(pid)? else {
            return Ok(None);
        };
        if before.dead {
            return Ok(None);
        }
        // A new descriptor, which `OwnedFd` then owns.
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if raw < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(None);
            }
            return Err(format!("cannot follow process {pid}: {error}"));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw as libc::c_int) };
        let Some(after) = stat(pid)? else {
            return Ok(None);
        };
        if after.dead || before.identity != after.identity {
            return Err(format!("process {pid} changed while it was looked at"));
        }
        Ok(Some(Self {
            identity: before.identity,
            fd,
        }))
    }

    pub fn fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn alive(&self) -> Result<bool, String> {
        Ok(!readable(self.fd(), Duration::ZERO)?)
    }

    pub fn signal(&self, signal: i32) -> Result<(), String> {
        // The pidfd stays with the process it was opened on, whatever its number is given to next.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(format!(
                    "cannot signal process {}: {error}",
                    self.identity.pid
                ));
            }
        }
        Ok(())
    }
}

/// Whether `fd` becomes readable within `timeout`.
pub fn readable(fd: RawFd, timeout: Duration) -> Result<bool, String> {
    let ready = wait(&[fd], Some(Instant::now() + timeout))?;
    Ok(!ready.is_empty())
}

/// The descriptors among `fds` that became readable by `until`, or by any time without it.
pub fn wait(fds: &[RawFd], until: Option<Instant>) -> Result<Vec<RawFd>, String> {
    let mut polled: Vec<libc::pollfd> = fds
        .iter()
        .map(|&fd| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    loop {
        let timeout = until.map_or(-1, |until| {
            until
                .saturating_duration_since(Instant::now())
                .as_nanos()
                .div_ceil(1_000_000)
                .min(i32::MAX as u128) as i32
        });
        // Every descriptor lives past the call.
        let result =
            unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as libc::nfds_t, timeout) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(format!("cannot wait: {error}"));
        }
        if polled.iter().any(|fd| fd.revents & libc::POLLNVAL != 0) {
            return Err("a descriptor waited on became invalid".to_owned());
        }
        return Ok(polled
            .iter()
            .filter(|fd| fd.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0)
            .map(|fd| fd.fd)
            .collect());
    }
}

pub fn stat(pid: i32) -> Result<Option<Stat>, String> {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(line) => parse_stat(pid, &line).map(Some),
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ESRCH)) => Ok(None),
        Err(error) => Err(format!("cannot look at process {pid}: {error}")),
    }
}

pub fn parse_stat(pid: i32, line: &str) -> Result<Stat, String> {
    let invalid = || format!("cannot read /proc/{pid}/stat");
    let start = line.find('(').ok_or_else(invalid)?;
    let end = line
        .rfind(')')
        .filter(|end| *end > start)
        .ok_or_else(invalid)?;
    let mut fields = line[end + 1..].split_whitespace();
    let state = fields.next().ok_or_else(invalid)?;
    let parent = fields
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    let started = fields
        .nth(17)
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    Ok(Stat {
        identity: Identity { pid, started },
        parent,
        name: line[start + 1..end].to_owned(),
        dead: matches!(state, "Z" | "X" | "x"),
    })
}

/// This process and every one above it.
pub fn ancestors() -> Result<HashSet<i32>, String> {
    let mut found = HashSet::new();
    let mut pid = std::process::id() as i32;
    while pid > 0 && found.insert(pid) {
        let stat =
            stat(pid)?.ok_or_else(|| format!("process {pid} went while it was looked at"))?;
        pid = stat.parent;
    }
    Ok(found)
}

/// Every living process `parent` started.
pub fn children_of(parent: i32) -> Result<Vec<Stat>, String> {
    let entries =
        fs::read_dir("/proc").map_err(|error| format!("cannot list processes: {error}"))?;
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot list processes: {error}"))?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        if let Some(stat) = stat(pid)?
            && stat.parent == parent
            && !stat.dead
        {
            children.push(stat);
        }
    }
    Ok(children)
}

/// Wakes a thread waiting with [`wait`] from another.
pub struct Bell(OwnedFd);

impl Bell {
    pub fn new() -> Result<Self, String> {
        // A new descriptor, which `OwnedFd` then owns.
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        if fd < 0 {
            return Err("cannot make an eventfd".to_owned());
        }
        Ok(Self(unsafe { OwnedFd::from_raw_fd(fd) }))
    }

    pub fn fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }

    pub fn ring(&self) {
        let one = 1_u64.to_ne_bytes();
        // Full only if it is already ringing.
        unsafe { libc::write(self.fd(), one.as_ptr().cast(), one.len()) };
    }

    pub fn quiet(&self) {
        let mut count = [0_u8; 8];
        // Nothing to read is the same as having read it.
        unsafe { libc::read(self.fd(), count.as_mut_ptr().cast(), count.len()) };
    }
}

#[cfg(test)]
#[path = "process_test.rs"]
mod tests;
