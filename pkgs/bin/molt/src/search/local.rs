//! Looks a catalog result up in the user's own flake, whose locked nixpkgs and modules can differ
//! from the unstable catalog.

use std::{process::Command, sync::atomic::AtomicBool, time::Duration};

use molt::command::{self, OutputLimits};
use serde_json::Value;

use super::model::SearchResultItem;
use crate::config::Config;

const NIX: &str = "nix";
const TIMEOUT: Duration = Duration::from_secs(180);
const MAX_OUTPUT: usize = 1 << 20;
/// Longer values are cut so the inspector stays readable.
const MAX_SHOWN: usize = 4000;
/// Option definitions below `mkOptionDefault` (1500) come from a configuration, not the module.
const SET_IN_CONFIGURATION: &str = r#"o: { set = (o.highestPrio or 1500) < 1500; value = if o.isDefined then o.value else null; }"#;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct LocalValue {
    /// What was evaluated, as a flake attribute.
    pub attribute: String,
    /// `None` when the attribute path cannot tell a definition from the default.
    pub set: Option<bool>,
    pub value: String,
}

/// How a result is evaluated: the flake attribute, the `--apply` function and whether the result
/// is the `{ set, value }` record.
fn plan(
    config: &Config,
    item: &SearchResultItem,
    user: &str,
) -> Result<(String, &'static str, bool), String> {
    let path = attribute_path(item.name())
        .ok_or("This name has placeholders or quoting that cannot be evaluated directly")?;
    let host = &config.nixos_flake;
    Ok(match item {
        SearchResultItem::Package(_) => (
            format!("nixosConfigurations.{host}.pkgs.{path}"),
            r#"p: p.version or p.name or "(no version)""#,
            false,
        ),
        SearchResultItem::NixOsOption(_) => (
            format!("nixosConfigurations.{host}.options.{path}"),
            SET_IN_CONFIGURATION,
            true,
        ),
        SearchResultItem::HomeManagerOption(_) => match &config.home_flake {
            Some(home) => (
                format!("homeConfigurations.{home}.options.{path}"),
                SET_IN_CONFIGURATION,
                true,
            ),
            None => {
                attribute_path(user).ok_or("The current user name cannot be evaluated")?;
                (
                    format!("nixosConfigurations.{host}.config.home-manager.users.{user}.{path}"),
                    "v: v",
                    false,
                )
            }
        },
    })
}

/// The name as a Nix attribute path, when every segment is a plain identifier.
fn attribute_path(name: &str) -> Option<&str> {
    name.split('.')
        .all(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                && characters.all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '\'')
                })
        })
        .then_some(name)
}

pub(super) fn evaluate(
    config: &Config,
    item: &SearchResultItem,
    cancellation: &AtomicBool,
) -> Result<LocalValue, String> {
    let user = std::env::var("USER").unwrap_or_default();
    let (attribute, apply, record) = plan(config, item, &user)?;
    let output = command::run(
        Command::new(NIX)
            .args(["eval", "--json", "--apply", apply])
            .arg(format!(".#{attribute}"))
            .current_dir(&config.flake_dir),
        "nix eval",
        cancellation,
        TIMEOUT,
        OutputLimits {
            stdout: MAX_OUTPUT,
            stderr: MAX_OUTPUT,
        },
        |_| {},
        |stderr| last_error(&String::from_utf8_lossy(stderr)),
    )?;
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("nix eval returned invalid JSON: {error}"))?;
    let (set, value) = if record {
        (
            value.get("set").and_then(Value::as_bool),
            value.get("value").cloned().unwrap_or(Value::Null),
        )
    } else {
        (None, value)
    };
    Ok(LocalValue {
        attribute: format!(".#{attribute}"),
        set,
        value: shown(&value),
    })
}

fn shown(value: &Value) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        value => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
    };
    match text.char_indices().nth(MAX_SHOWN) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

/// Nix prints the evaluation trace before the error that caused it; the last `error:` line is the
/// cause.
fn last_error(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .rfind(|line| line.starts_with("error:"))
        .or_else(|| stderr.lines().map(str::trim).rfind(|line| !line.is_empty()))
        .map_or_else(|| "nix eval failed".to_owned(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn config(home_flake: Option<&str>) -> Config {
        toml::from_str(&format!(
            "flake_dir = \"/flake\"\nnixos_flake = \"host\"\n{}",
            home_flake.map_or_else(String::new, |home| format!("home_flake = \"{home}\"\n"))
        ))
        .unwrap()
    }

    fn item(kind: &str, name: &str) -> SearchResultItem {
        let field = if kind == "package" {
            "package_attr_name"
        } else {
            "option_name"
        };
        let source = json!({ field: name });
        match kind {
            "package" => SearchResultItem::Package(serde_json::from_value(source).unwrap()),
            "option" => SearchResultItem::NixOsOption(serde_json::from_value(source).unwrap()),
            _ => SearchResultItem::HomeManagerOption(serde_json::from_value(source).unwrap()),
        }
    }

    #[test]
    fn plans_attributes_for_each_kind_of_result() {
        let nixos = config(None);
        assert_eq!(
            plan(&nixos, &item("package", "python313Packages.requests"), "me")
                .unwrap()
                .0,
            "nixosConfigurations.host.pkgs.python313Packages.requests"
        );
        assert_eq!(
            plan(&nixos, &item("option", "services.nginx.enable"), "me").unwrap(),
            (
                "nixosConfigurations.host.options.services.nginx.enable".into(),
                SET_IN_CONFIGURATION,
                true
            )
        );
        assert_eq!(
            plan(&nixos, &item("home", "programs.git.enable"), "me").unwrap(),
            (
                "nixosConfigurations.host.config.home-manager.users.me.programs.git.enable".into(),
                "v: v",
                false
            )
        );
        assert_eq!(
            plan(
                &config(Some("me@host")),
                &item("home", "programs.git.enable"),
                "me"
            )
            .unwrap()
            .0,
            "homeConfigurations.me@host.options.programs.git.enable"
        );
        for name in [
            "services.nginx.virtualHosts.<name>.root",
            "a.\"b\".c",
            "a..b",
            "1a",
        ] {
            assert!(plan(&nixos, &item("option", name), "me").is_err(), "{name}");
        }
        assert!(plan(&nixos, &item("home", "programs.git.enable"), "").is_err());
    }

    #[test]
    fn reports_the_causing_error_and_bounds_values() {
        assert_eq!(
            last_error("error:\n  … while evaluating\n\n  error: attribute 'x' missing\n"),
            "error: attribute 'x' missing"
        );
        assert_eq!(last_error("warning: dirty\n"), "warning: dirty");
        assert_eq!(last_error(""), "nix eval failed");
        assert_eq!(shown(&json!("15.2.0")), "15.2.0");
        assert_eq!(shown(&json!(["a"])), "[\n  \"a\"\n]");
        assert_eq!(
            shown(&json!("x".repeat(MAX_SHOWN + 5))).chars().count(),
            MAX_SHOWN + 1
        );
    }
}
