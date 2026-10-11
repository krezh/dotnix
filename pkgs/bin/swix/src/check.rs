//! `swix check`: compares the flake repository with its upstream without starting GTK, so
//! scripts can poll it.

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};

use gtk::glib::ExitCode;
use serde::Serialize;
use swix::command::CANCELLED;

use crate::config::load_config;
use crate::home::repository::{self, RepositoryKind};

const USAGE: &str = "usage: swix check [--json] [--no-fetch]";
const HELP: &str = "\
Report incoming and local changes between the flake repository and its upstream.

  --json      print one JSON object: {\"vcs\",\"upstream\",\"incoming\",\"local\",\"fetched\"}
  --no-fetch  compare against the last fetched upstream instead of fetching (no network)
  -h, --help  show this help

Exits 0 on success, 1 when the check fails, 2 on invalid arguments, 130 when interrupted.";

/// Set by SIGINT/SIGTERM; the running git/jj child is in its own process group, so it only
/// stops when command execution sees this flag.
static CANCELLATION: AtomicBool = AtomicBool::new(false);

struct Options {
    json: bool,
    fetch: bool,
}

#[derive(Serialize)]
struct Report<'a> {
    vcs: &'a str,
    upstream: &'a str,
    incoming: usize,
    local: usize,
    fetched: bool,
}

/// Runs `swix check`; `arguments` are those after the subcommand.
pub(crate) fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    let options = match parse(arguments) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}\n\n{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("swix check: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    install_signal_handlers();
    let status = match load_config()
        .and_then(|config| repository::check(&config.flake_dir, options.fetch, &CANCELLATION))
    {
        Ok(status) => status,
        Err(error) if error == CANCELLED => return ExitCode::from(130),
        Err(error) => {
            eprintln!("swix check: {error}");
            return ExitCode::FAILURE;
        }
    };
    let vcs = match status.kind {
        RepositoryKind::Git => "git",
        RepositoryKind::Jj => "jj",
    };
    if options.json {
        let report = Report {
            vcs,
            upstream: &status.upstream,
            incoming: status.incoming,
            local: status.local,
            fetched: options.fetch,
        };
        match serde_json::to_string(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("swix check: failed to encode JSON: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!(
            "{} incoming, {} local ({vcs} {})",
            status.incoming, status.local, status.upstream
        );
    }
    ExitCode::SUCCESS
}

/// `Ok(None)` asks for help.
fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Option<Options>, String> {
    let mut options = Options {
        json: false,
        fetch: true,
    };
    for argument in arguments {
        match argument.to_str() {
            Some("--json") => options.json = true,
            Some("--no-fetch") => options.fetch = false,
            Some("-h" | "--help") => return Ok(None),
            _ => {
                return Err(format!(
                    "unknown argument: {}",
                    argument.to_string_lossy()
                ));
            }
        }
    }
    Ok(Some(options))
}

fn install_signal_handlers() {
    extern "C" fn cancel(_: libc::c_int) {
        CANCELLATION.store(true, Ordering::Relaxed);
    }
    let handler = cancel as extern "C" fn(libc::c_int) as libc::sighandler_t;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe {
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}
