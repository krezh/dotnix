use crate::config::Options;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const IPC_TIMEOUT: Duration = Duration::from_secs(1);
const CLOSE_INTERVAL: Duration = Duration::from_secs(5);
const MAX_REPLY: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub address: String,
    pub class: String,
    pub title: String,
    pub pid: i32,
    window: bool,
    identity: Option<Identity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Identity {
    pid: i32,
    started: u64,
}

struct Process {
    identity: Identity,
    fd: OwnedFd,
}

struct ProcStat {
    identity: Identity,
    parent: i32,
    name: String,
    dead: bool,
}

#[derive(Deserialize)]
struct Client {
    address: String,
    class: String,
    title: String,
    pid: i32,
}

#[derive(Deserialize)]
struct Layer {
    address: String,
    namespace: String,
    pid: i32,
}

#[derive(Deserialize)]
struct MonitorLayers {
    levels: HashMap<String, Vec<Layer>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    #[serde(default)]
    config_provider: Option<String>,
}

pub struct Session {
    socket: PathBuf,
    apps: Vec<App>,
    processes: HashMap<Identity, Process>,
    compositor: Process,
    dry_run: bool,
    no_exit: bool,
    verbose: bool,
    lua: bool,
    begun: bool,
    forced: bool,
    finished: bool,
    failure: Option<String>,
    last_close: Instant,
    termed: HashSet<Identity>,
}

