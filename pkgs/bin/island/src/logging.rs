//! Recent entries are kept in memory and streamed over island's log socket for `island logs`, much
//! as `hyprctl rollinglog` reads Hyprland.

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    collections::{HashMap, VecDeque},
    env,
    fmt::{self, Write as _},
    fs::{self, File},
    io::{self, BufRead, BufReader, IsTerminal, Read, Write as _},
    os::{
        fd::AsFd,
        unix::{
            fs::{MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::PathBuf,
    sync::{
        Arc, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, SyncSender},
    },
    thread,
    time::Duration,
};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing::{
    Event, Level, Subscriber, debug, error, info,
    field::{Field, Visit},
    level_filters::LevelFilter,
    warn,
};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    reload,
    util::SubscriberInitExt,
};

use crate::supervisor::{self, Ended};

const CRATE: &str = "island";
pub const DEFAULT: &str = "info";
/// Entries kept for clients that connect later.
const BACKLOG: usize = 1000;
/// Entries a client may fall behind by before it is dropped.
const CLIENT_QUEUE: usize = 1024;
/// How often a waiting client is checked for having gone away.
const CLIENT_CHECK: Duration = Duration::from_secs(1);

pub const LOGS_USAGE: &str = "\
usage: island logs [-n LINES] [--level LEVEL] [--module NAME] [--no-follow]

  -n LINES        recent entries to show before following (default 50)
  --level LEVEL   error, warn, info, debug or trace: show this level and more severe ones,
                  raising what island logs to it while connected
  --module NAME   only this module or feature, e.g. audio, media, tray, supervisor
  --no-follow     print the recent entries and exit

Reads the running island directly over its log socket, however it was started.";

static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn check_level(text: &str) -> Result<LevelFilter, String> {
    text.parse::<LevelFilter>()
        .map_err(|_| format!("'{text}' is not a log level (error, warn, info, debug, trace or off)"))
}

/// `spec` is like `debug` or `audio=debug,tray=trace`: a bare level applies to island, `NAME=LEVEL`
/// to one of its modules or features, or a dependency. Dependencies otherwise log warnings and
/// errors only, since GPUI, wgpu and zbus are noisy. `at_least` raises island's level for connected
/// `island logs --level` clients, never lowering it.
fn filter(spec: &str, at_least: Option<LevelFilter>) -> Result<EnvFilter, String> {
    let mut island = check_level(DEFAULT)?;
    let mut directives = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        match part.split_once('=') {
            None => island = check_level(part)?,
            Some((name, level)) => {
                check_level(level)?;
                directives.push(format!("{name}={level}"));
                if !name.contains("::") && name != CRATE {
                    directives.push(format!("{CRATE}::{name}={level}"));
                    directives.push(format!("{CRATE}::features::{name}={level}"));
                }
            }
        }
    }
    let island = at_least.map_or(island, |raised| island.max(raised));
    let spec = format!("warn,{CRATE}={island},{}", directives.join(","));
    EnvFilter::builder().parse(spec).map_err(|error| error.to_string())
}

/// Whether stderr is the journal stream systemd gave the service, rather than a terminal that merely
/// inherited `JOURNAL_STREAM` from a service-run terminal emulator.
fn stderr_is_journal() -> bool {
    let Some(stream) = env::var_os("JOURNAL_STREAM") else {
        return false;
    };
    let Some((device, inode)) = stream.to_str().and_then(|stream| stream.split_once(':')) else {
        return false;
    };
    let Ok(stderr) = io::stderr()
        .as_fd()
        .try_clone_to_owned()
        .map(File::from)
        .and_then(|file| file.metadata())
    else {
        return false;
    };
    device.parse() == Ok(stderr.dev()) && inode.parse() == Ok(stderr.ino())
}

fn use_color(stream: &impl IsTerminal) -> bool {
    stream.is_terminal() && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}

/// The module part of a target: `island::shell` is `shell`, and a feature's
/// `island::features::audio::pulse` is `audio::pulse`.
fn module(target: &str) -> &str {
    target
        .strip_prefix("island::features::")
        .or_else(|| target.strip_prefix("island::"))
        .unwrap_or(target)
}

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    /// Unix time in microseconds.
    time: i64,
    level: String,
    module: String,
    message: String,
    fields: Vec<(String, String)>,
}

impl Entry {
    fn level(&self) -> Level {
        self.level.parse().unwrap_or(Level::INFO)
    }

    fn render(&self, color: bool) -> String {
        let time = DateTime::<Utc>::from_timestamp_micros(self.time)
            .unwrap_or_default()
            .with_timezone(&Local)
            .format("%H:%M:%S%.3f");
        let level = self.level.as_str();
        let mut line = String::new();
        let _ = if color {
            let tint = match level {
                "ERROR" => "31",
                "WARN" => "33",
                "INFO" => "32",
                "DEBUG" => "34",
                _ => "35",
            };
            write!(line, "\x1b[2m{time}\x1b[0m \x1b[{tint}m{level:>5}\x1b[0m \x1b[2m{}:\x1b[0m ", self.module)
        } else {
            write!(line, "{time} {level:>5} {}: ", self.module)
        };
        line.push_str(&self.message);
        for (name, value) in &self.fields {
            let _ = if color {
                write!(line, " \x1b[3m{name}\x1b[0m\x1b[2m=\x1b[0m{value}")
            } else {
                write!(line, " {name}={value}")
            };
        }
        line
    }
}

