use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use horismos::EpignosisConfig;
use themelion::MediaType;
use tracing::instrument;

use crate::MetadataResolver;
use crate::cache::MetadataCache;
use crate::error::EpignosisError;
use crate::identity::{
    EnrichedMetadata, FingerprintResult, MediaIdentity, ProviderEnrichment, UnidentifiedItem,
};
use crate::providers::acoustid::AcoustIdProvider;
use crate::providers::audnexus::AudnexusProvider;
use crate::providers::comicvine::ComicVineProvider;
use crate::providers::googlebooks::GoogleBooksProvider;
use crate::providers::itunes::ItunesProvider;
use crate::providers::musicbrainz::MusicBrainzProvider;
use crate::providers::openlibrary::OpenLibraryProvider;
use crate::providers::tmdb::TmdbProvider;
use crate::providers::tvdb::TvdbProvider;
use crate::providers::{MetadataProvider, ProviderResult, SearchQuery};
use crate::rate_limit::ProviderQueues;

/// Provider credentials supplied at construction time.
#[derive(Debug, Clone, Default)]
pub struct ProviderCredentials {
    pub acoustid_key: String,
    pub tmdb_key: String,
    pub tvdb_key: String,
    pub comicvine_key: String,
    pub google_books_key: Option<String>,
}

pub struct EpignosisService {
    #[expect(dead_code)]
    client: reqwest::Client,
    queues: Arc<ProviderQueues>,
    cache: Arc<MetadataCache<String, serde_json::Value>>,
    #[expect(dead_code)]
    config: EpignosisConfig,
    musicbrainz: MusicBrainzProvider,
    #[expect(dead_code)]
    acoustid: AcoustIdProvider,
    tmdb: TmdbProvider,
    tvdb: TvdbProvider,
    audnexus: AudnexusProvider,
    openlibrary: OpenLibraryProvider,
    google_books: GoogleBooksProvider,
    itunes: ItunesProvider,
    comicvine: ComicVineProvider,
}

impl EpignosisService {
    pub fn new(config: EpignosisConfig, credentials: ProviderCredentials) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.provider_timeout_secs))
            .build()
            .unwrap_or_default();

        let cache = Arc::new(MetadataCache::new(Duration::from_secs(
            config.cache_ttl_secs,
        )));
        let queues = Arc::new(ProviderQueues::new());

        let musicbrainz = MusicBrainzProvider::new(client.clone());
        let acoustid = AcoustIdProvider::new(client.clone(), credentials.acoustid_key.clone());
        let tmdb = TmdbProvider::new(client.clone(), credentials.tmdb_key.clone());
        let tvdb = TvdbProvider::new(client.clone(), credentials.tvdb_key.clone());
        let audnexus = AudnexusProvider::new(client.clone());
        let openlibrary = OpenLibraryProvider::new(client.clone());
        let google_books = GoogleBooksProvider::new(client.clone(), credentials.google_books_key);
        let itunes = ItunesProvider::new(client.clone());
        let comicvine = ComicVineProvider::new(client.clone(), credentials.comicvine_key.clone());

        Self {
            client,
            queues,
            cache,
            config,
            musicbrainz,
            acoustid,
            tmdb,
            tvdb,
            audnexus,
            openlibrary,
            google_books,
            itunes,
            comicvine,
        }
    }

    /// Returns the canonical provider name for a given media type.
    pub fn canonical_provider_for(media_type: MediaType) -> &'static str {
        match media_type {
            MediaType::Music => "musicbrainz",
            MediaType::Movie => "tmdb",
            MediaType::Tv => "tvdb",
            MediaType::Audiobook => "audnexus",
            MediaType::Book => "openlibrary",
            MediaType::Comic => "comicvine",
            MediaType::Podcast => "itunes",
            MediaType::News => "itunes",
            _ => "musicbrainz",
        }
    }

    fn build_query(item: &UnidentifiedItem) -> SearchQuery {
        let (title, artist, year, isbn) = if let Some(tags) = &item.tags {
            (
                tags.title
                    .clone()
                    .unwrap_or_else(|| item.filename_hint.clone().unwrap_or_default()),
                tags.artist.clone().or_else(|| tags.album_artist.clone()),
                tags.year,
                tags.isbn.clone(),
            )
        } else {
            (
                item.filename_hint.clone().unwrap_or_default(),
                None,
                None,
                None,
            )
        };

        SearchQuery {
            media_type: item.media_type,
            title,
            artist,
            year,
            isbn,
            extra: None,
        }
    }

    /// Book-aware scoring: ISBN exact > title+author+year > title-only.
    fn score_book_result(result: &ProviderResult, query: &SearchQuery) -> f64 {
        // ISBN exact match
        if let Some(ref query_isbn) = query.isbn {
            let raw_isbns = result
                .raw
                .get("isbn")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());
            if let Some(ref isbns) = raw_isbns
                && isbns.iter().any(|i| *i == query_isbn)
            {
                return 1.0;
            }

            let isbn_10 = result.raw.get("isbn_10").and_then(|v| v.as_str());
            let isbn_13 = result.raw.get("isbn_13").and_then(|v| v.as_str());
            if isbn_10 == Some(query_isbn) || isbn_13 == Some(query_isbn) {
                return 1.0;
            }
        }

        // title+author+year
        let title_match =
            !query.title.is_empty() && result.title.to_lowercase() == query.title.to_lowercase();
        let author_match = result.artist.as_ref().map(|a| a.to_lowercase())
            == query.artist.as_ref().map(|a| a.to_lowercase());
        let year_match = result.year == query.year;

        if title_match && author_match && year_match {
            return 0.8;
        }

        // title-only
        if title_match {
            return 0.4;
        }

        0.2
    }
}

