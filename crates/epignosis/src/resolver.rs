use std::path::Path;
use std::sync::{Arc, Weak};
use std::time::Duration;

use aggelmata::MediaType;
use horismos::EpignosisConfig;
use tracing::{Instrument, instrument};

use crate::MetadataResolver;
use crate::cache::MetadataCache;
use crate::error::EpignosisError;
use crate::identity::{
    EnrichedMetadata, FingerprintMatchStatus, FingerprintResult, MediaIdentity, ProviderEnrichment,
    UnidentifiedItem,
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
// WHY: pure data — authentication credentials for a metadata provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderCredentials {
    pub acoustid_key: String,
    /// TMDB API Read Access Token (v4); sent as an Authorization Bearer
    /// header, never as a URL query parameter.
    pub tmdb_key: String,
    pub tvdb_key: String,
    pub comicvine_key: String,
    pub google_books_key: Option<String>,
}

impl From<&EpignosisConfig> for ProviderCredentials {
    /// WHY `unwrap_or_default()` for the four `String`-typed keys: an absent
    /// config key must reproduce today's anonymous behavior (empty string —
    /// each provider already runs unauthenticated/rate-limited on an empty
    /// key, never rejects it), so wiring credentials in is behavior-
    /// preserving for an operator who sets none. `google_books_key` stays
    /// `Option` — `GoogleBooksProvider::new` already treats `None` as
    /// keyless.
    fn from(config: &EpignosisConfig) -> Self {
        Self {
            acoustid_key: config.acoustid_key.clone().unwrap_or_default(),
            tmdb_key: config.tmdb_key.clone().unwrap_or_default(),
            tvdb_key: config.tvdb_key.clone().unwrap_or_default(),
            comicvine_key: config.comicvine_key.clone().unwrap_or_default(),
            google_books_key: config.google_books_key.clone(),
        }
    }
}

pub struct ProviderBackedResolver {
    queues: Arc<ProviderQueues>,
    cache: Arc<MetadataCache<String, serde_json::Value>>,
    config: EpignosisConfig,
    musicbrainz: MusicBrainzProvider,
    acoustid: AcoustIdProvider,
    tmdb: TmdbProvider,
    tvdb: TvdbProvider,
    audnexus: AudnexusProvider,
    openlibrary: OpenLibraryProvider,
    google_books: GoogleBooksProvider,
    itunes: ItunesProvider,
    comicvine: ComicVineProvider,
    // NOTE: binary name rather than a hardcoded call site so tests can point
    // fingerprinting at a stub executable.
    fpcalc_binary: String,
}

impl ProviderBackedResolver {
    pub fn new(config: EpignosisConfig, credentials: ProviderCredentials) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.provider_timeout_secs))
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config (not applicable here)

        let cache_ttl = Duration::from_secs(config.cache_ttl_secs);
        let cache = Arc::new(MetadataCache::new(cache_ttl));
        // WHY floor: a zero or sub-second configured TTL would busy-loop the sweeper.
        let sweep_interval = cache_ttl.max(Duration::from_secs(1));
        Self::spawn_cache_eviction_sweeper(Arc::downgrade(&cache), sweep_interval);
        let queues = Arc::new(ProviderQueues::new());

        let mut musicbrainz = MusicBrainzProvider::new(client.clone());
        let mut acoustid = AcoustIdProvider::new(client.clone(), credentials.acoustid_key.clone());
        let mut tmdb = TmdbProvider::new(client.clone(), credentials.tmdb_key.clone());
        let mut tvdb = TvdbProvider::new(client.clone(), credentials.tvdb_key.clone());
        let mut audnexus = AudnexusProvider::new(client.clone());
        let mut openlibrary = OpenLibraryProvider::new(client.clone());
        let mut google_books =
            GoogleBooksProvider::new(client.clone(), credentials.google_books_key);
        let mut itunes = ItunesProvider::new(client.clone());
        let mut comicvine =
            ComicVineProvider::new(client.clone(), credentials.comicvine_key.clone());

        let body_cap = config.provider_response_max_bytes;
        musicbrainz.max_body_bytes = body_cap;
        acoustid.max_body_bytes = body_cap;
        tmdb.max_body_bytes = body_cap;
        tvdb.max_body_bytes = body_cap;
        audnexus.max_body_bytes = body_cap;
        openlibrary.max_body_bytes = body_cap;
        google_books.max_body_bytes = body_cap;
        itunes.max_body_bytes = body_cap;
        comicvine.max_body_bytes = body_cap;

        Self {
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
            fpcalc_binary: crate::fingerprint::FPCALC_BINARY.to_string(),
        }
    }

    /// Periodically evicts expired identity-cache entries every `interval`.
    ///
    /// The cache is otherwise only swept lazily, on a `get()` for the SAME
    /// key after its TTL — for a long-running server, identities are rarely
    /// re-queried, so without this the DashMap grows unbounded.
    ///
    /// WHY Weak: the sweeper holds no strong reference to the cache, so it
    /// exits cleanly (cancel-safe, no shutdown-token wiring needed) once the
    /// resolver's `Arc<MetadataCache>` is dropped.
    fn spawn_cache_eviction_sweeper(
        cache: Weak<MetadataCache<String, serde_json::Value>>,
        interval: Duration,
    ) {
        tokio::spawn(
            async move {
                let mut ticker = tokio::time::interval(interval);
                ticker.tick().await; // WHY: the first tick fires immediately; skip it so eviction starts after one full interval.
                loop {
                    ticker.tick().await;
                    let Some(cache) = cache.upgrade() else {
                        break;
                    };
                    cache.evict_expired();
                }
            }
            .instrument(tracing::info_span!("metadata_cache_eviction_sweeper")),
        );
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
        // WHY: both-None must not count as an author match — an
        // author-unknown result would otherwise claim the author-match tier.
        let author_match = matches!(
            (result.artist.as_deref(), query.artist.as_deref()),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b)
        );
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

    fn book_isbn(identity: &MediaIdentity) -> Option<String> {
        identity
            .extra
            .get("isbn_13")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                identity
                    .extra
                    .get("isbn_10")
                    .and_then(serde_json::Value::as_str)
            })
            .or_else(|| {
                identity
                    .extra
                    .get("isbn")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|isbns| isbns.iter().find_map(serde_json::Value::as_str))
            })
            .map(str::to_string)
    }

    fn book_query(identity: &MediaIdentity) -> SearchQuery {
        SearchQuery {
            media_type: MediaType::Book,
            title: identity.canonical_title.clone(),
            artist: identity.canonical_artist.clone(),
            year: identity.year,
            isbn: Self::book_isbn(identity),
            extra: None,
        }
    }

    /// Folds AcoustID lookup matches into a computed fingerprint, classified
    /// against `self.config`'s fingerprint thresholds (#575): the
    /// best-scoring match's score decides whether the match is `Accepted`
    /// (>= `fingerprint_accept_threshold`), `Ambiguous` (>=
    /// `fingerprint_ambiguous_threshold` but below accept — a held
    /// candidate, never auto-applied), or `NoMatch` (below ambiguous, or no
    /// matches at all) — in which case match identifiers are dropped rather
    /// than surfaced as a false positive.
    fn merge_lookup_matches(
        &self,
        computed: FingerprintResult,
        matches: &[ProviderResult],
    ) -> FingerprintResult {
        let best = matches.iter().max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let confidence = best.map_or(0.0, |m| m.score);
        let match_status = if confidence >= self.config.fingerprint_accept_threshold {
            FingerprintMatchStatus::Accepted
        } else if confidence >= self.config.fingerprint_ambiguous_threshold {
            FingerprintMatchStatus::Ambiguous
        } else {
            FingerprintMatchStatus::NoMatch
        };

        if match_status == FingerprintMatchStatus::NoMatch {
            return FingerprintResult {
                acoustid_id: None,
                confidence,
                mb_recording_ids: Vec::new(),
                match_status,
                ..computed
            };
        }

        FingerprintResult {
            acoustid_id: best.and_then(|m| {
                m.raw
                    .get("acoustid")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }),
            confidence,
            mb_recording_ids: matches.iter().map(|m| m.provider_id.0.clone()).collect(),
            match_status,
            ..computed
        }
    }
}

