use std::{
  env,
  fmt::{
    self,
    Write as _,
  },
  fs,
  io::{
    self,
    IsTerminal as _,
    Write as _,
  },
  path::{
    Path,
    PathBuf,
  },
  process,
};

use anyhow::{
  Context as _,
  Result,
};
use clap::Parser as _;
use serde::{
  Deserialize,
  Serialize,
};
use yansi::Paint as _;

use nix_closure_diff::{
  StorePath,
  diff,
  store::{
    self,
    StoreBackend,
  },
};

struct WriteFmt<W: io::Write>(W);

impl<W: io::Write> fmt::Write for WriteFmt<W> {
  fn write_str(&mut self, string: &str) -> fmt::Result {
    self.0.write_all(string.as_bytes()).map_err(|_| fmt::Error)
  }
}

#[derive(clap::Parser, Debug)]
#[command(version, about = "Diff Nix closures with JSON snapshot support")]
struct Cli {
  #[command(subcommand)]
  command: Command,

  #[command(flatten)]
  verbose: clap_verbosity_flag::Verbosity,

  /// Controls when to use color.
  #[arg(
      long,
      default_value_t = clap::ColorChoice::Auto,
      value_name = "WHEN",
      global = true,
  )]
  color: clap::ColorChoice,

  /// Fall back to a backend that is focused solely on absolutely guaranteeing
  /// correct results at the cost of memory usage and query speed.
  #[arg(long, default_value_t = false, global = true)]
  force_correctness: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
  /// Create a JSON snapshot of a NixOS system closure
  Snapshot {
    /// Path to the NixOS system
    system_path: PathBuf,

    /// Output JSON file path
    #[arg(short, long)]
    output: PathBuf,
  },

  /// Diff two system paths or JSON snapshots
  Diff {
    /// Old system path or JSON snapshot
    old: PathBuf,

    /// New system path or JSON snapshot
    new: PathBuf,
  },
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
  system_path:  String,
  paths:        Vec<String>,
  system_paths: Vec<String>,
}

fn create_snapshot(
  system_path: &Path,
  force_correctness: bool,
) -> Result<Snapshot> {
  let mut connection = if force_correctness {
    store::CombinedStoreBackend::default_eager()
  } else {
    store::CombinedStoreBackend::default_lazy()
  };

  connection.connect()?;

  // Query dependencies
  let paths: Vec<String> = connection
    .query_dependents(system_path)
    .with_context(|| {
      format!(
        "failed to query dependencies of '{}'",
        system_path.display()
      )
    })?
    .map(|sp: StorePath| sp.to_str().unwrap_or("").to_string())
    .collect();

  // Query system derivations
  let system_paths: Vec<String> = connection
    .query_system_derivations(system_path)
    .with_context(|| {
      format!(
        "failed to query system derivations of '{}'",
        system_path.display()
      )
    })?
    .map(|sp: StorePath| sp.to_str().unwrap_or("").to_string())
    .collect();

  Ok(Snapshot {
    system_path: system_path.to_string_lossy().to_string(),
    paths,
    system_paths,
  })
}

fn load_or_query_snapshot(
  path: &Path,
  force_correctness: bool,
) -> Result<Snapshot> {
  // Check if it's a JSON file
  if path.extension().and_then(|s| s.to_str()) == Some("json") {
    let contents =
      fs::read_to_string(path).with_context(|| {
        format!("failed to read snapshot file '{}'", path.display())
      })?;
    serde_json::from_str(&contents).with_context(|| {
      format!("failed to parse snapshot file '{}'", path.display())
    })
  } else {
    // It's a system path, create snapshot on the fly
    create_snapshot(path, force_correctness)
  }
}

