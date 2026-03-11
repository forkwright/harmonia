use std::path::Path;
use std::time::Duration;

use lofty::prelude::{Accessor, AudioFile, TaggedFileExt};
use lofty::prelude::ItemKey;
use tracing::instrument;

use crate::decode::{Codec, GaplessInfo};
use crate::error::DecodeError;

/// Tag and audio property metadata read from a source file via lofty.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub duration: Option<Duration>,

    pub replaygain_track_gain: Option<f32>,
    pub replaygain_track_peak: Option<f32>,
    pub replaygain_album_gain: Option<f32>,
    pub replaygain_album_peak: Option<f32>,

    /// EBU R128 track loudness offset in 1/256 LU units (i16).
    pub r128_track_gain: Option<i16>,
    /// EBU R128 album loudness offset in 1/256 LU units (i16).
    pub r128_album_gain: Option<i16>,
}

/// Reads gapless timing metadata for `path`.
///
/// Sources per codec:
/// - **MP3**: Xing/LAME header (via symphonia codec params with `enable_gapless`)
/// - **Vorbis**: 3456-sample pre-skip (hardcoded; symphonia issue #418)
/// - **AAC / ALAC**: iTunSMPB / codec params from MP4 container
/// - **Opus**: OGG header pre-skip via symphonia codec params
/// - **FLAC / WAV / AIFF**: no encoder delay — returns `None`
#[instrument]
pub fn read_gapless_info(path: &Path, codec: &Codec) -> Option<GaplessInfo> {
    use symphonia::core::codecs::CODEC_TYPE_NULL;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    if matches!(codec, Codec::Vorbis) {
        return Some(GaplessInfo { encoder_delay: 3456, encoder_padding: 0, total_samples: None });
    }

    if matches!(codec, Codec::Flac | Codec::Wav | Codec::Aiff) {
        return None;
    }

    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions { enable_gapless: true, ..Default::default() },
            &MetadataOptions::default(),
        )
        .ok()?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;

    let p = &track.codec_params;
    if p.delay.is_some() || p.padding.is_some() {
        Some(GaplessInfo {
            encoder_delay: p.delay.unwrap_or(0),
            encoder_padding: p.padding.unwrap_or(0),
            total_samples: p.n_frames,
        })
    } else {
        None
    }
}

/// Reads tag and audio properties from `path` using lofty.
///
/// ReplayGain and R128 values are parsed from string tags. Missing or
/// unparseable values are silently omitted — callers should apply sensible
/// defaults.
#[instrument]
pub fn read_track_metadata(path: &Path) -> Result<TrackMetadata, DecodeError> {
    let tagged = lofty::read_from_path(path).map_err(|e| DecodeError::Metadata {
        message: format!("lofty read failed: {e}"),
        location: snafu::location!(),
    })?;

    let duration = Some(tagged.properties().duration());

    let (title, artist, album, track_number, rg_tg, rg_tp, rg_ag, rg_ap, r128_tg, r128_ag) =
        if let Some(tag) = tagged.primary_tag() {
            (
                tag.title().map(|v| v.to_string()),
                tag.artist().map(|v| v.to_string()),
                tag.album().map(|v| v.to_string()),
                tag.track(),
                tag.get_string(&ItemKey::ReplayGainTrackGain).and_then(parse_gain_db),
                tag.get_string(&ItemKey::ReplayGainTrackPeak).and_then(parse_float),
                tag.get_string(&ItemKey::ReplayGainAlbumGain).and_then(parse_gain_db),
                tag.get_string(&ItemKey::ReplayGainAlbumPeak).and_then(parse_float),
                tag.get_string(&ItemKey::Unknown("R128_TRACK_GAIN".to_string()))
                    .and_then(|s| s.trim().parse().ok()),
                tag.get_string(&ItemKey::Unknown("R128_ALBUM_GAIN".to_string()))
                    .and_then(|s| s.trim().parse().ok()),
            )
        } else {
            (None, None, None, None, None, None, None, None, None, None)
        };

    Ok(TrackMetadata {
        title,
        artist,
        album,
        track_number,
        duration,
        replaygain_track_gain: rg_tg,
        replaygain_track_peak: rg_tp,
        replaygain_album_gain: rg_ag,
        replaygain_album_peak: rg_ap,
        r128_track_gain: r128_tg,
        r128_album_gain: r128_ag,
    })
}

/// Parses a ReplayGain dB string like `"-3.50 dB"` or `"-3.50"` to `f32`.
fn parse_gain_db(s: &str) -> Option<f32> {
    s.trim().trim_end_matches("dB").trim().parse().ok()
}

/// Parses a bare float string.
fn parse_float(s: &str) -> Option<f32> {
    s.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vorbis_gapless_hardcoded_3456() {
        // read_gapless_info needs a real path for non-Vorbis; for Vorbis it short-circuits.
        let info = read_gapless_info(Path::new("/dev/null"), &Codec::Vorbis);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.encoder_delay, 3456);
        assert_eq!(info.encoder_padding, 0);
    }

    #[test]
    fn flac_gapless_is_none() {
        let info = read_gapless_info(Path::new("/dev/null"), &Codec::Flac);
        assert!(info.is_none());
    }

    #[test]
    fn wav_gapless_is_none() {
        let info = read_gapless_info(Path::new("/dev/null"), &Codec::Wav);
        assert!(info.is_none());
    }

    #[test]
    fn aiff_gapless_is_none() {
        let info = read_gapless_info(Path::new("/dev/null"), &Codec::Aiff);
        assert!(info.is_none());
    }

    #[test]
    fn parse_gain_db_with_suffix() {
        let v = parse_gain_db("-3.50 dB").unwrap();
        assert!((v - (-3.50f32)).abs() < 1e-4);
    }

    #[test]
    fn parse_gain_db_without_suffix() {
        let v = parse_gain_db("-3.50").unwrap();
        assert!((v - (-3.50f32)).abs() < 1e-4);
    }

    #[test]
    fn parse_gain_db_invalid_returns_none() {
        assert!(parse_gain_db("not a number").is_none());
    }
}
