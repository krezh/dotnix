mod changelog;
mod diff;
mod nix;
mod progress;
mod tui;

use std::{
    env,
    io::{self, BufRead as _, IsTerminal as _, Write as _},
    path::{Path, PathBuf},
    process,
};

use clap::{Args, CommandFactory as _, Parser, Subcommand};
use clap_complete::{Shell, generate};
use eyre::{Result, bail, eyre};

use crate::{nix::Activation, tui::Outcome};

#[derive(Parser)]
#[command(
    name = "norn",
    version,
    about = "Rebuild NixOS, and read what actually changed"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// NixOS configuration management
    Os {
        #[command(subcommand)]
        action: OsAction,
    },

    /// Browse the changelogs between two already-built system closures
    Diff {
        /// The closure upgraded from, e.g. /nix/var/nix/profiles/system-42-link
        old: PathBuf,
        /// The closure upgraded to
        new: PathBuf,
    },

    /// Print one package's release notes for an upgrade, without any TUI
    Changelog {
        /// nixpkgs attribute name, e.g. claude-code
        name: String,
        /// Version upgraded from
        old: String,
        /// Version upgraded to
        new: String,
    },

    /// Generate a shell completion script
    Completion { shell: Shell },
}

#[derive(Subcommand)]
enum OsAction {
    /// Build the configuration without activating it
    Build(RebuildArgs),
    /// Build and activate, and make it the boot default
    Switch(RebuildArgs),
    /// Build and make it the boot default, to activate on next boot
    Boot(RebuildArgs),
    /// Build and activate, without making it the boot default
    Test(RebuildArgs),
}

#[derive(Args)]
struct RebuildArgs {
    /// Flake holding the nixosConfigurations
    #[arg(long, short, env = "NH_FLAKE")]
    flake: Option<String>,

    /// Configuration to build, defaulting to the system hostname
    #[arg(long, short = 'H')]
    hostname: Option<String>,

    /// Ask before activating when the changelog browser is skipped
    #[arg(long, short)]
    ask: bool,

    /// Show Nix's own build output instead of the progress display
    #[arg(long)]
    plain: bool,

    /// Skip the changelog browser entirely
    #[arg(long)]
    no_changelog: bool,

    /// Where to keep the result symlink
    #[arg(long, short)]
    out_link: Option<PathBuf>,

    /// Extra arguments forwarded to `nix build`
    #[arg(last = true)]
    extra: Vec<String>,
}

impl RebuildArgs {
    fn flake(&self) -> Result<String> {
        self.flake
            .clone()
            .or_else(|| env::var("FLAKE").ok())
            .filter(|flake| !flake.is_empty())
            .ok_or_else(|| eyre!("no flake given — pass --flake, or set NH_FLAKE"))
    }

    fn out_link(&self) -> PathBuf {
        self.out_link.clone().unwrap_or_else(|| {
            env::var_os("XDG_RUNTIME_DIR")
                .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
                .join("norn-result")
        })
    }
}

fn confirm(question: &str) -> Result<bool> {
    print!("{question} [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

fn rebuild(activation: Option<Activation>, args: &RebuildArgs) -> Result<()> {
    let flake = args.flake()?;
    let host = match &args.hostname {
        Some(hostname) => hostname.clone(),
        None => nix::hostname()?,
    };

    let out_link = args.out_link();
    if let Some(parent) = out_link.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let attr = nix::toplevel_attr(&flake, &host);

    // The full-screen session needs a terminal to draw on; without one, fall
    // back to Nix's own streaming output.
    let interactive = !args.plain && io::stderr().is_terminal();

    let (toplevel, outcome) = if interactive {
        tui::run(tui::Rebuild {
            label: host,
            attr: attr.clone(),
            extra: args.extra.clone(),
            command: nix::build_command(&attr, &out_link, &args.extra),
            old_profile: PathBuf::from(nix::CURRENT_PROFILE),
            out_link: out_link.clone(),
            browse: !args.no_changelog,
        })?
    } else {
        (
            nix::build_plain(&attr, &out_link, &args.extra)?,
            Outcome::Continue,
        )
    };

    let Some(activation) = activation else {
        println!("{}", toplevel.display());
        return Ok(());
    };

    match outcome {
        Outcome::Abort => bail!("aborted before activating"),
        // When the browser ran, pressing `c` there *was* the confirmation.
        // Only ask again if it never appeared.
        Outcome::Continue => {
            let browsed = interactive && !args.no_changelog;
            if !browsed && args.ask && !confirm("Apply the config?")? {
                bail!("aborted before activating");
            }
        }
    }

    nix::activate(&toplevel, activation)
}

fn show_diff(old: &Path, new: &Path) -> Result<()> {
    let (changes, summary) = diff::changes(old, new)?;

    if changes.is_empty() {
        println!("{summary}");
        println!("No version changes.");
        return Ok(());
    }

    if io::stderr().is_terminal() {
        tui::browse_only(changes, summary)?;
    } else {
        println!("{summary}");
        for change in changes {
            println!("{} {} -> {}", change.name, change.old, change.new);
        }
    }
    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Cmd::Completion { shell } => {
            generate(shell, &mut Cli::command(), "norn", &mut io::stdout());
            Ok(())
        }
        Cmd::Changelog { name, old, new } => {
            print!("{}", changelog::markdown_for(&name, &old, &new)?);
            Ok(())
        }
        Cmd::Diff { old, new } => show_diff(&old, &new),
        Cmd::Os { action } => match &action {
            OsAction::Build(args) => rebuild(None, args),
            OsAction::Switch(args) => rebuild(Some(Activation::Switch), args),
            OsAction::Boot(args) => rebuild(Some(Activation::Boot), args),
            OsAction::Test(args) => rebuild(Some(Activation::Test), args),
        },
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("\x1b[1;31m✗\x1b[0m {error:#}");
        process::exit(1);
    }
}
