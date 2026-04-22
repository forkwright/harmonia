use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://api.audnex.us";

pub struct AudnexusProvider {
    client: reqwest::Client,
}

impl AudnexusProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct AudnexusBook {
    asin: String,
    title: String,
    authors: Option<Vec<AudnexusAuthor>>,
    narrators: Option<Vec<AudnexusNarrator>>,
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    summary: Option<String>,
    genres: Option<Vec<AudnexusGenre>>,
    image: Option<String>,
    #[serde(rename = "seriesPrimary")]
    series_primary: Option<AudnexusSeries>,
    /// Total runtime in minutes as returned by the API.
    #[serde(rename = "runtimeLengthMin")]
    runtime_length_min: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AudnexusAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AudnexusNarrator {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AudnexusGenre {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AudnexusSeries {
    name: Option<String>,
    position: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusChaptersResponse {
    // Deserialization-only fields: exercised by parser round-trip tests below,
    // so the dead_code expect is gated on non-test builds.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parser round-trip tests use this field")
    )]
    asin: String,
    #[serde(rename = "brandIntroDurationMs")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parser round-trip tests use this field")
    )]
    brand_intro_duration_ms: Option<u64>,
    #[serde(rename = "brandOutroDurationMs")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parser round-trip tests use this field")
    )]
    brand_outro_duration_ms: Option<u64>,
    chapters: Option<Vec<AudnexusChapter>>,
}

#[derive(Debug, Deserialize)]
struct AudnexusChapter {
    title: Option<String>,
    #[serde(rename = "startOffsetMs")]
    start_offset_ms: Option<u64>,
    #[serde(rename = "startOffsetSec")]
    start_offset_sec: Option<u64>,
    #[serde(rename = "lengthMs")]
    length_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AudnexusSearchResponse {
    books: Option<Vec<AudnexusSearchResult>>,
}

#[derive(Debug, Deserialize)]
struct AudnexusSearchResult {
    asin: String,
    title: String,
    authors: Option<Vec<AudnexusAuthor>>,
}

impl AudnexusProvider {
    async fn fetch_chapters(
        &self,
        asin: &str,
    ) -> Result<Option<AudnexusChaptersResponse>, EpignosisError> {
        let url = format!("{BASE_URL}/chapters/{asin}");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "audnexus",
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "audnexus",
        })?;

        let chapters: AudnexusChaptersResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu {
                provider: "audnexus",
            })?;

        Ok(Some(chapters))
    }
}

impl MetadataProvider for AudnexusProvider {
    fn name(&self) -> &str {
        "audnexus"
    }

