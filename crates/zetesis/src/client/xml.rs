use serde::Deserialize;

use crate::types::{IndexerCaps, IndexerCategory, SearchFunction, SearchLimits, ServerInfo};

#[derive(Debug, Deserialize)]
pub struct TorznabFeed {
    pub channel: TorznabChannel,
}

#[derive(Debug, Deserialize)]
pub struct TorznabChannel {
    pub title: Option<String>,
    #[serde(rename = "item", default)]
    pub items: Vec<TorznabItem>,
}

#[derive(Debug, Deserialize)]
pub struct TorznabItem {
    pub title: String,
    pub guid: Option<String>,
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,
    pub size: Option<u64>,
    pub link: Option<String>,
    #[serde(rename = "attr", default)]
    pub attrs: Vec<TorznabAttr>,
}

#[derive(Debug, Deserialize)]
pub struct TorznabAttr {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}

pub(crate) fn get_attr<'a>(attrs: &'a [TorznabAttr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

pub fn get_attr_u64(attrs: &[TorznabAttr], name: &str) -> Option<u64> {
    get_attr(attrs, name)?.parse().ok()
}

pub(crate) fn get_attr_f64(attrs: &[TorznabAttr], name: &str) -> Option<f64> {
    get_attr(attrs, name)?.parse().ok()
}

pub(crate) fn get_attr_u32(attrs: &[TorznabAttr], name: &str) -> Option<u32> {
    get_attr(attrs, name)?.parse().ok()
}

// --- Caps XML parsing ---

#[derive(Debug, Deserialize)]
pub struct CapsRoot {
    pub server: Option<CapsServer>,
    pub limits: Option<CapsLimits>,
    pub searching: Option<CapsSearching>,
    pub categories: Option<CapsCategories>,
}

#[derive(Debug, Deserialize)]
pub struct CapsServer {
    #[serde(rename = "@title")]
    pub title: Option<String>,
    #[serde(rename = "@version")]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CapsLimits {
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@max")]
    pub max: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CapsSearching {
    pub search: Option<CapsSearchFunc>,
    #[serde(rename = "tv-search")]
    pub tv_search: Option<CapsSearchFunc>,
    #[serde(rename = "movie-search")]
    pub movie_search: Option<CapsSearchFunc>,
    #[serde(rename = "music-search")]
    pub music_search: Option<CapsSearchFunc>,
    #[serde(rename = "book-search")]
    pub book_search: Option<CapsSearchFunc>,
}

#[derive(Debug, Deserialize)]
pub struct CapsSearchFunc {
    #[serde(rename = "@available")]
    pub available: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CapsCategories {
    #[serde(rename = "category", default)]
    pub categories: Vec<CapsCategory>,
}

#[derive(Debug, Deserialize)]
pub struct CapsCategory {
    #[serde(rename = "@id")]
    pub id: Option<String>,
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "subcat", default)]
    pub subcategories: Vec<CapsCategory>,
}

impl CapsRoot {
    pub(crate) fn into_indexer_caps(self) -> IndexerCaps {
        let server = self.server.map_or_else(
            || ServerInfo {
                title: None,
                version: None,
            },
            |s| ServerInfo {
                title: s.title,
                version: s.version,
            },
        );

        let limits = self
            .limits
            .map_or_else(SearchLimits::default, |l| SearchLimits {
                default: l.default.and_then(|v| v.parse().ok()).unwrap_or(100),
                max: l.max.and_then(|v| v.parse().ok()).unwrap_or(100),
            });

        let search_functions = self
            .searching
            .map(|s| {
                let mut funcs = Vec::new();
                if let Some(f) = s.search {
                    funcs.push(SearchFunction {
                        function_type: "search".to_string(),
                        available: f.available.as_deref() == Some("yes"),
                    });
                }
                if let Some(f) = s.tv_search {
                    funcs.push(SearchFunction {
                        function_type: "tvsearch".to_string(),
                        available: f.available.as_deref() == Some("yes"),
                    });
                }
                if let Some(f) = s.movie_search {
                    funcs.push(SearchFunction {
                        function_type: "movie".to_string(),
                        available: f.available.as_deref() == Some("yes"),
                    });
                }
                if let Some(f) = s.music_search {
                    funcs.push(SearchFunction {
                        function_type: "music".to_string(),
                        available: f.available.as_deref() == Some("yes"),
                    });
                }
                if let Some(f) = s.book_search {
                    funcs.push(SearchFunction {
                        function_type: "book".to_string(),
                        available: f.available.as_deref() == Some("yes"),
                    });
                }
                funcs
            })
            .unwrap_or_default();

        let categories = self
            .categories
            .map(|c| c.categories.into_iter().map(convert_category).collect())
            .unwrap_or_default();

        IndexerCaps {
            server,
            limits,
            search_functions,
            categories,
        }
    }
}

fn convert_category(c: CapsCategory) -> IndexerCategory {
    IndexerCategory {
        id: c.id.and_then(|v| v.parse().ok()).unwrap_or(0),
        name: c.name.unwrap_or_default(),
        subcategories: c.subcategories.into_iter().map(convert_category).collect(),
    }
}

pub(crate) fn parse_feed_xml(xml: &str) -> Result<TorznabFeed, quick_xml::DeError> {
    quick_xml::de::from_str(xml)
}

pub(crate) fn parse_caps_xml(xml: &str) -> Result<IndexerCaps, quick_xml::DeError> {
    let caps_root: CapsRoot = quick_xml::de::from_str(xml)?;
    Ok(caps_root.into_indexer_caps())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_torznab_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Test Indexer</title>
    <item>
      <title>Test.Release.2024.FLAC</title>
      <guid>abc123</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <size>734003200</size>
      <link>https://example.com/download/abc123</link>
      <torznab:attr name="seeders" value="42"/>
      <torznab:attr name="leechers" value="5"/>
      <torznab:attr name="infohash" value="deadbeef1234567890abcdef1234567890abcdef"/>
      <torznab:attr name="category" value="3000"/>
      <torznab:attr name="downloadvolumefactor" value="0.0"/>
      <torznab:attr name="uploadvolumefactor" value="2.0"/>
    </item>
  </channel>
</rss>"#;

        let feed = parse_feed_xml(xml).unwrap();
        assert_eq!(feed.channel.title.as_deref(), Some("Test Indexer"));
        assert_eq!(feed.channel.items.len(), 1);

        let item = &feed.channel.items[0];
        assert_eq!(item.title, "Test.Release.2024.FLAC");
        assert_eq!(item.guid.as_deref(), Some("abc123"));
        assert_eq!(item.size, Some(734003200));

        assert_eq!(get_attr_u32(&item.attrs, "seeders"), Some(42));
        assert_eq!(get_attr_u32(&item.attrs, "leechers"), Some(5));
        assert_eq!(
            get_attr(&item.attrs, "infohash"),
            Some("deadbeef1234567890abcdef1234567890abcdef")
        );
        assert_eq!(get_attr_f64(&item.attrs, "downloadvolumefactor"), Some(0.0));
        assert_eq!(get_attr_f64(&item.attrs, "uploadvolumefactor"), Some(2.0));
    }

    #[test]
    fn parse_newznab_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/">
  <channel>
    <title>Usenet Indexer</title>
    <item>
      <title>Test.Release.2024.NZB</title>
      <guid>nzb-guid-456</guid>
      <size>524288000</size>
      <link>https://example.com/getnzb/nzb-guid-456</link>
      <newznab:attr name="category" value="2000"/>
      <newznab:attr name="grabs" value="150"/>
    </item>
  </channel>
</rss>"#;

        let feed = parse_feed_xml(xml).unwrap();
        assert_eq!(feed.channel.items.len(), 1);

        let item = &feed.channel.items[0];
        assert_eq!(item.title, "Test.Release.2024.NZB");
        assert_eq!(get_attr_u32(&item.attrs, "grabs"), Some(150));
        assert!(get_attr(&item.attrs, "infohash").is_none());
        assert!(get_attr(&item.attrs, "seeders").is_none());
    }

    #[test]
    fn parse_empty_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Empty</title>
  </channel>
