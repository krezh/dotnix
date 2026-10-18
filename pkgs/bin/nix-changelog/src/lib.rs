use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{LazyLock, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::{env, fmt, fs};

use anyhow::{Context, Result, bail};
use rustix::process::{Pid, Signal, kill_process_group};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Changelog {
    pub pname: String,
    pub version: String,
    pub description: Option<String>,
    pub releases: Vec<Release>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Release {
    pub tag: String,
    pub body: String,
}

/// Everything a caller can observe/steer while resolving.
pub struct Session<'a> {
    /// `flake.lock` is read from here and every `nix` subprocess runs in it.
    pub flake_dir: &'a Path,
    /// Polled between network requests and while `nix` subprocesses run.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Receives human-readable notes on which flake reference is being tried.
    pub progress: &'a dyn Fn(&str),
}

#[derive(Deserialize, Default)]
pub struct Meta {
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub changelog: Option<String>,
}

#[derive(Deserialize)]
pub struct PackageInfo {
    pub pname: String,
    pub version: String,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    flake_input: Option<String>,
}

#[derive(Serialize)]
struct LockedFlakeInput {
    name: String,
    flake_ref: String,
    repository: Option<String>,
    rank: usize,
}

struct RepoRef {
    host: String,
    owner: String,
    repo: String,
}

enum VersionSpec {
    Single(String),
    Range {
        from: Option<String>,
        to: Option<String>,
    },
}

/// The error every cancelled operation returns; resolution stops on it instead of trying the
/// next candidate.
#[derive(Debug)]
struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

fn ensure_live(session: &Session<'_>) -> Result<()> {
    if (session.cancelled)() {
        return Err(Cancelled.into());
    }
    Ok(())
}

// `flake-self-attrs` is Lix's gate for flakes declaring `inputs.self` (e.g. `?submodules=1`).
const NIX_FEATURES: &str = "nix-command flakes flake-self-attrs";
const NIX_POLL_INTERVAL: Duration = Duration::from_millis(50);
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = "nix-changelog";
const MAX_RELEASE_PAGES: u32 = 20;

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .build()
        .into()
});

/// Runs `nix` in the flake directory. It gets its own process group so cancellation also kills
/// the evaluator processes it spawns.
fn run_nix(session: &Session<'_>, args: &[&str], spawn_error: &str) -> Result<Output> {
    ensure_live(session)?;
    let mut child = Command::new("nix")
        .args(args)
        .current_dir(session.flake_dir)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| spawn_error.to_owned())?;
    // Drained concurrently so a chatty `nix` never blocks on a full pipe.
    let stdout = drain(child.stdout.take());
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
    let stderr = drain_stderr(child.stderr.take(), stderr_tx);
    let status = loop {
        if (session.cancelled)() {
            let _ = kill_process_group(Pid::from_child(&child), Signal::KILL);
            let _ = child.wait();
            return Err(Cancelled.into());
        }
        while let Ok(line) = stderr_rx.try_recv() {
            (session.progress)(&line);
        }
        match child.try_wait().with_context(|| spawn_error.to_owned())? {
            Some(status) => break status,
            None => thread::sleep(NIX_POLL_INTERVAL),
        }
    };
    while let Ok(line) = stderr_rx.try_recv() {
        (session.progress)(&line);
    }
    Ok(Output {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

fn drain_stderr(
    pipe: Option<impl Read + Send + 'static>,
    sender: std::sync::mpsc::Sender<String>,
) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        use std::io::BufRead;
        let mut full = Vec::new();
        if let Some(pipe) = pipe {
            let mut reader = std::io::BufReader::new(pipe);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("warning:") {
                    let _ = sender.send(trimmed.to_string());
                }
                full.extend_from_slice(line.as_bytes());
                line.clear();
            }
        }
        full
    })
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buffer);
        }
        buffer
    })
}

fn eval_package_info(session: &Session<'_>, flake_ref: &str) -> Result<PackageInfo> {
    let apply = r#"p:
      let
        meta = p.meta or {};
        stringOrFirst = value:
          if builtins.isString value then value
          else if builtins.isList value then
            let strings = builtins.filter builtins.isString value;
            in if strings == [] then null else builtins.head strings
          else null;
      in {
        pname = p.pname or (p.name or "unknown");
        version = p.version or "";
        meta = {
          description = stringOrFirst (meta.description or null);
          homepage = stringOrFirst (meta.homepage or null);
          changelog = stringOrFirst (meta.changelog or null);
        };
      }"#;

    let output = run_nix(
        session,
        &[
            "eval",
            "--extra-experimental-features",
            NIX_FEATURES,
            "--no-warn-dirty",
            "--json",
            flake_ref,
            "--apply",
            apply,
        ],
        "failed to run `nix eval` — is Nix installed?",
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "couldn't evaluate {flake_ref} — check that the flake reference and attribute exist\n{}",
            stderr.trim()
        );
    }

    serde_json::from_slice(&output.stdout).context("nix returned unexpected JSON")
}

