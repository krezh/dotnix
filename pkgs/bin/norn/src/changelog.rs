use std::{env, process::Command, sync::OnceLock, time::Duration};

use eyre::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

const MAX_RELEASE_PAGES: u32 = 20;

#[derive(Deserialize, Default)]
struct Meta {
    homepage: Option<String>,
    changelog: Option<String>,
}

#[derive(Deserialize)]
struct PackageInfo {
    #[serde(default)]
    meta: Meta,
}

struct RepoRef {
    host: String,
    owner: String,
    repo: String,
}

struct Release {
    tag: String,
    body: String,
}

/// Reads a package's `meta` out of nixpkgs. Both fields are optional, so the
/// apply expression defaults every level rather than throwing on absence.
fn eval_meta(name: &str) -> Result<Meta> {
    let apply = "p: { meta = { \
                homepage = (p.meta or {}).homepage or null; \
                changelog = (p.meta or {}).changelog or null; \
              }; }";

    let output = Command::new("nix")
        .args([
            "eval",
            "--extra-experimental-features",
            "nix-command flakes",
            "--no-warn-dirty",
            "--json",
            &format!("nixpkgs#{name}"),
            "--apply",
            apply,
        ])
        .output()
        .context("failed to run `nix eval` — is Nix installed?")?;

    if !output.status.success() {
        bail!("no nixpkgs attribute named {name}");
    }

    let info: PackageInfo =
        serde_json::from_slice(&output.stdout).context("nix returned unexpected JSON")?;
    Ok(info.meta)
}

/// Looks up a host's token from Nix's own `access-tokens` setting, which is
/// reported already parsed as a `{host: token}` object, with an optional
/// `type:` prefix on the value (GitLab's `PAT:`/`OAuth2:`).
fn nix_access_token(host: &str) -> Option<String> {
    let output = Command::new("nix")
        .args(["config", "show", "--json"])
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
    Some(
        token
            .split_once(':')
            .map_or(token, |(_, rest)| rest)
            .to_owned(),
    )
}

fn github_token() -> Option<&'static str> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            env::var("GITHUB_TOKEN")
                .ok()
                .or_else(|| nix_access_token("github.com"))
        })
        .as_deref()
}

/// One shared agent, so every lookup is bounded: an unreachable forge has to
/// fail its row, not wedge it on "loading changelog…" forever.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(15)))
                .build(),
        )
    })
}

fn fetch_json(url: &str) -> Result<Value> {
    let mut request = agent().get(url).header("User-Agent", "norn");
    if url.starts_with("https://api.github.com")
        && let Some(token) = github_token()
    {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    request
        .call()
        .with_context(|| format!("failed to fetch {url}"))?
        .body_mut()
        .read_json::<Value>()
        .context("failed to parse JSON response")
}

fn fetch_text(url: &str) -> Result<String> {
    agent()
        .get(url)
        .header("User-Agent", "norn")
        .call()
        .with_context(|| format!("failed to fetch {url}"))?
        .body_mut()
        .read_to_string()
        .context("failed to read response body")
}

fn repo_ref_from_url(raw: &str) -> Option<RepoRef> {
    let parsed = Url::parse(raw).ok()?;
    let host = parsed.host_str()?.to_owned();
    let mut segments = parsed.path_segments()?;
    let owner = segments.next()?.to_owned();
    let repo = segments.next()?.trim_end_matches(".git").to_owned();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(RepoRef { host, owner, repo })
}

/// Drops a `v` prefix, but only where it introduces a version — `vim-9.1` is
/// named after the program, not tagged `v`.
fn normalize_version(version: &str) -> &str {
    version
        .strip_prefix(['v', 'V'])
        .filter(|rest| rest.starts_with(|ch: char| ch.is_ascii_digit()))
        .unwrap_or(version)
}

/// Whether a release tag denotes the version nixpkgs calls `wanted`.
///
/// Forges are inconsistent about what goes in front of the number: `1.8.2`,
/// `v1.8.2`, `jq-1.8.2`, `cli/v1.8.2`. A prefix is accepted only when a
/// separator divides it from the version, so `11.2.3` never answers for `1.2.3`.
fn tag_matches(tag: &str, wanted: &str) -> bool {
    let tag = tag.rsplit('/').next().unwrap_or(tag);
    let (tag, wanted) = (normalize_version(tag), normalize_version(wanted));

    tag == wanted
        || tag
            .strip_suffix(wanted)
            .and_then(|prefix| prefix.chars().next_back())
            .is_some_and(|last| matches!(last, '-' | '_'))
}

/// The tag named by a URL that points at one release, as nixpkgs' `changelog`
/// attribute usually does — `…/releases/tag/v1.2.3`, or GitLab's
/// `…/-/releases/v1.2.3`.
fn release_tag_from_url(raw: &str) -> Option<String> {
    let url = Url::parse(raw).ok()?;
    let segments: Vec<&str> = url.path_segments()?.collect();
    match segments.as_slice() {
        [_, _, "releases", "tag", tag, ..] | [_, _, "-", "releases", tag, ..] => {
            Some((*tag).to_owned())
        }
        _ => None,
    }
}

/// The field a forge puts release notes in.
fn body_field(host: &str) -> &'static str {
    if host == "gitlab.com" {
        "description"
    } else {
        "body"
    }
}

