use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
}

pub struct SubscriptionManager {
    feeds: Vec<Feed>,
    config_path: std::path::PathBuf,
}

impl SubscriptionManager {
    pub fn new(config_path: impl AsRef<Path>) -> Result<Self> {
        let config_path = config_path.as_ref().to_path_buf();
        let feeds = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)?
        } else {
            Vec::new()
        };

        Ok(Self { feeds, config_path })
    }

    pub fn add(&mut self, url: &str) -> Result<()> {
        if self.feeds.iter().any(|f| f.url == url) {
            bail!("Feed already exists: {}", url);
        }

        self.feeds.push(Feed {
            url: url.to_string(),
            title: None,
        });
        self.save()
    }

    pub fn delete(&mut self, url: &str) -> Result<bool> {
        let original_len = self.feeds.len();
        self.feeds.retain(|f| f.url != url);

        if self.feeds.len() != original_len {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn list(&self) -> &[Feed] {
        &self.feeds
    }

    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.feeds)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_creates_empty_subscription_list() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let manager = SubscriptionManager::new(&config_path).unwrap();

        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_add_feed() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed.xml", None).unwrap();

        let feeds = manager.list();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].url, "https://example.com/feed.xml");
    }

    #[test]
    fn test_add_duplicate_feed_returns_error() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed.xml", None).unwrap();

        let result = manager.add("https://example.com/feed.xml", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_feed() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed.xml", None).unwrap();

        let deleted = manager.delete("https://example.com/feed.xml").unwrap();

        assert!(deleted);
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_delete_nonexistent_feed_returns_false() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();

        let deleted = manager.delete("https://example.com/feed.xml").unwrap();

        assert!(!deleted);
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        {
            let mut manager = SubscriptionManager::new(&config_path).unwrap();
            manager.add("https://example.com/feed1.xml", None).unwrap();
            manager.add("https://example.com/feed2.xml", None).unwrap();
        }

        let manager = SubscriptionManager::new(&config_path).unwrap();
        let feeds = manager.list();

        assert_eq!(feeds.len(), 2);
    }

    #[test]
    fn test_list_returns_all_feeds() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed1.xml", None).unwrap();
        manager.add("https://example.com/feed2.xml", None).unwrap();
        manager.add("https://example.com/feed3.xml", None).unwrap();

        let feeds = manager.list();

        assert_eq!(feeds.len(), 3);
    }

    #[test]
    fn test_add_feed_with_title() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager
            .add("https://example.com/feed.xml", Some("My Blog".to_string()))
            .unwrap();

        let feeds = manager.list();
        assert_eq!(feeds[0].title, Some("My Blog".to_string()));
    }

    #[test]
    fn test_add_feed_title_persists() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        {
            let mut manager = SubscriptionManager::new(&config_path).unwrap();
            manager
                .add("https://example.com/feed.xml", Some("My Blog".to_string()))
                .unwrap();
        }

        let manager = SubscriptionManager::new(&config_path).unwrap();
        let feeds = manager.list();
        assert_eq!(feeds[0].title, Some("My Blog".to_string()));
    }
}
