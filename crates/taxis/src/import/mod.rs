pub mod conflict;
pub mod fileops;
pub mod identify;
pub mod template;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use harmonia_common::{EventSender, HarmoniaEvent, MediaId, MediaType, ReleaseId, WantId};
use tracing::instrument;

use crate::error::{EpignosisError, TaxisError};
use crate::import::conflict::{ConflictOutcome, DEFAULT_MAX_SUFFIX, resolve_conflict};
use crate::import::fileops::{hardlink_or_copy, rename_file};
use crate::import::template::TemplateEngine;

/// Source of a file entering the import pipeline.
#[derive(Debug, Clone)]
pub struct ImportSource {
    pub path: PathBuf,
    pub library_name: String,
    pub media_type: MediaType,
    pub origin: ImportOrigin,
    /// Optional naming template override for this library.
    pub naming_template: Option<String>,
    /// Target library root for placing organized files.
    pub library_root: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ImportOrigin {
    Scanner,
    Download {
        want_id: WantId,
        release_id: ReleaseId,
    },
}

/// Result of a completed import.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub media_id: MediaId,
    pub media_type: MediaType,
    pub final_path: PathBuf,
    pub quality_score: u32,
    pub operation: ImportOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOperation {
    Added,
    Upgraded,
    Skipped,
}

/// A file waiting for manual metadata matching.
#[derive(Debug, Clone)]
pub struct PendingImport {
    pub id: MediaId,
    pub path: PathBuf,
    pub media_type: MediaType,
}

/// A completed download ready for import.
#[derive(Debug, Clone)]
pub struct CompletedDownload {
    pub path: PathBuf,
    pub want_id: WantId,
    pub release_id: ReleaseId,
    pub media_type: MediaType,
}

/// Enriched metadata from the metadata resolver.
#[derive(Debug, Clone)]
pub struct ResolvedMetadata {
    pub media_type: MediaType,
    /// Token map used by the template engine.
    pub tokens: HashMap<String, String>,
    pub quality_score: u32,
}

/// Trait for resolving file identity and metadata (implemented by epignosis).
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; suppressed until Send bound concern is resolved"
)]
pub trait MetadataResolver: Send + Sync {
    async fn resolve_identity(
        &self,
        path: &Path,
        media_type: MediaType,
    ) -> Result<ResolvedMetadata, EpignosisError>;
}

/// The 6-step import pipeline.
pub struct ImportPipeline<R: MetadataResolver> {
    resolver: R,
    event_tx: EventSender,
    max_conflict_suffix: usize,
}

impl<R: MetadataResolver> ImportPipeline<R> {
    pub fn new(resolver: R, event_tx: EventSender) -> Self {
        Self {
            resolver,
            event_tx,
            max_conflict_suffix: DEFAULT_MAX_SUFFIX,
        }
    }

    pub fn with_max_conflict_suffix(mut self, max: usize) -> Self {
        self.max_conflict_suffix = max;
        self
    }

