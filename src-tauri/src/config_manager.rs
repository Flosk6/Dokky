use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::DokkyError;

/// A game action shortcut: press a key → tap in a zone.
/// Position is randomized within the zone to simulate human-like taps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAction {
    /// Keyboard key (e.g. "1", "2", "a", "Space")
    pub key: String,
    /// Display label (e.g. "Sort 1", "Attaque")
    pub label: String,
    /// Normalized center X (0.0 - 1.0)
    pub x: f64,
    /// Normalized center Y (0.0 - 1.0)
    pub y: f64,
    /// Normalized zone width (0.0 - 1.0), default 0.02
    #[serde(default = "default_zone_size")]
    pub w: f64,
    /// Normalized zone height (0.0 - 1.0), default 0.02
    #[serde(default = "default_zone_size")]
    pub h: f64,
}

fn default_zone_size() -> f64 {
    0.02
}

fn default_video_preset() -> String {
    "high".to_string()
}

fn default_true() -> bool {
    true
}

fn default_iframe_interval() -> u32 {
    2
}

/// Video & performance settings. When preset is not "custom", these are ignored
/// and the preset values are used instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    /// Resolution width (e.g. 1920)
    #[serde(default)]
    pub width: u32,
    /// Resolution height (e.g. 1080)
    #[serde(default)]
    pub height: u32,
    /// DPI for virtual display
    #[serde(default)]
    pub dpi: u32,
    /// Max FPS
    #[serde(default)]
    pub fps: u32,
    /// Bitrate in bps
    #[serde(default)]
    pub bitrate: u32,
    /// Use H.264 Baseline profile (no B-frames, less CPU)
    #[serde(default = "default_true")]
    pub baseline_profile: bool,
    /// I-frame interval in seconds
    #[serde(default = "default_iframe_interval")]
    pub iframe_interval: u32,
    /// Hide system decorations on virtual display
    #[serde(default = "default_true")]
    pub no_vd_system_decorations: bool,
    /// Disable Android animations on device
    #[serde(default)]
    pub disable_animations: bool,
    /// Turn off device screen to save battery
    #[serde(default)]
    pub screen_off: bool,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            dpi: 240,
            fps: 60,
            bitrate: 8_000_000,
            baseline_profile: true,
            iframe_interval: 2,
            no_vd_system_decorations: true,
            disable_animations: false,
            screen_off: false,
        }
    }
}

/// Navigation shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationShortcuts {
    pub new_session: String,
    pub close_session: String,
    pub next_tab: String,
    pub prev_tab: String,
}

impl Default for NavigationShortcuts {
    fn default() -> Self {
        Self {
            new_session: "Ctrl+T".to_string(),
            close_session: "Ctrl+W".to_string(),
            next_tab: "Ctrl+Tab".to_string(),
            prev_tab: "Ctrl+Shift+Tab".to_string(),
        }
    }
}

/// Full app configuration, persisted to ~/.dokky/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub navigation: NavigationShortcuts,
    pub game_actions: Vec<GameAction>,
    #[serde(default = "default_video_preset")]
    pub video_preset: String,
    #[serde(default)]
    pub video_settings: VideoSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            navigation: NavigationShortcuts::default(),
            game_actions: Vec::new(),
            video_preset: default_video_preset(),
            video_settings: VideoSettings::default(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dokky")
        .join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), DokkyError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DokkyError::ApkCloneFailed(format!("create config dir: {}", e)))?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("serialize config: {}", e)))?;
    std::fs::write(&path, data)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("write config: {}", e)))?;
    log::info!("[config] Saved to {:?}", path);
    Ok(())
}
