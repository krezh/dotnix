use std::{fs, path::Path};

fn sources(dir: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path.to_string_lossy().into_owned());
        }
    }
}

#[test]
fn features_never_reach_the_toolkit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/features");
    let mut files = Vec::new();
    sources(&root, &mut files);
    assert!(files.len() > 100);
    let prefix = format!("{}/", root.display());
    let leaks: Vec<String> = files
        .iter()
        .filter(|path| {
            let relative = path.trim_start_matches(&prefix);
            let source = fs::read_to_string(path).unwrap();
            !relative.ends_with("_test.rs")
                && (source.contains("gpui")
                    || source.contains("Context<")
                    || source.contains("island::{")
                    || source.contains("\n    island,\n")
                    || source.contains("crate::island"))
        })
        .cloned()
        .collect();
    assert!(leaks.is_empty(), "the toolkit leaked into {leaks:#?}");
}