impl Session {
    pub fn connect(options: &Options) -> Result<Self, String> {
        let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| {
            "Dusk requires a Hyprland session (HYPRLAND_INSTANCE_SIGNATURE is missing)".to_string()
        })?;
        if signature.is_empty() || signature.contains('/') || signature == "." || signature == ".."
        {
            return Err("Invalid HYPRLAND_INSTANCE_SIGNATURE".into());
        }
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })));
        let directory = runtime.join("hypr").join(signature);
        let lock = fs::read_to_string(directory.join("hyprland.lock"))
            .map_err(|error| format!("Cannot read hyprland.lock: {error}"))?;
        let compositor_pid = parse_lock(&lock)?;
        let compositor = Process::open(compositor_pid)?
            .ok_or_else(|| "The Hyprland process is no longer running".to_string())?;
        let mut excluded = ancestors()?;
        excluded.insert(compositor_pid);
        let socket = directory.join(".socket.sock");
        let status: Status = json_request(&socket, "j/status")?;
        let lua = match status.config_provider.as_deref() {
            Some("lua") => true,
            Some("hyprlang") | None => false,
            Some(provider) => {
                return Err(format!("Unsupported Hyprland config provider: {provider}"));
            }
        };
        let clients = read_clients(&socket)?;
        let layers: HashMap<String, MonitorLayers> = json_request(&socket, "j/layers")?;
        let children = children_of(compositor_pid)?;
        let mut apps = Vec::new();
        for client in &clients {
            if !excluded.contains(&client.pid) {
                apps.push(App {
                    address: client.address.clone(),
                    class: client.class.clone(),
                    title: client.title.clone(),
                    pid: client.pid,
                    window: true,
                    identity: None,
                });
            }
        }
        let window_pids: HashSet<i32> = clients
            .iter()
            .filter(|client| client.pid > 0)
            .map(|client| client.pid)
            .collect();
        let mut fallback_pids = HashSet::new();
        for monitor in layers.into_values() {
            for layer in monitor.levels.into_values().flatten() {
                validate_address(&layer.address)?;
                if layer.pid <= 0 {
                    return Err(format!(
                        "Layer {} has no usable process ID",
                        layer.namespace
                    ));
                }
                if !excluded.contains(&layer.pid)
                    && !window_pids.contains(&layer.pid)
                    && fallback_pids.insert(layer.pid)
                {
                    apps.push(App {
                        address: layer.address,
                        class: layer.namespace,
                        title: String::new(),
                        pid: layer.pid,
                        window: false,
                        identity: None,
                    });
                }
            }
        }
        for child in children {
            let pid = child.identity.pid;
            if child.name != "Xwayland"
                && !excluded.contains(&pid)
                && !window_pids.contains(&pid)
                && fallback_pids.insert(pid)
            {
                apps.push(App {
                    address: String::new(),
                    class: child.name,
                    title: String::new(),
                    pid,
                    window: false,
                    identity: None,
                });
            }
        }
        let mut processes = HashMap::new();
        let mut identities = HashMap::new();
        for app in &mut apps {
            if app.pid <= 0 {
                continue;
            }
            let identity = if let Some(identity) = identities.get(&app.pid) {
                *identity
            } else {
                let process = Process::open(app.pid)?;
                let identity = process.as_ref().map(|process| process.identity);
                if let Some(process) = process {
                    if process_name(app.pid)?.as_deref() == Some("Xwayland") {
                        excluded.insert(app.pid);
                    }
                    processes.insert(process.identity, process);
                }
                identities.insert(app.pid, identity);
                identity
            };
            app.identity = identity;
        }
        apps.retain(|app| !excluded.contains(&app.pid) && (app.window || app.identity.is_some()));
        apps.sort_by(|left, right| {
            left.class
                .cmp(&right.class)
                .then(left.title.cmp(&right.title))
                .then(left.pid.cmp(&right.pid))
        });
        if !compositor.alive()? {
            return Err("Hyprland exited during application discovery".into());
        }
        Ok(Self {
            socket,
            apps,
            processes,
            compositor,
            dry_run: options.dry_run,
            no_exit: options.no_exit,
            verbose: options.verbose,
            lua,
            begun: false,
            forced: false,
            finished: false,
            failure: None,
            last_close: Instant::now(),
            termed: HashSet::new(),
        })
    }

    pub fn apps(&self) -> &[App] {
        &self.apps
    }

    pub fn begin(&mut self) -> Result<(), String> {
        self.healthy()?;
        if self.begun {
            return Ok(());
        }
        self.begun = true;
        let result = self.update(true);
        self.record(result)
    }

    pub fn poll(&mut self) -> Result<(), String> {
        self.healthy()?;
        if !self.begun {
            return Err("Session exit has not begun".into());
        }
        let result = self.update(self.last_close.elapsed() >= CLOSE_INTERVAL);
        self.record(result)
    }

    pub fn force(&mut self) -> Result<(), String> {
        self.healthy()?;
        if !self.begun {
            return Err("Session exit has not begun".into());
        }
        if self.dry_run {
            self.forced = true;
            return Ok(());
        }
        let result = self.force_inner();
        self.record(result)
    }

    fn force_inner(&mut self) -> Result<(), String> {
        let clients = read_clients(&self.socket)?;
        for app in &self.apps {
            if app.window && app.identity.is_none() && window_present(app, &clients) {
                return Err(format!(
                    "Cannot safely force quit {}: no process identity",
                    app.class
                ));
            }
        }
        let mut signaled = HashSet::new();
        for app in &self.apps {
            if let Some(identity) = app.identity
                && signaled.insert(identity)
            {
                self.signal(identity, libc::SIGKILL)?;
            }
        }
        self.forced = true;
        Ok(())
    }

    pub fn finish(&mut self, options: &Options) -> Result<(), String> {
        if self.dry_run || options.dry_run || self.no_exit || options.no_exit {
            return Ok(());
        }
        self.healthy()?;
        if !self.begun || (!self.forced && !self.apps.is_empty()) {
            return Err("Refusing to exit Hyprland before application shutdown completes".into());
        }
        if self.finished {
            return Err("Session exit completion has already been requested".into());
        }
        self.finished = true;
        let result = self.finish_inner(options);
        self.record(result)
    }

    fn finish_inner(&self, options: &Options) -> Result<(), String> {
        if !self.compositor.alive()? {
            return Err("Hyprland exited before session exit could be confirmed".into());
        }
        self.dispatch(if self.lua {
            "/dispatch hl.dsp.exit()"
        } else {
            "/dispatch exit"
        })?;
        if !self.compositor.wait(Duration::from_secs(5))? {
            return Err("Hyprland did not exit within five seconds; completion command and VT switch were not run".into());
        }
        if let Some(vt) = options.vt.filter(|vt| *vt > 0) {
            let status = Command::new("sudo")
                .args(["-n", "chvt", &vt.to_string()])
                .status()
                .map_err(|error| format!("Cannot switch to VT {vt}: {error}"))?;
            if !status.success() {
                return Err(format!("Switching to VT {vt} failed: {status}"));
            }
        }
        if let Some(command) = &options.command {
            Command::new("/bin/sh")
                .args(["-c", command])
                .spawn()
                .map_err(|error| format!("Cannot start completion command: {error}"))?;
        }
        Ok(())
    }

    fn healthy(&self) -> Result<(), String> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn record(&mut self, result: Result<(), String>) -> Result<(), String> {
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }

    fn update(&mut self, repeat: bool) -> Result<(), String> {
        let clients = read_clients(&self.socket)?;
        let mut living = HashSet::new();
        for (identity, process) in &self.processes {
            if process.alive()? {
                living.insert(*identity);
            }
        }
        self.apps.retain(|app| keep_app(app, &clients, &living));
        if self.dry_run {
            return Ok(());
        }
        let window_pids: HashSet<i32> = clients
            .iter()
            .filter(|client| client.pid > 0)
            .map(|client| client.pid)
            .collect();
        let mut signals = HashSet::new();
        for app in &self.apps {
            if repeat
                && app.window
                && window_present(app, &clients)
                && app
                    .identity
                    .is_none_or(|identity| living.contains(&identity))
            {
                let command = if self.lua {
                    format!(
                        "/dispatch hl.dsp.window.close({{ window = 'address:{}' }})",
                        app.address
                    )
                } else {
                    format!("/dispatch closewindow address:{}", app.address)
                };
                self.dispatch(&command)?;
            }
            if let Some(identity) = app.identity
                && should_term(identity, &living, &window_pids, &self.termed, repeat)
            {
                signals.insert(identity);
            }
        }
        for identity in signals {
            self.signal(identity, libc::SIGTERM)?;
            self.termed.insert(identity);
        }
        if repeat {
            self.last_close = Instant::now();
        }
        Ok(())
    }

    fn dispatch(&self, command: &str) -> Result<(), String> {
        if self.dry_run {
            return Ok(());
        }
        if self.verbose {
            eprintln!("dusk: {command}");
        }
        let reply = request(&self.socket, command)?;
        if reply.trim() != "ok" {
            return Err(format!("Hyprland rejected {command}: {}", reply.trim()));
        }
        Ok(())
    }

    fn signal(&self, identity: Identity, signal: i32) -> Result<(), String> {
        if self.dry_run {
            return Ok(());
        }
        let process = self
            .processes
            .get(&identity)
            .ok_or_else(|| format!("Missing process identity for PID {}", identity.pid))?;
        if self.verbose {
            eprintln!("dusk: signal {signal} to PID {}", identity.pid);
        }
        process.signal(signal)
    }
}

