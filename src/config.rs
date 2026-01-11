use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

const DEFAULT_REFRESH_INTERVAL: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    RefreshIntervalSecs,
    AutoSort,
}

impl FromStr for ConfigKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "refresh_interval_secs" => Ok(ConfigKey::RefreshIntervalSecs),
            "auto_sort" => Ok(ConfigKey::AutoSort),
            _ => anyhow::bail!("Unknown setting: {}", s),
        }
    }
}

impl fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigKey::RefreshIntervalSecs => write!(f, "refresh_interval_secs"),
            ConfigKey::AutoSort => write!(f, "auto_sort"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default)]
    pub auto_sort: bool,
}

fn default_refresh_interval() -> u64 {
    DEFAULT_REFRESH_INTERVAL
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: DEFAULT_REFRESH_INTERVAL,
            auto_sort: false,
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

    pub fn save(&self) -> Result<()> {
        let path = get_settings_path()?;
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn set(&mut self, key: ConfigKey, value: &str) -> Result<()> {
        match key {
            ConfigKey::RefreshIntervalSecs => {
                self.refresh_interval_secs = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid value for {}", key))?;
            }
            ConfigKey::AutoSort => {
                self.auto_sort = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid value for {} (use true/false)", key))?;
            }
        }
        Ok(())
    }

    pub fn display(&self) -> String {
        format!(
            "refresh_interval_secs = {}\nauto_sort = {}",
            self.refresh_interval_secs, self.auto_sort
        )
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

    #[test]
    fn test_settings_auto_sort_default_false() {
        let settings = Settings::default();

        assert!(!settings.auto_sort);
    }

    #[test]
    fn test_settings_set_valid_key() {
        let mut settings = Settings::default();

        let result = settings.set(ConfigKey::AutoSort, "true");

        assert!(result.is_ok());
        assert!(settings.auto_sort);
    }

    #[test]
    fn test_settings_set_invalid_value() {
        let mut settings = Settings::default();

        let result = settings.set(ConfigKey::AutoSort, "invalid");

        assert!(result.is_err());
    }

    #[test]
    fn test_config_key_from_str_refresh_interval() {
        let key: ConfigKey = "refresh_interval_secs".parse().unwrap();

        assert!(matches!(key, ConfigKey::RefreshIntervalSecs));
    }

    #[test]
    fn test_config_key_from_str_auto_sort() {
        let key: ConfigKey = "auto_sort".parse().unwrap();

        assert!(matches!(key, ConfigKey::AutoSort));
    }

    #[test]
    fn test_config_key_from_str_invalid() {
        let result: Result<ConfigKey, _> = "invalid_key".parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_config_key_display_refresh_interval() {
        assert_eq!(
            ConfigKey::RefreshIntervalSecs.to_string(),
            "refresh_interval_secs"
        );
    }

    #[test]
    fn test_config_key_display_auto_sort() {
        assert_eq!(ConfigKey::AutoSort.to_string(), "auto_sort");
    }
}
