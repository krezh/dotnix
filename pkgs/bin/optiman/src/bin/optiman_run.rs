use std::cmp::Reverse;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

macro_rules! log {
    ($($arg:tt)*) => { eprintln!("[optiman-run] {}", format!($($arg)*)) };
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: optiman-run [--uninstall] <command> [args...]");
        std::process::exit(1);
    }

    let uninstall = args[0] == "--uninstall";
    let cmd_args = if uninstall { &args[1..] } else { &args[..] };

    if uninstall && cmd_args.is_empty() {
        eprintln!("Usage: optiman-run --uninstall <command> [args...]");
        std::process::exit(1);
    }

    match detect_game_dir(cmd_args) {
        Some(game_dir) => {
            log!("game dir: {}", game_dir.display());
            if uninstall {
                match uninstall_from_game(&game_dir) {
                    Ok(()) => log!("uninstall complete"),
                    Err(e) => log!("uninstall failed: {}", e),
                }
                return;
            }
            match deploy_to_game(&game_dir) {
                Ok(()) => log!("deploy complete"),
                Err(e) => log!("deploy failed: {}", e),
            }
        }
        None => log!("could not detect game directory"),
    }

    if uninstall {
        return;
    }

    let current = std::env::var("WINEDLLOVERRIDES").unwrap_or_default();
    let overrides = if current.is_empty() {
        "dxgi=n,b".to_string()
    } else {
        format!("{},dxgi=n,b", current)
    };

    log!("WINEDLLOVERRIDES={}", overrides);
    log!("exec: {}", cmd_args[0]);

    let err = std::process::Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .env("WINEDLLOVERRIDES", overrides)
        .exec();

    log!("exec failed: {}", err);
    std::process::exit(1);
}

const BAD_NAME_FRAGMENTS: &[&str] = &[
    "launcher", "setup", "install", "uninstall", "updater", "update",
    "subprocess", "helper", "crash", "report", "cef", "dotnet", "vcredist",
];

const BAD_PATH_FRAGMENTS: &[&str] = &[
    "launcher", "runtimes", "runtime", "redist", "cef", "dotnet",
    "crash", "helper", "support",
];

fn is_launcher_exe(p: &Path) -> bool {
    let name = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if BAD_NAME_FRAGMENTS.iter().any(|f| name.contains(f)) {
        return true;
    }

    p.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        BAD_PATH_FRAGMENTS.iter().any(|f| s.contains(f))
    })
}

fn detect_game_dir(args: &[String]) -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;

    for arg in args {
        let p = PathBuf::from(arg);
        if p.extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            if is_launcher_exe(&p) {
                log!("skipping launcher exe: {}", p.display());
                fallback = fallback.or_else(|| p.parent().map(|d| d.to_path_buf()));
                continue;
            }
            if let Some(parent) = p.parent() {
                log!("found exe: {}", p.display());
                return Some(parent.to_path_buf());
            }
        }
    }

    if let Ok(path) = std::env::var("STEAM_COMPAT_INSTALL_PATH") {
        let root = PathBuf::from(&path);
        if root.exists() {
            if let Some(game_exe) = find_game_exe(&root) {
                log!("found game exe in install tree: {}", game_exe.display());
                return game_exe.parent().map(|p| p.to_path_buf());
            }
            log!("using STEAM_COMPAT_INSTALL_PATH root: {}", path);
            return Some(root);
        }
    }

    if let Some(ref d) = fallback {
        log!("falling back to launcher dir: {}", d.display());
    }
    fallback
}

fn find_game_exe(root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    collect_exes(root, 0, 5, &mut candidates);
    candidates.sort_by_key(|p| Reverse(p.components().count()));
    candidates.into_iter().next()
}

fn collect_exes(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_exes(&path, depth + 1, max_depth, out);
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
            && !is_launcher_exe(&path)
        {
            out.push(path);
        }
    }
}

fn deployed_dll_names(src: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("dll"))
                    .unwrap_or(false)
            {
                let fname = path.file_name().unwrap().to_string_lossy();
                let dest = if fname == "OptiScaler.dll" {
                    "dxgi.dll".to_string()
                } else {
                    fname.to_string()
                };
                names.push(dest);
            }
        }
    }
    names
}

