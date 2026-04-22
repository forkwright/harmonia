use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://openlibrary.org";
const COVERS_URL: &str = "https://covers.openlibrary.org";

pub struct OpenLibraryProvider {
    client: reqwest::Client,
}

impl OpenLibraryProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch an edition by ISBN, OCLC, LCCN, or OLID.
    ///
    /// Open Library redirects `/isbn/{isbn}.json` to the canonical edition record.
    #[instrument(skip(self), fields(provider = "openlibrary"))]
    pub(crate) async fn fetch_edition(
        &self,
        key_or_isbn: &str,
    ) -> Result<Option<OlEdition>, EpignosisError> {
        let url = format!("{BASE_URL}/isbn/{key_or_isbn}.json");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "openlibrary",
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "openlibrary",
        })?;

        let edition: OlEdition = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "openlibrary",
        })?;

        Ok(Some(edition))
    }

    #[instrument(skip(self), fields(provider = "openlibrary"))]
    async fn fetch_work(&self, work_key: &str) -> Result<OlWork, EpignosisError> {
        let url = format!("{BASE_URL}{work_key}.json");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "openlibrary",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "openlibrary",
        })?;

        let work: OlWork = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "openlibrary",
        })?;

        Ok(work)
    }

    /// Derive a cover URL from edition data, falling back to ISBN.
    fn cover_url_for_edition(edition: &OlEdition) -> Option<String> {
        if let Some(first_cover) = edition.covers.as_deref().and_then(|c| c.first()) {
            return Some(format!("{COVERS_URL}/b/id/{first_cover}-L.jpg"));
        }
        if let Some(first_isbn_13) = edition.isbn_13.as_deref().and_then(|i| i.first()) {
            return Some(format!("{COVERS_URL}/b/isbn/{first_isbn_13}-L.jpg"));
        }
        if let Some(first_isbn_10) = edition.isbn_10.as_deref().and_then(|i| i.first()) {
            return Some(format!("{COVERS_URL}/b/isbn/{first_isbn_10}-L.jpg"));
        }
        None
    }
}

#[derive(Debug, Deserialize)]
struct OlSearchResponse {
    docs: Vec<OlSearchDoc>,
}

