use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub install_dir: PathBuf,
    pub executable_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct GameInfo {
    pub app_id: String,
    pub name: String,
    pub install_dir: String,
    pub state_flags: Option<String>,
    pub has_language_config: bool,
}

impl SteamGame {
    pub fn new(app_id: String, name: String, install_dir: PathBuf) -> Self {
        Self {
            app_id,
            name,
            install_dir,
            executable_path: None,
        }
    }

    pub fn with_executable(mut self, exe_path: PathBuf) -> Self {
        self.executable_path = Some(exe_path);
        self
    }
}