    #[instrument(skip(self), fields(provider = "audnexus"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/books");
        let response = self
            .client
            .get(&url)
            .query(&[("title", &query.title)])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "audnexus",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "audnexus",
        })?;

        let parsed: AudnexusSearchResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu {
                provider: "audnexus",
            })?;

        let results = parsed
            .books
            .unwrap_or_default()
            .into_iter()
            .map(|book| {
                let artist = book
                    .authors
                    .as_deref()
                    .and_then(|a| a.first())
                    .map(|a| a.name.clone());
                let raw = serde_json::json!({ "asin": book.asin });
                ProviderResult {
                    provider_id: book.asin,
                    title: book.title,
                    artist,
                    year: None,
                    score: 1.0,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "audnexus"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/books/{provider_id}");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "audnexus",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "audnexus",
        })?;

        let book: AudnexusBook = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "audnexus",
        })?;

        let artist = book
            .authors
            .as_deref()
            .and_then(|a| a.first())
            .map(|a| a.name.clone());

        let narrators: Vec<String> = book
            .narrators
            .unwrap_or_default()
            .into_iter()
            .map(|n| n.name)
            .collect();

        let year = book
            .release_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let genres: Vec<String> = book
            .genres
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.name)
            .collect();

        let series_name = book.series_primary.as_ref().and_then(|s| s.name.clone());
        let series_position = book
            .series_primary
            .as_ref()
            .and_then(|s| s.position.as_deref())
            .and_then(|p| p.parse::<f64>().ok());

        // Convert runtime from minutes to milliseconds for consistency with the
        // audiobook DB schema (duration_ms).
        let total_duration_ms = book.runtime_length_min.map(|m| u64::from(m) * 60 * 1000);

        // Fetch chapters — missing chapters (404) are non-fatal.
        let chapters_data = self.fetch_chapters(&book.asin).await.ok().flatten();

        let (chapter_count, chapters_json) = match chapters_data {
            Some(ref c) => {
                let count = c.chapters.as_ref().map(|ch| ch.len());
                let json = c.chapters.as_deref().map(|ch| {
                    ch.iter()
                        .map(|chapter| {
                            serde_json::json!({
                                "title": chapter.title,
                                "start_offset_ms": chapter.start_offset_ms
                                    .or_else(|| chapter.start_offset_sec.map(|s| s * 1000)),
                                "length_ms": chapter.length_ms,
                            })
                        })
                        .collect::<Vec<_>>()
                });
                (count, json)
            }
            None => (None, None),
        };

        let extra = serde_json::json!({
            "summary": book.summary,
            "image": book.image,
            "genres": genres,
            "narrators": narrators,
            "series_name": series_name,
            "series_position": series_position,
            "total_duration_ms": total_duration_ms,
            "chapter_count": chapter_count,
            "chapters": chapters_json,
        });

        Ok(ProviderMetadata {
            provider_id: book.asin,
            title: book.title,
            artist,
            year,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Struct parsing ────────────────────────────────────────────────────────

    #[test]
    fn parse_book_full_fields() {
        let json = r#"{
            "asin": "B002V1CBBG",
            "title": "Dune",
            "authors": [{"name": "Frank Herbert"}],
            "narrators": [{"name": "Scott Brick"}, {"name": "Orlagh Cassidy"}],
            "releaseDate": "2007-09-05",
            "summary": "A science fiction epic.",
            "genres": [{"name": "Science Fiction"}, {"name": "Classic"}],
            "image": "https://example.com/cover.jpg",
            "seriesPrimary": {"name": "Dune Chronicles", "position": "1"},
            "runtimeLengthMin": 1260
        }"#;

        let book: AudnexusBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.asin, "B002V1CBBG");
        assert_eq!(book.title, "Dune");
        let authors = book.authors.as_deref().unwrap();
        assert_eq!(authors[0].name, "Frank Herbert");
        let narrators = book.narrators.as_deref().unwrap();
        assert_eq!(narrators.len(), 2);
        assert_eq!(narrators[0].name, "Scott Brick");
        assert_eq!(narrators[1].name, "Orlagh Cassidy");
        assert_eq!(book.release_date.as_deref(), Some("2007-09-05"));
        let series = book.series_primary.as_ref().unwrap();
        assert_eq!(series.name.as_deref(), Some("Dune Chronicles"));
        assert_eq!(series.position.as_deref(), Some("1"));
        assert_eq!(book.runtime_length_min, Some(1260));
    }

    #[test]
    fn parse_book_minimal_fields() {
        let json = r#"{"asin": "B001234567", "title": "Unknown Book"}"#;
        let book: AudnexusBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.asin, "B001234567");
        assert_eq!(book.title, "Unknown Book");
        assert!(book.authors.is_none());
        assert!(book.narrators.is_none());
        assert!(book.series_primary.is_none());
        assert!(book.runtime_length_min.is_none());
    }

    #[test]
    fn parse_book_null_optional_fields() {
        let json = r#"{
            "asin": "B000000001",
            "title": "Sparse Book",
            "authors": null,
            "narrators": null,
            "releaseDate": null,
            "summary": null,
            "genres": null,
            "image": null,
            "seriesPrimary": null,
            "runtimeLengthMin": null
        }"#;
        let book: AudnexusBook = serde_json::from_str(json).unwrap();
        assert!(book.narrators.is_none());
        assert!(book.series_primary.is_none());
        assert!(book.runtime_length_min.is_none());
    }

    #[test]
    fn parse_chapters_response_full() {
        let json = r#"{
            "asin": "B002V1CBBG",
            "brandIntroDurationMs": 2000,
            "brandOutroDurationMs": 1500,
            "chapters": [
                {
                    "title": "Part 1: Arrakis",
                    "startOffsetMs": 0,
                    "startOffsetSec": 0,
                    "lengthMs": 3600000
                },
                {
                    "title": "Part 2: Muad'Dib",
                    "startOffsetMs": 3600000,
                    "startOffsetSec": 3600,
                    "lengthMs": 5400000
                }
            ]
        }"#;

        let resp: AudnexusChaptersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.asin, "B002V1CBBG");
        assert_eq!(resp.brand_intro_duration_ms, Some(2000));
        assert_eq!(resp.brand_outro_duration_ms, Some(1500));
        let chapters = resp.chapters.as_deref().unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title.as_deref(), Some("Part 1: Arrakis"));
        assert_eq!(chapters[0].start_offset_ms, Some(0));
        assert_eq!(chapters[0].length_ms, Some(3600000));
        assert_eq!(chapters[1].start_offset_ms, Some(3600000));
    }

    #[test]
    fn parse_chapters_response_missing_chapters() {
        let json = r#"{"asin": "B001234567"}"#;
        let resp: AudnexusChaptersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.asin, "B001234567");
        assert!(resp.chapters.is_none());
    }

    #[test]
    fn parse_search_response_with_books() {
        let json = r#"{
            "books": [
                {"asin": "B002V1CBBG", "title": "Dune", "authors": [{"name": "Frank Herbert"}]},
                {"asin": "B000ABCDEF", "title": "Dune Messiah", "authors": [{"name": "Frank Herbert"}]}
            ]
        }"#;

        let resp: AudnexusSearchResponse = serde_json::from_str(json).unwrap();
        let books = resp.books.as_deref().unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].asin, "B002V1CBBG");
        assert_eq!(books[1].title, "Dune Messiah");
    }

    #[test]
    fn parse_search_response_empty_books() {
        let json = r#"{"books": []}"#;
        let resp: AudnexusSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.books.as_deref().unwrap().len(), 0);
    }

    #[test]
    fn parse_search_response_null_books() {
        let json = r#"{"books": null}"#;
        let resp: AudnexusSearchResponse = serde_json::from_str(json).unwrap();
        assert!(resp.books.is_none());
    }

    // ── Field derivation ──────────────────────────────────────────────────────

    #[test]
    fn year_extracted_from_release_date() {
        let json = r#"{
            "asin": "B000000001",
            "title": "T",
            "releaseDate": "2019-11-05T00:00:00Z"
        }"#;
        let book: AudnexusBook = serde_json::from_str(json).unwrap();
        let year: Option<u32> = book
            .release_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());
        assert_eq!(year, Some(2019));
    }

    #[test]
    fn runtime_minutes_converts_to_milliseconds() {
        let runtime_min: u32 = 1260; // 21 hours
        let duration_ms = u64::from(runtime_min) * 60 * 1000;
        assert_eq!(duration_ms, 75_600_000);
    }

    #[test]
    fn series_position_parsed_as_float() {
        let series = AudnexusSeries {
            name: Some("Dune Chronicles".to_string()),
            position: Some("1".to_string()),
        };
        let pos: Option<f64> = series.position.as_deref().and_then(|p| p.parse().ok());
        assert_eq!(pos, Some(1.0));
    }

    #[test]
    fn series_position_fractional() {
        let series = AudnexusSeries {
            name: Some("Some Series".to_string()),
            position: Some("2.5".to_string()),
        };
        let pos: Option<f64> = series.position.as_deref().and_then(|p| p.parse().ok());
        assert_eq!(pos, Some(2.5));
    }

    #[test]
    fn narrators_collected_from_book() {
        let json = r#"{
            "asin": "B002V1CBBG",
            "title": "Dune",
            "narrators": [{"name": "Scott Brick"}, {"name": "Orlagh Cassidy"}]
        }"#;
        let book: AudnexusBook = serde_json::from_str(json).unwrap();
        let names: Vec<String> = book
            .narrators
            .unwrap_or_default()
            .into_iter()
            .map(|n| n.name)
            .collect();
        assert_eq!(names, vec!["Scott Brick", "Orlagh Cassidy"]);
    }

    #[test]
    fn chapter_count_from_chapters_response() {
        let json = r#"{
            "asin": "B002V1CBBG",
            "chapters": [
                {"title": "Ch 1", "startOffsetMs": 0, "lengthMs": 1000},
                {"title": "Ch 2", "startOffsetMs": 1000, "lengthMs": 2000},
                {"title": "Ch 3", "startOffsetMs": 3000, "lengthMs": 1500}
            ]
        }"#;
        let resp: AudnexusChaptersResponse = serde_json::from_str(json).unwrap();
        let count = resp.chapters.as_ref().map(|ch| ch.len());
        assert_eq!(count, Some(3));
    }

    #[test]
    fn start_offset_fallback_from_seconds() {
        // When startOffsetMs is absent, fall back to startOffsetSec * 1000.
        let json = r#"{
            "asin": "B000000001",
            "chapters": [
                {"title": "Ch 1", "startOffsetSec": 120, "lengthMs": 60000}
            ]
        }"#;
        let resp: AudnexusChaptersResponse = serde_json::from_str(json).unwrap();
        let ch = &resp.chapters.as_deref().unwrap()[0];
        let start_ms = ch
            .start_offset_ms
            .or_else(|| ch.start_offset_sec.map(|s| s * 1000));
        assert_eq!(start_ms, Some(120_000));
    }

    // ── Error handling (400/404/malformed) ────────────────────────────────────

    #[test]
    fn malformed_book_json_fails_to_parse() {
        let json = r#"{"asin": "B000000001"}"#; // missing required "title"
        let result = serde_json::from_str::<AudnexusBook>(json);
        assert!(result.is_err());
    }

    #[test]
    fn malformed_chapters_json_fails_to_parse() {
        let result = serde_json::from_str::<AudnexusChaptersResponse>("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields_in_book_are_ignored() {
        // Forward-compat: unknown fields from the API should not cause parse failures.
        let json = r#"{
            "asin": "B000000001",
            "title": "Future Book",
            "unknownFieldFromApi": "some value",
            "anotherNewField": 42
        }"#;
        let result = serde_json::from_str::<AudnexusBook>(json);
        assert!(result.is_ok());
    }
}