fn target_version(version_spec: Option<&str>) -> Option<&str> {
    let version_spec = version_spec?;
    let version = version_spec
        .split_once("..")
        .map_or(version_spec, |(_, to)| to);
    let version = version.trim();
    (!version.is_empty() && !version.eq_ignore_ascii_case("latest")).then_some(version)
}

fn eval_direct_flake_packages(
    session: &Session<'_>,
    package: &str,
    version: Option<&str>,
    inputs: &[LockedFlakeInput],
) -> Result<PackageInfo> {
    let package_name = package;
    let package = serde_json::to_string(package_name)?;
    let version = serde_json::to_string(version.unwrap_or_default())?;
    let sources = serde_json::to_string(&serde_json::to_string(inputs)?)?;
    let expression = format!(
        r#"let
          package = {package};
          expectedVersion = {version};
          system = builtins.currentSystem;
          sources = map
            (source: {{
              flake = builtins.getFlake source.flake_ref;
              input = source.name;
            }})
            (builtins.fromJSON {sources});
          packagesFor = source:
            let
              packages = source.flake.packages.${{system}} or {{}};
              packageNames = builtins.attrNames packages;
              preferredNames = builtins.filter
                (name: name == package || name == "default")
                packageNames;
              remainingNames = builtins.filter
                (name: name != package && name != "default")
                packageNames;
            in map
              (name: {{
                package = packages.${{name}};
                flakeInput = source.input;
              }})
              (preferredNames ++ remainingNames);
          matchesFor = source: exact:
            let
              matches = builtins.filter (candidate:
                let result = builtins.tryEval (
                  builtins.isAttrs candidate.package &&
                  (candidate.package.pname or (candidate.package.name or "")) == package &&
                  (!exact || (candidate.package.version or "") == expectedVersion)
                );
                in result.success && result.value
              ) (packagesFor source);
            in if matches == [] then null else builtins.head matches;
          findMatch = remaining: exact:
            if remaining == [] then null
            else
              let found = matchesFor (builtins.head remaining) exact;
              in if found != null then found else findMatch (builtins.tail remaining) exact;
          exactMatch = if expectedVersion == "" then null else findMatch sources true;
          fallbackMatch = if exactMatch == null then findMatch sources false else null;
          selected =
            if exactMatch != null then exactMatch
            else if fallbackMatch != null then fallbackMatch
            else throw "package not found in the direct flake inputs";
          resolved = selected.package;
          meta = resolved.meta or {{}};
          stringOrFirst = value:
            if builtins.isString value then value
            else if builtins.isList value then
              let strings = builtins.filter builtins.isString value;
              in if strings == [] then null else builtins.head strings
            else null;
        in {{
          pname = resolved.pname or (resolved.name or "unknown");
          version = resolved.version or "";
          flake_input = selected.flakeInput;
          meta = {{
            description = stringOrFirst (meta.description or null);
            homepage = stringOrFirst (meta.homepage or null);
            changelog = stringOrFirst (meta.changelog or null);
          }};
        }}"#
    );
    let output = run_nix(
        session,
        &[
            "eval",
            "--extra-experimental-features",
            NIX_FEATURES,
            "--no-warn-dirty",
            "--impure",
            "--json",
            "--expr",
            &expression,
        ],
        "failed to inspect the direct flake inputs",
    )?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "couldn't resolve {package_name} from the direct flake inputs\n{}",
            stderr.trim()
        );
    }
    serde_json::from_slice(&output.stdout).context("nix returned unexpected JSON")
}

fn repository_from_locked(locked: &Value) -> Option<String> {
    match locked.get("type")?.as_str()? {
        "github" => Some(format!(
            "https://github.com/{}/{}",
            locked.get("owner")?.as_str()?,
            locked.get("repo")?.as_str()?
        )),
        "gitlab" => Some(format!(
            "https://gitlab.com/{}/{}",
            locked.get("owner")?.as_str()?,
            locked.get("repo")?.as_str()?
        )),
        "git" => locked.get("url")?.as_str().map(str::to_owned),
        _ => None,
    }
}

fn repository_from_lock(lock: &Value, input: &str) -> Option<String> {
    repository_from_locked(locked_input(lock, input)?)
}

fn locked_input<'a>(lock: &'a Value, input: &str) -> Option<&'a Value> {
    let root = lock.get("root")?.as_str()?;
    let node = lock
        .get("nodes")?
        .get(root)?
        .get("inputs")?
        .get(input)?
        .as_str()?;
    lock.get("nodes")?.get(node)?.get("locked")
}

fn flake_ref_from_locked(locked: &Value) -> Option<String> {
    match locked.get("type")?.as_str()? {
        "github" => Some(format!(
            "github:{}/{}/{}",
            locked.get("owner")?.as_str()?,
            locked.get("repo")?.as_str()?,
            locked.get("rev")?.as_str()?
        )),
        "gitlab" => Some(format!(
            "gitlab:{}/{}/{}",
            locked.get("owner")?.as_str()?,
            locked.get("repo")?.as_str()?,
            locked.get("rev")?.as_str()?
        )),
        "git" => {
            let url = locked.get("url")?.as_str()?;
            let separator = if url.contains('?') { '&' } else { '?' };
            Some(format!(
                "git+{url}{separator}rev={}",
                locked.get("rev")?.as_str()?
            ))
        }
        _ => None,
    }
}

