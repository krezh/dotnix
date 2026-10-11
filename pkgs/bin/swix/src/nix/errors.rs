//! Picking the most useful error out of nix output, and the derivations it blames.

use std::cell::RefCell;

use serde_json::Value;

use super::strip_ansi;

pub(crate) fn nix_error_message(stderr: &[u8]) -> Option<String> {
    let best = RefCell::new(None);
    for line in String::from_utf8_lossy(stderr).lines() {
        update_best_error(&best, line);
    }
    best.into_inner().map(|(_, message)| message)
}

pub(super) fn update_best_error(best: &RefCell<Option<(u8, String)>>, line: &str) {
    let Some(event) = line
        .strip_prefix("@nix ")
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
    else {
        return;
    };
    if event.get("action").and_then(Value::as_str) != Some("msg")
        || event
            .get("level")
            .and_then(Value::as_u64)
            .is_some_and(|level| level != 0)
    {
        return;
    }
    let Some(message) = event
        .get("raw_msg")
        .or_else(|| event.get("msg"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let message = strip_ansi(message).trim().to_owned();
    if message.is_empty() {
        return;
    }
    let priority = error_priority(&message);
    if best
        .borrow()
        .as_ref()
        .is_none_or(|(best_priority, _)| priority > *best_priority)
    {
        best.replace(Some((priority, message)));
    }
}

fn error_priority(message: &str) -> u8 {
    if is_dependency_failure(message) {
        0
    } else if !failed_derivations(message).is_empty() {
        if message.to_lowercase().contains(" log lines:") {
            4
        } else {
            3
        }
    } else if message
        .lines()
        .any(|line| line.trim_start().starts_with("error:"))
    {
        2
    } else {
        1
    }
}

fn is_dependency_failure(message: &str) -> bool {
    let message = message
        .trim()
        .strip_prefix("error:")
        .unwrap_or(message)
        .trim();
    (message.contains("dependencies of derivation '") && message.ends_with("failed to build"))
        || (message.starts_with("Cannot build '")
            && failure_reason(&message.lines().collect::<Vec<_>>(), 0)
                .is_some_and(is_dependency_reason))
}

/// Derivations whose own build failed, in the order an error message names them. Derivations
/// that failed only because a dependency did are left out.
pub(crate) fn failed_derivations(message: &str) -> Vec<String> {
    let lines = message.lines().collect::<Vec<_>>();
    let mut paths = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let quoted = |marker: &str| {
            line.split_once(marker)
                .and_then(|(_, rest)| rest.split('\'').next())
                .filter(|path| path.starts_with("/nix/store/") && path.ends_with(".drv"))
        };
        let path = quoted("builder for '")
            .or_else(|| quoted("hash mismatch in fixed-output derivation '"))
            // Newer Nix names every unbuilt derivation this way; the next line gives the reason.
            .or_else(|| {
                quoted("Cannot build '").filter(|_| {
                    failure_reason(&lines, index).is_none_or(|reason| !is_dependency_reason(reason))
                })
            });
        if let Some(path) = path
            && !paths.iter().any(|known| known == path)
        {
            paths.push(path.to_owned());
        }
    }
    paths
}

/// The `Reason:` line following a `Cannot build` line at `index`.
fn failure_reason<'a>(lines: &[&'a str], index: usize) -> Option<&'a str> {
    lines
        .iter()
        .skip(index + 1)
        .take(2)
        .find_map(|line| line.trim().strip_prefix("Reason:"))
}

fn is_dependency_reason(reason: &str) -> bool {
    reason.contains("dependency failed") || reason.contains("dependencies failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_originating_nix_build_error() {
        let stderr = br#"@nix {"action":"msg","level":1,"raw_msg":"warning: deprecated option"}
@nix {"action":"msg","level":0,"raw_msg":"linking '/nix/store/system_fish-completions/uptime.fish' to '/nix/store/.links/content-address' not allowed"}
@nix {"action":"msg","level":0,"raw_msg":"\u001b[31;1merror:\u001b[0m builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"}
@nix {"action":"msg","level":0,"raw_msg":"error: 2 dependencies of derivation '/nix/store/home-manager.drv' failed to build"}
@nix {"action":"msg","level":0,"raw_msg":"error: 1 dependencies of derivation '/nix/store/nixos-system-odin.drv' failed to build"}"#;
        assert_eq!(
            nix_error_message(stderr).as_deref(),
            Some(
                "error: builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"
            )
        );
        assert_eq!(
            strip_ansi("Resolving package\n\u{1b}[1;31m✗\u{1b}[0m failed"),
            "Resolving package\n✗ failed"
        );
    }

    #[test]
    fn extracts_failed_derivations_but_not_dependents() {
        assert_eq!(
            failed_derivations(
                "error: builder for '/nix/store/a-chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > builder for 'x' failed"
            ),
            ["/nix/store/a-chomp.drv"]
        );
        assert!(
            failed_derivations(
                "error: 2 dependencies of derivation '/nix/store/b-home.drv' failed to build"
            )
            .is_empty()
        );
        assert_eq!(
            failed_derivations(
                "error: Cannot build '/nix/store/c-swix-0.1.0.drv'.\n       Reason: builder failed with exit code 1.\n       Output paths:\n         /nix/store/c-swix-0.1.0"
            ),
            ["/nix/store/c-swix-0.1.0.drv"]
        );
        let dependent = "error: Cannot build '/nix/store/d-system.drv'.\n       Reason: 1 dependency failed.";
        assert!(failed_derivations(dependent).is_empty());
        assert!(is_dependency_failure(dependent));
        assert_eq!(
            failed_derivations(
                "error: hash mismatch in fixed-output derivation '/nix/store/e-src.drv':\n  specified: sha256-A\n     got:    sha256-B"
            ),
            ["/nix/store/e-src.drv"]
        );
    }
}
