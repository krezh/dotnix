use super::*;

fn options(bitrate: &str, encode_resolution: &str, max_fps: u32) -> RecordingOptions {
    RecordingOptions {
        bitrate: bitrate.to_string(),
        encode_resolution: encode_resolution.to_string(),
        max_fps,
        codec: "auto".to_string(),
    }
}

#[test]
fn keeps_a_configured_bitrate() {
    assert_eq!(
        resolve_bitrate(&options("15 MB", "", 60), Some((2560, 1440))),
        "15 MB"
    );
}

#[test]
fn scales_the_bitrate_with_the_recorded_area() {
    let options = options("", "", 60);
    assert_eq!(resolve_bitrate(&options, Some((1920, 1080))), "5 MB");
    assert_eq!(resolve_bitrate(&options, Some((2560, 1440))), "10 MB");
    assert_eq!(resolve_bitrate(&options, Some((3840, 2160))), "22 MB");
}

#[test]
fn sizes_the_bitrate_to_the_encoder_resolution() {
    let options = options("", "1920x1080", 60);
    assert_eq!(resolve_bitrate(&options, Some((2560, 1440))), "5 MB");
}

#[test]
fn falls_back_without_a_known_area() {
    assert_eq!(resolve_bitrate(&options("", "", 60), None), DEFAULT_BITRATE);
}

#[test]
fn reads_the_current_process_identity() {
    let pid = std::process::id();
    assert!(process_start_time(pid).is_some());
}

#[test]
fn rejects_a_recorder_that_exits_during_startup() {
    use clap::Parser;
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let recorder = directory.path().join("wl-screenrec");
    fs::write(&recorder, "#!/bin/sh\nexit 23\n").unwrap();
    fs::set_permissions(&recorder, fs::Permissions::from_mode(0o700)).unwrap();

    let previous = std::env::var_os("XDG_RUNTIME_DIR");
    std::env::set_var("XDG_RUNTIME_DIR", directory.path());
    let mut settings = crate::cli::Args::parse_from(["chomp"]).resolve(Default::default());
    settings.wl_screenrec = recorder.to_string_lossy().into_owned();
    let result = start_recording(
        &settings,
        None,
        None,
        Some((1920, 1080)),
        &directory.path().join("recording.mp4"),
    );
    match previous {
        Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
        None => std::env::remove_var("XDG_RUNTIME_DIR"),
    }

    assert!(result.is_err());
}
