use std::path::Path;

use lofty::prelude::{Accessor, ItemKey, TaggedFileExt};
use tracing::instrument;

use crate::error::TaxisError;

// WHY: pure data — embedded tag fields read from a media file via lofty.
/// Embedded metadata read from a single media file.
///
/// A missing field (untagged file, unsupported container, or a genuinely
/// absent tag item) is `None`, never an error — callers fall back to other
/// hint sources (DB, filename) per field.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FileTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
}

impl FileTags {
    /// True if every field is `None` — either the file carried no primary
    /// tag at all, or its primary tag existed but populated none of the
    /// fields this resolver ladder consumes. Distinct FROM a read error
    /// (`Err`): this is a successful read that simply found nothing,
    /// signalling callers to weight DB/filename fallbacks more heavily.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.album_artist.is_none()
            && self.track_number.is_none()
            && self.disc_number.is_none()
            && self.year.is_none()
    }
}

/// Reads embedded tags FROM `path` using lofty.
///
/// Runs on a blocking thread — lofty's reader is synchronous. A malformed or
/// untagged file is a real error here (the caller decides whether that is
/// fatal); an absent individual field within a successfully-read tag is not.
#[instrument]
pub async fn read_tags(path: &Path) -> Result<FileTags, TaxisError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || read_tags_blocking(&path))
        .await
        .map_err(|e| TaxisError::BlockingTaskFailed {
            message: e.to_string(),
            location: snafu::location!(),
        })
        .and_then(|r| r)
}

