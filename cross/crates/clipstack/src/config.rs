//! User configuration: JSON file, every key optional, same shape as the macOS Swift app.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Maximum number of items kept on the stack. Oldest are dropped first.
    pub max_items: usize,
    /// History circulates at this size; starred items never count.
    pub max_history: usize,
    /// Save history to disk so it survives restarts. Starred items are always saved.
    pub persist_history: bool,
    /// Seconds between clipboard checks.
    pub poll_interval: f64,
    /// Show the stack count next to the tray icon (macOS title / Windows+Linux tooltip).
    pub show_count_in_menubar: bool,
    /// Which end of the stack the main paste hotkey takes: "lifo" (newest first, default)
    /// or "fifo" (oldest first — paste in the order you copied). The second paste hotkey
    /// always takes the other end. Also switchable from the menu.
    pub paste_order: String,
    pub hotkeys: Hotkeys,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    /// Pop the most recently copied item and paste it (LIFO).
    pub paste_newest: String,
    /// Pop the oldest item and paste it (FIFO).
    pub paste_oldest: String,
    /// Empty the stack.
    pub clear: String,
    /// Open the tray menu (where the platform allows it).
    pub peek: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                paste_newest: "cmd+ctrl+v".into(),
                paste_oldest: "cmd+ctrl+shift+v".into(),
                clear: "cmd+ctrl+backspace".into(),
                peek: "cmd+ctrl+l".into(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                paste_newest: "ctrl+alt+v".into(),
                paste_oldest: "ctrl+alt+shift+v".into(),
                clear: "ctrl+alt+backspace".into(),
                peek: "ctrl+alt+l".into(),
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_items: 5,
            max_history: 15,
            persist_history: true,
            poll_interval: 0.2,
            show_count_in_menubar: true,
            paste_order: "lifo".into(),
            hotkeys: Hotkeys::default(),
        }
    }
}

impl Config {
    pub fn is_fifo(&self) -> bool {
        self.paste_order.eq_ignore_ascii_case("fifo")
    }

    /// ~/.config/clipstack on macOS and Linux, %APPDATA%\clipstack on Windows.
    pub fn dir() -> PathBuf {
        #[cfg(windows)]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("clipstack")
        }
        #[cfg(not(windows))]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("clipstack")
        }
    }

    pub fn file() -> PathBuf {
        Self::dir().join("config.json")
    }

    /// Loads the config, writing a default file first if none exists.
    /// A broken file is logged and defaults are used so the app still runs.
    pub fn load() -> Config {
        let path = Self::file();
        if !path.exists() {
            let c = Config::default();
            c.save();
            return c;
        }
        match std::fs::read_to_string(&path).map_err(anyhow::Error::from).and_then(|s| {
            serde_json::from_str::<Config>(&s).map_err(anyhow::Error::from)
        }) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("could not read {}: {e}; using defaults", path.display());
                Config::default()
            }
        }
    }

    pub fn save(&self) {
        let path = Self::file();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&path, s) {
                    log::warn!("could not write {}: {e}", path.display());
                }
            }
            Err(e) => log::warn!("could not serialise config: {e}"),
        }
    }
}
