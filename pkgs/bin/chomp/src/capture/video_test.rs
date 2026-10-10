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
        resolve_bitrate(&options("15 MB", "", 60), 2560, 1440),
        "15 MB"
    );
}

#[test]
fn scales_the_bitrate_with_the_recorded_area() {
    let options = options("", "", 60);
    assert_eq!(resolve_bitrate(&options, 1920, 1080), "5 MB");
    assert_eq!(resolve_bitrate(&options, 2560, 1440), "10 MB");
    assert_eq!(resolve_bitrate(&options, 3840, 2160), "22 MB");
}

#[test]
fn sizes_the_bitrate_to_the_encoder_resolution() {
    let options = options("", "1920x1080", 60);
    let (width, height) = output_dimensions(&options, 2560, 1440).unwrap();
    assert_eq!(resolve_bitrate(&options, width, height), "5 MB");
}
