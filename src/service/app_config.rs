use directories_next::ProjectDirs;
use iced::Theme;
use serde::{Deserialize, Serialize};

use crate::model::flow8::{FLOW8Controller, SyncInterval};

const CONFIG_FILE_NAME: &str = "settings.toml";

/// User-facing settings persisted across restarts.
///
/// We keep the file format intentionally small and string-based so it stays easy
/// to review in a pull request and easy to migrate later if the app grows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub theme: Option<String>,
    pub sync_interval: Option<String>,
    pub close_to_tray_on_close: Option<bool>,
}

pub fn load() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config {}: {}", path.display(), e))?;

    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config {}: {}", path.display(), e))
}

pub fn save_from_controller(controller: &FLOW8Controller) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir {}: {}", parent.display(), e))?;
    }

    let config = AppConfig {
        theme: Some(controller.theme.to_string()),
        sync_interval: Some(controller.sync_interval.to_string()),
        close_to_tray_on_close: Some(controller.close_to_tray_on_close),
    };

    let content = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write config {}: {}", path.display(), e))
}

pub fn apply_to_controller(config: &AppConfig, controller: &mut FLOW8Controller) {
    if let Some(theme_name) = config.theme.as_deref() {
        if let Some(theme) = parse_theme(theme_name) {
            controller.theme = theme;
        }
    }

    if let Some(interval_name) = config.sync_interval.as_deref() {
        if let Some(interval) = parse_sync_interval(interval_name) {
            controller.sync_interval = interval;
        }
    }

    if let Some(close_to_tray) = config.close_to_tray_on_close {
        controller.close_to_tray_on_close = close_to_tray;
    }
}

fn config_path() -> Result<std::path::PathBuf, String> {
    let dirs = ProjectDirs::from("br", "abelroes", "flow-8-midi")
        .ok_or_else(|| "Could not resolve application config directory".to_string())?;
    Ok(dirs.config_dir().join(CONFIG_FILE_NAME))
}

fn parse_theme(name: &str) -> Option<Theme> {
    Theme::ALL
        .iter()
        .find(|theme| theme.to_string() == name)
        .cloned()
}

fn parse_sync_interval(name: &str) -> Option<SyncInterval> {
    SyncInterval::ALL
        .iter()
        .find(|interval| interval.to_string() == name)
        .copied()
}
