use anyhow::Result;
use std::env;
use std::path::PathBuf;

pub fn get_config_path() -> Result<PathBuf> {
    let xdg_config_home = env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from);
    let path = get_config_path_with_xdg(xdg_config_home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn get_config_path_with_xdg(xdg_config_home: Option<PathBuf>) -> PathBuf {
    let config_dir = xdg_config_home.unwrap_or_else(|| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home).join(".config")
    });
    config_dir.join("rss").join("feeds.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir_returns_xdg_path() {
        let path = get_config_path().unwrap();

        assert!(path.to_string_lossy().contains(".config/rss/feeds.json"));
    }

    #[test]
    fn test_get_config_dir_with_custom_xdg() {
        let path = get_config_path_with_xdg(Some("/tmp/custom-config".into()));

        assert_eq!(path, PathBuf::from("/tmp/custom-config/rss/feeds.json"));
    }

    #[test]
    fn test_get_config_dir_without_xdg_uses_home() {
        let home = std::env::var("HOME").unwrap();
        let path = get_config_path_with_xdg(None);

        assert_eq!(path, PathBuf::from(format!("{}/.config/rss/feeds.json", home)));
    }
}