/// Text values unquoted.
#[derive(Default)]
struct Collect {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for Collect {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields.push((field.name().to_owned(), value.to_owned()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push((field.name().to_owned(), format!("{value:?}")));
        }
    }
}

fn entry(event: &Event<'_>) -> Entry {
    let metadata = event.metadata();
    let mut collect = Collect::default();
    event.record(&mut collect);
    Entry {
        time: Utc::now().timestamp_micros(),
        level: metadata.level().as_str().to_owned(),
        module: module(metadata.target()).to_owned(),
        message: collect.message,
        fields: collect.fields,
    }
}

struct Compact;

impl<S, N> FormatEvent<S, N> for Compact
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, _: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> fmt::Result {
        let color = writer.has_ansi_escapes();
        writeln!(writer, "{}", entry(event).render(color))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Request {
    lines: usize,
    level: Option<String>,
    module: Option<String>,
    follow: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            lines: 50,
            level: None,
            module: None,
            follow: true,
        }
    }
}

impl Request {
    fn wants(&self, entry: &Entry, level: Option<Level>) -> bool {
        level.is_none_or(|level| entry.level() <= level)
            && self.module.as_ref().is_none_or(|wanted| {
                entry.module == *wanted || entry.module.starts_with(&format!("{wanted}::"))
            })
    }
}

#[derive(Default)]
struct Hub {
    backlog: Mutex<VecDeque<Arc<Entry>>>,
    clients: Mutex<Vec<SyncSender<Arc<Entry>>>>,
    configured: Mutex<String>,
    raised: Mutex<HashMap<u64, LevelFilter>>,
}

static HUB: LazyLock<Hub> = LazyLock::new(Hub::default);

impl Hub {
    /// Lock order is backlog, then clients, here and in [`Hub::subscribe`], so a client joining
    /// mid-entry neither misses nor repeats it.
    fn publish(&self, entry: Entry) {
        let entry = Arc::new(entry);
        let mut backlog = lock(&self.backlog);
        if backlog.len() == BACKLOG {
            backlog.pop_front();
        }
        backlog.push_back(entry.clone());
        lock(&self.clients).retain(|client| client.try_send(entry.clone()).is_ok());
    }

    fn subscribe(&self, request: &Request, level: Option<Level>) -> (Vec<Arc<Entry>>, Option<mpsc::Receiver<Arc<Entry>>>) {
        let backlog = lock(&self.backlog);
        let receiver = request.follow.then(|| {
            let (sender, receiver) = mpsc::sync_channel(CLIENT_QUEUE);
            lock(&self.clients).push(sender);
            receiver
        });
        let mut recent: Vec<Arc<Entry>> = backlog
            .iter()
            .rev()
            .filter(|entry| request.wants(entry, level))
            .take(request.lines)
            .cloned()
            .collect();
        recent.reverse();
        (recent, receiver)
    }

    fn apply(&self) -> Result<(), String> {
        let spec = lock(&self.configured).clone();
        let at_least = lock(&self.raised).values().max().copied();
        let filter = filter(&spec, at_least)?;
        FILTER
            .get()
            .ok_or("logging is not running")?
            .reload(filter)
            .map_err(|error| error.to_string())
    }

    fn raise(&self, client: u64, level: LevelFilter) {
        lock(&self.raised).insert(client, level);
        if let Err(error) = self.apply() {
            warn!("cannot raise the log level: {error}");
        }
    }

    fn release(&self, client: u64) {
        if lock(&self.raised).remove(&client).is_some()
            && let Err(error) = self.apply()
        {
            warn!("cannot restore the log level: {error}");
        }
    }
}

struct Rolling;

impl<S: Subscriber> Layer<S> for Rolling {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        HUB.publish(entry(event));
    }
}

/// An invalid `spec` falls back to the default and is reported once logging runs.
pub fn init(spec: &str) {
    let (initial, invalid) = match filter(spec, None) {
        Ok(filter) => (filter, None),
        Err(error) => (EnvFilter::new(format!("warn,{CRATE}={DEFAULT}")), Some(error)),
    };
    *lock(&HUB.configured) = if invalid.is_none() { spec } else { DEFAULT }.to_owned();
    let (initial, handle) = reload::Layer::new(initial);
    let journal = stderr_is_journal().then(|| tracing_journald::layer().ok()).flatten();
    let terminal = journal.is_none().then(|| {
        tracing_subscriber::fmt::layer()
            .with_writer(io::stderr)
            .with_ansi(use_color(&io::stderr()))
            .event_format(Compact)
    });
    let installed = Registry::default()
        .with(initial)
        .with(Rolling)
        .with(journal)
        .with(terminal)
        .try_init();
    if installed.is_ok() {
        let _ = FILTER.set(handle);
    }
    if let Some(error) = invalid {
        error!(spec, "invalid log filter, using the default: {error}");
    }
}

