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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_changelog_output() {
        let changelog = parse_changelog(
            br#"{"pname":"demo","version":"2.0","description":null,"releases":[]}"#,
        )
        .unwrap();
        assert_eq!(changelog.pname, "demo");
        assert!(parse_changelog(b"not json").is_err());
    }
}
