use std::env;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::model::{CleanupGroup, CleanupItem, CleanupKind};
use crate::ui::duration_text;

const TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const OUTPUT_LIMIT: usize = 32 * 1024 * 1024;
const API_VERSION: &str = "v1.52";
const DEFAULT_SOCKET: &str = "/var/run/docker.sock";

/// Prune steps in the order the CLI's `system prune --all --volumes` runs them, plus build cache.
const PRUNE_STEPS: [(&str, &str); 5] = [
    ("containers/prune", "Docker container cleanup"),
    ("networks/prune", "Docker network cleanup"),
    // filters={"all":["true"]}: include named volumes, not only anonymous ones.
    (
        "volumes/prune?filters=%7B%22all%22%3A%5B%22true%22%5D%7D",
        "Docker volume cleanup",
    ),
    // filters={"dangling":["false"]}: remove every unused image, not only dangling ones.
    (
        "images/prune?filters=%7B%22dangling%22%3A%5B%22false%22%5D%7D",
        "Docker image cleanup",
    ),
    ("build/prune?all=true", "Docker builder cleanup"),
];

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    match endpoint() {
        Ok(socket) => scan_at(&socket, cancellation),
        Err(error) => CleanupGroup::unavailable(CleanupKind::Docker, error),
    }
}

pub(super) fn clean(cancellation: &AtomicBool) -> Result<u64, String> {
    clean_at(&endpoint()?, cancellation)
}

fn scan_at(socket: &Path, cancellation: &AtomicBool) -> CleanupGroup {
    let usage: DiskUsage = match request(
        socket,
        "GET",
        "system/df?verbose=1",
        cancellation,
        SCAN_TIMEOUT,
    )
    .and_then(|body| decode(&body, "Docker storage scan"))
    {
        Ok(usage) => usage,
        Err(error) => return CleanupGroup::unavailable(CleanupKind::Docker, error),
    };

    CleanupGroup {
        kind: CleanupKind::Docker,
        available: true,
        reclaimable: usage.reclaimable(),
        items: usage.items(now_secs()),
        note: Some(
            "Removes all unused Docker data, including named volumes. Volumes attached to any container are retained."
                .to_owned(),
        ),
        volumes: Vec::new(),
    }
}

fn clean_at(socket: &Path, cancellation: &AtomicBool) -> Result<u64, String> {
    let mut reclaimed = 0_u64;
    let mut errors = Vec::new();

    for (path, name) in PRUNE_STEPS {
        match request(socket, "POST", path, cancellation, TIMEOUT)
            .and_then(|body| decode::<Pruned>(&body, name))
        {
            Ok(pruned) => reclaimed = reclaimed.saturating_add(pruned.space_reclaimed),
            Err(error) if error == swix::command::CANCELLED => return Err(error),
            Err(error) => errors.push(format!("{name} failed: {error}")),
        }
    }

    if errors.is_empty() {
        Ok(reclaimed)
    } else {
        Err(errors.join("; "))
    }
}

