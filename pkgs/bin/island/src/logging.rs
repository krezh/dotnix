//! Recent entries are kept in memory and streamed over island's log socket for `island logs`, much
//! as `hyprctl rollinglog` reads Hyprland.

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    collections::{HashMap, VecDeque},
    env, fmt,
    fs::{self, File},
    io::{self, BufRead, BufReader, IsTerminal, Read, Write as _},
    net::Shutdown,
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
        mpsc::{self, SyncSender},
    },
    thread,
};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing::{
    Event, Level, Subscriber, debug, error,
    field::{Field, Visit},
    info,
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

use crate::{
    cli::{self, Line, Mistake, Tone},
    supervisor::{self, Ended},
};

const CRATE: &str = "island";
pub const DEFAULT: &str = "info";
/// Entries kept for clients that connect later.
const BACKLOG: usize = 1000;
/// Entries a client may fall behind by before it is dropped.
const CLIENT_QUEUE: usize = 1024;

static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn check_level(text: &str) -> Result<LevelFilter, String> {
    text.parse::<LevelFilter>().map_err(|_| {
        format!("'{text}' is not a log level (error, warn, info, debug, trace or off)")
    })
}

/// `spec` is like `debug` or `audio=debug,tray=trace`: a bare level applies to island, `NAME=LEVEL`
/// to one of its modules or features, or a dependency. Dependencies otherwise log warnings and
/// errors only, since GPUI, wgpu and zbus are noisy. `at_least` raises island's level for connected
/// `island logs --level` clients, never lowering it.
fn filter(spec: &str, at_least: Option<LevelFilter>) -> Result<EnvFilter, String> {
    let mut island = check_level(DEFAULT)?;
    let mut directives = Vec::new();
    for part in spec
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
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
    EnvFilter::builder()
        .parse(spec)
        .map_err(|error| error.to_string())
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
        let tone = match level {
            "ERROR" => Tone::Bad,
            "WARN" => Tone::Warning,
            "INFO" => Tone::Good,
            "DEBUG" => Tone::Command,
            _ => Tone::Heading,
        };
        let line = Line::new()
            .push(Tone::Faint, time.to_string())
            .plain(" ")
            .push(tone, format!("{level:>5}"))
            .plain(" ")
            .push(Tone::Muted, format!("{}:", self.module))
            .plain(" ")
            .plain(self.message.clone());
        self.fields
            .iter()
            .fold(line, |line, (name, value)| {
                line.plain(" ")
                    .push(Tone::Italic, name.clone())
                    .push(Tone::Faint, "=")
                    .plain(value.clone())
            })
            .render(color)
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
            self.fields
                .push((field.name().to_owned(), value.to_owned()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .push((field.name().to_owned(), format!("{value:?}")));
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
    fn format_event(
        &self,
        _: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
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
    clients: Mutex<HashMap<u64, SyncSender<Arc<Entry>>>>,
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
        lock(&self.clients).retain(|_, client| client.try_send(entry.clone()).is_ok());
    }

    fn subscribe(
        &self,
        client: u64,
        request: &Request,
        level: Option<Level>,
    ) -> (Vec<Arc<Entry>>, Option<mpsc::Receiver<Arc<Entry>>>) {
        let backlog = lock(&self.backlog);
        let receiver = request.follow.then(|| {
            let (sender, receiver) = mpsc::sync_channel(CLIENT_QUEUE);
            lock(&self.clients).insert(client, sender);
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
        lock(&self.clients).remove(&client);
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
        Err(error) => (
            EnvFilter::new(format!("warn,{CRATE}={DEFAULT}")),
            Some(error),
        ),
    };
    *lock(&HUB.configured) = if invalid.is_none() { spec } else { DEFAULT }.to_owned();
    let (initial, handle) = reload::Layer::new(initial);
    let journal = stderr_is_journal()
        .then(|| tracing_journald::layer().ok())
        .flatten();
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
    env::var_os("XDG_RUNTIME_DIR")
        .map(|runtime| PathBuf::from(runtime).join("island").join("logs.sock"))
}

fn send(stream: &mut UnixStream, entry: &Entry) -> bool {
    serde_json::to_string(entry).is_ok_and(|line| writeln!(stream, "{line}").is_ok())
}

fn serve_client(stream: UnixStream, client: u64) {
    let mut line = String::new();
    let Ok(reader) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(reader);
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let request: Request = serde_json::from_str(&line).unwrap_or_default();
    let level: Option<Level> = request
        .level
        .as_deref()
        .and_then(|level| level.parse().ok());
    debug!(
        client,
        level = request.level,
        module = request.module,
        follow = request.follow,
        "log client connected"
    );
    let (recent, receiver) = HUB.subscribe(client, &request, level);
    let mut stream = stream;
    if !recent.iter().all(|entry| send(&mut stream, entry)) {
        HUB.release(client);
        return;
    }
    let Some(receiver) = receiver else {
        return;
    };
    if let Some(level) = level {
        HUB.raise(client, LevelFilter::from_level(level));
    }
    let _ = thread::Builder::new()
        .name("island-log-reader".into())
        .spawn(move || {
            let _ = reader.read(&mut [0u8; 1]);
            HUB.release(client);
        });
    for entry in receiver {
        if request.wants(&entry, level) && !send(&mut stream, &entry) {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
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

fn parse_logs(args: Vec<String>) -> Result<Request, Mistake> {
    let mut request = Request::default();
    for (opt, value) in cli::parse_options(cli::LOGS, args, "island logs", &[])? {
        let value = value.unwrap_or_default();
        match opt.long {
            "lines" => {
                request.lines = value.parse().map_err(|_| {
                    Mistake::new(
                        Line::new()
                            .push(Tone::Command, opt.spelled())
                            .plain(" needs a whole number, got ")
                            .push(Tone::Strong, value.clone()),
                    )
                })?;
            }
            "level" => request.level = Some(value),
            "module" => request.module = Some(value),
            _ => request.follow = false,
        }
    }
    Ok(request)
}

fn plain(text: impl Into<String>) -> Mistake {
    Mistake::new(Line::new().plain(text))
}

fn follow(request: &Request) -> Result<(), Mistake> {
    let path = socket_path().ok_or_else(|| plain("XDG_RUNTIME_DIR is not set"))?;
    let mut stream = UnixStream::connect(&path).map_err(|_| cli::not_running())?;
    let line = serde_json::to_string(request).map_err(|error| plain(error.to_string()))?;
    writeln!(stream, "{line}").map_err(|error| plain(format!("Cannot reach island: {error}")))?;
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
        cli::print_help(Some("logs"));
        std::process::exit(0);
    }
    let request = match parse_logs(args) {
        Ok(request) => request,
        Err(mistake) => {
            cli::print_error(&mistake.lines());
            std::process::exit(2);
        }
    };
    match follow(&request) {
        Ok(()) => std::process::exit(0),
        Err(mistake) => {
            cli::print_error(&mistake.lines());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
#[path = "logging_test.rs"]
mod tests;