fn keep_app(app: &App, clients: &[Client], living: &HashSet<Identity>) -> bool {
    app.identity
        .is_some_and(|identity| living.contains(&identity))
        || (app.window && window_present(app, clients))
}

fn window_present(app: &App, clients: &[Client]) -> bool {
    clients
        .iter()
        .any(|client| client.address == app.address && client.pid == app.pid)
}

fn should_term(
    identity: Identity,
    living: &HashSet<Identity>,
    window_pids: &HashSet<i32>,
    termed: &HashSet<Identity>,
    repeat: bool,
) -> bool {
    living.contains(&identity)
        && !window_pids.contains(&identity.pid)
        && (repeat || !termed.contains(&identity))
}

fn parse_lock(lock: &str) -> Result<i32, String> {
    let mut lines = lock.lines();
    let pid = lines
        .next()
        .and_then(|line| line.parse::<i32>().ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| "hyprland.lock contains an invalid process ID".to_string())?;
    if lines.next().is_none_or(str::is_empty) || lines.any(|line| !line.is_empty()) {
        return Err("hyprland.lock contains invalid instance data".into());
    }
    Ok(pid)
}

fn validate_address(address: &str) -> Result<(), String> {
    let valid = address.strip_prefix("0x").is_some_and(|hex| {
        !hex.is_empty()
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            && u64::from_str_radix(hex, 16).is_ok_and(|value| value > 0)
    });
    if !valid {
        return Err(format!(
            "Hyprland returned an invalid window address: {address:?}"
        ));
    }
    Ok(())
}

