use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub preferred_proxy_dll: String,
    pub check_updates_on_startup: bool,
    pub last_update_check: Option<DateTime<Utc>>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            preferred_proxy_dll: "dxgi.dll".to_string(),
            check_updates_on_startup: false,
            last_update_check: None,
        }
    }
}
