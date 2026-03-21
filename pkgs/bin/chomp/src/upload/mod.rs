//! Upload domain
//!
//! Handles uploading files to remote services.
//!
//! ## Structure
//!
//! - **backend/** - Service-specific implementations (Zipline, Imgur, S3, etc.)
//! - **service.rs** - Common `UploadService` trait
//!
//! ## Adding New Upload Service
//!
//! 1. Create `backend/yourservice.rs`
//! 2. Implement the `UploadService` trait
//! 3. Export convenience function in this module

pub mod backend;
mod service;

pub use service::UploadService;

use anyhow::Result;

/// Uploads a file to Zipline and returns the public URL.
///
/// This is a convenience wrapper around the Zipline backend.
pub fn upload_to_zipline(
    zipline_url: &str,
    token_file: &str,
    use_original_name: bool,
    file_path: &str,
) -> Result<String> {
    use backend::zipline::ZiplineUploader;
    use service::UploadService;
    
    let uploader = ZiplineUploader::new(zipline_url, token_file, use_original_name)?;
    uploader.upload(file_path)
}