fn read_clients(socket: &Path) -> Result<Vec<Client>, String> {
    let clients: Vec<Client> = json_request(socket, "j/clients")?;
    let mut addresses = HashSet::new();
    for client in &clients {
        validate_address(&client.address)?;
        if !addresses.insert(&client.address) {
            return Err(format!(
                "Hyprland returned duplicate window address {}",
                client.address
            ));
        }
    }
    Ok(clients)
}

fn json_request<T: serde::de::DeserializeOwned>(socket: &Path, command: &str) -> Result<T, String> {
    let reply = request(socket, command)?;
    serde_json::from_str(&reply)
        .map_err(|error| format!("Invalid Hyprland {command} response: {error}"))
}

fn request(path: &Path, command: &str) -> Result<String, String> {
    let deadline = Instant::now() + IPC_TIMEOUT;
    let mut stream = connect_socket(path, deadline)?;
    let mut pending = command.as_bytes();
    while !pending.is_empty() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(ipc_error)?;
        match stream.write(pending) {
            Ok(0) => return Err("Hyprland IPC closed while sending a command".into()),
            Ok(count) => pending = &pending[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ipc_error(error)),
        }
    }
    let mut reply = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(ipc_error)?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if reply.len() + count > MAX_REPLY {
                    return Err("Hyprland IPC reply exceeds 16 MiB".into());
                }
                reply.extend_from_slice(&buffer[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ipc_error(error)),
        }
    }
    String::from_utf8(reply)
        .map_err(|error| format!("Hyprland IPC returned invalid UTF-8: {error}"))
}

fn remaining(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| "Hyprland IPC timed out".to_string())
}

fn ipc_error(error: io::Error) -> String {
    format!("Hyprland IPC failed: {error}")
}

fn connect_socket(path: &Path, deadline: Instant) -> Result<UnixStream, String> {
    let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err("Hyprland socket path is invalid or too long".into());
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as libc::c_char;
    }
    let raw = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    };
    if raw < 0 {
        return Err(ipc_error(io::Error::last_os_error()));
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    let result = unsafe {
        libc::connect(
            fd.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            mem::size_of_val(&address) as libc::socklen_t,
        )
    };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(format!("Cannot connect to {}: {error}", path.display()));
        }
        if !wait_fd(fd.as_raw_fd(), libc::POLLOUT, remaining(deadline)?)? {
            return Err("Hyprland IPC connection timed out".into());
        }
        let mut error: libc::c_int = 0;
        let mut length = mem::size_of_val(&error) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                (&mut error as *mut libc::c_int).cast(),
                &mut length,
            )
        } < 0
        {
            return Err(ipc_error(io::Error::last_os_error()));
        }
        if error != 0 {
            return Err(ipc_error(io::Error::from_raw_os_error(error)));
        }
    }
    let stream = UnixStream::from(fd);
    stream.set_nonblocking(false).map_err(ipc_error)?;
    Ok(stream)
}

fn wait_fd(fd: libc::c_int, events: libc::c_short, timeout: Duration) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let milliseconds = remaining
            .as_millis()
            .saturating_add(u128::from(!remaining.is_zero()))
            .min(i32::MAX as u128) as i32;
        let mut descriptor = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(format!("Process/socket polling failed: {error}"));
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err("Process/socket descriptor became invalid".into());
        }
        return Ok(result > 0);
    }
}