/// Fetches one page of releases, normalizing GitHub, GitLab and Forgejo/Gitea's
/// differing API shapes into a common form.
fn fetch_releases_page(repo: &RepoRef, page: u32) -> Result<Vec<Release>> {
    let url = match repo.host.as_str() {
        "github.com" => format!(
            "https://api.github.com/repos/{owner}/{repo}/releases?per_page=100&page={page}",
            owner = repo.owner,
            repo = repo.repo
        ),
        "gitlab.com" => format!(
            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases?per_page=100&page={page}&order_by=released_at&sort=desc",
            owner = repo.owner,
            repo = repo.repo
        ),
        host => format!(
            "https://{host}/api/v1/repos/{owner}/{repo}/releases?limit=50&page={page}",
            owner = repo.owner,
            repo = repo.repo
        ),
    };

    let field = body_field(&repo.host);
    let json = fetch_json(&url)?;
    Ok(json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|value| Release {
            tag: value["tag_name"].as_str().unwrap_or_default().to_owned(),
            body: value[field].as_str().unwrap_or_default().to_owned(),
        })
        .collect())
}

/// Fetches the single release a tag names.
///
/// nixpkgs' `meta.changelog` usually points straight at the release for the
/// version it packages, which is both exact and one request — no paging the
/// history hoping a tag will match.
fn release_by_tag(repo: &RepoRef, tag: &str) -> Result<Release> {
    let url = match repo.host.as_str() {
        "github.com" => format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}",
            owner = repo.owner,
            repo = repo.repo
        ),
        "gitlab.com" => format!(
            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases/{tag}",
            owner = repo.owner,
            repo = repo.repo
        ),
        host => format!(
            "https://{host}/api/v1/repos/{owner}/{repo}/releases/tags/{tag}",
            owner = repo.owner,
            repo = repo.repo
        ),
    };

    let json = fetch_json(&url)?;
    let body = json[body_field(&repo.host)]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if body.trim().is_empty() {
        bail!("release {tag} has no notes");
    }
    Ok(Release {
        tag: tag.to_owned(),
        body,
    })
}

/// Collects the releases an upgrade actually brings in: everything after `old`
/// up to and including `new`. `old` itself is excluded — you were already
/// running it.
///
/// An empty `old` means the package is newly installed, so there is no span to
/// walk: just its own release notes.
fn releases_between(repo: &RepoRef, old: &str, new: &str) -> Result<Vec<Release>> {
    let mut collecting = false;
    let mut collected = Vec::new();

    'pages: for page in 1..=MAX_RELEASE_PAGES {
        let releases = fetch_releases_page(repo, page)?;
        if releases.is_empty() {
            break;
        }

        for release in releases {
            if !collecting {
                if tag_matches(&release.tag, new) {
                    collecting = true;
                } else {
                    continue;
                }
            }

            if old.is_empty() {
                collected.push(release);
                break 'pages;
            }
            if tag_matches(&release.tag, old) {
                break 'pages;
            }
            collected.push(release);
        }
    }

    if collected.is_empty() {
        bail!("no releases tagged between {old} and {new}");
    }

    collected.reverse();
    Ok(collected)
}

fn render(releases: &[Release]) -> String {
    let mut markdown = String::new();
    for release in releases {
        markdown.push_str(&format!("# {tag}\n\n", tag = release.tag));
        let body = release.body.trim();
        if body.is_empty() {
            markdown.push_str("_No release notes._\n\n");
        } else {
            markdown.push_str(body);
            markdown.push_str("\n\n");
        }
    }
    markdown
}

