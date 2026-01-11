use crate::config;
use crate::feed::{self, Article};
use crate::subscription::SubscriptionManager;
use rayon::prelude::*;
use std::collections::HashMap;

pub struct FetchResult {
    pub articles: Vec<Article>,
    pub failed_feeds: Vec<String>,
    pub feed_status: HashMap<String, bool>,
}

impl FetchResult {
    pub fn failure_message(&self) -> Option<String> {
        if self.failed_feeds.is_empty() {
            return None;
        }

        if self.failed_feeds.len() == 1 {
            Some(format!("Failed: {}", self.failed_feeds[0]))
        } else {
            Some(format!("Failed: {} feeds", self.failed_feeds.len()))
        }
    }
}

struct FeedFetchOutcome {
    url: String,
    articles: Vec<Article>,
    error_name: Option<String>,
    success: bool,
}

pub fn fetch_all_feeds(manager: &SubscriptionManager) -> FetchResult {
    let feeds = manager.list();

    let outcomes: Vec<FeedFetchOutcome> = feeds
        .par_iter()
        .map(|f| {
            let name = f.title.clone().unwrap_or_else(|| f.url.clone());
            match feed::fetch_articles(&f.url) {
                Ok(articles) => FeedFetchOutcome {
                    url: f.url.clone(),
                    articles,
                    error_name: None,
                    success: true,
                },
                Err(_) => FeedFetchOutcome {
                    url: f.url.clone(),
                    articles: vec![],
                    error_name: Some(name),
                    success: false,
                },
            }
        })
        .collect();

    let mut articles: Vec<Article> = outcomes.iter().flat_map(|o| o.articles.clone()).collect();

    let failed_feeds: Vec<String> = outcomes
        .iter()
        .filter_map(|o| o.error_name.clone())
        .collect();

    let feed_status: HashMap<String, bool> = outcomes
        .iter()
        .map(|o| (o.url.clone(), o.success))
        .collect();

    articles.sort_by(|a, b| b.published.cmp(&a.published));

    FetchResult {
        articles,
        failed_feeds,
        feed_status,
    }
}

pub fn fetch_feeds_from_config() -> Option<FetchResult> {
    let config_path = config::get_config_path().ok()?;
    let manager = SubscriptionManager::new(&config_path).ok()?;

    let result = fetch_all_feeds(&manager);

    if result.articles.is_empty() && result.failed_feeds.is_empty() {
        return None;
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_result_no_failures() {
        let result = FetchResult {
            articles: vec![],
            failed_feeds: vec![],
            feed_status: HashMap::new(),
        };

        assert!(result.failure_message().is_none());
    }

    #[test]
    fn test_fetch_result_single_failure() {
        let result = FetchResult {
            articles: vec![],
            failed_feeds: vec!["https://example.com/feed.xml".to_string()],
            feed_status: HashMap::new(),
        };

        let message = result.failure_message().unwrap();
        assert!(message.contains("Failed"));
        assert!(message.contains("https://example.com/feed.xml"));
    }

    #[test]
    fn test_fetch_result_multiple_failures() {
        let result = FetchResult {
            articles: vec![],
            failed_feeds: vec![
                "https://example.com/feed1.xml".to_string(),
                "https://example.com/feed2.xml".to_string(),
            ],
            feed_status: HashMap::new(),
        };

        let message = result.failure_message().unwrap();
        assert!(message.contains("Failed"));
        assert!(message.contains("2"));
    }
}
