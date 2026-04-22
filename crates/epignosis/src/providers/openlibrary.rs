use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://openlibrary.org";

pub struct OpenLibraryProvider {
    client: reqwest::Client,
}

impl OpenLibraryProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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
}

#[derive(Debug, Deserialize)]
struct OlWork {
    key: String,
    title: String,
    description: Option<OlDescription>,
    subjects: Option<Vec<String>>,
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
        let mut params = vec![("title", query.title.as_str()), ("limit", "10")];
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
                let raw = serde_json::json!({ "ol_key": doc.key });
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
        // provider_id is an OL key like "/works/OL12345W"
        let url = format!("{BASE_URL}{provider_id}.json");
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

        let description = work.description.as_ref().map(|d| d.text().to_string());
        let extra = serde_json::json!({
            "description": description,
            "subjects": work.subjects.unwrap_or_default(),
        });

        Ok(ProviderMetadata {
            provider_id: work.key,
            title: work.title,
            artist: None,
            year: None,
            extra,
        })
    }
}
