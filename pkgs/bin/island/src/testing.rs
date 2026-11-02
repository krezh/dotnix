use std::{
    env, fs,
    path::PathBuf,
    process,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let next = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!("island-{name}-{}-{next}", process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}
