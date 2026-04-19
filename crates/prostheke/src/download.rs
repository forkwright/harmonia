//! Subtitle file download, format detection, and storage.

use std::path::{Path, PathBuf};

use snafu::ResultExt;
use tracing::instrument;

use crate::error::{FileWriteFailedSnafu, ProsthekeError};
use crate::types::SubtitleFormat;

/// Derive the subtitle file path from the media file path.
///
/// Convention: `{media_stem}.{lang}.{ext}` alongside the media file.
///
/// Example: `/library/movie.mkv` + lang `en` + format `srt`
///          → `/library/movie.en.srt`
pub(crate) fn subtitle_path(media_path: &Path, language: &str, format: SubtitleFormat) -> PathBuf {
    let parent = media_path.parent().unwrap_or(Path::new("."));
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let filename = format!("{stem}.{language}.{}", format.as_str());
    parent.join(filename)
}

/// Detect subtitle format from file extension embedded in a URL or filename.
pub(crate) fn detect_format_from_name(name: &str) -> Option<SubtitleFormat> {
    let ext = name.rsplit('.').next()?;
    SubtitleFormat::from_extension(ext)
}

/// Write subtitle bytes to disk at the computed path.
#[instrument(skip(content), fields(path = %path.display()))]
pub async fn write_subtitle_file(path: &Path, content: &[u8]) -> Result<(), ProsthekeError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context(FileWriteFailedSnafu)?;
    }
    tokio::fs::write(path, content)
        .await
        .context(FileWriteFailedSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_filename_convention_srt() {
        let media = Path::new("/library/Inception.2010.mkv");
        let path = subtitle_path(media, "en", SubtitleFormat::Srt);
        assert_eq!(path, PathBuf::from("/library/Inception.2010.en.srt"));
    }

    #[test]
    fn subtitle_filename_convention_ass() {
        let media = Path::new("/library/Inception.2010.mkv");
        let path = subtitle_path(media, "fr", SubtitleFormat::Ass);
        assert_eq!(path, PathBuf::from("/library/Inception.2010.fr.ass"));
    }

    #[test]
    fn subtitle_filename_preserves_media_directory() {
        let media = Path::new("/data/movies/film.mkv");
        let path = subtitle_path(media, "ja", SubtitleFormat::Vtt);
        assert_eq!(path.parent().unwrap(), Path::new("/data/movies"));
    }

    #[test]
    fn detect_format_from_srt_url() {
        assert_eq!(
            detect_format_from_name("subtitle.en.srt"),
            Some(SubtitleFormat::Srt)
        );
    }

    #[test]
    fn detect_format_from_ass_url() {
        assert_eq!(
            detect_format_from_name("https://example.com/sub.ass"),
            Some(SubtitleFormat::Ass)
        );
    }

    #[test]
    fn detect_format_unknown_extension_returns_none() {
        assert_eq!(detect_format_from_name("subtitle.xyz"), None);
    }

    #[tokio::test]
    async fn write_subtitle_file_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("movie.en.srt");
        let content = b"1\n00:00:01,000 --> 00:00:02,000\nHello\n";
        write_subtitle_file(&path, content).await.unwrap();
        let read_back = tokio::fs::read(&path).await.unwrap();
        assert_eq!(read_back, content);
    }

    #[tokio::test]
    async fn write_subtitle_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sub").join("movie.en.srt");
        write_subtitle_file(&path, b"content").await.unwrap();
        assert!(path.exists());
    }
}
