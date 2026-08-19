//! Production `syntaxis::ImportService`: wires a completed download into the library via kathodos. // kanon:ignore STORAGE/no-migration-checksum -- false positive: this module runs ordinary apotheke::repo CRUD (insert_have/update_want_status), not schema migration code

use std::path::{Path, PathBuf};
use std::pin::Pin;

use aggelmata::{EventSender, MediaType, ReleaseId, WantId};
use apotheke::repo::want::{Have, Release, Want};
use apotheke::repo::{registry, want};
use apotheke::{DbError, begin_immediate, commit_tx};
use horismos::{LibraryConfig, Section, TaxisConfig};
use kathodos::import::identify::resolve_media_type;
use kathodos::import::tags::{FileTags, read_tags};
use kathodos::scanner::filter::is_supported_extension;
use kathodos::scanner::walk::walk_library;
use kathodos::{EpignosisError, ImportOrigin, ImportPipeline, ImportSource, MetadataResolver};
use snafu::{OptionExt, ResultExt, Snafu};
use sqlx::SqlitePool;
use syntaxis::{CompletedDownload, ImportService};
use tokio::sync::Semaphore;
use tracing::{info, instrument, warn};

/// Want media types with a library mapping. Every `wants.media_type` CHECK
/// value now resolves to a `horismos::MediaType` library type via
/// `kathodos::import::identify::resolve_media_type`; the gate remains as a
/// safety net for any future want type added without a library mapping.
const SUPPORTED_WANT_MEDIA_TYPES: [MediaType; 7] = [
    MediaType::Music,
    MediaType::Movie,
    MediaType::Book,
    MediaType::Audiobook,
    MediaType::Comic,
    MediaType::Podcast,
    MediaType::Tv,
];

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ImportAdapterError {
    #[snafu(display("database error: {source}"))]
    Database {
        source: DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("want {want_id} not found"))]
    WantNotFound {
        want_id: WantId,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("release {release_id} not found"))]
    ReleaseNotFound {
        release_id: ReleaseId,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unknown want media_type '{media_type}'"))]
    UnknownWantMediaType {
        media_type: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no library type maps to want media_type '{media_type}'"))]
    UnsupportedWantMediaType {
        media_type: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no configured library maps to media_type {media_type}"))]
    NoMatchingLibrary {
        media_type: MediaType,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("cannot stat download path {path:?}: {source}"))]
    Stat {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("library scan failed at {path:?}: {source}"))]
    Walk {
        path: PathBuf,
        #[snafu(source(from(kathodos::error::TaxisError, Box::new)))]
        source: Box<kathodos::error::TaxisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no importable files found at {path:?}"))]
    NoFiles {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unsupported file for media_type {media_type}: {path:?}"))]
    UnsupportedFile {
        path: PathBuf,
        media_type: MediaType,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("import failed for {path:?}: {source}"))]
    Pipeline {
        path: PathBuf,
        #[snafu(source(from(kathodos::error::TaxisError, Box::new)))]
        source: Box<kathodos::error::TaxisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// Wires a `syntaxis::CompletedDownload` into the library via kathodos'
/// `ImportPipeline`. The honest name for what #300 falsely closed without
/// building — see `syntaxis::ImportService`'s idempotency contract, which
/// this adapter and `kathodos::import::fileops::same_file` jointly satisfy.
///
/// `pub`, not `pub(crate)`: this crate is bin-only otherwise, and the
/// `tests/acquisition_integration` integration suite needs a real
/// `ImportAdapter` (not a mock) to exercise the full completion→import→have
/// flow — see `crates/archon/src/lib.rs`.
pub struct ImportAdapter {
    read_pool: SqlitePool,
    write_pool: SqlitePool,
    taxis: Section<TaxisConfig>,
    event_tx: EventSender,
}

impl ImportAdapter {
    pub fn new(
        read_pool: SqlitePool,
        write_pool: SqlitePool,
        taxis: Section<TaxisConfig>,
        event_tx: EventSender,
    ) -> Self {
        Self {
            read_pool,
            write_pool,
            taxis,
            event_tx,
        }
    }

    #[instrument(skip(self, completed), fields(want_id = %completed.want_id, release_id = %completed.release_id))]
    async fn import_inner(&self, completed: CompletedDownload) -> Result<(), ImportAdapterError> {
        let want_id_bytes = completed.want_id.as_bytes().to_vec();
        let release_id_bytes = completed.release_id.as_bytes().to_vec();

        // 1. want / release
        let want = want::get_want(&self.read_pool, &want_id_bytes)
            .await
            .context(DatabaseSnafu)?
            .context(WantNotFoundSnafu {
                want_id: completed.want_id,
            })?;
        let release = want::get_release(&self.read_pool, &release_id_bytes)
            .await
            .context(DatabaseSnafu)?
            .context(ReleaseNotFoundSnafu {
                release_id: completed.release_id,
            })?;

        // 2. media type mapping
        let mapped_media_type =
            MediaType::parse_want_str(&want.media_type).context(UnknownWantMediaTypeSnafu {
                media_type: want.media_type.clone(),
            })?;
        if !SUPPORTED_WANT_MEDIA_TYPES.contains(&mapped_media_type) {
            return UnsupportedWantMediaTypeSnafu {
                media_type: want.media_type.clone(),
            }
            .fail();
        }

        // 3. library selection — deterministic lexicographic pick among matches
        let taxis = self.taxis.get();
        let mut candidates: Vec<(&String, &LibraryConfig)> = taxis
            .libraries
            .iter()
            .filter(|(_, lib)| resolve_media_type(&lib.media_type) == Some(mapped_media_type))
            .collect();
        candidates.sort_by(|a, b| a.0.cmp(b.0));
        let (library_name, library_cfg) = candidates
            .first()
            .map(|(name, cfg)| ((*name).clone(), (*cfg).clone()))
            .context(NoMatchingLibrarySnafu {
                media_type: mapped_media_type,
            })?;

        // 4. haves short-circuit — idempotent no-op if a complete have for
        // this exact (want, release) still points at a present target on
        // disk. This is a fast-path optimization only, keyed on release_id
        // (the durable idempotency + upgrade logic lives in the
        // file_path-keyed transactional finalize at step 7/8 below, which
        // covers the case this fast path cannot see: a different release
        // resolving to the same library location).
        let now = jiff::Timestamp::now().to_string();
        let existing_haves = want::list_haves_for_want(&self.read_pool, &want_id_bytes)
            .await
            .context(DatabaseSnafu)?;
        let matching_have = existing_haves
            .into_iter()
            .find(|h| h.release_id.as_deref() == Some(release_id_bytes.as_slice()));
        if let Some(have) = &matching_have
            && have.status == "complete"
        {
            // WHY: for Music `have.file_path` is the album DIRECTORY — a
            // directory existing is not proof it holds anything (an empty
            // or partially-cleaned dir must fall through to re-import).
            // Movie/Book already point at the single file itself.
            let present = if mapped_media_type == MediaType::Music {
                directory_contains_media(&have.file_path, mapped_media_type).await
            } else {
                tokio::fs::try_exists(&have.file_path)
                    .await
                    .unwrap_or(false)
            };
            if present {
                info!(file_path = %have.file_path, "have already complete on disk  -  import is a no-op");
                // WHY: a complete have with files present must never return
                // Ok while leaving the want un-fulfilled — otherwise a
                // crash between a prior insert_have and update_want_status
                // (or a recovery::reload_queue replay) permanently strands
                // the want. update_want_status is idempotent.
                want::update_want_status(&self.write_pool, &want_id_bytes, "fulfilled", Some(&now))
                    .await
                    .context(DatabaseSnafu)?;
                return Ok(());
            }
            warn!(file_path = %have.file_path, "recorded have file missing or empty on disk  -  reimporting");
        }

        // 5. enumerate source files
        let files = enumerate_files(&completed.download_path, mapped_media_type).await?;

        // 6. per-file import through the shared kathodos pipeline
        let registry_display_name = match &want.registry_id {
            Some(id) => registry::get_registry_entry(&self.read_pool, id)
                .await
                .context(DatabaseSnafu)?
                .map(|entry| entry.display_name),
            None => None,
        };
        let resolver = DownloadResolver::new(&want, &release, registry_display_name);
        let pipeline = ImportPipeline::new(resolver, self.event_tx.clone());
        let mut results = Vec::with_capacity(files.len());
        for file in files {
            let source = ImportSource {
                path: file.clone(),
                library_name: library_name.clone(),
                media_type: mapped_media_type,
                origin: ImportOrigin::Download {
                    want_id: completed.want_id,
                    release_id: completed.release_id,
                },
                naming_template: None,
                library_root: library_cfg.path.clone(),
            };
            let result = pipeline
                .process(source)
                .await
                .context(PipelineSnafu { path: file })?;
            results.push(result);
        }
        // WHY: enumerate_files errors on zero files, so `results` is never
        // empty here — the .first() unwrap surface is a defensive Option
        // rather than a real reachable branch.
        let Some(first) = results.first() else {
            return NoFilesSnafu {
                path: completed.download_path.clone(),
            }
            .fail();
        };

        // 7+8. finalize atomically: land the have row and fulfil the want
        // in one `BEGIN IMMEDIATE` transaction, keyed on file_path (NOT
        // release_id) — for Music, file_path is the want-derived album
        // DIRECTORY, identical across every release of the same album, so
        // a second release for an already-fulfilled want (the
        // quality-upgrade path) resolves to the SAME file_path the first
        // release's have already owns. file_path is therefore the natural
        // collision key against `idx_haves_file_path` (unconditional
        // UNIQUE). Any prior have at this file_path — a same-release retry
        // or a genuine cross-release upgrade — is deleted inside the same
        // transaction, and its id becomes the new have's
        // `upgraded_from_id` (exactly what that column is for).
        let file_path = if mapped_media_type == MediaType::Music {
            first
                .final_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| first.final_path.clone())
        } else {
            first.final_path.clone()
        };
        let file_size_bytes = total_size(results.iter().map(|r| r.final_path.as_path())).await?;
        let file_path_str = file_path.to_string_lossy().into_owned();

        let mut tx = begin_immediate(&self.write_pool)
            .await
            .context(DatabaseSnafu)?;

        let stale = want::get_have_by_file_path(&mut *tx, &file_path_str)
            .await
            .context(DatabaseSnafu)?;
        let upgraded_from_id = if let Some(stale) = stale {
            want::delete_have(&mut *tx, &stale.id)
                .await
                .context(DatabaseSnafu)?;
            Some(stale.id)
        } else {
            None
        };

        let have = Have {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            want_id: want_id_bytes.clone(),
            release_id: Some(release_id_bytes.clone()),
            media_type: want.media_type.clone(),
            media_type_id: first.media_id.as_bytes().to_vec(),
            quality_score: release.quality_score,
            file_path: file_path_str,
            file_size_bytes,
            status: "complete".to_string(),
            imported_at: now.clone(),
            upgraded_from_id,
        };
        want::insert_have(&mut *tx, &have)
            .await
            .context(DatabaseSnafu)?;
        want::update_want_status(&mut *tx, &want_id_bytes, "fulfilled", Some(&now))
            .await
            .context(DatabaseSnafu)?;

        commit_tx(tx).await.context(DatabaseSnafu)?;

        Ok(())
    }
}

impl ImportService for ImportAdapter {
    fn import(
        &self,
        completed: CompletedDownload,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            self.import_inner(completed)
                .await
                .map_err(|e| e.to_string())
        })
    }
}

