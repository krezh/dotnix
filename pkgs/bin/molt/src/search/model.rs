use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

pub(super) const PAGE_SIZE: usize = 50;
pub(super) const RESULT_WINDOW: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SearchScope {
    All,
    Packages,
    NixOsOptions,
    HomeManagerOptions,
}

impl SearchScope {
    pub(super) const ALL: [Self; 4] = [
        Self::All,
        Self::Packages,
        Self::NixOsOptions,
        Self::HomeManagerOptions,
    ];

    pub(super) fn types(self) -> &'static [&'static str] {
        match self {
            Self::All => &["package", "option", "home-manager-option"],
            Self::Packages => &["package"],
            Self::NixOsOptions => &["option"],
            Self::HomeManagerOptions => &["home-manager-option"],
        }
    }

    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|scope| *scope == self).unwrap()
    }
}

/// Splits a leading scope prefix (`p:`, `o:`, `hm:` and their long forms) off the typed text.
pub(super) fn scoped_query(text: &str) -> (Option<SearchScope>, &str) {
    const PREFIXES: [(&str, SearchScope); 6] = [
        ("p:", SearchScope::Packages),
        ("pkg:", SearchScope::Packages),
        ("o:", SearchScope::NixOsOptions),
        ("nixos:", SearchScope::NixOsOptions),
        ("hm:", SearchScope::HomeManagerOptions),
        ("home:", SearchScope::HomeManagerOptions),
    ];
    let trimmed = text.trim_start();
    for (prefix, scope) in PREFIXES {
        if trimmed
            .get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        {
            return (Some(scope), trimmed[prefix.len()..].trim_start());
        }
    }
    (None, text)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SearchRequest {
    pub generation: u64,
    pub query: String,
    pub scope: SearchScope,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SearchResultItem {
    Package(PackageHit),
    NixOsOption(OptionHit),
    HomeManagerOption(OptionHit),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct PackageHit {
    #[serde(rename = "package_attr_name")]
    pub attr_name: String,
    #[serde(rename = "package_pversion", default)]
    pub version: Option<String>,
    #[serde(rename = "package_description", default)]
    pub description: Option<String>,
    #[serde(rename = "package_position", default)]
    pub position: Option<String>,
    #[serde(
        rename = "package_programs",
        default,
        deserialize_with = "nullable_list"
    )]
    pub programs: Vec<String>,
    #[serde(
        rename = "package_license_set",
        default,
        deserialize_with = "nullable_list"
    )]
    pub licenses: Vec<String>,
    #[serde(rename = "package_homepage", default)]
    pub homepage: Value,
}

fn nullable_list<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

impl PackageHit {
    pub(super) fn homepage_url(&self) -> Option<&str> {
        fn valid(value: &Value) -> Option<&str> {
            value.as_str().filter(|url| {
                !url.chars().any(char::is_whitespace)
                    && gtk::glib::Uri::parse(url, gtk::glib::UriFlags::NONE).is_ok_and(|uri| {
                        matches!(uri.scheme().as_str(), "https" | "http")
                            && uri.host().is_some_and(|host| !host.is_empty())
                    })
            })
        }
        match &self.homepage {
            Value::Array(values) => values.iter().find_map(valid),
            value => valid(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct OptionHit {
    #[serde(rename = "option_name")]
    pub name: String,
    #[serde(rename = "option_description", default)]
    pub description: Option<String>,
    #[serde(rename = "option_type", default)]
    pub option_type: Option<String>,
    #[serde(rename = "option_source", default)]
    pub source: Option<String>,
    #[serde(rename = "option_default", default)]
    pub default_value: Value,
    #[serde(rename = "option_example", default)]
    pub example: Value,
}

impl OptionHit {
    /// A value to put after `name =` in a copied snippet: `true` for a switch, else the example,
    /// the default, or an empty value of the option's type.
    fn placeholder(&self) -> String {
        let kind = self.option_type.as_deref().unwrap_or_default();
        if kind == "boolean" {
            return "true".into();
        }
        nix_text(&self.example)
            .or_else(|| nix_text(&self.default_value))
            .unwrap_or_else(|| {
                if kind.starts_with("list of") {
                    "[ ]"
                } else if kind.starts_with("attribute set") || kind.starts_with("lazy attribute set")
                {
                    "{ }"
                } else if kind.starts_with("string") || kind.starts_with("strings") {
                    "\"\""
                } else {
                    "null"
                }
                .into()
            })
    }
}

/// The catalog stores defaults and examples as Nix source text; rendered Markdown is prose, not
/// a value.
fn nix_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim_start().starts_with("<rendered-html>") => {
            let text = text.trim().trim_end_matches(';').trim_end();
            (!text.is_empty()).then(|| text.to_owned())
        }
        Value::Bool(_) | Value::Number(_) => Some(value.to_string()),
        _ => None,
    }
}

/// A repository-relative path that is safe to append to a GitHub blob URL.
fn repository_path(path: &str) -> Option<&str> {
    (!path.is_empty()
        && !path.starts_with('/')
        && !path.split('/').any(|part| part.is_empty() || part == "..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./+".contains(&byte)))
    .then_some(path)
}

impl SearchResultItem {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Package(hit) => &hit.attr_name,
            Self::NixOsOption(hit) | Self::HomeManagerOption(hit) => &hit.name,
        }
    }

    /// Nix to paste into a configuration: the package reference or an option assignment.
    pub(super) fn snippet(&self) -> String {
        match self {
            Self::Package(hit) => format!("pkgs.{}", hit.attr_name),
            Self::NixOsOption(hit) | Self::HomeManagerOption(hit) => {
                format!("{} = {};", hit.name, hit.placeholder())
            }
        }
    }

