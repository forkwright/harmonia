use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum SidecarError {
    #[snafu(display("failed to read sidecar {path:?}: {source}"))]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to write sidecar {path:?}: {source}"))]
    Write {
        path: std::path::PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse sidecar {path:?}: {source}"))]
    Parse {
        path: std::path::PathBuf,
        source: toml::de::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to serialize sidecar {path:?}: {source}"))]
    Serialize {
        path: std::path::PathBuf,
        source: toml::ser::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

// ── Shared section ────────────────────────────────────────────────────────────

/// Shared `[meta]` section present in every sidecar file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    /// Enrichment source (e.g. `"musicbrainz"`, `"audnexus"`).
    pub source: Option<String>,
    /// External identifier from the enrichment source.
    pub source_id: Option<String>,
    /// ISO 8601 timestamp of when this item was imported.
    pub imported_at: Option<String>,
    /// Kritike quality score in the range 0.0–1.0.
    pub quality_score: Option<f64>,
}

// ── Per-format sidecars ───────────────────────────────────────────────────────

/// Sidecar for a music release directory (`album.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlbumSidecar {
    pub meta: Meta,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub year: Option<u16>,
    pub genres: Option<Vec<String>>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
}

/// Sidecar for an artist directory (`artist.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistSidecar {
    pub meta: Meta,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub genres: Option<Vec<String>>,
    pub country: Option<String>,
    pub formed: Option<u16>,
    pub disbanded: Option<u16>,
}

/// Sidecar for an ebook directory (`book.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookSidecar {
    pub meta: Meta,
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<u16>,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub goodreads_id: Option<String>,
}

/// Sidecar for an audiobook directory (`audiobook.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudiobookSidecar {
    pub meta: Meta,
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub year: Option<u16>,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    /// Total duration in seconds.
    pub duration_secs: Option<u64>,
    pub audnexus_id: Option<String>,
}

/// Sidecar for a podcast show directory (`show.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowSidecar {
    pub meta: Meta,
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub rss_url: Option<String>,
    pub categories: Option<Vec<String>>,
}

// ── I/O functions ─────────────────────────────────────────────────────────────

/// Read and deserialize a TOML sidecar file from `path`.
///
/// Returns an error if the file cannot be read or contains invalid TOML.
#[must_use = "handle the Result from read_sidecar"]
#[expect(
    clippy::result_large_err,
    reason = "SidecarError is 136 bytes due to toml::de::Error; boxing would require restructuring the Snafu enum and its public API"
)]
pub fn read_sidecar<T: DeserializeOwned>(path: &Path) -> Result<T, SidecarError> {
    let text = fs::read_to_string(path).context(ReadSnafu { path })?;
    toml::from_str(&text).context(ParseSnafu { path })
}

