use std::{
    env,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::{error, info, warn};
use zbus::{blocking::Connection, zvariant::Value};

/// What to start: an argv plus the identity and directory it runs with.
pub struct Launch {
    /// Application id, the desktop file name without `.desktop`; names the unit.
    pub id: String,
    pub name: String,
    pub argv: Vec<String>,
    pub directory: Option<PathBuf>,
}

/// Variables that belong to island's own service, not to the apps it starts.
const PRIVATE_ENV: [&str; 8] = [
    "INVOCATION_ID",
    "JOURNAL_STREAM",
    "NOTIFY_SOCKET",
    "MANAGERPID",
    "SYSTEMD_EXEC_PID",
    "LISTEN_PID",
    "LISTEN_FDS",
    "LISTEN_FDNAMES",
];

/// Starts the app as its own transient systemd user service in `app.slice`, so it outlives
/// island and logs to the journal under its own name. Falls back to a detached process when the
/// user manager cannot be reached.
pub fn spawn(launch: Launch) {
    thread::spawn(move || {
        let Some(program) = resolve(&launch.argv[0]) else {
            error!(app = launch.name, program = launch.argv[0], "cannot launch: program not found in PATH");
            return;
        };
        match transient_unit(&launch, &program) {
            Ok(unit) => info!(app = launch.name, unit, "launched"),
            Err(error) => {
                warn!(app = launch.name, "systemd launch failed, starting it directly: {error}");
                detached(&launch, &program);
            }
        }
    });
}

fn resolve(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// `app-<id>@<random>.service`, the XDG naming systemd and desktops use for launched apps.
fn unit_name(id: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let escaped: String = id
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b':' | b'_' | b'.' => (byte as char).to_string(),
            _ => format!("\\x{byte:02x}"),
        })
        .collect();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_nanos() as u64);
    let random = nanos ^ (u64::from(std::process::id()) << 32) ^ COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("app-{escaped}@{random:016x}.service")
}

fn transient_unit(launch: &Launch, program: &Path) -> zbus::Result<String> {
    let environment: Vec<String> = env::vars()
        .filter(|(key, _)| !PRIVATE_ENV.contains(&key.as_str()))
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    let exec_start = vec![(program.to_string_lossy().into_owned(), launch.argv.clone(), false)];
    let directory = launch
        .directory
        .as_ref()
        .map_or_else(|| "~".to_owned(), |directory| directory.to_string_lossy().into_owned());
    let properties: Vec<(&str, Value)> = vec![
        ("Description", Value::from(launch.name.as_str())),
        ("Slice", Value::from("app.slice")),
        ("Type", Value::from("exec")),
        ("ExitType", Value::from("cgroup")),
        ("OOMPolicy", Value::from("continue")),
        ("CollectMode", Value::from("inactive-or-failed")),
        ("WorkingDirectory", Value::from(directory)),
        ("Environment", Value::from(environment)),
        ("ExecStart", Value::from(exec_start)),
    ];
    let auxiliary: Vec<(&str, Vec<(&str, Value)>)> = Vec::new();
    let connection = Connection::session()?;
    let unit = unit_name(&launch.id);
    connection.call_method(
        Some("org.freedesktop.systemd1"),
        "/org/freedesktop/systemd1",
        Some("org.freedesktop.systemd1.Manager"),
        "StartTransientUnit",
        &(unit.as_str(), "fail", properties, auxiliary),
    )?;
    Ok(unit)
}

fn detached(launch: &Launch, program: &Path) {
    let mut command = Command::new(program);
    command
        .args(&launch.argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_remove("NOTIFY_SOCKET")
        .current_dir(launch.directory.clone().or_else(|| env::var_os("HOME").map(PathBuf::from)).unwrap_or_else(|| "/".into()));
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    match command.spawn() {
        Ok(mut child) => {
            info!(app = launch.name, pid = child.id(), "launched directly");
            let _ = child.wait();
        }
        Err(error) => error!(app = launch.name, "launch failed: {error}"),
    }
}

/// Splits a desktop entry `Exec` line into an argv per the Desktop Entry spec: double-quoted
/// arguments with `\"`, `` \` ``, `\$` and `\\` escapes, `%%` for a literal percent, and field codes
/// dropped (island never passes files or URLs; `%c` becomes the app name, `%i` and `%k` are omitted).
pub fn parse_exec(exec: &str, name: &str) -> Option<Vec<String>> {
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut chars = exec.chars().peekable();
    while let Some(char) = chars.next() {
        match char {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '%' => match chars.next() {
                Some('%') => current.push('%'),
                Some('c') => {
                    current.push_str(name);
                    started = true;
                }
                _ => {}
            },
            char if char.is_whitespace() && !quoted => {
                if started || !current.is_empty() {
                    argv.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            char => {
                current.push(char);
                started = true;
            }
        }
    }
    if started || !current.is_empty() {
        argv.push(current);
    }
    let argv: Vec<String> = argv.into_iter().filter(|arg| !arg.is_empty()).collect();
    (!argv.is_empty() && !quoted).then_some(argv)
}

#[cfg(test)]
mod tests {
    use super::parse_exec;

    #[test]
    fn splits_plain_words_and_drops_field_codes() {
        assert_eq!(parse_exec("firefox %U", "Firefox"), Some(vec!["firefox".to_owned()]));
        assert_eq!(
            parse_exec("code --new-window %F", "Code"),
            Some(vec!["code".to_owned(), "--new-window".to_owned()])
        );
    }

    #[test]
    fn keeps_quoted_arguments_together() {
        assert_eq!(
            parse_exec(r#"sh -c "echo \"hi there\" > \$HOME/x""#, "Sh"),
            Some(vec!["sh".to_owned(), "-c".to_owned(), r#"echo "hi there" > $HOME/x"#.to_owned()])
        );
    }

    #[test]
    fn expands_percent_and_app_name() {
        assert_eq!(
            parse_exec("app --rate 100%% --title %c", "My App"),
            Some(vec![
                "app".to_owned(),
                "--rate".to_owned(),
                "100%".to_owned(),
                "--title".to_owned(),
                "My App".to_owned()
            ])
        );
    }

    #[test]
    fn rejects_unterminated_quotes_and_empty_lines() {
        assert_eq!(parse_exec(r#"app "broken"#, "App"), None);
        assert_eq!(parse_exec("  %U ", "App"), None);
    }
}
