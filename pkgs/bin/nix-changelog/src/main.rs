use std::path::Path;
use std::process;

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use nix_changelog::{
    PackageInfo, Release, Session, fetch_json, fetch_text, package_releases, resolve_package,
};
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

fn render_release_body(
    session: &Session<'_>,
    api: &str,
    field: &str,
    fallback_url: &str,
) -> Result<()> {
    let body = fetch_json(session, api)
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
fn show_changelog(session: &Session<'_>, raw_url: &str) -> Result<()> {
    if let Ok(url) = Url::parse(raw_url) {
        let host = url.host_str().unwrap_or_default().to_string();
        let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

        if host == "github.com" {
            if let [owner, repo, "blob", git_ref, rest @ ..] = segments.as_slice() {
                let raw = format!(
                    "https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{}",
                    rest.join("/")
                );
                return render_changelog(&fetch_text(session, &raw)?);
            }
            if let [owner, repo, "releases", "tag", tag] = segments.as_slice() {
                let api =
                    format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
                return render_release_body(session, &api, "body", raw_url);
            }
            if let [owner, repo, "releases"] = segments.as_slice() {
                let api = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
                return render_release_body(session, &api, "body", raw_url);
            }
        }

        if host == "gitlab.com" {
            if let [owner, repo, "-", "releases", tag] = segments.as_slice() {
                let api =
                    format!("https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases/{tag}");
                return render_release_body(session, &api, "description", raw_url);
            }
        }

        // Generic Forgejo/Gitea-style release tag page (e.g. codeberg.org),
        // sharing the GitHub API shape under /api/v1/repos/...
        if let [owner, repo, "releases", "tag", tag] = segments.as_slice() {
            let api = format!("https://{host}/api/v1/repos/{owner}/{repo}/releases/tags/{tag}");
            return render_release_body(session, &api, "body", raw_url);
        }
    }

    if raw_url.ends_with(".md") || raw_url.ends_with(".markdown") {
        return render_markdown(&fetch_text(session, raw_url)?);
    }

    println!("No structured changelog renderer matched this URL:");
    print_box(raw_url);
    Ok(())
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

fn run(raw_ref: &str, version_spec: Option<&str>, json: bool) -> Result<()> {
    let session = Session {
        flake_dir: Path::new("."),
        cancelled: &|| false,
        progress: &|line| eprintln!("{line}"),
    };
    let info = resolve_package(&session, raw_ref, version_spec)?;

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
        return show_changelog(&session, &url);
    };

    let changelog = package_releases(&session, info, raw_spec)?;
    if json {
        serde_json::to_writer(std::io::stdout(), &changelog)?;
        println!();
        Ok(())
    } else {
        render_releases(&changelog.releases)
    }
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
}