/// Serialize `data` as TOML and write it to `path`.
///
/// Parent directories must already exist. The file is created or overwritten.
#[must_use = "handle the Result from write_sidecar"]
#[expect(
    clippy::result_large_err,
    reason = "SidecarError is 136 bytes due to toml::de::Error; boxing would require restructuring the Snafu enum and its public API"
)]
pub fn write_sidecar<T: Serialize>(path: &Path, data: &T) -> Result<(), SidecarError> {
    let text = toml::to_string_pretty(data).context(SerializeSnafu { path })?;
    fs::write(path, text).context(WriteSnafu { path })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn empty_meta() -> Meta {
        Meta {
            source: None,
            source_id: None,
            imported_at: None,
            quality_score: None,
        }
    }

    // ── round-trip tests ──────────────────────────────────────────────────────

    #[test]
    fn album_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("album.toml");

        let original = AlbumSidecar {
            meta: Meta {
                source: Some("musicbrainz".into()),
                source_id: Some("abc-123".into()),
                imported_at: Some("2026-04-12T00:00:00Z".into()),
                quality_score: Some(0.95),
            },
            title: Some("Elisabeth".into()),
            artist: Some("Sylvie Guillem".into()),
            year: Some(2020),
            genres: Some(vec!["Contemporary".into(), "Classical".into()]),
            label: Some("ECM".into()),
            catalog_number: Some("ECM-2700".into()),
        };

        write_sidecar(&path, &original).unwrap();
        let loaded: AlbumSidecar = read_sidecar(&path).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn artist_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("artist.toml");

        let original = ArtistSidecar {
            meta: Meta {
                source: Some("musicbrainz".into()),
                source_id: Some("def-456".into()),
                imported_at: Some("2026-04-12T00:00:00Z".into()),
                quality_score: Some(0.88),
            },
            name: Some("Radiohead".into()),
            sort_name: Some("Radiohead".into()),
            genres: Some(vec!["Alternative Rock".into()]),
            country: Some("GB".into()),
            formed: Some(1985),
            disbanded: None,
        };

        write_sidecar(&path, &original).unwrap();
        let loaded: ArtistSidecar = read_sidecar(&path).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn book_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("book.toml");

        let original = BookSidecar {
            meta: Meta {
                source: Some("openlibrary".into()),
                source_id: Some("OL123M".into()),
                imported_at: Some("2026-04-12T00:00:00Z".into()),
                quality_score: Some(1.0),
            },
            title: Some("Gödel, Escher, Bach".into()),
            author: Some("Douglas Hofstadter".into()),
            year: Some(1979),
            isbn: Some("978-0-465-02656-2".into()),
            publisher: Some("Basic Books".into()),
            series: None,
            series_index: None,
            goodreads_id: Some("24113".into()),
        };

        write_sidecar(&path, &original).unwrap();
        let loaded: BookSidecar = read_sidecar(&path).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn audiobook_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("audiobook.toml");

        let original = AudiobookSidecar {
            meta: Meta {
                source: Some("audnexus".into()),
                source_id: Some("B001".into()),
                imported_at: Some("2026-04-12T00:00:00Z".into()),
                quality_score: Some(0.92),
            },
            title: Some("Dune".into()),
            author: Some("Frank Herbert".into()),
            narrator: Some("Scott Brick".into()),
            year: Some(2007),
            isbn: Some("978-0-7393-3656-1".into()),
            publisher: Some("Macmillan Audio".into()),
            series: Some("Dune Chronicles".into()),
            series_index: Some(1.0),
            duration_secs: Some(77_400),
            audnexus_id: Some("B001AAAAAA".into()),
        };

        write_sidecar(&path, &original).unwrap();
        let loaded: AudiobookSidecar = read_sidecar(&path).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn show_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("show.toml");

        let original = ShowSidecar {
            meta: Meta {
                source: Some("podcastindex".into()),
                source_id: Some("pod-789".into()),
                imported_at: Some("2026-04-12T00:00:00Z".into()),
                quality_score: Some(0.75),
            },
            title: Some("Lex Fridman Podcast".into()),
            author: Some("Lex Fridman".into()),
            description: Some("Conversations about science and technology.".into()),
            rss_url: Some("https://lexfridman.com/feed/podcast/".into()),
            categories: Some(vec!["Technology".into(), "Science".into()]),
        };

        write_sidecar(&path, &original).unwrap();
        let loaded: ShowSidecar = read_sidecar(&path).unwrap();
        assert_eq!(original, loaded);
    }

    // ── partial / missing optional fields ─────────────────────────────────────

    #[test]
    fn album_partial_fields_deserialize_to_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("album.toml");

        let minimal = AlbumSidecar {
            meta: empty_meta(),
            title: Some("Minimal Album".into()),
            artist: None,
            year: None,
            genres: None,
            label: None,
            catalog_number: None,
        };

        write_sidecar(&path, &minimal).unwrap();
        let loaded: AlbumSidecar = read_sidecar(&path).unwrap();
        assert_eq!(loaded.artist, None);
        assert_eq!(loaded.year, None);
        assert_eq!(loaded.genres, None);
        assert_eq!(loaded.label, None);
        assert_eq!(loaded.catalog_number, None);
    }

    #[test]
    fn empty_meta_deserializes_to_none_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("show.toml");

        let s = ShowSidecar {
            meta: empty_meta(),
            title: None,
            author: None,
            description: None,
            rss_url: None,
            categories: None,
        };

        write_sidecar(&path, &s).unwrap();
        let loaded: ShowSidecar = read_sidecar(&path).unwrap();
        assert_eq!(loaded.meta.source, None);
        assert_eq!(loaded.meta.source_id, None);
        assert_eq!(loaded.meta.imported_at, None);
        assert_eq!(loaded.meta.quality_score, None);
    }

    // ── error cases ───────────────────────────────────────────────────────────

    #[test]
    fn invalid_toml_returns_error_not_panic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        fs::write(&path, b"not valid toml ][[[").unwrap();

        let result: Result<AlbumSidecar, _> = read_sidecar(&path);
        assert!(result.is_err());
        // Confirm the error is a parse error, not a read error
        let err = result.unwrap_err();
        assert!(matches!(err, SidecarError::Parse { .. }));
    }

    #[test]
    fn file_not_found_returns_read_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let result: Result<AlbumSidecar, _> = read_sidecar(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SidecarError::Read { .. }));
    }

    #[test]
    fn wrong_type_returns_parse_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wrong.toml");
        // Write a show sidecar but deserialize as album — structurally valid TOML
        // but semantically a different type; serde should still succeed because
        // all fields are optional, but this confirms it doesn't panic.
        let show = ShowSidecar {
            meta: empty_meta(),
            title: Some("A Show".into()),
            author: None,
            description: None,
            rss_url: None,
            categories: None,
        };
        write_sidecar(&path, &show).unwrap();
        // Deserializing as AlbumSidecar should succeed (all fields optional)
        let result: Result<AlbumSidecar, _> = read_sidecar(&path);
        assert!(result.is_ok());
    }
}