/// Number of `#`s a markdown heading opens with; zero for anything else.
fn heading_level(line: &str) -> usize {
    line.chars().take_while(|&ch| ch == '#').count()
}

/// Whether a heading names exactly this version. A plain substring test would
/// let `## 11.2.3` answer for `1.2.3`, so the match has to stand alone: nothing
/// may continue the number on either side, and nothing may extend it into a
/// different release (`1.2.3-rc1` is not `1.2.3`). A leading `v` is allowed,
/// being how half of all changelogs write a version.
fn heading_names(line: &str, version: &str) -> bool {
    if version.is_empty() || heading_level(line) == 0 {
        return false;
    }
    line.match_indices(version).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + version.len()..].chars().next();
        !before
            .is_some_and(|ch| (ch.is_ascii_alphanumeric() && !matches!(ch, 'v' | 'V')) || ch == '.')
            && !after.is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '+'))
    })
}

/// Narrows a whole CHANGELOG.md to the span an upgrade actually covers.
///
/// These files run newest-first under a heading per release, so the span runs
/// from the heading naming `new` down to the one naming `old` — the versions
/// being installed, with the one already running excluded. Where `old` does not
/// appear (a new install, or a file that does not go back that far) the section
/// for `new` alone beats handing back the entire history.
fn slice_changelog(markdown: &str, old: &str, new: &str) -> Option<String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let start = lines.iter().position(|line| heading_names(line, new))?;
    let level = heading_level(lines[start]);

    let rest = &lines[start + 1..];
    let end = rest
        .iter()
        .position(|line| heading_names(line, old))
        .or_else(|| rest.iter().position(|line| heading_level(line) == level))
        .map_or(lines.len(), |offset| start + 1 + offset);

    Some(lines[start..end].join("\n"))
}

/// Fetches whatever `meta.changelog` points at, when it points at something
/// renderable. GitHub's `blob` and `raw` paths are HTML viewers; the file
/// itself lives on `raw.githubusercontent.com`.
fn changelog_text(raw: &str) -> Option<String> {
    if let Ok(url) = Url::parse(raw)
        && url.host_str() == Some("github.com")
    {
        let segments: Vec<&str> = url.path_segments().map(Iterator::collect)?;
        if let [owner, repo, "blob" | "raw", git_ref, rest @ ..] = segments.as_slice() {
            return fetch_text(&format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{path}",
                path = rest.join("/")
            ))
            .ok();
        }
    }

    if raw.ends_with(".md") || raw.ends_with(".markdown") {
        return fetch_text(raw).ok();
    }

    None
}

/// Last resort when a project publishes no matching releases: the changelog
/// file, narrowed to the versions in question. A bare link is the end of the
/// line — it is what the reader could have found for themselves.
fn changelog_file(meta: &Meta, repo: Option<&RepoRef>, old: &str, new: &str) -> Option<String> {
    let text = meta
        .changelog
        .as_deref()
        .and_then(changelog_text)
        // Plenty of projects keep a CHANGELOG.md and never mention it in
        // `meta.changelog`; it sits at a predictable place in the repository.
        .or_else(|| {
            let repo = repo.filter(|repo| repo.host == "github.com")?;
            fetch_text(&format!(
                "https://raw.githubusercontent.com/{owner}/{name}/HEAD/CHANGELOG.md",
                owner = repo.owner,
                name = repo.repo
            ))
            .ok()
        });

    if let Some(text) = text {
        return Some(slice_changelog(&text, old, new).unwrap_or(text));
    }

    let raw = meta.changelog.as_deref()?;
    Some(format!("Changelog lives at <{raw}>.\n"))
}

