use std::process::{self, Command};
use std::{env, fs};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// Pretty-print a package's changelog, resolved from its Nix flake metadata.
#[derive(Parser)]
#[command(name = "nix-changelog", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    /// Flake reference; bare names try the active flake before nixpkgs
    flake_ref: Option<String>,

    /// A specific version (e.g. 1.2.3, latest) or a range (1.0.0..latest, 1.0.0.., ..2.0.0)
    #[arg(value_name = "VERSION_OR_RANGE")]
    version_spec: Option<String>,

    /// Emit structured release data for graphical clients.
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a shell completion script
    Completion { shell: Shell },
}

#[derive(Deserialize, Default)]
struct Meta {
    description: Option<String>,
    homepage: Option<String>,
    changelog: Option<String>,
}

#[derive(Deserialize)]
struct PackageInfo {
    pname: String,
    version: String,
    #[serde(default)]
    meta: Meta,
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

#[derive(Serialize)]
struct Release {
    tag: String,
    body: String,
}

#[derive(Serialize)]
struct ChangelogOutput<'a> {
    pname: &'a str,
    version: &'a str,
    description: Option<&'a str>,
    releases: &'a [Release],
}

enum VersionSpec {
    Single(String),
    Range {
        from: Option<String>,
        to: Option<String>,
    },
}

fn eval_package_info(flake_ref: &str) -> Result<PackageInfo> {
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

    let output = Command::new("nix")
        .args([
            "eval",
            "--extra-experimental-features",
            "nix-command flakes",
            "--no-warn-dirty",
            "--json",
            flake_ref,
            "--apply",
            apply,
        ])
        .output()
        .context("failed to run `nix eval` — is Nix installed?")?;

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
    let output = Command::new("nix")
        .args([
            "eval",
            "--extra-experimental-features",
            "nix-command flakes",
            "--no-warn-dirty",
            "--impure",
            "--json",
            "--expr",
            &expression,
        ])
        .output()
        .context("failed to inspect the direct flake inputs")?;
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

fn resolve_package_info(raw_ref: &str, version_spec: Option<&str>) -> Result<PackageInfo> {
    if raw_ref.contains('#') {
        eprintln!("Resolving {raw_ref}…");
        return eval_package_info(raw_ref);
    }

    let version = target_version(version_spec);
    let lock = fs::read("flake.lock")
        .ok()
        .and_then(|lock| serde_json::from_slice::<Value>(&lock).ok());
    let direct_inputs = lock
        .as_ref()
        .map_or_else(Vec::new, |lock| direct_flake_inputs(lock, raw_ref));
    let mut errors = Vec::new();
    let root_ref = format!(".#{raw_ref}");
    eprintln!("Resolving {root_ref}…");
    match eval_package_info(&root_ref) {
        Ok(info) if matches_package(&info, raw_ref, version) => return Ok(info),
        Ok(info) => errors.push(format!(
            "{root_ref} resolved to {} {}",
            info.pname, info.version
        )),
        Err(error) => errors.push(format!("{error:#}")),
    }

    if let Some(lock) = lock.as_ref()
        && let Some(homepage) = repository_from_lock(lock, raw_ref)
    {
        return Ok(PackageInfo {
            pname: raw_ref.to_owned(),
            version: version.unwrap_or_default().to_owned(),
            meta: Meta {
                homepage: Some(homepage),
                ..Meta::default()
            },
            flake_input: Some(raw_ref.to_owned()),
        });
    }

    for input in direct_inputs.iter().take_while(|input| input.rank > 0) {
        let input_ref = format!("{}#{raw_ref}", input.flake_ref);
        eprintln!("Resolving {input_ref}…");
        match eval_package_info(&input_ref) {
            Ok(mut info) if matches_package(&info, raw_ref, version) => {
                info.flake_input = Some(input.name.clone());
                if info.meta.homepage.is_none() {
                    info.meta.homepage.clone_from(&input.repository);
                }
                return Ok(info);
            }
            Ok(info) => errors.push(format!(
                "{input_ref} resolved to {} {}",
                info.pname, info.version
            )),
            Err(error) => errors.push(format!("{error:#}")),
        }
    }

    let nixpkgs_ref = lock
        .as_ref()
        .and_then(|lock| flake_ref_from_lock(lock, "nixpkgs"))
        .map_or_else(
            || format!("nixpkgs#{raw_ref}"),
            |flake_ref| format!("{flake_ref}#{raw_ref}"),
        );
    eprintln!("Resolving {nixpkgs_ref}…");
    match eval_package_info(&nixpkgs_ref) {
        Ok(info) if matches_package(&info, raw_ref, version) => return Ok(info),
        Ok(info) => errors.push(format!(
            "{nixpkgs_ref} resolved to {} {}",
            info.pname, info.version
        )),
        Err(error) => errors.push(format!("{error:#}")),
    }

    if !nixpkgs_ref.starts_with("nixpkgs#") {
        let registry_ref = format!("nixpkgs#{raw_ref}");
        eprintln!("Resolving {registry_ref}…");
        match eval_package_info(&registry_ref) {
            Ok(info) if matches_package(&info, raw_ref, version) => return Ok(info),
            Ok(info) => errors.push(format!(
                "{registry_ref} resolved to {} {}",
                info.pname, info.version
            )),
            Err(error) => errors.push(format!("{error:#}")),
        }
    }

    eprintln!("Searching the remaining direct flake inputs…");
    match eval_direct_flake_packages(raw_ref, version, &direct_inputs) {
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
        Err(error) => {
            errors.push(format!("{error:#}"));
            bail!(
                "couldn't resolve {raw_ref} from the active flake, its inputs, or nixpkgs\n{}",
                errors.join("\n\n")
            )
        }
    }
}

fn print_box(text: &str) {
    let width = text.chars().count() + 2;
    let bar = "─".repeat(width);
    println!("\x1b[1;35m╭{bar}╮\x1b[0m");
    println!("\x1b[1;35m│ {text} │\x1b[0m");
    println!("\x1b[1;35m╰{bar}╯\x1b[0m");
}

fn print_header(info: &PackageInfo) {
    let title = if info.version.is_empty() {
        info.pname.clone()
    } else {
        format!("{} {}", info.pname, info.version)
    };
    print_box(&title);
    println!();
    if let Some(desc) = &info.meta.description {
        println!("\x1b[3;90m{desc}\x1b[0m\n");
    }
}

fn render_markdown(markdown: &str) -> Result<()> {
    use glamour::{Renderer, Style};

    let width = terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80);

    let renderer = Renderer::new()
        .with_style(Style::Dark)
        .with_word_wrap(width);
    print!("{}", renderer.render(markdown));
    Ok(())
}