impl Process {
    fn open(pid: i32) -> Result<Option<Self>, String> {
        let Some(before) = proc_stat(pid)? else {
            return Ok(None);
        };
        if before.dead {
            return Ok(None);
        }
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if raw < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(None);
            }
            return Err(format!(
                "Cannot safely track PID {pid} (pidfd_open): {error}"
            ));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw as libc::c_int) };
        let Some(after) = proc_stat(pid)? else {
            return Ok(None);
        };
        if after.dead || before.identity != after.identity {
            return Err(format!("PID {pid} changed identity during discovery"));
        }
        Ok(Some(Self {
            identity: before.identity,
            fd,
        }))
    }

    fn alive(&self) -> Result<bool, String> {
        Ok(!self.wait(Duration::ZERO)?)
    }

    fn wait(&self, timeout: Duration) -> Result<bool, String> {
        wait_fd(self.fd.as_raw_fd(), libc::POLLIN, timeout)
    }

    fn signal(&self, signal: i32) -> Result<(), String> {
        // The pidfd keeps signaling tied to the original process across PID reuse.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.fd.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(format!(
                    "Cannot send signal {signal} to PID {}: {error}",
                    self.identity.pid
                ));
            }
        }
        Ok(())
    }
}

fn proc_stat(pid: i32) -> Result<Option<ProcStat>, String> {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => parse_stat(pid, &stat).map(Some),
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ESRCH)) => Ok(None),
        Err(error) => Err(format!("Cannot inspect PID {pid}: {error}")),
    }
}

fn parse_stat(pid: i32, stat: &str) -> Result<ProcStat, String> {
    let invalid = || format!("Invalid /proc/{pid}/stat");
    let start = stat.find('(').ok_or_else(invalid)?;
    let end = stat
        .rfind(')')
        .filter(|end| *end > start)
        .ok_or_else(invalid)?;
    let mut fields = stat[end + 1..].split_whitespace();
    let state = fields.next().ok_or_else(invalid)?;
    let parent = fields
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    let started = fields
        .nth(17)
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    Ok(ProcStat {
        identity: Identity { pid, started },
        parent,
        name: stat[start + 1..end].to_owned(),
        dead: matches!(state, "Z" | "X" | "x"),
    })
}

fn process_name(pid: i32) -> Result<Option<String>, String> {
    Ok(proc_stat(pid)?.map(|stat| stat.name))
}

fn ancestors() -> Result<HashSet<i32>, String> {
    let mut excluded = HashSet::new();
    let mut pid = std::process::id() as i32;
    while pid > 0 && excluded.insert(pid) {
        let stat = proc_stat(pid)?
            .ok_or_else(|| format!("Ancestor PID {pid} disappeared during discovery"))?;
        pid = stat.parent;
    }
    Ok(excluded)
}