impl MetadataResolver for EpignosisService {
    #[instrument(skip(self, item, ct), fields(media_type = ?item.media_type))]
    async fn resolve_identity(
        &self,
        item: &UnidentifiedItem,
        ct: tokio_util::sync::CancellationToken,
    ) -> Result<MediaIdentity, EpignosisError> {
        let cache_key = format!("identity:{}:{}", item.media_type, item.media_id);

        if let Some(cached) = self.cache.get(&cache_key)
            && let Ok(identity) = serde_json::from_value::<MediaIdentity>(cached)
        {
            return Ok(identity);
        }

        let query = Self::build_query(item);
        let provider_name = Self::canonical_provider_for(item.media_type);

        let results = tokio::select! {
            result = self.search_canonical(item.media_type, &query) => result?,
            _ = ct.cancelled() => {
                return Err(EpignosisError::IdentityNotResolved {
                    provider: provider_name.to_string(),
                    query: query.title.clone(),
                    location: snafu::location!(),
                });
            }
        };

        // For books, try Google Books fallback if canonical provider returned nothing.
        let results = if results.is_empty() && item.media_type == MediaType::Book {
            tokio::select! {
                result = self.search_google_books(&query) => result.unwrap_or_default(),
                _ = ct.cancelled() => {
                    return Err(EpignosisError::IdentityNotResolved {
                        provider: provider_name.to_string(),
                        query: query.title.clone(),
                        location: snafu::location!(),
                    });
                }
            }
        } else {
            results
        };

        let mut results = results;
        if item.media_type == MediaType::Book {
            for result in &mut results {
                result.score = Self::score_book_result(result, &query);
            }
        }

        let best = results
            .into_iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| EpignosisError::IdentityNotResolved {
                provider: provider_name.to_string(),
                query: query.title.clone(),
                location: snafu::location!(),
            })?;

        let identity = MediaIdentity {
            media_id: item.media_id,
            media_type: item.media_type,
            provider: provider_name.to_string(),
            provider_id: best.provider_id,
            canonical_title: best.title,
            canonical_artist: best.artist,
            year: best.year,
            extra: best.raw,
        };

        if let Ok(value) = serde_json::to_value(&identity) {
            self.cache.insert(cache_key, value);
        }

