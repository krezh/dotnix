use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum HelperRequest<'a> {
    Activate {
        baseline: &'a Path,
        output: &'a Path,
    },
    Cleanup {
        nix: bool,
        journals: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum OwnedHelperRequest {
    Activate { baseline: PathBuf, output: PathBuf },
    Cleanup { nix: bool, journals: bool },
}
