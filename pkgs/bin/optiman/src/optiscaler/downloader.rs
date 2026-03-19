use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("OptiMan/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Downloads a file from the given URL to the specified path.
    /// Calls the progress callback with (bytes_downloaded, total_bytes).
    pub async fn download_file<F>(
        &self,
        url: &str,
        dest: &PathBuf,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(u64, u64),
    {
        tracing::info!("Downloading from: {}", url);

        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            anyhow::bail!("Download failed with status: {}", response.status());
        }

        let total_size = response.content_length().unwrap_or(0);
        tracing::info!("Download size: {} bytes", total_size);

        let mut file = File::create(dest)
            .await
            .context("Failed to create destination file")?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write chunk")?;

            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        file.flush().await.context("Failed to flush file")?;
        tracing::info!("Download complete: {:?}", dest);

        Ok(())
    }

    /// Downloads to a temporary file and returns its path.
    pub async fn download_to_temp<F>(
        &self,
        url: &str,
        filename: &str,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: FnMut(u64, u64),
    {
        let temp_dir = std::env::temp_dir().join("optiman");
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;

        let dest = temp_dir.join(filename);
        self.download_file(url, &dest, progress_callback).await?;

        Ok(dest)
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