fn flake_ref_from_lock(lock: &Value, input: &str) -> Option<String> {
    flake_ref_from_locked(locked_input(lock, input)?)
}

fn relevant_token(token: &str) -> bool {
    !token.is_empty() && !matches!(token, "nix" | "flake")
}

fn input_rank(package: &str, input: &str, repository: Option<&str>) -> usize {
    package
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| relevant_token(token))
        .filter(|package_token| {
            input
                .split(|c: char| !c.is_ascii_alphanumeric())
                .chain(
                    repository
                        .into_iter()
                        .flat_map(|repo| repo.split(|c: char| !c.is_ascii_alphanumeric())),
                )
                .filter(|token| relevant_token(token))
                .any(|input_token| input_token.eq_ignore_ascii_case(package_token))
        })
        .count()
}

fn direct_flake_inputs(lock: &Value, package: &str) -> Vec<LockedFlakeInput> {
    let Some(root) = lock.get("root").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Some(inputs) = lock
        .get("nodes")
        .and_then(|nodes| nodes.get(root))
        .and_then(|root| root.get("inputs"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let Some(nodes) = lock.get("nodes") else {
        return Vec::new();
    };

    let mut resolved = inputs
        .iter()
        .filter(|(name, _)| name.as_str() != "nixpkgs")
        .filter_map(|(name, node_name)| {
            let node = nodes.get(node_name.as_str()?)?;
            if node.get("flake").and_then(Value::as_bool) == Some(false) {
                return None;
            }
            let locked = node.get("locked")?;
            let flake_ref = flake_ref_from_locked(locked)?;
            let repository = repository_from_locked(locked);
            Some(LockedFlakeInput {
                rank: input_rank(package, name, repository.as_deref()),
                name: name.clone(),
                flake_ref,
                repository,
            })
        })
        .collect::<Vec<_>>();
    resolved.sort_by(|left, right| {
        right
            .rank
            .cmp(&left.rank)
            .then_with(|| left.name.cmp(&right.name))
    });
    resolved
}

fn matches_package(info: &PackageInfo, package: &str, version: Option<&str>) -> bool {
    info.pname == package
        && version
            .is_none_or(|version| normalize_version(&info.version) == normalize_version(version))
}

/// Evaluates one candidate reference. A mismatch or evaluation failure is recorded in `errors`
/// so resolution can move on; only cancellation is returned as an error.
fn try_candidate(
    session: &Session<'_>,
    flake_ref: &str,
    package: &str,
    version: Option<&str>,
    errors: &mut Vec<String>,
) -> Result<Option<PackageInfo>> {
    (session.progress)(&format!("Resolving {flake_ref}…"));
    match eval_package_info(session, flake_ref) {
        Ok(info) if matches_package(&info, package, version) => Ok(Some(info)),
        Ok(info) => {
            errors.push(format!(
                "{flake_ref} resolved to {} {}",
                info.pname, info.version
            ));
            Ok(None)
        }
        Err(error) if error.is::<Cancelled>() => Err(error),
        Err(error) => {
            errors.push(format!("{error:#}"));
            Ok(None)
        }
    }
}

/// Resolves `package` (bare name or flake ref with `#`). Bare names try the active flake, its
/// matching direct inputs, then nixpkgs, preferring the version `version_spec` targets.
pub fn resolve_package(
    session: &Session<'_>,
    package: &str,
    version_spec: Option<&str>,
) -> Result<PackageInfo> {
    if package.contains('#') {
        (session.progress)(&format!("Resolving {package}…"));
        return eval_package_info(session, package);
    }

    let version = target_version(version_spec);
    let lock = fs::read(session.flake_dir.join("flake.lock"))
        .ok()
        .and_then(|lock| serde_json::from_slice::<Value>(&lock).ok());
    let direct_inputs = lock
        .as_ref()
        .map_or_else(Vec::new, |lock| direct_flake_inputs(lock, package));
    let mut errors = Vec::new();
    let root_ref = format!(".#{package}");
    if let Some(info) = try_candidate(session, &root_ref, package, version, &mut errors)? {
        return Ok(info);
    }

    if let Some(lock) = lock.as_ref()
        && let Some(homepage) = repository_from_lock(lock, package)
    {
        return Ok(PackageInfo {
            pname: package.to_owned(),
            version: version.unwrap_or_default().to_owned(),
            meta: Meta {
                homepage: Some(homepage),
                ..Meta::default()
            },
            flake_input: Some(package.to_owned()),
        });
    }

    for input in direct_inputs.iter().take_while(|input| input.rank > 0) {
        let input_ref = format!("{}#{package}", input.flake_ref);
        (session.progress)(&format!("Checking direct input {}…", input.name));
        if let Some(mut info) = try_candidate(session, &input_ref, package, version, &mut errors)? {
            info.flake_input = Some(input.name.clone());
            if info.meta.homepage.is_none() {
                info.meta.homepage.clone_from(&input.repository);
            }
            return Ok(info);
        }
    }

    let nixpkgs_ref = lock
        .as_ref()
        .and_then(|lock| flake_ref_from_lock(lock, "nixpkgs"))
        .map_or_else(
            || format!("nixpkgs#{package}"),
            |flake_ref| format!("{flake_ref}#{package}"),
        );
    if let Some(info) = try_candidate(session, &nixpkgs_ref, package, version, &mut errors)? {
        return Ok(info);
    }

    if !nixpkgs_ref.starts_with("nixpkgs#") {
        let registry_ref = format!("nixpkgs#{package}");
        if let Some(info) = try_candidate(session, &registry_ref, package, version, &mut errors)? {
            return Ok(info);
        }
    }

    (session.progress)("Searching the remaining direct flake inputs…");
    match eval_direct_flake_packages(session, package, version, &direct_inputs) {
        Ok(mut info) => {
            if info.meta.homepage.is_none()
                && let Some(input) = direct_inputs
                    .iter()
                    .find(|input| Some(input.name.as_str()) == info.flake_input.as_deref())
            {
                info.meta.homepage.clone_from(&input.repository);
            }
            Ok(info)
        }
        Err(error) if error.is::<Cancelled>() => Err(error),
        Err(error) => {
            errors.push(format!("{error:#}"));
            bail!(
                "couldn't resolve {package} from the active flake, its inputs, or nixpkgs\n{}",
                errors.join("\n\n")
            )
        }
    }
}

pub fn fetch_text(session: &Session<'_>, url: &str) -> Result<String> {
    ensure_live(session)?;
    AGENT
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("failed to fetch {url}"))?
        .body_mut()
        .read_to_string()
        .context("failed to read response body")
}

/// Looks up a host's token from Nix's own `access-tokens` setting. `nix
/// show-config --json` reports it already parsed as a `{host: token}`
/// object (rather than the raw `host=token ...` string from nix.conf), with
/// an optional `type:` prefix on the token (e.g. GitLab's `PAT:`/`OAuth2:`).
fn nix_access_token(session: &Session<'_>, host: &str) -> Option<String> {
    let output = run_nix(
        session,
        &["show-config", "--json"],
        "failed to run `nix show-config`",
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let config: Value = serde_json::from_slice(&output.stdout).ok()?;
    let token = config
        .get("access-tokens")?
        .get("value")?
        .get(host)?
        .as_str()?;
    Some(token.split_once(':').map_or(token, |(_, t)| t).to_string())
}

fn github_token(session: &Session<'_>) -> Option<&'static str> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    if let Some(token) = TOKEN.get() {
        return token.as_deref();
    }
    let token = env::var("GITHUB_TOKEN")
        .ok()
        .or_else(|| nix_access_token(session, "github.com"));
    // A lookup cut short by cancellation says nothing about whether a token exists.
    if token.is_none() && (session.cancelled)() {
        return None;
    }
    TOKEN.get_or_init(|| token).as_deref()
}

pub fn fetch_json(session: &Session<'_>, url: &str) -> Result<Value> {
    let token = if url.starts_with("https://api.github.com") {
        github_token(session)
    } else {
        None
    };
    ensure_live(session)?;
    let mut req = AGENT.get(url).header("User-Agent", USER_AGENT);
    if let Some(token) = token {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    req.call()
        .with_context(|| format!("failed to fetch {url}"))?
        .body_mut()
        .read_json::<Value>()
        .context("failed to parse JSON response")
}

fn repo_ref_from_url(url: &str) -> Option<RepoRef> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let mut segments = parsed.path_segments()?;
    let owner = segments.next()?.to_string();
    let repo = segments.next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    // Reject URLs that look like documentation pages rather than forge repos.
    let is_file = |s: &str| {
        s.ends_with(".html")
            || s.ends_with(".htm")
            || s.ends_with(".xml")
            || s.ends_with(".php")
            || s.ends_with(".asp")
            || s.ends_with(".txt")
            || s.ends_with(".md")
            || s.ends_with(".json")
    };
    if is_file(&owner) || is_file(&repo) {
        return None;
    }
    Some(RepoRef { host, owner, repo })
}

fn normalize_version(v: &str) -> &str {
    v.strip_prefix(['v', 'V']).unwrap_or(v)
}

fn tag_matches(tag: &str, wanted: &str) -> bool {
    normalize_version(tag) == normalize_version(wanted)
}

fn parse_version_spec(raw: &str) -> VersionSpec {
    if let Some((from, to)) = raw.split_once("..") {
        let norm = |s: &str| {
            let s = s.trim();
            if s.is_empty() || s.eq_ignore_ascii_case("latest") {
                None
            } else {
                Some(s.to_string())
            }
        };
        VersionSpec::Range {
            from: norm(from),
            to: norm(to),
        }
    } else {
        VersionSpec::Single(raw.trim().to_string())
    }
}

/// Fetch one page of releases for a repo, normalizing across GitHub,
/// GitLab, and Forgejo/Gitea's differing API shapes.
fn fetch_releases_page(session: &Session<'_>, repo: &RepoRef, page: u32) -> Result<Vec<Release>> {
    (session.progress)(&format!(
        "Fetching releases for {}/{} (page {page}) from {}…",
        repo.owner, repo.repo, repo.host
    ));
    let (url, body_field) = match repo.host.as_str() {
        "github.com" => (
            format!(
                "https://api.github.com/repos/{}/{}/releases?per_page=100&page={page}",
                repo.owner, repo.repo
            ),
            "body",
        ),
        "gitlab.com" => (
            format!(
                "https://gitlab.com/api/v4/projects/{}%2F{}/releases?per_page=100&page={page}&order_by=released_at&sort=desc",
                repo.owner, repo.repo
            ),
            "description",
        ),
        host => (
            format!(
                "https://{host}/api/v1/repos/{}/{}/releases?limit=50&page={page}",
                repo.owner, repo.repo
            ),
            "body",
        ),
    };

    let json = fetch_json(session, &url)?;
    Ok(json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|v| Release {
            tag: v["tag_name"].as_str().unwrap_or_default().to_string(),
            body: v[body_field].as_str().unwrap_or_default().to_string(),
        })
        .collect())
}

fn find_release(session: &Session<'_>, repo: &RepoRef, wanted: &str) -> Result<Release> {
    (session.progress)(&format!(
        "Looking up release {} for {}/{}…",
        wanted, repo.owner, repo.repo
    ));
    for page in 1..=MAX_RELEASE_PAGES {
        let releases = fetch_releases_page(session, repo, page)?;
        if releases.is_empty() {
            break;
        }
        if let Some(r) = releases.into_iter().find(|r| tag_matches(&r.tag, wanted)) {
            return Ok(r);
        }
    }
    bail!(
        "couldn't find a release matching version {wanted} for {}/{}",
        repo.owner,
        repo.repo
    );
}

fn find_latest_release(session: &Session<'_>, repo: &RepoRef) -> Result<Release> {
    fetch_releases_page(session, repo, 1)?
        .into_iter()
        .next()
        .context("no releases found")
}

/// Collect every release after `from` up to and including `to`, newest-first.
fn collect_range(
    session: &Session<'_>,
    repo: &RepoRef,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<Release>> {
    let mut collecting = to.is_none();
    let mut found_to = to.is_none();
    let mut found_from = from.is_none();
    let mut collected = Vec::new();

    'outer: for page in 1..=MAX_RELEASE_PAGES {
        let releases = fetch_releases_page(session, repo, page)?;
        if releases.is_empty() {
            break;
        }
        for r in releases {
            if !collecting {
                match to {
                    Some(to_v) if tag_matches(&r.tag, to_v) => {
                        collecting = true;
                        found_to = true;
                    }
                    _ => continue,
                }
            }

            if from.is_some_and(|f| tag_matches(&r.tag, f)) {
                found_from = true;
                break 'outer;
            }
            collected.push(r);
        }
    }

    if !found_to {
        bail!(
            "couldn't find a release matching version {} for {}/{}",
            to.unwrap(),
            repo.owner,
            repo.repo
        );
    }
    if !found_from {
        bail!(
            "couldn't find a release matching version {} for {}/{}",
            from.unwrap(),
            repo.owner,
            repo.repo
        );
    }

    Ok(collected)
}

fn repo_ref_from_meta(meta: &Meta) -> Option<RepoRef> {
    meta.homepage
        .as_deref()
        .and_then(repo_ref_from_url)
        .or_else(|| meta.changelog.as_deref().and_then(repo_ref_from_url))
}

/// Fetches `meta.changelog` directly when it doesn't point at a forge repository (e.g. a
/// plain HTML release-notes page like mesa's).
fn fetch_direct_changelog(session: &Session<'_>, url: &str, tag: &str) -> Result<Release> {
    (session.progress)(&format!("Fetching changelog from {url}…"));
    let body = fetch_text(session, url)?;
    (session.progress)("Extracting changelog content…");
    let body = extract_html_content(&body);
    Ok(Release {
        tag: tag.to_string(),
        body,
    })
}

/// Extracts the meaningful content from a full HTML document. Looks for `<main>` or
/// `<article>` first, then `<body>`, and strips `<script>`, `<style>`, `<nav>`,
/// `<footer>`, and `<header>` blocks. Returns the input unchanged if it isn't HTML.
fn extract_html_content(html: &str) -> String {
    // Not HTML at all — return as-is (plain text or markdown changelogs).
    if !html.trim_start().starts_with('<') {
        return html.to_string();
    }

    // Pick the narrowest content container.
    let content = find_element_content(html, "main")
        .or_else(|| find_element_content(html, "article"))
        .or_else(|| find_element_content(html, "body"))
        .unwrap_or(html);

    // Strip non-content elements.
    let stripped = strip_elements(
        content,
        &["script", "style", "nav", "footer", "header", "noscript"],
    );

    // Strip headerlink and permalink anchors (e.g. Sphinx's `<a class="headerlink" ...>¶</a>`).
    strip_header_links(&stripped)
}

/// Removes header permalink anchors such as Sphinx's `<a class="headerlink" ...>¶</a>`,
/// GitHub/MkDocs anchor tags, and links consisting solely of pilcrow (`¶`) or `#` symbols.
fn strip_header_links(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<a") {
        let after = rest.as_bytes().get(start + 2).copied();
        if !after.is_some_and(|b| b.is_ascii_whitespace()) {
            result.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }
        let Some(tag_end) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start..start + tag_end + 1];
        let Some(close_end) = rest[start + tag_end + 1..].find("</a>") else {
            break;
        };
        let inner = &rest[start + tag_end + 1..start + tag_end + 1 + close_end];
        let full_end = start + tag_end + 1 + close_end + 4;

        let trimmed_inner = inner.trim();
        let is_headerlink = tag.contains("headerlink")
            || tag.contains("anchor")
            || tag.contains("hash-link")
            || trimmed_inner == "¶"
            || trimmed_inner == "&para;"
            || trimmed_inner == "&#182;"
            || (tag.contains("href=\"#") && (trimmed_inner == "#" || trimmed_inner.is_empty()));

        if is_headerlink {
            result.push_str(&rest[..start]);
            rest = &rest[full_end..];
        } else {
            result.push_str(&rest[..start + tag_end + 1]);
            rest = &rest[start + tag_end + 1..];
        }
    }
    result.push_str(rest);
    result
}