fn is_plaintext_release_header(line: &str) -> bool {
    let Some((_, version)) = line.rsplit_once(" - v") else {
        return false;
    };
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn plaintext_changelog_to_markdown(changelog: &str) -> Option<String> {
    let has_release_header = changelog.lines().any(is_plaintext_release_header);
    let has_indented_bullet = changelog
        .lines()
        .any(|line| line.starts_with(char::is_whitespace) && line.trim_start().starts_with("- "));
    if !has_release_header || !has_indented_bullet {
        return None;
    }

    let mut markdown = String::with_capacity(changelog.len());
    for line in changelog.lines() {
        let trimmed = line.trim();
        if is_plaintext_release_header(line) {
            if !markdown.ends_with("\n\n") && !markdown.is_empty() {
                markdown.push('\n');
            }
            markdown.push_str("## ");
            for c in line.chars() {
                if matches!(c, '<' | '>') {
                    markdown.push('\\');
                }
                markdown.push(c);
            }
            markdown.push_str("\n\n");
        } else if trimmed.starts_with("- ") {
            markdown.push_str(trimmed);
            markdown.push('\n');
        } else if trimmed.is_empty() {
            if !markdown.ends_with("\n\n") {
                markdown.push('\n');
            }
        } else {
            markdown.push_str("  ");
            markdown.push_str(trimmed);
            markdown.push('\n');
        }
    }
    Some(markdown)
}

fn render_changelog(changelog: &str) -> Result<()> {
    match plaintext_changelog_to_markdown(changelog) {
        Some(markdown) => render_markdown(&markdown),
        None => render_markdown(changelog),
    }
}

fn fetch_text(url: &str) -> Result<String> {
    ureq::get(url)
        .header("User-Agent", "nix-changelog")
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
fn nix_access_token(host: &str) -> Option<String> {
    let output = Command::new("nix")
        .args(["show-config", "--json"])
        .output()
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

fn github_token() -> Option<&'static str> {
    static TOKEN: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    TOKEN
        .get_or_init(|| {
            env::var("GITHUB_TOKEN")
                .ok()
                .or_else(|| nix_access_token("github.com"))
        })
        .as_deref()
}

fn fetch_json(url: &str) -> Result<Value> {
    let mut req = ureq::get(url).header("User-Agent", "nix-changelog");
    if url.starts_with("https://api.github.com") {
        if let Some(token) = github_token() {
            req = req.header("Authorization", &format!("Bearer {token}"));
        }
    }
    req.call()
        .with_context(|| format!("failed to fetch {url}"))?
        .body_mut()
        .read_json::<Value>()
        .context("failed to parse JSON response")
}

fn render_release_body(api: &str, field: &str, fallback_url: &str) -> Result<()> {
    let body = fetch_json(api)
        .ok()
        .and_then(|v| v[field].as_str().map(str::to_string))
        .filter(|s| !s.trim().is_empty());

    match body {
        Some(body) => render_markdown(&body),
        None => {
            println!("No release notes found — see:");
            print_box(fallback_url);
            Ok(())
        }
    }
}

/// Resolve a changelog/homepage URL to rendered text.
fn show_changelog(raw_url: &str) -> Result<()> {
    if let Ok(url) = Url::parse(raw_url) {
        let host = url.host_str().unwrap_or_default().to_string();
        let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

        if host == "github.com" {
            if let [owner, repo, "blob", git_ref, rest @ ..] = segments.as_slice() {
                let raw = format!(
                    "https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{}",
                    rest.join("/")
                );
                return render_changelog(&fetch_text(&raw)?);
            }
            if let [owner, repo, "releases", "tag", tag] = segments.as_slice() {
                let api =
                    format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
                return render_release_body(&api, "body", raw_url);
            }
            if let [owner, repo, "releases"] = segments.as_slice() {
                let api = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
                return render_release_body(&api, "body", raw_url);
            }
        }

        if host == "gitlab.com" {
            if let [owner, repo, "-", "releases", tag] = segments.as_slice() {
                let api =
                    format!("https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases/{tag}");
                return render_release_body(&api, "description", raw_url);
            }
        }

        // Generic Forgejo/Gitea-style release tag page (e.g. codeberg.org),
        // sharing the GitHub API shape under /api/v1/repos/...
        if let [owner, repo, "releases", "tag", tag] = segments.as_slice() {
            let api = format!("https://{host}/api/v1/repos/{owner}/{repo}/releases/tags/{tag}");
            return render_release_body(&api, "body", raw_url);
        }
    }

    if raw_url.ends_with(".md") || raw_url.ends_with(".markdown") {
        return render_markdown(&fetch_text(raw_url)?);
    }

    println!("No structured changelog renderer matched this URL:");
    print_box(raw_url);
    Ok(())
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
fn fetch_releases_page(repo: &RepoRef, page: u32) -> Result<Vec<Release>> {
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

    let json = fetch_json(&url)?;
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

const MAX_RELEASE_PAGES: u32 = 20;

fn find_release(repo: &RepoRef, wanted: &str) -> Result<Release> {
    for page in 1..=MAX_RELEASE_PAGES {
        let releases = fetch_releases_page(repo, page)?;
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

fn find_latest_release(repo: &RepoRef) -> Result<Release> {
    fetch_releases_page(repo, 1)?
        .into_iter()
        .next()
        .context("no releases found")
}

/// Collect every release between `from` and `to`, newest-first and inclusive.
fn collect_range(repo: &RepoRef, from: Option<&str>, to: Option<&str>) -> Result<Vec<Release>> {
    let mut collecting = to.is_none();
    let mut found_to = to.is_none();
    let mut found_from = from.is_none();
    let mut collected = Vec::new();

    'outer: for page in 1..=MAX_RELEASE_PAGES {
        let releases = fetch_releases_page(repo, page)?;
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

            let is_from = from.is_some_and(|f| tag_matches(&r.tag, f));
            collected.push(r);
            if is_from {
                found_from = true;
                break 'outer;
            }
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

fn render_releases(releases: &[Release]) -> Result<()> {
    if releases.is_empty() {
        println!("No releases found in that range.");
        return Ok(());
    }

    let mut combined = String::new();
    for r in releases {
        combined.push_str(&format!("# {}\n\n", r.tag));
        combined.push_str(r.body.trim());
        combined.push_str("\n\n---\n\n");
    }
    render_markdown(&combined)
}

fn output_releases(info: &PackageInfo, releases: &[Release], json: bool) -> Result<()> {
    if json {
        serde_json::to_writer(
            std::io::stdout(),
            &ChangelogOutput {
                pname: &info.pname,
                version: &info.version,
                description: info.meta.description.as_deref(),
                releases,
            },
        )?;
        println!();
        Ok(())
    } else {
        render_releases(releases)
    }
}

fn repo_ref_from_meta(meta: &Meta) -> Option<RepoRef> {
    meta.homepage
        .as_deref()
        .and_then(repo_ref_from_url)
        .or_else(|| meta.changelog.as_deref().and_then(repo_ref_from_url))
}

fn run(raw_ref: &str, version_spec: Option<&str>, json: bool) -> Result<()> {
    let info = resolve_package_info(raw_ref, version_spec)?;

    if !json {
        print_header(&info);
    }

    let Some(raw_spec) = version_spec else {
        if json {
            bail!("--json requires a version or range");
        }
        let source_url = info
            .meta
            .changelog
            .clone()
            .or_else(|| info.meta.homepage.clone());
        let Some(url) = source_url else {
            bail!("no changelog or homepage metadata found for {}", info.pname);
        };
        return show_changelog(&url);
    };

    let repo = repo_ref_from_meta(&info.meta).with_context(|| {
        format!(
            "couldn't determine the source repository for {} from its metadata",
            info.pname
        )
    })?;

    let releases = match parse_version_spec(raw_spec) {
        VersionSpec::Single(v) if v.eq_ignore_ascii_case("latest") => {
            vec![find_latest_release(&repo)?]
        }
        VersionSpec::Single(v) => vec![find_release(&repo, &v)?],
        VersionSpec::Range { from, to } => collect_range(&repo, from.as_deref(), to.as_deref())?,
    };
    output_releases(&info, &releases, json)
}

fn main() {
    let cli = Cli::parse();

    if let Some(Cmd::Completion { shell }) = cli.command {
        generate(
            shell,
            &mut Cli::command(),
            "nix-changelog",
            &mut std::io::stdout(),
        );
        return;
    }

    let Some(raw_ref) = cli.flake_ref else {
        let _ = Cli::command().print_help();
        println!();
        process::exit(1);
    };

    if let Err(err) = run(&raw_ref, cli.version_spec.as_deref(), cli.json) {
        eprintln!("\x1b[1;31m✗\x1b[0m {err:#}");
        process::exit(1);
    }
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
    fn parses_plaintext_release_entries_as_markdown() {
        let changelog = "\
Tue Jul 07th 2026 - maintainer <maintainer@example.com> - v5.2.3
    - BUGFIX: Fix MIME header encoding
    - WEBUI: Escape CSS selectors

Mon Jun 15th 2026 - maintainer <maintainer@example.com> - v5.2.2
    - FEATURE: Show files in file managers
";

        assert_eq!(
            plaintext_changelog_to_markdown(changelog).as_deref(),
            Some(
                "\
## Tue Jul 07th 2026 - maintainer \\<maintainer@example.com\\> - v5.2.3

- BUGFIX: Fix MIME header encoding
- WEBUI: Escape CSS selectors

## Mon Jun 15th 2026 - maintainer \\<maintainer@example.com\\> - v5.2.2

- FEATURE: Show files in file managers
"
            )
        );
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
}
