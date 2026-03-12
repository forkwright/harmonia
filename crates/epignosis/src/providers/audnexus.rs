use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

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
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    summary: Option<String>,
    genres: Option<Vec<AudnexusGenre>>,
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AudnexusGenre {
    name: String,
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

        let extra = serde_json::json!({
            "summary": book.summary,
            "image": book.image,
            "genres": genres,
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
