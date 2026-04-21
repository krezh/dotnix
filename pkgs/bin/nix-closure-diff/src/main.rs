use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "nix-closure-diff")]
#[command(about = "Diff NixOS system closures with JSON snapshot support", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a JSON snapshot of a system closure
    Snapshot {
        /// Path to built NixOS system (e.g., /run/current-system or result/)
        system_path: PathBuf,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Diff two JSON snapshots
    Diff {
        /// First snapshot JSON file
        old: PathBuf,

        /// Second snapshot JSON file
        new: PathBuf,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    system_path: String,
    packages: HashMap<String, HashMap<String, usize>>,
    system_packages: HashSet<String>,
}

#[derive(Debug)]
struct Package {
    name: String,
    versions: Vec<String>,
}

#[derive(Debug)]
struct VersionWithCount {
    version: String,
    count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeStatus {
    Upgraded,
    Downgraded,
    UpgradeDowngrade,
}

impl ChangeStatus {
    fn char(self) -> char {
        match self {
            Self::Upgraded => 'U',
            Self::Downgraded => 'D',
            Self::UpgradeDowngrade => 'C',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionStatus {
    Selected,         // * - system package in both
    NewlySelected,    // + - dependency before, system package now
    Unselected,       // . - dependency in both
    NewlyUnselected,  // - - system package before, dependency now
}

impl SelectionStatus {
    fn from_packages(name: &str, old_sys: &HashSet<String>, new_sys: &HashSet<String>) -> Self {
        match (old_sys.contains(name), new_sys.contains(name)) {
            (true, true) => Self::Selected,
            (true, false) => Self::NewlyUnselected,
            (false, true) => Self::NewlySelected,
            (false, false) => Self::Unselected,
        }
    }

    fn char(self) -> char {
        match self {
            Self::Selected => '*',
            Self::NewlySelected => '+',
            Self::Unselected => '.',
            Self::NewlyUnselected => '-',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffStatus {
    Changed(ChangeStatus),
    Added,
    Removed,
}

impl DiffStatus {
    fn char(self) -> char {
        match self {
            Self::Changed(status) => status.char(),
            Self::Added => 'A',
            Self::Removed => 'R',
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Snapshot { system_path, output } => {
            let snapshot = create_snapshot(&system_path)?;
            let json = serde_json::to_string_pretty(&snapshot)?;

            if let Some(out_path) = output {
                std::fs::write(out_path, json)?;
            } else {
                println!("{}", json);
            }
        }
        Commands::Diff { old, new } => {
            let old_snapshot: Snapshot = serde_json::from_str(
                &std::fs::read_to_string(old).context("Failed to read old snapshot")?
            )?;
            let new_snapshot: Snapshot = serde_json::from_str(
                &std::fs::read_to_string(new).context("Failed to read new snapshot")?
            )?;

            diff_snapshots(&old_snapshot, &new_snapshot)?;
        }
    }

    Ok(())
}

fn create_snapshot(system_path: &PathBuf) -> Result<Snapshot> {
    let system_path_str = system_path
        .canonicalize()
        .context("Failed to canonicalize path")?
        .to_str()
        .context("Invalid system path")?
        .to_string();

    // Query Nix database directly like dix does
    let packages = query_closure_from_db(&system_path_str)?;
    let system_packages = query_system_derivations(&system_path_str)?;

    Ok(Snapshot {
        system_path: system_path_str,
        packages,
        system_packages,
    })
}

// Copied from dix: query the Nix SQLite database
fn query_closure_from_db(path: &str) -> Result<HashMap<String, HashMap<String, usize>>> {
    use rusqlite::Connection;

    let db_path = "/nix/var/nix/db/db.sqlite";
    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    // Set pragmas like dix does
    conn.execute_batch(
        "PRAGMA mmap_size=268435456;
         PRAGMA temp_store=2;
         PRAGMA query_only;"
    )?;

    // Use dix's query
    const QUERY_DEPENDENTS: &str = "
        WITH RECURSIVE
            graph(p) AS (
                SELECT id
                FROM ValidPaths
                WHERE path = ?
            UNION
                SELECT reference FROM Refs
                JOIN graph ON referrer = p
            )
        SELECT path from graph
        JOIN ValidPaths ON id = p;
    ";

    let mut stmt = conn.prepare(QUERY_DEPENDENTS)?;
    let paths: Vec<String> = stmt
        .query_map([path], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    parse_paths(&paths)
}

fn query_system_derivations(path: &str) -> Result<HashSet<String>> {
    use rusqlite::Connection;

    let db_path = "/nix/var/nix/db/db.sqlite";
    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    conn.execute_batch(
        "PRAGMA mmap_size=268435456;
         PRAGMA temp_store=2;
         PRAGMA query_only;"
    )?;

    const QUERY_SYSTEM_DERIVATIONS: &str = "
        WITH
          systemderiv AS (
            SELECT id FROM ValidPaths
            WHERE path = ?
          ),
          systempath AS (
            SELECT reference as id FROM systemderiv sd
            JOIN Refs ON sd.id = referrer
            JOIN ValidPaths vp ON reference = vp.id
            WHERE (vp.path LIKE '%-system-path')
          ),
          pkgs AS (
              SELECT reference as id FROM Refs
              JOIN systempath ON referrer = id
          )
        SELECT path FROM pkgs
        JOIN ValidPaths vp ON vp.id = pkgs.id;
    ";

    let mut stmt = conn.prepare(QUERY_SYSTEM_DERIVATIONS)?;
    let paths: Vec<String> = stmt
        .query_map([path], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    let mut system_packages = HashSet::new();
    for path in paths {
        if let Ok((name, _)) = parse_store_path(&path) {
            system_packages.insert(name);
        }
    }

    Ok(system_packages)
}

fn parse_paths(paths: &[String]) -> Result<HashMap<String, HashMap<String, usize>>> {
    let mut packages: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for path in paths {
        if path.ends_with(".drv") {
            continue;
        }

        match parse_store_path(path) {
            Ok((name, version_opt)) => {
                let version = version_opt.unwrap_or_else(|| "<none>".to_string());
                *packages
                    .entry(name)
                    .or_insert_with(HashMap::new)
                    .entry(version)
                    .or_insert(0) += 1;
            }
            Err(_) => {
                // Fallback for non-standard paths
                if let Some(basename) = Path::new(path).file_name().and_then(|n| n.to_str()) {
                    let name = if basename.len() > 33 && basename.chars().nth(32) == Some('-') {
                        &basename[33..]
                    } else {
                        basename
                    };
                    *packages
                        .entry(name.to_string())
                        .or_insert_with(HashMap::new)
                        .entry("<none>".to_string())
                        .or_insert(0) += 1;
                }
            }
        }
    }

    Ok(packages)
}

// Copied from dix (GPL-3.0)
fn parse_store_path(path: &str) -> Result<(String, Option<String>)> {
    static STORE_PATH_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?<prefix>((/nix/store/)|(/tmp/.+?/))[a-zA-Z0-9]{32}-)(?<name>.+?)(-(?<version>[0-9].*?))?$",
        )
        .expect("failed to compile regex for Nix store paths")
    });

    let captures = STORE_PATH_REGEX
        .captures(path)
        .ok_or_else(|| anyhow::anyhow!("path '{}' does not match expected Nix store format", path))?;

    let name = captures
        .name("name")
        .map(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("failed to extract name from path '{}'", path))?;

    if name.is_empty() {
        bail!("failed to extract name from path '{}'", path);
    }

    let version = captures
        .name("version")
        .map(|c| c.as_str().trim_start_matches('-').to_owned());

    Ok((name.to_string(), version))
}

fn diff_snapshots(old: &Snapshot, new: &Snapshot) -> Result<()> {
    println!("<<< {}", old.system_path);
    println!(">>> {}", new.system_path);
    println!();

    #[derive(Debug)]
    struct Diff {
        name: String,
        old_unique: Vec<VersionWithCount>,
        new_unique: Vec<VersionWithCount>,
        has_common: bool,
        status: DiffStatus,
        selection: SelectionStatus,
    }

    let mut diffs = Vec::new();

    let all_packages: HashSet<_> = old.packages.keys()
        .chain(new.packages.keys())
        .collect();

    for pkg_name in all_packages {
        let old_vers = old.packages.get(pkg_name);
        let new_vers = new.packages.get(pkg_name);

        let (status, old_v, new_v, has_common) = match (old_vers, new_vers) {
            (Some(old_map), Some(new_map)) if old_map != new_map => {
                // Track versions that have the EXACT same count (truly common)
                let mut has_truly_common = false;
                let mut unique_old = Vec::new();
                let mut unique_new = Vec::new();

                // Get all versions that exist in either map
                let all_versions: HashSet<_> = old_map.keys().chain(new_map.keys()).collect();

                for ver in all_versions {
                    let old_count = old_map.get(ver).copied().unwrap_or(0);
                    let new_count = new_map.get(ver).copied().unwrap_or(0);

                    let min_count = old_count.min(new_count);
                    if min_count > 0 {
                        has_truly_common = true;
                    }

                    if old_count > min_count {
                        unique_old.push(VersionWithCount {
                            version: (*ver).clone(),
                            count: old_count - min_count,
                        });
                    }

                    if new_count > min_count {
                        unique_new.push(VersionWithCount {
                            version: (*ver).clone(),
                            count: new_count - min_count,
                        });
                    }
                }

                unique_old.sort_by(|a, b| a.version.cmp(&b.version));
                unique_new.sort_by(|a, b| a.version.cmp(&b.version));

                let unique_old_vers: Vec<_> = unique_old.iter().map(|v| v.version.clone()).collect();
                let unique_new_vers: Vec<_> = unique_new.iter().map(|v| v.version.clone()).collect();

                let change_status = determine_change_status_from_unique(&unique_old_vers, &unique_new_vers, has_truly_common);
                (DiffStatus::Changed(change_status), unique_old, unique_new, has_truly_common)
            }
            (None, Some(new_map)) => {
                let vers: Vec<_> = new_map.iter()
                    .map(|(v, &c)| VersionWithCount { version: v.clone(), count: c })
                    .collect();
                (DiffStatus::Added, vec![], vers, false)
            }
            (Some(old_map), None) => {
                let vers: Vec<_> = old_map.iter()
                    .map(|(v, &c)| VersionWithCount { version: v.clone(), count: c })
                    .collect();
                (DiffStatus::Removed, vers, vec![], false)
            }
            _ => continue,
        };

        let selection = SelectionStatus::from_packages(
            pkg_name,
            &old.system_packages,
            &new.system_packages
        );

        diffs.push(Diff {
            name: pkg_name.to_string(),
            old_unique: old_v,
            new_unique: new_v,
            has_common,
            status,
            selection,
        });
    }

    diffs.sort_by(|a, b| a.name.cmp(&b.name));

    let changed: Vec<_> = diffs.iter().filter(|d| matches!(d.status, DiffStatus::Changed(_))).collect();
    let added: Vec<_> = diffs.iter().filter(|d| matches!(d.status, DiffStatus::Added)).collect();
    let removed: Vec<_> = diffs.iter().filter(|d| matches!(d.status, DiffStatus::Removed)).collect();

    if !changed.is_empty() {
        println!("CHANGED");
        for diff in changed {
            let tag = format!("[{}{}]", diff.status.char(), diff.selection.char());
            let old_str = format_versions_with_counts(&diff.old_unique, diff.has_common);
            let new_str = format_versions_with_counts(&diff.new_unique, diff.has_common);
            println!("{} {:<48} {} -> {}", tag, diff.name, old_str, new_str);
        }
        println!();
    }

    if !added.is_empty() {
        println!("ADDED");
        for diff in added {
            let tag = format!("[{}{}]", diff.status.char(), diff.selection.char());
            println!("{} {:<48} <none>", tag, diff.name);
        }
        println!();
    }

    if !removed.is_empty() {
        println!("REMOVED");
        for diff in removed {
            let tag = format!("[{}{}]", diff.status.char(), diff.selection.char());
            println!("{} {:<48} <none>", tag, diff.name);
        }
        println!();
    }

    let total = diffs.len();
    if total == 0 {
        println!("No changes");
    } else {
        println!();
        println!("Note: SIZE/DIFF not available (requires built systems)");
    }

    Ok(())
}

fn determine_change_status_from_unique(unique_old: &[String], unique_new: &[String], has_common: bool) -> ChangeStatus {
    // Match dix's logic from lines 972-982:
    // if unique_old.is_empty() || unique_new.is_empty() => Changed(UpgradeDowngrade)
    // else => determine_change_status(&unique_old, &unique_new)

    if unique_old.is_empty() || unique_new.is_empty() {
        return ChangeStatus::UpgradeDowngrade;
    }

    // Both unique sets are non-empty - determine if it's an upgrade, downgrade, or mixed
    let mut saw_upgrade = false;
    let mut saw_downgrade = false;

    for old_v in unique_old {
        for new_v in unique_new {
            match compare_versions(old_v, new_v) {
                std::cmp::Ordering::Less => saw_upgrade = true,
                std::cmp::Ordering::Greater => saw_downgrade = true,
                std::cmp::Ordering::Equal => {},
            }
            if saw_upgrade && saw_downgrade {
                return ChangeStatus::UpgradeDowngrade;
            }
        }
    }

    if saw_upgrade && !saw_downgrade {
        ChangeStatus::Upgraded
    } else if saw_downgrade && !saw_upgrade {
        ChangeStatus::Downgraded
    } else {
        ChangeStatus::UpgradeDowngrade
    }
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse_component(s: &str) -> Vec<VersionPart> {
        let mut parts = Vec::new();
        let mut current_num = String::new();
        let mut current_str = String::new();

        for c in s.chars() {
            if c.is_numeric() {
                if !current_str.is_empty() {
                    parts.push(VersionPart::Str(current_str.clone()));
                    current_str.clear();
                }
                current_num.push(c);
            } else if c == '.' || c == '-' || c == '_' {
                if !current_num.is_empty() {
                    if let Ok(n) = current_num.parse::<u64>() {
                        parts.push(VersionPart::Num(n));
                    }
                    current_num.clear();
                }
                if !current_str.is_empty() {
                    parts.push(VersionPart::Str(current_str.clone()));
                    current_str.clear();
                }
            } else {
                if !current_num.is_empty() {
                    if let Ok(n) = current_num.parse::<u64>() {
                        parts.push(VersionPart::Num(n));
                    }
                    current_num.clear();
                }
                current_str.push(c);
            }
        }

        if !current_num.is_empty() {
            if let Ok(n) = current_num.parse::<u64>() {
                parts.push(VersionPart::Num(n));
            }
        }
        if !current_str.is_empty() {
            parts.push(VersionPart::Str(current_str));
        }

        parts
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum VersionPart {
        Num(u64),
        Str(String),
    }

    let a_parts = parse_component(a);
    let b_parts = parse_component(b);

    a_parts.cmp(&b_parts)
}

fn format_versions_with_counts(versions: &[VersionWithCount], has_common: bool) -> String {
    let mut result = String::new();
    let limit = 2;

    for (i, ver) in versions.iter().enumerate() {
        if i >= limit {
            if !result.is_empty() {
                result.push_str(", ");
            }
            result.push_str("<others>");
            break;
        }
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&ver.version);
        if ver.count > 1 {
            result.push_str(&format!(" ×{}", ver.count));
        }
    }

    if has_common && !result.contains("<others>") {
        if !result.is_empty() {
            result.push_str(", ");
        }
        result.push_str("<others>");
    }

    result
}
