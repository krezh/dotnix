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
