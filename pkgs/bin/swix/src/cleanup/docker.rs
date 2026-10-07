use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde_json::{Map, Value};

use super::model::{CleanupGroup, CleanupItem, CleanupKind, parse_size};
use super::run_command;

const TIMEOUT: Duration = Duration::from_secs(30 * 60);
const OUTPUT_LIMIT: usize = 32 * 1024 * 1024;

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let mut summary_command = Command::new("docker");
    summary_command.args(["system", "df", "--format", "{{json .}}"]);
    let summary = match run_command(
        &mut summary_command,
        "Docker storage scan",
        cancellation,
        Duration::from_secs(60),
        OUTPUT_LIMIT,
    ) {
        Ok(output) => output,
        Err(error) => return CleanupGroup::unavailable(CleanupKind::Docker, error),
    };
    let reclaimable = match parse_summary(&summary.stdout) {
        Ok(bytes) => bytes,
        Err(error) => return CleanupGroup::unavailable(CleanupKind::Docker, error),
    };

    let mut detail_command = Command::new("docker");
    detail_command.args(["system", "df", "--verbose", "--format", "{{json .}}"]);
    let items = run_command(
        &mut detail_command,
        "Docker detail scan",
        cancellation,
        Duration::from_secs(60),
        OUTPUT_LIMIT,
    )
    .and_then(|output| parse_details(&output.stdout))
    .unwrap_or_default();

    CleanupGroup {
        kind: CleanupKind::Docker,
        available: true,
        reclaimable,
        items,
        note: Some(
            "Removes all unused Docker data, including named volumes. Volumes attached to any container are retained."
                .to_owned(),
        ),
    }
}

pub(super) fn clean(cancellation: &AtomicBool) -> Result<u64, String> {
    clean_with(Path::new("docker"), cancellation)
}

fn clean_with(program: &Path, cancellation: &AtomicBool) -> Result<u64, String> {
    let jobs: [(&[&str], &str); 3] = [
        (
            &["system", "prune", "--all", "--force"],
            "Docker system cleanup",
        ),
        (
            &["volume", "prune", "--all", "--force"],
            "Docker volume cleanup",
        ),
        (
            &["builder", "prune", "--all", "--force"],
            "Docker builder cleanup",
        ),
    ];
    let mut reclaimed = 0_u64;
    let mut errors = Vec::new();

    for (arguments, name) in jobs {
        let mut command = Command::new(program);
        command.args(arguments);
        match run_command(&mut command, name, cancellation, TIMEOUT, OUTPUT_LIMIT) {
            Ok(output) => {
                reclaimed = reclaimed.saturating_add(parse_prune_reclaimed(&output.stdout));
            }
            Err(error) => errors.push(error),
        }
    }

    if errors.is_empty() {
        Ok(reclaimed)
    } else {
        Err(errors.join("; "))
    }
}

fn parse_prune_reclaimed(output: &[u8]) -> u64 {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("Total reclaimed space:")
                .or_else(|| line.strip_prefix("Total:"))
                .and_then(parse_size)
        })
        .sum()
}

fn parse_summary(output: &[u8]) -> Result<u64, String> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .try_fold(0_u64, |total, line| {
            let value: Value = serde_json::from_str(line)
                .map_err(|error| format!("Docker storage scan returned invalid JSON: {error}"))?;
            let reclaimable = value
                .get("Reclaimable")
                .and_then(Value::as_str)
                .and_then(parse_size)
                .unwrap_or(0);
            Ok(total.saturating_add(reclaimable))
        })
}

