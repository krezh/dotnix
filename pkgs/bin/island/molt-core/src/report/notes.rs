//! A changed package's release notes, fetched with nix-changelog, and a changed file's diff: what
//! each detail screen of the review knows, and how it is worded.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use nix_changelog::Session;

pub use nix_changelog::Changelog;

use crate::command::CANCELLED;
use crate::report::config_diff::{ConfigChange, ConfigContent, DiffLineKind};
use crate::report::markdown::{MarkdownBlockKind, heading_tone, markdown_blocks};
use crate::report::present::Ink;
use crate::report::{Change, Report};
use crate::units::{Hue, Units};

const CHANGELOG_TIMEOUT: Duration = Duration::from_secs(2 * 60);

#[derive(Clone)]
pub struct Notes {
    pub change: Change,
    /// `NixOS #thor`.
    pub configuration: String,
    pub flake_dir: PathBuf,
    /// Tells this fetch's messages from an earlier one's.
    pub generation: u64,
    pub started: Instant,
    /// What nix-changelog tried, and when.
    pub steps: Vec<(String, Duration)>,
    pub result: Option<Result<Changelog, String>>,
    version: u64,
}

impl PartialEq for Notes {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.version == other.version
    }
}

impl std::fmt::Debug for Notes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Notes")
            .field("package", &self.change.name)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

pub enum Msg {
    Step(u64, String),
    Done(u64, Result<Changelog, String>),
    Tick,
}

pub struct Effect {
    pub generation: u64,
    pub flake_dir: PathBuf,
    pub name: String,
    pub versions: String,
}

impl Notes {
    /// The notes for `change`, with notes already fetched when `cached` has them.
    pub fn open(
        change: Change,
        report: &Report,
        generation: u64,
        cached: Option<Changelog>,
    ) -> (Self, Option<Effect>) {
        let versions = Self::versions(&change);
        let effect = cached.is_none().then(|| Effect {
            generation,
            flake_dir: report.flake_dir.clone(),
            name: change.name.clone(),
            versions,
        });
        let notes = Self {
            configuration: format!("{} #{}", report.target.name(), report.flake),
            flake_dir: report.flake_dir.clone(),
            change,
            generation,
            started: Instant::now(),
            steps: Vec::new(),
            result: cached.map(Ok),
            version: 0,
        };
        (notes, effect)
    }

    /// `1.0..2.0`, or the new version alone for an added package.
    pub fn versions(change: &Change) -> String {
        if change.old.is_empty() {
            change.new.clone()
        } else {
            format!("{}..{}", change.old, change.new)
        }
    }

    /// Where fetched notes are kept between visits.
    pub fn cache_key(&self) -> String {
        format!(
            "{}\0{}\0{}",
            self.flake_dir.display(),
            self.change.name,
            Self::versions(&self.change)
        )
    }

    pub fn update(&mut self, msg: Msg) {
        self.version += 1;
        match msg {
            Msg::Step(generation, step) if generation == self.generation => {
                self.steps.push((step, self.started.elapsed()));
            }
            Msg::Done(generation, result) if generation == self.generation => {
                self.result = Some(result);
            }
            _ => {}
        }
    }
}

/// Fetches the notes, telling `emit` each step nix-changelog tries.
pub fn run(effect: Effect, cancellation: &AtomicBool, emit: &(dyn Fn(Msg) + Sync)) {
    let started = Instant::now();
    let generation = effect.generation;
    let result = nix_changelog::releases(
        &Session {
            flake_dir: &effect.flake_dir,
            cancelled: &|| {
                cancellation.load(Ordering::Relaxed) || started.elapsed() > CHANGELOG_TIMEOUT
            },
            progress: &|step| emit(Msg::Step(generation, step.to_owned())),
        },
        &effect.name,
        &effect.versions,
    )
    .map_err(|error| {
        if cancellation.load(Ordering::Relaxed) {
            CANCELLED.to_owned()
        } else if started.elapsed() > CHANGELOG_TIMEOUT {
            format!(
                "nix-changelog timed out after {} seconds",
                CHANGELOG_TIMEOUT.as_secs()
            )
        } else {
            format!("{error:#}")
        }
    });
    emit(Msg::Done(generation, result));
}

pub struct Block {
    pub kind: MarkdownBlockKind,
    /// Pango-style markup; `markdown::spans` reads it as text and styles.
    pub markup: String,
    /// The section's colour, as its heading names it.
    pub hue: Option<Hue>,
}

pub struct ReleaseView {
    pub tag: String,
    pub blocks: Vec<Block>,
}

