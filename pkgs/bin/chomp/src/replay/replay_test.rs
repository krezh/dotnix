use super::*;

#[test]
fn resolves_relative_output_against_the_callers_directory() {
    let current_directory = std::env::current_dir().unwrap();
    let output = absolute_output_path(PathBuf::from("clip.mp4")).unwrap();

    assert_eq!(output, current_directory.join("clip.mp4"));
}

#[test]
fn preserves_absolute_output_paths() {
    let output = PathBuf::from("/tmp/clip.mp4");

    assert_eq!(absolute_output_path(output.clone()).unwrap(), output);
}
