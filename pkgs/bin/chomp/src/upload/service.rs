//! Upload service trait and implementations

use anyhow::Result;

/// Common interface for file upload services
pub trait UploadService {
    /// Uploads a file and returns the public URL.
    fn upload(&self, file_path: &str) -> Result<String>;
}
