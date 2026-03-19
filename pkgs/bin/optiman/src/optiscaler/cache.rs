use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Manages the cache for downloaded OptiScaler releases.
pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    /// Creates a new cache instance.
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        fs::create_dir_all(&cache_dir)
            .context("Failed to create cache directory")?;

        Ok(Self { cache_dir })
    }

    /// Gets the cache directory path.
    fn get_cache_dir() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Could not determine data directory")?
            .join("optiman")
            .join("cache");

        Ok(data_dir)
    }

    /// Gets the path for a cached version.
    fn version_path(&self, version: &str) -> PathBuf {
        self.cache_dir.join(version)
    }

    /// Checks if a version is cached.
    pub fn has_version(&self, version: &str) -> bool {
        let version_dir = self.version_path(version);
        version_dir.exists() && version_dir.is_dir()
    }

    /// Gets the path to a cached version's directory.
    pub fn get_version_path(&self, version: &str) -> Option<PathBuf> {
        let version_dir = self.version_path(version);
        if version_dir.exists() {
            Some(version_dir)
        } else {
            None
        }
    }

    /// Extracts an archive file to the cache for a specific version.
    pub fn cache_version_from_archive(&self, version: &str, archive_path: &Path) -> Result<PathBuf> {
        tracing::info!("Caching OptiScaler version {} from archive", version);

        let version_dir = self.version_path(version);

        if version_dir.exists() {
            tracing::info!("Version {} already cached, removing old cache", version);
            fs::remove_dir_all(&version_dir)
                .context("Failed to remove old cache")?;
        }

        fs::create_dir_all(&version_dir)
            .context("Failed to create version cache directory")?;

        // Determine archive type by extension
        let extension = archive_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "zip" => self.extract_zip(archive_path, &version_dir)?,
            "7z" => self.extract_7z(archive_path, &version_dir)?,
            _ => anyhow::bail!("Unsupported archive format: {}", extension),
        }

        tracing::info!("Cached OptiScaler version {} at {:?}", version, version_dir);
        Ok(version_dir)
    }

    fn extract_zip(&self, zip_path: &Path, dest_dir: &Path) -> Result<()> {
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

    fn extract_7z(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        sevenz_rust::decompress_file(archive_path, dest_dir)
            .map_err(|e| anyhow::anyhow!("Failed to extract 7z archive: {}", e))?;
        Ok(())
    }

    /// Removes a cached version.
    pub fn remove_version(&self, version: &str) -> Result<()> {
        let version_dir = self.version_path(version);
        if version_dir.exists() {
            fs::remove_dir_all(&version_dir)
                .context("Failed to remove cached version")?;
            tracing::info!("Removed cached version {}", version);
        }
        Ok(())
    }

    /// Lists all cached versions.
    pub fn list_versions(&self) -> Result<Vec<String>> {
        let mut versions = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(versions);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    versions.push(name.to_string());
                }
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Clears the entire cache.
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .context("Failed to clear cache")?;
            fs::create_dir_all(&self.cache_dir)
                .context("Failed to recreate cache directory")?;
            tracing::info!("Cleared OptiScaler cache");
        }
        Ok(())
    }
}
