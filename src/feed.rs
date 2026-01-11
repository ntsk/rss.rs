use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub link: String,
    pub published: Option<DateTime<Utc>>,
    pub feed_title: String,
}

fn sanitize_text(text: impl AsRef<str>) -> String {
    text.as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_xml(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '&' {
            if is_valid_entity(&chars, i) {
                result.push('&');
            } else {
                result.push_str("&amp;");
            }
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }

    result
}

fn is_valid_entity(chars: &[char], start: usize) -> bool {
    let remaining: String = chars[start..].iter().collect();

    if remaining.starts_with("&amp;")
        || remaining.starts_with("&lt;")
        || remaining.starts_with("&gt;")
        || remaining.starts_with("&quot;")
        || remaining.starts_with("&apos;")
    {
        return true;
    }

    if let Some(hex_part) = remaining
        .strip_prefix("&#x")
        .or_else(|| remaining.strip_prefix("&#X"))
    {
        for (j, c) in hex_part.chars().enumerate() {
            if c == ';' {
                return j > 0;
            }
            if !c.is_ascii_hexdigit() {
                return false;
            }
        }
    }

    if let Some(num_part) = remaining.strip_prefix("&#") {
        for (j, c) in num_part.chars().enumerate() {
            if c == ';' {
                return j > 0;
            }
            if !c.is_ascii_digit() {
                return false;
            }
        }
    }

    false
}

fn create_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .no_proxy()
        .build()
        .map_err(Into::into)
}

pub fn fetch_articles(url: &str) -> Result<Vec<Article>> {
    let client = create_client()?;
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to connect to {}", url))?;
    let content = response
        .text()
        .with_context(|| format!("Failed to read response from {}", url))?;
    parse_feed(&content).with_context(|| format!("Failed to parse feed from {}", url))
}

pub fn fetch_article_content(url: &str, width: usize) -> Result<String> {
    let client = create_client()?;
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to connect to {}", url))?;
    let html = response
        .text()
        .with_context(|| format!("Failed to read response from {}", url))?;

    let (target_url, target_html) = resolve_article_url(&client, url, &html)?;

    let mut content_html = target_html.clone();
    if let Ok(parsed_url) = target_url.parse::<reqwest::Url>()
        && let Ok(extracted) =
            readability::extractor::extract(&mut target_html.as_bytes(), &parsed_url)
    {
        content_html = extracted.content;
    }

    Ok(html2text::from_read(content_html.as_bytes(), width))
}

fn resolve_article_url(
    client: &reqwest::blocking::Client,
    url: &str,
    html: &str,
) -> Result<(String, String)> {
    if let Some(canonical_url) = extract_canonical_url(html)
        && canonical_url != url
    {
        let response = client
            .get(&canonical_url)
            .send()
            .with_context(|| format!("Failed to connect to {}", canonical_url))?;
        let canonical_html = response
            .text()
            .with_context(|| format!("Failed to read response from {}", canonical_url))?;
        return Ok((canonical_url, canonical_html));
    }
    Ok((url.to_string(), html.to_string()))
}

fn extract_canonical_url(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let canonical_pos = lower.find("rel=\"canonical\"")?;
    let search_start = canonical_pos.saturating_sub(200);
    let search_area = &html[search_start..canonical_pos + 50];

    if let Some(href_pos) = search_area.to_lowercase().find("href=\"") {
        let href_start = search_start + href_pos + 6;
        let rest = &html[href_start..];
        if let Some(end) = rest.find('"') {
            let url = rest[..end].to_string();
            if url.starts_with("http") {
                return Some(url);
            }
        }
    }
    None
}

pub fn parse_feed(content: &str) -> Result<Vec<Article>> {
    if let Ok(channel) = content.parse::<rss::Channel>() {
        return parse_rss_channel(&channel);
    }

    if let Ok(feed) = content.parse::<atom_syndication::Feed>() {
        return parse_atom_feed(&feed);
    }

    let sanitized = sanitize_xml(content);
    if let Ok(channel) = sanitized.parse::<rss::Channel>() {
        return parse_rss_channel(&channel);
    }

    if let Ok(feed) = sanitized.parse::<atom_syndication::Feed>() {
        return parse_atom_feed(&feed);
    }

    bail!("Failed to parse feed: unsupported format or invalid XML")
}

fn parse_rss_channel(channel: &rss::Channel) -> Result<Vec<Article>> {
    let feed_title = sanitize_text(channel.title());
    let articles = channel
        .items()
        .iter()
        .filter_map(|item| {
            let title = sanitize_text(item.title()?);
            let link = item.link()?.to_string();
            let published = item.pub_date().and_then(parse_date).or_else(|| {
                item.dublin_core_ext()
                    .and_then(|dc| dc.dates().first())
                    .and_then(|d| parse_date(d))
            });

            Some(Article {
                title,
                link,
                published,
                feed_title: feed_title.clone(),
            })
        })
        .collect();

    Ok(articles)
}

fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    let formats = [
        "%a, %d %b %Y %H:%M:%S %z",
        "%a, %d %b %Y %H:%M:%S %Z",
        "%a, %-d %b %Y %H:%M:%S %z",
        "%d %b %Y %H:%M:%S %z",
        "%Y-%m-%dT%H:%M:%S%z",
    ];

    for fmt in formats {
        if let Ok(dt) = DateTime::parse_from_str(date_str, fmt) {
            return Some(dt.with_timezone(&Utc));
        }
    }

    None
}

