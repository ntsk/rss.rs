use anyhow::Result;
use std::path::PathBuf;

pub fn get_config_path() -> Result<PathBuf> {
    todo!()
}

fn get_config_path_with_xdg(_xdg_config_home: Option<PathBuf>) -> PathBuf {
    todo!()
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
