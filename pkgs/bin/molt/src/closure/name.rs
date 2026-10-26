pub(super) fn split_store_name(path: &str) -> (&str, &str) {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let name = file_name
        .split_once('-')
        .filter(|(hash, _)| {
            hash.len() == 32 && hash.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .map_or(file_name, |(_, name)| name);
    let name = name.strip_suffix(".drv").unwrap_or(name);
    for (index, _) in name.match_indices('-') {
        if index == 0 {
            continue;
        }
        let suffix = &name[index + 1..];
        let starts_with_digit = suffix
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit());
        let git_hash = (7..=40).contains(&suffix.len())
            && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
            && suffix.bytes().any(|byte| byte.is_ascii_alphabetic());
        if starts_with_digit || git_hash {
            return (&name[..index], suffix);
        }
    }
    (name, "")
}

pub(super) fn compact_version(version: &str) -> String {
    const OUTPUT_SUFFIXES: &[&str] = &[
        "-fish-completions",
        "_fish-completions",
        "-fhsenv-profile",
        "-fhsenv-rootfs",
        "-libgcc",
        "-bwrap",
        "-extracted",
        "-patched",
        "-init",
        "-bin",
        "-dev",
        "-doc",
        "-info",
        "-lib",
        "-man",
        "-out",
        "-static",
        "-debug",
    ];
    OUTPUT_SUFFIXES
        .iter()
        .find_map(|suffix| {
            version
                .strip_suffix(suffix)
                .filter(|base| base.bytes().any(|byte| byte.is_ascii_digit()))
        })
        .unwrap_or(version)
        .to_owned()
}

pub(super) fn compact_versions(mut old: &str, mut new: &str) -> (String, String) {
    while let (Some((old_base, old_suffix)), Some((new_base, new_suffix))) =
        (old.rsplit_once('-'), new.rsplit_once('-'))
    {
        if old_suffix != new_suffix
            || !old_suffix.chars().any(char::is_alphabetic)
            || !old_suffix
                .chars()
                .all(|character| character.is_alphanumeric() || matches!(character, '_' | '.'))
        {
            break;
        }
        old = old_base;
        new = new_base;
    }
    (old.to_owned(), new.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_store_names_and_derivations() {
        for (file, expected) in [
            ("foo-1.0-bin", ("foo", "1.0-bin")),
            ("foo-bar-2.0.drv", ("foo-bar", "2.0")),
            ("slumber-config.toml", ("slumber-config.toml", "")),
            ("slumber-config.toml.drv", ("slumber-config.toml", "")),
            (
                "X-Restart-Triggers-dbus-broker",
                ("X-Restart-Triggers-dbus-broker", ""),
            ),
            ("-1-leading-2.0", ("-1-leading", "2.0")),
            ("foo-", ("foo-", "")),
            ("foo-\u{0661}", ("foo-\u{0661}", "")),
        ] {
            let path = format!("/nix/store/0123456789abcdefghijklmnopqrstuv-{file}");
            assert_eq!(split_store_name(&path), expected, "{file}");
        }
    }

    #[test]
    fn recognizes_only_complete_git_hash_suffixes() {
        for hash in [
            "abcdef0",
            "ABCDEF0",
            "abcdef0123456789abcdef0123456789abcdef01",
        ] {
            let path = format!("/nix/store/0123456789abcdefghijklmnopqrstuv-foo-{hash}");
            assert_eq!(split_store_name(&path), ("foo", hash));
        }
        for suffix in [
            "abcdef",
            "abcdef0123456789abcdef0123456789abcdef012",
            "abcdefg",
            "abcdef0-bin",
        ] {
            let name = format!("foo-{suffix}");
            let path = format!("/nix/store/0123456789abcdefghijklmnopqrstuv-{name}");
            assert_eq!(split_store_name(&path), (name.as_str(), ""));
        }
        assert_eq!(split_store_name("foo-1234567"), ("foo", "1234567"));
    }

    #[test]
    fn compacts_known_outputs_once_and_only_with_digits() {
        for (version, expected) in [
            ("10.3.2_fish-completions", "10.3.2"),
            ("1.0-fish-completions", "1.0"),
            ("1.0-fhsenv-profile", "1.0"),
            ("1.0-fhsenv-rootfs", "1.0"),
            ("1.0-libgcc", "1.0"),
            ("1.0-bwrap", "1.0"),
            ("1.0-extracted", "1.0"),
            ("1.0-patched", "1.0"),
            ("1.0-init", "1.0"),
            ("1.0-bin", "1.0"),
            ("1.0-dev", "1.0"),
            ("1.0-doc", "1.0"),
            ("1.0-info", "1.0"),
            ("1.0-lib", "1.0"),
            ("1.0-man", "1.0"),
            ("1.0-out", "1.0"),
            ("1.0-static", "1.0"),
            ("1.0-debug", "1.0"),
            ("1.0-bin-debug", "1.0-bin"),
            ("abcdefa-bin", "abcdefa-bin"),
            ("1.0-rc1", "1.0-rc1"),
            ("", ""),
        ] {
            assert_eq!(compact_version(version), expected, "{version}");
        }
    }

    #[test]
    fn strips_only_shared_output_suffixes() {
        for (old, new, expected_old, expected_new) in [
            ("1.6.5-bwrap", "1.8.3-bwrap", "1.6.5", "1.8.3"),
            ("1.2.0-rc1", "1.3.0", "1.2.0-rc1", "1.3.0"),
            (
                "1.0-custom_foo.bar-dev",
                "2.0-custom_foo.bar-dev",
                "1.0",
                "2.0",
            ),
            ("1.0-rc1", "2.0-rc1", "1.0", "2.0"),
            ("1.0-123", "2.0-123", "1.0-123", "2.0-123"),
            ("1.0-foo+bar", "2.0-foo+bar", "1.0-foo+bar", "2.0-foo+bar"),
            ("1.0-", "2.0-", "1.0-", "2.0-"),
            ("foo", "foo", "foo", "foo"),
            ("", "", "", ""),
        ] {
            assert_eq!(
                compact_versions(old, new),
                (expected_old.to_owned(), expected_new.to_owned())
            );
        }
    }
}
