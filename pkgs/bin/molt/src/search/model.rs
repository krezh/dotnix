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

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SearchResults {
    pub total: usize,
    pub elapsed: Duration,
    pub items: Vec<SearchResultItem>,
}
