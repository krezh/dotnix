use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config::TrackedGames;
use crate::steam::SteamGame;
use super::github::Release;
use super::downloader::Downloader;
use super::cache::Cache;

pub struct Installer {
    tracked_games: Arc<Mutex<TrackedGames>>,
    downloader: Downloader,
    cache: Cache,
}

impl Installer {
    pub fn new(tracked_games: Arc<Mutex<TrackedGames>>) -> Result<Self> {
        Ok(Self {
            tracked_games,
            downloader: Downloader::new(),
            cache: Cache::new()?,
        })
    }

    /// Extracts an archive file to a destination directory.
    fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<()> {
        tracing::info!("Extracting {:?} to {:?}", archive_path, dest_dir);

        fs::create_dir_all(dest_dir)
            .context("Failed to create extraction directory")?;

        // Determine archive type by extension
        let extension = archive_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "zip" => Self::extract_zip(archive_path, dest_dir)?,
            "7z" => Self::extract_7z(archive_path, dest_dir)?,
            _ => anyhow::bail!("Unsupported archive format: {}", extension),
        }

        tracing::info!("Extraction complete");
        Ok(())
    }

    /// Extracts a ZIP file to a destination directory.
    fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = fs::File::open(zip_path)
            .context("Failed to open ZIP file")?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read ZIP archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .context("Failed to read ZIP entry")?;

            let outpath = match file.enclosed_name() {
                Some(path) => dest_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .context("Failed to create directory")?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)
                        .context("Failed to create parent directory")?;
                }

                let mut outfile = fs::File::create(&outpath)
                    .context("Failed to create file")?;

                std::io::copy(&mut file, &mut outfile)
                    .context("Failed to extract file")?;
            }
        }

        Ok(())
    }

    /// Extracts a 7z file to a destination directory.
    fn extract_7z(archive_path: &Path, dest_dir: &Path) -> Result<()> {
        sevenz_rust::decompress_file(archive_path, dest_dir)
            .map_err(|e| anyhow::anyhow!("Failed to extract 7z archive: {}", e))?;
        Ok(())
    }

    /// Creates a default OptiScaler.ini file.
    fn create_default_ini(path: &Path) -> Result<()> {
        let default_ini = r#"[General]
LogLevel=2
LogToFile=false

[Upscaler]
UpscalerOutput=FSR31

[FrameGen]
FrameGenOutput=FSR3FG
"#;

        fs::write(path, default_ini)
            .context("Failed to write default OptiScaler.ini")?;

        tracing::info!("Created default OptiScaler.ini at {:?}", path);
        Ok(())
    }

    /// Installs OptiScaler to a game.
    pub async fn install<F>(
        &self,
        game: &SteamGame,
        release: &Release,
        proxy_dll: &str,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(u64, u64),
    {
        tracing::info!("Installing OptiScaler {} to {}", release.tag_name, game.name);

        // Check if this version is already cached
        let extract_dir = if self.cache.has_version(&release.tag_name) {
            tracing::info!("Using cached version {}", release.tag_name);
            self.cache.get_version_path(&release.tag_name)
                .context("Failed to get cached version path")?
        } else {
            tracing::info!("Version {} not cached, downloading", release.tag_name);

            // Find the asset to download (look for archive files)
            tracing::debug!("Available assets: {:?}", release.assets.iter().map(|a| &a.name).collect::<Vec<_>>());
            
            let asset = release.assets.iter()
                .find(|a| {
                    let name_lower = a.name.to_lowercase();
                    name_lower.ends_with(".zip") || 
                    name_lower.ends_with(".7z") || 
                    name_lower.ends_with(".rar")
                })
                .context(format!(
                    "No archive asset found in release. Available assets: {}",
                    release.assets.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                ))?;

            // Download the release
            let archive_path = self.downloader.download_to_temp(
                &asset.browser_download_url,
                &asset.name,
                &mut progress_callback,
            ).await?;

            // Cache the version
            let cached_path = self.cache.cache_version_from_archive(&release.tag_name, &archive_path)?;

            // Clean up downloaded archive
            let _ = fs::remove_file(archive_path);

            cached_path
        };

        // Get the directory where the game executable is located
        let target_dir = game.executable_path.as_ref()
            .and_then(|p| p.parent())
            .context("Could not determine executable directory - no executable path found")?;

        tracing::info!("Installing to directory: {:?}", target_dir);

        // Copy OptiScaler files to game directory
        self.copy_optiscaler_files(&extract_dir, target_dir)?;

        // Rename OptiScaler.dll to the chosen proxy DLL
        let optiscaler_dll = target_dir.join("OptiScaler.dll");
        let proxy_dll_path = target_dir.join(proxy_dll);

        if optiscaler_dll.exists() {
            fs::rename(&optiscaler_dll, &proxy_dll_path)
                .context("Failed to rename OptiScaler.dll")?;
            tracing::info!("Renamed OptiScaler.dll to {}", proxy_dll);
        }

        // Create default INI if it doesn't exist
        let ini_path = target_dir.join("OptiScaler.ini");
        if !ini_path.exists() {
            Self::create_default_ini(&ini_path)?;
        }

        // Update tracked games
        if let Ok(mut tracked) = self.tracked_games.lock() {
            if let Some(entry) = tracked.get_game_mut(&game.app_id) {
                entry.installed = true;
                entry.optiscaler_version = Some(release.tag_name.clone());
                entry.proxy_dll = Some(proxy_dll.to_string());
                entry.installed_date = Some(chrono::Utc::now());
            }
            tracked.save()?;
        }

        tracing::info!("OptiScaler installation complete");
        Ok(())
    }

    /// Copies OptiScaler files from the extracted directory to the game directory.
    fn copy_optiscaler_files(&self, source_dir: &Path, dest_dir: &Path) -> Result<()> {
        // OptiScaler files typically include:
        // - OptiScaler.dll
        // - nvngx_dlss.dll (if bundled)
        // - Other dependencies

        for entry in fs::read_dir(source_dir)
            .context("Failed to read source directory")? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source_path = entry.path();
            let file_name = entry.file_name();

            if file_type.is_file() {
                let dest_path = dest_dir.join(&file_name);
                fs::copy(&source_path, &dest_path)
                    .with_context(|| format!("Failed to copy {:?}", file_name))?;
                tracing::debug!("Copied {:?}", file_name);
            } else if file_type.is_dir() {
                // Recursively copy subdirectories
                let dest_subdir = dest_dir.join(&file_name);
                self.copy_directory_recursive(&source_path, &dest_subdir)?;
            }
        }

        Ok(())
    }

    /// Recursively copies a directory.
    fn copy_directory_recursive(&self, source: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest)
            .context("Failed to create destination directory")?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if entry.file_type()?.is_dir() {
                self.copy_directory_recursive(&source_path, &dest_path)?;
            } else {
                fs::copy(&source_path, &dest_path)?;
            }
        }

        Ok(())
    }

    /// Removes OptiScaler from a game.
    pub fn remove(&self, game: &SteamGame) -> Result<()> {
        tracing::info!("Removing OptiScaler from {}", game.name);

        let target_dir = game.executable_path.as_ref()
            .and_then(|p| p.parent())
            .context("Could not determine executable directory - no executable path found")?;

        // Get proxy DLL name from tracked games
        let proxy_dll = if let Ok(tracked) = self.tracked_games.lock() {
            tracked.get_game(&game.app_id)
                .and_then(|e| e.proxy_dll.clone())
        } else {
            None
        };

        // Remove the proxy DLL
        if let Some(proxy_dll_name) = proxy_dll {
            let proxy_path = target_dir.join(&proxy_dll_name);
            if proxy_path.exists() {
                fs::remove_file(&proxy_path)
                    .context("Failed to remove proxy DLL")?;
                tracing::info!("Removed {}", proxy_dll_name);
            }
        }

        // Remove other OptiScaler files and directories
        let files_to_remove = [
            "OptiScaler.log",
            "OptiScaler.ini",
            "fakenvapi.dll",
            "fakenvapi.ini",
            "fakenvapi.log",
            "dlssg_to_fsr3_amd_is_better.dll",
            "dlssg_to_fsr3.log",
            "nvngx_dlss.dll",
            "nvngx.dll",
        ];
        
        for file in files_to_remove {
            let path = target_dir.join(file);
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    tracing::warn!("Failed to remove {}: {}", file, e);
                } else {
                    tracing::debug!("Removed {}", file);
                }
            }
        }
        
        // Remove directories
        let dirs_to_remove = ["D3D12_Optiscaler", "DlssOverrides", "Licenses"];
        for dir in dirs_to_remove {
            let path = target_dir.join(dir);
            if path.exists() && path.is_dir() {
                if let Err(e) = fs::remove_dir_all(&path) {
                    tracing::warn!("Failed to remove directory {}: {}", dir, e);
                } else {
                    tracing::debug!("Removed directory {}", dir);
                }
            }
        }

        // Update tracked games
        if let Ok(mut tracked) = self.tracked_games.lock() {
            if let Some(entry) = tracked.get_game_mut(&game.app_id) {
                entry.installed = false;
                entry.optiscaler_version = None;
                entry.proxy_dll = None;
                entry.installed_date = None;
            }
            tracked.save()?;
        }

        tracing::info!("OptiScaler removal complete");
        Ok(())
    }

    /// Verifies if OptiScaler is installed for a game.
    pub fn verify_installation(&self, game: &SteamGame) -> bool {
        if let Ok(tracked) = self.tracked_games.lock() {
            if let Some(entry) = tracked.get_game(&game.app_id) {
                if !entry.installed {
                    return false;
                }

                // Verify the proxy DLL actually exists
                if let Some(proxy_dll) = &entry.proxy_dll {
                    if let Some(target_dir) = game.executable_path.as_ref().and_then(|p| p.parent()) {
                        let proxy_path = target_dir.join(proxy_dll);
                        return proxy_path.exists();
                    }
                    return false;
                }
            }
        }

        false
    }
}
