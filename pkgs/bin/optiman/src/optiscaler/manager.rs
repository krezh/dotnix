use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::downloader::Downloader;
use super::github::Release;

const VERSION_FILE: &str = "version";
const FILES_DIR: &str = "files";

pub struct Manager {
    downloader: Downloader,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            downloader: Downloader::new(),
        }
    }

    fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .context("Could not determine data directory")?
            .join("optiman");
        fs::create_dir_all(&dir).context("Failed to create data directory")?;
        Ok(dir)
    }

    pub fn files_dir() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join(FILES_DIR))
    }

    fn version_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join(VERSION_FILE))
    }

    pub fn current_version() -> Option<String> {
        let path = Self::version_path().ok()?;
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    pub fn is_installed() -> bool {
        Self::files_dir()
            .map(|p| p.join("OptiScaler.dll").exists())
            .unwrap_or(false)
    }

    /// Downloads the given release and extracts all files to the optiman data directory.
    pub async fn install<F>(&self, release: &Release, mut progress: F) -> Result<()>
    where
        F: FnMut(u64, u64),
    {
        let asset = release
            .assets
            .iter()
            .find(|a| {
                let n = a.name.to_lowercase();
                n.ends_with(".zip") || n.ends_with(".7z")
            })
            .context("No archive asset found in release")?;

        let archive = self
            .downloader
            .download_to_temp(&asset.browser_download_url, &asset.name, &mut progress)
            .await?;

        let files_dir = Self::files_dir()?;
        if files_dir.exists() {
            fs::remove_dir_all(&files_dir).context("Failed to clear previous install")?;
        }
        fs::create_dir_all(&files_dir).context("Failed to create files directory")?;

        Self::extract(&archive, &files_dir)?;
        let _ = fs::remove_file(&archive);

        if !files_dir.join("OptiScaler.dll").exists() {
            anyhow::bail!("OptiScaler.dll not found in release archive");
        }

        fs::write(Self::version_path()?, &release.tag_name)
            .context("Failed to write version file")?;

        tracing::info!("Installed OptiScaler {} to {:?}", release.tag_name, files_dir);
        Ok(())
    }

    fn extract(archive: &Path, dest: &Path) -> Result<()> {
        match archive.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "zip" => {
                let file = fs::File::open(archive)?;
                let mut zip = zip::ZipArchive::new(file)?;
                for i in 0..zip.len() {
                    let mut entry = zip.by_index(i)?;
                    let out = match entry.enclosed_name() {
                        Some(p) => dest.join(p),
                        None => continue,
                    };
                    if entry.name().ends_with('/') {
                        fs::create_dir_all(&out)?;
                    } else {
                        if let Some(p) = out.parent() {
                            fs::create_dir_all(p)?;
                        }
                        std::io::copy(&mut entry, &mut fs::File::create(&out)?)?;
                    }
                }
            }
            "7z" => {
                sevenz_rust::decompress_file(archive, dest)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
            ext => anyhow::bail!("Unsupported archive format: {}", ext),
        }
        Ok(())
    }
}