/// Resolves a package's release notes for an upgrade, as markdown.
pub fn markdown_for(name: &str, old: &str, new: &str) -> Result<String> {
    let meta = eval_meta(name)?;

    let repo = meta
        .changelog
        .as_deref()
        .and_then(repo_ref_from_url)
        .or_else(|| meta.homepage.as_deref().and_then(repo_ref_from_url));
    let tag = meta.changelog.as_deref().and_then(release_tag_from_url);

    if let Some(repo) = &repo {
        // A fresh install spans nothing, so the one release nixpkgs points at
        // is the whole answer — no reason to page the history for it.
        if old.is_empty()
            && let Some(tag) = &tag
            && let Ok(release) = release_by_tag(repo, tag)
        {
            return Ok(render(&[release]));
        }

        if let Ok(releases) = releases_between(repo, old, new) {
            return Ok(render(&releases));
        }

        // The span walk matches tags to nixpkgs version strings and plenty of
        // projects tag in a shape it cannot reach (`jq-1.8.2` for 1.8.2, say).
        // The tag nixpkgs recorded needs no guessing.
        if let Some(tag) = &tag
            && let Ok(release) = release_by_tag(repo, tag)
        {
            return Ok(render(&[release]));
        }
    }

    changelog_file(&meta, repo.as_ref(), old, new)
        .ok_or_else(|| eyre::eyre!("no changelog or homepage metadata for {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape that sent `jq` to a bare link: nixpkgs says 1.8.2, upstream
    /// tags `jq-1.8.2`.
    #[test]
    fn tags_match_across_the_usual_prefixes() {
        for (tag, wanted) in [
            ("1.8.2", "1.8.2"),
            ("v1.8.2", "1.8.2"),
            ("jq-1.8.2", "1.8.2"),
            ("release_2024.1", "2024.1"),
            ("cli/v1.8.2", "1.8.2"),
            ("v1.8.2", "v1.8.2"),
        ] {
            assert!(tag_matches(tag, wanted), "{tag} should match {wanted}");
        }
    }

    #[test]
    fn a_longer_version_is_not_a_match() {
        for (tag, wanted) in [
            ("11.2.3", "1.2.3"),
            ("1.2.30", "1.2.3"),
            ("v2.0.0", "1.0.0"),
            ("vim-9.1", "9.2"),
        ] {
            assert!(!tag_matches(tag, wanted), "{tag} must not match {wanted}");
        }
    }

    /// `vim-9.1` is named after the program; stripping its `v` would leave
    /// `im-9.1` and match nothing.
    #[test]
    fn a_leading_v_is_only_stripped_from_a_version() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("vim-9.1"), "vim-9.1");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }

    #[test]
    fn release_urls_yield_their_tag() {
        assert_eq!(
            release_tag_from_url("https://github.com/jqlang/jq/releases/tag/jq-1.8.2").as_deref(),
            Some("jq-1.8.2")
        );
        assert_eq!(
            release_tag_from_url("https://gitlab.com/o/r/-/releases/v1.2.3").as_deref(),
            Some("v1.2.3")
        );
        assert_eq!(
            release_tag_from_url("https://github.com/helix-editor/helix/blob/25.07.1/CHANGELOG.md"),
            None
        );
    }

    const CHANGELOG: &str = "\
# Changelog

## 11.0.0
newest, unrelated

## 1.2.0
- added a thing
- fixed another

## 1.1.0
- an intermediate release

## 1.0.0
- the version already installed

## 0.9.0
- ancient history
";

    /// An upgrade shows every version it brings in, and stops at the one that
    /// was already running.
    #[test]
    fn an_upgrade_spans_the_versions_it_installs() {
        let slice = slice_changelog(CHANGELOG, "1.0.0", "1.2.0").expect("1.2.0 is in the file");
        assert!(slice.starts_with("## 1.2.0"), "{slice}");
        assert!(slice.contains("an intermediate release"), "{slice}");
        assert!(!slice.contains("already installed"), "{slice}");
        assert!(!slice.contains("newest, unrelated"), "{slice}");
    }

    /// With no previous version there is no span, so the section stands alone.
    #[test]
    fn an_install_shows_only_its_own_section() {
        let slice = slice_changelog(CHANGELOG, "", "1.1.0").expect("1.1.0 is in the file");
        assert_eq!(slice, "## 1.1.0\n- an intermediate release\n");
    }

    /// `## 11.0.0` must not be mistaken for `1.0.0`.
    #[test]
    fn a_heading_matches_only_its_own_version() {
        assert!(heading_names("## 1.0.0", "1.0.0"));
        assert!(heading_names("## v1.0.0 — codename", "1.0.0"));
        assert!(!heading_names("## 11.0.0", "1.0.0"));
        assert!(!heading_names("## 1.0.0-rc1", "1.0.0"));
        assert!(!heading_names("released 1.0.0", "1.0.0"), "not a heading");
    }

    #[test]
    fn an_unknown_version_slices_nothing() {
        assert!(slice_changelog(CHANGELOG, "1.0.0", "7.7.7").is_none());
    }
}
