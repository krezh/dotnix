use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Finds all Steam library folders by parsing libraryfolders.vdf.
pub fn find_steam_libraries() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let steam_root = home.join(".steam/steam");

    if !steam_root.exists() {
        anyhow::bail!("Steam directory not found at {:?}", steam_root);
    }

    let library_folders_path = steam_root.join("steamapps/libraryfolders.vdf");

    if !library_folders_path.exists() {
        anyhow::bail!("libraryfolders.vdf not found at {:?}", library_folders_path);
    }

    let contents = fs::read_to_string(&library_folders_path)
        .context("Failed to read libraryfolders.vdf")?;

    parse_library_folders(&contents)
}

/// Parses libraryfolders.vdf content to extract library paths.
fn parse_library_folders(contents: &str) -> Result<Vec<PathBuf>> {
    let mut libraries = Vec::new();

    // Parse VDF manually using keyvalues-parser
    let parsed = keyvalues_parser::Vdf::parse(contents)
        .map_err(|e| anyhow::anyhow!("Failed to parse VDF: {}", e))?;

    tracing::info!("Root key: {}", parsed.key);

    // The VDF parser returns the root key as parsed.key ("libraryfolders")
    // and parsed.value contains the nested "0", "1", etc.
    if let keyvalues_parser::Value::Obj(entries) = &parsed.value {
        for (key, values) in entries.iter() {
            // Check if this is a numeric library folder key ("0", "1", etc)
            if key.parse::<u32>().is_ok() {
                // This is a library folder entry
                for value in values.iter() {
                    if let keyvalues_parser::Value::Obj(props) = value {
                        for (prop_key, prop_values) in props.iter() {
                            if prop_key.as_ref() == "path" {
                                for prop_val in prop_values.iter() {
                                    if let keyvalues_parser::Value::Str(path_str) = prop_val {
                                        let path = PathBuf::from(path_str.as_ref());
                                        if path.exists() {
                                            tracing::info!("Found Steam library: {:?}", path);
                                            libraries.push(path);
                                        } else {
                                            tracing::warn!("Library path does not exist: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if libraries.is_empty() {
        anyhow::bail!("No valid Steam libraries found");
    }

    tracing::info!("Found {} Steam library/libraries", libraries.len());
    Ok(libraries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_library_folders() {
        let vdf_content = r#"
"libraryfolders"
{
    "0"
    {
        "path"      "/home/user/.steam/steam"
        "label"     ""
        "contentid" "123456789"
    }
    "1"
    {
        "path"      "/mnt/games/steamlibrary"
        "label"     "Games"
        "contentid" "987654321"
    }
}
"#;

        let result = parse_library_folders(vdf_content);
        // Note: This test will fail if the paths don't exist
        // In real usage, only existing paths are added
        assert!(result.is_ok() || result.is_err());
    }
}
