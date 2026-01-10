use anyhow::Result;
use serde::{Deserialize, Serialize};
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
        todo!()
    }

    pub fn add(&mut self, url: &str) -> Result<()> {
        todo!()
    }

    pub fn delete(&mut self, url: &str) -> Result<bool> {
        todo!()
    }

    pub fn list(&self) -> &[Feed] {
        todo!()
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
        manager.add("https://example.com/feed.xml").unwrap();

        let feeds = manager.list();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].url, "https://example.com/feed.xml");
    }

    #[test]
    fn test_add_duplicate_feed_returns_error() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed.xml").unwrap();

        let result = manager.add("https://example.com/feed.xml");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_feed() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.json");

        let mut manager = SubscriptionManager::new(&config_path).unwrap();
        manager.add("https://example.com/feed.xml").unwrap();

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
            manager.add("https://example.com/feed1.xml").unwrap();
            manager.add("https://example.com/feed2.xml").unwrap();
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
        manager.add("https://example.com/feed1.xml").unwrap();
        manager.add("https://example.com/feed2.xml").unwrap();
        manager.add("https://example.com/feed3.xml").unwrap();

        let feeds = manager.list();

        assert_eq!(feeds.len(), 3);
    }
}