fn endpoint() -> Result<PathBuf, String> {
    let var = |name: &str| env::var(name).ok().filter(|value| !value.is_empty());
    let config_dir = var("DOCKER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| var("HOME").map(|home| Path::new(&home).join(".docker")));
    resolve_socket(
        var("DOCKER_HOST").as_deref(),
        var("DOCKER_CONTEXT").as_deref(),
        config_dir.as_deref(),
    )
}

/// Mirrors the docker CLI: `DOCKER_HOST`, then `DOCKER_CONTEXT`, then the config's
/// `currentContext`, then the default socket.
fn resolve_socket(
    docker_host: Option<&str>,
    docker_context: Option<&str>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(host) = docker_host {
        return unix_socket(host, "DOCKER_HOST");
    }
    let context = match docker_context {
        Some(context) => Some(context.to_owned()),
        None => config_dir.and_then(current_context),
    };
    match (context, config_dir) {
        (Some(context), Some(config_dir)) if context != "default" => {
            context_host(config_dir, &context)
                .and_then(|host| unix_socket(&host, &format!("Docker context {context}")))
        }
        (Some(context), None) if context != "default" => Err(format!(
            "Docker context {context} cannot be resolved without a Docker config directory"
        )),
        _ => Ok(PathBuf::from(DEFAULT_SOCKET)),
    }
}

fn unix_socket(host: &str, source: &str) -> Result<PathBuf, String> {
    host.strip_prefix("unix://")
        .map(PathBuf::from)
        .ok_or_else(|| format!("{source} points to {host}; only unix:// Docker sockets are supported"))
}

fn current_context(config_dir: &Path) -> Option<String> {
    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct Config {
        current_context: Option<String>,
    }
    let bytes = fs::read(config_dir.join("config.json")).ok()?;
    serde_json::from_slice::<Config>(&bytes)
        .ok()?
        .current_context
        .filter(|context| !context.is_empty())
}

fn context_host(config_dir: &Path, context: &str) -> Result<String, String> {
    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "PascalCase")]
    struct Meta {
        name: String,
        endpoints: Endpoints,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Endpoints {
        docker: Endpoint,
    }
    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "PascalCase")]
    struct Endpoint {
        host: String,
    }

    let not_found = || format!("Docker context {context} was not found");
    let entries = fs::read_dir(config_dir.join("contexts/meta")).map_err(|_| not_found())?;
    entries
        .flatten()
        .filter_map(|entry| fs::read(entry.path().join("meta.json")).ok())
        .filter_map(|bytes| serde_json::from_slice::<Meta>(&bytes).ok())
        .find(|meta| meta.name == context)
        .map(|meta| meta.endpoints.docker.host)
        .ok_or_else(not_found)
}

fn request(
    socket: &Path,
    method: &str,
    path: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + timeout;
    let mut stream = UnixStream::connect(socket)
        .map_err(|error| format!("Docker is not reachable at {}: {error}", socket.display()))?;
    let io_error = |error: std::io::Error| format!("Docker request failed: {error}");
    stream
        .set_read_timeout(Some(POLL_INTERVAL))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(SCAN_TIMEOUT))
        .map_err(io_error)?;
    write!(
        stream,
        "{method} /{API_VERSION}/{path} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    )
    .map_err(io_error)?;

    let mut response = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        if cancellation.load(Ordering::Relaxed) {
            return Err(swix::command::CANCELLED.to_owned());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Docker did not respond within {} seconds",
                timeout.as_secs()
            ));
        }
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.len() > OUTPUT_LIMIT {
                    return Err("Docker response exceeded the size limit".to_owned());
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(io_error(error)),
        }
    }
    parse_response(&response)
}

fn parse_response(response: &[u8]) -> Result<Vec<u8>, String> {
    let malformed = || "Docker returned a malformed HTTP response".to_owned();
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(malformed)?;
    let head = std::str::from_utf8(&response[..split]).map_err(|_| malformed())?;
    let raw_body = &response[split + 4..];
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or_else(malformed)?;
    let status: u16 = status_line
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(malformed)?;
    let chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("transfer-encoding")
                && value.to_ascii_lowercase().contains("chunked")
        })
    });
    let body = if chunked {
        decode_chunked(raw_body).ok_or_else(malformed)?
    } else {
        raw_body.to_vec()
    };

    if (200..300).contains(&status) {
        return Ok(body);
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        message: String,
    }
    Err(serde_json::from_slice::<ErrorBody>(&body)
        .map(|error| error.message)
        .unwrap_or_else(|_| status_line.to_owned()))
}

