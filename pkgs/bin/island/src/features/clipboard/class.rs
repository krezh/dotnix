//! What an entry is, read from its kind and its text: a link, code or a color stand apart from
//! plain text.

use super::entry::Kind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Class {
    Text,
    Link,
    Code,
    Color,
    Picture,
    Files,
}

/// Commands that start a line read as a shell command.
const COMMANDS: &[&str] = &[
    "awk",
    "bash",
    "brew",
    "cargo",
    "cat",
    "cd",
    "chmod",
    "chown",
    "cp",
    "curl",
    "docker",
    "echo",
    "export",
    "find",
    "fish",
    "flux",
    "gh",
    "git",
    "go",
    "grep",
    "helm",
    "jj",
    "journalctl",
    "kubectl",
    "ln",
    "ls",
    "make",
    "mkdir",
    "mv",
    "nix",
    "nix-shell",
    "nixos-rebuild",
    "npm",
    "npx",
    "pacman",
    "pip",
    "podman",
    "python",
    "python3",
    "rm",
    "rsync",
    "scp",
    "sed",
    "sh",
    "ssh",
    "sudo",
    "systemctl",
    "tar",
    "talosctl",
    "tmux",
    "uv",
    "wget",
    "yarn",
];

impl Class {
    pub(super) fn of(kind: Kind, text: &str) -> Self {
        match kind {
            Kind::Image => Class::Picture,
            Kind::Files => Class::Files,
            Kind::Text if site(text).is_some() => Class::Link,
            Kind::Text if color(text).is_some() => Class::Color,
            Kind::Text if code(text) => Class::Code,
            Kind::Text => Class::Text,
        }
    }
}

/// Where a link leads, when the text is one address and nothing else.
pub(super) fn site(text: &str) -> Option<&str> {
    let text = text.trim();
    if text.chars().any(char::is_whitespace) {
        return None;
    }
    let (scheme, rest) = text.split_once("://")?;
    if !["http", "https", "ftp", "sftp", "ssh", "git"]
        .contains(&scheme.to_ascii_lowercase().as_str())
    {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    let host = match host.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name,
        _ => host,
    };
    (!host.is_empty()).then_some(host)
}

/// The color the text writes, as `0xrrggbb`: `#rgb`, `#rrggbb` or `rgb(r, g, b)`.
pub(super) fn color(text: &str) -> Option<u32> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        return match hex.len() {
            3 => {
                let value = u32::from_str_radix(hex, 16).ok()?;
                let [r, g, b] = [(value >> 8) & 0xf, (value >> 4) & 0xf, value & 0xf];
                Some((r * 0x11) << 16 | (g * 0x11) << 8 | b * 0x11)
            }
            6 => u32::from_str_radix(hex, 16).ok(),
            _ => None,
        };
    }
    let inner = text
        .strip_prefix("rgb(")
        .or_else(|| text.strip_prefix("rgba("))?
        .strip_suffix(')')?;
    let channels: Vec<u32> = inner
        .split([',', ' ', '/'])
        .filter(|part| !part.is_empty())
        .take(3)
        .map(|part| part.parse::<u32>().ok().filter(|value| *value <= 255))
        .collect::<Option<_>>()?;
    match channels[..] {
        [r, g, b] => Some(r << 16 | g << 8 | b),
        _ => None,
    }
}

/// A shell command, or several lines that read as source.
pub(super) fn code(text: &str) -> bool {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    match lines[..] {
        [] => false,
        [line] => command(line),
        _ => {
            let shaped = lines
                .iter()
                .filter(|line| {
                    let trimmed = line.trim_end();
                    line.starts_with([' ', '\t'])
                        || trimmed.ends_with(['{', '}', ';', '(', ')', '[', ']', ',', ':', '\\'])
                        || command(line)
                })
                .count();
            shaped * 2 >= lines.len()
        }
    }
}

fn command(line: &str) -> bool {
    let line = line.trim();
    let line = line.strip_prefix("$ ").unwrap_or(line);
    let first = line.split_whitespace().next().unwrap_or_default();
    COMMANDS.contains(&first) && line.len() > first.len()
}

/// Hue in degrees, then saturation and lightness in percent.
pub(super) fn hsl(rgb: u32) -> (u32, u32, u32) {
    let [_, r, g, b] = rgb.to_be_bytes().map(|channel| f32::from(channel) / 255.0);
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let lightness = (high + low) / 2.0;
    let spread = high - low;
    if spread == 0.0 {
        return (0, 0, (lightness * 100.0).round() as u32);
    }
    let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if high == r {
        60.0 * ((g - b) / spread).rem_euclid(6.0)
    } else if high == g {
        60.0 * ((b - r) / spread + 2.0)
    } else {
        60.0 * ((r - g) / spread + 4.0)
    };
    (
        hue.round() as u32 % 360,
        (saturation * 100.0).round() as u32,
        (lightness * 100.0).round() as u32,
    )
}

#[cfg(test)]
#[path = "class_test.rs"]
mod tests;