/// Returns the inner HTML of the first occurrence of `<tag ...>...</tag>`.
fn find_element_content<'a>(html: &'a str, tag: &str) -> Option<&'a str> {
    let open_pattern = format!("<{tag}");
    let close_pattern = format!("</{tag}>");

    let open_start = html
        .match_indices(&open_pattern)
        .find(|(i, _)| {
            // The character after the tag name must be whitespace or '>'.
            html.as_bytes()
                .get(i + open_pattern.len())
                .is_some_and(|&b| b == b'>' || b.is_ascii_whitespace())
        })?
        .0;

    // Skip past the opening tag's '>'.
    let content_start = open_start + html[open_start..].find('>')? + 1;

    // Find the matching close tag (no nesting support needed for these container elements).
    let content_end = html[content_start..].find(&close_pattern)? + content_start;

    Some(&html[content_start..content_end])
}

/// Removes every `<tag ...>...</tag>` block for each tag in `tags`. Handles nested
/// occurrences by tracking depth.
fn strip_elements(html: &str, tags: &[&str]) -> String {
    let mut result = html.to_string();
    for tag in tags {
        result = strip_one_element(&result, tag);
    }
    result
}

fn strip_one_element(html: &str, tag: &str) -> String {
    let open_pattern = format!("<{tag}");
    let close_pattern = format!("</{tag}>");
    let mut result = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open_pos) = rest.find(&open_pattern) {
        // Verify it's actually an element open (followed by '>' or whitespace).
        let after = rest.as_bytes().get(open_pos + open_pattern.len()).copied();
        if !after.is_some_and(|b| b == b'>' || b.is_ascii_whitespace()) {
            result.push_str(&rest[..open_pos + open_pattern.len()]);
            rest = &rest[open_pos + open_pattern.len()..];
            continue;
        }
        result.push_str(&rest[..open_pos]);
        rest = &rest[open_pos..];
        // Find the close, tracking nesting depth.
        let mut depth: u32 = 1;
        let mut cursor = rest[open_pattern.len()..]
            .find('>')
            .map(|i| i + open_pattern.len() + 1)
            .unwrap_or(rest.len());
        while depth > 0 {
            let next_open = rest[cursor..].find(&open_pattern).and_then(|i| {
                let after = rest
                    .as_bytes()
                    .get(cursor + i + open_pattern.len())
                    .copied();
                after
                    .is_some_and(|b| b == b'>' || b.is_ascii_whitespace())
                    .then_some(i)
            });
            let next_close = rest[cursor..].find(&close_pattern);
            match (next_open, next_close) {
                (Some(o), Some(c)) if o < c => {
                    depth += 1;
                    cursor += o + open_pattern.len();
                }
                (_, Some(c)) => {
                    depth -= 1;
                    cursor += c + close_pattern.len();
                }
                _ => {
                    // Unterminated — skip the rest.
                    cursor = rest.len();
                    break;
                }
            }
        }
        rest = &rest[cursor..];
    }
    result.push_str(rest);
    result
}