        Ok(identity)
    }

    #[instrument(skip(self, identity, ct), fields(provider = %identity.provider))]
    async fn enrich(
        &self,
        identity: &MediaIdentity,
        ct: tokio_util::sync::CancellationToken,
    ) -> Result<EnrichedMetadata, EpignosisError> {
        let mut enrichments = Vec::new();

        let primary_result = tokio::select! {
            result = self.enrich_from_canonical(identity) => result,
            _ = ct.cancelled() => return Ok(EnrichedMetadata {
                identity: identity.clone(),
                enrichments,
            }),
        };

        if let Ok(data) = primary_result {
            enrichments.push(ProviderEnrichment {
                provider: identity.provider.clone(),
                data,
            });
        }

        let secondary_result = tokio::select! {
            result = self.enrich_from_secondary(identity) => result,
            _ = ct.cancelled() => return Ok(EnrichedMetadata {
                identity: identity.clone(),
                enrichments,
            }),
        };

        if let Some((provider, data)) = secondary_result {
            enrichments.push(ProviderEnrichment { provider, data });
        }

        Ok(EnrichedMetadata {
            identity: identity.clone(),
            enrichments,
        })
    }

    #[instrument(skip(self, _ct), fields(path = %file_path.display()))]
    async fn fingerprint_audio(
        &self,
        file_path: &Path,
        _ct: tokio_util::sync::CancellationToken,
    ) -> Result<FingerprintResult, EpignosisError> {
        // Fingerprinting requires a native fpcalc binary (Chromaprint).
        // This delegates to an external process and returns the result.
        // The actual chromaprint invocation is deferred to the host process.
        Err(EpignosisError::FingerprintFailed {
            path: file_path.to_path_buf(),
            message: "fpcalc not available in this build".to_string(),
            location: snafu::location!(),
        })
    }
}

impl EpignosisService {
    async fn search_canonical(
        &self,
        media_type: MediaType,
        query: &SearchQuery,
    ) -> Result<Vec<crate::providers::ProviderResult>, EpignosisError> {
        match media_type {
            MediaType::Music => {
                self.queues.musicbrainz.acquire().await;
                self.musicbrainz.search(query).await
            }
            MediaType::Movie => {
                self.queues.tmdb.acquire().await;
                self.tmdb.search(query).await
            }
            MediaType::Tv => {
                self.queues.tvdb.acquire().await;
                self.tvdb.search(query).await
            }
            MediaType::Audiobook => {
                self.queues.audnexus.acquire().await;
                self.audnexus.search(query).await
            }
            MediaType::Book => {
                self.queues.openlibrary.acquire().await;
                self.openlibrary.search(query).await
            }
            MediaType::Comic => {
                self.queues.comicvine.acquire().await;
                self.comicvine.search(query).await
            }
            MediaType::Podcast | MediaType::News => {
                self.queues.itunes.acquire().await;
                self.itunes.search(query).await
            }
            _ => Ok(vec![]),
        }
    }