pub fn set(spec: &str) -> Result<(), String> {
    filter(spec, None)?;
    *lock(&HUB.configured) = spec.to_owned();
    HUB.apply()?;
    info!(spec, "log filter changed");
    Ok(())
}

/// So a crash on any thread is logged as an error entry instead of bare stderr text.
pub fn catch_panics() {
    std::panic::set_hook(Box::new(|info| {
        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");
        let message = info.payload_as_str().unwrap_or("unknown cause");
        let at = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_default();
        let backtrace = Backtrace::capture();
        if backtrace.status() == BacktraceStatus::Captured {
            error!(target: "island::panic", thread, at, "panicked: {message}\n{backtrace}");
        } else {
            error!(target: "island::panic", thread, at, "panicked: {message}");
        }
    }));
}

fn socket_path() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join("island").join("logs.sock"))
}

/// Whether a client that sends nothing after its request has hung up.
fn hung_up(stream: &UnixStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return true;
    }
    let mut byte = [0u8; 1];
    let gone = match (&*stream).read(&mut byte) {
        Ok(_) => true,
        Err(error) => error.kind() != io::ErrorKind::WouldBlock,
    };
    let _ = stream.set_nonblocking(false);
    gone
}

fn send(stream: &mut UnixStream, entry: &Entry) -> bool {
    serde_json::to_string(entry).is_ok_and(|line| writeln!(stream, "{line}").is_ok())
}

fn serve_client(stream: UnixStream, client: u64) {
    let mut line = String::new();
    let Ok(reader) = stream.try_clone() else {
        return;
    };
    if BufReader::new(reader).read_line(&mut line).is_err() {
        return;
    }
    let request: Request = serde_json::from_str(&line).unwrap_or_default();
    let level: Option<Level> = request.level.as_deref().and_then(|level| level.parse().ok());
    debug!(client, level = request.level, module = request.module, follow = request.follow, "log client connected");
    let (recent, receiver) = HUB.subscribe(&request, level);
    let mut stream = stream;
    if !recent.iter().all(|entry| send(&mut stream, entry)) {
        return;
    }
    let Some(receiver) = receiver else {
        return;
    };
    if let Some(level) = level {
        HUB.raise(client, LevelFilter::from_level(level));
    }
    loop {
        match receiver.recv_timeout(CLIENT_CHECK) {
            Ok(entry) if request.wants(&entry, level) && !send(&mut stream, &entry) => break,
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) if hung_up(&stream) => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    HUB.release(client);
    debug!(client, "log client left");
}

/// Called once island owns its instance name, so a second island cannot take the socket over.
pub fn serve() {
    let Some(path) = socket_path() else {
        warn!("XDG_RUNTIME_DIR is not set; island logs will not work");
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    let _ = fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) => {
            error!(path = %path.display(), "cannot open the log socket: {error}");
            return;
        }
    };
    debug!(path = %path.display(), "serving logs");
    static CLIENTS: AtomicU64 = AtomicU64::new(0);
    let served = supervisor::supervise("logs", move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client = CLIENTS.fetch_add(1, Ordering::Relaxed);
                    let _ = thread::Builder::new()
                        .name("island-log-client".into())
                        .spawn(move || serve_client(stream, client));
                }
                Err(error) => return Ended::Failed(error.to_string()),
            }
        }
        Ended::Failed("the log socket closed".to_owned())
    });
    if let Err(error) = served {
        error!("cannot serve logs: {error}");
    }
}

fn parse_logs(args: Vec<String>) -> Result<Request, String> {
    let mut request = Request::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" | "--lines" => {
                let value = args.next().ok_or("-n needs a number")?;
                request.lines = value.parse().map_err(|_| format!("'{value}' is not a number"))?;
            }
            "--level" => {
                let value = args.next().ok_or("--level needs a level")?;
                value.parse::<Level>().map_err(|_| format!("'{value}' is not a log level"))?;
                request.level = Some(value);
            }
            "--module" => request.module = Some(args.next().ok_or("--module needs a name")?),
            "--no-follow" => request.follow = false,
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(request)
}

fn follow(request: &Request) -> Result<(), String> {
    let path = socket_path().ok_or("XDG_RUNTIME_DIR is not set")?;
    let mut stream = UnixStream::connect(&path).map_err(|_| "island is not running".to_owned())?;
    let line = serde_json::to_string(request).map_err(|error| error.to_string())?;
    writeln!(stream, "{line}").map_err(|error| format!("cannot reach island: {error}"))?;
    let color = use_color(&io::stdout());
    let mut stdout = io::stdout().lock();
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            break;
        };
        let Ok(entry) = serde_json::from_str::<Entry>(&line) else {
            continue;
        };
        if writeln!(stdout, "{}", entry.render(color)).is_err() {
            break;
        }
    }
    Ok(())
}

pub fn logs(args: Vec<String>) -> ! {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("{LOGS_USAGE}");
        std::process::exit(0);
    }
    match parse_logs(args).and_then(|request| follow(&request)) {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("island logs: {error}");
            std::process::exit(1);
        }
    }
}
