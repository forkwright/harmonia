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

use crate::decode::{AudioDecoder, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

/// WavPack decoder — not yet implemented.
///
/// Probe routing rejects WavPack files with `UnsupportedCodec` before this type
/// is instantiated. The struct exists to hold the future implementation once
/// wavpack-sys is added.
pub struct WavPackDecoder;

impl AudioDecoder for WavPackDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>> {
        todo!("WavPack decode not yet implemented — extremely rare codec")
    }

    fn seek(
        &mut self,
        _position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        todo!("WavPack seek not yet implemented — extremely rare codec")
    }

    fn stream_params(&self) -> StreamParams {
        todo!("WavPack stream_params not yet implemented — extremely rare codec")
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        todo!("WavPack gapless_info not yet implemented — extremely rare codec")
    }
}

#[cfg(test)]
mod tests {
    use crate::decode::Codec;
    use crate::error::DecodeError;

    #[test]
    fn wavpack_unsupported_codec_error_contains_codec_name() {
        // probe.rs returns UnsupportedCodec for WavPack before WavPackDecoder is instantiated.
        let err = DecodeError::UnsupportedCodec {
            codec: Codec::Other("WavPack".to_string()),
            location: snafu::Location::new(file!(), line!(), column!()),
        };
        assert!(err.to_string().contains("WavPack"), "error must name the codec: {err}");
    }
}
