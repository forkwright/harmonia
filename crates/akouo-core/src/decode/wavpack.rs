// P1-03: WavPackDecoder skeleton.
//
// WavPack is extremely rare in real music libraries. This skeleton satisfies the
// AudioDecoder trait so probe.rs can reject WavPack files cleanly. Implement via
// wavpack-sys = "0.4" when there is genuine user demand.
//
// Tracking: uncomment `wavpack-sys = "0.4"` in Cargo.toml and fill in the bodies below.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

/// WavPack decoder — not yet implemented.
///
/// Probe routing rejects WavPack files with `UnsupportedCodec` before this type
/// is instantiated. The struct exists to hold the future implementation once
/// wavpack-sys is added.
pub struct WavPackDecoder;

fn unsupported_codec() -> DecodeError {
    DecodeError::UnsupportedCodec {
        codec: Codec::Other("WavPack".to_string()),
        location: std::panic::Location::caller(),
    }
}

impl AudioDecoder for WavPackDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>> {
        Box::pin(async { Err(unsupported_codec()) })
    }

    fn seek(
        &mut self,
        _position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        Box::pin(async { Err(unsupported_codec()) })
    }

    fn stream_params(&self) -> StreamParams {
        StreamParams {
            codec: Codec::Other("WavPack".to_string()),
            sample_rate: 0,
            channels: 0,
            bit_depth: None,
            duration: None,
            bitrate: None,
        }
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AudioDecoder, WavPackDecoder};
    use crate::decode::Codec;
    use crate::error::DecodeError;

    #[test]
    fn wavpack_unsupported_codec_error_contains_codec_name() {
        // probe.rs returns UnsupportedCodec for WavPack before WavPackDecoder is instantiated.
        let err = DecodeError::UnsupportedCodec {
            codec: Codec::Other("WavPack".to_string()),
            location: std::panic::Location::caller(),
        };
        assert!(
            err.to_string().contains("WavPack"),
            "error must name the codec: {err}"
        );
    }

    #[tokio::test]
    async fn wavpack_decoder_frame_returns_unsupported_codec() {
        let mut decoder = WavPackDecoder;

        let err = decoder
            .next_frame()
            .await
            .expect_err("WavPack decoder must not produce frames without backend support");

        assert!(matches!(
            err,
            DecodeError::UnsupportedCodec {
                codec: Codec::Other(ref codec),
                ..
            } if codec == "WavPack"
        ));
    }

    #[tokio::test]
    async fn wavpack_decoder_seek_returns_unsupported_codec() {
        let mut decoder = WavPackDecoder;

        let err = decoder
            .seek(Duration::from_secs(1))
            .await
            .expect_err("WavPack decoder must not seek without backend support");

        assert!(matches!(
            err,
            DecodeError::UnsupportedCodec {
                codec: Codec::Other(ref codec),
                ..
            } if codec == "WavPack"
        ));
    }

    #[test]
    fn wavpack_decoder_params_are_explicit_placeholder() {
        let decoder = WavPackDecoder;

        let params = decoder.stream_params();

        assert_eq!(params.codec, Codec::Other("WavPack".to_string()));
        assert_eq!(params.sample_rate, 0);
        assert!(decoder.gapless_info().is_none());
    }
}