pub struct NotesView {
    /// `firefox release notes`.
    pub title: String,
    /// `143.0.1 → 144.0`, then ` · NixOS #thor`.
    pub versions: Vec<(String, Ink)>,
    pub configuration: String,
    pub back: &'static str,
    /// `Release notes`, or `Release notes · 3`.
    pub heading: String,
    /// The package and version nix-changelog resolved.
    pub source: Option<String>,
    pub loading: Option<(String, String)>,
    /// Each step tried, and when.
    pub steps: Vec<(String, String)>,
    /// `Steps attempted · 5.1 s`, once it failed.
    pub steps_title: Option<String>,
    pub error: Option<(&'static str, String)>,
    pub description: Option<String>,
    pub releases: Vec<ReleaseView>,
    pub empty: Option<&'static str>,
}

fn version_range(change: &Change) -> Vec<(String, Ink)> {
    match (change.old.is_empty(), change.new.is_empty()) {
        (false, false) => vec![
            (change.old.clone(), Ink::Plain),
            (" → ".to_owned(), Ink::Plain),
            (change.new.clone(), Ink::New),
        ],
        (true, _) => vec![(change.new.clone(), Ink::New)],
        (false, true) => vec![(change.old.clone(), Ink::Plain)],
    }
}

fn releases(changelog: &Changelog) -> Vec<ReleaseView> {
    changelog
        .releases
        .iter()
        .map(|release| {
            // Headings like "Bug Fixes" tint themselves and the items under them, until the next
            // heading at the same or a higher level.
            let mut tone: Option<(u8, Hue)> = None;
            let blocks = markdown_blocks(&release.body)
                .into_iter()
                .map(|block| {
                    let hue = if let MarkdownBlockKind::Heading(level) = block.kind {
                        let own = heading_tone(&block.markup);
                        if tone.is_none_or(|(tone_level, _)| level <= tone_level) {
                            tone = own.map(|hue| (level, hue));
                        }
                        own.or(tone.map(|(_, hue)| hue))
                    } else {
                        tone.map(|(_, hue)| hue)
                    };
                    Block {
                        kind: block.kind,
                        markup: block.markup,
                        hue,
                    }
                })
                .collect();
            ReleaseView {
                tag: release.tag.clone(),
                blocks,
            }
        })
        .collect()
}

pub fn view(notes: &Notes, units: &dyn Units) -> NotesView {
    let timing = |duration| units.timing(duration, true);
    let steps = notes
        .steps
        .iter()
        .map(|(step, at)| (step.clone(), timing(*at)))
        .collect();
    let base = NotesView {
        title: format!("{} release notes", notes.change.name),
        versions: version_range(&notes.change),
        configuration: notes.configuration.clone(),
        back: "Back to report",
        heading: "Release notes".to_owned(),
        source: None,
        loading: None,
        steps,
        steps_title: None,
        error: None,
        description: None,
        releases: Vec::new(),
        empty: None,
    };
    match &notes.result {
        None => NotesView {
            loading: Some((
                notes
                    .steps
                    .last()
                    .map_or_else(|| "Looking up release notes…".to_owned(), |(step, _)| step.clone()),
                timing(notes.started.elapsed()),
            )),
            ..base
        },
        Some(Err(error)) => NotesView {
            error: Some(("Release notes unavailable", error.clone())),
            steps_title: (!notes.steps.is_empty())
                .then(|| format!("Steps attempted · {}", timing(notes.started.elapsed()))),
            ..base
        },
        Some(Ok(changelog)) => NotesView {
            heading: if changelog.releases.is_empty() {
                "Release notes".to_owned()
            } else {
                format!("Release notes · {}", changelog.releases.len())
            },
            source: Some(format!("{} {}", changelog.pname, changelog.version)),
            description: changelog.description.clone(),
            releases: releases(changelog),
            empty: changelog
                .releases
                .is_empty()
                .then_some("No release notes found for this version"),
            steps: Vec::new(),
            ..base
        },
    }
}

pub struct DiffView {
    /// The file's name.
    pub title: String,
    /// `Changed · ~/.config/ghostty/config · NixOS #thor`.
    pub subtitle: String,
    pub back: &'static str,
    pub heading: &'static str,
    pub note: &'static str,
    /// Each line with its sign, and how it changed.
    pub lines: Vec<(String, DiffLineKind)>,
    pub binary: Option<&'static str>,
}

pub fn diff(change: &ConfigChange, report: &Report) -> DiffView {
    let (lines, binary) = match &change.content {
        ConfigContent::Text { lines, .. } => (
            lines
                .iter()
                .map(|line| {
                    let sign = match line.kind {
                        DiffLineKind::Hunk => "",
                        DiffLineKind::Context => " ",
                        DiffLineKind::Removed => "-",
                        DiffLineKind::Added => "+",
                    };
                    (format!("{sign}{}", line.text), line.kind)
                })
                .collect(),
            None,
        ),
        ConfigContent::Binary => (
            Vec::new(),
            Some("Binary or oversized file: contents are not shown"),
        ),
    };
    DiffView {
        title: change
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&change.path)
            .to_owned(),
        subtitle: format!(
            "{} · {} · {} #{}",
            change.status.label(),
            change.path,
            report.target.name(),
            report.flake
        ),
        back: "Back to report",
        heading: "Diff",
        note: "store hashes shown as …",
        lines,
        binary,
    }
}
