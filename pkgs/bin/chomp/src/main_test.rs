use super::*;

fn settings() -> Settings {
    Args::parse_from(["chomp"]).resolve(Config::default())
}

#[test]
fn generated_output_names_do_not_collide() {
    let directory = tempfile::tempdir().unwrap();
    let mut settings = settings();
    settings.save_path = directory.path().to_path_buf();

    let first = generate_output_path(&settings, "png").unwrap();
    let second = generate_output_path(&settings, "png").unwrap();

    assert_ne!(first, second);
    assert!(!first.exists());
    assert!(!second.exists());
}

#[test]
fn rejects_an_output_extension_that_does_not_match_the_format() {
    let mut settings = settings();
    settings.output = Some(PathBuf::from("capture.jpg"));

    assert!(generate_output_path(&settings, "png").is_err());
}

#[test]
fn no_upload_overrides_configured_zipline() {
    let mut config = Config::default();
    config.upload.zipline.url = "https://zipline.example".to_string();
    config.upload.zipline.token = "/run/secrets/zipline".to_string();

    assert!(should_upload(
        &Args::parse_from(["chomp"]).resolve(config.clone())
    ));
    assert!(!should_upload(
        &Args::parse_from(["chomp", "--no-upload"]).resolve(config)
    ));
}
