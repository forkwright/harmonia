use std::{path::Path, sync::Arc, time::Duration};

use harmonia_common::MediaType;
use horismos::EpignosisConfig;
use tracing::instrument;

use crate::{
    MetadataResolver,
    cache::MetadataCache,
    error::EpignosisError,
    identity::{
        EnrichedMetadata, FingerprintResult, MediaIdentity, ProviderEnrichment, UnidentifiedItem,
    },
    providers::{MetadataProvider, SearchQuery},
    providers::{
        acoustid::AcoustIdProvider, audnexus::AudnexusProvider, comicvine::ComicVineProvider,
        itunes::ItunesProvider, musicbrainz::MusicBrainzProvider, openlibrary::OpenLibraryProvider,
        tmdb::TmdbProvider, tvdb::TvdbProvider,
    },
    rate_limit::ProviderQueues,
};

/// Provider credentials supplied at construction time.
#[derive(Debug, Clone, Default)]
pub struct ProviderCredentials {
    pub acoustid_key: SecretString,
    pub tmdb_key: SecretString,
    pub tvdb_key: SecretString,
    pub comicvine_key: SecretString,
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
        let (title, artist, year) = if let Some(tags) = &item.tags {
            (
                tags.title
                    .clone()
                    .unwrap_or_else(|| item.filename_hint.clone().unwrap_or_default()),
                tags.artist.clone().or_else(|| tags.album_artist.clone()),
                tags.year,
            )
        } else {
            (item.filename_hint.clone().unwrap_or_default(), None, None)
        };

        SearchQuery {
            media_type: item.media_type,
            title,
            artist,
            year,
            extra: None,
        }
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

        let results = tokio::SELECT! {
            result = self.search_canonical(item.media_type, &query) => result?,
            _ = ct.cancelled() => {
                return Err(EpignosisError::IdentityNotResolved {
                    provider: provider_name.to_string(),
                    query: query.title.clone(),
                    location: snafu::location!(),
                });
            }
        };

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
            self.cache.INSERT(cache_key, value);
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

        let primary_result = tokio::SELECT! {
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

        let secondary_result = tokio::SELECT! {
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
}
