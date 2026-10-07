use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::filesystem::{home, items_for_paths};
use super::model::{CleanupGroup, CleanupKind};

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let Some(home) = home() else {
        return CleanupGroup::unavailable(CleanupKind::Development, "HOME is unavailable");
    };
    let Ok(root_metadata) = fs::metadata(&home) else {
        return CleanupGroup::unavailable(
            CleanupKind::Development,
            format!("cannot inspect {}", home.display()),
        );
    };
    let mut pending = vec![home.clone()];
    let mut candidates = Vec::new();
    while let Some(directory) = pending.pop() {
        if cancellation.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                if is_nix_result(&path) {
                    let label = project_label("Nix result", &path, &home);
                    candidates.push((path, label));
                }
                continue;
            }
            if !file_type.is_dir() || skip_directory(&path, &home) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.dev() != root_metadata.dev() {
                continue;
            }
            if let Some(kind) = artifact_kind(&path) {
                let label = project_label(kind, &path, &home);
                candidates.push((path, label));
                continue;
            }
            pending.push(path);
        }
    }
    let mut items = items_for_paths(candidates, &home, cancellation);
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    let reclaimable = items.iter().filter_map(|item| item.bytes).sum();
    CleanupGroup {
        kind: CleanupKind::Development,
        available: true,
        reclaimable,
        items,
        note: Some("Only reproducible build and dependency directories are selected".to_owned()),
    }
}

fn artifact_kind(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    let parent = path.parent()?;
    match name {
        "target" if parent.join("Cargo.toml").is_file() => Some("Rust target"),
        "node_modules" if has_any(parent, &["package.json", "pnpm-lock.yaml", "bun.lock"]) => {
            Some("JavaScript dependencies")
        }
        ".next" | ".nuxt" | ".svelte-kit" if parent.join("package.json").is_file() => {
            Some("Web build")
        }
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" => Some("Python cache"),
        ".venv" if has_any(parent, &["pyproject.toml", "requirements.txt", "setup.py"]) => {
            Some("Python environment")
        }
        ".zig-cache" | "zig-cache" if parent.join("build.zig").is_file() => Some("Zig cache"),
        ".direnv" if has_any(parent, &[".envrc", "flake.nix"]) => Some("direnv environment"),
        _ => None,
    }
}

fn is_nix_result(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    (name == "result" || name.starts_with("result-"))
        && path
            .parent()
            .is_some_and(|parent| parent.join("flake.nix").is_file())
}

fn skip_directory(path: &Path, home: &Path) -> bool {
    let relative = path.strip_prefix(home).unwrap_or(path);
    let Some(first) = relative.components().next() else {
        return false;
    };
    matches!(
        first.as_os_str().to_str(),
        Some(
            ".bun"
                | ".cache"
                | ".cargo"
                | ".config"
                | ".gradle"
                | ".local"
                | ".m2"
                | ".mozilla"
                | ".nix-defexpr"
                | ".rustup"
                | ".steam"
                | ".var"
                | "Downloads"
                | "Games"
                | "Music"
                | "Pictures"
                | "Videos"
        )
    ) || matches!(
        path.file_name().and_then(OsStr::to_str),
        Some(".git" | ".jj" | ".svn")
    )
}

fn project_label(kind: &str, artifact: &Path, home: &Path) -> String {
    let project = artifact
        .parent()
        .and_then(|path| path.strip_prefix(home).ok())
        .map(|path| path.display().to_string())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "home".to_owned());
    format!("{kind} · {project}")
}

fn has_any(directory: &Path, names: &[&str]) -> bool {
    names.iter().any(|name| directory.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_manifest_backed_artifacts() {
        let root = std::env::temp_dir().join(format!("swix-artifact-test-{}", std::process::id()));
        let project = root.join("project");
        fs::create_dir_all(project.join("target")).unwrap();
        fs::write(project.join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(artifact_kind(&project.join("target")), Some("Rust target"));
        assert_eq!(artifact_kind(&root.join("target")), None);
        let _ = fs::remove_dir_all(root);
    }
}