fn parse_atom_feed(feed: &atom_syndication::Feed) -> Result<Vec<Article>> {
    let feed_title = sanitize_text(feed.title());
    let articles = feed
        .entries()
        .iter()
        .filter_map(|entry| {
            let title = sanitize_text(entry.title());
            let link = entry.links().first()?.href().to_string();
            let published = entry
                .published()
                .unwrap_or_else(|| entry.updated())
                .with_timezone(&Utc);

            Some(Article {
                title,
                link,
                published: Some(published),
                feed_title: feed_title.clone(),
            })
        })
        .collect();

    Ok(articles)
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
      <pubDate>Wed, 01 Jan 2025 12:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Second Post</title>
      <link>https://example.com/second</link>
      <pubDate>Thu, 02 Jan 2025 12:00:00 +0000</pubDate>
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
        let articles = parse_feed(SAMPLE_RSS).unwrap();

        assert_eq!(articles.len(), 2);
    }

    #[test]
    fn test_parse_rss_article_has_title() {
        let articles = parse_feed(SAMPLE_RSS).unwrap();

        assert_eq!(articles[0].title, "First Post");
        assert_eq!(articles[1].title, "Second Post");
    }

    #[test]
    fn test_parse_rss_article_has_link() {
        let articles = parse_feed(SAMPLE_RSS).unwrap();

        assert_eq!(articles[0].link, "https://example.com/first");
    }

    #[test]
    fn test_parse_rss_article_has_published_date() {
        let articles = parse_feed(SAMPLE_RSS).unwrap();

        assert!(articles[0].published.is_some());
    }

    #[test]
    fn test_parse_rss_article_has_feed_title() {
        let articles = parse_feed(SAMPLE_RSS).unwrap();

        assert_eq!(articles[0].feed_title, "Sample Blog");
    }

    #[test]
    fn test_parse_atom_returns_articles() {
        let articles = parse_feed(SAMPLE_ATOM).unwrap();

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Atom Entry");
    }

    #[test]
    fn test_parse_invalid_feed_returns_error() {
        let result = parse_feed("invalid xml");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_atom_prefers_published_over_updated() {
        let atom_with_published = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Feed</title>
  <entry>
    <title>Test Entry</title>
    <link href="https://example.com/entry"/>
    <published>2025-01-01T10:00:00Z</published>
    <updated>2025-01-10T15:00:00Z</updated>
  </entry>
</feed>"#;

        let articles = parse_feed(atom_with_published).unwrap();

        assert_eq!(articles.len(), 1);
        let published = articles[0].published.unwrap();
        assert_eq!(published.format("%Y-%m-%d").to_string(), "2025-01-01");
    }

    #[test]
    fn test_parse_atom_falls_back_to_updated_when_no_published() {
        let atom_without_published = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Feed</title>
  <entry>
    <title>Test Entry</title>
    <link href="https://example.com/entry"/>
    <updated>2025-01-10T15:00:00Z</updated>
  </entry>
</feed>"#;

        let articles = parse_feed(atom_without_published).unwrap();

        assert_eq!(articles.len(), 1);
        let published = articles[0].published.unwrap();
        assert_eq!(published.format("%Y-%m-%d").to_string(), "2025-01-10");
    }

    #[test]
    fn test_parse_rss_removes_newlines_from_titles() {
        let rss_with_newlines = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Sample
Blog</title>
    <item>
      <title>First
Post</title>
      <link>https://example.com/first</link>
    </item>
  </channel>
</rss>"#;

        let articles = parse_feed(rss_with_newlines).unwrap();

        assert_eq!(articles[0].feed_title, "Sample Blog");
        assert_eq!(articles[0].title, "First Post");
    }

    #[test]
    fn test_parse_atom_removes_newlines_from_titles() {
        let atom_with_newlines = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Sample
Atom</title>
  <entry>
    <title>Atom
Entry</title>
    <link href="https://example.com/atom-entry"/>
    <updated>2025-01-03T12:00:00Z</updated>
  </entry>
</feed>"#;

        let articles = parse_feed(atom_with_newlines).unwrap();

        assert_eq!(articles[0].feed_title, "Sample Atom");
        assert_eq!(articles[0].title, "Atom Entry");
    }

    #[test]
    fn test_parse_rss10_with_dc_date() {
        let rss10 = r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF
 xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns="http://purl.org/rss/1.0/"
 xmlns:dc="http://purl.org/dc/elements/1.1/"
>
<channel>
<title>Sample RDF Feed</title>
<link>https://example.com</link>
</channel>
<item rdf:about="https://example.com/item1">
<title>RDF Item</title>
<link>https://example.com/item1</link>
<dc:date>2025-01-11T12:00:00Z</dc:date>
</item>
</rdf:RDF>"#;

        let articles = parse_feed(rss10).unwrap();

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "RDF Item");
        assert_eq!(articles[0].link, "https://example.com/item1");
        assert!(articles[0].published.is_some());
    }

    #[test]
    fn test_sanitize_unescaped_ampersands() {
        let input = "url?foo=1&bar=2&amp;baz=3";
        let expected = "url?foo=1&amp;bar=2&amp;baz=3";
        assert_eq!(sanitize_xml(input), expected);
    }

    #[test]
    fn test_sanitize_preserves_valid_entities() {
        let input = "&amp; &lt; &gt; &quot; &apos; &#38; &#x26;";
        assert_eq!(sanitize_xml(input), input);
    }

    #[test]
    fn test_parse_rss_with_unescaped_ampersands() {
        let rss_with_ampersands = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Test Item</title>
      <link>https://example.com?foo=1&amp;bar=2</link>
      <description>Image: https://example.com/img?w=100&h=200</description>
    </item>
  </channel>
</rss>"#;

        let articles = parse_feed(rss_with_ampersands).unwrap();

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Test Item");
    }
}