fn decode_chunked(mut input: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = input.windows(2).position(|window| window == b"\r\n")?;
        let size_line = std::str::from_utf8(&input[..line_end]).ok()?;
        let size_text = size_line.split(';').next()?.trim();
        let size = usize::from_str_radix(size_text, 16).ok()?;
        input = &input[line_end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(input.get(..size)?);
        input = input.get(size..)?.strip_prefix(b"\r\n")?;
    }
}

fn decode<T: DeserializeOwned>(body: &[u8], name: &str) -> Result<T, String> {
    serde_json::from_slice(body).map_err(|error| format!("{name} returned invalid JSON: {error}"))
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Pruned {
    space_reclaimed: u64,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct DiskUsage {
    image_usage: Usage<Image>,
    container_usage: Usage<Container>,
    volume_usage: Usage<Volume>,
    build_cache_usage: Usage<BuildCache>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Usage<T> {
    reclaimable: u64,
    items: Vec<T>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Image {
    id: String,
    repo_tags: Vec<String>,
    created: i64,
    size: i64,
    containers: i64,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Container {
    id: String,
    names: Vec<String>,
    size_rw: i64,
    state: String,
    status: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Volume {
    name: String,
    usage_data: VolumeUsageData,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct VolumeUsageData {
    size: i64,
    ref_count: i64,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct BuildCache {
    #[serde(rename = "ID")]
    id: String,
    description: String,
    in_use: bool,
    size: i64,
}

impl DiskUsage {
    fn reclaimable(&self) -> u64 {
        [
            self.image_usage.reclaimable,
            self.container_usage.reclaimable,
            self.volume_usage.reclaimable,
            self.build_cache_usage.reclaimable,
        ]
        .into_iter()
        .fold(0, u64::saturating_add)
    }

    fn items(self, now: i64) -> Vec<CleanupItem> {
        let images = self
            .image_usage
            .items
            .into_iter()
            .filter(|image| image.containers == 0)
            .map(|image| {
                let id = short_id(&image.id);
                let label = image
                    .repo_tags
                    .iter()
                    .find(|tag| *tag != "<none>:<none>")
                    .map_or_else(|| id.to_owned(), Clone::clone);
                let age = u64::try_from(now.saturating_sub(image.created)).unwrap_or(0);
                item(
                    label,
                    format!("Unused image · {} old · {id}", duration_text(age)),
                    image.size,
                )
            });
        let containers = self
            .container_usage
            .items
            .into_iter()
            .filter(|container| container.state != "running")
            .map(|container| {
                let label = container
                    .names
                    .first()
                    .map(|name| name.trim_start_matches('/').to_owned())
                    .unwrap_or_else(|| short_id(&container.id).to_owned());
                item(
                    label,
                    format!("Stopped container · {}", container.status),
                    container.size_rw,
                )
            });
        let volumes = self
            .volume_usage
            .items
            .into_iter()
            .filter(|volume| volume.usage_data.ref_count == 0)
            .map(|volume| {
                item(
                    volume.name,
                    "Unused local volume".to_owned(),
                    volume.usage_data.size,
                )
            });
        let caches = self
            .build_cache_usage
            .items
            .into_iter()
            .filter(|cache| !cache.in_use)
            .map(|cache| {
                item(
                    short_id(&cache.id).to_owned(),
                    format!("Build cache · {}", cache.description),
                    cache.size,
                )
            });

        let mut items: Vec<CleanupItem> = images
            .chain(containers)
            .chain(volumes)
            .chain(caches)
            .collect();
        items.sort_by(|left, right| {
            right
                .bytes
                .cmp(&left.bytes)
                .then_with(|| left.label.cmp(&right.label))
        });
        items
    }
}

fn item(label: String, detail: String, size: i64) -> CleanupItem {
    CleanupItem {
        label,
        detail,
        bytes: u64::try_from(size).ok(),
        path: None,
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

fn short_id(id: &str) -> &str {
    let id = id.strip_prefix("sha256:").unwrap_or(id);
    id.get(..12).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::AtomicU64;
    use std::thread::{self, JoinHandle};

    use super::*;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = env::temp_dir().join(format!(
                "swix-docker-cleanup-test-{}-{id}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        /// Serves one canned response per connection and returns the request lines it saw.
        fn serve(&self, responses: Vec<String>) -> (PathBuf, JoinHandle<Vec<String>>) {
            let socket = self.root.join("docker.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            let server = thread::spawn(move || {
                responses
                    .into_iter()
                    .map(|response| {
                        let (mut stream, _) = listener.accept().unwrap();
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut request_line = String::new();
                        reader.read_line(&mut request_line).unwrap();
                        let mut header = String::new();
                        while reader.read_line(&mut header).unwrap() > 0 && header != "\r\n" {
                            header.clear();
                        }
                        stream.write_all(response.as_bytes()).unwrap();
                        request_line.trim_end().to_owned()
                    })
                    .collect()
            });
            (socket, server)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn plain(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    fn chunked(body: &str) -> String {
        let (first, second) = body.split_at(body.len() / 2);
        format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{first}\r\n{:x};ext=1\r\n{second}\r\n0\r\n\r\n",
            first.len(),
            second.len()
        )
    }

    fn percent_decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut decoded = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap();
                decoded.push(u8::from_str_radix(hex, 16).unwrap());
                index += 3;
            } else {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
        String::from_utf8(decoded).unwrap()
    }

    #[test]
    fn prune_sums_space_reclaimed_across_steps() {
        let fixture = Fixture::new();
        let (socket, server) = fixture.serve(vec![
            plain("200 OK", r#"{"ContainersDeleted":["a"],"SpaceReclaimed":1}"#),
            plain("200 OK", r#"{"NetworksDeleted":[]}"#),
            chunked(r#"{"VolumesDeleted":["v"],"SpaceReclaimed":20}"#),
            plain("200 OK", r#"{"ImagesDeleted":[],"SpaceReclaimed":300}"#),
            chunked(r#"{"CachesDeleted":["c"],"SpaceReclaimed":4000}"#),
        ]);

        let reclaimed = clean_at(&socket, &AtomicBool::new(false)).unwrap();

        assert_eq!(reclaimed, 4321);
        let requests: Vec<String> = server
            .join()
            .unwrap()
            .iter()
            .map(|line| percent_decode(line))
            .collect();
        assert_eq!(
            requests,
            [
                "POST /v1.52/containers/prune HTTP/1.1",
                "POST /v1.52/networks/prune HTTP/1.1",
                r#"POST /v1.52/volumes/prune?filters={"all":["true"]} HTTP/1.1"#,
                r#"POST /v1.52/images/prune?filters={"dangling":["false"]} HTTP/1.1"#,
                "POST /v1.52/build/prune?all=true HTTP/1.1",
            ]
        );
    }

    #[test]
    fn failed_prune_step_is_reported_and_later_steps_still_run() {
        let fixture = Fixture::new();
        let (socket, server) = fixture.serve(vec![
            plain("200 OK", r#"{"SpaceReclaimed":1}"#),
            plain("500 Internal Server Error", r#"{"message":"network busy"}"#),
            plain("200 OK", r#"{"SpaceReclaimed":2}"#),
            plain("200 OK", r#"{"SpaceReclaimed":3}"#),
            plain("200 OK", r#"{"SpaceReclaimed":4}"#),
        ]);

        let error = clean_at(&socket, &AtomicBool::new(false)).unwrap_err();

        assert_eq!(error, "Docker network cleanup failed: network busy");
        assert_eq!(server.join().unwrap().len(), 5);
    }

    #[test]
    fn unreachable_socket_is_reported() {
        let fixture = Fixture::new();
        let group = scan_at(&fixture.root.join("missing.sock"), &AtomicBool::new(false));
        assert!(!group.available);
    }

    #[test]
    fn scan_totals_reclaimable_and_keeps_only_prunable_objects() {
        let fixture = Fixture::new();
        let body = r#"{
            "ImageUsage":{"Reclaimable":1000,"Items":[
                {"Id":"sha256:1234567890abcdef","RepoTags":["<none>:<none>","app:dev"],"Created":0,"Size":1000,"Containers":0},
                {"Id":"sha256:fedcba9876543210","RepoTags":["live:latest"],"Created":0,"Size":2000,"Containers":1}]},
            "ContainerUsage":{"Reclaimable":10,"Items":[
                {"Id":"c0ffee0000000000","Names":["/old"],"SizeRw":10,"State":"exited","Status":"Exited (0) 2 days ago"},
                {"Id":"beef000000000000","Names":["/web"],"SizeRw":50,"State":"running","Status":"Up 1 hour"}]},
            "VolumeUsage":{"Reclaimable":200,"Items":[
                {"Name":"unused","UsageData":{"Size":200,"RefCount":0}},
                {"Name":"data","UsageData":{"Size":900,"RefCount":1}}]},
            "BuildCacheUsage":{"Reclaimable":30,"Items":[
                {"ID":"cacheid0123456789","Description":"layer","InUse":false,"Size":30},
                {"ID":"busy0123456789","Description":"mount","InUse":true,"Size":70}]}
        }"#;
        let (socket, server) = fixture.serve(vec![chunked(body)]);

        let group = scan_at(&socket, &AtomicBool::new(false));

        assert_eq!(
            server.join().unwrap(),
            ["GET /v1.52/system/df?verbose=1 HTTP/1.1"]
        );
        assert!(group.available);
        assert_eq!(group.reclaimable, 1240);
        let labels: Vec<&str> = group.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["app:dev", "unused", "cacheid01234", "old"]);
        assert_eq!(group.items[0].bytes, Some(1000));
        assert!(group.items[0].detail.ends_with("· 1234567890ab"));
        assert_eq!(group.items[3].detail, "Stopped container · Exited (0) 2 days ago");
    }

    #[test]
    fn docker_host_takes_precedence() {
        let fixture = Fixture::new();
        assert_eq!(
            resolve_socket(
                Some("unix:///run/user/1000/docker.sock"),
                Some("remote"),
                Some(&fixture.root)
            )
            .unwrap(),
            Path::new("/run/user/1000/docker.sock")
        );
        assert!(
            resolve_socket(Some("tcp://10.0.0.1:2376"), None, None)
                .unwrap_err()
                .contains("only unix://")
        );
    }

    #[test]
    fn context_is_resolved_from_meta_files() {
        let fixture = Fixture::new();
        let meta = fixture.root.join("contexts/meta/0123abcd");
        fs::create_dir_all(&meta).unwrap();
        fs::write(
            meta.join("meta.json"),
            r#"{"Name":"rootless","Endpoints":{"docker":{"Host":"unix:///run/user/1000/docker.sock"}}}"#,
        )
        .unwrap();
        fs::write(
            fixture.root.join("config.json"),
            r#"{"currentContext":"rootless"}"#,
        )
        .unwrap();

        assert_eq!(
            resolve_socket(None, None, Some(&fixture.root)).unwrap(),
            Path::new("/run/user/1000/docker.sock")
        );
        assert_eq!(
            resolve_socket(None, Some("default"), Some(&fixture.root)).unwrap(),
            Path::new(DEFAULT_SOCKET)
        );
        assert!(resolve_socket(None, Some("missing"), Some(&fixture.root)).is_err());
    }

    #[test]
    fn falls_back_to_default_socket() {
        let fixture = Fixture::new();
        assert_eq!(
            resolve_socket(None, None, Some(&fixture.root)).unwrap(),
            Path::new(DEFAULT_SOCKET)
        );
        assert_eq!(
            resolve_socket(None, None, None).unwrap(),
            Path::new(DEFAULT_SOCKET)
        );
    }
}
