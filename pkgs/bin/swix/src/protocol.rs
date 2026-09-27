use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ActivationRequest<'a> {
    pub baseline: &'a Path,
    pub output: &'a Path,
}

#[derive(Debug, Deserialize)]
pub struct OwnedActivationRequest {
    pub baseline: PathBuf,
    pub output: PathBuf,
}