/// True if `path` is a directory holding at least one importable media file
/// for `media_type`.
///
/// FIX 4 (re-review): checking mere non-emptiness let a debris-only directory
/// (an album whose audio was removed by a cleanup, leaving only `.nfo` /
/// thumbnail / `.DS_Store` sidecars) read as a complete have — the want would
/// be marked fulfilled with no real content and never re-acquired. A missing,
/// unreadable, empty, or media-free directory is `false` so the short-circuit
/// falls through to re-import.
async fn directory_contains_media(path: &str, media_type: MediaType) -> bool {
    let path = PathBuf::from(path);
    tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&path) else {
            return false;
        };
        entries.flatten().any(|entry| {
            let p = entry.path();
            p.is_file() && is_supported_extension(&p, media_type)
        })
    })
    .await
    .unwrap_or(false)
}

/// Resolves `download_path` INTO the list of importable source files:
/// the containing directory's supported members for a multi-file download,
/// or the file itself for a single-file one (see `CompletedDownload::download_path`).
async fn enumerate_files(
    download_path: &Path,
    media_type: MediaType,
) -> Result<Vec<PathBuf>, ImportAdapterError> {
    let metadata = tokio::fs::metadata(download_path)
        .await
        .context(StatSnafu {
            path: download_path.to_path_buf(),
        })?;

    if metadata.is_dir() {
        // WHY: one walk per import call — a fresh permit-1 semaphore is
        // sufficient (this is not the scanner's cross-library concurrency
        // limiter; `walk_library` merely requires a `&Semaphore` handle).
        let semaphore = Semaphore::new(1);
        let (results, _stats) = walk_library(download_path, media_type, &semaphore)
            .await
            .context(WalkSnafu {
                path: download_path.to_path_buf(),
            })?;
        if results.is_empty() {
            return NoFilesSnafu {
                path: download_path.to_path_buf(),
            }
            .fail();
        }
        let files: Vec<PathBuf> = results.into_iter().map(|r| r.path).collect();
        // FIX 3: Music imports every track (distinct file_path per file —
        // correct). Movie/Book have no per-file disambiguation token in
        // their naming template (see kathodos::import::template::
        // valid_tokens), so every enumerated file would resolve to the
        // SAME target path — import only the single largest candidate
        // (the feature/main file); companions (samples, extras, subtitle
        // sidecars) are skipped and info-logged.
        if media_type == MediaType::Music {
            Ok(files)
        } else {
            select_largest_file(files)
                .await
                .context(NoFilesSnafu {
                    path: download_path.to_path_buf(),
                })
                .map(|p| vec![p])
        }
    } else {
        if !is_supported_extension(download_path, media_type) {
            return UnsupportedFileSnafu {
                path: download_path.to_path_buf(),
                media_type,
            }
            .fail();
        }
        Ok(vec![download_path.to_path_buf()])
    }
}

