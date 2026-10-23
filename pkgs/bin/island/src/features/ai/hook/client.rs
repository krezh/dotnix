use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

use zbus::{block_on, connection, fdo::DBusProxy, names::BusName};

use super::{
    plugin,
    server::{INTERFACE, NAME, PATH},
};
use crate::{cli, dbus};

/// Longer than any hook may run, so Claude Code's own timeout is the one that ends the wait.
const WAIT: Duration = Duration::from_secs(2 * 60 * 60);
/// How long a passed-through command's caller still gives island the event once it is done.
const AFTER_COMMAND: Duration = Duration::from_secs(1);

/// Passes one Claude Code hook to the running island and prints its answer. It always exits 0, at
/// once when island is not running, so Claude Code carries on as if there were no hook. After
/// `--`, a command such as a status line gets the same input and prints its own output, without
/// waiting on island. `plugin <dir>` instead writes the plugin that runs these hooks.
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        cli::print_help(Some("ai-hook"));
        return 0;
    }
    let Some(event) = args.first() else {
        return mistake(
            cli::Line::new()
                .push(cli::Tone::Command, "island ai-hook")
                .plain(" needs an event"),
        );
    };
    if event == "plugin" {
        return write_plugin(args.get(1));
    }
    // island's own chat is not a session to follow.
    let followed = std::env::var_os(crate::features::ai::converse::CHAT_ENV).is_none();
    let at = chrono::Utc::now().timestamp_millis();
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }
    let command = args
        .iter()
        .position(|arg| arg == "--")
        .map(|at| &args[at + 1..])
        .filter(|command| !command.is_empty());
    if let Some(command) = command {
        pass_through(command, event.clone(), input, at, followed);
        return 0;
    }
    if !followed {
        return 0;
    }
    if let Some(reply) = send(event, &input, at)
        && !reply.is_empty()
    {
        let mut out = std::io::stdout();
        let _ = out.write_all(reply.as_bytes());
        let _ = out.flush();
    }
    0
}

fn mistake(what: cli::Line) -> i32 {
    cli::print_error(&cli::Mistake::new(what).see_help("island ai-hook").lines());
    0
}

/// Island's answer to one hook, or `None` when it could not be asked.
fn send(event: &str, input: &str, at: i64) -> Option<String> {
    let reply: Result<String, zbus::Error> = block_on(async {
        let connection = connection::Builder::session()
            .method_timeout(WAIT)
            .build()
            .await?;
        let name = BusName::try_from(NAME)?;
        if !DBusProxy::new(&connection).await?.name_has_owner(name).await? {
            return Ok(String::new());
        }
        let proxy = dbus::uncached(&connection, NAME, PATH, INTERFACE).await?;
        proxy.call("Hook", &(event, input, at)).await
    });
    reply.ok()
}

/// Runs `command` on `input` while island gets the event, and gives island only a moment more
/// once the command is done.
fn pass_through(command: &[String], event: String, input: String, at: i64, followed: bool) {
    let child = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .spawn();
    let told = followed.then(|| {
        let (sent, told) = mpsc::channel();
        let input = input.clone();
        std::thread::spawn(move || {
            let _ = send(&event, &input, at);
            let _ = sent.send(());
        });
        told
    });
    if let Ok(mut child) = child {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let _ = child.wait();
    }
    if let Some(told) = told {
        let _ = told.recv_timeout(AFTER_COMMAND);
    }
}

fn write_plugin(dir: Option<&String>) -> i32 {
    let Some(dir) = dir else {
        return mistake(
            cli::Line::new()
                .push(cli::Tone::Command, "island ai-hook plugin")
                .plain(" needs a directory"),
        );
    };
    let written =
        std::env::current_exe().and_then(|island| plugin::write(Path::new(dir), &island));
    match written {
        Ok(()) => 0,
        Err(error) => {
            cli::print_error(
                &cli::Mistake::new(
                    cli::Line::new().plain(format!("cannot write the plugin to {dir}: {error}")),
                )
                .lines(),
            );
            1
        }
    }
}
