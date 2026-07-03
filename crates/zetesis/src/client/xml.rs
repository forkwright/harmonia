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

pub fn get_attr<'a>(attrs: &'a [TorznabAttr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

pub fn get_attr_u64(attrs: &[TorznabAttr], name: &str) -> Option<u64> {
    get_attr(attrs, name)?.parse().ok()
}

pub fn get_attr_f64(attrs: &[TorznabAttr], name: &str) -> Option<f64> {
    get_attr(attrs, name)?.parse().ok()
}

pub fn get_attr_u32(attrs: &[TorznabAttr], name: &str) -> Option<u32> {
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
    pub fn into_indexer_caps(self) -> IndexerCaps {
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
            .unwrap_or_default(); // WHY: Option chain — .map produces Option, not Result

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

// WHY: iterative post-order traversal — category XML is third-party data, and
// recursive conversion would let a hostile deeply-nested caps document
// overflow the stack.
fn convert_category(root: CapsCategory) -> IndexerCategory {
    struct Frame {
        id: u32,
        name: String,
        pending: std::vec::IntoIter<CapsCategory>,
        converted: Vec<IndexerCategory>,
    }

    fn open(c: CapsCategory) -> Frame {
        Frame {
            id: c.id.and_then(|v| v.parse().ok()).unwrap_or(0),
            name: c.name.unwrap_or_default(),
            pending: c.subcategories.into_iter(),
            converted: Vec::new(),
        }
    }

    let mut stack = vec![open(root)];
    loop {
        if let Some(child) = stack.last_mut().and_then(|f| f.pending.next()) {
            stack.push(open(child));
            continue;
        }
        let Some(finished) = stack.pop() else {
            // INVARIANT: unreachable — the root frame always exits via the
            // `None => return` arm below; kept total for lint.
            return IndexerCategory {
                id: 0,
                name: String::new(),
                subcategories: Vec::new(),
            };
        };
        let node = IndexerCategory {
            id: finished.id,
            name: finished.name,
            subcategories: finished.converted,
        };
        match stack.last_mut() {
            Some(parent) => parent.converted.push(node),
            None => return node,
        }
    }
}

pub fn parse_feed_xml(xml: &str) -> Result<TorznabFeed, quick_xml::DeError> {
    quick_xml::de::from_str(xml)
}

/// Maximum XML element nesting depth accepted in a caps document.
///
/// WHY: caps XML is third-party data whose `<category><subcat>...` nesting maps
/// 1:1 onto serde's recursive descent into the self-referential
/// [`CapsCategory`]. Beyond this depth the recursive deserialize overflows the
/// stack DURING parse — before the iterative post-parse [`convert_category`]
/// can run — so the ceiling is enforced on the raw event stream first. Real
/// Torznab caps nest one `<subcat>` level; 32 leaves generous headroom.
const MAX_CAPS_XML_DEPTH: usize = 32;

/// Rejects a caps document whose element nesting exceeds [`MAX_CAPS_XML_DEPTH`],
/// scanning the raw event stream so no stack-recursive deserialize is entered.
///
/// WHY: a malformed-but-shallow document is left for `from_str` to report with
/// its canonical error — this guard only fires on the deep-nesting DoS class.
fn reject_excessive_depth(xml: &str) -> Result<(), quick_xml::DeError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    let mut depth: usize = 0;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                depth += 1;
                if depth > MAX_CAPS_XML_DEPTH {
                    return Err(quick_xml::DeError::Custom(format!(
                        "caps XML nesting exceeds the maximum depth of {MAX_CAPS_XML_DEPTH}"
                    )));
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) => return Ok(()),
            // WHY: a malformed document is not this guard's concern — stop the
            // pre-scan and let from_str surface the authoritative parse error.
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
}

pub fn parse_caps_xml(xml: &str) -> Result<IndexerCaps, quick_xml::DeError> {
    reject_excessive_depth(xml)?;
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
    fn convert_category_survives_hostile_nesting_depth() {
        const DEPTH: usize = 100_000;
        let mut node = CapsCategory {
            id: Some("1".to_string()),
            name: Some("leaf".to_string()),
            subcategories: Vec::new(),
        };
        for i in 0..DEPTH {
            node = CapsCategory {
                id: Some(format!("{i}")),
                name: Some(format!("level-{i}")),
                subcategories: vec![node],
            };
        }

        let converted = convert_category(node);

        // NOTE: the assertion walk (and teardown) is iterative too — a
        // recursive walk or plain drop would re-introduce the overflow the
        // conversion just avoided.
        let mut count = 0usize;
        let mut work = vec![converted];
        while let Some(mut n) = work.pop() {
            count += 1;
            work.append(&mut n.subcategories);
        }
        assert_eq!(count, DEPTH + 1);
    }

    #[test]
    fn parse_caps_xml_rejects_hostile_nesting_depth() {
        // WHY: exercises the REAL parse path (from_str) — untrusted caps XML
        // with deeply-nested <subcat> would overflow the stack during serde's
        // recursive descent (before the iterative converter runs); the depth
        // pre-scan must reject it with a clean error, never crash the process.
        const DEPTH: usize = 50_000;
        let mut xml =
            String::from(r#"<?xml version="1.0"?><caps><categories><category id="1" name="root">"#);
        for i in 0..DEPTH {
            xml.push_str(&format!(r#"<subcat id="{i}" name="n">"#));
        }
        for _ in 0..DEPTH {
            xml.push_str("</subcat>");
        }
        xml.push_str("</category></categories></caps>");

        let result = parse_caps_xml(&xml);
        assert!(
            result.is_err(),
            "hostile nesting depth must be rejected, not parsed"
        );
    }

    #[test]
    fn convert_category_round_trips_shallow_tree() {
        let node = CapsCategory {
            id: Some("2000".to_string()),
            name: Some("Movies".to_string()),
            subcategories: vec![CapsCategory {
                id: Some("2010".to_string()),
                name: Some("Movies/Foreign".to_string()),
                subcategories: Vec::new(),
            }],
        };

        let converted = convert_category(node);
        assert_eq!(converted.id, 2000);
        assert_eq!(converted.name, "Movies");
        assert_eq!(converted.subcategories.len(), 1);
        assert_eq!(converted.subcategories[0].id, 2010);
        assert_eq!(converted.subcategories[0].name, "Movies/Foreign");
        assert!(converted.subcategories[0].subcategories.is_empty());
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