fn parse_details(output: &[u8]) -> Result<Vec<CleanupItem>, String> {
    let value: Value = serde_json::from_slice(output)
        .map_err(|error| format!("Docker detail scan returned invalid JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or("Docker detail scan returned an unexpected response")?;
    let mut items = Vec::new();
    append_images(object, &mut items);
    append_containers(object, &mut items);
    append_volumes(object, &mut items);
    append_build_cache(object, &mut items);
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(items)
}

fn append_images(object: &Map<String, Value>, items: &mut Vec<CleanupItem>) {
    for image in array(object, "Images") {
        if field(image, "Containers") != Some("0") {
            continue;
        }
        let repository = field(image, "Repository").unwrap_or("<none>");
        let tag = field(image, "Tag").unwrap_or("<none>");
        let id = short_id(field(image, "ID").unwrap_or("unknown"));
        let label = if repository == "<none>" {
            id.to_owned()
        } else {
            format!("{repository}:{tag}")
        };
        items.push(CleanupItem {
            label,
            detail: format!(
                "Unused image · {} · {id}",
                field(image, "CreatedSince").unwrap_or("age unknown")
            ),
            bytes: field(image, "Size").and_then(parse_size),
            path: None,
        });
    }
}

fn append_containers(object: &Map<String, Value>, items: &mut Vec<CleanupItem>) {
    for container in array(object, "Containers") {
        let status = field(container, "Status").unwrap_or_default();
        if status.starts_with("Up ") {
            continue;
        }
        items.push(CleanupItem {
            label: field(container, "Names")
                .or_else(|| field(container, "Name"))
                .unwrap_or_else(|| short_id(field(container, "ID").unwrap_or("unknown")))
                .to_owned(),
            detail: format!("Stopped container · {status}"),
            bytes: field(container, "Size").and_then(parse_size),
            path: None,
        });
    }
}

fn append_volumes(object: &Map<String, Value>, items: &mut Vec<CleanupItem>) {
    for volume in array(object, "Volumes") {
        if field(volume, "Links") != Some("0") {
            continue;
        }
        items.push(CleanupItem {
            label: field(volume, "Name").unwrap_or("unnamed volume").to_owned(),
            detail: "Unused local volume".to_owned(),
            bytes: field(volume, "Size").and_then(parse_size),
            path: None,
        });
    }
}

fn append_build_cache(object: &Map<String, Value>, items: &mut Vec<CleanupItem>) {
    for cache in array(object, "BuildCache") {
        if field(cache, "InUse") != Some("false") {
            continue;
        }
        items.push(CleanupItem {
            label: short_id(field(cache, "ID").unwrap_or("unknown")).to_owned(),
            detail: format!(
                "Build cache · {}",
                field(cache, "Description").unwrap_or("no description")
            ),
            bytes: field(cache, "Size").and_then(parse_size),
            path: None,
        });
    }
}

fn array<'a>(object: &'a Map<String, Value>, key: &str) -> impl Iterator<Item = &'a Value> {
    object
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn short_id(id: &str) -> &str {
    id.strip_prefix("sha256:")
        .unwrap_or(id)
        .get(..12)
        .unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        program: PathBuf,
        log: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "swix-docker-cleanup-test-{}-{id}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let program = root.join("docker");
            let log = root.join("calls");
            fs::write(
                &program,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1\" in\n  system) printf 'Total reclaimed space: 1MB\\n' ;;\n  volume) printf 'Total reclaimed space: 2MB\\n' ;;\n  builder) printf 'Total:\\t3MB\\n' ;;\n  *) exit 2 ;;\nesac\n",
                    log.display()
                ),
            )
            .unwrap();
            let mut permissions = fs::metadata(&program).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&program, permissions).unwrap();
            Self { root, program, log }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn totals_docker_reclaimable_space() {
        let output = br#"{"Reclaimable":"11.06GB (97%)"}
{"Reclaimable":"344.3MB (100%)"}
{"Reclaimable":"15.9GB"}"#;
        assert_eq!(parse_summary(output).unwrap(), 29_309_104_292);
    }
    #[test]
    fn cleanup_prunes_named_volumes_and_reports_actual_output() {
        let fixture = Fixture::new();

        let reclaimed = clean_with(&fixture.program, &AtomicBool::new(false)).unwrap();

        assert_eq!(reclaimed, 6 * 1024 * 1024);
        let calls = fs::read_to_string(&fixture.log).unwrap();
        assert!(calls.contains("system prune --all --force"));
        assert!(calls.contains("volume prune --all --force"));
        assert!(calls.contains("builder prune --all --force"));
    }

    #[test]
    fn details_include_only_prunable_objects() {
        let output = br#"{"Images":[{"Containers":"0","Repository":"app","Tag":"dev","ID":"sha256:1234567890abcdef","CreatedSince":"2 days ago","Size":"1GB"},{"Containers":"1","Repository":"live","Tag":"latest","ID":"sha256:fedcba","Size":"2GB"}],"Containers":[],"Volumes":[{"Links":"0","Name":"unused","Size":"2MB"}],"BuildCache":[{"InUse":"false","ID":"cache-id","Description":"layer","Size":"3MB"}]}"#;
        let items = parse_details(output).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|item| item.label == "app:dev"));
        assert!(!items.iter().any(|item| item.label == "live:latest"));
    }
}