fn children_of(parent: i32) -> Result<Vec<ProcStat>, String> {
    let entries =
        fs::read_dir("/proc").map_err(|error| format!("Cannot enumerate processes: {error}"))?;
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("Cannot enumerate processes: {error}"))?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        if let Some(stat) = proc_stat(pid)?
            && stat.parent == parent
            && !stat.dead
        {
            children.push(stat);
        }
    }
    Ok(children)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(started: u64) -> Identity {
        Identity { pid: 1234, started }
    }

    fn app() -> App {
        App {
            address: "0xabcd".into(),
            class: "Editor".into(),
            title: "Unsaved".into(),
            pid: 1234,
            window: true,
            identity: Some(identity(10)),
        }
    }

    fn client(address: &str) -> Client {
        Client {
            address: address.into(),
            class: "Editor".into(),
            title: "Unsaved".into(),
            pid: 1234,
        }
    }

    fn session(dry_run: bool) -> Session {
        let raw = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        assert!(raw >= 0);
        Session {
            socket: PathBuf::from("/dusk-test/nonexistent.sock"),
            apps: vec![app()],
            processes: HashMap::new(),
            compositor: Process {
                identity: identity(99),
                fd: unsafe { OwnedFd::from_raw_fd(raw) },
            },
            dry_run,
            no_exit: false,
            verbose: false,
            lua: false,
            begun: true,
            forced: false,
            finished: false,
            failure: None,
            last_close: Instant::now(),
            termed: HashSet::new(),
        }
    }

    fn options() -> Options {
        Options {
            dry_run: false,
            no_exit: false,
            top_label: String::new(),
            command: Some("exit 99".into()),
            verbose: false,
            no_fork: true,
            vt: Some(2),
            appearance: Default::default(),
        }
    }

    #[test]
    fn retains_until_process_and_window_are_gone() {
        let app = app();
        let living = HashSet::from([identity(10)]);
        assert!(keep_app(&app, &[], &living));
        assert!(keep_app(&app, &[client("0xabcd")], &HashSet::new()));
        assert!(!keep_app(&app, &[], &HashSet::new()));
    }

    #[test]
    fn reused_pid_does_not_keep_original_process_alive() {
        assert!(!keep_app(&app(), &[], &HashSet::from([identity(11)])));
    }

    #[test]
    fn any_window_prevents_process_fallback_signal() {
        let living = HashSet::from([identity(10)]);
        let windows = HashSet::from([1234]);
        assert!(!should_term(
            identity(10),
            &living,
            &windows,
            &HashSet::new(),
            true
        ));
        assert!(should_term(
            identity(10),
            &living,
            &HashSet::new(),
            &HashSet::new(),
            false
        ));
        assert!(!should_term(
            identity(10),
            &living,
            &HashSet::new(),
            &living,
            false
        ));
        assert!(should_term(
            identity(10),
            &living,
            &HashSet::new(),
            &living,
            true
        ));
    }

    #[test]
    fn dead_and_reused_processes_are_not_signaled() {
        assert!(!should_term(
            identity(10),
            &HashSet::from([identity(11)]),
            &HashSet::new(),
            &HashSet::new(),
            true
        ));
    }

    #[test]
    fn parses_proc_names_containing_spaces_and_parentheses() {
        let stat = format!("1234 (an odd (name)) S 42 {} 9876 0", ["0"; 17].join(" "));
        let parsed = parse_stat(1234, &stat).unwrap();
        assert_eq!(parsed.parent, 42);
        assert_eq!(parsed.identity.started, 9876);
        assert_eq!(parsed.name, "an odd (name)");
    }

    #[test]
    fn rejects_malformed_socket_data_before_dispatch() {
        assert!(serde_json::from_str::<Vec<Client>>("{}").is_err());
        assert!(
            serde_json::from_str::<Vec<Client>>(
                r#"[{"address":"0x1","class":"x","title":"x","pid":"12"}]"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<HashMap<String, MonitorLayers>>(r#"{"DP-1":{}}"#).is_err());
        for address in ["", "0x0", "0x+1", "0x1' })", "address:0x1", "0xno"] {
            assert!(validate_address(address).is_err());
        }
        assert!(validate_address("0x123abc").is_ok());
    }

    #[test]
    fn validates_compositor_lock() {
        assert_eq!(parse_lock("1234\nwayland-1\n").unwrap(), 1234);
        for lock in ["", "1\nwayland-1\n", "1234\n", "1234\nwayland-1\nextra"] {
            assert!(parse_lock(lock).is_err());
        }
    }

    #[test]
    fn dry_run_guards_dispatch_signal_force_and_finish() {
        let mut session = session(true);
        assert!(session.dispatch("/dispatch exit").is_ok());
        assert!(session.signal(identity(10), libc::SIGKILL).is_ok());
        assert!(session.force().is_ok());
        assert!(session.finish(&options()).is_ok());
        assert!(!session.finished);
    }

    #[test]
    fn no_exit_suppresses_all_completion_side_effects() {
        let mut session = session(false);
        session.no_exit = true;
        assert!(session.finish(&options()).is_ok());
        assert!(!session.finished);
    }

    #[test]
    fn failure_latches_and_prevents_forced_completion() {
        let mut session = session(false);
        session.apps.clear();
        session
            .record(Err("invalid IPC response".into()))
            .unwrap_err();
        assert_eq!(session.poll().unwrap_err(), "invalid IPC response");
        assert_eq!(session.force().unwrap_err(), "invalid IPC response");
        assert_eq!(
            session.finish(&options()).unwrap_err(),
            "invalid IPC response"
        );
        assert!(!session.finished);
    }

    #[test]
    fn completion_requires_empty_apps_or_successful_force() {
        let mut session = session(false);
        assert!(
            session
                .finish(&options())
                .unwrap_err()
                .contains("before application shutdown")
        );
        assert!(!session.finished);
    }
}