fn real_main() -> Result<()> {
  let Cli {
    command,
    verbose,
    color,
    force_correctness,
  } = Cli::parse();

  yansi::whenever(match color {
    clap::ColorChoice::Auto => yansi::Condition::from(should_style),
    clap::ColorChoice::Always => yansi::Condition::ALWAYS,
    clap::ColorChoice::Never => yansi::Condition::NEVER,
  });

  env_logger::Builder::new()
    .filter_level(verbose.log_level_filter())
    .format(|out, arguments| {
      let header = match arguments.level() {
        log::Level::Error => "error:".red(),
        log::Level::Warn => "warn:".yellow(),
        log::Level::Info => "info:".green(),
        log::Level::Debug => "debug:".blue(),
        log::Level::Trace => "trace:".cyan(),
      };

      writeln!(out, "{header} {message}", message = arguments.args())
    })
    .init();

  match command {
    Command::Snapshot {
      system_path,
      output,
    } => {
      let snapshot = create_snapshot(&system_path, force_correctness)?;
      let json = serde_json::to_string_pretty(&snapshot)?;
      fs::write(&output, json).with_context(|| {
        format!("failed to write snapshot to '{}'", output.display())
      })?;
      eprintln!("Snapshot saved to {}", output.display());
      Ok(())
    },

    Command::Diff { old, new } => {
      let old_snapshot = load_or_query_snapshot(&old, force_correctness)?;
      let new_snapshot = load_or_query_snapshot(&new, force_correctness)?;

      let mut out = WriteFmt(io::stdout());

      writeln!(
        out,
        "{arrows} {old}",
        arrows = "<<<".bold(),
        old = old_snapshot.system_path,
      )?;
      writeln!(
        out,
        "{arrows} {new}",
        arrows = ">>>".bold(),
        new = new_snapshot.system_path,
      )?;
      writeln!(out)?;

      // Convert paths back to StorePath
      let old_paths = old_snapshot
        .paths
        .into_iter()
        .filter_map(|p| StorePath::try_from(PathBuf::from(p)).ok());
      let new_paths = new_snapshot
        .paths
        .into_iter()
        .filter_map(|p| StorePath::try_from(PathBuf::from(p)).ok());

      let old_system_paths = old_snapshot
        .system_paths
        .into_iter()
        .filter_map(|p| StorePath::try_from(PathBuf::from(p)).ok());
      let new_system_paths = new_snapshot
        .system_paths
        .into_iter()
        .filter_map(|p| StorePath::try_from(PathBuf::from(p)).ok());

      // Use dix's diff logic
      let wrote = diff::write_packages_diffln(
        &mut out,
        old_paths,
        new_paths,
        old_system_paths,
        new_system_paths,
      )?;

      if wrote > 0 {
        writeln!(out)?;
      }

      // Note: We don't include closure size diff for snapshot mode
      // as that would require querying the store
      if !old.extension().map_or(false, |e| e == "json")
        && !new.extension().map_or(false, |e| e == "json")
      {
        // Both are system paths, we can calculate closure sizes
        let mut connection = if force_correctness {
          store::CombinedStoreBackend::default_eager()
        } else {
          store::CombinedStoreBackend::default_lazy()
        };
        connection.connect()?;

        let size_old = connection.query_closure_size(&old)?;
        let size_new = connection.query_closure_size(&new)?;

        diff::write_size_diff(&mut out, size_old, size_new)?;
      }

      Ok(())
    },
  }
}

#[allow(clippy::allow_attributes, clippy::exit)]
fn main() {
  let Err(error) = real_main() else {
    return;
  };

  let mut err = io::stderr();

  let mut message = String::new();
  let mut chain = error.chain().rev().peekable();

  while let Some(error) = chain.next() {
    let _ = write!(
      err,
      "{header} ",
      header = if chain.peek().is_none() {
        "error:"
      } else {
        "cause:"
      }
      .red()
      .bold(),
    );

    String::clear(&mut message);
    let _ = write!(message, "{error}");

    let mut chars = message.char_indices();

    let _ = match (chars.next(), chars.next()) {
      (Some((_, first)), Some((second_start, second)))
        if second.is_lowercase() =>
      {
        writeln!(
          err,
          "{first_lowercase}{rest}",
          first_lowercase = first.to_lowercase(),
          rest = &message[second_start..],
        )
      },

      _ => {
        writeln!(err, "{message}")
      },
    };
  }

  process::exit(1);
}

// https://bixense.com/clicolors/
fn should_style() -> bool {
  // If NO_COLOR is set and is not empty, don't style.
  if let Some(value) = env::var_os("NO_COLOR")
    && !value.is_empty()
  {
    return false;
  }

  // If CLICOLOR is set and is 0, don't style.
  if let Some(value) = env::var_os("CLICOLOR")
    && value == "0"
  {
    return false;
  }

  // If CLICOLOR_FORCE is set and not 0, always style.
  if let Some(value) = env::var_os("CLICOLOR_FORCE")
    && value != "0"
  {
    return true;
  }

  // Style if it is a terminal.
  io::stdout().is_terminal()
}
