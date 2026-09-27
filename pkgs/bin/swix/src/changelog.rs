use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(crate) struct ChangelogOutput {
    pub(crate) pname: String,
    pub(crate) version: String,
    pub(crate) description: Option<String>,
    pub(crate) releases: Vec<ChangelogRelease>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct ChangelogRelease {
    pub(crate) tag: String,
    pub(crate) body: String,
}
pub(crate) fn parse_changelog(json: &[u8]) -> Result<ChangelogOutput, String> {
    serde_json::from_slice(json)
        .map_err(|error| format!("nix-changelog returned invalid data: {error}"))
}
