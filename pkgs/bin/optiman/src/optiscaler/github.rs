use anyhow::{Context, Result};
use serde::Deserialize;

const OPTISCALER_REPO: &str = "optiscaler/OptiScaler";
const GITHUB_API_BASE: &str = "https://api.github.com";

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
    pub published_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

pub struct GitHubClient {
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn new() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-github-api-version"),
            reqwest::header::HeaderValue::from_static("2022-11-28"),
        );
        if let Some(token) = dirs::config_dir()
            .map(|p| p.join("optiman/github_token"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }

        let client = reqwest::Client::builder()
            .user_agent("OptiMan/0.1.0")
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Gets the latest stable release.
    pub async fn get_latest_release(&self) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/releases/latest",
            GITHUB_API_BASE, OPTISCALER_REPO
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch latest release")?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API returned error: {}", response.status());
        }

        let release: Release = response
            .json()
            .await
            .context("Failed to parse release JSON")?;

        Ok(release)
    }

    /// Gets the nightly release.
    pub async fn get_nightly_release(&self) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/releases/tags/nightly",
            GITHUB_API_BASE, OPTISCALER_REPO
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch nightly release")?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API returned error: {}", response.status());
        }

        let release: Release = response
            .json()
            .await
            .context("Failed to parse release JSON")?;

        Ok(release)
    }

    /// Gets all releases.
    pub async fn get_releases(&self) -> Result<Vec<Release>> {
        let url = format!("{}/repos/{}/releases", GITHUB_API_BASE, OPTISCALER_REPO);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch releases")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API {}: {}", status, body.trim());
        }

        let releases: Vec<Release> = response
            .json()
            .await
            .context("Failed to parse releases JSON")?;

        Ok(releases)
    }
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}