/// Picks the single largest file (the feature/main file) from a multi-file
/// candidate set, info-logging the rest as skipped companions. `None` only
/// for an empty input (callers guarantee non-empty via a prior
/// `results.is_empty()` check upstream in `enumerate_files`).
async fn select_largest_file(files: Vec<PathBuf>) -> Option<PathBuf> {
    let mut sized: Vec<(PathBuf, u64)> = tokio::task::spawn_blocking(move || {
        files
            .into_iter()
            .map(|p| {
                let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                (p, size)
            })
            .collect()
    })
    .await
    .unwrap_or_default(); // WHY: a spawn_blocking join failure (task panic) falls back to no candidates, not a fabricated file — `largest` below then legitimately becomes `None`

    sized.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let mut candidates = sized.into_iter();
    let largest = candidates.next().map(|(p, _)| p);
    for (skipped, _) in candidates {
        info!(path = %skipped.display(), "skipping companion file  -  single largest file wins for this media type");
    }
    largest
}

async fn total_size<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<i64, ImportAdapterError> {
    let owned: Vec<PathBuf> = paths.map(Path::to_path_buf).collect();
    let sum = tokio::task::spawn_blocking(move || {
        owned
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum::<u64>()
    })
    .await
    .unwrap_or(0);
    Ok(i64::try_from(sum).unwrap_or(i64::MAX))
}

