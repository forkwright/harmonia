use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://www.googleapis.com/books/v1";

pub struct GoogleBooksProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl GoogleBooksProvider {
    pub fn new(client: reqwest::Client, api_key: Option<String>) -> Self {
        Self { client, api_key }
    }
}

#[derive(Debug, Deserialize)]
struct GbSearchResponse {
    items: Option<Vec<GbVolume>>,
}

#[derive(Debug, Deserialize)]
struct GbVolume {
    id: String,
    #[serde(rename = "volumeInfo")]
    volume_info: GbVolumeInfo,
}

#[derive(Debug, Deserialize)]
struct GbVolumeInfo {
    title: String,
    authors: Option<Vec<String>>,
    publisher: Option<String>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
    description: Option<String>,
    #[serde(rename = "industryIdentifiers")]
    industry_identifiers: Option<Vec<GbIndustryIdentifier>>,
    #[serde(rename = "pageCount")]
    page_count: Option<i64>,
    categories: Option<Vec<String>>,
    language: Option<String>,
    #[serde(rename = "imageLinks")]
    image_links: Option<GbImageLinks>,
}

#[derive(Debug, Deserialize)]
struct GbIndustryIdentifier {
    #[serde(rename = "type")]
    type_: String,
    identifier: String,
}

#[derive(Debug, Deserialize)]
struct GbImageLinks {
    #[serde(rename = "smallThumbnail")]
    small_thumbnail: Option<String>,
    thumbnail: Option<String>,
}

impl MetadataProvider for GoogleBooksProvider {
    fn name(&self) -> &str {
        "google_books"
    }

    #[instrument(skip(self), fields(provider = "google_books"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let mut q_parts = Vec::new();

        if let Some(ref isbn) = query.isbn {
            q_parts.push(format!("isbn:{isbn}"));
        } else {
            q_parts.push(format!("intitle:{}", query.title));
            if let Some(ref artist) = query.artist {
                q_parts.push(format!("inauthor:{artist}"));
            }
        }

        let q = q_parts.join("+");
        let mut params = vec![("q", q.as_str()), ("maxResults", "10")];

        let key_str;
        if let Some(ref key) = self.api_key {
            key_str = key.clone();
            params.push(("key", &key_str));
        }