/// Releases for `version_spec` (single version, `latest`, or a `from..to` range) of an already
/// resolved package, fetched from the repository its metadata points at.
pub fn package_releases(
    session: &Session<'_>,
    info: PackageInfo,
    version_spec: &str,
) -> Result<Changelog> {
    if let Some(repo) = repo_ref_from_meta(&info.meta) {
        let releases = match parse_version_spec(version_spec) {
            VersionSpec::Single(v) if v.eq_ignore_ascii_case("latest") => {
                vec![find_latest_release(session, &repo)?]
            }
            VersionSpec::Single(v) => vec![find_release(session, &repo, &v)?],
            VersionSpec::Range { from, to } => {
                collect_range(session, &repo, from.as_deref(), to.as_deref())?
            }
        };
        return Ok(Changelog {
            pname: info.pname,
            version: info.version,
            description: info.meta.description,
            releases,
        });
    }

    // No forge repository detected — try fetching meta.changelog as a direct URL.
    if let Some(changelog_url) = info.meta.changelog.as_deref() {
        let release = fetch_direct_changelog(session, changelog_url, &info.version)
            .with_context(|| format!("failed to fetch changelog from {changelog_url}"))?;
        return Ok(Changelog {
            pname: info.pname,
            version: info.version,
            description: info.meta.description,
            releases: vec![release],
        });
    }

    // Neither a forge repository nor a direct changelog URL was available.
    match info.meta.homepage.as_deref() {
        Some(homepage) => bail!(
            "couldn't determine a release source for {}\n\
             • homepage ({homepage}) does not point to a supported Git forge (GitHub, GitLab, Forgejo/Codeberg)\n\
             • meta.changelog is not defined",
            info.pname
        ),
        None => bail!(
            "{} has no repository or changelog in its Nix metadata (meta.homepage and meta.changelog are unset)",
            info.pname
        ),
    }
}

