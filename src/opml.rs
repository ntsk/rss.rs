use crate::subscription::Feed;
use anyhow::{Context, Result};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::fs;
use std::io::Cursor;
use std::path::Path;

pub fn import(path: &Path) -> Result<Vec<Feed>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read OPML file: {:?}", path))?;
    parse_opml(&content)
}

pub fn export(path: &Path, feeds: &[Feed]) -> Result<()> {
    let content = generate_opml(feeds)?;
    fs::write(path, content).with_context(|| format!("Failed to write OPML file: {:?}", path))?;
    Ok(())
}

fn parse_opml(content: &str) -> Result<Vec<Feed>> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut feeds = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().as_ref() == b"outline" => {
                let mut url = None;
                let mut title = None;

                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"xmlUrl" => {
                            url = Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                        b"title" | b"text" if title.is_none() => {
                            title = Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                        _ => {}
                    }
                }

                if let Some(url) = url {
                    feeds.push(Feed { url, title });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e).context("Failed to parse OPML"),
            _ => {}
        }
    }

    Ok(feeds)
}

fn generate_opml(feeds: &[Feed]) -> Result<String> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut opml = BytesStart::new("opml");
    opml.push_attribute(("version", "2.0"));
    writer.write_event(Event::Start(opml))?;

    writer.write_event(Event::Start(BytesStart::new("head")))?;
    writer.write_event(Event::Start(BytesStart::new("title")))?;
    writer.write_event(Event::Text(BytesText::new("RSS Subscriptions")))?;
    writer.write_event(Event::End(BytesEnd::new("title")))?;
    writer.write_event(Event::End(BytesEnd::new("head")))?;

    writer.write_event(Event::Start(BytesStart::new("body")))?;

    for feed in feeds {
        let mut outline = BytesStart::new("outline");
        outline.push_attribute(("type", "rss"));
        outline.push_attribute(("xmlUrl", feed.url.as_str()));
        if let Some(title) = &feed.title {
            outline.push_attribute(("title", title.as_str()));
            outline.push_attribute(("text", title.as_str()));
        }
        writer.write_event(Event::Empty(outline))?;
    }

    writer.write_event(Event::End(BytesEnd::new("body")))?;
    writer.write_event(Event::End(BytesEnd::new("opml")))?;

    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OPML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Feeds</title></head>
  <body>
    <outline type="rss" xmlUrl="https://example.com/feed1.xml" title="Feed 1"/>
    <outline type="rss" xmlUrl="https://example.com/feed2.xml" text="Feed 2"/>
  </body>
</opml>"#;

    #[test]
    fn test_parse_opml_returns_feeds() {
        let feeds = parse_opml(SAMPLE_OPML).unwrap();

        assert_eq!(feeds.len(), 2);
    }

    #[test]
    fn test_parse_opml_extracts_url() {
        let feeds = parse_opml(SAMPLE_OPML).unwrap();

        assert_eq!(feeds[0].url, "https://example.com/feed1.xml");
        assert_eq!(feeds[1].url, "https://example.com/feed2.xml");
    }

    #[test]
    fn test_parse_opml_extracts_title() {
        let feeds = parse_opml(SAMPLE_OPML).unwrap();

        assert_eq!(feeds[0].title, Some("Feed 1".to_string()));
        assert_eq!(feeds[1].title, Some("Feed 2".to_string()));
    }

    #[test]
    fn test_generate_opml_creates_valid_xml() {
        let feeds = vec![Feed {
            url: "https://example.com/feed.xml".to_string(),
            title: Some("Test Feed".to_string()),
        }];

        let result = generate_opml(&feeds).unwrap();

        assert!(result.contains("xmlUrl=\"https://example.com/feed.xml\""));
        assert!(result.contains("title=\"Test Feed\""));
    }

    #[test]
    fn test_roundtrip() {
        let original = vec![
            Feed {
                url: "https://example.com/feed1.xml".to_string(),
                title: Some("Feed 1".to_string()),
            },
            Feed {
                url: "https://example.com/feed2.xml".to_string(),
                title: None,
            },
        ];

        let opml = generate_opml(&original).unwrap();
        let parsed = parse_opml(&opml).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].url, original[0].url);
        assert_eq!(parsed[0].title, original[0].title);
        assert_eq!(parsed[1].url, original[1].url);
    }
}