impl MetadataResolver for ProviderBackedResolver {
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

        let results = tokio::select! {
            result = self.search_canonical(item.media_type, &query) => result?,
            _ = ct.cancelled() => {
                return Err(EpignosisError::IdentityNotResolved {
                    provider: Self::canonical_provider_for(item.media_type).to_string(),
                    query: query.title.clone(),
                    location: snafu::location!(),
                });
            }
        };

        // For books, try Google Books fallback if canonical provider returned nothing.
        let results = if results.is_empty() && item.media_type == MediaType::Book {
            tokio::select! {
                result = self.search_google_books(&query) => result.unwrap_or_else(|e| { tracing::warn!(error = %e, "google books fallback failed"); vec![] }),
                _ = ct.cancelled() => {
                    return Err(EpignosisError::IdentityNotResolved {
                        provider: "google_books".to_string(),
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
                provider: Self::canonical_provider_for(item.media_type).to_string(),
                query: query.title.clone(),
                location: snafu::location!(),
            })?;

        let ProviderResult {
            provider,
            provider_id,
            title,
            artist,
            year,
            raw,
            ..
        } = best;

        let identity = MediaIdentity {
            media_id: item.media_id,
            media_type: item.media_type,
            provider,
            provider_id,
            canonical_title: title,
            canonical_artist: artist,
            year,
            extra: raw,
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

        match primary_result {
            Ok(data) => {
                enrichments.push(ProviderEnrichment {
                    provider: identity.provider.clone(),
                    data,
                });
            }
            // WHY: the failure is downgraded to a partial-success return
            // here, so this is the handled site — log it before dropping it.
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    provider = %identity.provider,
                    "canonical enrichment failed"
                );
            }
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

    #[instrument(skip(self, ct), fields(path = %file_path.display()))]
    async fn fingerprint_audio(
        &self,
        file_path: &Path,
        ct: tokio_util::sync::CancellationToken,
    ) -> Result<FingerprintResult, EpignosisError> {
        let raw = crate::fingerprint::compute(&self.fpcalc_binary, file_path, &ct).await?;
        let computed = FingerprintResult {
            fingerprint: raw.fingerprint,
            duration_secs: raw.duration,
            acoustid_id: None,
            mb_recording_ids: Vec::new(),
            confidence: 0.0,
            match_status: FingerprintMatchStatus::NoMatch,
        };

        self.queues.acoustid.acquire().await;
        let matches = tokio::select! {
            matches = self.acoustid.lookup_fingerprint(&computed) => matches?,
            // WHY: the fingerprint itself is already computed; cancellation
            // mid-lookup returns it unidentified rather than discarding it.
            _ = ct.cancelled() => return Ok(computed),
        };

        Ok(self.merge_lookup_matches(computed, &matches))
    }
}

impl ProviderBackedResolver {
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
                self.musicbrainz
                    .get_metadata(&identity.provider_id.0)
                    .await?
            }
            MediaType::Movie => {
                self.queues.tmdb.acquire().await;
                self.tmdb.get_metadata(&identity.provider_id.0).await?
            }
            MediaType::Tv => {
                self.queues.tvdb.acquire().await;
                self.tvdb.get_metadata(&identity.provider_id.0).await?
            }
            MediaType::Audiobook => {
                self.queues.audnexus.acquire().await;
                self.audnexus.get_metadata(&identity.provider_id.0).await?
            }
            MediaType::Book if identity.provider == "openlibrary" => {
                self.queues.openlibrary.acquire().await;
                self.openlibrary
                    .get_metadata(&identity.provider_id.0)
                    .await?
            }
            MediaType::Book if identity.provider == "google_books" => {
                self.queues.google_books.acquire().await;
                self.google_books
                    .get_metadata(&identity.provider_id.0)
                    .await?
            }
            MediaType::Comic => {
                self.queues.comicvine.acquire().await;
                self.comicvine.get_metadata(&identity.provider_id.0).await?
            }
            MediaType::Podcast | MediaType::News => {
                self.queues.itunes.acquire().await;
                self.itunes.get_metadata(&identity.provider_id.0).await?
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
                // WHY: identity.provider_id for Tv is a TVDB series id; TMDB
                // requires its own TV id, so cross-reference via /find first
                // and skip enrichment when TMDB holds no mapping.
                self.queues.tmdb.acquire().await;
                let tmdb_tv_id = self
                    .tmdb
                    .find_by_external_id(&identity.provider_id.0, "tvdb_id")
                    .await
                    .ok()
                    .flatten()?;
                self.queues.tmdb.acquire().await;
                let meta = self.tmdb.get_tv_metadata(&tmdb_tv_id.0).await.ok()?;
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
                    .get_metadata(&best.provider_id.0)
                    .await
                    .ok()?;
                Some(("openlibrary".to_string(), meta.extra))
            }
            // WHY: book IDs are provider-local. Obtain an ID owned by the
            // secondary provider through a fresh ISBN/title search before
            // requesting detail from that provider.
            MediaType::Book if identity.provider == "openlibrary" => {
                let query = Self::book_query(identity);
                self.queues.google_books.acquire().await;
                let result = self
                    .google_books
                    .search(&query)
                    .await
                    .ok()?
                    .into_iter()
                    .next()?;
                self.queues.google_books.acquire().await;
                let meta = self
                    .google_books
                    .get_metadata(&result.provider_id.0)
                    .await
                    .ok()?;
                Some(("google_books".to_string(), meta.extra))
            }
            MediaType::Book if identity.provider == "google_books" => {
                let query = Self::book_query(identity);
                self.queues.openlibrary.acquire().await;
                let result = self
                    .openlibrary
                    .search(&query)
                    .await
                    .ok()?
                    .into_iter()
                    .next()?;
                self.queues.openlibrary.acquire().await;
                let meta = self
                    .openlibrary
                    .get_metadata(&result.provider_id.0)
                    .await
                    .ok()?;
                Some(("openlibrary".to_string(), meta.extra))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use aggelmata::MediaId;

    use super::*;
    use crate::identity::MetadataProviderId;
    use crate::test_support::spawn_sequential_http;

    // WHY: the book cross-reference tests drive the same fixture record through two
    // different provider paths, so the ids have to agree across both. Holding one copy
    // here keeps a later edit from changing the id in one test and silently weakening
    // the namespace assertion in the other.
    const OPEN_LIBRARY_ID: &str = "/works/OL-X-W";
    const GOOGLE_BOOKS_ID: &str = "GB-Y";
    const ISBN: &str = "9780441013593";

    fn tv_identity(tvdb_id: &str) -> MediaIdentity {
        MediaIdentity {
            media_id: MediaId::new(),
            media_type: MediaType::Tv,
            provider: "tvdb".to_string(),
            provider_id: MetadataProviderId(tvdb_id.to_string()),
            canonical_title: "Breaking Bad".to_string(),
            canonical_artist: None,
            year: Some(2008),
            extra: serde_json::Value::Null,
        }
    }

    fn book_item(title: &str, author: &str) -> UnidentifiedItem {
        UnidentifiedItem {
            media_id: MediaId::new(),
            media_type: MediaType::Book,
            file_path: std::path::PathBuf::from("/library/book.epub"),
            filename_hint: Some(title.to_string()),
            tags: Some(crate::identity::EmbeddedTags {
                title: Some(title.to_string()),
                artist: Some(author.to_string()),
                year: Some(1965),
                ..Default::default()
            }),
        }
    }

    fn book_identity(provider: &str, provider_id: &str, extra: serde_json::Value) -> MediaIdentity {
        MediaIdentity {
            media_id: MediaId::new(),
            media_type: MediaType::Book,
            provider: provider.to_string(),
            provider_id: MetadataProviderId(provider_id.to_string()),
            canonical_title: "Dune".to_string(),
            canonical_artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            extra,
        }
    }

    fn test_resolver() -> ProviderBackedResolver {
        ProviderBackedResolver::new(
            horismos::EpignosisConfig::default(),
            ProviderCredentials::default(),
        )
    }

    // ── #578: EpignosisConfig -> ProviderCredentials mapping ───────────────

    #[test]
    fn provider_credentials_from_config_maps_present_keys() {
        let config = horismos::EpignosisConfig {
            acoustid_key: Some("acoustid-secret".to_string()),
            tmdb_key: Some("tmdb-secret".to_string()),
            tvdb_key: Some("tvdb-secret".to_string()),
            comicvine_key: Some("comicvine-secret".to_string()),
            google_books_key: Some("google-books-secret".to_string()),
            ..horismos::EpignosisConfig::default()
        };

        let credentials = ProviderCredentials::from(&config);

        assert_eq!(credentials.acoustid_key, "acoustid-secret");
        assert_eq!(credentials.tmdb_key, "tmdb-secret");
        assert_eq!(credentials.tvdb_key, "tvdb-secret");
        assert_eq!(credentials.comicvine_key, "comicvine-secret");
        assert_eq!(
            credentials.google_books_key.as_deref(),
            Some("google-books-secret")
        );
    }

    #[test]
    fn provider_credentials_from_config_absent_keys_are_behavior_preserving() {
        // WHY: absent keys must reproduce today's anonymous behavior — an
        // empty string for the four String-typed fields (each provider
        // already tolerates that as keyless/rate-limited, never rejects it)
        // and `None` for google_books_key (GoogleBooksProvider's existing
        // keyless path).
        let config = horismos::EpignosisConfig::default();

        let credentials = ProviderCredentials::from(&config);

        assert_eq!(credentials.acoustid_key, "");
        assert_eq!(credentials.tmdb_key, "");
        assert_eq!(credentials.tvdb_key, "");
        assert_eq!(credentials.comicvine_key, "");
        assert_eq!(credentials.google_books_key, None);
    }

    #[tokio::test]
    async fn enrich_from_secondary_tv_resolves_via_tmdb_find_cross_reference() {
        let find_body = serde_json::json!({
            "movie_results": [],
            "tv_results": [{ "id": 1396, "name": "Breaking Bad" }],
        })
        .to_string();
        let tv_body = serde_json::json!({
            "id": 1396,
            "name": "Breaking Bad",
            "first_air_date": "2008-01-20",
            "overview": "A chemistry teacher turns to crime.",
            "number_of_seasons": 5,
            "genres": [{ "name": "Drama" }],
        })
        .to_string();
        let (base_url, handle) =
            spawn_sequential_http(vec![(200, find_body), (200, tv_body)]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let enrichment = resolver.enrich_from_secondary(&tv_identity("81189")).await;

        let (provider, extra) = enrichment.expect("cross-referenced enrichment must succeed");
        assert_eq!(provider, "tmdb");
        assert_eq!(extra["overview"], "A chemistry teacher turns to crime.");
        assert_eq!(extra["seasons"], 5);

        let requests = handle.await.unwrap();
        assert!(
            requests[0].starts_with("GET /find/81189"),
            "the TVDB id must go through /find, never straight into a TMDB detail endpoint"
        );
        assert!(requests[0].contains("external_source=tvdb_id"));
        assert!(
            requests[1].starts_with("GET /tv/1396"),
            "detail fetch must use the cross-referenced TMDB TV id"
        );
    }

    #[tokio::test]
    async fn enrich_from_secondary_tv_without_mapping_returns_none() {
        let find_body = serde_json::json!({ "movie_results": [], "tv_results": [] }).to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, find_body)]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let enrichment = resolver.enrich_from_secondary(&tv_identity("999999")).await;

        assert!(
            enrichment.is_none(),
            "no cross-reference means no enrichment, not a wrong-namespace fetch"
        );
        let requests = handle.await.unwrap();
        assert_eq!(
            requests.len(),
            1,
            "no detail request may follow a find miss"
        );
    }

    #[tokio::test]
    async fn book_fallback_keeps_provider_ids_in_their_namespaces() {
        let openlibrary_search_miss = serde_json::json!({ "docs": [] }).to_string();
        let openlibrary_search_hit = serde_json::json!({
            "docs": [{
                "key": OPEN_LIBRARY_ID,
                "title": "Dune",
                "author_name": ["Frank Herbert"],
                "first_publish_year": 1965,
                "isbn": [ISBN],
            }],
        })
        .to_string();
        let openlibrary_detail = serde_json::json!({
            "key": OPEN_LIBRARY_ID,
            "title": "Dune",
            "description": "Open Library metadata",
        })
        .to_string();
        let (openlibrary_base_url, openlibrary_handle) = spawn_sequential_http(vec![
            (200, openlibrary_search_miss),
            (200, openlibrary_search_hit),
            (200, openlibrary_detail),
        ])
        .await;

        let google_books_search_hit = serde_json::json!({
            "items": [{
                "id": GOOGLE_BOOKS_ID,
                "volumeInfo": {
                    "title": "Dune",
                    "authors": ["Frank Herbert"],
                    "publishedDate": "1965",
                    "industryIdentifiers": [
                        { "type": "ISBN_13", "identifier": ISBN },
                    ],
                },
            }],
        })
        .to_string();
        let google_books_detail = serde_json::json!({
            "id": GOOGLE_BOOKS_ID,
            "volumeInfo": {
                "title": "Dune",
                "authors": ["Frank Herbert"],
                "publishedDate": "1965",
                "description": "Google Books metadata",
                "industryIdentifiers": [
                    { "type": "ISBN_13", "identifier": ISBN },
                ],
            },
        })
        .to_string();
        let (google_books_base_url, google_books_handle) = spawn_sequential_http(vec![
            (200, google_books_search_hit),
            (200, google_books_detail),
        ])
        .await;

        let mut resolver = test_resolver();
        resolver.openlibrary =
            OpenLibraryProvider::with_base_url(reqwest::Client::new(), openlibrary_base_url);
        resolver.google_books =
            GoogleBooksProvider::with_base_url(reqwest::Client::new(), None, google_books_base_url);

        let identity = resolver
            .resolve_identity(
                &book_item("Dune", "Frank Herbert"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(identity.provider, "google_books");
        assert_eq!(identity.provider_id.0, GOOGLE_BOOKS_ID);

        let enriched = resolver
            .enrich(&identity, tokio_util::sync::CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(enriched.enrichments.len(), 2);
        assert_eq!(enriched.enrichments[0].provider, "google_books");
        assert_eq!(enriched.enrichments[1].provider, "openlibrary");

        let openlibrary_requests = openlibrary_handle.await.unwrap();
        assert_eq!(openlibrary_requests.len(), 3);
        assert!(openlibrary_requests[0].starts_with("GET /search.json?"));
        assert!(openlibrary_requests[0].contains("title=Dune"));
        assert!(openlibrary_requests[1].starts_with("GET /search.json?"));
        assert!(openlibrary_requests[1].contains("isbn=9780441013593"));
        assert!(openlibrary_requests[2].starts_with("GET /works/OL-X-W.json"));
        assert!(
            openlibrary_requests
                .iter()
                .all(|request| !request.contains(GOOGLE_BOOKS_ID)),
            "Open Library must never receive the opaque Google Books id"
        );

        let google_books_requests = google_books_handle.await.unwrap();
        assert_eq!(google_books_requests.len(), 2);
        assert!(google_books_requests[0].starts_with("GET /volumes?"));
        assert!(google_books_requests[1].starts_with(&format!("GET /volumes/{GOOGLE_BOOKS_ID}")));
        // NOTE: the bare substring is deliberate and must not be replaced by
        // OPEN_LIBRARY_ID. The const carries the `/works/` prefix, so matching on it
        // would pass on a request that leaked the bare id with the prefix stripped —
        // exactly the leak this assertion exists to catch.
        assert!(
            google_books_requests
                .iter()
                .all(|request| !request.contains("OL-X-W")),
            "Google Books must never receive the Open Library id"
        );
    }

    #[tokio::test]
    async fn openlibrary_book_uses_isbn_to_find_google_books_id() {
        let openlibrary_detail = serde_json::json!({
            "key": OPEN_LIBRARY_ID,
            "title": "Dune",
            "description": "Open Library metadata",
        })
        .to_string();
        let (openlibrary_base_url, openlibrary_handle) =
            spawn_sequential_http(vec![(200, openlibrary_detail)]).await;

        let google_books_search_hit = serde_json::json!({
            "items": [{
                "id": GOOGLE_BOOKS_ID,
                "volumeInfo": {
                    "title": "Dune",
                    "authors": ["Frank Herbert"],
                    "industryIdentifiers": [
                        { "type": "ISBN_13", "identifier": ISBN },
                    ],
                },
            }],
        })
        .to_string();
        let google_books_detail = serde_json::json!({
            "id": GOOGLE_BOOKS_ID,
            "volumeInfo": {
                "title": "Dune",
                "authors": ["Frank Herbert"],
                "description": "Google Books metadata",
            },
        })
        .to_string();
        let (google_books_base_url, google_books_handle) = spawn_sequential_http(vec![
            (200, google_books_search_hit),
            (200, google_books_detail),
        ])
        .await;

        let mut resolver = test_resolver();
        resolver.openlibrary =
            OpenLibraryProvider::with_base_url(reqwest::Client::new(), openlibrary_base_url);
        resolver.google_books =
            GoogleBooksProvider::with_base_url(reqwest::Client::new(), None, google_books_base_url);
        let identity = book_identity(
            "openlibrary",
            OPEN_LIBRARY_ID,
            serde_json::json!({ "isbn": [ISBN] }),
        );

        let enriched = resolver
            .enrich(&identity, tokio_util::sync::CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(enriched.enrichments.len(), 2);
        assert_eq!(enriched.enrichments[0].provider, "openlibrary");
        assert_eq!(enriched.enrichments[1].provider, "google_books");

        let openlibrary_requests = openlibrary_handle.await.unwrap();
        assert_eq!(openlibrary_requests.len(), 1);
        assert!(openlibrary_requests[0].starts_with(&format!("GET {OPEN_LIBRARY_ID}.json")));
        assert!(!openlibrary_requests[0].contains(GOOGLE_BOOKS_ID));

        let google_books_requests = google_books_handle.await.unwrap();
        assert_eq!(google_books_requests.len(), 2);
        assert!(google_books_requests[0].starts_with("GET /volumes?"));
        assert!(google_books_requests[0].contains(&format!("isbn%3A{ISBN}")));
        assert!(google_books_requests[1].starts_with(&format!("GET /volumes/{GOOGLE_BOOKS_ID}")));
        // NOTE: see the matching assertion in
        // `book_fallback_keeps_provider_ids_in_their_namespaces` — the bare substring is
        // deliberately looser than OPEN_LIBRARY_ID and must stay that way.
        assert!(
            google_books_requests
                .iter()
                .all(|request| !request.contains("OL-X-W")),
            "Google Books must use its search result id, never the Open Library id"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fingerprint_audio_wires_fpcalc_into_acoustid_lookup() {
        let lookup_body = serde_json::json!({
            "results": [{
                "id": "acoustid-123",
                "score": 0.93,
                "recordings": [{
                    "id": "mb-rec-1",
                    "title": "Song",
                    "artists": [{ "name": "Artist" }],
                }],
            }],
        })
        .to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, lookup_body)]).await;

        let dir = tempfile::tempdir().unwrap();
        let stub = crate::test_support::write_stub_script(
            dir.path(),
            "fpcalc-stub",
            "#!/bin/sh\nprintf '{\"duration\": 42.5, \"fingerprint\": \"AQFAKE\"}'\n",
        );

        let mut resolver = test_resolver();
        resolver.acoustid =
            AcoustIdProvider::with_base_url(reqwest::Client::new(), "key", base_url);
        resolver.fpcalc_binary = stub.to_str().unwrap().to_string();

        let result = resolver
            .fingerprint_audio(
                Path::new("/audio/track.flac"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(result.fingerprint, "AQFAKE");
        assert!((result.duration_secs - 42.5).abs() < f64::EPSILON);
        assert_eq!(result.acoustid_id.as_deref(), Some("acoustid-123"));
        assert_eq!(result.mb_recording_ids, vec!["mb-rec-1".to_string()]);
        assert!((result.confidence - 0.93).abs() < f64::EPSILON);

        let requests = handle.await.unwrap();
        assert!(
            requests[0].contains("fingerprint=AQFAKE"),
            "the computed fingerprint must reach the lookup call"
        );
        assert!(requests[0].contains("duration=42"));
    }

    #[tokio::test]
    async fn fingerprint_audio_missing_fpcalc_is_a_clean_error() {
        let mut resolver = test_resolver();
        resolver.fpcalc_binary = "fpcalc-test-binary-that-does-not-exist".to_string();

        let err = resolver
            .fingerprint_audio(
                Path::new("/audio/track.flac"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap_err();

        match err {
            EpignosisError::FingerprintFailed { message, .. } => {
                assert!(message.contains("not found on PATH"), "{message}");
            }
            other => panic!("expected FingerprintFailed, got {other:?}"),
        }
    }

    fn fp_computed() -> FingerprintResult {
        FingerprintResult {
            fingerprint: "AQ".to_string(),
            duration_secs: 10.0,
            acoustid_id: None,
            mb_recording_ids: Vec::new(),
            confidence: 0.0,
            match_status: FingerprintMatchStatus::NoMatch,
        }
    }

    fn score_match(id: &str, acoustid: &str, score: f64) -> ProviderResult {
        ProviderResult {
            provider: "acoustid".to_string(),
            provider_id: MetadataProviderId(id.to_string()),
            title: id.to_string(),
            artist: None,
            year: None,
            score,
            raw: serde_json::json!({ "acoustid": acoustid }),
        }
    }

    // WHY tokio::test: ProviderBackedResolver::new spawns the cache-eviction
    // sweeper via tokio::spawn — constructing one (even just to call the sync
    // merge_lookup_matches method) requires a running reactor.
    #[tokio::test]
    async fn merge_lookup_matches_picks_best_score() {
        let matches = vec![
            score_match("mb-low", "acoustid-low", 0.4),
            score_match("mb-high", "acoustid-high", 0.9),
        ];

        let merged = test_resolver().merge_lookup_matches(fp_computed(), &matches);

        assert_eq!(merged.acoustid_id.as_deref(), Some("acoustid-high"));
        assert!((merged.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(
            merged.mb_recording_ids,
            vec!["mb-low".to_string(), "mb-high".to_string()]
        );
        assert_eq!(merged.fingerprint, "AQ");
        assert_eq!(merged.match_status, FingerprintMatchStatus::Accepted);
    }

    #[tokio::test]
    async fn merge_lookup_matches_empty_is_unidentified() {
        let merged = test_resolver().merge_lookup_matches(fp_computed(), &[]);

        assert_eq!(merged.acoustid_id, None);
        assert!(merged.mb_recording_ids.is_empty());
        assert!(merged.confidence.abs() < f64::EPSILON);
        assert_eq!(
            merged.fingerprint, "AQ",
            "the raw fingerprint survives a no-match lookup"
        );
        assert_eq!(merged.match_status, FingerprintMatchStatus::NoMatch);
    }

    // ── #575: fingerprint match classification ────────────────────────────

    #[tokio::test]
    async fn merge_lookup_matches_accept_threshold_score_is_accepted() {
        // WHY 0.8: default EpignosisConfig::fingerprint_accept_threshold.
        let matches = vec![score_match("mb-1", "acoustid-1", 0.8)];

        let merged = test_resolver().merge_lookup_matches(fp_computed(), &matches);

        assert_eq!(merged.match_status, FingerprintMatchStatus::Accepted);
        assert_eq!(merged.acoustid_id.as_deref(), Some("acoustid-1"));
        assert_eq!(merged.mb_recording_ids, vec!["mb-1".to_string()]);
    }

    #[tokio::test]
    async fn merge_lookup_matches_ambiguous_score_is_held_not_applied() {
        // WHY 0.6: between the default ambiguous (0.5) and accept (0.8)
        // thresholds — a real candidate, but not confident enough to apply
        // automatically.
        let matches = vec![score_match("mb-1", "acoustid-1", 0.6)];

        let merged = test_resolver().merge_lookup_matches(fp_computed(), &matches);

        assert_eq!(merged.match_status, FingerprintMatchStatus::Ambiguous);
        assert!(
            (merged.confidence - 0.6).abs() < f64::EPSILON,
            "an ambiguous match is still returned as a candidate, not silently blanked"
        );
        assert_eq!(
            merged.acoustid_id.as_deref(),
            Some("acoustid-1"),
            "the candidate identity must be present for the caller to hold/confirm"
        );
    }

    #[tokio::test]
    async fn merge_lookup_matches_below_ambiguous_threshold_is_dropped() {
        // WHY 0.1: below the default ambiguous threshold (0.5) — this is the
        // #575 bug scenario, a low-confidence garbage match.
        let matches = vec![score_match("mb-garbage", "acoustid-garbage", 0.1)];

        let merged = test_resolver().merge_lookup_matches(fp_computed(), &matches);

        assert_eq!(merged.match_status, FingerprintMatchStatus::NoMatch);
        assert_eq!(
            merged.acoustid_id, None,
            "a below-threshold match must never surface an acoustid id"
        );
        assert!(
            merged.mb_recording_ids.is_empty(),
            "a below-threshold match must never surface an MB recording id"
        );
    }

    #[tokio::test]
    async fn merge_lookup_matches_uses_configured_thresholds_not_hardcoded_defaults() {
        // WHY: proves the resolver reads ITS OWN config, not a module
        // constant — a score that is "accepted" under the default thresholds
        // must classify as ambiguous under stricter configured ones.
        let resolver = ProviderBackedResolver::new(
            horismos::EpignosisConfig {
                fingerprint_accept_threshold: 0.95,
                fingerprint_ambiguous_threshold: 0.6,
                ..horismos::EpignosisConfig::default()
            },
            ProviderCredentials::default(),
        );
        let matches = vec![score_match("mb-1", "acoustid-1", 0.8)];

        let merged = resolver.merge_lookup_matches(fp_computed(), &matches);

        assert_eq!(merged.match_status, FingerprintMatchStatus::Ambiguous);
    }

    #[test]
    fn canonical_provider_music() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Music),
            "musicbrainz"
        );
    }

    #[test]
    fn canonical_provider_movie() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Movie),
            "tmdb"
        );
    }

    #[test]
    fn canonical_provider_tv() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Tv),
            "tvdb"
        );
    }

    #[test]
    fn canonical_provider_audiobook() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Audiobook),
            "audnexus"
        );
    }

    #[test]
    fn canonical_provider_book() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Book),
            "openlibrary"
        );
    }

    #[test]
    fn canonical_provider_comic() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Comic),
            "comicvine"
        );
    }

    #[test]
    fn canonical_provider_podcast() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::Podcast),
            "itunes"
        );
    }

    #[test]
    fn canonical_provider_news() {
        assert_eq!(
            ProviderBackedResolver::canonical_provider_for(MediaType::News),
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
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({
                "isbn": ["9780441013593", "0441013597"],
            }),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 1.0).abs() < f64::EPSILON
        );
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
            provider: "google_books".to_string(),
            provider_id: MetadataProviderId("abc123".to_string()),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({
                "isbn_13": "9780441013593",
            }),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 1.0).abs() < f64::EPSILON
        );
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
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "Dune".to_string(),
            artist: Some("Frank Herbert".to_string()),
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 0.8).abs() < f64::EPSILON
        );
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
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "Dune".to_string(),
            artist: Some("Different Author".to_string()),
            year: Some(2000),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 0.4).abs() < f64::EPSILON
        );
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
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "Foundation".to_string(),
            artist: Some("Isaac Asimov".to_string()),
            year: Some(1951),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 0.2).abs() < f64::EPSILON
        );
    }

    #[test]
    fn book_score_no_query_artist_does_not_inflate() {
        let query = SearchQuery {
            media_type: MediaType::Book,
            title: "Dune".to_string(),
            artist: None,
            year: Some(1965),
            isbn: None,
            extra: None,
        };

        let result = ProviderResult {
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "Dune".to_string(),
            artist: None,
            year: Some(1965),
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 0.4).abs() < f64::EPSILON,
            "None == None must not claim the title+author+year tier"
        );
    }

    fn music_item(tags: Option<crate::identity::EmbeddedTags>) -> UnidentifiedItem {
        UnidentifiedItem {
            media_id: MediaId::new(),
            media_type: MediaType::Music,
            file_path: std::path::PathBuf::from("/library/track.flac"),
            filename_hint: Some("Fallback Title".to_string()),
            tags,
        }
    }

    #[test]
    fn build_query_prefers_tags_over_filename() {
        let item = music_item(Some(crate::identity::EmbeddedTags {
            title: Some("Tag Title".to_string()),
            artist: Some("Tag Artist".to_string()),
            year: Some(1999),
            ..Default::default()
        }));

        let query = ProviderBackedResolver::build_query(&item);

        assert_eq!(query.title, "Tag Title");
        assert_eq!(query.artist.as_deref(), Some("Tag Artist"));
        assert_eq!(query.year, Some(1999));
    }

    #[test]
    fn build_query_falls_back_to_filename_hint() {
        let item = music_item(Some(crate::identity::EmbeddedTags {
            title: None,
            ..Default::default()
        }));

        let query = ProviderBackedResolver::build_query(&item);

        assert_eq!(query.title, "Fallback Title");
    }

    #[test]
    fn build_query_prefers_album_artist_when_artist_missing() {
        let item = music_item(Some(crate::identity::EmbeddedTags {
            title: Some("Tag Title".to_string()),
            artist: None,
            album_artist: Some("Album Artist".to_string()),
            ..Default::default()
        }));

        let query = ProviderBackedResolver::build_query(&item);

        assert_eq!(query.artist.as_deref(), Some("Album Artist"));
    }

    #[test]
    fn build_query_no_tags_uses_filename_only() {
        let item = music_item(None);

        let query = ProviderBackedResolver::build_query(&item);

        assert_eq!(query.title, "Fallback Title");
        assert_eq!(query.artist, None);
        assert_eq!(query.year, None);
        assert_eq!(query.isbn, None);
    }

    fn movie_item(title: &str) -> UnidentifiedItem {
        UnidentifiedItem {
            media_id: MediaId::new(),
            media_type: MediaType::Movie,
            file_path: std::path::PathBuf::from("/library/movie.mkv"),
            filename_hint: Some(title.to_string()),
            tags: None,
        }
    }

    #[tokio::test]
    async fn resolve_identity_picks_best_result_and_caches_it() {
        let search_body = serde_json::json!({
            "results": [
                { "id": 27205, "title": "Inception", "release_date": "2010-07-16", "popularity": 900.0 },
                { "id": 999, "title": "Inception Parody", "release_date": "2012-01-01", "popularity": 10.0 },
            ],
        })
        .to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, search_body)]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);
        let item = movie_item("Inception");
        let ct = tokio_util::sync::CancellationToken::new();

        let identity = resolver.resolve_identity(&item, ct.clone()).await.unwrap();

        assert_eq!(identity.provider, "tmdb");
        assert_eq!(identity.provider_id.0, "27205");
        assert_eq!(identity.canonical_title, "Inception");
        assert_eq!(identity.year, Some(2010));

        // WHY: the scripted server answers exactly one request — a second
        // resolve must come FROM the cache, not the network.
        let cached = resolver.resolve_identity(&item, ct).await.unwrap();
        assert_eq!(cached.provider_id.0, "27205");

        let requests = handle.await.unwrap();
        assert_eq!(requests.len(), 1, "second resolve must not hit the network");
    }

    #[tokio::test]
    async fn resolve_identity_no_results_is_identity_not_resolved() {
        let search_body = serde_json::json!({ "results": [] }).to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, search_body)]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let err = resolver
            .resolve_identity(
                &movie_item("Nonexistent"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, EpignosisError::IdentityNotResolved { .. }),
            "empty provider results must surface as IdentityNotResolved: {err:?}"
        );
        handle.await.unwrap();
    }

    fn movie_identity(tmdb_id: &str) -> MediaIdentity {
        MediaIdentity {
            media_id: MediaId::new(),
            media_type: MediaType::Movie,
            provider: "tmdb".to_string(),
            provider_id: MetadataProviderId(tmdb_id.to_string()),
            canonical_title: "Inception".to_string(),
            canonical_artist: None,
            year: Some(2010),
            extra: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn enrich_movie_attaches_canonical_metadata() {
        let detail_body = serde_json::json!({
            "id": 27205,
            "title": "Inception",
            "release_date": "2010-07-16",
            "overview": "Dream heist.",
            "runtime": 148,
            "genres": [{ "name": "Sci-Fi" }],
        })
        .to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, detail_body)]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let enriched = resolver
            .enrich(
                &movie_identity("27205"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(enriched.enrichments.len(), 1);
        assert_eq!(enriched.enrichments[0].provider, "tmdb");
        assert_eq!(enriched.enrichments[0].data["overview"], "Dream heist.");
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn enrich_primary_failure_returns_partial_success() {
        let (base_url, handle) = spawn_sequential_http(vec![(500, "not json".to_string())]).await;

        let mut resolver = test_resolver();
        resolver.tmdb = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let enriched = resolver
            .enrich(
                &movie_identity("27205"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap();

        assert!(
            enriched.enrichments.is_empty(),
            "a failed canonical enrichment is dropped, not propagated"
        );
        assert_eq!(enriched.identity.provider_id.0, "27205");
        handle.await.unwrap();
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
            provider: "openlibrary".to_string(),
            provider_id: MetadataProviderId("/works/OL123W".to_string()),
            title: "".to_string(),
            artist: None,
            year: None,
            score: 1.0,
            raw: serde_json::json!({}),
        };

        assert!(
            (ProviderBackedResolver::score_book_result(&result, &query) - 0.2).abs() < f64::EPSILON
        );
    }

    // ── #548: periodic cache eviction sweeper ──────────────────────────────

    #[tokio::test]
    async fn cache_eviction_sweeper_removes_expired_entries_without_a_get() {
        // WHY real (unpaused) time: MetadataCache's TTL bookkeeping is built
        // on std::time::Instant, which tokio's mock clock does not affect —
        // a paused-clock test would race the sweeper against a TTL that
        // never actually elapses in wall-clock terms. A short real interval
        // keeps this fast and deterministic.
        let cache: Arc<MetadataCache<String, serde_json::Value>> =
            Arc::new(MetadataCache::new(Duration::from_millis(1)));
        ProviderBackedResolver::spawn_cache_eviction_sweeper(
            Arc::downgrade(&cache),
            Duration::from_millis(20),
        );

        cache.insert_with_ttl(
            "stale-key".to_string(),
            serde_json::json!({ "x": 1 }),
            Some(Duration::from_millis(1)),
        );
        assert_eq!(cache.len(), 1);

        // WHY: wait several sweeper intervals WITHOUT ever calling get() on
        // the stale key — a get()-triggered eviction would pass this test
        // even without the fix, since it's the SAME-key lazy path #548
        // reports as insufficient for rarely-requeried identities.
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(
            cache.len(),
            0,
            "the periodic sweeper must evict the expired entry without a get() on it"
        );
    }

    #[tokio::test]
    async fn cache_eviction_sweeper_stops_after_resolver_is_dropped() {
        let resolver = ProviderBackedResolver::new(
            horismos::EpignosisConfig::default(),
            ProviderCredentials::default(),
        );
        let cache_weak = Arc::downgrade(&resolver.cache);

        drop(resolver);

        assert!(
            cache_weak.upgrade().is_none(),
            "dropping the resolver must drop its Arc<MetadataCache> — the sweeper must not hold a strong reference that outlives it"
        );

        // WHY: give the sweeper's spawned task a chance to observe the
        // dropped Weak and exit its loop cleanly — proves it self-terminates
        // rather than looping on an upgrade() that will never succeed again.
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