    async fn search_google_books(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<crate::providers::ProviderResult>, EpignosisError> {
        self.queues.google_books.acquire().await;
        self.google_books.search(query).await
    }

    async fn enrich_from_canonical(
        &self,
        identity: &MediaIdentity,
    ) -> Result<serde_json::Value, EpignosisError> {
        let metadata = match identity.media_type {
            MediaType::Music => {
                self.queues.musicbrainz.acquire().await;
                self.musicbrainz.get_metadata(&identity.provider_id).await?
            }
            MediaType::Movie => {
                self.queues.tmdb.acquire().await;
                self.tmdb.get_metadata(&identity.provider_id).await?
            }
            MediaType::Tv => {
                self.queues.tvdb.acquire().await;
                self.tvdb.get_metadata(&identity.provider_id).await?
            }
            MediaType::Audiobook => {
                self.queues.audnexus.acquire().await;
                self.audnexus.get_metadata(&identity.provider_id).await?
            }
            MediaType::Book => {
                self.queues.openlibrary.acquire().await;
                self.openlibrary.get_metadata(&identity.provider_id).await?
            }
            MediaType::Comic => {
                self.queues.comicvine.acquire().await;
                self.comicvine.get_metadata(&identity.provider_id).await?
            }
            MediaType::Podcast | MediaType::News => {
                self.queues.itunes.acquire().await;
                self.itunes.get_metadata(&identity.provider_id).await?
            }
            _ => return Ok(serde_json::Value::Null),
        };

        Ok(metadata.extra)
    }

    async fn enrich_from_secondary(
        &self,
        identity: &MediaIdentity,
    ) -> Option<(String, serde_json::Value)> {
        match identity.media_type {
            MediaType::Tv => {
                self.queues.tmdb.acquire().await;
                let meta = self.tmdb.get_metadata(&identity.provider_id).await.ok()?;
                Some(("tmdb".to_string(), meta.extra))
            }
            MediaType::Audiobook => {
                self.queues.openlibrary.acquire().await;
                let query = SearchQuery {
                    media_type: identity.media_type,
                    title: identity.canonical_title.clone(),
                    artist: identity.canonical_artist.clone(),
                    year: identity.year,
                    isbn: None,
                    extra: None,
                };
                let results = self.openlibrary.search(&query).await.ok()?;
                let best = results.into_iter().next()?;
                let meta = self
                    .openlibrary
                    .get_metadata(&best.provider_id)
                    .await
                    .ok()?;
                Some(("openlibrary".to_string(), meta.extra))
            }
            MediaType::Book => {
                self.queues.google_books.acquire().await;
                let meta = self
                    .google_books
                    .get_metadata(&identity.provider_id)
                    .await
                    .ok()?;
                Some(("google_books".to_string(), meta.extra))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_provider_music() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Music),
            "musicbrainz"
        );
    }

    #[test]
    fn canonical_provider_movie() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Movie),
            "tmdb"
        );
    }

    #[test]
    fn canonical_provider_tv() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Tv),
            "tvdb"
        );
    }

    #[test]
    fn canonical_provider_audiobook() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Audiobook),
            "audnexus"
        );
    }

    #[test]
    fn canonical_provider_book() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Book),
            "openlibrary"
        );
    }

    #[test]
    fn canonical_provider_comic() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Comic),
            "comicvine"
        );
    }

    #[test]
    fn canonical_provider_podcast() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::Podcast),
            "itunes"
        );
    }

    #[test]
    fn canonical_provider_news() {
        assert_eq!(
            EpignosisService::canonical_provider_for(MediaType::News),
            "itunes"
        );
    }

    #[test]
    fn book_score_isbn_exact_match() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: None,
            year: None,
            isbn: Some("9780441013593".to_string()),
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "/works/OL123W".to_string(),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({
                "isbn": ["9780441013593", "0441013597"],
            }),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn book_score_isbn_13_field_match() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: None,
            year: None,
            isbn: Some("9780441013593".to_string()),
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "abc123".to_string(),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({
                "isbn_13": "9780441013593",
            }),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn book_score_title_author_year_match() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            isbn: None,
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "/works/OL123W".to_string(),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn book_score_title_only_match() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: None,
            year: None,
            isbn: None,
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "/works/OL123W".to_string(),
            title: "Dune".to_string(),
            artist: Some("Different Author".to_string()),
            year: Some(2000),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn book_score_no_match() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: None,
            year: None,
            isbn: None,
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "/works/OL123W".to_string(),
            title: "Foundation".to_string(),
            artist: Some("Isaac Asimov".to_string()),
            year: Some(1951),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn book_score_empty_title_never_exact() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "".to_string(),
            artist: None,
            year: None,
            isbn: None,
            extra: None,
        };

        let result = ProviderResult {
            provider_id: "/works/OL123W".to_string(),
            title: "".to_string(),
            artist: None,
            year: None,
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!((EpignosisService::score_book_result(&result, &query) - 0.2).abs() < f64::EPSILON);
    }
}
