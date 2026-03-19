use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::{GameInfo, SteamGame};
use crate::game_detection::find_game_executable;

/// Checks if an app should be filtered out based on name or app ID.
fn should_filter_app(name: &str, app_id: &str) -> bool {
    let name_lower = name.to_lowercase();
    
    // Filter by name patterns
    if name_lower.starts_with("proton ") 
        || name_lower == "proton experimental"
        || name_lower.contains("proton ") && (name_lower.contains("runtime") || name_lower.contains("easyanticheat"))
        || name_lower.contains("steam linux runtime")
        || name_lower.contains("steam runtime")
        || name_lower.contains("redistributables") {
        return true;
    }
    
    // Filter by known tool app IDs
    // Proton versions: 1493710 (Experimental), 3658110 (10.0), 2180100 (9.0), 1887720 (Hotfix)
    // Steam Runtime: 1628350 (sniper), 1391110 (soldier), 1070560 (scout)
    // EAC Runtime: 1826330
    // Steamworks: 228980
    // Lossless Scaling: 993090
    let tool_app_ids = [
        "1493710", "3658110", "2180100", "1887720", "1391110",
        "1628350", "1070560", "1826330", "228980", "993090"
    ];
    
    if tool_app_ids.contains(&app_id) {
        return true;
    }
    
    false
}



/// Scans a Steam library directory for installed games.
pub fn scan_library(library_path: &Path) -> Result<Vec<SteamGame>> {
    let steamapps = library_path.join("steamapps");

    if !steamapps.exists() {
        anyhow::bail!("steamapps directory not found in library: {:?}", library_path);
    }

    let mut games = Vec::new();

    // Read all appmanifest_*.acf files
    let entries = fs::read_dir(&steamapps)
        .context("Failed to read steamapps directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with("appmanifest_") && filename.ends_with(".acf") {
                match parse_app_manifest(&path) {
                    Ok(game_info) => {
                        // Skip non-games (tools, runtimes, etc.)
                        if should_filter_app(&game_info.name, &game_info.app_id) {
                            continue;
                        }
                        
                        let install_path = steamapps
                            .join("common")
                            .join(&game_info.install_dir);

                        if install_path.exists() {
                            let mut game = SteamGame::new(
                                game_info.app_id,
                                game_info.name,
                                install_path,
                            );
                            
                            // Try to detect the game executable
                            match find_game_executable(&game) {
                                Ok(exe_path) => {
                                    tracing::info!("Detected executable for {}: {:?}", game.name, exe_path);
                                    game = game.with_executable(exe_path);
                                }
                                Err(e) => {
                                    tracing::warn!("Could not find executable for {}: {}", game.name, e);
                                }
                            }
                            
                            games.push(game);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    tracing::info!("Found {} games in library {:?}", games.len(), library_path);
    Ok(games)
}

/// Parses an appmanifest_*.acf file.
fn parse_app_manifest(path: &Path) -> Result<GameInfo> {
    let contents = fs::read_to_string(path)
        .context("Failed to read appmanifest file")?;

    let parsed = keyvalues_parser::Vdf::parse(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to parse VDF: {}", e))?;

    let mut app_id = None;
    let mut name = None;
    let mut install_dir = None;
    let mut state_flags = None;
    let mut has_language_config = false;

    // The root key is "AppState", properties are in parsed.value
    if let keyvalues_parser::Value::Obj(entries) = &parsed.value {
        for (key, values) in entries.iter() {
            match key.as_ref() {
                "appid" => {
                    for val in values.iter() {
                        if let keyvalues_parser::Value::Str(id) = val {
                            app_id = Some(id.to_string());
                        }
                    }
                }
                "name" => {
                    for val in values.iter() {
                        if let keyvalues_parser::Value::Str(n) = val {
                            name = Some(n.to_string());
                        }
                    }
                }
                "installdir" => {
                    for val in values.iter() {
                        if let keyvalues_parser::Value::Str(dir) = val {
                            install_dir = Some(dir.to_string());
                        }
                    }
                }
                "StateFlags" => {
                    for val in values.iter() {
                        if let keyvalues_parser::Value::Str(flags) = val {
                            state_flags = Some(flags.to_string());
                        }
                    }
                }
                "UserConfig" | "MountedConfig" => {
                    // Check if this config section has a language key
                    for val in values.iter() {
                        if let keyvalues_parser::Value::Obj(config_entries) = val {
                            for (config_key, _) in config_entries.iter() {
                                if config_key.as_ref() == "language" {
                                    has_language_config = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let app_id = app_id.context("Missing appid in appmanifest")?;
    let name = name.context("Missing name in appmanifest")?;
    let install_dir = install_dir.context("Missing installdir in appmanifest")?;

    Ok(GameInfo {
        app_id,
        name,
        install_dir,
        state_flags,
        has_language_config,
    })
}
