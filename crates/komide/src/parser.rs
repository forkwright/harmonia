use snafu::ResultExt;

use crate::error::{FeedParseSnafu, KomideError};

pub struct NormalizedFeed {
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub image_url: Option<String>,
    pub entries: Vec<NormalizedEntry>,
}

pub struct NormalizedEntry {
    pub guid: String,
    pub title: String,
    pub published: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub enclosures: Vec<Enclosure>,
    pub link: Option<String>,
}

pub struct Enclosure {
    pub url: String,
    pub content_type: Option<String>,
    pub length: Option<u64>,
}

pub fn parse_feed(bytes: &[u8]) -> Result<NormalizedFeed, KomideError> {
    let feed = feed_rs::parser::parse(bytes).context(FeedParseSnafu)?;
    Ok(normalize(feed))
}

fn normalize(feed: feed_rs::model::Feed) -> NormalizedFeed {
    let title = feed.title.map(|t| t.content).unwrap_or_default();
    let description = feed.description.map(|d| d.content);
    let link = feed.links.into_iter().next().map(|l| l.href);
    let image_url = feed.logo.map(|img| img.uri);
    let entries = feed.entries.into_iter().map(normalize_entry).collect();

    NormalizedFeed {
        title,
        description,
        link,
        image_url,
        entries,
    }
}

fn normalize_entry(entry: feed_rs::model::Entry) -> NormalizedEntry {
    let guid = entry.id;
    let title = entry.title.map(|t| t.content).unwrap_or_default();
    let published = entry.published.map(|dt| dt.to_rfc3339());
    let summary = entry.summary.map(|s| s.content);
    let content = entry.content.and_then(|c| c.body);

    // Primary link: first non-enclosure link
    let link = entry
        .links
        .iter()
        .find(|l| l.rel.as_deref() != Some("enclosure"))
        .or(entry.links.first())
        .map(|l| l.href.clone());

    // Enclosures from <enclosure> elements (appear as links with rel="enclosure")
    let mut enclosures: Vec<Enclosure> = entry
        .links
        .iter()
        .filter(|l| l.rel.as_deref() == Some("enclosure"))
        .map(|l| Enclosure {
            url: l.href.clone(),
            content_type: l.media_type.clone(),
            length: l.length,
        })
        .collect();

    // Also collect from media objects (media:content elements in RSS)
    for media_obj in &entry.media {
        for mc in &media_obj.content {
            if let Some(url) = &mc.url {
                enclosures.push(Enclosure {
                    url: url.to_string(),
                    content_type: mc.content_type.as_ref().map(|m| m.to_string()),
                    length: mc.size,
                });
            }
        }
    }

    NormalizedEntry {
        guid,
        title,
        published,
        summary,
        content,
        enclosures,
        link,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS_PODCAST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <channel>
    <title>Test Podcast</title>
    <description>A test podcast feed</description>
    <link>https://example.com</link>
    <item>
      <title>Episode 1</title>
      <guid>ep-001</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <description>First episode</description>
      <enclosure url="https://example.com/ep1.mp3" type="audio/mpeg" length="12345678"/>
      <link>https://example.com/ep1</link>
    </item>
    <item>
      <title>Episode 2</title>
      <guid>ep-002</guid>
      <pubDate>Tue, 02 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="https://example.com/ep2.m4a" type="audio/mp4" length="9876543"/>
    </item>
  </channel>
</rss>"#;

    const ATOM_NEWS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test News</title>
  <subtitle>A test news feed</subtitle>
  <link href="https://news.example.com"/>
  <entry>
    <id>article-001</id>
    <title>Breaking News</title>
    <published>2024-01-01T12:00:00Z</published>
    <summary>Something important happened</summary>
    <link href="https://news.example.com/breaking"/>
  </entry>
  <entry>
    <id>article-002</id>
    <title>Follow Up</title>
    <published>2024-01-02T08:00:00Z</published>
    <summary>More details on the story</summary>
    <link href="https://news.example.com/followup"/>
  </entry>
</feed>"#;

    #[test]
    fn parse_rss_podcast_entries_with_enclosures() {
        let feed = parse_feed(RSS_PODCAST).expect("parse failed");
        assert_eq!(feed.title, "Test Podcast");
        assert_eq!(feed.entries.len(), 2);

        let ep1 = &feed.entries[0];
        assert_eq!(ep1.guid, "ep-001");
        assert_eq!(ep1.title, "Episode 1");
        assert!(
            !ep1.enclosures.is_empty(),
            "episode 1 should have an enclosure"
        );
        let enc = &ep1.enclosures[0];
        assert_eq!(enc.url, "https://example.com/ep1.mp3");
        assert_eq!(enc.content_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(enc.length, Some(12345678));

        let ep2 = &feed.entries[1];
        assert!(
            !ep2.enclosures.is_empty(),
            "episode 2 should have an enclosure"
        );
        assert_eq!(ep2.enclosures[0].content_type.as_deref(), Some("audio/mp4"));
    }

    #[test]
    fn parse_atom_news_entries() {
        let feed = parse_feed(ATOM_NEWS).expect("parse failed");
        assert_eq!(feed.title, "Test News");
        assert_eq!(feed.entries.len(), 2);

        let art1 = &feed.entries[0];
        assert_eq!(art1.guid, "article-001");
        assert_eq!(art1.title, "Breaking News");
        assert!(art1.summary.is_some());
        assert!(
            art1.enclosures.is_empty(),
            "news entries should have no enclosures"
        );
        assert_eq!(
            art1.link.as_deref(),
            Some("https://news.example.com/breaking")
        );
    }

    #[test]
    fn parse_invalid_feed_returns_error() {
        let result = parse_feed(b"this is not a feed");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_feed_title_defaults_to_empty_string() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><item><guid>x</guid></item></channel></rss>"#;
        let feed = parse_feed(xml).expect("parse failed");
        assert_eq!(feed.title, "");
    }
}