fn deploy_to_game(game_dir: &Path) -> Result<(), String> {
    let src = dirs::data_dir()
        .ok_or_else(|| "could not determine data directory".to_string())?
        .join("optiman/files");

    log!("source dir: {}", src.display());

    if !src.join("OptiScaler.dll").exists() {
        return Err("OptiScaler not installed — run optiman to download it".to_string());
    }

    for entry in std::fs::read_dir(&src).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("dll"))
                .unwrap_or(false)
        {
            let fname = path.file_name().unwrap();
            let dest_name = if fname == "OptiScaler.dll" {
                "dxgi.dll".to_string()
            } else {
                fname.to_string_lossy().to_string()
            };
            log!("copy {} -> {}", fname.to_string_lossy(), dest_name);
            std::fs::copy(&path, game_dir.join(&dest_name))
                .map_err(|e| format!("failed to copy {}: {}", dest_name, e))?;
        }
    }

    for name in ["D3D12_Optiscaler", "D3D12_OptiScaler"] {
        let d = src.join(name);
        if d.is_dir() {
            log!("copy dir: {}", name);
            copy_dir_all(&d, &game_dir.join(name))
                .map_err(|e| format!("failed to copy {}: {}", name, e))?;
            break;
        }
    }

    for ini in ["OptiScaler.ini", "fakenvapi.ini"] {
        let s = src.join(ini);
        let d = game_dir.join(ini);
        if s.exists() && !d.exists() {
            log!("copy ini (first time): {}", ini);
            std::fs::copy(&s, &d).map_err(|e| format!("failed to copy {}: {}", ini, e))?;
        } else if d.exists() {
            log!("preserving existing: {}", ini);
        }
    }

    write_uninstaller(game_dir, &deployed_dll_names(&src))?;

    Ok(())
}

fn write_uninstaller(game_dir: &Path, dll_names: &[String]) -> Result<(), String> {
    let mut lines = vec![
        "#!/usr/bin/env bash".to_string(),
        r#"set -e"#.to_string(),
        r#"DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)""#.to_string(),
        r#"echo "Removing OptiScaler from $DIR...""#.to_string(),
    ];

    for name in dll_names {
        lines.push(format!(r#"rm -f "$DIR/{name}""#));
    }
    for ini in ["OptiScaler.ini", "fakenvapi.ini", "OptiScaler.log", "fakenvapi.log"] {
        lines.push(format!(r#"rm -f "$DIR/{ini}""#));
    }
    for dir in ["D3D12_Optiscaler", "D3D12_OptiScaler"] {
        lines.push(format!(r#"rm -rf "$DIR/{dir}""#));
    }
    lines.push(r#"rm -f "$DIR/remove_optiscaler.sh""#.to_string());
    lines.push(r#"echo "Done.""#.to_string());

    let script = lines.join("\n") + "\n";
    let path = game_dir.join("remove_optiscaler.sh");
    std::fs::write(&path, script).map_err(|e| format!("failed to write uninstaller: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to chmod uninstaller: {}", e))?;
    }

    log!("wrote remove_optiscaler.sh");
    Ok(())
}

fn uninstall_from_game(game_dir: &Path) -> Result<(), String> {
    let src = dirs::data_dir()
        .ok_or_else(|| "could not determine data directory".to_string())?
        .join("optiman/files");

    for name in deployed_dll_names(&src) {
        let f = game_dir.join(&name);
        if f.exists() {
            log!("remove: {}", name);
            std::fs::remove_file(&f).map_err(|e| format!("failed to remove {}: {}", name, e))?;
        }
    }

    for name in ["D3D12_Optiscaler", "D3D12_OptiScaler"] {
        let d = game_dir.join(name);
        if d.is_dir() {
            log!("remove dir: {}", name);
            std::fs::remove_dir_all(&d)
                .map_err(|e| format!("failed to remove {}: {}", name, e))?;
            break;
        }
    }

    for ini in ["OptiScaler.ini", "fakenvapi.ini"] {
        let f = game_dir.join(ini);
        if f.exists() {
            log!("remove: {}", ini);
            std::fs::remove_file(&f).map_err(|e| format!("failed to remove {}: {}", ini, e))?;
        }
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
