use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

const DEFAULT_REFRESH_INTERVAL: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
}

fn default_refresh_interval() -> u64 {
    DEFAULT_REFRESH_INTERVAL
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: DEFAULT_REFRESH_INTERVAL,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = get_settings_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let settings: Settings = toml::from_str(&content)?;
            Ok(settings)
        } else {
            Ok(Settings::default())
        }
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    let xdg_config_home = env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from);
    let config_dir = xdg_config_home.unwrap_or_else(|| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home).join(".config")
    });
    let dir = config_dir.join("rss");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("feeds.json"))
}

pub fn get_settings_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path_returns_feeds_json() {
        let path = get_config_path().unwrap();

        assert!(path.to_string_lossy().contains(".config/rss/feeds.json"));
    }

    #[test]
    fn test_get_settings_path_returns_config_toml() {
        let path = get_settings_path().unwrap();

        assert!(path.to_string_lossy().contains(".config/rss/config.toml"));
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();

        assert_eq!(settings.refresh_interval_secs, 300);
    }

    #[test]
    fn test_settings_load_returns_default_when_file_missing() {
        let settings = Settings::load().unwrap();

        assert_eq!(settings.refresh_interval_secs, 300);
    }
}