/// Releases for `version_spec` (single version, `latest`, or a `from..to` range) of `package`
/// (bare name or flake ref with `#`).
pub fn releases(session: &Session<'_>, package: &str, version_spec: &str) -> Result<Changelog> {
    let info = resolve_package(session, package, Some(version_spec))?;
    package_releases(session, info, version_spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_the_target_version_for_package_resolution() {
        assert_eq!(target_version(Some("1.0..2.0")), Some("2.0"));
        assert_eq!(target_version(Some("2.0")), Some("2.0"));
        assert_eq!(target_version(Some("1.0..latest")), None);
        assert_eq!(target_version(None), None);
    }

    #[test]
    fn resolves_input_repository_from_the_lock() {
        let lock = serde_json::json!({
            "root": "root",
            "nodes": {
                "root": { "inputs": { "demo": "demo" } },
                "demo": {
                    "locked": {
                        "type": "github",
                        "owner": "example",
                        "repo": "demo",
                        "rev": "0123456789abcdef"
                    }
                }
            }
        });
        assert_eq!(
            repository_from_lock(&lock, "demo").as_deref(),
            Some("https://github.com/example/demo")
        );
        assert_eq!(
            flake_ref_from_lock(&lock, "demo").as_deref(),
            Some("github:example/demo/0123456789abcdef")
        );
    }

    #[test]
    fn prioritizes_matching_direct_flake_repositories() {
        let lock = serde_json::json!({
            "root": "root",
            "nodes": {
                "root": {
                    "inputs": {
                        "docs": "docs",
                        "nix-cachyos-kernel": "cachyos",
                        "nixpkgs": "nixpkgs"
                    }
                },
                "docs": {
                    "flake": false,
                    "locked": {
                        "type": "github",
                        "owner": "example",
                        "repo": "docs",
                        "rev": "1111111111111111"
                    }
                },
                "cachyos": {
                    "locked": {
                        "type": "github",
                        "owner": "krezh",
                        "repo": "nix-cachyos-kernel",
                        "rev": "2222222222222222"
                    }
                },
                "nixpkgs": {
                    "locked": {
                        "type": "github",
                        "owner": "NixOS",
                        "repo": "nixpkgs",
                        "rev": "3333333333333333"
                    }
                }
            }
        });

        let inputs = direct_flake_inputs(&lock, "linux-cachyos");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].name, "nix-cachyos-kernel");
        assert_eq!(inputs[0].rank, 1);
        assert_eq!(
            inputs[0].flake_ref,
            "github:krezh/nix-cachyos-kernel/2222222222222222"
        );
    }

    #[test]
    fn rejects_documentation_urls_as_repo_refs() {
        // Direct HTML changelog pages must not be parsed as forge repos.
        assert!(repo_ref_from_url("https://docs.mesa3d.org/relnotes/26.2.5.html").is_none());
        assert!(repo_ref_from_url("https://example.com/docs/notes.htm").is_none());
        assert!(repo_ref_from_url("https://example.com/api/schema.json").is_none());
        assert!(repo_ref_from_url("https://example.com/readme.md/stuff").is_none());
    }

    #[test]
    fn accepts_valid_forge_repo_urls() {
        let gh = repo_ref_from_url("https://github.com/mesa/mesa").unwrap();
        assert_eq!(gh.host, "github.com");
        assert_eq!(gh.owner, "mesa");
        assert_eq!(gh.repo, "mesa");

        let gl = repo_ref_from_url("https://gitlab.com/foo/bar").unwrap();
        assert_eq!(gl.host, "gitlab.com");
        assert_eq!(gl.owner, "foo");
        assert_eq!(gl.repo, "bar");

        // Forgejo / self-hosted with .git suffix
        let fg = repo_ref_from_url("https://codeberg.org/owner/repo.git").unwrap();
        assert_eq!(fg.repo, "repo");
    }

    #[test]
    fn repo_ref_from_meta_skips_doc_changelog() {
        // When homepage doesn't look like a repo and changelog is a doc page,
        // repo_ref_from_meta should return None.
        let meta = Meta {
            description: None,
            homepage: Some("https://www.mesa3d.org/".into()),
            changelog: Some("https://docs.mesa3d.org/relnotes/26.2.5.html".into()),
        };
        assert!(repo_ref_from_meta(&meta).is_none());
    }

    #[test]
    fn repo_ref_from_meta_uses_homepage_over_changelog() {
        let meta = Meta {
            description: None,
            homepage: Some("https://github.com/mesa/mesa".into()),
            changelog: Some("https://docs.mesa3d.org/relnotes/26.2.5.html".into()),
        };
        let r = repo_ref_from_meta(&meta).unwrap();
        assert_eq!(r.host, "github.com");
        assert_eq!(r.owner, "mesa");
        assert_eq!(r.repo, "mesa");
    }

    #[test]
    fn extracts_main_content_from_html_page() {
        let html = concat!(
            "<!DOCTYPE html><html><head><title>Notes</title>",
            "<style>body{}</style></head>",
            "<body><nav><a>Home</a></nav>",
            "<main class=\"body\">",
            "<h1>Release</h1><p>Fixed bugs.</p>",
            "</main>",
            "<footer>Copyright</footer></body></html>"
        );
        let extracted = extract_html_content(html);
        assert!(extracted.contains("<h1>Release</h1>"));
        assert!(extracted.contains("<p>Fixed bugs.</p>"));
        assert!(!extracted.contains("<nav>"));
        assert!(!extracted.contains("<footer>"));
        assert!(!extracted.contains("<style>"));
        assert!(!extracted.contains("<title>"));
    }

    #[test]
    fn falls_back_to_body_without_main() {
        let html = concat!(
            "<html><head><script>var x;</script></head>",
            "<body><h1>Notes</h1><p>Content</p></body></html>"
        );
        let extracted = extract_html_content(html);
        assert!(extracted.contains("<h1>Notes</h1>"));
        assert!(!extracted.contains("<script>"));
    }

    #[test]
    fn strips_nested_elements() {
        let html = "<body><nav><div><a>Link</a></div></nav><p>Real</p></body>";
        let extracted = extract_html_content(html);
        assert!(extracted.contains("<p>Real</p>"));
        assert!(!extracted.contains("<nav>"));
        assert!(!extracted.contains("Link"));
    }

    #[test]
    fn non_html_passes_through() {
        let plain = "# Changelog\n\n- Fixed bug\n- Added feature\n";
        assert_eq!(extract_html_content(plain), plain);
    }

    #[test]
    fn strips_headerlinks_and_pilcrows_from_headings() {
        let html = concat!(
            "<main>",
            "<h1>Mesa 26.2.5 Release Notes / 2026-10-14<a class=\"headerlink\" href=\"#mesa-26-2-5-release-notes-2026-10-14\" title=\"Link to this heading\">¶</a></h1>\n",
            "<h2>Bug fixes<a class=\"headerlink\" href=\"#bug-fixes\" title=\"Link to this heading\">¶</a></h2>\n",
            "<p>Check <a href=\"https://mesa3d.org\">website</a> for more.</p>",
            "</main>"
        );
        let extracted = extract_html_content(html);
        assert!(extracted.contains("<h1>Mesa 26.2.5 Release Notes / 2026-10-14</h1>"));
        assert!(extracted.contains("<h2>Bug fixes</h2>"));
        assert!(!extracted.contains('¶'));
        assert!(!extracted.contains("headerlink"));
        // Regular links are preserved.
        assert!(extracted.contains("<a href=\"https://mesa3d.org\">website</a>"));
    }

    #[test]
    fn reports_clear_error_when_no_repo_or_changelog_found() {
        let session = Session {
            flake_dir: std::path::Path::new("."),
            cancelled: &|| false,
            progress: &|_| (),
        };
        let info = PackageInfo {
            pname: "lix".into(),
            version: "2.95.3".into(),
            meta: Meta {
                description: None,
                homepage: Some("https://lix.systems".into()),
                changelog: None,
            },
            flake_input: None,
        };
        let err = package_releases(&session, info, "2.95.3")
            .unwrap_err()
            .to_string();
        assert!(err.contains("lix"));
        assert!(err.contains("https://lix.systems"));
        assert!(err.contains("meta.changelog is not defined"));

        let bare = PackageInfo {
            pname: "mystery".into(),
            version: "1.0".into(),
            meta: Meta::default(),
            flake_input: None,
        };
        let err2 = package_releases(&session, bare, "1.0")
            .unwrap_err()
            .to_string();
        assert!(err2.contains("mystery"));
        assert!(err2.contains("unset"));
    }
}