        let url = format!("{BASE_URL}/volumes");
        let response =
            self.client
                .get(&url)
                .query(&params)
                .send()
                .await
                .context(ProviderRequestSnafu {
                    provider: "google_books",
                })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "google_books",
        })?;

        let parsed: GbSearchResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "google_books",
        })?;

        let results = parsed
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                let artist = item
                    .volume_info
                    .authors
                    .as_deref()
                    .and_then(|a| a.first())
                    .cloned();

                let year = item
                    .volume_info
                    .published_date
                    .as_deref()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse().ok());

                let isbn_10 = item
                    .volume_info
                    .industry_identifiers
                    .as_ref()
                    .and_then(|ids| {
                        ids.iter()
                            .find(|id| id.type_ == "ISBN_10")
                            .map(|id| id.identifier.clone())
                    });

                let isbn_13 = item
                    .volume_info
                    .industry_identifiers
                    .as_ref()
                    .and_then(|ids| {
                        ids.iter()
                            .find(|id| id.type_ == "ISBN_13")
                            .map(|id| id.identifier.clone())
                    });

                let raw = serde_json::json!({
                    "google_books_id": item.id,
                    "isbn_10": isbn_10,
                    "isbn_13": isbn_13,
                });

                ProviderResult {
                    provider_id: item.id,
                    title: item.volume_info.title,
                    artist,
                    year,
                    score: 1.0,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "google_books"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let mut params: Vec<(&str, &str)> = Vec::new();
        let key_str;
        if let Some(ref key) = self.api_key {
            key_str = key.clone();
            params.push(("key", &key_str));
        }

        let url = format!("{BASE_URL}/volumes/{provider_id}");
        let response =
            self.client
                .get(&url)
                .query(&params)
                .send()
                .await
                .context(ProviderRequestSnafu {
                    provider: "google_books",
                })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "google_books",
        })?;

        let volume: GbVolume = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "google_books",
        })?;

        let info = volume.volume_info;

        let artist = info.authors.as_deref().and_then(|a| a.first()).cloned();

        let year = info
            .published_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let isbn_10 = info.industry_identifiers.as_ref().and_then(|ids| {
            ids.iter()
                .find(|id| id.type_ == "ISBN_10")
                .map(|id| id.identifier.clone())
        });

        let isbn_13 = info.industry_identifiers.as_ref().and_then(|ids| {
            ids.iter()
                .find(|id| id.type_ == "ISBN_13")
                .map(|id| id.identifier.clone())
        });

        let image_url = info.image_links.as_ref().and_then(|img| {
            img.thumbnail
                .clone()
                .or_else(|| img.small_thumbnail.clone())
        });

        // Only canonical fields — never cache raw Google Books JSON in sidecars.
        let extra = serde_json::json!({
            "description": info.description,
            "language": info.language,
            "page_count": info.page_count,
            "publisher": info.publisher,
            "published_date": info.published_date,
            "subjects": info.categories.unwrap_or_default(),
            "image_url": image_url,
            "isbn_10": isbn_10,
            "isbn_13": isbn_13,
            "google_books_id": volume.id,
        });

        Ok(ProviderMetadata {
            provider_id: volume.id,
            title: info.title,
            artist,
            year,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_volume_full_fields() {
        let json = r#"{
            "id": "abc123",
            "volumeInfo": {
                "title": "Dune",
                "subtitle": "Book One",
                "authors": ["Frank Herbert"],
                "publisher": "Ace",
                "publishedDate": "1990-09-01",
                "description": "A science fiction epic.",
                "industryIdentifiers": [
                    {"type": "ISBN_13", "identifier": "9780441013593"},
                    {"type": "ISBN_10", "identifier": "0441013597"}
                ],
                "pageCount": 896,
                "categories": ["Fiction", "Science Fiction"],
                "language": "en",
                "imageLinks": {
                    "smallThumbnail": "http://example.com/small.jpg",
                    "thumbnail": "http://example.com/thumb.jpg"
                }
            }
        }"#;

        let volume: GbVolume = serde_json::from_str(json).unwrap();
        assert_eq!(volume.id, "abc123");
        assert_eq!(volume.volume_info.title, "Dune");
        assert_eq!(volume.volume_info.publisher.as_deref(), Some("Ace"));
        let authors = volume.volume_info.authors.as_deref().unwrap();
        assert_eq!(authors[0], "Frank Herbert");
        assert_eq!(volume.volume_info.page_count, Some(896));
        assert_eq!(volume.volume_info.language.as_deref(), Some("en"));
        let ids = volume.volume_info.industry_identifiers.as_deref().unwrap();
        assert_eq!(ids[0].type_, "ISBN_13");
        assert_eq!(ids[0].identifier, "9780441013593");
        let img = volume.volume_info.image_links.as_ref().unwrap();
        assert_eq!(
            img.thumbnail.as_deref(),
            Some("http://example.com/thumb.jpg")
        );
    }

    #[test]
    fn parse_volume_minimal_fields() {
        let json = r#"{
            "id": "xyz789",
            "volumeInfo": {
                "title": "Unknown Book"
            }
        }"#;

        let volume: GbVolume = serde_json::from_str(json).unwrap();
        assert_eq!(volume.id, "xyz789");
        assert_eq!(volume.volume_info.title, "Unknown Book");
        assert!(volume.volume_info.authors.is_none());
        assert!(volume.volume_info.description.is_none());
        assert!(volume.volume_info.image_links.is_none());
    }

    #[test]
    fn parse_search_response_with_items() {
        let json = r#"{
            "items": [
                {
                    "id": "abc123",
                    "volumeInfo": {
                        "title": "Dune",
                        "authors": ["Frank Herbert"],
                        "publishedDate": "1990"
                    }
                }
            ]
        }"#;

        let resp: GbSearchResponse = serde_json::from_str(json).unwrap();
        let items = resp.items.as_deref().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "abc123");
    }

    #[test]
    fn parse_search_response_empty_items() {
        let json = r#"{"items": []}"#;
        let resp: GbSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.as_deref().unwrap().len(), 0);
    }

    #[test]
    fn parse_search_response_null_items() {
        let json = r#"{}"#;
        let resp: GbSearchResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_none());
    }

    #[test]
    fn malformed_volume_json_fails_to_parse() {
        let json = r#"{"id": "abc123"}"#; // missing volumeInfo
        let result = serde_json::from_str::<GbVolume>(json);
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields_are_ignored() {
        let json = r#"{
            "id": "abc123",
            "volumeInfo": {
                "title": "Future Book",
                "unknownField": "value"
            }
        }"#;
        let result = serde_json::from_str::<GbVolume>(json);
        assert!(result.is_ok());
    }

    #[test]
    fn isbn_extraction_from_identifiers() {
        let json = r#"{
            "id": "abc123",
            "volumeInfo": {
                "title": "Dune",
                "industryIdentifiers": [
                    {"type": "ISBN_13", "identifier": "9780441013593"},
                    {"type": "ISBN_10", "identifier": "0441013597"}
                ]
            }
        }"#;

        let volume: GbVolume = serde_json::from_str(json).unwrap();
        let isbn_13 = volume
            .volume_info
            .industry_identifiers
            .as_ref()
            .and_then(|ids| {
                ids.iter()
                    .find(|id| id.type_ == "ISBN_13")
                    .map(|id| id.identifier.clone())
            });
        let isbn_10 = volume
            .volume_info
            .industry_identifiers
            .as_ref()
            .and_then(|ids| {
                ids.iter()
                    .find(|id| id.type_ == "ISBN_10")
                    .map(|id| id.identifier.clone())
            });
        assert_eq!(isbn_13, Some("9780441013593".to_string()));
        assert_eq!(isbn_10, Some("0441013597".to_string()));
    }

    #[test]
    fn year_extracted_from_published_date() {
        let json = r#"{
            "id": "abc123",
            "volumeInfo": {
                "title": "Dune",
                "publishedDate": "1990-09-01"
            }
        }"#;

        let volume: GbVolume = serde_json::from_str(json).unwrap();
        let year: Option<u32> = volume
            .volume_info
            .published_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());
        assert_eq!(year, Some(1990));
    }
}
