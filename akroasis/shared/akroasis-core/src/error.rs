use snafu::Snafu;

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum EngineError {
    #[snafu(display("decode error"))]
    Decode { source: DecodeError },

    #[snafu(display("DSP error"))]
    Dsp { source: DspError },

    #[snafu(display("output error"))]
    Output { source: OutputError },

    #[snafu(display("playback already in progress"))]
    AlreadyPlaying,

    #[snafu(display(
        "seek position {position_secs:.3}s out of bounds (duration: {duration_secs:.3}s)"
    ))]
    SeekOutOfBounds {
        position_secs: f64,
        duration_secs: f64,
    },
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum DecodeError {
    #[snafu(display("symphonia read failed: {message}"))]
    SymphoniaRead {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("symphonia decode failed: {message}"))]
    SymphoniaDecode {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("opus decode failed: {message}"))]
    OpusDecode {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unsupported codec: {codec:?}"))]
    UnsupportedCodec {
        codec: crate::decode::Codec,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("metadata error: {message}"))]
    Metadata {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum DspError {
    #[snafu(display("DSP stage '{stage}' failed: {message}"))]
    StageFailed { stage: String, message: String },

    #[snafu(display("invalid DSP configuration: {message}"))]
    InvalidConfig { message: String },
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum OutputError {
    #[snafu(display("no audio output device available"))]
    NoDevice,

    #[snafu(display("failed to open output device '{device}': {message}"))]
    DeviceOpen { device: String, message: String },

    #[snafu(display("output format not supported: {message}"))]
    FormatUnsupported { message: String },

    #[snafu(display("output stream error: {message}"))]
    StreamError { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_error_already_playing_display() {
        let e = EngineError::AlreadyPlaying;
        assert!(e.to_string().contains("already in progress"));
    }

    #[test]
    fn engine_error_seek_out_of_bounds_display() {
        let e = EngineError::SeekOutOfBounds {
            position_secs: 10.5,
            duration_secs: 5.0,
        };
        let msg = e.to_string();
        assert!(msg.contains("10.500"));
        assert!(msg.contains("5.000"));
    }

    #[test]
    fn output_error_no_device_display() {
        let e = OutputError::NoDevice;
        assert!(e.to_string().contains("no audio output device"));
    }
}