/// Best-effort 4-digit year extraction FROM free text (e.g. a release
/// title like "Album Name (2019) [FLAC]"). Splits on maximal digit runs so a
/// longer run (catalog numbers, bitrates) never false-matches on its first 4
/// digits; `None` if no run of exactly 4 digits falls in a plausible
/// release-year range.
fn best_effort_year(text: &str) -> Option<u32> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|run| run.chars().count() == 4)
        .find_map(|run| {
            run.parse::<u32>()
                .ok()
                .filter(|year| (1900..=2099).contains(year))
        })
}

/// Per-import `MetadataResolver`: builds template tokens from a fixed
/// priority ladder — embedded tags first, then DB hints (want/release/
/// registry), then a filename-parse fallback. Deliberately does not touch
/// epignosis's provider/network lookups: imports stay deterministic; richer
/// identification is a later enrichment pass over the same library.
struct DownloadResolver {
    want_title: String,
    artist_or_author: Option<String>,
    release_title: String,
    quality_score: u32,
}

impl DownloadResolver {
    fn new(want: &Want, release: &Release, registry_display_name: Option<String>) -> Self {
        Self {
            want_title: want.title.clone(),
            artist_or_author: registry_display_name,
            release_title: release.title.clone(),
            quality_score: u32::try_from(release.quality_score.max(0)).unwrap_or(u32::MAX),
        }
    }
}