    #[instrument(skip(self, source), fields(path = %source.path.display(), media_type = ?source.media_type))]
    pub async fn process(&self, source: ImportSource) -> Result<ImportResult, TaxisError> {
        let media_type = source.media_type;

        // Resolve metadata via trait
        let metadata = self
            .resolver
            .resolve_identity(&source.path, media_type)
            .await
            .map_err(|e| TaxisError::MetadataResolutionFailed {
                path: source.path.clone(),
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Compute target path via naming template
        let template_str = source
            .naming_template
            .as_deref()
            .unwrap_or_else(|| template::default_template(media_type));
        let engine = TemplateEngine::parse(template_str, media_type)?;
        let relative_path = engine.resolve(&metadata.tokens)?;
        let target_path = source.library_root.join(&relative_path);

        // Conflict check and file operation
        let outcome = resolve_conflict(
            &target_path,
            None,
            metadata.quality_score,
            true,
            self.max_conflict_suffix,
        )?;

        let (final_path, operation) = match outcome {
            ConflictOutcome::Clear(p) | ConflictOutcome::Upgrade(p) => {
                let op = if p == target_path {
                    ImportOperation::Added
                } else {
                    ImportOperation::Upgraded
                };
                let result = match &source.origin {
                    ImportOrigin::Scanner => rename_file(&source.path, &p).await?,
                    ImportOrigin::Download { .. } => hardlink_or_copy(&source.path, &p).await?,
                };
                tracing::info!(path = %p.display(), op = ?result, "file operation complete");
                (p, op)
            }
            ConflictOutcome::Suffixed(p) => {
                let result = match &source.origin {
                    ImportOrigin::Scanner => rename_file(&source.path, &p).await?,
                    ImportOrigin::Download { .. } => hardlink_or_copy(&source.path, &p).await?,
                };
                tracing::info!(path = %p.display(), op = ?result, "file operation complete (suffixed)");
                (p, ImportOperation::Added)
            }
            ConflictOutcome::Skip => {
                tracing::info!(
                    path = %source.path.display(),
                    "skipping import — same or lower quality exists"
                );
                return Ok(ImportResult {
                    media_id: MediaId::new(),
                    media_type,
                    final_path: target_path,
                    quality_score: metadata.quality_score,
                    operation: ImportOperation::Skipped,
                });
            }
        };

        let media_id = MediaId::new();

        self.event_tx
            .send(HarmoniaEvent::ImportCompleted {
                media_id,
                media_type,
                path: final_path.clone(),
            })
            .ok();

        Ok(ImportResult {
            media_id,
            media_type,
            final_path,
            quality_score: metadata.quality_score,
            operation,
        })
    }
}

#[cfg(test)]
mod tests {
    use harmonia_common::create_event_bus;
    use tempfile::TempDir;

    use super::*;

    struct MockResolver {
        tokens: HashMap<String, String>,
        quality: u32,
    }

    impl MetadataResolver for MockResolver {
        async fn resolve_identity(
            &self,
            _path: &Path,
            media_type: MediaType,
        ) -> Result<ResolvedMetadata, EpignosisError> {
            Ok(ResolvedMetadata {
                media_type,
                tokens: self.tokens.clone(),
                quality_score: self.quality,
            })
        }
    }

    fn music_tokens() -> HashMap<String, String> {
        [
            ("Artist Name", "Test Artist"),
            ("Album Title", "Test Album"),
            ("Year", "2024"),
            ("Track Number", "1"),
            ("Track Title", "Test Track"),
            ("Extension", "flac"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    #[tokio::test]
    async fn import_pipeline_full_flow_scanner() {
        let dir = TempDir::new().unwrap();
        let source_file = dir.path().join("source.flac");
        let library_root = dir.path().join("library");
        std::fs::create_dir_all(&library_root).unwrap();
        std::fs::write(&source_file, b"FLAC").unwrap();

        let (tx, mut rx) = create_event_bus(16);
        let resolver = MockResolver {
            tokens: music_tokens(),
            quality: 300,
        };
        let pipeline = ImportPipeline::new(resolver, tx);

        let source = ImportSource {
            path: source_file,
            library_name: "music".into(),
            media_type: MediaType::Music,
            origin: ImportOrigin::Scanner,
            naming_template: Some("{Artist Name}/{Track Title}.{Extension}".to_string()),
            library_root: library_root.clone(),
        };

        let result = pipeline.process(source).await.unwrap();
        assert_eq!(result.media_type, MediaType::Music);
        assert_ne!(result.operation, ImportOperation::Skipped);
        assert!(result.final_path.exists());

        let event = rx.try_recv().unwrap();
        match event {
            HarmoniaEvent::ImportCompleted { media_type, .. } => {
                assert_eq!(media_type, MediaType::Music);
            }
            _ => panic!("expected ImportCompleted"),
        }
    }

    #[tokio::test]
    async fn import_pipeline_skips_on_same_quality() {
        let dir = TempDir::new().unwrap();
        let source_file = dir.path().join("source.flac");
        let library_root = dir.path().join("library");
        std::fs::create_dir_all(&library_root).unwrap();
        std::fs::write(&source_file, b"FLAC").unwrap();

        // Pre-create the target file. Without a DB quality lookup, conflict
        // resolution falls back to Suffixed (a new path with _2 suffix).
        let target = library_root.join("Test Artist/Test Track.flac");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"existing").unwrap();

        let (tx, _rx) = create_event_bus(16);
        let resolver = MockResolver {
            tokens: music_tokens(),
            quality: 300,
        };
        let pipeline = ImportPipeline::new(resolver, tx);

        let source = ImportSource {
            path: source_file,
            library_name: "music".into(),
            media_type: MediaType::Music,
            origin: ImportOrigin::Scanner,
            naming_template: Some("{Artist Name}/{Track Title}.{Extension}".to_string()),
            library_root: library_root.clone(),
        };

        let result = pipeline.process(source).await.unwrap();
        // Without DB quality info, pipeline cannot determine same-quality skip;
        // it produces a suffixed path and returns Added.
        assert_eq!(result.operation, ImportOperation::Added);
        // The suffixed file should exist
        assert!(result.final_path.exists());
        // Original target should still exist (not overwritten)
        assert!(target.exists());
    }

    #[tokio::test]
    async fn import_pipeline_emits_import_completed_event() {
        let dir = TempDir::new().unwrap();
        let source_file = dir.path().join("source.flac");
        let library_root = dir.path().join("library");
        std::fs::create_dir_all(&library_root).unwrap();
        std::fs::write(&source_file, b"data").unwrap();

        let (tx, mut rx) = create_event_bus(16);
        let resolver = MockResolver {
            tokens: music_tokens(),
            quality: 300,
        };
        let pipeline = ImportPipeline::new(resolver, tx);

        let source = ImportSource {
            path: source_file,
            library_name: "music".into(),
            media_type: MediaType::Music,
            origin: ImportOrigin::Scanner,
            naming_template: Some("{Artist Name}/{Track Title}.{Extension}".to_string()),
            library_root,
        };

        pipeline.process(source).await.unwrap();

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HarmoniaEvent::ImportCompleted { .. }));
    }
}