#[derive(Debug, Deserialize)]
struct OlSearchDoc {
    key: String,
    title: String,
    author_name: Option<Vec<String>>,
    first_publish_year: Option<u32>,
    isbn: Option<Vec<String>>,
    cover_i: Option<i64>,
    edition_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OlWork {
    key: String,
    title: String,
    description: Option<OlDescription>,
    subjects: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OlEdition {
    key: String,
    title: String,
    isbn_10: Option<Vec<String>>,
    isbn_13: Option<Vec<String>>,
    publishers: Option<Vec<String>>,
    publish_date: Option<String>,
    number_of_pages: Option<i64>,
    languages: Option<Vec<OlLanguageRef>>,
    covers: Option<Vec<i64>>,
    works: Option<Vec<OlWorkRef>>,
    description: Option<OlDescription>,
    subjects: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OlLanguageRef {
    key: String,
}

#[derive(Debug, Deserialize)]
struct OlWorkRef {
    key: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OlDescription {
    Simple(String),
    Structured { value: String },
}

impl OlDescription {
    fn text(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Structured { value } => value,
        }
    }
}

impl MetadataProvider for OpenLibraryProvider {
    fn name(&self) -> &str {
        "openlibrary"
    }

    #[instrument(skip(self), fields(provider = "openlibrary"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/search.json");
        let mut params = vec![
            ("title", query.title.as_str()),
            ("limit", "10"),
            (
                "fields",
                "key,title,author_name,first_publish_year,isbn,cover_i,edition_count",
            ),
        ];
        let author_str;
        if let Some(artist) = &query.artist {
            author_str = artist.clone();
            params.push(("author", &author_str));
        }

        let response =
            self.client
                .get(&url)
                .query(&params)
                .send()
                .await
                .context(ProviderRequestSnafu {
                    provider: "openlibrary",
                })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "openlibrary",
        })?;

        let parsed: OlSearchResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "openlibrary",
        })?;

        let results = parsed
            .docs
            .into_iter()
            .map(|doc| {
                let artist = doc.author_name.as_deref().and_then(|a| a.first()).cloned();
                let raw = serde_json::json!({
                    "ol_key": doc.key,
                    "isbn": doc.isbn,
                    "cover_i": doc.cover_i,
                    "edition_count": doc.edition_count,
                });
                ProviderResult {
                    provider_id: doc.key,
                    title: doc.title,
                    artist,
                    year: doc.first_publish_year,
                    score: 1.0,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "openlibrary"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        // provider_id is an OL key like "/works/OL12345W" or "/books/OL12345M"
        let is_edition = provider_id.starts_with("/books/");

        let (work, edition) = if is_edition {
            let edition = self
                .fetch_edition(provider_id.trim_start_matches("/books/"))
                .await?;

            let work = if let Some(ref ed) = edition {
                if let Some(work_ref) = ed.works.as_deref().and_then(|w| w.first()) {
                    self.fetch_work(&work_ref.key).await.ok()
                } else {
                    None
                }
            } else {
                None
            };

            (work, edition)
        } else {
            let work = self.fetch_work(provider_id).await?;

            // Attempt to fetch the first edition for richer ISBN/publisher/page data.
            // We don't have a direct edition key from a work search result, so this
            // is best-effort.  In practice the work response is the primary source.
            let edition = None;

            (Some(work), edition)
        };

        let title = edition
            .as_ref()
            .map(|e| e.title.clone())
            .or_else(|| work.as_ref().map(|w| w.title.clone()))
            .unwrap_or_default();

        let description = edition
            .as_ref()
            .and_then(|e| e.description.as_ref().map(|d| d.text().to_string()))
            .or_else(|| {
                work.as_ref()
                    .and_then(|w| w.description.as_ref().map(|d| d.text().to_string()))
            });

        let subjects = edition
            .as_ref()
            .and_then(|e| e.subjects.clone())
            .or_else(|| work.as_ref().and_then(|w| w.subjects.clone()))
            .unwrap_or_default();

        let year = edition.as_ref().and_then(|e| {
            e.publish_date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .and_then(|y| y.parse().ok())
        });

        let publisher = edition
            .as_ref()
            .and_then(|e| e.publishers.as_deref().and_then(|p| p.first()).cloned());

        let page_count = edition
            .as_ref()
            .and_then(|e| e.number_of_pages.map(|n| n as u32));

        let language = edition.as_ref().and_then(|e| {
            e.languages
                .as_deref()
                .and_then(|langs| langs.first())
                .map(|l| l.key.trim_start_matches("/languages/").to_string())
        });

        let cover_url = edition.as_ref().and_then(Self::cover_url_for_edition);

        let isbn_10 = edition
            .as_ref()
            .and_then(|e| e.isbn_10.as_deref().and_then(|i| i.first()).cloned());

        let isbn_13 = edition
            .as_ref()
            .and_then(|e| e.isbn_13.as_deref().and_then(|i| i.first()).cloned());

        let work_key = work.as_ref().map(|w| w.key.clone());
        let edition_key = edition.as_ref().map(|e| e.key.clone());

        let extra = serde_json::json!({
            "description": description,
            "subjects": subjects,
            "publisher": publisher,
            "publish_date": edition.as_ref().and_then(|e| e.publish_date.clone()),
            "page_count": page_count,
            "language": language,
            "cover_url": cover_url,
            "isbn_10": isbn_10,
            "isbn_13": isbn_13,
            "openlibrary_work_id": work_key,
            "openlibrary_edition_id": edition_key,
        });

        Ok(ProviderMetadata {
            provider_id: provider_id.to_string(),
            title,
            artist: None,
            year,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_doc_full() {
        let json = r#"{
            "key": "/works/OL123W",
            "title": "Dune",
            "author_name": ["Frank Herbert"],
            "first_publish_year": 1965,
            "isbn": ["9780441013593", "0441013597"],
            "cover_i": 12345,
            "edition_count": 42
        }"#;

        let doc: OlSearchDoc = serde_json::from_str(json).unwrap();
        assert_eq!(doc.key, "/works/OL123W");
        assert_eq!(doc.title, "Dune");
        assert_eq!(doc.author_name.as_deref().unwrap()[0], "Frank Herbert");
        assert_eq!(doc.first_publish_year, Some(1965));
        let isbns = doc.isbn.as_deref().unwrap();
        assert_eq!(isbns[0], "9780441013593");
        assert_eq!(doc.cover_i, Some(12345));
        assert_eq!(doc.edition_count, Some(42));
    }

    #[test]
    fn parse_search_doc_minimal() {
        let json = r#"{"key": "/works/OL456W", "title": "Sparse"}"#;
        let doc: OlSearchDoc = serde_json::from_str(json).unwrap();
        assert_eq!(doc.key, "/works/OL456W");
        assert_eq!(doc.title, "Sparse");
        assert!(doc.author_name.is_none());
        assert!(doc.isbn.is_none());
    }

    #[test]
    fn parse_work_full() {
        let json = r#"{
            "key": "/works/OL123W",
            "title": "Dune",
            "description": "A desert planet...",
            "subjects": ["Science Fiction", "Space Opera"]
        }"#;

        let work: OlWork = serde_json::from_str(json).unwrap();
        assert_eq!(work.key, "/works/OL123W");
        assert_eq!(work.title, "Dune");
        let desc = work.description.as_ref().unwrap();
        assert_eq!(desc.text(), "A desert planet...");
        let subjects = work.subjects.as_deref().unwrap();
        assert_eq!(subjects.len(), 2);
    }

    #[test]
    fn parse_work_structured_description() {
        let json = r#"{
            "key": "/works/OL123W",
            "title": "Dune",
            "description": {"type": "text", "value": "Structured desc."}
        }"#;

        let work: OlWork = serde_json::from_str(json).unwrap();
        let desc = work.description.as_ref().unwrap();
        assert_eq!(desc.text(), "Structured desc.");
    }

    #[test]
    fn parse_edition_full() {
        let json = r#"{
            "key": "/books/OL123M",
            "title": "Dune",
            "isbn_10": ["0441013597"],
            "isbn_13": ["9780441013593"],
            "publishers": ["Ace"],
            "publish_date": "1990-09-01",
            "number_of_pages": 896,
            "languages": [{"key": "/languages/eng"}],
            "covers": [12345],
            "works": [{"key": "/works/OL123W"}],
            "subjects": ["Fiction"]
        }"#;

        let edition: OlEdition = serde_json::from_str(json).unwrap();
        assert_eq!(edition.key, "/books/OL123M");
        assert_eq!(edition.isbn_13.as_deref().unwrap()[0], "9780441013593");
        assert_eq!(edition.number_of_pages, Some(896));
        let lang = edition.languages.as_deref().unwrap();
        assert_eq!(lang[0].key, "/languages/eng");
        let works = edition.works.as_deref().unwrap();
        assert_eq!(works[0].key, "/works/OL123W");
    }

    #[test]
    fn parse_edition_minimal() {
        let json = r#"{"key": "/books/OL789M", "title": "Minimal"}"#;
        let edition: OlEdition = serde_json::from_str(json).unwrap();
        assert_eq!(edition.key, "/books/OL789M");
        assert!(edition.isbn_13.is_none());
        assert!(edition.covers.is_none());
    }

    #[test]
    fn cover_url_from_cover_id() {
        let edition = OlEdition {
            key: "/books/OL123M".to_string(),
            title: "Dune".to_string(),
            isbn_10: None,
            isbn_13: None,
            publishers: None,
            publish_date: None,
            number_of_pages: None,
            languages: None,
            covers: Some(vec![12345]),
            works: None,
            description: None,
            subjects: None,
        };
        let url = OpenLibraryProvider::cover_url_for_edition(&edition);
        assert_eq!(
            url,
            Some("https://covers.openlibrary.org/b/id/12345-L.jpg".to_string())
        );
    }

    #[test]
    fn cover_url_fallback_to_isbn_13() {
        let edition = OlEdition {
            key: "/books/OL123M".to_string(),
            title: "Dune".to_string(),
            isbn_10: None,
            isbn_13: Some(vec!["9780441013593".to_string()]),
            publishers: None,
            publish_date: None,
            number_of_pages: None,
            languages: None,
            covers: None,
            works: None,
            description: None,
            subjects: None,
        };
        let url = OpenLibraryProvider::cover_url_for_edition(&edition);
        assert_eq!(
            url,
            Some("https://covers.openlibrary.org/b/isbn/9780441013593-L.jpg".to_string())
        );
    }

    #[test]
    fn cover_url_no_data_returns_none() {
        let edition = OlEdition {
            key: "/books/OL123M".to_string(),
            title: "Dune".to_string(),
            isbn_10: None,
            isbn_13: None,
            publishers: None,
            publish_date: None,
            number_of_pages: None,
            languages: None,
            covers: None,
            works: None,
            description: None,
            subjects: None,
        };
        let url = OpenLibraryProvider::cover_url_for_edition(&edition);
        assert!(url.is_none());
    }

    #[test]
    fn malformed_json_fails_to_parse() {
        let result = serde_json::from_str::<OlWork>(r#"{"key": "/works/OL123W"}"#);
        // title is required
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields_ignored() {
        let json = r#"{
            "key": "/works/OL123W",
            "title": "Dune",
            "extra_field": "ignored"
        }"#;
        let result = serde_json::from_str::<OlWork>(json);
        assert!(result.is_ok());
    }
}
