use anyhow::Result;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub link: String,
    pub published: Option<DateTime<Utc>>,
    pub feed_title: String,
}

pub fn fetch_articles(url: &str) -> Result<Vec<Article>> {
    todo!()
}

pub fn parse_rss(content: &str, feed_url: &str) -> Result<Vec<Article>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Sample Blog</title>
    <link>https://example.com</link>
    <description>A sample blog</description>
    <item>
      <title>First Post</title>
      <link>https://example.com/first</link>
      <pubDate>Sat, 01 Jan 2025 12:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Second Post</title>
      <link>https://example.com/second</link>
      <pubDate>Sun, 02 Jan 2025 12:00:00 +0000</pubDate>
    </item>
  </channel>
</rss>"#;

    const SAMPLE_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Sample Atom Feed</title>
  <link href="https://example.com"/>
  <entry>
    <title>Atom Entry</title>
    <link href="https://example.com/atom-entry"/>
    <updated>2025-01-03T12:00:00Z</updated>
  </entry>
</feed>"#;

    #[test]
    fn test_parse_rss_returns_articles() {
        let articles = parse_rss(SAMPLE_RSS, "https://example.com/feed.xml").unwrap();

        assert_eq!(articles.len(), 2);
    }

    #[test]
    fn test_parse_rss_article_has_title() {
        let articles = parse_rss(SAMPLE_RSS, "https://example.com/feed.xml").unwrap();

        assert_eq!(articles[0].title, "First Post");
        assert_eq!(articles[1].title, "Second Post");
    }

    #[test]
    fn test_parse_rss_article_has_link() {
        let articles = parse_rss(SAMPLE_RSS, "https://example.com/feed.xml").unwrap();

        assert_eq!(articles[0].link, "https://example.com/first");
    }

    #[test]
    fn test_parse_rss_article_has_published_date() {
        let articles = parse_rss(SAMPLE_RSS, "https://example.com/feed.xml").unwrap();

        assert!(articles[0].published.is_some());
    }

    #[test]
    fn test_parse_rss_article_has_feed_title() {
        let articles = parse_rss(SAMPLE_RSS, "https://example.com/feed.xml").unwrap();

        assert_eq!(articles[0].feed_title, "Sample Blog");
    }

    #[test]
    fn test_parse_atom_returns_articles() {
        let articles = parse_rss(SAMPLE_ATOM, "https://example.com/atom.xml").unwrap();

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Atom Entry");
    }

    #[test]
    fn test_parse_invalid_feed_returns_error() {
        let result = parse_rss("invalid xml", "https://example.com/feed.xml");

        assert!(result.is_err());
    }
}
