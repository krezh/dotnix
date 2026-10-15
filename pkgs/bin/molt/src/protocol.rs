use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How the helper applies a NixOS closure.
///
/// Not part of the wire format: each mode is its own request operation, so a helper that predates
/// a mode rejects the request as unknown instead of silently performing a different activation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivationMode {
    #[default]
    Switch,
    Boot,
}

impl ActivationMode {
    /// The `switch-to-configuration` action for this mode.
    pub fn action(self) -> &'static str {
        match self {
            Self::Switch => "switch",
            Self::Boot => "boot",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum HelperRequest<'a> {
    Activate {
        request_id: &'a str,
        baseline: &'a Path,
        output: &'a Path,
    },
    ActivateOnBoot {
        request_id: &'a str,
        baseline: &'a Path,
        output: &'a Path,
    },
    Analyze {
        request_id: &'a str,
        output: &'a Path,
    },
    Cleanup {
        request_id: &'a str,
        nix: bool,
        journals: bool,
        keep_generations: usize,
        journal_max_age: &'a str,
    },
    Cancel {
        request_id: &'a str,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum OwnedHelperRequest {
    Activate {
        request_id: String,
        baseline: PathBuf,
        output: PathBuf,
    },
    ActivateOnBoot {
        request_id: String,
        baseline: PathBuf,
        output: PathBuf,
    },
    Analyze {
        request_id: String,
        output: PathBuf,
    },
    Cleanup {
        request_id: String,
        nix: bool,
        journals: bool,
        keep_generations: usize,
        journal_max_age: String,
    },
    Cancel {
        request_id: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HelperResponse {
    Ok {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    Error {
        message: String,
    },
}
