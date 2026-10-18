//! Command-line argument parsing.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct CommandLine {
    pub(super) host: Option<String>,
    pub(super) keybinds: Option<bool>,
}

pub(super) fn parse_command_line(
    arguments: Vec<std::ffi::OsString>,
) -> Result<CommandLine, String> {
    let mut host = None;
    let mut keybinds = None;
    let mut arguments = arguments.into_iter().skip(1);
    while let Some(argument) = arguments.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "command-line arguments must be valid UTF-8".to_owned())?;
        if argument == "--host" {
            let value = arguments
                .next()
                .ok_or("--host requires a NixOS configuration name")?
                .into_string()
                .map_err(|_| "host name must be valid UTF-8".to_owned())?;
            if value.trim().is_empty() || value.starts_with('-') {
                return Err("--host requires a non-empty NixOS configuration name".to_owned());
            }
            if host.replace(value).is_some() {
                return Err("--host may only be specified once".to_owned());
            }
        } else if let Some(value) = argument.strip_prefix("--host=") {
            let value = value.to_owned();
            if value.trim().is_empty() || value.starts_with('-') {
                return Err("--host requires a non-empty NixOS configuration name".to_owned());
            }
            if host.replace(value).is_some() {
                return Err("--host may only be specified once".to_owned());
            }
        } else if argument == "--no-keybinds" || argument == "--disable-keybinds" {
            keybinds = Some(false);
        } else if argument == "--keybinds" {
            keybinds = Some(true);
        } else if let Some(value) = argument.strip_prefix("--keybinds=") {
            match value {
                "true" | "1" | "yes" => keybinds = Some(true),
                "false" | "0" | "no" => keybinds = Some(false),
                _ => {
                    return Err(format!(
                        "invalid value {value:?} for --keybinds; expected true or false"
                    ));
                }
            }
        } else {
            return Err(format!(
                "unknown option {argument:?}; expected --host <name>, --no-keybinds"
            ));
        }
    }
    Ok(CommandLine { host, keybinds })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_override() {
        let args = |values: &[&str]| values.iter().map(std::ffi::OsString::from).collect();
        assert_eq!(
            parse_command_line(args(&["molt", "--host", "odin"])).unwrap(),
            CommandLine {
                host: Some("odin".to_owned()),
                keybinds: None,
            }
        );
        assert_eq!(
            parse_command_line(args(&["molt", "--host=thor"])).unwrap(),
            CommandLine {
                host: Some("thor".to_owned()),
                keybinds: None,
            }
        );
        assert_eq!(
            parse_command_line(args(&["molt"])).unwrap(),
            CommandLine::default()
        );
        assert_eq!(
            parse_command_line(args(&["molt", "--no-keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(false),
            }
        );
        assert_eq!(
            parse_command_line(args(&["molt", "--disable-keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(false),
            }
        );
        assert_eq!(
            parse_command_line(args(&["molt", "--keybinds"])).unwrap(),
            CommandLine {
                host: None,
                keybinds: Some(true),
            }
        );
        assert_eq!(
            parse_command_line(args(&["molt", "--host", "odin", "--no-keybinds"])).unwrap(),
            CommandLine {
                host: Some("odin".to_owned()),
                keybinds: Some(false),
            }
        );
        assert!(parse_command_line(args(&["molt", "--host"])).is_err());
        assert!(parse_command_line(args(&["molt", "--unknown"])).is_err());
    }
}
