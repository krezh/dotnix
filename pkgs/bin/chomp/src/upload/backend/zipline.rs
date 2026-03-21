//! Zipline upload functionality

use anyhow::{Context, Result};
use reqwest::blocking::{multipart, Client};
use serde::Deserialize;
use std::fs;
use std::time::Duration;

use crate::upload::service::UploadService;

#[derive(Deserialize)]
struct ZiplineResponse {
    files: Vec<ZiplineFile>,
}

#[derive(Deserialize)]
struct ZiplineFile {
    url: String,
}

pub struct ZiplineUploader {
    client: Client,
    url: String,
    token: String,
    use_original_name: bool,
}

impl ZiplineUploader {
    pub fn new(zipline_url: &str, token_file: &str, use_original_name: bool) -> Result<Self> {
        let token = fs::read_to_string(token_file)
            .context("Failed to read Zipline token file")?
            .trim()
            .to_string();

        if token.is_empty() {
            anyhow::bail!("Zipline token is empty");
        }

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            url: zipline_url.to_string(),
            token,
            use_original_name,
        })
    }

}

impl UploadService for ZiplineUploader {
    fn upload(&self, file_path: &str) -> Result<String> {
        let path = std::path::Path::new(file_path);
        let file_name = path
            .file_name()
            .context("Invalid file path")?
            .to_string_lossy()
            .to_string();

        let file_content = fs::read(file_path).context("Failed to read file for upload")?;

        // Determine MIME type based on file extension
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("mkv") => "video/x-matroska",
            _ => "application/octet-stream",
        };

        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(file_content)
                .file_name(file_name)
                .mime_str(mime_type)?,
        );

        let response = self
            .client
            .post(format!("{}/api/upload", self.url))
            .header("Authorization", &self.token)
            .header(
                "x-zipline-original-name",
                self.use_original_name.to_string(),
            )
            .header("User-Agent", "chomp/1.0")
            .multipart(form)
            .send()
            .context("Failed to send upload request")?;

        if !response.status().is_success() {
            anyhow::bail!("Upload failed with status: {}", response.status());
        }

        let body = response.text().context("Failed to read response body")?;
        let zipline_response: ZiplineResponse = serde_json::from_str(&body)
            .context("Failed to parse Zipline response")?;

        if zipline_response.files.is_empty() {
            anyhow::bail!("No files in Zipline response");
        }

        Ok(zipline_response.files[0].url.clone())
    }
}