    /// The declaration on GitHub. nixpkgs links use the catalog's revision when it is known;
    /// the catalog does not record which Home Manager commit it indexed.
    pub(super) fn source_url(&self, revision: Option<&str>) -> Option<String> {
        let nixpkgs = |path: &str, line: Option<&str>| {
            let path = repository_path(path)?;
            let line = line.map_or_else(String::new, |line| format!("#L{line}"));
            Some(format!(
                "https://github.com/NixOS/nixpkgs/blob/{}/{path}{line}",
                revision.unwrap_or("nixos-unstable")
            ))
        };
        match self {
            Self::Package(hit) => {
                let position = hit.position.as_deref()?;
                match position.rsplit_once(':') {
                    Some((path, line))
                        if !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit()) =>
                    {
                        nixpkgs(path, Some(line))
                    }
                    _ => nixpkgs(position, None),
                }
            }
            Self::NixOsOption(hit) => nixpkgs(hit.source.as_deref()?, None),
            Self::HomeManagerOption(hit) => {
                let path = repository_path(hit.source.as_deref()?)?;
                Some(format!(
                    "https://github.com/nix-community/home-manager/blob/master/{path}"
                ))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SearchResults {
    pub total: usize,
    pub elapsed: Duration,
    pub items: Vec<SearchResultItem>,
    /// The nixpkgs commit the catalog index was built from.
    pub revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn option(name: &str, kind: Option<&str>, default: Value, example: Value) -> OptionHit {
        OptionHit {
            name: name.into(),
            description: None,
            option_type: kind.map(str::to_owned),
            source: Some("nixos/modules/services/web-servers/nginx/default.nix".into()),
            default_value: default,
            example,
        }
    }

    #[test]
    fn prefixes_select_a_scope_and_are_stripped() {
        assert_eq!(
            scoped_query("p: ripgrep"),
            (Some(SearchScope::Packages), "ripgrep")
        );
        assert_eq!(
            scoped_query("  HM:programs.git"),
            (Some(SearchScope::HomeManagerOptions), "programs.git")
        );
        assert_eq!(
            scoped_query("nixos:nginx"),
            (Some(SearchScope::NixOsOptions), "nginx")
        );
        assert_eq!(scoped_query("python3"), (None, "python3"));
        assert_eq!(scoped_query("hmm"), (None, "hmm"));
        assert_eq!(scoped_query("é:"), (None, "é:"));
    }

    #[test]
    fn snippets_pick_a_value_that_fits_the_option() {
        let item = |hit| SearchResultItem::NixOsOption(hit).snippet();
        assert_eq!(
            item(option("a.enable", Some("boolean"), json!("false"), json!("true"))),
            "a.enable = true;"
        );
        assert_eq!(
            item(option(
                "a.hosts",
                Some("attribute set of (submodule)"),
                json!("{ }"),
                json!("{\n  x = { };\n};\n")
            )),
            "a.hosts = {\n  x = { };\n};"
        );
        assert_eq!(
            item(option("a.list", Some("list of string"), Value::Null, Value::Null)),
            "a.list = [ ];"
        );
        assert_eq!(
            item(option("a.user", Some("string"), json!("\"nginx\""), Value::Null)),
            "a.user = \"nginx\";"
        );
        assert_eq!(
            item(option(
                "a.port",
                Some("16 bit unsigned integer"),
                json!("8080"),
                json!("<rendered-html><p>A port</p></rendered-html>")
            )),
            "a.port = 8080;"
        );
        assert_eq!(
            item(option("a.package", Some("package"), Value::Null, Value::Null)),
            "a.package = null;"
        );
        let package = serde_json::from_value(json!({"package_attr_name": "ripgrep"})).unwrap();
        assert_eq!(SearchResultItem::Package(package).snippet(), "pkgs.ripgrep");
    }

    #[test]
    fn source_links_point_at_the_indexed_revision() {
        let package = |position: &str| {
            SearchResultItem::Package(
                serde_json::from_value(json!({
                    "package_attr_name": "ripgrep",
                    "package_position": position,
                }))
                .unwrap(),
            )
        };
        assert_eq!(
            package("pkgs/by-name/ri/ripgrep/package.nix:66")
                .source_url(Some("02c087f3"))
                .as_deref(),
            Some("https://github.com/NixOS/nixpkgs/blob/02c087f3/pkgs/by-name/ri/ripgrep/package.nix#L66")
        );
        assert_eq!(
            package("pkgs/top-level/all-packages.nix")
                .source_url(None)
                .as_deref(),
            Some("https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/top-level/all-packages.nix")
        );
        for invalid in ["/nix/store/x/default.nix:1", "../etc/passwd", "a b.nix", ""] {
            assert_eq!(package(invalid).source_url(None), None, "{invalid}");
        }
        let nixos = option("a", None, Value::Null, Value::Null);
        assert_eq!(
            SearchResultItem::NixOsOption(nixos.clone()).source_url(Some("abc")).as_deref(),
            Some("https://github.com/NixOS/nixpkgs/blob/abc/nixos/modules/services/web-servers/nginx/default.nix")
        );
        let home = OptionHit {
            source: Some("modules/programs/git.nix".into()),
            ..nixos
        };
        assert_eq!(
            SearchResultItem::HomeManagerOption(home).source_url(Some("abc")).as_deref(),
            Some("https://github.com/nix-community/home-manager/blob/master/modules/programs/git.nix")
        );
    }
}