fn read_tags_blocking(path: &Path) -> Result<FileTags, TaxisError> {
    let tagged = lofty::read_from_path(path).map_err(|source| TaxisError::TagRead {
        path: path.to_path_buf(),
        source,
        location: snafu::location!(),
    })?;

    let Some(tag) = tagged.primary_tag() else {
        return Ok(FileTags::default());
    };

    let year = tag
        .date()
        .map(|ts| u32::from(ts.year))
        .or_else(|| tag.get_string(ItemKey::Year).and_then(|s| s.parse().ok()));

    Ok(FileTags {
        title: tag.title().map(|v| v.to_string()),
        artist: tag.artist().map(|v| v.to_string()),
        album: tag.album().map(|v| v.to_string()),
        album_artist: tag.get_string(ItemKey::AlbumArtist).map(|v| v.to_string()),
        track_number: tag.track(),
        disc_number: tag.disk(),
        year,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    #[tokio::test]
    async fn corrupt_file_returns_tag_read_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupt.flac");
        std::fs::write(&path, b"not a real flac file").unwrap();

        let result = read_tags(&path).await;
        assert!(matches!(result, Err(TaxisError::TagRead { .. })));
    }

    #[tokio::test]
    async fn nonexistent_file_returns_tag_read_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.flac");

        let result = read_tags(&path).await;
        assert!(result.is_err());
    }

    #[test]
    fn file_tags_default_is_all_none() {
        let tags = FileTags::default();
        assert!(tags.title.is_none());
        assert!(tags.artist.is_none());
        assert!(tags.album.is_none());
        assert!(tags.album_artist.is_none());
        assert!(tags.track_number.is_none());
        assert!(tags.disc_number.is_none());
        assert!(tags.year.is_none());
    }

    // ── real-file positive read (hand-built FLAC + VORBIS_COMMENT tag) ──────

    /// Big-endian bit packer FOR the FLAC STREAMINFO block's odd-width
    /// fields (20-bit sample rate, 3-bit channel count, 5-bit bit depth,
    /// 36-bit sample total) — hand-computing those byte boundaries is
    /// error-prone, so this packs them programmatically instead.
    struct BitWriter {
        buf: Vec<u8>,
        acc: u64,
        nbits: u32,
    }

    impl BitWriter {
        fn new() -> Self {
            Self {
                buf: Vec::new(),
                acc: 0,
                nbits: 0,
            }
        }

        fn write_bits(&mut self, value: u64, bits: u32) {
            self.acc = (self.acc << bits) | (value & ((1u64 << bits) - 1));
            self.nbits += bits;
            while self.nbits >= 8 {
                self.nbits -= 8;
                self.buf.push(((self.acc >> self.nbits) & 0xFF) as u8);
            }
        }

        fn finish(self) -> Vec<u8> {
            self.buf
        }
    }

    fn streaminfo_block() -> Vec<u8> {
        let mut bw = BitWriter::new();
        bw.write_bits(4096, 16); // min block size
        bw.write_bits(4096, 16); // max block size
        bw.write_bits(0, 24); // min frame size (unknown)
        bw.write_bits(0, 24); // max frame size (unknown)
        bw.write_bits(44100, 20); // sample rate
        bw.write_bits(1, 3); // channels - 1 (2 channels)
        bw.write_bits(15, 5); // bits per sample - 1 (16 bits)
        bw.write_bits(0, 36); // total samples (unknown — no audio frames follow)
        let mut block = bw.finish();
        block.extend_from_slice(&[0u8; 16]); // MD5 signature (all-zero = unknown, valid per spec)
        block
    }

    fn vorbis_comment_block(tags: &[(&str, &str)]) -> Vec<u8> {
        let vendor = b"kathodos-test";
        let mut data = Vec::new();
        data.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        data.extend_from_slice(vendor);
        data.extend_from_slice(&(tags.len() as u32).to_le_bytes());
        for (k, v) in tags {
            let comment = format!("{k}={v}");
            data.extend_from_slice(&(comment.len() as u32).to_le_bytes());
            data.extend_from_slice(comment.as_bytes());
        }
        data
    }

    /// Builds a minimal valid FLAC file (STREAMINFO + VORBIS_COMMENT, zero
    /// audio frames) with the given tags — the smallest real container
    /// exercising `read_tags`'s success path against actual lofty parsing
    /// rather than a mock.
    fn make_tagged_flac(
        title: &str,
        artist: &str,
        album: &str,
        track: &str,
        year: &str,
    ) -> NamedTempFile {
        let streaminfo = streaminfo_block();
        let vorbis = vorbis_comment_block(&[
            ("TITLE", title),
            ("ARTIST", artist),
            ("ALBUM", album),
            ("TRACKNUMBER", track),
            ("DATE", year),
        ]);

        let mut out = Vec::new();
        out.extend_from_slice(b"fLaC");

        out.push(0x00); // block type 0 (STREAMINFO), not last
        let len = streaminfo.len() as u32;
        out.extend_from_slice(&len.to_be_bytes()[1..4]);
        out.extend_from_slice(&streaminfo);

        out.push(0x84); // last-block flag (0x80) | block type 4 (VORBIS_COMMENT)
        let len2 = vorbis.len() as u32;
        out.extend_from_slice(&len2.to_be_bytes()[1..4]);
        out.extend_from_slice(&vorbis);

        let mut f = tempfile::Builder::new().suffix(".flac").tempfile().unwrap();
        f.write_all(&out).unwrap();
        f
    }

    #[tokio::test]
    async fn reads_embedded_tags_from_real_file() {
        let flac = make_tagged_flac("Test Track", "Test Artist", "Test Album", "3", "1999");

        let tags = read_tags(flac.path()).await.unwrap();

        assert_eq!(tags.title.as_deref(), Some("Test Track"));
        assert_eq!(tags.artist.as_deref(), Some("Test Artist"));
        assert_eq!(tags.album.as_deref(), Some("Test Album"));
        assert_eq!(tags.track_number, Some(3));
        assert_eq!(tags.year, Some(1999));
    }

    #[tokio::test]
    async fn untagged_valid_file_returns_all_none() {
        // WHY: a STREAMINFO-only FLAC (no VORBIS_COMMENT block) has no
        // primary tag at all — distinct FROM `make_tagged_flac` with empty
        // string values, which would still produce an (empty-string) tag.
        let streaminfo = streaminfo_block();
        let mut out = Vec::new();
        out.extend_from_slice(b"fLaC");
        out.push(0x80); // block type 0 (STREAMINFO), LAST block
        let len = streaminfo.len() as u32;
        out.extend_from_slice(&len.to_be_bytes()[1..4]);
        out.extend_from_slice(&streaminfo);

        let mut f = tempfile::Builder::new().suffix(".flac").tempfile().unwrap();
        f.write_all(&out).unwrap();

        let tags = read_tags(f.path()).await.unwrap();
        assert_eq!(tags.title, None);
        assert_eq!(tags.artist, None);
    }
}