</rss>"#;

        let feed = parse_feed_xml(xml).unwrap();
        assert!(feed.channel.items.is_empty());
    }

    #[test]
    fn parse_caps_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server title="Test Indexer" version="1.0"/>
  <limits default="100" max="500"/>
  <searching>
    <search available="yes"/>
    <tv-search available="yes"/>
    <movie-search available="yes"/>
    <music-search available="no"/>
    <book-search available="no"/>
  </searching>
  <categories>
    <category id="2000" name="Movies">
      <subcat id="2010" name="Movies/Foreign"/>
      <subcat id="2020" name="Movies/Other"/>
    </category>
    <category id="5000" name="TV">
      <subcat id="5010" name="TV/WEB-DL"/>
    </category>
  </categories>
</caps>"#;

        let caps = parse_caps_xml(xml).unwrap();
        assert_eq!(caps.server.title.as_deref(), Some("Test Indexer"));
        assert_eq!(caps.limits.default, 100);
        assert_eq!(caps.limits.max, 500);

        assert_eq!(caps.search_functions.len(), 5);
        let search = caps
            .search_functions
            .iter()
            .find(|f| f.function_type == "search")
            .unwrap();
        assert!(search.available);
        let music = caps
            .search_functions
            .iter()
            .find(|f| f.function_type == "music")
            .unwrap();
        assert!(!music.available);

        assert_eq!(caps.categories.len(), 2);
        assert_eq!(caps.categories[0].id, 2000);
        assert_eq!(caps.categories[0].name, "Movies");
        assert_eq!(caps.categories[0].subcategories.len(), 2);
    }

    #[test]
    fn parse_minimal_caps() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server title="Minimal"/>
</caps>"#;

        let caps = parse_caps_xml(xml).unwrap();
        assert_eq!(caps.server.title.as_deref(), Some("Minimal"));
        assert_eq!(caps.limits.default, 100);
        assert!(caps.search_functions.is_empty());
        assert!(caps.categories.is_empty());
    }

    #[test]
    fn attr_helpers() {
        let attrs = vec![
            TorznabAttr {
                name: "seeders".to_string(),
                value: "42".to_string(),
            },
            TorznabAttr {
                name: "size".to_string(),
                value: "1234567890".to_string(),
            },
            TorznabAttr {
                name: "ratio".to_string(),
                value: "1.5".to_string(),
            },
        ];

        assert_eq!(get_attr(&attrs, "seeders"), Some("42"));
        assert_eq!(get_attr(&attrs, "missing"), None);
        assert_eq!(get_attr_u64(&attrs, "size"), Some(1234567890));
        assert_eq!(get_attr_u64(&attrs, "seeders"), Some(42));
        assert_eq!(get_attr_f64(&attrs, "ratio"), Some(1.5));
        assert_eq!(get_attr_u32(&attrs, "seeders"), Some(42));
    }
}