impl MetadataResolver for DownloadResolver {
    async fn resolve_identity(
        &self,
        path: &Path,
        media_type: MediaType,
    ) -> Result<kathodos::ResolvedMetadata, EpignosisError> {
        let tags = match read_tags(path).await {
            Ok(tags) => tags,
            Err(e) => {
                // WHY: a per-file tag-read failure is not fatal — the
                // resolver falls back to DB hints / filename parsing.
                warn!(path = %path.display(), error = %e, "tag read failed  -  falling back to DB/filename hints");
                FileTags::default()
            }
        };
        if tags.is_empty() {
            // WHY: distinct FROM the read-error branch above — a successful
            // read that simply found no populated fields (untagged file, or
            // a container lofty parses but doesn't map any items for). Same
            // fallback path, worth a lower-noise log line for diagnosing a
            // library that resolves mostly FROM filenames.
            tracing::debug!(path = %path.display(), "no embedded tags  -  resolving FROM DB/filename hints only");
        }
        let parsed = epignosis::parse_filename(path);
        let year = tags.year.or_else(|| best_effort_year(&self.release_title));
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let mut tokens = std::collections::HashMap::new();
        tokens.insert("Extension".to_string(), extension);

        match media_type {
            MediaType::Music => {
                let artist = tags
                    .artist
                    .clone()
                    .or_else(|| tags.album_artist.clone())
                    .or_else(|| self.artist_or_author.clone())
                    .or_else(|| parsed.artist.clone());
                if let Some(v) = artist {
                    tokens.insert("Artist Name".to_string(), v);
                }
                tokens.insert("Album Title".to_string(), self.want_title.clone());
                if let Some(y) = year {
                    tokens.insert("Year".to_string(), y.to_string());
                }
                if let Some(t) = tags.track_number.or(parsed.track_number) {
                    tokens.insert("Track Number".to_string(), t.to_string());
                }
                if let Some(d) = tags.disc_number {
                    tokens.insert("Disc Number".to_string(), d.to_string());
                }
                let title = tags
                    .title
                    .clone()
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(parsed.title.clone()).filter(|s| !s.is_empty()));
                if let Some(t) = title {
                    tokens.insert("Track Title".to_string(), t);
                }
            }
            MediaType::Movie => {
                tokens.insert("Movie Title".to_string(), self.want_title.clone());
                if let Some(y) = year {
                    tokens.insert("Year".to_string(), y.to_string());
                }
            }
            MediaType::Book => {
                if let Some(v) = self.artist_or_author.clone() {
                    tokens.insert("Author Name".to_string(), v);
                }
                tokens.insert("Title".to_string(), self.want_title.clone());
                if let Some(y) = year {
                    tokens.insert("Year".to_string(), y.to_string());
                }
            }
            MediaType::Audiobook => {
                let author = tags
                    .artist
                    .clone()
                    .or_else(|| tags.album_artist.clone())
                    .or_else(|| self.artist_or_author.clone())
                    .or_else(|| parsed.artist.clone());
                if let Some(v) = author {
                    tokens.insert("Author Name".to_string(), v);
                }
                tokens.insert("Title".to_string(), self.want_title.clone());
                if let Some(y) = year {
                    tokens.insert("Year".to_string(), y.to_string());
                }
            }
            MediaType::Comic => {
                tokens.insert("Series Name".to_string(), self.want_title.clone());
                if let Some(y) = year {
                    tokens.insert("Year".to_string(), y.to_string());
                }
            }
            MediaType::Podcast => {
                tokens.insert("Podcast Title".to_string(), self.want_title.clone());
                let episode = tags
                    .title
                    .clone()
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(parsed.title.clone()).filter(|s| !s.is_empty()));
                if let Some(t) = episode {
                    tokens.insert("Episode Title".to_string(), t);
                }
            }
            MediaType::Tv => {
                tokens.insert("Series Title".to_string(), self.want_title.clone());
                let episode_title = tags
                    .title
                    .clone()
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(parsed.title.clone()).filter(|s| !s.is_empty()));
                if let Some(t) = episode_title {
                    tokens.insert("Episode Title".to_string(), t);
                }
            }
            // WHY: `News` has no want representation (parse_want_str -> None), so
            // no want ever maps to it, and the gate admits the other 7. This arm
            // is the non_exhaustive fallback for a future aggelmata variant.
            // NOTE: season/episode/issue/series tokens need metadata enrichment,
            // not available in this filename/DB-hint resolver — the template
            // skips the missing tokens, keeping the primary identifying tokens.
            _ => {}
        }

        Ok(kathodos::ResolvedMetadata {
            media_type,
            tokens,
            quality_score: self.quality_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aggelmata::create_event_bus;
    use apotheke::migrate::MIGRATOR;
    use horismos::{LibraryConfig, MediaType as LibMediaType, WatcherMode};

    use super::*;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    async fn seed_profile(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn adapter(pool: SqlitePool, libraries: HashMap<String, LibraryConfig>) -> ImportAdapter {
        let (event_tx, _rx) = create_event_bus(16);
        ImportAdapter::new(
            pool.clone(),
            pool,
            Section::fixed(TaxisConfig {
                libraries,
                ..TaxisConfig::default()
            }),
            event_tx,
        )
    }

    fn library(path: &str, media_type: LibMediaType) -> LibraryConfig {
        LibraryConfig {
            path: PathBuf::from(path),
            media_type,
            watcher_mode: WatcherMode::Auto,
            poll_interval_seconds: 300,
            scan_interval_hours: 24,
        }
    }

    fn music_library(path: &str) -> LibraryConfig {
        library(path, LibMediaType::Music)
    }

    fn make_completed(
        want_id: WantId,
        release_id: ReleaseId,
        download_path: PathBuf,
    ) -> CompletedDownload {
        CompletedDownload {
            download_id: aggelmata::DownloadId::new(),
            download_path,
            source_path: PathBuf::from("/unused"),
            want_id,
            release_id,
            protocol: syntaxis::DownloadProtocol::Torrent,
            requires_copy: false,
        }
    }

    async fn seed_want_release(pool: &SqlitePool, media_type: &str) -> (WantId, ReleaseId) {
        let profile_id = seed_profile(pool).await;
        let want_id = WantId::new();
        want::insert_want(
            pool,
            &Want {
                id: want_id.as_bytes().to_vec(),
                media_type: media_type.to_string(),
                title: "Test Album".to_string(),
                registry_id: None,
                quality_profile_id: profile_id,
                status: "searching".to_string(),
                source: None,
                source_ref: None,
                added_at: "2026-01-01T00:00:00Z".to_string(),
                fulfilled_at: None,
            },
        )
        .await
        .unwrap();

        let release_id = ReleaseId::new();
        want::insert_release(
            pool,
            &Release {
                id: release_id.as_bytes().to_vec(),
                want_id: want_id.as_bytes().to_vec(),
                indexer_id: 1,
                title: "Test Album FLAC".to_string(),
                size_bytes: 1000,
                quality_score: 90,
                custom_format_score: 0,
                download_url: "https://example.com/release.torrent".to_string(),
                protocol: "torrent".to_string(),
                info_hash: None,
                found_at: "2026-01-01T00:00:00Z".to_string(),
                grabbed_at: None,
                rejected_reason: None,
            },
        )
        .await
        .unwrap();

        (want_id, release_id)
    }

    #[tokio::test]
    async fn no_matching_library_is_an_error() {
        let pool = migrated_pool().await;
        let (want_id, release_id) = seed_want_release(&pool, "music_album").await;
        let dir = tempfile::TempDir::new().unwrap();
        let download_path = dir.path().join("album");
        std::fs::create_dir_all(&download_path).unwrap();
        std::fs::write(download_path.join("track.flac"), b"data").unwrap();

        let svc = adapter(pool, HashMap::new());
        let result = svc
            .import_inner(make_completed(want_id, release_id, download_path))
            .await;

        assert!(matches!(
            result,
            Err(ImportAdapterError::NoMatchingLibrary { .. })
        ));
    }

    #[tokio::test]
    async fn each_media_type_imports_into_its_library() {
        // WHY(#612): a want of each newly-supported type resolves its library
        // type and imports end-to-end, rather than erroring at the gate.
        let cases = [
            ("audiobook", LibMediaType::Audiobook, "book.m4b"),
            ("comic", LibMediaType::Comic, "issue.cbz"),
            ("podcast", LibMediaType::Podcast, "episode.mp3"),
            ("tv_series", LibMediaType::Tv, "episode.mkv"),
        ];
        for (want_type, lib_type, filename) in cases {
            let pool = migrated_pool().await;
            let (want_id, release_id) = seed_want_release(&pool, want_type).await;
            let dir = tempfile::TempDir::new().unwrap();
            let download_path = dir.path().join("download");
            std::fs::create_dir_all(&download_path).unwrap();
            std::fs::write(download_path.join(filename), b"data").unwrap();
            let lib_root = dir.path().join("library");
            std::fs::create_dir_all(&lib_root).unwrap();

            let mut libraries = HashMap::new();
            libraries.insert(
                "lib".to_string(),
                library(lib_root.to_str().unwrap(), lib_type),
            );
            let svc = adapter(pool.clone(), libraries);

            svc.import_inner(make_completed(want_id, release_id, download_path))
                .await
                .unwrap_or_else(|e| panic!("{want_type} import failed: {e}"));

            let want = want::get_want(&pool, want_id.as_bytes().as_ref())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                want.status, "fulfilled",
                "{want_type} want should be fulfilled after import"
            );

            // WHY: the imported file must land in the library carrying the title
            // (not an anonymous `.ext`) — proves resolve_identity populates a
            // naming token for this type, per #612's organizing purpose.
            let haves = want::list_haves_for_want(&pool, want_id.as_bytes().as_ref())
                .await
                .unwrap();
            let landed = haves
                .iter()
                .find(|h| h.file_path.starts_with(lib_root.to_str().unwrap()))
                .unwrap_or_else(|| panic!("{want_type}: no have landed in library: {haves:?}"));
            assert!(
                landed.file_path.contains("Test Album"),
                "{want_type}: imported file must carry the title, got {:?}",
                landed.file_path
            );
        }
    }

    #[tokio::test]
    async fn two_music_libraries_picks_lexicographically_first() {
        let pool = migrated_pool().await;
        let (want_id, release_id) = seed_want_release(&pool, "music_album").await;
        let dir = tempfile::TempDir::new().unwrap();
        let download_path = dir.path().join("album");
        std::fs::create_dir_all(&download_path).unwrap();
        std::fs::write(download_path.join("track.flac"), b"data").unwrap();

        let lib_root_a = dir.path().join("lib-a");
        let lib_root_b = dir.path().join("lib-b");
        std::fs::create_dir_all(&lib_root_a).unwrap();
        std::fs::create_dir_all(&lib_root_b).unwrap();

        let mut libraries = HashMap::new();
        libraries.insert(
            "zzz-library".to_string(),
            music_library(lib_root_b.to_str().unwrap()),
        );
        libraries.insert(
            "aaa-library".to_string(),
            music_library(lib_root_a.to_str().unwrap()),
        );

        let svc = adapter(pool, libraries);
        svc.import_inner(make_completed(want_id, release_id, download_path))
            .await
            .unwrap();

        // "aaa-library" sorts first lexicographically — its root must have
        // received the import, "zzz-library" must not.
        let a_has_files = std::fs::read_dir(&lib_root_a).unwrap().next().is_some();
        assert!(a_has_files, "lexicographically-first library must be used");
        let b_has_files = std::fs::read_dir(&lib_root_b).unwrap().next().is_some();
        assert!(
            !b_has_files,
            "non-selected library must not receive the import"
        );
    }

    // WHY: Music `have.file_path` is the album DIRECTORY (see FIX 4) — the
    // short-circuit fixture must reflect that, not a bare file, or the
    // directory-emptiness check would (correctly) refuse to short-circuit.
    fn complete_music_have(
        want_id: WantId,
        release_id: ReleaseId,
        file_path: &std::path::Path,
    ) -> Have {
        Have {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            want_id: want_id.as_bytes().to_vec(),
            release_id: Some(release_id.as_bytes().to_vec()),
            media_type: "music_album".to_string(),
            media_type_id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            quality_score: 90,
            file_path: file_path.to_string_lossy().into_owned(),
            file_size_bytes: 4,
            status: "complete".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            upgraded_from_id: None,
        }
    }

    // ── FIX 1b / crash-replay: short-circuit must fulfil the want ──────────

    #[tokio::test]
    async fn haves_short_circuit_skips_reimport_when_directory_still_present() {
        let pool = migrated_pool().await;
        let (want_id, release_id) = seed_want_release(&pool, "music_album").await;
        let dir = tempfile::TempDir::new().unwrap();

        let existing_album_dir = dir.path().join("already-here-album");
        std::fs::create_dir_all(&existing_album_dir).unwrap();
        std::fs::write(existing_album_dir.join("track.flac"), b"data").unwrap();

        want::insert_have(
            &pool,
            &complete_music_have(want_id, release_id, &existing_album_dir),
        )
        .await
        .unwrap();

        // A download_path that would error if actually touched — proves the
        // short-circuit returns before any filesystem enumeration.
        let untouched_download_path = dir.path().join("does-not-exist");

        let mut libraries = HashMap::new();
        libraries.insert(
            "music".to_string(),
            music_library(dir.path().to_str().unwrap()),
        );
        let svc = adapter(pool.clone(), libraries);

        let result = svc
            .import_inner(make_completed(want_id, release_id, untouched_download_path))
            .await;
        assert!(result.is_ok(), "expected no-op Ok, got {result:?}");

        // FIX 1b (crash-replay): a complete have with files present must
        // never return Ok while leaving the want un-fulfilled — this is the
        // self-heal for a crash between a prior insert_have and
        // update_want_status (or a recovery::reload_queue replay).
        // `seed_want_release` leaves the want at "searching"; the
        // short-circuit alone must be the thing that moves it to
        // "fulfilled" here, since no full import ran.
        let want = want::get_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(want.status, "fulfilled");
    }

    // ── FIX 4: empty/partial directory must not short-circuit ──────────────

    #[tokio::test]
    async fn haves_short_circuit_falls_through_when_album_directory_has_no_media() {
        let pool = migrated_pool().await;
        let (want_id, release_id) = seed_want_release(&pool, "music_album").await;
        let dir = tempfile::TempDir::new().unwrap();

        // An album directory recorded as "complete" whose audio was removed by
        // a cleanup, leaving only non-media debris (a .nfo sidecar). This is
        // NON-empty, so a mere entry-count check would wrongly short-circuit;
        // the media-aware check must still fall through and re-import.
        let empty_album_dir = dir.path().join("debris-album");
        std::fs::create_dir_all(&empty_album_dir).unwrap();
        std::fs::write(empty_album_dir.join("album.nfo"), b"leftover metadata").unwrap();

        want::insert_have(
            &pool,
            &complete_music_have(want_id, release_id, &empty_album_dir),
        )
        .await
        .unwrap();

        let download_path = dir.path().join("download");
        std::fs::create_dir_all(&download_path).unwrap();
        std::fs::write(download_path.join("track.flac"), b"data").unwrap();

        let lib_root = dir.path().join("library");
        std::fs::create_dir_all(&lib_root).unwrap();
        let mut libraries = HashMap::new();
        libraries.insert(
            "music".to_string(),
            music_library(lib_root.to_str().unwrap()),
        );
        let svc = adapter(pool.clone(), libraries);

        svc.import_inner(make_completed(want_id, release_id, download_path))
            .await
            .unwrap();

        let haves = want::list_haves_for_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap();
        let reimported = haves
            .iter()
            .any(|h| h.file_path.starts_with(lib_root.to_str().unwrap()));
        assert!(
            reimported,
            "an empty recorded directory must fall through and actually re-import, \
             not short-circuit as complete: {haves:?}"
        );

        let want = want::get_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(want.status, "fulfilled");
    }

    // ── FIX 1a/1c: upgrade path — file_path-keyed, atomic finalize ─────────

    #[tokio::test]
    async fn second_release_for_same_want_upgrades_have_without_unique_violation() {
        let pool = migrated_pool().await;
        let (want_id, release_a) = seed_want_release(&pool, "music_album").await;

        // A second release for the SAME want — the quality-upgrade path.
        // Both releases resolve to the SAME album directory (same want
        // title, same filename — see DownloadResolver::resolve_identity),
        // which is exactly the file_path collision FIX 1a addresses.
        let release_b = ReleaseId::new();
        want::insert_release(
            &pool,
            &Release {
                id: release_b.as_bytes().to_vec(),
                want_id: want_id.as_bytes().to_vec(),
                indexer_id: 1,
                title: "Test Album FLAC Upgrade".to_string(),
                size_bytes: 2000,
                quality_score: 95,
                custom_format_score: 0,
                download_url: "https://example.com/release-b.torrent".to_string(),
                protocol: "torrent".to_string(),
                info_hash: None,
                found_at: "2026-01-01T00:00:00Z".to_string(),
                grabbed_at: None,
                rejected_reason: None,
            },
        )
        .await
        .unwrap();

        let dir = tempfile::TempDir::new().unwrap();
        let lib_root = dir.path().join("library");
        std::fs::create_dir_all(&lib_root).unwrap();
        let mut libraries = HashMap::new();
        libraries.insert(
            "music".to_string(),
            music_library(lib_root.to_str().unwrap()),
        );
        let svc = adapter(pool.clone(), libraries);

        let download_a = dir.path().join("download-a");
        std::fs::create_dir_all(&download_a).unwrap();
        std::fs::write(download_a.join("track.flac"), b"release a data").unwrap();
        svc.import_inner(make_completed(want_id, release_a, download_a))
            .await
            .unwrap();

        let haves_after_a = want::list_haves_for_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap();
        assert_eq!(haves_after_a.len(), 1);
        let first_have_id = haves_after_a[0].id.clone();

        let download_b = dir.path().join("download-b");
        std::fs::create_dir_all(&download_b).unwrap();
        std::fs::write(download_b.join("track.flac"), b"release b data upgraded").unwrap();

        // Must NOT be a UNIQUE-constraint (idx_haves_file_path) violation.
        svc.import_inner(make_completed(want_id, release_b, download_b))
            .await
            .unwrap();

        let haves_after_b = want::list_haves_for_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap();
        assert_eq!(
            haves_after_b.len(),
            1,
            "upgrade must replace, not duplicate, the have row: {haves_after_b:?}"
        );
        let have = &haves_after_b[0];
        assert_eq!(
            have.release_id.as_deref(),
            Some(release_b.as_bytes().as_slice())
        );
        assert_eq!(
            have.upgraded_from_id.as_deref(),
            Some(first_have_id.as_slice()),
            "upgraded_from_id must point at the prior have it replaced"
        );

        let want = want::get_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(want.status, "fulfilled");
    }

    // ── FIX 3: Movie/Book multi-file collision — largest file wins ─────────

    #[tokio::test]
    async fn movie_multi_file_download_imports_only_the_largest_file() {
        let pool = migrated_pool().await;
        let (want_id, release_id) = seed_want_release(&pool, "movie").await;

        let dir = tempfile::TempDir::new().unwrap();
        let download_path = dir.path().join("movie-download");
        std::fs::create_dir_all(&download_path).unwrap();
        std::fs::write(download_path.join("movie.mkv"), vec![0u8; 5000]).unwrap();
        std::fs::write(download_path.join("sample.mkv"), vec![0u8; 100]).unwrap();

        let lib_root = dir.path().join("library");
        std::fs::create_dir_all(&lib_root).unwrap();
        let mut libraries = HashMap::new();
        libraries.insert(
            "movies".to_string(),
            LibraryConfig {
                path: lib_root.clone(),
                media_type: LibMediaType::Video,
                watcher_mode: WatcherMode::Auto,
                poll_interval_seconds: 300,
                scan_interval_hours: 24,
            },
        );
        let svc = adapter(pool.clone(), libraries);

        svc.import_inner(make_completed(want_id, release_id, download_path))
            .await
            .unwrap();

        let haves = want::list_haves_for_want(&pool, want_id.as_bytes().as_ref())
            .await
            .unwrap();
        assert_eq!(
            haves.len(),
            1,
            "only the single largest file should have been imported: {haves:?}"
        );
        assert_eq!(haves[0].file_size_bytes, 5000);
    }

    #[test]
    fn best_effort_year_finds_a_plausible_four_digit_run() {
        assert_eq!(best_effort_year("Album Name (2019) [FLAC]"), Some(2019));
        assert_eq!(best_effort_year("no year here"), None);
        // A 5-digit run must not false-match on its first 4 digits.
        assert_eq!(best_effort_year("catalog-12345"), None);
        assert_eq!(best_effort_year(""), None);
    }
}
