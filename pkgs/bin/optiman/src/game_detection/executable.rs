use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::steam::SteamGame;

/// Finds the main game executable for a Steam game.
pub fn find_game_executable(game: &SteamGame) -> Result<PathBuf> {
    // Strategy:
    // 1. Check for Unreal Engine pattern: Game/Binaries/Win64/*.exe
    // 2. Look for *.exe files in the root directory
    // 3. Check common subdirectories (bin, binaries, game, etc.)
    // 4. Fall back to recursive search (limited depth)

    // 1. Check for Unreal Engine pattern
    if let Some(exe) = check_unreal_engine_pattern(&game.install_dir)? {
        return Ok(exe);
    }

    // 2. Look for .exe files in the root directory
    if let Some(exe) = find_exe_in_directory(&game.install_dir, false)? {
        return Ok(exe);
    }

    // 3. Check common subdirectories
    let common_dirs = ["bin", "Bin", "binaries", "Binaries", "Game", "game"];
    for dir_name in common_dirs {
        let subdir = game.install_dir.join(dir_name);
        if subdir.exists() && subdir.is_dir() {
            if let Some(exe) = find_exe_in_directory(&subdir, false)? {
                return Ok(exe);
            }
        }
    }

    // 4. Fall back to recursive search (limit depth to avoid performance issues)
    if let Some(exe) = find_exe_recursive(&game.install_dir, 0, 6)? {
        return Ok(exe);
    }

    anyhow::bail!("Could not find game executable for: {}", game.name)
}

/// Checks for Unreal Engine pattern: <GameName>/Binaries/Win64/<GameName>.exe
fn check_unreal_engine_pattern(install_dir: &std::path::Path) -> Result<Option<PathBuf>> {
    // Look for any subdirectory that has Binaries/Win64
    let entries = match fs::read_dir(install_dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let binaries_win64 = path.join("Binaries").join("Win64");
            if binaries_win64.exists() && binaries_win64.is_dir() {
                if let Some(exe) = find_exe_in_directory(&binaries_win64, false)? {
                    return Ok(Some(exe));
                }
            }
        }
    }

    Ok(None)
}

/// Finds the first .exe file in a directory (optionally recursive).
fn find_exe_in_directory(dir: &std::path::Path, recursive: bool) -> Result<Option<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("exe") {
                    // Skip common launchers/updaters/crashreporters
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        let filename_lower = filename.to_lowercase();
                        if filename_lower.contains("launcher") ||
                            filename_lower.contains("updater") ||
                            filename_lower.contains("crash") ||
                            filename_lower.contains("unins") ||
                            filename_lower.contains("setup") ||
                            filename_lower == "unrealcefsubprocess" {
                            continue;
                        }
                    }
                    candidates.push(path);
                }
            }
        } else if recursive && path.is_dir() {
            if let Some(exe) = find_exe_in_directory(&path, true)? {
                return Ok(Some(exe));
            }
        }
    }

    // Return the first candidate (or the one with shortest name as heuristic)
    if !candidates.is_empty() {
        candidates.sort_by_key(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.len())
                .unwrap_or(usize::MAX)
        });
        Ok(Some(candidates[0].clone()))
    } else {
        Ok(None)
    }
}

/// Recursively searches for game executables with depth limit.
fn find_exe_recursive(dir: &std::path::Path, current_depth: usize, max_depth: usize) -> Result<Option<PathBuf>> {
    if current_depth >= max_depth {
        return Ok(None);
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    let mut candidates = Vec::new();
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("exe") {
                    if should_skip_executable(&path) {
                        continue;
                    }
                    candidates.push(path);
                }
            }
        } else if path.is_dir() {
            // Skip common non-game directories
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let dir_name_lower = dir_name.to_lowercase();
                if dir_name_lower == "advguide"
                    || dir_name_lower == "easyanticheat"
                    || dir_name_lower == "_commonredist"
                    || dir_name_lower.starts_with(".")
                    || dir_name_lower.contains("redist") {
                    continue;
                }
            }
            subdirs.push(path);
        }
    }

    // If we found candidates at this level, return the best one
    if !candidates.is_empty() {
        candidates.sort_by_key(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.len())
                .unwrap_or(usize::MAX)
        });
        return Ok(Some(candidates[0].clone()));
    }

    // Otherwise, recurse into subdirectories
    for subdir in subdirs {
        if let Some(exe) = find_exe_recursive(&subdir, current_depth + 1, max_depth)? {
            return Ok(Some(exe));
        }
    }

    Ok(None)
}

/// Checks if an executable should be skipped based on its name.
fn should_skip_executable(path: &std::path::Path) -> bool {
    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
        let filename_lower = filename.to_lowercase();
        if filename_lower.contains("launcher")
            || filename_lower.contains("updater")
            || filename_lower.contains("crash")
            || filename_lower.contains("unins")
            || filename_lower.contains("setup")
            || filename_lower.contains("eac")
            || filename_lower.contains("anticheat")
            || filename_lower.contains("guide")
            || filename_lower == "unrealcefsubprocess"
            || filename_lower == "start_protected_game" {
            return true;
        }
    }
    false
}
