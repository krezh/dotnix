//! Upload service trait and implementations

use anyhow::Result;
use std::path::Path;

/// Common interface for file upload services
pub trait UploadService {
    /// Returns the display name of the service.
    fn name(&self) -> &'static str;
    /// Uploads a file and returns the public URL.
    fn upload(&self, file_path: &Path) -> Result<String>;
}
